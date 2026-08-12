use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::{self, Command};
use std::time::{SystemTime, UNIX_EPOCH};
use tiny_society::{TINY_SOCIETY_PACK_ID, TINY_SOCIETY_PACK_VERSION};
use world_host::WorldRegistry;
use world_pack_catalog::PackCatalog;
use world_pack_protocol::{PackManifest, PackRuntimeManifest};

fn temp_dir() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let root = env::temp_dir().join(format!(
        "world-machine-tiny-society-external-{}-{nonce}",
        process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    root
}

#[test]
fn tiny_society_can_be_installed_and_run_as_a_real_external_pack() {
    let binary = env!("CARGO_BIN_EXE_tiny-society-pack");
    let output = Command::new(binary)
        .arg("--print-manifest")
        .output()
        .unwrap();
    assert!(output.status.success());
    let manifest_json = String::from_utf8(output.stdout).unwrap();
    let manifest = PackManifest::from_json(&manifest_json).unwrap();
    assert_eq!(manifest.descriptor.pack.id, TINY_SOCIETY_PACK_ID);
    assert_eq!(manifest.descriptor.pack.version, TINY_SOCIETY_PACK_VERSION);
    match &manifest.runtime {
        PackRuntimeManifest::Process { command, args } => {
            assert!(PathBuf::from(command).is_absolute());
            assert!(args.is_empty());
        }
    }

    let root = temp_dir();
    let manifest_path = root.join("tiny-society.world-pack.json");
    fs::write(&manifest_path, manifest_json).unwrap();
    let mut catalog = PackCatalog::open(root.join("catalog.json")).unwrap();
    let installed = catalog.install_manifest(&manifest_path).unwrap();
    assert!(installed.managed);
    fs::remove_file(&manifest_path).unwrap();

    let source = catalog.trusted_source().unwrap();
    let mut registry = WorldRegistry::new();
    registry.install_source(&source).unwrap();

    let mut session = registry.create(TINY_SOCIETY_PACK_ID).unwrap();
    assert_eq!(session.pack().id, TINY_SOCIETY_PACK_ID);
    assert_eq!(session.pack().version, TINY_SOCIETY_PACK_VERSION);
    let initial = session.snapshot();
    let advanced = session.advance_background(1).unwrap();
    assert!(advanced.world_time >= initial.world_time);

    let archive = session.archive().unwrap().unwrap();
    assert_eq!(archive.pack, session.pack());
    drop(session);

    let reopened = registry.open_archive(&archive).unwrap();
    assert_eq!(reopened.pack(), archive.pack);
    assert_eq!(reopened.snapshot().world_time, archive.world_time);
}

#[test]
fn tiny_society_portable_bundle_runs_after_the_bundle_is_removed() {
    let binary = env!("CARGO_BIN_EXE_tiny-society-pack");
    let root = temp_dir();
    let bundle_path = root.join("tiny-society.worldpack");
    let status = Command::new(binary)
        .arg("--write-bundle")
        .arg(&bundle_path)
        .status()
        .unwrap();
    assert!(status.success());

    let mut catalog = PackCatalog::open(root.join("catalog.json")).unwrap();
    let installed = catalog.install_bundle(&bundle_path).unwrap();
    assert!(installed.managed);
    assert_ne!(
        installed.command_path,
        PathBuf::from(binary).canonicalize().unwrap()
    );
    fs::remove_file(&bundle_path).unwrap();

    let source = catalog.trusted_source().unwrap();
    let mut registry = WorldRegistry::new();
    registry.install_source(&source).unwrap();
    let mut session = registry.create(TINY_SOCIETY_PACK_ID).unwrap();
    let initial = session.snapshot();
    let advanced = session.advance_background(1).unwrap();
    assert!(advanced.world_time >= initial.world_time);
    let archive = session.archive().unwrap().unwrap();
    drop(session);
    let reopened = registry.open_archive(&archive).unwrap();
    assert_eq!(reopened.pack(), archive.pack);
    assert_eq!(reopened.snapshot().world_time, archive.world_time);
}

#[test]
fn corrupt_portable_bundle_leaves_no_catalog_or_managed_pack() {
    let binary = env!("CARGO_BIN_EXE_tiny-society-pack");
    let root = temp_dir();
    let bundle_path = root.join("corrupt.worldpack");
    let status = Command::new(binary)
        .arg("--write-bundle")
        .arg(&bundle_path)
        .status()
        .unwrap();
    assert!(status.success());

    // Preserve a valid header/layout while changing one payload byte so install
    // reaches managed staging and fails on the streaming SHA-256 verification.
    let mut bytes = fs::read(&bundle_path).unwrap();
    let header_len = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
    let program_offset = 12 + header_len;
    assert!(program_offset < bytes.len());
    bytes[program_offset] ^= 0x01;
    fs::write(&bundle_path, bytes).unwrap();

    let mut catalog = PackCatalog::open(root.join("catalog.json")).unwrap();
    assert!(catalog.install_bundle(&bundle_path).is_err());
    assert!(catalog.entries().is_empty());
    let installed_root = root.join("Installed");
    let leftovers = fs::read_dir(installed_root)
        .map(|entries| entries.count())
        .unwrap_or(0);
    assert_eq!(leftovers, 0);
}
