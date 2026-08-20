//! Resolve the installed analyst runtime without leaking process, PATH, or settings I/O into GPUI.

use std::env;
use std::fs::File;
use std::path::{Path, PathBuf};
use world_machine_desktop::analyst_readiness::{
    self, DesktopAnalystRuntimeIssue, DesktopAnalystRuntimeIssueKind,
    DesktopAnalystRuntimeReadiness,
};
use world_machine_desktop::analyst_session::DesktopAnalystConfig;
use world_machine_desktop::analyst_settings::{
    self, DesktopAnalystProgramSelections, DesktopAnalystSettings,
};

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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum AnalystRuntimeProgram {
    Node,
    Pi,
}

#[derive(Clone, Debug)]
pub(crate) struct AnalystRuntimeStatus {
    pub readiness: DesktopAnalystRuntimeReadiness,
    pub selections: Option<DesktopAnalystProgramSelections>,
    pub settings: Option<DesktopAnalystSettings>,
}

impl AnalystRuntimeStatus {
    fn unavailable(message: impl Into<String>) -> Self {
        Self {
            readiness: DesktopAnalystRuntimeReadiness::unavailable(
                DesktopAnalystRuntimeIssueKind::RuntimeUnavailable,
                message,
            ),
            selections: None,
            settings: None,
        }
    }
}

pub(crate) fn discover() -> DesktopAnalystRuntimeReadiness {
    discover_status().readiness
}

pub(crate) fn discover_status() -> AnalystRuntimeStatus {
    let settings_root = match analyst_settings::application_support_root() {
        Ok(root) => root,
        Err(error) => return AnalystRuntimeStatus::unavailable(error.to_string()),
    };
    let settings = match analyst_settings::load(&settings_root) {
        Ok(settings) => settings,
        Err(error) => return AnalystRuntimeStatus::unavailable(error.to_string()),
    };
    let selections = analyst_settings::selections(
        &settings,
        env_path(NODE_PROGRAM_ENV),
        env_path(PI_PROGRAM_ENV),
    );
    let readiness = match discover_config(&selections) {
        Ok(config) => analyst_readiness::check(config),
        Err(issue) => DesktopAnalystRuntimeReadiness::Unavailable { issue },
    };
    AnalystRuntimeStatus {
        readiness,
        selections: Some(selections),
        settings: Some(settings),
    }
}

pub(crate) fn save_program(program: AnalystRuntimeProgram, path: PathBuf) -> Result<(), String> {
    if !path.is_absolute() {
        return Err(format!(
            "World Analyst executable selection must be an absolute path: {}",
            path.display()
        ));
    }
    if environment_controls(program) {
        return Err(format!(
            "{} is controlled by an environment override; remove that override before saving a user path",
            program_label(program)
        ));
    }
    let root = analyst_settings::application_support_root().map_err(|error| error.to_string())?;
    let result = match program {
        AnalystRuntimeProgram::Node => analyst_settings::save_node_program(&root, path),
        AnalystRuntimeProgram::Pi => analyst_settings::save_pi_program(&root, path),
    };
    result.map_err(|error| error.to_string())
}

pub(crate) fn clear_program(program: AnalystRuntimeProgram) -> Result<(), String> {
    let root = analyst_settings::application_support_root().map_err(|error| error.to_string())?;
    let result = match program {
        AnalystRuntimeProgram::Node => analyst_settings::clear_node_program(&root),
        AnalystRuntimeProgram::Pi => analyst_settings::clear_pi_program(&root),
    };
    result.map_err(|error| error.to_string())
}

fn environment_controls(program: AnalystRuntimeProgram) -> bool {
    match program {
        AnalystRuntimeProgram::Node => env_path(NODE_PROGRAM_ENV).is_some(),
        AnalystRuntimeProgram::Pi => env_path(PI_PROGRAM_ENV).is_some(),
    }
}

fn program_label(program: AnalystRuntimeProgram) -> &'static str {
    match program {
        AnalystRuntimeProgram::Node => "Node",
        AnalystRuntimeProgram::Pi => "Pi",
    }
}

fn discover_config(
    selections: &DesktopAnalystProgramSelections,
) -> Result<DesktopAnalystConfig, DesktopAnalystRuntimeIssue> {
    let root = discover_root()?;
    validate_root(&root)?;

    let mut config = DesktopAnalystConfig::new(root.join(TURN_HOST));
    config.node_program = selections.node.program.clone();
    config.pi_program = Some(selections.pi.program.clone());
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
        let Ok(metadata) = path.metadata() else {
            return Err(DesktopAnalystRuntimeIssue::new(
                DesktopAnalystRuntimeIssueKind::RuntimeIncomplete,
                format!(
                    "World Machine analyst runtime is incomplete: missing {}",
                    path.display()
                ),
            ));
        };
        if !metadata.is_file() || metadata.len() == 0 {
            return Err(DesktopAnalystRuntimeIssue::new(
                DesktopAnalystRuntimeIssueKind::RuntimeIncomplete,
                format!(
                    "World Machine analyst runtime is incomplete: {} is not a non-empty file",
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
    use world_machine_desktop::analyst_settings::DesktopAnalystProgramSource;

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
    fn persisted_selection_enters_existing_session_config() {
        let selections = DesktopAnalystProgramSelections {
            node: world_machine_desktop::analyst_settings::DesktopAnalystProgramSelection {
                program: PathBuf::from("/persisted/node"),
                source: DesktopAnalystProgramSource::Persisted,
            },
            pi: world_machine_desktop::analyst_settings::DesktopAnalystProgramSelection {
                program: PathBuf::from("/persisted/pi"),
                source: DesktopAnalystProgramSource::Persisted,
            },
        };
        let mut config = DesktopAnalystConfig::new("/tmp/turn-host.mjs");
        config.node_program = selections.node.program.clone();
        config.pi_program = Some(selections.pi.program.clone());
        assert_eq!(config.node_program, PathBuf::from("/persisted/node"));
        assert_eq!(config.pi_program, Some(PathBuf::from("/persisted/pi")));
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
