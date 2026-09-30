//! Replay-identical fixtures.
//!
//! Every saved World in a Pack's `tests/fixtures` is opened, replayed and
//! played on by a fixed, deterministic builder. The `Debug` text of every
//! snapshot along the way, and of the event log at the end, is digested and
//! compared with a golden file written by the code before a refactor. A
//! refactor that is meant to change nothing must leave every digest alone.
//!
//! - `WORLD_PACK_BLESS=1` rewrites the golden files (only on a tree whose
//!   behaviour you trust).
//! - `WORLD_PACK_DUMP=<dir>` writes the full text of every step, to diff.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use world_document::WorldDocument;
use world_host::{WorldRegistry, WorldSession};
use world_projection::ProjectionIntent::InvokeCommand;

/// One step of a replay trace: a label and the full text it digests.
pub struct Step {
    pub label: String,
    pub text: String,
}

/// 64-bit FNV-1a: small, stable and dependency-free.
pub fn fnv64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// The builder every fixture test uses: the first open question each day;
/// every third day a plot built when one is on offer, or else something made
/// by hand; then the day let pass with `pass`.
pub fn builder(session: &mut Box<dyn WorldSession>, pass: &str, days: usize) {
    for day in 1..=days {
        builder_day(session, pass, day);
    }
}

fn builder_day(session: &mut Box<dyn WorldSession>, pass: &str, day: usize) {
    let snapshot = session.snapshot();
    if let Some(answer) = snapshot
        .commands
        .iter()
        .find(|c| c.question.is_some() && c.unavailable.is_none() && c.id != pass)
    {
        let _ = session.handle(InvokeCommand(answer.id.clone()));
    }
    if day.is_multiple_of(3) {
        let snapshot = session.snapshot();
        let plot = snapshot
            .canvas
            .plots
            .iter()
            .flat_map(|plot| plot.offers.iter())
            .find(|offer| offer.unavailable.is_none())
            .map(|offer| offer.command.clone());
        let deed = plot.or_else(|| {
            snapshot
                .commands
                .iter()
                .filter(|c| c.unavailable.is_none())
                .find(|c| c.hand.as_ref().is_some_and(|hand| hand.verb != "Undo"))
                .map(|c| c.id.clone())
        });
        if let Some(deed) = deed {
            let _ = session.handle(InvokeCommand(deed));
        }
    }
    session
        .handle(InvokeCommand(pass.into()))
        .expect("the day passes");
}

/// Opens `fixture`, checks it replays event for event, then plays it on for
/// `days` days, recording the snapshot after opening and after every day, and
/// the event log at the end.
pub fn trace(registry: &WorldRegistry, fixture: &Path, pass: &str, days: usize) -> Vec<Step> {
    let bytes = std::fs::read(fixture).expect("the fixture");
    let archive = WorldDocument::from_bytes(&bytes).unwrap().archive;
    let mut session = registry.open_archive(&archive).expect("the fixture opens");
    let reopened = session.archive().unwrap().unwrap();
    assert_eq!(
        reopened.events,
        archive.events,
        "{} replays event for event",
        fixture.display()
    );
    let mut steps = vec![Step {
        label: "open".into(),
        text: format!("{:#?}", session.snapshot()),
    }];
    for day in 1..=days {
        builder_day(&mut session, pass, day);
        steps.push(Step {
            label: format!("day-{day:03}"),
            text: format!("{:#?}", session.snapshot()),
        });
    }
    let after = session.archive().unwrap().unwrap();
    assert_eq!(after.events[..archive.events.len()], archive.events[..]);
    steps.push(Step {
        label: "events".into(),
        text: format!("{:#?}", &after.events[archive.events.len()..]),
    });
    steps
}

/// Every `*.world` file in `dir`, sorted.
pub fn fixtures(dir: &Path) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = std::fs::read_dir(dir)
        .expect("the fixtures directory")
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "world"))
        .collect();
    out.sort();
    out
}

/// Traces every fixture in `fixtures_dir` and compares the digests with
/// `golden_dir/<fixture>.digest`. A fixture without a golden file is
/// reported and skipped unless blessing.
pub fn assert_replay_identical(
    registry: &WorldRegistry,
    fixtures_dir: &Path,
    golden_dir: &Path,
    pass: &str,
    days: usize,
) {
    let bless = std::env::var_os("WORLD_PACK_BLESS").is_some();
    let dump = std::env::var_os("WORLD_PACK_DUMP").map(PathBuf::from);
    let mut failures = Vec::new();
    let mut checked = 0;
    for fixture in fixtures(fixtures_dir) {
        let stem = fixture.file_stem().unwrap().to_string_lossy().to_string();
        let golden = golden_dir.join(format!("{stem}.digest"));
        if !bless && !golden.exists() {
            eprintln!("no golden digest for {stem}: run with WORLD_PACK_BLESS=1");
            continue;
        }
        let steps = trace(registry, &fixture, pass, days);
        let mut digest = String::new();
        for step in &steps {
            let _ = writeln!(
                digest,
                "{} {:016x} {}",
                step.label,
                fnv64(step.text.as_bytes()),
                step.text.len()
            );
        }
        if let Some(dir) = &dump {
            let dir = dir.join(&stem);
            std::fs::create_dir_all(&dir).unwrap();
            for step in &steps {
                std::fs::write(dir.join(format!("{}.txt", step.label)), &step.text).unwrap();
            }
        }
        if bless {
            std::fs::create_dir_all(golden_dir).unwrap();
            std::fs::write(&golden, &digest).unwrap();
            continue;
        }
        checked += 1;
        let expected = std::fs::read_to_string(&golden).unwrap();
        if expected != digest {
            let first = expected
                .lines()
                .zip(digest.lines())
                .find(|(a, b)| a != b)
                .map(|(a, b)| format!("expected `{a}`, got `{b}`"))
                .unwrap_or_else(|| "a different number of steps".into());
            failures.push(format!("{stem}: {first}"));
        }
    }
    assert!(
        failures.is_empty(),
        "not replay-identical:\n{}",
        failures.join("\n")
    );
    assert!(bless || checked > 0, "no fixture was checked");
}
