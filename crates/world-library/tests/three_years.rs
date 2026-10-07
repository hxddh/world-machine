//! Three years of Tiny Society (1,080 days): what a long-lived World costs
//! to keep, to share and to look at.
//!
//! The World is lived once per run of this file and shared by its tests;
//! in a debug build that takes a few minutes. While working on this file,
//! set `WORLD_MACHINE_THREE_YEARS` to a path to keep it between runs.
//!
//! The snapshot benchmark is timed in a release build:
//!
//! ```text
//! cargo test --release -p world-library --test three_years -- --ignored --nocapture
//! ```

use std::sync::OnceLock;
use std::time::{Duration, Instant};

use world_document::WorldDocument;
use world_host::WorldRegistry;
use world_library::{decode_world_code, encode_world_code, WorldVisit};
use world_persistence::{CheckpointFit, WorldArchive};
use world_projection::ProjectionIntent::InvokeCommand;
use world_projection::ProjectionSnapshot;

const PASS: &str = "tiny-society.let-day-pass";
const DAYS: u64 = 1_080;
/// A year as the v0.19 plan counts one: 360 days.
const A_YEAR: u64 = 360;

/// A warm player's World after `days` days: every day they answer the first
/// question on offer, make something every third day, and let the day pass.
fn lived(days: u64) -> WorldArchive {
    let registry = registry();
    let pack = registry.descriptors()[0].pack.id.clone();
    let mut session = registry.create(&pack).unwrap();
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

/// The three-year World as its owner's file keeps it: its archive, what its
/// snapshot says about it, and its checkpoint.
fn three_years() -> &'static WorldDocument {
    static WORLD: OnceLock<WorldDocument> = OnceLock::new();
    WORLD.get_or_init(|| {
        let cache = std::env::var_os("WORLD_MACHINE_THREE_YEARS").map(std::path::PathBuf::from);
        let archive = match cache.as_ref().and_then(|path| std::fs::read(path).ok()) {
            Some(bytes) => WorldDocument::from_bytes(&bytes).unwrap().archive,
            None => {
                let started = Instant::now();
                let archive = lived(DAYS);
                eprintln!(
                    "lived {DAYS} days ({} events) in {:?}",
                    archive.events.len(),
                    started.elapsed()
                );
                archive
            }
        };
        let document = document(archive);
        if let Some(path) = cache {
            std::fs::write(path, document.to_bytes().unwrap()).unwrap();
        }
        document
    })
}

fn document(archive: WorldArchive) -> WorldDocument {
    let session = registry().open_archive(&archive).unwrap();
    let mut document = WorldDocument::new(archive);
    world_library::describe_from_snapshot(&mut document.metadata, &session.snapshot());
    document.settle_checkpoint();
    document
}

fn registry() -> WorldRegistry {
    world_builtins::registry().unwrap()
}

fn day_length(document: &WorldDocument) -> u64 {
    document.metadata.display_calendar.as_ref().unwrap().length
}

/// The World as it stood at the end of its first year.
fn first_year(document: &WorldDocument) -> WorldDocument {
    let end = A_YEAR * day_length(document);
    let mut archive = document.archive.clone();
    archive.events.retain(|event| event.world_time <= end);
    archive.world_time = end;
    archive.pending.clear();
    WorldDocument {
        archive,
        metadata: document.metadata.clone(),
    }
}

fn megabytes(bytes: usize) -> f64 {
    bytes as f64 / 1_000_000.0
}

#[test]
fn a_three_year_world_saves_under_three_megabytes() {
    let document = three_years();
    assert!(document.archive.world_time >= DAYS * day_length(document));

    // The compact encoding against the tagged one, both as one line.
    let tagged = serde_json::to_vec(&document.archive).unwrap().len();
    let compact = document.to_compact_json().unwrap().len();
    assert!(
        compact * 3 <= tagged,
        "compact {compact} bytes is not a third of tagged {tagged}"
    );

    let started = Instant::now();
    let file = document.to_bytes().unwrap();
    let saved_in = started.elapsed();
    assert!(
        file.len() < 3_000_000,
        "three years saved in {:.2} MB",
        megabytes(file.len())
    );

    let year = first_year(document);
    let year_file = year.to_bytes().unwrap();
    assert!(
        year_file.len() < 1_000_000,
        "a year saved in {:.2} MB",
        megabytes(year_file.len())
    );
    eprintln!(
        "one year: {} events, tagged {:.2} MB, compact {:.2} MB, file {:.2} MB; \
         three years: {} events, tagged {:.2} MB, compact {:.2} MB, file {:.2} MB saved in {saved_in:?}",
        year.archive.events.len(),
        megabytes(serde_json::to_vec(&year.archive).unwrap().len()),
        megabytes(year.to_compact_json().unwrap().len()),
        megabytes(year_file.len()),
        document.archive.events.len(),
        megabytes(tagged),
        megabytes(compact),
        megabytes(file.len()),
    );
}

#[test]
fn a_three_year_world_reopens_event_for_event() {
    let document = three_years();
    let reread = WorldDocument::from_bytes(&document.to_bytes().unwrap()).unwrap();
    assert_eq!(reread.archive.events.len(), document.archive.events.len());
    for (index, (theirs, ours)) in reread
        .archive
        .events
        .iter()
        .zip(&document.archive.events)
        .enumerate()
    {
        assert_eq!(theirs, ours, "event {index} differs");
    }
    assert_eq!(reread.archive, document.archive);
    // The Pack's drawings are kept as JSON numbers, which come back to the
    // nearest float rather than to the last digit; the rest exactly.
    let without_drawings = |document: &WorldDocument| {
        let mut metadata = document.metadata.clone();
        metadata.display_drawings.clear();
        metadata
    };
    assert_eq!(without_drawings(&reread), without_drawings(document));
    assert_eq!(
        reread.metadata.display_drawings.len(),
        document.metadata.display_drawings.len()
    );

    // Its checkpoint is at the start of its latest season, and the full
    // history stays in the file.
    let checkpoint = reread.archive.checkpoint.as_ref().expect("a checkpoint");
    assert_eq!(checkpoint, document.archive.checkpoint.as_ref().unwrap());
    assert_eq!(checkpoint.fit(&reread.archive), Some(CheckpointFit::Within));
    assert!(checkpoint.events > 0 && checkpoint.events <= reread.archive.events.len());

    // Opened from its checkpoint it is the World its full replay is.
    let registry = registry();
    let mut whole = document.archive.clone();
    whole.checkpoint = None;
    let full = registry.open_archive(&whole).unwrap();
    let started = Instant::now();
    let resumed = registry.open_archive(&reread.archive).unwrap();
    eprintln!("opened from its checkpoint in {:?}", started.elapsed());
    assert_eq!(resumed.archive().unwrap().unwrap(), whole);
    assert_eq!(resumed.snapshot(), full.snapshot());
}

/// A three-year World file reads to the same World whichever way it is
/// read: straight into the archive, as World Machine now reads one, or
/// through a tree of JSON values, as it did before.
#[test]
fn a_three_year_world_file_reads_as_it_always_did() {
    use std::io::Read;

    let document = three_years();
    let file = document.to_bytes().unwrap();
    let mut json = Vec::new();
    flate2::read::GzDecoder::new(file.as_slice())
        .read_to_end(&mut json)
        .unwrap();
    let value: serde_json::Value = serde_json::from_slice(&json).unwrap();
    let archive = WorldArchive::from_json_value(&value).unwrap();
    let metadata: world_document::WorldDocumentMetadata =
        serde_json::from_value(value["document"].clone()).unwrap();

    let started = Instant::now();
    let read = WorldDocument::from_bytes(&file).unwrap();
    eprintln!("read a three-year World file in {:?}", started.elapsed());
    assert_eq!(read.archive, archive);
    assert_eq!(read.metadata, metadata);
}

/// The place as a visitor sees it: everything on it, where it stands and
/// how its people spend the day. Only the day each work was built is told
/// from the history before the code's season, so it stays with the owner.
fn as_a_visitor_sees_the_place(
    canvas: &world_projection::CanvasProjection,
) -> world_projection::CanvasProjection {
    let mut canvas = canvas.clone();
    for item in &mut canvas.items {
        item.built = None;
    }
    canvas
}

/// What a visitor sees of a World, which a code must carry: where it
/// stands, who and what is in it, what can be done and what is on offer.
///
/// What is told from the history before the code's season (History, the
/// briefing, the book, keepsakes, letters and chapters) stays behind with
/// the owner's file for now: those are still read from every event rather
/// than kept in the World's state.
fn as_a_visitor_sees(snapshot: &ProjectionSnapshot) -> impl PartialEq + std::fmt::Debug + '_ {
    (
        (
            &snapshot.title,
            snapshot.world_time,
            as_a_visitor_sees_the_place(&snapshot.canvas),
            &snapshot.calendar,
            &snapshot.scenery,
            &snapshot.gauges,
        ),
        (
            &snapshot.commands,
            &snapshot.goals,
            &snapshot.weather,
            &snapshot.drawings,
            &snapshot.exchanges,
            &snapshot.talks,
            &snapshot.collection,
        ),
    )
}

#[test]
fn a_code_reopens_as_the_same_world() {
    let document = three_years();
    let started = Instant::now();
    let code = encode_world_code(document).unwrap();
    eprintln!(
        "a {}-character code, made in {:?}",
        code.len(),
        started.elapsed()
    );
    assert!(code.starts_with("wm2:"));
    // The plan's target: 50,000 characters. The largest part of it is the
    // lives System's memory of the lines heard in its latest ninety days.
    assert!(code.len() < 50_000, "{} characters", code.len());

    // It carries the latest season, where the World stands and what is
    // pending; the history before its checkpoint stays with the owner.
    let decoded = decode_world_code(&code).unwrap();
    let checkpoint = decoded.archive.checkpoint.clone().expect("a checkpoint");
    assert_eq!(checkpoint, *document.archive.checkpoint.as_ref().unwrap());
    assert_eq!(
        decoded.archive.events,
        document.archive.events[checkpoint.events..]
    );
    assert_eq!(decoded.archive.world_time, document.archive.world_time);
    assert_eq!(decoded.archive.pending, document.archive.pending);
    assert_eq!(decoded.archive.pack, document.archive.pack);

    let registry = registry();
    let owner = registry.open_archive(&document.archive).unwrap();
    let visit = WorldVisit::open_code(&code, &registry).unwrap();
    let (theirs, ours) = (visit.snapshot(), owner.snapshot());
    assert_eq!(as_a_visitor_sees(&theirs), as_a_visitor_sees(&ours));
    assert_eq!(
        visit.world_code().unwrap(),
        code,
        "a visit's code is the same code"
    );
}

/// The snapshot benchmark at three years. Debug builds are not
/// representative; run it in release (see the top of this file).
///
/// Each snapshot is timed on a different day: the player answers and lets
/// a day pass, as they would, and the World is looked at as it then
/// stands. The session's own snapshot after the day has already been
/// taken, and the World keeps what it showed for as long as it stands as
/// it does (as a Pack in a process of its own always has), so the look
/// timed here costs what looking again at an unchanged World does: a copy,
/// and whatever a session adds. What the first look after a day costs is
/// in the day's own time, printed here and barred with the save in
/// `a_three_year_turn_with_its_save_takes_under_thirty_five_milliseconds`.
#[test]
#[ignore]
fn a_three_year_snapshot_takes_under_fifteen_milliseconds() {
    let document = three_years();
    let registry = registry();
    let mut session = registry.open_archive(&document.archive).unwrap();
    let mut snapshot = session.snapshot();
    let started = Instant::now();
    let file = document.to_bytes().unwrap();
    eprintln!("saved {} bytes in {:?}", file.len(), started.elapsed());
    let started = Instant::now();
    let _ = WorldDocument::from_bytes(&file).unwrap();
    eprintln!("read in {:?}", started.elapsed());
    // More days, for profiling: WORLD_MACHINE_SNAPSHOTS=200.
    let count = std::env::var("WORLD_MACHINE_SNAPSHOTS")
        .ok()
        .and_then(|count| count.parse().ok())
        .unwrap_or(21);
    let mut times = Vec::new();
    let mut days = Vec::new();
    for _ in 0..count {
        if let Some(answer) = snapshot
            .commands
            .iter()
            .find(|c| c.question.is_some() && c.unavailable.is_none() && c.id != PASS)
        {
            let _ = session.handle(InvokeCommand(answer.id.clone()));
        }
        let started = Instant::now();
        let _ = session.handle(InvokeCommand(PASS.into())).unwrap();
        days.push(started.elapsed());
        let started = Instant::now();
        snapshot = session.snapshot();
        times.push(started.elapsed());
    }
    times.sort();
    days.sort();
    let median = times[times.len() / 2];
    eprintln!(
        "snapshot on {count} days after day {DAYS}: median {median:?}, fastest {:?}, slowest {:?}; \
         the day itself with its first snapshot, no save: median {:?}",
        times[0],
        times[times.len() - 1],
        days[days.len() / 2],
    );
    assert!(median < Duration::from_millis(15), "median {median:?}");
}

/// A turn at three years as the app plays one: the World opened from its
/// file, then each day an answer and the day let pass, each change handed
/// to the World's writer (which makes and writes the file away from the
/// turn, in order) as it is kept, and the snapshot after it returned. In
/// release (see the top of this file).
#[test]
#[ignore]
fn a_three_year_turn_with_its_save_takes_under_thirty_five_milliseconds() {
    let document = three_years();
    let registry = registry();
    let dir = std::env::temp_dir().join(format!("three-years-turn-{}", std::process::id()));
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
        .unwrap_or(15);
    let mut turns = Vec::new();
    for _ in 0..count {
        if let Some(answer) = snapshot
            .commands
            .iter()
            .find(|c| c.question.is_some() && c.unavailable.is_none() && c.id != PASS)
        {
            let started = Instant::now();
            if session
                .handle(InvokeCommand(answer.id.clone()), &registry, &library)
                .is_ok()
            {
                turns.push(started.elapsed());
            }
        }
        let started = Instant::now();
        snapshot = session
            .handle(InvokeCommand(PASS.into()), &registry, &library)
            .unwrap();
        turns.push(started.elapsed());
    }
    // Written away from the turns (see `writer`): all of it is on disk
    // once the World is flushed, as it is when it is closed.
    let started = Instant::now();
    session.flush().unwrap();
    let flushed = started.elapsed();
    let on_disk = WorldDocument::from_bytes(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(on_disk.archive.world_time, snapshot.world_time);
    assert_eq!(
        on_disk.archive.events.len(),
        session.current_archive().unwrap().events.len()
    );
    eprintln!("the last turn's file on disk {flushed:?} after it");
    let _ = std::fs::remove_dir_all(&dir);
    turns.sort();
    let median = turns[turns.len() / 2];
    eprintln!(
        "{} turns with their saves after day {DAYS}: median {median:?}, fastest {:?}, slowest {:?}",
        turns.len(),
        turns[0],
        turns[turns.len() - 1]
    );
    assert!(median < Duration::from_millis(35), "median {median:?}");
}

/// How much memory a three-year World holds in the app: its file opened
/// as the app opens it, then 20 days played as the app plays them (an
/// answer and the day let pass, each saved), in a process of its own so
/// nothing else this file holds is counted. At most 100 MB resident after
/// the 20 days (104–105 MB at v0.26). Linux only (it reads
/// `/proc/self/status`), in release (see the top of this file).
#[test]
#[ignore]
fn a_three_year_world_holds_at_most_a_hundred_megabytes_after_twenty_days() {
    if !cfg!(target_os = "linux") {
        return;
    }
    // In the process of its own: open, play, and say what it holds.
    if let Some(path) = std::env::var_os("WORLD_MACHINE_RSS_WORLD") {
        let (opened, after) = play_twenty_days(std::path::Path::new(&path));
        println!("\nrss opened {opened} after {after}");
        return;
    }
    let document = three_years();
    let dir = std::env::temp_dir().join(format!("three-years-rss-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("three.world");
    std::fs::write(&path, document.to_bytes().unwrap()).unwrap();
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "a_three_year_world_holds_at_most_a_hundred_megabytes_after_twenty_days",
            "--ignored",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("WORLD_MACHINE_RSS_WORLD", &path)
        .output()
        .unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    let stdout = String::from_utf8_lossy(&output.stdout);
    // After the test harness's own "test … ..." on the same line.
    let line = stdout
        .lines()
        .find_map(|line| line.split_once("rss opened ").map(|(_, rest)| rest))
        .unwrap_or_else(|| panic!("the World was played: {stdout}"));
    let numbers = line
        .split_whitespace()
        .filter_map(|word| word.parse::<f64>().ok())
        .collect::<Vec<_>>();
    let (opened, after) = (numbers[0], numbers[1]);
    eprintln!("resident: {opened:.1} MB opened, {after:.1} MB after 20 days");
    assert!(after <= 100.0, "{after:.1} MB after 20 days");
}

/// Opens the World file at `path` as the app does and plays 20 days on it;
/// the resident memory, in MB, once it is open and after the 20 days.
fn play_twenty_days(path: &std::path::Path) -> (f64, f64) {
    let registry = registry();
    let library = world_library::WorldLibrary::new(path.with_file_name("library"));
    let mut session =
        world_library::DurableWorldSession::open_file(path.to_path_buf(), &registry).unwrap();
    let mut snapshot = session.snapshot();
    let opened = resident_megabytes();
    for _ in 0..20 {
        if let Some(answer) = snapshot
            .commands
            .iter()
            .find(|c| c.question.is_some() && c.unavailable.is_none() && c.id != PASS)
        {
            let _ = session.handle(InvokeCommand(answer.id.clone()), &registry, &library);
        }
        snapshot = session
            .handle(InvokeCommand(PASS.into()), &registry, &library)
            .unwrap();
        let _ = session.snapshot();
    }
    session.flush().unwrap();
    (opened, resident_megabytes())
}

/// This process's resident memory in MB, from `/proc/self/status`.
fn resident_megabytes() -> f64 {
    let status = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    status
        .lines()
        .find(|line| line.starts_with("VmRSS:"))
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|kb| kb.parse::<f64>().ok())
        .map_or(0.0, |kb| kb / 1024.0)
}

/// What a three-year World's snapshot weighs on the wire, as a Pack in a
/// process of its own sends it: 1,243,230 bytes of JSON at v0.25, with
/// room for a tenth more before this fails.
#[test]
fn a_three_year_snapshot_stays_within_a_tenth_of_its_v0_25_size_on_the_wire() {
    let document = three_years();
    let session = registry().open_archive(&document.archive).unwrap();
    let wire = world_pack_protocol::ProjectionSnapshotWire::from(&session.snapshot());
    let bytes = serde_json::to_vec(&wire).unwrap().len();
    eprintln!("a three-year snapshot is {bytes} bytes on the wire");
    assert!(bytes <= 1_367_000, "{bytes} bytes");
}
