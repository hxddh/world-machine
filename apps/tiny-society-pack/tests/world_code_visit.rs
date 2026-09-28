//! A `wm2:` World code, which carries a checkpoint and only the season
//! since, opens through Tiny Society running as a process Pack, as the
//! desktop app runs it. A Pack from before checkpoints is refused it with a
//! plain reason, and still opens a whole history.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{self, Command};
use std::time::{SystemTime, UNIX_EPOCH};
use tiny_society::TINY_SOCIETY_PACK_ID;
use world_document::WorldDocument;
use world_host::WorldRegistry;
use world_library::{decode_world_code, encode_world_code, WorldCodeError, WorldVisit};
use world_pack_catalog::PackCatalog;
use world_pack_process::{ProcessPack, ProcessPackSource};
use world_pack_protocol::{PackManifest, PACK_PROTOCOL_VERSION_V3, PACK_PROTOCOL_VERSION_V4};
use world_projection::ProjectionIntent::InvokeCommand;

const PASS: &str = "tiny-society.let-day-pass";
/// Past two of Tiny Society's thirty-day seasons.
const DAYS: u64 = 70;

fn temp_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let root = env::temp_dir().join(format!(
        "world-machine-code-visit-{label}-{}-{nanos}",
        process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    root
}

/// A registry holding the Tiny Society Pack binary as a process Pack,
/// installed through the catalog as the desktop app installs it.
fn installed_registry(root: &Path) -> WorldRegistry {
    let manifest_path = write_manifest(root, PACK_PROTOCOL_VERSION_V4);
    let mut catalog = PackCatalog::open(root.join("catalog.json")).unwrap();
    catalog.install_manifest(&manifest_path).unwrap();
    let mut registry = WorldRegistry::new();
    registry
        .install_source(&catalog.trusted_source().unwrap())
        .unwrap();
    registry
}

/// A registry holding the Pack binary as a process Pack whose manifest
/// says it speaks `protocol_version`, as a Pack built before checkpoints
/// does.
fn registry_speaking(root: &Path, protocol_version: u32) -> WorldRegistry {
    let pack = ProcessPack::load(write_manifest(root, protocol_version)).unwrap();
    assert_eq!(pack.protocol_version, protocol_version);
    let mut registry = WorldRegistry::new();
    registry
        .install_source(&ProcessPackSource::from_packs(vec![pack]))
        .unwrap();
    registry
}

fn write_manifest(root: &Path, protocol_version: u32) -> PathBuf {
    let binary = env!("CARGO_BIN_EXE_tiny-society-pack");
    let output = Command::new(binary)
        .arg("--print-manifest")
        .output()
        .unwrap();
    assert!(output.status.success());
    let mut manifest = PackManifest::from_json(&String::from_utf8(output.stdout).unwrap()).unwrap();
    assert_eq!(manifest.protocol_version, PACK_PROTOCOL_VERSION_V4);
    manifest.protocol_version = protocol_version;
    let manifest_path = root.join(format!("tiny-society-v{protocol_version}.world-pack.json"));
    fs::write(&manifest_path, manifest.to_json_pretty().unwrap()).unwrap();
    manifest_path
}

/// A World played in-process for a few seasons, as its owner's file keeps
/// it: with its checkpoint at the start of its latest season.
fn played() -> WorldDocument {
    let mut registry = WorldRegistry::new();
    registry
        .register(tiny_society::tiny_society_registration())
        .unwrap();
    let mut session = registry.create(TINY_SOCIETY_PACK_ID).unwrap();
    let mut snapshot = session.snapshot();
    for _ in 0..DAYS {
        if let Some(answer) = snapshot
            .commands
            .iter()
            .find(|c| c.question.is_some() && c.unavailable.is_none() && c.id != PASS)
        {
            let _ = session.handle(InvokeCommand(answer.id.clone()));
        }
        snapshot = session.handle(InvokeCommand(PASS.into())).unwrap();
    }
    let mut document = WorldDocument::new(session.archive().unwrap().unwrap());
    world_library::describe_from_snapshot(&mut document.metadata, &snapshot);
    document.settle_checkpoint();
    document
}

#[test]
fn a_code_with_a_checkpoint_opens_through_a_process_pack() {
    let document = played();
    let code = encode_world_code(&document).unwrap();
    assert!(code.starts_with("wm2:"));
    let carried = decode_world_code(&code).unwrap().archive;
    let checkpoint = carried.checkpoint.clone().expect("a checkpoint");
    assert!(checkpoint.events > 0, "the code sums up the seasons before");
    assert!(carried.events.len() < document.archive.events.len());

    // Opened in-process and through the Pack's own process, it is the same
    // World, and where the owner's World stands.
    let root = temp_dir("v4");
    let process = installed_registry(&root);
    let mut in_process = WorldRegistry::new();
    in_process
        .register(tiny_society::tiny_society_registration())
        .unwrap();
    let visit = WorldVisit::open_code(&code, &process).unwrap();
    let here = WorldVisit::open_code(&code, &in_process).unwrap();
    let (theirs, ours) = (visit.snapshot(), here.snapshot());
    assert_eq!(theirs.world_time, document.archive.world_time);
    assert_eq!(theirs.world_time, ours.world_time);
    assert_eq!(theirs.title, ours.title);
    assert_eq!(theirs.canvas.items, ours.canvas.items);
    assert_eq!(theirs.commands, ours.commands);
    assert_eq!(visit.world_code().unwrap(), code);

    // The owner's whole history, with its checkpoint, opens there too.
    let owner = process.open_archive(&document.archive).unwrap();
    assert_eq!(owner.snapshot().world_time, document.archive.world_time);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn a_pack_from_before_checkpoints_opens_a_whole_history_but_not_a_code() {
    let document = played();
    let code = encode_world_code(&document).unwrap();
    let root = temp_dir("v3");
    let older = registry_speaking(&root, PACK_PROTOCOL_VERSION_V3);

    // It is handed the whole history without the checkpoint, and replays it.
    let owner = older.open_archive(&document.archive).unwrap();
    assert_eq!(owner.snapshot().world_time, document.archive.world_time);

    // A code's season alone it could only replay wrongly, so it is refused.
    let refused = WorldVisit::open_code(&code, &older).err().expect("refused");
    assert!(
        matches!(refused, WorldCodeError::CannotOpen(_)),
        "{refused:?}"
    );
    assert!(refused.to_string().contains("needs a newer"), "{refused}");
    let _ = fs::remove_dir_all(root);
}
