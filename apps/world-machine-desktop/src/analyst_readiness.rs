use crate::analyst_session::DesktopAnalystConfig;
use std::env;
use std::ffi::{OsStr, OsString};
use std::fmt;
use std::fs;
use std::path::{Component, Path, PathBuf};

const LAUNCHER_PROGRAMS: [&str; 2] = ["bash", "dirname"];

#[cfg(any(target_os = "macos", target_os = "linux"))]
mod effective_access {
    use std::os::raw::{c_char, c_int};
    use std::path::Path;

    const X_OK: c_int = 1;

    #[cfg(target_os = "macos")]
    const AT_FDCWD: c_int = -2;
    #[cfg(target_os = "macos")]
    const AT_EACCESS: c_int = 0x0010;

    #[cfg(target_os = "linux")]
    const AT_FDCWD: c_int = -100;
    #[cfg(target_os = "linux")]
    const AT_EACCESS: c_int = 0x0200;

    unsafe extern "C" {
        fn faccessat(dirfd: c_int, pathname: *const c_char, mode: c_int, flags: c_int) -> c_int;
        fn geteuid() -> u32;
    }

    pub(super) fn can_execute(path: &Path) -> bool {
        use std::ffi::CString;
        use std::os::unix::ffi::OsStrExt;

        let Ok(path) = CString::new(path.as_os_str().as_bytes()) else {
            return false;
        };
        // SAFETY: `path` is a live NUL-terminated C string for the duration of the call.
        // `faccessat` only reads it and does not retain the pointer. AT_EACCESS asks the OS to
        // apply the current effective user/group and filesystem ACL semantics.
        unsafe { faccessat(AT_FDCWD, path.as_ptr(), X_OK, AT_EACCESS) == 0 }
    }

    #[cfg(test)]
    pub(super) fn effective_uid() -> u32 {
        // SAFETY: `geteuid` takes no arguments and has no memory-safety preconditions.
        unsafe { geteuid() }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DesktopAnalystRuntimeIssueKind {
    RuntimeUnavailable,
    RuntimeIncomplete,
    LauncherUnavailable,
    AnalystProgramUnavailable,
    NodeUnavailable,
    PiUnavailable,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DesktopAnalystRuntimeIssue {
    kind: DesktopAnalystRuntimeIssueKind,
    message: String,
}

impl DesktopAnalystRuntimeIssue {
    pub fn new(kind: DesktopAnalystRuntimeIssueKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    pub fn kind(&self) -> DesktopAnalystRuntimeIssueKind {
        self.kind
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for DesktopAnalystRuntimeIssue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DesktopAnalystRuntimeReadiness {
    Ready { config: DesktopAnalystConfig },
    Unavailable { issue: DesktopAnalystRuntimeIssue },
}

impl DesktopAnalystRuntimeReadiness {
    pub fn ready(config: DesktopAnalystConfig) -> Self {
        Self::Ready { config }
    }

    pub fn unavailable(kind: DesktopAnalystRuntimeIssueKind, message: impl Into<String>) -> Self {
        Self::Unavailable {
            issue: DesktopAnalystRuntimeIssue::new(kind, message),
        }
    }

    pub fn is_ready(&self) -> bool {
        matches!(self, Self::Ready { .. })
    }

    pub fn config(&self) -> Option<&DesktopAnalystConfig> {
        match self {
            Self::Ready { config } => Some(config),
            Self::Unavailable { .. } => None,
        }
    }

    pub fn issue(&self) -> Option<&DesktopAnalystRuntimeIssue> {
        match self {
            Self::Ready { .. } => None,
            Self::Unavailable { issue } => Some(issue),
        }
    }
}

pub fn check(config: DesktopAnalystConfig) -> DesktopAnalystRuntimeReadiness {
    check_with_environment(config, env::var_os("PATH"), env::current_dir().ok())
}

fn check_with_environment(
    mut config: DesktopAnalystConfig,
    path: Option<OsString>,
    current_dir: Option<PathBuf>,
) -> DesktopAnalystRuntimeReadiness {
    if !config.turn_host_script.is_file() {
        return DesktopAnalystRuntimeReadiness::unavailable(
            DesktopAnalystRuntimeIssueKind::RuntimeIncomplete,
            format!(
                "World Machine analyst runtime is incomplete: missing {}",
                config.turn_host_script.display()
            ),
        );
    }

    for launcher_program in LAUNCHER_PROGRAMS {
        if resolve_launcher_program(Path::new(launcher_program), path.as_deref()).is_none() {
            return DesktopAnalystRuntimeReadiness::unavailable(
                DesktopAnalystRuntimeIssueKind::LauncherUnavailable,
                format!(
                    "World Machine analyst launcher needs `{launcher_program}`, but it is not executable in an absolute PATH directory. Ensure the standard macOS system executable directories are available on PATH."
                ),
            );
        }
    }

    let node_program = match resolve_program(
        &config.node_program,
        path.as_deref(),
        current_dir.as_deref(),
    ) {
        Some(program) => program,
        None => {
            return DesktopAnalystRuntimeReadiness::unavailable(
                DesktopAnalystRuntimeIssueKind::NodeUnavailable,
                format!(
                    "World Machine analyst needs Node, but `{}` is not executable or not available on PATH. Set WORLD_MACHINE_NODE_PROGRAM to an executable Node path.",
                    config.node_program.display()
                ),
            );
        }
    };

    let requested_pi = config
        .pi_program
        .clone()
        .unwrap_or_else(|| PathBuf::from("pi"));
    let pi_program = match resolve_program(&requested_pi, path.as_deref(), current_dir.as_deref()) {
        Some(program) => program,
        None => {
            return DesktopAnalystRuntimeReadiness::unavailable(
                DesktopAnalystRuntimeIssueKind::PiUnavailable,
                format!(
                    "World Machine analyst needs Pi, but `{}` is not executable or not available on PATH. Set PI_PROGRAM to the Pi executable path.",
                    requested_pi.display()
                ),
            );
        }
    };

    let Some(requested_analyst) = config.analyst_program.clone() else {
        return DesktopAnalystRuntimeReadiness::unavailable(
            DesktopAnalystRuntimeIssueKind::AnalystProgramUnavailable,
            "World Machine analyst tool host is not configured.",
        );
    };
    let analyst_program =
        match resolve_program(&requested_analyst, path.as_deref(), current_dir.as_deref()) {
            Some(program) => program,
            None => {
                return DesktopAnalystRuntimeReadiness::unavailable(
                    DesktopAnalystRuntimeIssueKind::AnalystProgramUnavailable,
                    format!(
                        "World Machine analyst tool host is not executable or unavailable: {}",
                        requested_analyst.display()
                    ),
                );
            }
        };

    config.node_program = node_program;
    config.pi_program = Some(pi_program);
    config.analyst_program = Some(analyst_program);
    DesktopAnalystRuntimeReadiness::ready(config)
}

fn resolve_launcher_program(program: &Path, path: Option<&OsStr>) -> Option<PathBuf> {
    let path = path?;
    env::split_paths(path)
        .filter(|directory| directory.is_absolute())
        .map(|directory| directory.join(program))
        .find(|candidate| executable_file(candidate))
}

fn resolve_program(
    program: &Path,
    path: Option<&OsStr>,
    current_dir: Option<&Path>,
) -> Option<PathBuf> {
    if is_explicit_path(program) {
        let candidate = if program.is_absolute() {
            program.to_path_buf()
        } else {
            current_dir?.join(program)
        };
        return executable_file(&candidate).then_some(candidate);
    }

    let path = path?;
    env::split_paths(path)
        .filter_map(|directory| {
            let directory = if directory.is_absolute() {
                directory
            } else {
                current_dir?.join(directory)
            };
            Some(directory.join(program))
        })
        .find(|candidate| executable_file(candidate))
}

fn is_explicit_path(program: &Path) -> bool {
    program.is_absolute()
        || program
            .components()
            .any(|component| matches!(component, Component::CurDir | Component::ParentDir))
        || program.components().count() > 1
}

fn executable_file(path: &Path) -> bool {
    let Ok(metadata) = fs::metadata(path) else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        effective_access::can_execute(path)
    }
    #[cfg(all(unix, not(any(target_os = "macos", target_os = "linux"))))]
    {
        false
    }
    #[cfg(not(unix))]
    {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(1);

    struct Fixture {
        root: PathBuf,
        bin: PathBuf,
        turn_host: PathBuf,
        analyst: PathBuf,
    }

    impl Fixture {
        fn new() -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let root = env::temp_dir().join(format!(
                "world-machine-m225-readiness-{}-{nonce}-{sequence}",
                std::process::id()
            ));
            let bin = root.join("bin");
            fs::create_dir_all(&bin).unwrap();
            write_launcher_tools(&bin);
            let turn_host = root.join("turn-host.mjs");
            fs::write(&turn_host, "// test host\n").unwrap();
            let analyst = bin.join("world-agent-tool-stdio");
            write_executable(&analyst);
            Self {
                root,
                bin,
                turn_host,
                analyst,
            }
        }

        fn executable(&self, name: &str) -> PathBuf {
            let path = self.bin.join(name);
            write_executable(&path);
            path
        }

        fn config(&self) -> DesktopAnalystConfig {
            let mut config = DesktopAnalystConfig::new(&self.turn_host);
            config.node_program = PathBuf::from("node");
            config.pi_program = None;
            config.analyst_program = Some(self.analyst.clone());
            config
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn write_launcher_tools(directory: &Path) {
        fs::create_dir_all(directory).unwrap();
        write_executable(&directory.join("bash"));
        write_executable(&directory.join("dirname"));
    }

    fn write_executable(path: &Path) {
        fs::write(path, "#!/bin/sh\nexit 0\n").unwrap();
        #[cfg(unix)]
        set_mode(path, 0o755);
    }

    #[cfg(unix)]
    fn write_non_executable(path: &Path) {
        fs::write(path, "#!/bin/sh\nexit 0\n").unwrap();
        set_mode(path, 0o644);
    }

    #[cfg(unix)]
    fn set_mode(path: &Path, mode: u32) {
        use std::os::unix::fs::PermissionsExt;

        let mut permissions = fs::metadata(path).unwrap().permissions();
        permissions.set_mode(mode);
        fs::set_permissions(path, permissions).unwrap();
    }

    #[test]
    fn bare_node_and_default_pi_resolve_from_path() {
        let fixture = Fixture::new();
        let node = fixture.executable("node");
        let pi = fixture.executable("pi");
        let readiness = check_with_environment(
            fixture.config(),
            Some(fixture.bin.clone().into_os_string()),
            Some(fixture.root.clone()),
        );
        let config = readiness.config().expect("runtime should be ready");
        assert_eq!(config.node_program, node);
        assert_eq!(config.pi_program.as_deref(), Some(pi.as_path()));
        assert_eq!(
            config.analyst_program.as_deref(),
            Some(fixture.analyst.as_path())
        );
    }

    #[test]
    fn relative_path_entries_are_resolved_for_node_and_pi_only() {
        let fixture = Fixture::new();
        let node = fixture.executable("node");
        let pi = fixture.executable("pi");
        let launcher_bin = fixture.root.join("launcher-bin");
        write_launcher_tools(&launcher_bin);
        let search_path = env::join_paths([PathBuf::from("bin"), launcher_bin]).unwrap();
        let readiness = check_with_environment(
            fixture.config(),
            Some(search_path),
            Some(fixture.root.clone()),
        );
        let config = readiness
            .config()
            .expect("relative PATH entry should resolve Node and Pi");
        assert_eq!(config.node_program, node);
        assert_eq!(config.pi_program.as_deref(), Some(pi.as_path()));
    }

    #[test]
    fn relative_path_entries_do_not_satisfy_launcher_dependencies() {
        let fixture = Fixture::new();
        let node = fixture.executable("custom-node");
        let pi = fixture.executable("custom-pi");
        let mut config = fixture.config();
        config.node_program = node;
        config.pi_program = Some(pi);
        let readiness = check_with_environment(
            config,
            Some(PathBuf::from("bin").into_os_string()),
            Some(fixture.root.clone()),
        );
        assert_eq!(
            readiness.issue().unwrap().kind(),
            DesktopAnalystRuntimeIssueKind::LauncherUnavailable
        );
    }

    #[test]
    fn explicit_program_overrides_only_need_launcher_path() {
        let fixture = Fixture::new();
        let node = fixture.executable("custom-node");
        let pi = fixture.executable("custom-pi");
        let mut config = fixture.config();
        config.node_program = node.clone();
        config.pi_program = Some(pi.clone());
        let readiness = check_with_environment(
            config,
            Some(fixture.bin.clone().into_os_string()),
            Some(fixture.root.clone()),
        );
        let config = readiness
            .config()
            .expect("explicit executables should be ready");
        assert_eq!(config.node_program, node);
        assert_eq!(config.pi_program.as_deref(), Some(pi.as_path()));
    }

    #[test]
    fn missing_bash_is_reported_before_ready() {
        let fixture = Fixture::new();
        let node = fixture.executable("custom-node");
        let pi = fixture.executable("custom-pi");
        let isolated = fixture.root.join("no-launcher");
        fs::create_dir_all(&isolated).unwrap();
        let mut config = fixture.config();
        config.node_program = node;
        config.pi_program = Some(pi);
        let readiness = check_with_environment(
            config,
            Some(isolated.into_os_string()),
            Some(fixture.root.clone()),
        );
        let issue = readiness.issue().expect("missing bash should fail");
        assert_eq!(
            issue.kind(),
            DesktopAnalystRuntimeIssueKind::LauncherUnavailable
        );
        assert!(issue.message().contains("`bash`"));
    }

    #[test]
    fn missing_dirname_is_reported_before_ready() {
        let fixture = Fixture::new();
        let node = fixture.executable("custom-node");
        let pi = fixture.executable("custom-pi");
        let isolated = fixture.root.join("bash-only");
        fs::create_dir_all(&isolated).unwrap();
        write_executable(&isolated.join("bash"));
        let mut config = fixture.config();
        config.node_program = node;
        config.pi_program = Some(pi);
        let readiness = check_with_environment(
            config,
            Some(isolated.into_os_string()),
            Some(fixture.root.clone()),
        );
        let issue = readiness.issue().expect("missing dirname should fail");
        assert_eq!(
            issue.kind(),
            DesktopAnalystRuntimeIssueKind::LauncherUnavailable
        );
        assert!(issue.message().contains("`dirname`"));
    }

    #[test]
    fn finder_like_limited_path_reports_missing_node() {
        let fixture = Fixture::new();
        let limited = fixture.root.join("finder-bin");
        write_launcher_tools(&limited);
        let readiness = check_with_environment(
            fixture.config(),
            Some(limited.into_os_string()),
            Some(fixture.root.clone()),
        );
        let issue = readiness.issue().expect("limited PATH should fail");
        assert_eq!(
            issue.kind(),
            DesktopAnalystRuntimeIssueKind::NodeUnavailable
        );
        assert!(issue.message().contains("WORLD_MACHINE_NODE_PROGRAM"));
    }

    #[test]
    fn missing_pi_is_reported_after_node_resolves() {
        let fixture = Fixture::new();
        fixture.executable("node");
        let readiness = check_with_environment(
            fixture.config(),
            Some(fixture.bin.clone().into_os_string()),
            Some(fixture.root.clone()),
        );
        let issue = readiness.issue().expect("missing Pi should fail");
        assert_eq!(issue.kind(), DesktopAnalystRuntimeIssueKind::PiUnavailable);
        assert!(issue.message().contains("PI_PROGRAM"));
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[test]
    fn execute_bit_for_another_class_does_not_imply_current_user_access() {
        if effective_access::effective_uid() == 0 {
            return;
        }

        let fixture = Fixture::new();
        let node = fixture.bin.join("node");
        fs::write(&node, "#!/bin/sh\nexit 0\n").unwrap();
        set_mode(&node, 0o010);
        fixture.executable("pi");
        let readiness = check_with_environment(
            fixture.config(),
            Some(fixture.bin.clone().into_os_string()),
            Some(fixture.root.clone()),
        );
        assert_eq!(
            readiness.issue().unwrap().kind(),
            DesktopAnalystRuntimeIssueKind::NodeUnavailable
        );
    }

    #[cfg(unix)]
    #[test]
    fn non_executable_node_is_rejected() {
        let fixture = Fixture::new();
        write_non_executable(&fixture.bin.join("node"));
        fixture.executable("pi");
        let readiness = check_with_environment(
            fixture.config(),
            Some(fixture.bin.clone().into_os_string()),
            Some(fixture.root.clone()),
        );
        assert_eq!(
            readiness.issue().unwrap().kind(),
            DesktopAnalystRuntimeIssueKind::NodeUnavailable
        );
    }

    #[cfg(unix)]
    #[test]
    fn non_executable_pi_is_rejected() {
        let fixture = Fixture::new();
        fixture.executable("node");
        write_non_executable(&fixture.bin.join("pi"));
        let readiness = check_with_environment(
            fixture.config(),
            Some(fixture.bin.clone().into_os_string()),
            Some(fixture.root.clone()),
        );
        assert_eq!(
            readiness.issue().unwrap().kind(),
            DesktopAnalystRuntimeIssueKind::PiUnavailable
        );
    }

    #[cfg(unix)]
    #[test]
    fn non_executable_tool_host_is_rejected() {
        let fixture = Fixture::new();
        fixture.executable("node");
        fixture.executable("pi");
        set_mode(&fixture.analyst, 0o644);
        let readiness = check_with_environment(
            fixture.config(),
            Some(fixture.bin.clone().into_os_string()),
            Some(fixture.root.clone()),
        );
        assert_eq!(
            readiness.issue().unwrap().kind(),
            DesktopAnalystRuntimeIssueKind::AnalystProgramUnavailable
        );
    }

    #[test]
    fn missing_turn_host_is_runtime_incomplete() {
        let fixture = Fixture::new();
        fixture.executable("node");
        fixture.executable("pi");
        fs::remove_file(&fixture.turn_host).unwrap();
        let readiness = check_with_environment(
            fixture.config(),
            Some(fixture.bin.clone().into_os_string()),
            Some(fixture.root.clone()),
        );
        assert_eq!(
            readiness.issue().unwrap().kind(),
            DesktopAnalystRuntimeIssueKind::RuntimeIncomplete
        );
    }
}
