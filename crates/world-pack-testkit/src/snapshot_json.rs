//! Snapshot JSON identical to a golden trace, byte for byte.
//!
//! Every saved World in a Pack's `tests/fixtures` is opened and played on by
//! a builder for a few days; the wire JSON (`ProjectionSnapshotWire`, what a
//! Pack process sends) of every snapshot along the way is digested and
//! compared with `tests/golden/snapshot_json/<fixture>.digest`, written by
//! the code before the v0.25 speed work. Faster paths and caches must never
//! change what a player is shown; so each day's snapshot is also taken again
//! from the unchanged World, which must give the same JSON.
//!
//! - `WORLD_MACHINE_BLESS=1` rewrites the golden files (only on a tree
//!   whose snapshots you trust).
//! - `WORLD_MACHINE_DUMP=<dir>` writes every snapshot's JSON, to diff.
//!
//! The JSON is digested as it is written, never kept as a string; in a
//! debug build only every fifth day is digested (see
//! [`crate::replay::digest_every`]).

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use world_document::WorldDocument;
use world_host::{WorldRegistry, WorldSession};
use world_pack_protocol::ProjectionSnapshotWire;
use world_projection::ProjectionIntent::InvokeCommand;
use world_projection::ProjectionSnapshot;

use crate::replay::{compare, digest_every, digested, fixtures, Digest};

/// A snapshot's wire JSON, digested as it is written ([`Digest`]), and kept
/// only when it is to be written out (`WORLD_MACHINE_DUMP`).
pub struct Json {
    pub digest: Digest,
    pub text: Option<String>,
}

fn json_of(snapshot: &ProjectionSnapshot, keep: bool) -> Json {
    let wire = ProjectionSnapshotWire::from(snapshot);
    let mut digest = Digest::new();
    serde_json::to_writer(&mut digest, &wire).expect("snapshot JSON");
    Json {
        digest,
        text: keep.then(|| serde_json::to_string(&wire).expect("snapshot JSON")),
    }
}

/// The fixture opened and played on for `days` days: each snapshot's
/// label and JSON (kept as text only with `keep`).
pub fn trace(
    registry: &WorldRegistry,
    fixture: &Path,
    pass: &str,
    days: usize,
    keep: bool,
) -> Vec<(String, Json)> {
    let every = digest_every();
    let bytes = std::fs::read(fixture).expect("the fixture");
    let archive = WorldDocument::from_bytes(&bytes).unwrap().archive;
    let mut session: Box<dyn WorldSession> =
        registry.open_archive(&archive).expect("the fixture opens");
    let mut steps = Vec::new();
    let mut snapshot = session.snapshot();
    steps.push(("opened".to_string(), json_of(&snapshot, keep)));
    for day in 1..=days {
        // Every day is played; only some are digested (see `digest_every`).
        let today = digested(day, days, every);
        if let Some(answer) = snapshot
            .commands
            .iter()
            .find(|c| c.question.is_some() && c.unavailable.is_none() && c.id != pass)
        {
            let id = answer.id.clone();
            if let Ok(next) = session.handle(InvokeCommand(id.clone())) {
                if today {
                    steps.push((format!("day {day} answered {id}"), json_of(&next, keep)));
                }
                snapshot = next;
            }
        }
        if day.is_multiple_of(3) {
            let offer = snapshot
                .canvas
                .plots
                .iter()
                .flat_map(|plot| plot.offers.iter())
                .find(|offer| offer.unavailable.is_none())
                .map(|offer| offer.command.clone())
                .or_else(|| {
                    snapshot
                        .commands
                        .iter()
                        .filter(|c| c.unavailable.is_none())
                        .find(|c| c.hand.as_ref().is_some_and(|hand| hand.verb != "Undo"))
                        .map(|c| c.id.clone())
                });
            if let Some(offer) = offer {
                if let Ok(next) = session.handle(InvokeCommand(offer.clone())) {
                    if today {
                        steps.push((format!("day {day} made {offer}"), json_of(&next, keep)));
                    }
                }
            }
        }
        let next = session
            .handle(InvokeCommand(pass.into()))
            .expect("the day passes");
        if today {
            steps.push((format!("day {day} passed"), json_of(&next, keep)));
        }
        // Looked at again, as the app may: the same World, the same JSON.
        snapshot = session.snapshot();
        if today {
            steps.push((
                format!("day {day} looked at again"),
                json_of(&snapshot, keep),
            ));
        }
    }
    steps
}

/// Traces every fixture in `fixtures_dir` for `days` days and compares the
/// digests with `golden_dir/<fixture>.digest`. A fixture without a golden
/// file is reported and skipped unless blessing.
pub fn assert_snapshot_json_identical(
    registry: &WorldRegistry,
    fixtures_dir: &Path,
    golden_dir: &Path,
    pass: &str,
    days: usize,
) {
    let bless = std::env::var_os("WORLD_MACHINE_BLESS").is_some();
    let dump = std::env::var_os("WORLD_MACHINE_DUMP").map(PathBuf::from);
    let mut failures = Vec::new();
    let mut checked = 0;
    for fixture in fixtures(fixtures_dir) {
        let stem = fixture.file_stem().unwrap().to_string_lossy().to_string();
        let golden = golden_dir.join(format!("{stem}.digest"));
        if !bless && !golden.exists() {
            eprintln!("no golden snapshot JSON for {stem}: run with WORLD_MACHINE_BLESS=1");
            continue;
        }
        let steps = trace(registry, &fixture, pass, days, dump.is_some());
        let mut digest = String::new();
        for (label, json) in &steps {
            let _ = writeln!(
                digest,
                "{:016x} {} {label}",
                json.digest.hash(),
                json.digest.len()
            );
        }
        if let Some(dir) = &dump {
            std::fs::create_dir_all(dir).unwrap();
            for (at, (_, json)) in steps.iter().enumerate() {
                let text = json.text.as_deref().unwrap_or_default();
                std::fs::write(dir.join(format!("{stem}-{at:02}.json")), text).unwrap();
            }
        }
        if bless {
            std::fs::create_dir_all(golden_dir).unwrap();
            std::fs::write(&golden, &digest).unwrap();
            continue;
        }
        checked += 1;
        let expected = std::fs::read_to_string(&golden).unwrap();
        if let Some(first) = compare(&expected, &digest, digest_every() == 1) {
            failures.push(format!("{stem}: {first}"));
        }
    }
    assert!(
        failures.is_empty(),
        "snapshot JSON differs:\n{}",
        failures.join("\n")
    );
    assert!(bless || checked > 0, "no fixture was checked");
}
