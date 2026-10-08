#![forbid(unsafe_code)]

use std::env;
use std::error::Error;
use std::path::PathBuf;
use world_pack_server::{manifest_for_current_exe, serve_stdio, write_current_exe_bundle};

fn main() -> Result<(), Box<dyn Error>> {
    let registration = tiny_society::tiny_society_registration();
    let args = env::args_os().skip(1).collect::<Vec<_>>();
    if args.len() == 1 && args[0] == "--print-manifest" {
        let manifest = manifest_for_current_exe(&registration.descriptor)?;
        println!("{}", manifest.to_json_pretty()?);
        return Ok(());
    }
    if args.len() == 2 && args[0] == "--write-bundle" {
        let destination = PathBuf::from(&args[1]);
        write_current_exe_bundle(&registration.descriptor, destination)?;
        return Ok(());
    }
    if !args.is_empty() {
        return Err("unsupported arguments; run without arguments as a Pack server, use --print-manifest, or use --write-bundle PATH"
            .to_string()
            .into());
    }
    // People speak in a language model's words only when the player switched
    // the World voice on; otherwise they answer in the harbour's own.
    let voice = world_voice::Voice::from_process()?;
    let registration = if voice == world_voice::Voice::None {
        registration
    } else {
        tiny_society::tiny_society_registration_with_listener(std::sync::Arc::new(move || {
            voice.listener_or_own_ears()
        }))
    };
    serve_stdio(registration)?;
    Ok(())
}
