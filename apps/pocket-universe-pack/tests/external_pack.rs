use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::{self, Command};
use std::time::{SystemTime, UNIX_EPOCH};
use pocket_universe::{
    POCKET_UNIVERSE_PACK_ID, SEED_MARS_COLONY_COMMAND,
};
use world_host::WorldRegistry;
use world_pack_catalog::PackCatalog;
use world_projection::ProjectionIntent;

fn temp_dir() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let root = env::temp_dir().join(format!(
        "world-machine-pocket-universe-external-{}-{nonce}",
        process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    root
}

#[test]
fn pocket_universe_is_a_real_external_pack_with_durable_seed_and_growth() {
    let binary = env!("CARGO_BIN_EXE_pocket-universe-pack");
    let root = temp_dir();
    let bundle_path = root.join("pocket-universe.worldpack");
    let status = Command::new(binary)
        .arg("--write-bundle")
        .arg(&bundle_path)
        .status()
        .unwrap();
    assert!(status.success());

    let mut catalog = PackCatalog::open(root.join("catalog.json")).unwrap();
    let preview = catalog.inspect_install(&bundle_path).unwrap();
    assert_eq!(preview.pack().id, POCKET_UNIVERSE_PACK_ID);
    let installed = catalog.install_reviewed_pending_probe(&preview).unwrap();
    assert!(!installed.enabled);
    assert!(!installed.active);
    fs::remove_file(&bundle_path).unwrap();

    let probe = catalog.probe(&installed.pack).unwrap();
    assert_eq!(probe.pack, installed.pack);
    assert_eq!(probe.created_world_time, 0);
    assert_eq!(probe.reopened_world_time, 0);

    catalog.set_enabled(&installed.pack, true).unwrap();
    catalog.activate(&installed.pack).unwrap();
    let source = catalog.trusted_source().unwrap();
    let mut registry = WorldRegistry::new();
    registry.install_source(&source).unwrap();

    let mut session = registry.create(POCKET_UNIVERSE_PACK_ID).unwrap();
    let empty = session.snapshot();
    assert_eq!(empty.title, "Pocket Universe · Empty World");
    let seeded = session
        .handle(ProjectionIntent::InvokeCommand(
            SEED_MARS_COLONY_COMMAND.into(),
        ))
        .unwrap();
    assert_eq!(seeded.title, "Ares Pocket Colony");
    assert!(seeded
        .collection
        .items
        .iter()
        .any(|item| item.title == "Ares Habitat"));

    let grown = session.advance_background(2).unwrap();
    assert_eq!(grown.world_time, 20);
    assert!(grown
        .briefing
        .as_ref()
        .expect("Pocket Universe has a briefing")
        .title
        .contains("Generation 2"));

    let archive = session.archive().unwrap().unwrap();
    let before = session.snapshot();
    drop(session);

    let reopened = registry.open_archive(&archive).unwrap();
    assert_eq!(reopened.snapshot(), before);
    assert_eq!(reopened.archive().unwrap().unwrap(), archive);
}
