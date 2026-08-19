use std::env;
use std::path::{Path, PathBuf};
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

pub(crate) fn discover() -> Result<DesktopAnalystConfig, String> {
    let root = discover_root()?;
    validate_root(&root)?;

    let mut config = DesktopAnalystConfig::new(root.join(TURN_HOST));
    config.node_program = env_path(NODE_PROGRAM_ENV).unwrap_or_else(|| PathBuf::from("node"));
    config.pi_program = env_path(PI_PROGRAM_ENV);
    config.analyst_program = Some(
        env_path(ANALYST_PROGRAM_ENV).unwrap_or_else(|| root.join(BUNDLED_ANALYST_PROGRAM)),
    );
    config.provider = env_value(PROVIDER_ENV);
    config.model = env_value(MODEL_ENV);
    config.thinking = env_value(THINKING_ENV);
    config.timeout_ms = Some(TURN_TIMEOUT_MS);

    let analyst_program = config
        .analyst_program
        .as_deref()
        .expect("analyst program is always resolved");
    if analyst_program.components().count() > 1 && !analyst_program.is_file() {
        return Err(format!(
            "World Machine analyst executable not found: {}",
            analyst_program.display()
        ));
    }

    Ok(config)
}

fn discover_root() -> Result<PathBuf, String> {
    if let Some(root) = env_path(RUNTIME_ROOT_ENV) {
        return Ok(root);
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

    Err(format!(
        "World Machine analyst runtime is unavailable. Expected a bundled `Analyst Runtime` resource or set {RUNTIME_ROOT_ENV}."
    ))
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

fn validate_root(root: &Path) -> Result<(), String> {
    for relative in [TURN_HOST, RPC_MODULE, EXTENSION, CLIENT_MODULE, LAUNCHER] {
        let path = root.join(relative);
        if !path.is_file() {
            return Err(format!(
                "World Machine analyst runtime is incomplete: missing {}",
                path.display()
            ));
        }
    }
    Ok(())
}

fn env_path(name: &str) -> Option<PathBuf> {
    env::var_os(name).filter(|value| !value.is_empty()).map(PathBuf::from)
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
