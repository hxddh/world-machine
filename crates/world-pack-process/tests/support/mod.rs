//! The scripted stand-in Pack (`src/bin/world-pack-fixture.rs`) for these
//! tests, on every platform.

#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};
use world_pack_process::ProcessPack;
use world_pack_protocol::{PackManifest, PackRuntimeManifest};

/// The setting that names the fixture's script.
pub const FIXTURE_SCRIPT: &str = "WORLD_MACHINE_FIXTURE_SCRIPT";

pub fn fixture_program() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_world-pack-fixture"))
}

/// Steps that answer each request with one of `responses`, in order.
pub fn respond(responses: &[String]) -> Vec<String> {
    responses
        .iter()
        .map(|response| format!("respond\t{response}"))
        .collect()
}

/// Writes `manifest`, its process made the fixture running `steps`, into
/// `root`, and loads it told where its script is.
pub fn fixture_pack(root: &Path, mut manifest: PackManifest, steps: &[String]) -> ProcessPack {
    let script = root.join("fixture.script");
    fs::write(&script, steps.join("\n")).unwrap();
    manifest.runtime = PackRuntimeManifest::Process {
        command: fixture_program().to_string_lossy().into_owned(),
        args: Vec::new(),
    };
    let manifest_path = root.join("fixture.world-pack.json");
    fs::write(&manifest_path, manifest.to_json_pretty().unwrap()).unwrap();
    ProcessPack::load(&manifest_path)
        .unwrap()
        .with_settings([(FIXTURE_SCRIPT, script.to_string_lossy().into_owned())])
        .unwrap()
}
