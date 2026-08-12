use std::env;
use std::error::Error;
use world_pack_server::{manifest_for_current_exe, serve_stdio};

fn main() -> Result<(), Box<dyn Error>> {
    let registration = tiny_society::tiny_society_registration();
    let args = env::args_os().skip(1).collect::<Vec<_>>();
    if args.len() == 1 && args[0] == "--print-manifest" {
        let manifest = manifest_for_current_exe(&registration.descriptor)?;
        println!("{}", manifest.to_json_pretty()?);
        return Ok(());
    }
    if !args.is_empty() {
        return Err(
            "unsupported arguments; run without arguments as a Pack server or use --print-manifest"
                .to_string()
                .into(),
        );
    }
    serve_stdio(registration)?;
    Ok(())
}
