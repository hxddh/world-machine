//! Three years of each Pocket Universe place (1,080 periods): what a turn
//! costs with its save, and what looking again at an unchanged World costs,
//! as the app plays one. The harbour's are in `three_years.rs`; until
//! v0.29 nothing timed a Pocket Universe place, and its year-three turn
//! (38–42 ms in v0.28) missed the bar the harbour met.
//!
//! The benches are timed in a release build:
//!
//! ```text
//! cargo test --release -p world-library --test pocket_three_years -- --ignored --nocapture --test-threads=1
//! ```
//!
//! Each place is lived once per run (about a minute each in release). Set
//! `WORLD_MACHINE_POCKET_THREE_YEARS` to a directory to keep the three
//! Worlds between runs.

use std::sync::OnceLock;
use std::time::{Duration, Instant};

use pocket_universe::{
    NUDGE_COMMAND, POCKET_UNIVERSE_PACK_ID, SEED_1980S_TOWN_COMMAND, SEED_MARS_COLONY_COMMAND,
    SEED_PENGUIN_CIVILIZATION_COMMAND,
};
use world_document::WorldDocument;
use world_host::WorldRegistry;
use world_persistence::WorldArchive;
use world_projection::ProjectionIntent::InvokeCommand;
use world_projection::ProjectionSnapshot;

const DAYS: u64 = 1_080;
const PLACES: [(&str, &str); 3] = [
    ("ares", SEED_MARS_COLONY_COMMAND),
    ("maple", SEED_1980S_TOWN_COMMAND),
    ("icebridge", SEED_PENGUIN_CIVILIZATION_COMMAND),
];

fn registry() -> WorldRegistry {
    let mut registry = WorldRegistry::new();
    registry
        .register(pocket_universe::pocket_universe_registration())
        .unwrap();
    registry
}

fn answer_of(snapshot: &ProjectionSnapshot) -> Option<String> {
    snapshot
        .commands
        .iter()
        .find(|c| c.question.is_some() && c.unavailable.is_none() && c.id != NUDGE_COMMAND)
        .map(|c| c.id.clone())
}

/// A warm player's place after `days` periods: every period they answer
/// the first question on offer, build on a plot (or lend a hand) every
/// third, and let the period pass.
fn lived(seed: &str, days: u64) -> WorldArchive {
    let mut session = registry().create(POCKET_UNIVERSE_PACK_ID).unwrap();
    let mut snapshot = session.handle(InvokeCommand(seed.into())).unwrap();
    for day in 1..=days {
        if let Some(answer) = answer_of(&snapshot) {
            if let Ok(next) = session.handle(InvokeCommand(answer)) {
                snapshot = next;
            }
        }
        if day % 3 == 0 {
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
        snapshot = session.handle(InvokeCommand(NUDGE_COMMAND.into())).unwrap();
    }
    session.archive().unwrap().unwrap()
}

/// The place after three years, as its owner's file keeps it.
fn three_years(place: usize) -> &'static WorldDocument {
    static WORLDS: [OnceLock<WorldDocument>; 3] =
        [OnceLock::new(), OnceLock::new(), OnceLock::new()];
    WORLDS[place].get_or_init(|| {
        let (name, seed) = PLACES[place];
        let cache = std::env::var_os("WORLD_MACHINE_POCKET_THREE_YEARS")
            .map(|dir| std::path::PathBuf::from(dir).join(format!("{name}-{DAYS}.world")));
        if let Some(bytes) = cache.as_ref().and_then(|path| std::fs::read(path).ok()) {
            return WorldDocument::from_bytes(&bytes).unwrap();
        }
        let started = Instant::now();
        let archive = lived(seed, DAYS);
        eprintln!(
            "{name}: lived {DAYS} periods ({} events) in {:?}",
            archive.events.len(),
            started.elapsed()
        );
        let session = registry().open_archive(&archive).unwrap();
        let mut document = WorldDocument::new(archive);
        world_library::describe_from_snapshot(&mut document.metadata, &session.snapshot());
        document.settle_checkpoint();
        if let Some(path) = cache {
            std::fs::write(path, document.to_bytes().unwrap()).unwrap();
        }
        document
    })
}

fn quantile(sorted: &[Duration], q: f64) -> Duration {
    sorted[((sorted.len() as f64 - 1.0) * q).round() as usize]
}

/// A turn at three years as the app plays one: the World opened from its
/// file, then each period an answer and the period let pass, each change
/// handed to the World's writer as it is kept and the snapshot after it
/// returned. Under 30 ms median and 35 ms at the 95th percentile.
fn turn_with_its_save(place: usize) {
    let (name, _) = PLACES[place];
    let document = three_years(place);
    let registry = registry();
    let dir = std::env::temp_dir().join(format!("pocket-turn-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("three.world");
    std::fs::write(&path, document.to_bytes().unwrap()).unwrap();
    let library = world_library::WorldLibrary::new(dir.join("library"));
    let mut session =
        world_library::DurableWorldSession::open_file(path.clone(), &registry).unwrap();
    let mut snapshot = session.snapshot();
    let count = std::env::var("WORLD_MACHINE_TURNS")
        .ok()
        .and_then(|count| count.parse().ok())
        .unwrap_or(21);
    let mut turns = Vec::new();
    for _ in 0..count {
        if let Some(answer) = answer_of(&snapshot) {
            let started = Instant::now();
            if session
                .handle(InvokeCommand(answer), &registry, &library)
                .is_ok()
            {
                turns.push(started.elapsed());
            }
        }
        let started = Instant::now();
        snapshot = session
            .handle(InvokeCommand(NUDGE_COMMAND.into()), &registry, &library)
            .unwrap();
        turns.push(started.elapsed());
    }
    session.flush().unwrap();
    let on_disk = WorldDocument::from_bytes(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(on_disk.archive.world_time, snapshot.world_time);
    let _ = std::fs::remove_dir_all(&dir);
    turns.sort();
    let (median, p95) = (quantile(&turns, 0.5), quantile(&turns, 0.95));
    eprintln!(
        "{name}: {} turns with their saves after period {DAYS}: median {median:?}, p95 {p95:?}, \
         fastest {:?}, slowest {:?}",
        turns.len(),
        turns[0],
        turns[turns.len() - 1]
    );
    assert!(
        median < Duration::from_millis(30),
        "{name}: median {median:?}"
    );
    assert!(p95 < Duration::from_millis(35), "{name}: p95 {p95:?}");
}

/// Looking again at a three-year place that has not changed since it was
/// last looked at: under 15 ms median.
fn look_again(place: usize) {
    let (name, _) = PLACES[place];
    let document = three_years(place);
    let registry = registry();
    let mut session = registry.open_archive(&document.archive).unwrap();
    let mut snapshot = session.snapshot();
    let mut times = Vec::new();
    for _ in 0..21 {
        if let Some(answer) = answer_of(&snapshot) {
            let _ = session.handle(InvokeCommand(answer));
        }
        let _ = session.handle(InvokeCommand(NUDGE_COMMAND.into())).unwrap();
        let started = Instant::now();
        snapshot = session.snapshot();
        times.push(started.elapsed());
    }
    times.sort();
    let median = quantile(&times, 0.5);
    eprintln!(
        "{name}: looking again at an unchanged World after period {DAYS}: median {median:?}, \
         p95 {:?}, slowest {:?}",
        quantile(&times, 0.95),
        times[times.len() - 1]
    );
    assert!(
        median < Duration::from_millis(15),
        "{name}: median {median:?}"
    );
}

#[test]
#[ignore = "a benchmark: run in release (see the top of this file)"]
fn ares_at_three_years_turns_under_thirty_milliseconds_with_its_save() {
    turn_with_its_save(0);
}

#[test]
#[ignore = "a benchmark: run in release (see the top of this file)"]
fn maple_street_at_three_years_turns_under_thirty_milliseconds_with_its_save() {
    turn_with_its_save(1);
}

#[test]
#[ignore = "a benchmark: run in release (see the top of this file)"]
fn icebridge_at_three_years_turns_under_thirty_milliseconds_with_its_save() {
    turn_with_its_save(2);
}

#[test]
#[ignore = "a benchmark: run in release (see the top of this file)"]
fn ares_at_three_years_looks_again_under_fifteen_milliseconds() {
    look_again(0);
}

#[test]
#[ignore = "a benchmark: run in release (see the top of this file)"]
fn maple_street_at_three_years_looks_again_under_fifteen_milliseconds() {
    look_again(1);
}

#[test]
#[ignore = "a benchmark: run in release (see the top of this file)"]
fn icebridge_at_three_years_looks_again_under_fifteen_milliseconds() {
    look_again(2);
}
