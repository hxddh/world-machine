use pocket_universe::{
    pocket_universe_descriptor, pocket_universe_registration,
    pocket_universe_registration_with_agent_runtime_profile,
};
use std::env;
use std::error::Error;
use std::path::PathBuf;
use world_pack_server::{manifest_for_current_exe, serve_stdio, write_current_exe_bundle};
use world_pi_rpc::{PiCommand, PiRpcRuntime, ProcessPiRpcTransport};

const MIND_ENV: &str = "WORLD_MACHINE_POCKET_UNIVERSE_MIND";
const PI_PROGRAM_ENV: &str = "WORLD_MACHINE_PI_PROGRAM";

fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args_os().skip(1).collect::<Vec<_>>();
    let descriptor = pocket_universe_descriptor();
    if args.len() == 1 && args[0] == "--print-manifest" {
        let manifest = manifest_for_current_exe(&descriptor)?;
        println!("{}", manifest.to_json_pretty()?);
        return Ok(());
    }
    if args.len() == 2 && args[0] == "--write-bundle" {
        let destination = PathBuf::from(&args[1]);
        write_current_exe_bundle(&descriptor, destination)?;
        return Ok(());
    }
    if !args.is_empty() {
        return Err("unsupported arguments; run without arguments as a Pack server, use --print-manifest, or use --write-bundle PATH"
            .to_string()
            .into());
    }

    match env::var(MIND_ENV).as_deref().unwrap_or("deterministic") {
        "deterministic" => serve_stdio(pocket_universe_registration())?,
        "pi" => {
            let program = env::var(PI_PROGRAM_ENV).unwrap_or_else(|_| "pi".into());
            let command = PiCommand::decision_only(program);
            serve_stdio(pocket_universe_registration_with_agent_runtime_profile(
                move || PiRpcRuntime::new(ProcessPiRpcTransport::new(command.clone())),
                "pi",
            ))?;
        }
        other => {
            return Err(format!(
                "unsupported {MIND_ENV} value {other:?}; expected deterministic or pi"
            )
            .into())
        }
    }
    Ok(())
}
