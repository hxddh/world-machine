//! Replay-identical fixtures.
//!
//! A Pack's oldest and newest saved Worlds in `tests/fixtures` are opened,
//! replayed and played on by a fixed, deterministic builder. The `Debug`
//! text of every snapshot along the way, and of the event log at the end,
//! is digested and compared with a golden file written by the code before a
//! refactor. A refactor that is meant to change nothing must leave every
//! digest alone. Every other fixture is replayed and played on without a
//! golden file ([`assert_every_fixture_replays_and_continues`]): until
//! REVIEW_v0.29 S8 each had digests of its own, re-blessed on every
//! intended change.
//!
//! The text is digested as it is formatted ([`Digest`]), never kept: about
//! 3 MB a snapshot, 2 GB a run, that v0.26 built as strings only to hash
//! them. The digests are the same as of the whole text, so the golden files
//! did not change. Every day is played; in a debug build (the push CI) only
//! every fifth day's snapshot is digested and checked, and in release (the
//! nightly) every day's ([`digest_every`]).
//!
//! - `WORLD_PACK_GOLDEN_EVERY=<n>` digests every n-th day instead.
//! - `WORLD_PACK_BLESS=1` rewrites the golden files (only on a tree whose
//!   behaviour you trust).
//! - `WORLD_PACK_DUMP=<dir>` writes the full text of every step, to diff.

use std::fmt::{self, Debug, Write as _};
use std::path::{Path, PathBuf};

use world_document::WorldDocument;
use world_host::{WorldRegistry, WorldSession};
use world_pack_protocol::ProjectionSnapshotWire;
use world_projection::ProjectionIntent::InvokeCommand;
use world_projection::ProjectionSnapshot;

/// One step of a replay trace: a label, the digest of its text, and the
/// text itself only when it is to be written out (`WORLD_PACK_DUMP`).
pub struct Step {
    pub label: String,
    pub digest: Digest,
    pub text: Option<String>,
}

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// 64-bit FNV-1a: small, stable and dependency-free.
pub fn fnv64(bytes: &[u8]) -> u64 {
    let mut digest = Digest::new();
    digest.add(bytes);
    digest.hash()
}

/// [`fnv64`] and the length of a text, taken as it is written, so the text
/// never has to be held: write a value's `Debug` into it with `write!`, or
/// its JSON with `serde_json::to_writer`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Digest {
    hash: u64,
    len: usize,
}

impl Default for Digest {
    fn default() -> Self {
        Self::new()
    }
}

impl Digest {
    pub fn new() -> Self {
        Self {
            hash: FNV_OFFSET,
            len: 0,
        }
    }

    /// The digest of `value`'s pretty `Debug` text (`{:#?}`).
    pub fn of_debug(value: &impl Debug) -> Self {
        let mut digest = Self::new();
        let _ = write!(digest, "{value:#?}");
        digest
    }

    pub fn add(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.hash ^= u64::from(*byte);
            self.hash = self.hash.wrapping_mul(FNV_PRIME);
        }
        self.len += bytes.len();
    }

    /// [`fnv64`] of everything written.
    pub fn hash(&self) -> u64 {
        self.hash
    }

    /// How many bytes were written.
    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl fmt::Write for Digest {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        self.add(text.as_bytes());
        Ok(())
    }
}

impl std::io::Write for Digest {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.add(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Which days' snapshots are digested and checked against the golden
/// files: every `n`-th day (and the last). Every day in a release build and
/// when blessing; every fifth in debug, where formatting a snapshot's text
/// costs most of the test. `WORLD_PACK_GOLDEN_EVERY` overrides it.
pub fn digest_every() -> usize {
    if std::env::var_os("WORLD_PACK_BLESS").is_some()
        || std::env::var_os("WORLD_MACHINE_BLESS").is_some()
    {
        return 1;
    }
    std::env::var("WORLD_PACK_GOLDEN_EVERY")
        .ok()
        .and_then(|every| every.parse().ok())
        .filter(|every| *every > 0)
        .unwrap_or(if cfg!(debug_assertions) { 5 } else { 1 })
}

/// Whether day `day` of `days` is digested, digesting every `every`-th.
pub fn digested(day: usize, days: usize, every: usize) -> bool {
    day.is_multiple_of(every) || day == days
}

/// Checks `digest` (lines of `<label> …` or `… <label>`, as made here)
/// against `golden`, line by line, matching labels: each line made must be
/// in the golden file as it is. With every step digested, the two must be
/// the same; with fewer, the golden file may have more. `None` if they
/// agree, else the first difference.
pub(crate) fn compare(expected: &str, digest: &str, every_step: bool) -> Option<String> {
    if every_step {
        if expected == digest {
            return None;
        }
        return Some(
            expected
                .lines()
                .zip(digest.lines())
                .find(|(a, b)| a != b)
                .map(|(a, b)| format!("expected `{a}`, got `{b}`"))
                .unwrap_or_else(|| "a different number of steps".into()),
        );
    }
    let mut golden = expected.lines();
    for line in digest.lines() {
        // In order: the golden file's lines are skipped up to this one.
        if !golden.any(|golden| golden == line) {
            return Some(format!("got `{line}`, which the golden file does not have"));
        }
    }
    None
}

/// A step of `value`'s `Debug` text: digested as it is formatted, and kept
/// only when `keep` (to be written out).
fn step(label: String, value: &impl Debug, keep: bool) -> Step {
    Step {
        label,
        digest: Digest::of_debug(value),
        text: keep.then(|| format!("{value:#?}")),
    }
}

/// The builder every fixture test uses: the first open question each day;
/// every third day a plot built when one is on offer, or else something made
/// by hand; then the day let pass with `pass`.
pub fn builder(session: &mut Box<dyn WorldSession>, pass: &str, days: usize) {
    for day in 1..=days {
        builder_day(session, pass, day, false);
    }
}

/// One builder day. With `strict`, a command the snapshot offered as
/// available must be accepted: the fixtures without a golden digest rely on
/// it to notice an answer or deed that has stopped working.
fn builder_day(session: &mut Box<dyn WorldSession>, pass: &str, day: usize, strict: bool) {
    let snapshot = session.snapshot();
    if let Some(answer) = snapshot
        .commands
        .iter()
        .find(|c| c.question.is_some() && c.unavailable.is_none() && c.id != pass)
    {
        let taken = session.handle(InvokeCommand(answer.id.clone()));
        if strict {
            if let Err(error) = taken {
                panic!(
                    "day {day}: the available answer {} was refused: {error:?}",
                    answer.id
                );
            }
        }
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
            let done = session.handle(InvokeCommand(deed.clone()));
            if strict {
                if let Err(error) = done {
                    panic!("day {day}: the available deed {deed} was refused: {error:?}");
                }
            }
        }
    }
    session
        .handle(InvokeCommand(pass.into()))
        .expect("the day passes");
}

/// Opens `fixture`, checks it replays event for event, then plays it on for
/// `days` days, recording the snapshot after opening and after every day, and
/// the event log at the end; each step's text is kept only with `keep`.
pub fn trace(
    registry: &WorldRegistry,
    fixture: &Path,
    pass: &str,
    days: usize,
    keep: bool,
) -> Vec<Step> {
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
    let every = digest_every();
    let mut steps = vec![step("open".into(), &session.snapshot(), keep)];
    for day in 1..=days {
        builder_day(&mut session, pass, day, false);
        if digested(day, days, every) {
            steps.push(step(format!("day-{day:03}"), &session.snapshot(), keep));
        }
    }
    let after = session.archive().unwrap().unwrap();
    assert_eq!(after.events[..archive.events.len()], archive.events[..]);
    steps.push(step(
        "events".into(),
        &&after.events[archive.events.len()..],
        keep,
    ));
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

/// Fails if two fixtures in `dir` are the same file: each release's
/// fixture must add a World of its own, not run the same one twice.
pub fn assert_fixtures_differ(dir: &Path) {
    let mut seen: Vec<(Vec<u8>, PathBuf)> = Vec::new();
    let mut same = Vec::new();
    for fixture in fixtures(dir) {
        let bytes = std::fs::read(&fixture).expect("the fixture");
        if let Some((_, first)) = seen.iter().find(|(known, _)| *known == bytes) {
            same.push(format!("{} = {}", first.display(), fixture.display()));
        }
        seen.push((bytes, fixture));
    }
    assert!(seen.len() > 1, "fixtures in {}", dir.display());
    assert!(same.is_empty(), "the same file twice:\n{}", same.join("\n"));
}

/// The fixtures in `fixtures_dir` named by `goldens` (file stems, such as
/// `v020-<place>`): the ones whose play-on is pinned to golden digests.
/// Each must exist; none is skipped.
pub fn golden_fixtures(fixtures_dir: &Path, goldens: &[&str]) -> Vec<PathBuf> {
    assert!(!goldens.is_empty(), "no golden fixture named");
    goldens
        .iter()
        .map(|stem| {
            let fixture = fixtures_dir.join(format!("{stem}.world"));
            assert!(fixture.exists(), "no fixture {}", fixture.display());
            fixture
        })
        .collect()
}

/// Traces the fixtures named by `goldens` ([`golden_fixtures`]) and compares
/// the digests with `golden_dir/<fixture>.digest`, which must be there
/// unless blessing. Only a representative few are pinned, the oldest and
/// the newest of a Pack's fixtures (REVIEW_v0.29 S8): every fixture's replay
/// and play-on is checked without a golden file by
/// [`assert_every_fixture_replays_and_continues`], and the state its events
/// rebuild by [`crate::invariants::assert_replayed_state_never_changes`].
pub fn assert_replay_identical(
    registry: &WorldRegistry,
    fixtures_dir: &Path,
    golden_dir: &Path,
    goldens: &[&str],
    pass: &str,
    days: usize,
) {
    let bless = std::env::var_os("WORLD_PACK_BLESS").is_some();
    let dump = std::env::var_os("WORLD_PACK_DUMP").map(PathBuf::from);
    let mut failures = Vec::new();
    for fixture in golden_fixtures(fixtures_dir, goldens) {
        let stem = fixture.file_stem().unwrap().to_string_lossy().to_string();
        let golden = golden_dir.join(format!("{stem}.digest"));
        assert!(
            bless || golden.exists(),
            "no golden digest {}: run with WORLD_PACK_BLESS=1",
            golden.display()
        );
        let steps = trace(registry, &fixture, pass, days, dump.is_some());
        let mut digest = String::new();
        for step in &steps {
            let _ = writeln!(
                digest,
                "{} {:016x} {}",
                step.label,
                step.digest.hash(),
                step.digest.len()
            );
        }
        if let Some(dir) = &dump {
            let dir = dir.join(&stem);
            std::fs::create_dir_all(&dir).unwrap();
            for step in &steps {
                let text = step.text.as_deref().unwrap_or_default();
                std::fs::write(dir.join(format!("{}.txt", step.label)), text).unwrap();
            }
        }
        if bless {
            std::fs::create_dir_all(golden_dir).unwrap();
            std::fs::write(&golden, &digest).unwrap();
            continue;
        }
        let expected = std::fs::read_to_string(&golden).unwrap();
        if let Some(first) = compare(&expected, &digest, digest_every() == 1) {
            failures.push(format!("{stem}: {first}"));
        }
    }
    assert!(
        failures.is_empty(),
        "not replay-identical:\n{}",
        failures.join("\n")
    );
}

/// A snapshot's process wire form (`ProjectionSnapshotWire`, what a Pack
/// process sends), written as JSON and read back: the JSON decodes to the
/// same wire value, and that to the same snapshot.
pub fn assert_wire_decodes(snapshot: &ProjectionSnapshot, at: &str) {
    let wire = ProjectionSnapshotWire::from(snapshot);
    let json = serde_json::to_string(&wire).expect("snapshot JSON");
    let decoded: ProjectionSnapshotWire =
        serde_json::from_str(&json).unwrap_or_else(|error| panic!("{at}: {error}"));
    assert!(
        decoded == wire,
        "{at}: the wire JSON reads back differently"
    );
    let back =
        ProjectionSnapshot::try_from(decoded).unwrap_or_else(|error| panic!("{at}: {error:?}"));
    assert!(
        back == *snapshot,
        "{at}: the wire form decodes to another snapshot"
    );
}

/// Every fixture in `fixtures_dir`, with no golden file: it replays event
/// for event, opens to the same snapshot each time, and plays on `days`
/// days by the [`builder`] without a panic. What is played on only appends
/// to the stored events; each day's snapshot is the same looked at again,
/// and its wire JSON decodes back to it; and the World played on reopens
/// from its own events to the very snapshot it showed. The cheap guard of
/// what the golden play-on digests once pinned for every fixture
/// (REVIEW_v0.29 S8); the oldest and the newest keep theirs
/// ([`assert_replay_identical`]).
pub fn assert_every_fixture_replays_and_continues(
    registry: &WorldRegistry,
    fixtures_dir: &Path,
    pass: &str,
    days: usize,
) {
    let all = fixtures(fixtures_dir);
    assert!(!all.is_empty(), "no fixture in {}", fixtures_dir.display());
    for fixture in all {
        let at = fixture.file_stem().unwrap().to_string_lossy().to_string();
        let bytes = std::fs::read(&fixture).expect("the fixture");
        let archive = WorldDocument::from_bytes(&bytes).unwrap().archive;
        let mut session = registry
            .open_archive(&archive)
            .unwrap_or_else(|error| panic!("{at} opens: {error:?}"));
        let replayed = session.archive().unwrap().unwrap();
        assert!(
            replayed.events == archive.events,
            "{at}: replays event for event"
        );
        let opened = session.snapshot();
        assert!(
            registry.open_archive(&archive).unwrap().snapshot() == opened,
            "{at}: opens to the same snapshot each time"
        );
        assert_wire_decodes(&opened, &format!("{at} opened"));
        for day in 1..=days {
            builder_day(&mut session, pass, day, true);
            let snapshot = session.snapshot();
            assert!(
                session.snapshot() == snapshot,
                "{at} day {day}: the same looked at again"
            );
            assert_wire_decodes(&snapshot, &format!("{at} day {day}"));
        }
        let after = session.archive().unwrap().unwrap();
        let before = archive.events.len();
        assert!(
            after.events.len() > before,
            "{at}: playing on records events"
        );
        assert!(
            after.events[..before] == archive.events[..],
            "{at}: playing on only appends"
        );
        let reopened = registry
            .open_archive(&after)
            .unwrap_or_else(|error| panic!("{at} played on reopens: {error:?}"));
        assert!(
            reopened.archive().unwrap().unwrap().events == after.events,
            "{at}: played on, it replays event for event"
        );
        assert!(
            reopened.snapshot() == session.snapshot(),
            "{at}: played on, it reopens to the snapshot it showed"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A digest taken as text is written is the digest of the whole text,
    /// however it arrives: the golden files written from whole strings
    /// still hold.
    #[test]
    fn a_streamed_digest_is_the_digest_of_the_whole_text() {
        let value = vec![("harbour", 3_u64, Some(1.5_f32)), ("quay", 4, None)];
        let whole = format!("{value:#?}");
        let streamed = Digest::of_debug(&value);
        assert_eq!(streamed.hash(), fnv64(whole.as_bytes()));
        assert_eq!(streamed.len(), whole.len());
        let mut json = Digest::new();
        serde_json::to_writer(&mut json, &value).unwrap();
        let text = serde_json::to_string(&value).unwrap();
        assert_eq!(
            (json.hash(), json.len()),
            (fnv64(text.as_bytes()), text.len())
        );
        // Known values: FNV-1a of the empty text is its offset basis.
        assert_eq!(fnv64(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv64(b"a"), 0xaf63_dc4c_8601_ec8c);
    }

    /// Digesting fewer days still checks each one digested against the
    /// golden file, in order; digesting every day checks it all.
    #[test]
    fn fewer_days_are_checked_against_the_lines_they_match() {
        let golden = "open 1 10\nday-001 2 10\nday-002 3 10\nevents 4 10\n";
        assert_eq!(
            compare(golden, "open 1 10\nday-002 3 10\nevents 4 10\n", false),
            None
        );
        assert!(compare(golden, "open 1 10\nday-002 9 10\n", false).is_some());
        assert!(compare(golden, "day-002 3 10\nopen 1 10\n", false).is_some());
        assert!(compare(golden, "open 1 10\nday-002 3 10\n", true).is_some());
        assert_eq!(compare(golden, golden, true), None);
        assert!(digested(5, 30, 5) && digested(30, 30, 5) && digested(7, 7, 5));
        assert!(!digested(4, 30, 5));
    }
}
