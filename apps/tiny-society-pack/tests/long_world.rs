//! A World kept for years opens, plays a day, saves and reopens through
//! Tiny Society running as a process Pack, as the desktop app runs it.
//!
//! Before protocol v5 the whole archive crossed as tagged JSON in one
//! frame, and frames stop at 16 MiB: a warm player's World stopped opening
//! at about day 660. From v5 the archive crosses packed (compact, deflated,
//! base64).
//!
//! The long plays are ignored; run them in a release build:
//!
//! ```text
//! cargo test --release -p tiny-society-pack --test long_world -- --ignored --nocapture
//! ```
//!
//! Set `WORLD_MACHINE_THREE_YEARS` to a World file to reuse (or keep) the
//! three-year World, as `crates/world-library/tests/three_years.rs` does.

use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{self, Command};
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tiny_society::TINY_SOCIETY_PACK_ID;
use world_document::WorldDocument;
use world_host::WorldRegistry;
use world_pack_catalog::PackCatalog;
use world_pack_process::{ProcessPack, ProcessPackSource};
use world_pack_protocol::{
    encode_request, encode_response, PackManifest, PackRequest, PackRequestEnvelope, PackResponse,
    PackResponseEnvelope, PACK_FRAME_LIMIT_BEFORE_V5, PACK_PROTOCOL_VERSION,
    PACK_PROTOCOL_VERSION_V4, PACK_PROTOCOL_VERSION_V5,
};
use world_persistence::{ArchivedValue, WorldArchive};
use world_projection::ProjectionIntent::InvokeCommand;

const PASS: &str = "tiny-society.let-day-pass";

fn temp_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let root = env::temp_dir().join(format!(
        "world-machine-long-world-{label}-{}-{nanos}",
        process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    root
}

/// The Pack binary as the desktop app runs it: from its `.worldpack`,
/// installed through the catalog.
fn installed_registry(root: &Path) -> WorldRegistry {
    let bundle = root.join("tiny-society.worldpack");
    let status = Command::new(env!("CARGO_BIN_EXE_tiny-society-pack"))
        .arg("--write-bundle")
        .arg(&bundle)
        .status()
        .unwrap();
    assert!(status.success());
    let mut catalog = PackCatalog::open(root.join("catalog.json")).unwrap();
    let installed = catalog.install_bundle(&bundle).unwrap();
    let pack = ProcessPack::load(&installed.manifest_path).unwrap();
    assert_eq!(pack.protocol_version, PACK_PROTOCOL_VERSION);
    let mut registry = WorldRegistry::new();
    registry
        .install_source(&catalog.trusted_source().unwrap())
        .unwrap();
    registry
}

/// The Pack binary behind a manifest that says it speaks
/// `protocol_version`, as a Pack built before v5 does.
fn registry_speaking(root: &Path, protocol_version: u32) -> WorldRegistry {
    let output = Command::new(env!("CARGO_BIN_EXE_tiny-society-pack"))
        .arg("--print-manifest")
        .output()
        .unwrap();
    assert!(output.status.success());
    let mut manifest = PackManifest::from_json(&String::from_utf8(output.stdout).unwrap()).unwrap();
    manifest.protocol_version = protocol_version;
    let path = root.join(format!("tiny-society-v{protocol_version}.world-pack.json"));
    fs::write(&path, manifest.to_json_pretty().unwrap()).unwrap();
    let mut registry = WorldRegistry::new();
    registry
        .install_source(&ProcessPackSource::from_packs(vec![ProcessPack::load(
            path,
        )
        .unwrap()]))
        .unwrap();
    registry
}

fn in_process() -> WorldRegistry {
    let mut registry = WorldRegistry::new();
    registry
        .register(tiny_society::tiny_society_registration())
        .unwrap();
    registry
}

/// A warm player's World after `days` days, as `three_years.rs` plays it:
/// every day they answer the first question on offer, make something every
/// third day, and let the day pass.
fn lived(days: u64) -> WorldArchive {
    let mut session = in_process().create(TINY_SOCIETY_PACK_ID).unwrap();
    let mut snapshot = session.snapshot();
    for day in 1..=days {
        if let Some(answer) = snapshot
            .commands
            .iter()
            .find(|c| c.question.is_some() && c.unavailable.is_none() && c.id != PASS)
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
        snapshot = session.handle(InvokeCommand(PASS.into())).unwrap();
    }
    session.archive().unwrap().unwrap()
}

/// The World as its owner's file keeps it: with what its snapshot says
/// about it and its checkpoint at the start of its latest season.
fn kept(archive: WorldArchive) -> WorldDocument {
    let session = in_process().open_archive(&archive).unwrap();
    let mut document = WorldDocument::new(archive);
    world_library::describe_from_snapshot(&mut document.metadata, &session.snapshot());
    document.settle_checkpoint();
    document
}

/// The archive `archive` becomes when saved to a World file and read back,
/// as the app saves and opens it.
fn through_a_file(archive: WorldArchive, like: &WorldDocument) -> WorldDocument {
    let mut document = WorldDocument::new(archive);
    document.metadata = like.metadata.clone();
    document.settle_checkpoint();
    WorldDocument::from_bytes(&document.to_bytes().unwrap()).unwrap()
}

fn frame_bytes(protocol_version: u32, archive: &WorldArchive) -> (usize, usize) {
    let open = encode_request(
        &PackRequestEnvelope::for_version(
            protocol_version,
            2,
            PackRequest::Open {
                archive: archive.clone(),
            },
        )
        .unwrap(),
    )
    .unwrap()
    .len()
        + 1;
    let mut saved = archive.clone();
    saved.checkpoint = None;
    let archived = encode_response(
        &PackResponseEnvelope::for_version(
            protocol_version,
            3,
            PackResponse::Archive {
                archive: Some(saved),
            },
        )
        .unwrap(),
    )
    .unwrap()
    .len()
        + 1;
    (open, archived)
}

fn megabytes(bytes: usize) -> f64 {
    bytes as f64 / 1_000_000.0
}

/// Opens `document` through `registry`, plays a day, saves, and reopens
/// what was saved: each step is the World opened in-process would take.
fn open_play_save_reopen(registry: &WorldRegistry, document: &WorldDocument) {
    let archive = &document.archive;
    let started = Instant::now();
    let mut process = registry.open_archive(archive).unwrap();
    eprintln!(
        "opened {} events through the process Pack in {:?}",
        archive.events.len(),
        started.elapsed()
    );
    let mut here = in_process().open_archive(archive).unwrap();
    let (theirs, ours) = (process.snapshot(), here.snapshot());
    assert_eq!(theirs.world_time, archive.world_time);
    assert_eq!(theirs.world_time, ours.world_time);
    assert_eq!(theirs.title, ours.title);
    assert_eq!(theirs.commands, ours.commands);

    // A day, played there and here alike.
    let next = process.handle(InvokeCommand(PASS.into())).unwrap();
    let expected = here.handle(InvokeCommand(PASS.into())).unwrap();
    assert!(next.world_time > archive.world_time);
    assert_eq!(next.world_time, expected.world_time);

    // Saved, it keeps the whole history, exactly, and the day after it.
    let started = Instant::now();
    let saved = process.archive().unwrap().unwrap();
    eprintln!("saved through the process Pack in {:?}", started.elapsed());
    assert_eq!(saved, here.archive().unwrap().unwrap());
    assert!(saved.events.len() > archive.events.len());
    assert_eq!(saved.events[..archive.events.len()], archive.events[..]);
    assert_eq!(saved.world_time, next.world_time);

    // Written to a file and opened again, it is the same World.
    let reread = through_a_file(saved.clone(), document);
    let reopened = registry.open_archive(&reread.archive).unwrap();
    assert_eq!(reopened.snapshot().world_time, next.world_time);
    assert_eq!(reopened.snapshot().commands, next.commands);
    assert_eq!(reopened.archive().unwrap().unwrap(), saved);
}

/// `archive` with notes added to its events' payloads until its tagged
/// archive is over `bytes`: a history as long in bytes as a warm player's
/// after two years, from a World played for a few weeks. The notes read
/// like a history's text, so they pack about as a history does.
fn padded(mut archive: WorldArchive, bytes: usize) -> WorldArchive {
    let short = serde_json::to_vec(&archive).unwrap().len();
    let per_event = bytes.saturating_sub(short) / archive.events.len() + 1;
    let places = [
        "harbour",
        "pier",
        "market",
        "chapel",
        "lighthouse",
        "bakery",
    ];
    let deeds = [
        "mended",
        "painted",
        "counted",
        "carried",
        "sang about",
        "argued over",
    ];
    for event in &mut archive.events {
        let mut notes = Vec::new();
        let mut written = 0;
        let mut n = event.id;
        while written < per_event {
            n = n
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let line = format!(
                "Day {}: someone {} the {} ({} of {}).",
                event.world_time / 24,
                deeds[(n >> 33) as usize % deeds.len()],
                places[(n >> 40) as usize % places.len()],
                (n >> 48) % 97,
                event.id
            );
            written += line.len() + 20;
            notes.push(ArchivedValue::Text(line));
        }
        event
            .payload
            .insert("test.notes".into(), ArchivedValue::List(notes));
    }
    assert!(serde_json::to_vec(&archive).unwrap().len() > bytes);
    archive
}

/// Opening and saving through a process Pack no longer stops at 16 MiB: a
/// history whose tagged archive is over it crosses packed, and plays on.
#[test]
fn a_history_over_16_mib_opens_plays_and_saves_through_the_process_pack() {
    let document = kept(padded(lived(40), 17 * 1024 * 1024));
    let archive = &document.archive;
    let (tagged_open, tagged_archive) = frame_bytes(PACK_PROTOCOL_VERSION_V4, archive);
    let (packed_open, packed_archive) = frame_bytes(PACK_PROTOCOL_VERSION_V5, archive);
    eprintln!(
        "{} events: tagged open {:.1} MB / archive {:.1} MB; packed open {:.2} MB / archive {:.2} MB",
        archive.events.len(),
        megabytes(tagged_open),
        megabytes(tagged_archive),
        megabytes(packed_open),
        megabytes(packed_archive),
    );
    assert!(tagged_open > PACK_FRAME_LIMIT_BEFORE_V5);
    assert!(tagged_archive > PACK_FRAME_LIMIT_BEFORE_V5);
    assert!(packed_open * 4 < PACK_FRAME_LIMIT_BEFORE_V5);

    let root = temp_dir("over-16");
    open_play_save_reopen(&installed_registry(&root), &document);

    // A Pack that speaks v4 is still handed a World that fits, and refused
    // this one with a plain reason, before anything crosses.
    let older = registry_speaking(&root, PACK_PROTOCOL_VERSION_V4);
    let short = kept(lived(3));
    let opened = older.open_archive(&short.archive).unwrap();
    assert_eq!(opened.snapshot().world_time, short.archive.world_time);
    let refused = older.open_archive(archive).err().expect("refused");
    assert!(
        refused
            .to_string()
            .contains("too long for a Pack on protocol v4"),
        "{refused}"
    );
    let _ = fs::remove_dir_all(root);
}

/// The three-year World (1,080 days), from a World file as v0.19.0 wrote
/// it (the World file format is unchanged since), through the process Pack.
#[test]
#[ignore = "lives 1,080 days; run in release"]
fn a_three_year_world_file_opens_plays_saves_and_reopens_through_the_process_pack() {
    let cache = env::var_os("WORLD_MACHINE_THREE_YEARS").map(PathBuf::from);
    let bytes = match cache.as_ref().and_then(|path| fs::read(path).ok()) {
        Some(bytes) => bytes,
        None => {
            let started = Instant::now();
            let document = kept(lived(1_080));
            eprintln!("lived 1,080 days in {:?}", started.elapsed());
            let bytes = document.to_bytes().unwrap();
            if let Some(path) = cache.as_ref() {
                fs::write(path, &bytes).unwrap();
            }
            bytes
        }
    };
    let document = WorldDocument::from_bytes(&bytes).unwrap();
    report_frames("three years", &document);
    let root = temp_dir("three-years");
    open_play_save_reopen(&installed_registry(&root), &document);
    let _ = fs::remove_dir_all(root);
}

/// A year, three and ten, played once: how big `open` and `archive` are on
/// the wire before and from v5, and ten years opening through the Pack.
#[test]
#[ignore = "lives 3,600 days; run in release"]
fn ten_years_fit_the_wire_with_room_to_spare() {
    let mut session = in_process().create(TINY_SOCIETY_PACK_ID).unwrap();
    let mut snapshot = session.snapshot();
    let mut at = BTreeMap::new();
    let started = Instant::now();
    for day in 1..=3_600u64 {
        if let Some(answer) = snapshot
            .commands
            .iter()
            .find(|c| c.question.is_some() && c.unavailable.is_none() && c.id != PASS)
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
        snapshot = session.handle(InvokeCommand(PASS.into())).unwrap();
        if [360, 1_080, 1_800, 3_600].contains(&day) {
            eprintln!("lived {day} days in {:?}", started.elapsed());
            at.insert(day, session.archive().unwrap().unwrap());
        }
    }
    let mut ten_years = None;
    for (day, archive) in at {
        let document = kept(archive);
        let (open, _) = report_frames(&format!("day {day}"), &document);
        assert!(open * 2 < world_pack_protocol::PACK_FRAME_LIMIT);
        ten_years = Some(document);
    }
    let root = temp_dir("ten-years");
    open_play_save_reopen(&installed_registry(&root), &ten_years.unwrap());
    let _ = fs::remove_dir_all(root);
}

fn report_frames(label: &str, document: &WorldDocument) -> (usize, usize) {
    let archive = &document.archive;
    let (tagged_open, tagged_archive) = frame_bytes(PACK_PROTOCOL_VERSION_V4, archive);
    let started = Instant::now();
    let (open, saved) = frame_bytes(PACK_PROTOCOL_VERSION_V5, archive);
    let packing = started.elapsed();
    eprintln!(
        "{label}: {} events, file {:.2} MB; v4 open {:.1} MB, archive {:.1} MB; v5 open {:.2} MB, archive {:.2} MB (both packed in {packing:?})",
        archive.events.len(),
        megabytes(document.to_bytes().unwrap().len()),
        megabytes(tagged_open),
        megabytes(tagged_archive),
        megabytes(open),
        megabytes(saved),
    );
    (open, saved)
}
