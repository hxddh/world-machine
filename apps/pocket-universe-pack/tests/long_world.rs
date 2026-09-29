//! Maple Street kept for three years opens, plays a day, saves and reopens
//! through Pocket Universe running as a process Pack, as the desktop app
//! runs it. Before protocol v5 its archive crossed 16 MiB, the most a frame
//! held, at about day 1,020.
//!
//! ```text
//! cargo test --release -p pocket-universe-pack --test long_world -- --ignored --nocapture
//! ```

use pocket_universe::{NUDGE_COMMAND, POCKET_UNIVERSE_PACK_ID, SEED_1980S_TOWN_COMMAND};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::{self, Command};
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use world_document::WorldDocument;
use world_host::WorldRegistry;
use world_pack_catalog::PackCatalog;
use world_pack_protocol::{
    encode_request, PackRequest, PackRequestEnvelope, PACK_FRAME_LIMIT_BEFORE_V5,
    PACK_PROTOCOL_VERSION_V4, PACK_PROTOCOL_VERSION_V5,
};
use world_persistence::WorldArchive;
use world_projection::ProjectionIntent::InvokeCommand;

fn in_process() -> WorldRegistry {
    let mut registry = WorldRegistry::new();
    registry
        .register(pocket_universe::pocket_universe_registration())
        .unwrap();
    registry
}

/// A warm player's Maple Street after `days` days: every day they answer
/// the first question on offer, make something every third day, and nudge
/// the day along.
fn lived(days: u64) -> WorldArchive {
    let mut session = in_process().create(POCKET_UNIVERSE_PACK_ID).unwrap();
    let mut snapshot = session
        .handle(InvokeCommand(SEED_1980S_TOWN_COMMAND.into()))
        .unwrap();
    for day in 1..=days {
        if let Some(answer) = snapshot
            .commands
            .iter()
            .find(|c| c.question.is_some() && c.unavailable.is_none() && c.id != NUDGE_COMMAND)
        {
            if let Ok(next) = session.handle(InvokeCommand(answer.id.clone())) {
                snapshot = next;
            }
        }
        if day % 3 == 0 {
            let deed = snapshot
                .commands
                .iter()
                .filter(|c| c.unavailable.is_none())
                .find(|c| c.hand.as_ref().is_some_and(|hand| hand.verb != "Undo"))
                .map(|c| c.id.clone());
            if let Some(deed) = deed {
                let _ = session.handle(InvokeCommand(deed));
            }
        }
        snapshot = session.handle(InvokeCommand(NUDGE_COMMAND.into())).unwrap();
    }
    session.archive().unwrap().unwrap()
}

fn open_frame(protocol_version: u32, archive: &WorldArchive) -> usize {
    let request = PackRequestEnvelope::for_version(
        protocol_version,
        2,
        PackRequest::Open {
            archive: archive.clone(),
        },
    )
    .unwrap();
    encode_request(&request).unwrap().len() + 1
}

#[test]
#[ignore = "lives 1,080 days; run in release"]
fn three_years_of_maple_street_open_play_save_and_reopen_through_the_process_pack() {
    let started = Instant::now();
    let archive = lived(1_080);
    eprintln!(
        "lived 1,080 days ({} events) in {:?}",
        archive.events.len(),
        started.elapsed()
    );
    let mut document = WorldDocument::new(archive);
    document.settle_checkpoint();
    let document = WorldDocument::from_bytes(&document.to_bytes().unwrap()).unwrap();
    let archive = &document.archive;
    let (tagged, packed) = (
        open_frame(PACK_PROTOCOL_VERSION_V4, archive),
        open_frame(PACK_PROTOCOL_VERSION_V5, archive),
    );
    eprintln!(
        "Maple Street day 1,080: v4 open {:.1} MB, v5 open {:.2} MB",
        tagged as f64 / 1e6,
        packed as f64 / 1e6
    );
    assert!(tagged > PACK_FRAME_LIMIT_BEFORE_V5);
    assert!(packed * 4 < PACK_FRAME_LIMIT_BEFORE_V5);

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let root = env::temp_dir().join(format!(
        "world-machine-maple-long-{}-{nanos}",
        process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    let bundle: PathBuf = root.join("pocket-universe.worldpack");
    let status = Command::new(env!("CARGO_BIN_EXE_pocket-universe-pack"))
        .arg("--write-bundle")
        .arg(&bundle)
        .status()
        .unwrap();
    assert!(status.success());
    let mut catalog = PackCatalog::open(root.join("catalog.json")).unwrap();
    catalog.install_bundle(&bundle).unwrap();
    let mut registry = WorldRegistry::new();
    registry
        .install_source(&catalog.trusted_source().unwrap())
        .unwrap();

    let started = Instant::now();
    let mut process = registry.open_archive(archive).unwrap();
    eprintln!("opened through the process Pack in {:?}", started.elapsed());
    let mut here = in_process().open_archive(archive).unwrap();
    assert_eq!(process.snapshot().world_time, archive.world_time);
    assert_eq!(process.snapshot().commands, here.snapshot().commands);

    let next = process.handle(InvokeCommand(NUDGE_COMMAND.into())).unwrap();
    let expected = here.handle(InvokeCommand(NUDGE_COMMAND.into())).unwrap();
    assert_eq!(next.world_time, expected.world_time);
    assert!(next.world_time > archive.world_time);

    let saved = process.archive().unwrap().unwrap();
    assert_eq!(saved, here.archive().unwrap().unwrap());
    assert_eq!(saved.events[..archive.events.len()], archive.events[..]);

    let mut document = WorldDocument::new(saved.clone());
    document.settle_checkpoint();
    let reread = WorldDocument::from_bytes(&document.to_bytes().unwrap()).unwrap();
    let reopened = registry.open_archive(&reread.archive).unwrap();
    assert_eq!(reopened.snapshot().world_time, next.world_time);
    assert_eq!(reopened.archive().unwrap().unwrap(), saved);
    let _ = fs::remove_dir_all(root);
}
