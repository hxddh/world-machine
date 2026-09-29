use pocket_universe::{
    NUDGE_COMMAND, POCKET_UNIVERSE_PACK_ID, POCKET_UNIVERSE_PACK_VERSION, SEED_MARS_COLONY_COMMAND,
};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::{self, Command};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use world_host::WorldRegistry;
use world_pack_catalog::PackCatalog;
use world_projection::ProjectionIntent;

static TEMP_DIR_NONCE: AtomicU64 = AtomicU64::new(1);

fn temp_dir() -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let nonce = TEMP_DIR_NONCE.fetch_add(1, Ordering::Relaxed);
    let root = env::temp_dir().join(format!(
        "world-machine-pocket-universe-external-{}-{timestamp}-{nonce}",
        process::id()
    ));
    fs::create_dir(&root).unwrap();
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
    assert_eq!(preview.pack().version, POCKET_UNIVERSE_PACK_VERSION);
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
    assert_eq!(empty.title, "A new World");
    // Each place to begin crosses the process boundary with its picture.
    assert!(!empty.commands.is_empty());
    assert!(empty
        .commands
        .iter()
        .all(|command| command.scenery.is_some()));
    let seeded = session
        .handle(ProjectionIntent::InvokeCommand(
            SEED_MARS_COLONY_COMMAND.into(),
        ))
        .unwrap();
    assert_eq!(seeded.title, "Ares Pocket Colony");
    // Its stories are asked for across the boundary too (v7).
    let person = seeded
        .canvas
        .items
        .iter()
        .find(|item| item.kind == world_projection::CanvasItemKind::Actor)
        .unwrap()
        .id;
    match session
        .story(world_projection::StoryRequest::Legend(person))
        .unwrap()
    {
        Some(world_projection::StoryPage::Legend(legend)) => assert!(!legend.title.is_empty()),
        other => panic!("no legend: {other:?}"),
    }
    assert_eq!(
        session
            .story(world_projection::StoryRequest::Almanac(1))
            .unwrap(),
        None
    );
    // What it keeps score of, and how its choices move it, cross the
    // process boundary too.
    assert_eq!(
        seeded
            .gauges
            .iter()
            .map(|gauge| gauge.id.as_str())
            .collect::<Vec<_>>(),
        ["trust", "tension", "stores"]
    );
    assert!(seeded.scenery.is_some(), "a started World keeps its look");
    assert!(
        seeded
            .canvas
            .items
            .iter()
            .any(|item| item.label == "Ares Habitat"
                && item.shape == Some(world_projection::MarkShape::Dome)),
        "the habitat crosses the process boundary as a dome, not a house"
    );
    assert!(seeded
        .collection
        .items
        .iter()
        .any(|item| item.title == "Ares Habitat"));

    let grown = session.advance_background(3).unwrap();
    assert_eq!(grown.world_time, 30);
    assert_eq!(
        grown
            .briefing
            .as_ref()
            .expect("Pocket Universe has a return briefing")
            .title,
        "While you were away"
    );
    // The questions people ask, and what answering them moves, cross the
    // process boundary too.
    let mut answered = false;
    for _ in 0..10 {
        let snapshot = session.snapshot();
        if let Some(answer) = snapshot
            .commands
            .iter()
            .find(|command| command.question.is_some() && command.unavailable.is_none())
        {
            session
                .handle(ProjectionIntent::InvokeCommand(answer.id.clone()))
                .unwrap();
            answered = true;
            break;
        }
        session
            .handle(ProjectionIntent::InvokeCommand(NUDGE_COMMAND.into()))
            .unwrap();
    }
    assert!(answered, "somebody asks something within ten periods");

    let archive = session.archive().unwrap().unwrap();
    let before = session.snapshot();
    drop(session);

    let reopened = registry.open_archive(&archive).unwrap();
    assert_eq!(reopened.snapshot(), before);
    assert_eq!(reopened.archive().unwrap().unwrap(), archive);
}
