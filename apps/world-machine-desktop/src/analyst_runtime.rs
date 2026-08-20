//! Resolve the installed analyst runtime without leaking process or PATH details into GPUI code.

use std::env;
use std::fs::File;
use std::path::{Path, PathBuf};
use world_machine_desktop::analyst_readiness::{
    self, DesktopAnalystRuntimeIssue, DesktopAnalystRuntimeIssueKind,
    DesktopAnalystRuntimeReadiness,
};
use world_machine_desktop::analyst_session::DesktopAnalystConfig;

const RUNTIME_ROOT_ENV: &str = "WORLD_MACHINE_ANALYST_RUNTIME_ROOT";
const NODE_PROGRAM_ENV: &str = "WORLD_MACHINE_NODE_PROGRAM";
const PI_PROGRAM_ENV: &str = "PI_PROGRAM";
const ANALYST_PROGRAM_ENV: &str = "WORLD_MACHINE_ANALYST_PROGRAM";
const PROVIDER_ENV: &str = "WORLD_MACHINE_ANALYST_PROVIDER";
const MODEL_ENV: &str = "WORLD_MACHINE_ANALYST_MODEL";
const THINKING_ENV: &str = "WORLD_MACHINE_ANALYST_THINKING";
const TURN_TIMEOUT_MS: u64 = 120_000;

const TURN_HOST: &str = "integrations/pi/world-machine-analyst-turn-host.mjs";
const RPC_MODULE: &str = "integrations/pi/world-machine-analyst-rpc.mjs";
const EXTENSION: &str = "integrations/pi/world-machine-analyst.mjs";
const CLIENT_MODULE: &str = "integrations/pi/world-machine-analyst-client.mjs";
const LAUNCHER: &str = "scripts/run-pi-analyst.sh";
const BUNDLED_ANALYST_PROGRAM: &str = "bin/world-agent-tool-stdio";

pub(crate) fn discover() -> DesktopAnalystRuntimeReadiness {
    match discover_config() {
        Ok(config) => analyst_readiness::check(config),
        Err(issue) => DesktopAnalystRuntimeReadiness::Unavailable { issue },
    }
}

fn discover_config() -> Result<DesktopAnalystConfig, DesktopAnalystRuntimeIssue> {
    let root = discover_root()?;
    validate_root(&root)?;

    let mut config = DesktopAnalystConfig::new(root.join(TURN_HOST));
    config.node_program = env_path(NODE_PROGRAM_ENV).unwrap_or_else(|| PathBuf::from("node"));
    config.pi_program = env_path(PI_PROGRAM_ENV);
    config.analyst_program =
        Some(env_path(ANALYST_PROGRAM_ENV).unwrap_or_else(|| root.join(BUNDLED_ANALYST_PROGRAM)));
    config.provider = env_value(PROVIDER_ENV);
    config.model = env_value(MODEL_ENV);
    config.thinking = env_value(THINKING_ENV);
    config.timeout_ms = Some(TURN_TIMEOUT_MS);
    Ok(config)
}

fn discover_root() -> Result<PathBuf, DesktopAnalystRuntimeIssue> {
    if let Some(root) = env_path(RUNTIME_ROOT_ENV) {
        return resolve_root_override(root, env::current_dir().ok().as_deref());
    }

    if let Ok(executable) = env::current_exe() {
        if let Some(root) = bundled_runtime_root(&executable) {
            if root.join(TURN_HOST).is_file() {
                return Ok(root);
            }
        }
    }

    if let Ok(current_dir) = env::current_dir() {
        if current_dir.join(TURN_HOST).is_file() && current_dir.join(LAUNCHER).is_file() {
            return Ok(current_dir);
        }
    }

    Err(DesktopAnalystRuntimeIssue::new(
        DesktopAnalystRuntimeIssueKind::RuntimeUnavailable,
        format!(
            "World Machine analyst runtime is unavailable. Expected the bundled `Analyst Runtime` resource or set {RUNTIME_ROOT_ENV}."
        ),
    ))
}

fn resolve_root_override(
    root: PathBuf,
    current_dir: Option<&Path>,
) -> Result<PathBuf, DesktopAnalystRuntimeIssue> {
    if root.is_absolute() {
        return Ok(root);
    }
    current_dir.map(|directory| directory.join(root)).ok_or_else(|| {
        DesktopAnalystRuntimeIssue::new(
            DesktopAnalystRuntimeIssueKind::RuntimeUnavailable,
            format!(
                "{RUNTIME_ROOT_ENV} is relative, but the current directory could not be resolved. Set it to an absolute analyst runtime path."
            ),
        )
    })
}

fn bundled_runtime_root(executable: &Path) -> Option<PathBuf> {
    let macos = executable.parent()?;
    if macos.file_name()?.to_str()? != "MacOS" {
        return None;
    }
    let contents = macos.parent()?;
    if contents.file_name()?.to_str()? != "Contents" {
        return None;
    }
    Some(contents.join("Resources").join("Analyst Runtime"))
}

fn validate_root(root: &Path) -> Result<(), DesktopAnalystRuntimeIssue> {
    for relative in [TURN_HOST, RPC_MODULE, EXTENSION, CLIENT_MODULE, LAUNCHER] {
        let path = root.join(relative);
        if !path.is_file() {
            return Err(DesktopAnalystRuntimeIssue::new(
                DesktopAnalystRuntimeIssueKind::RuntimeIncomplete,
                format!(
                    "World Machine analyst runtime is incomplete: missing {}",
                    path.display()
                ),
            ));
        }
        if File::open(&path).is_err() {
            return Err(DesktopAnalystRuntimeIssue::new(
                DesktopAnalystRuntimeIssueKind::RuntimeIncomplete,
                format!(
                    "World Machine analyst runtime is incomplete: {} is not readable by the current user",
                    path.display()
                ),
            ));
        }
    }
    Ok(())
}

fn env_path(name: &str) -> Option<PathBuf> {
    env::var_os(name)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

fn env_value(name: &str) -> Option<String> {
    env::var(name)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_runtime_root_is_derived_from_app_executable() {
        assert_eq!(
            bundled_runtime_root(Path::new(
                "/Applications/World Machine.app/Contents/MacOS/world-machine-desktop"
            )),
            Some(PathBuf::from(
                "/Applications/World Machine.app/Contents/Resources/Analyst Runtime"
            ))
        );
        assert_eq!(
            bundled_runtime_root(Path::new("/tmp/world-machine-desktop")),
            None
        );
    }

    #[test]
    fn relative_runtime_root_override_is_normalized() {
        assert_eq!(
            resolve_root_override(
                PathBuf::from("dev/Analyst Runtime"),
                Some(Path::new("/tmp/world-machine"))
            )
            .unwrap(),
            PathBuf::from("/tmp/world-machine/dev/Analyst Runtime")
        );
    }

    #[test]
    fn relative_runtime_root_requires_current_directory() {
        let issue = resolve_root_override(PathBuf::from("Analyst Runtime"), None).unwrap_err();
        assert_eq!(
            issue.kind(),
            DesktopAnalystRuntimeIssueKind::RuntimeUnavailable
        );
        assert!(issue.message().contains("absolute analyst runtime path"));
    }

    #[test]
    fn absolute_runtime_root_override_is_preserved() {
        let root = PathBuf::from("/tmp/Analyst Runtime");
        assert_eq!(resolve_root_override(root.clone(), None).unwrap(), root);
    }

    #[test]
    fn runtime_layout_keeps_rpc_launcher_relative_contract() {
        let root = PathBuf::from("/tmp/Analyst Runtime");
        assert_eq!(
            root.join(TURN_HOST),
            PathBuf::from(
                "/tmp/Analyst Runtime/integrations/pi/world-machine-analyst-turn-host.mjs"
            )
        );
        assert_eq!(
            root.join(LAUNCHER),
            PathBuf::from("/tmp/Analyst Runtime/scripts/run-pi-analyst.sh")
        );
        assert_eq!(
            root.join(BUNDLED_ANALYST_PROGRAM),
            PathBuf::from("/tmp/Analyst Runtime/bin/world-agent-tool-stdio")
        );
    }
}
