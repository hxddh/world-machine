//! One day a day (v0.30 S4): sittings as the World's ledger gives them,
//! played through the harbour as the app plays them. The World is told only
//! that a sitting began, and passes its days by its own card; the player's
//! clock never enters what it records. The ledger itself is host metadata
//! the World's file carries beside its history (1.0), never in it.

use std::collections::BTreeSet;
use std::path::PathBuf;
use tiny_society::TinySocietyBranch;
use world_document::{DocumentSittings, WorldDocument};
use world_library::{DurableWorldSession, WorldDocumentId, WorldLibrary};
use world_observer::{SittingLedger, FIRST_SITTING_DAYS};
use world_projection::{CommandRole, ProjectionIntent, SITTING_BEGINS_COMMAND};

fn temp_root(label: &str) -> PathBuf {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "world-machine-one-day-{}-{nonce}-{label}",
        std::process::id()
    ))
}

/// The day's own card, as the window finds it: by its role.
fn pass_one_day(branch: &mut TinySocietyBranch) {
    let day = branch
        .projection_snapshot()
        .commands
        .into_iter()
        .find(|command| command.role == Some(CommandRole::PassesTime))
        .expect("the day's card");
    branch.invoke_projection_command(&day.id).unwrap();
}

/// A new harbour played through `sittings`, the first on local day `first`
/// and each `gap` local days after the last, every day each allows passed:
/// the World as it then stands.
fn play(first: i64, gap: i64, sittings: i64) -> TinySocietyBranch {
    let mut ledger = SittingLedger::default();
    let mut branch = TinySocietyBranch::new_world().unwrap();
    for n in 0..sittings {
        let today = first + n * gap;
        // The app opens the World twice the same day: one sitting.
        for _ in 0..2 {
            let sitting = ledger.sit(today);
            // The app tells the World on every open until the sitting
            // passes a day; the World keeps its morning to one a day, and
            // refuses the rest.
            if sitting.left == sitting.allowance {
                let came = branch
                    .invoke_projection_command(SITTING_BEGINS_COMMAND)
                    .is_ok();
                assert!(sitting.new || !came, "a second morning the same day");
            }
            for _ in 0..sitting.left {
                pass_one_day(&mut branch);
                ledger.spend_day(today);
            }
        }
    }
    branch
}

fn to_document(ledger: &SittingLedger) -> DocumentSittings {
    DocumentSittings {
        sittings: ledger.sittings,
        began_on: ledger.began_on,
        days_passed: ledger.days_passed,
        morning_read: ledger.morning_read.clone(),
    }
}

/// The same, played the app's way: a Library harbour opened for each
/// sitting, its ledger written into its file as host metadata after every
/// step, and its days passed by its own card. The World's file as it then
/// stands.
fn play_in_file(first: i64, gap: i64, sittings: i64, label: &str) -> (PathBuf, PathBuf) {
    let root = temp_root(label);
    let library = WorldLibrary::new(root.join("Worlds"));
    let mut registry = world_host::WorldRegistry::new();
    registry
        .register(tiny_society::tiny_society_registration())
        .unwrap();
    let id = WorldDocumentId::new("one-day-a-day").unwrap();
    DurableWorldSession::create(
        id.clone(),
        tiny_society::TINY_SOCIETY_PACK_ID,
        &registry,
        &library,
    )
    .unwrap();
    for n in 0..sittings {
        let today = first + n * gap;
        let mut session = DurableWorldSession::open(id.clone(), &registry, &library).unwrap();
        let mut ledger = session
            .sittings()
            .map(|kept| SittingLedger {
                sittings: kept.sittings,
                began_on: kept.began_on,
                days_passed: kept.days_passed,
                morning_read: kept.morning_read.clone(),
            })
            .unwrap_or_default();
        let sitting = ledger.sit(today);
        session
            .set_sittings(to_document(&ledger), &library)
            .unwrap();
        if sitting.left == sitting.allowance {
            let _ = session.handle(
                ProjectionIntent::InvokeCommand(SITTING_BEGINS_COMMAND.into()),
                &registry,
                &library,
            );
        }
        for _ in 0..sitting.left {
            let day = session
                .snapshot()
                .commands
                .into_iter()
                .find(|command| command.role == Some(CommandRole::PassesTime))
                .expect("the day's card");
            session
                .handle(ProjectionIntent::InvokeCommand(day.id), &registry, &library)
                .unwrap();
            ledger.spend_day(today);
            session
                .set_sittings(to_document(&ledger), &library)
                .unwrap();
        }
        session.flush().unwrap();
    }
    (library.path(&id), root)
}

/// The record holds no wall time: the same sittings on two calendars far
/// apart, one a day apart and one a month apart, record the very same
/// events and come to the very same state, and no number of either clock
/// is anywhere in them.
#[test]
fn no_event_or_state_carries_the_wall_clock() {
    // Local days (since 1970) and their midnights in Unix seconds, of two
    // calendars nobody's World could stumble on by chance.
    let (one, two) = (31_337_421_i64, 48_151_623_i64);
    let daily = play(one, 1, 4);
    let monthly = play(two, 30, 4);
    assert_eq!(
        daily.world().events(),
        monthly.world().events(),
        "a month away is one day, as a day away is"
    );
    assert_eq!(daily.world().state(), monthly.world().state());
    let archive = daily.archive_json().unwrap();
    let state = format!("{:?}", daily.world().state());
    for n in 0..4 {
        for day in [one + n, two + 30 * n] {
            for number in [day, day * 86_400] {
                let number = number.to_string();
                assert!(!archive.contains(&number), "{number} in the record");
                assert!(!state.contains(&number), "{number} in the state");
            }
        }
    }
    // And the clock of the run itself is nowhere either.
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        .to_string();
    assert!(
        !archive.contains(&now),
        "this run's own clock in the record"
    );
}

/// The same, with the ledger in the World's file (1.0): the file carries
/// the local day its latest sitting began on, as host metadata beside the
/// history, and nothing else of either clock; the history and the state it
/// replays to are the same on both calendars, and hold no number of
/// either; and replaying the file's events, with its ledger thrown away,
/// is the very same World.
#[test]
fn the_world_file_carries_the_ledger_beside_a_history_without_the_wall_clock() {
    let (one, two) = (31_337_421_i64, 48_151_623_i64);
    let (daily, daily_root) = play_in_file(one, 1, 4, "file-daily");
    let (monthly, monthly_root) = play_in_file(two, 30, 4, "file-monthly");
    let read = |path: &PathBuf| WorldDocument::from_bytes(&std::fs::read(path).unwrap()).unwrap();
    let (daily, monthly) = (read(&daily), read(&monthly));
    // The ledger is the file's: the last sitting's day, on each calendar.
    let kept = daily.metadata.host_sittings.clone().expect("a ledger");
    assert_eq!((kept.sittings, kept.began_on), (4, Some(one + 3)));
    let kept = monthly.metadata.host_sittings.clone().expect("a ledger");
    assert_eq!((kept.sittings, kept.began_on), (4, Some(two + 90)));
    // The history is not: the same on both, with no clock in it.
    assert_eq!(daily.archive.events, monthly.archive.events);
    assert_eq!(daily.archive.world_time, monthly.archive.world_time);
    let history = serde_json::to_string(&daily.archive).unwrap();
    let society = tiny_society::TinySociety::resume_archive(&daily.archive).unwrap();
    let state = format!("{:?}", society.world().state());
    for n in 0..4 {
        for day in [one + n, two + 30 * n] {
            for number in [day, day * 86_400] {
                let number = number.to_string();
                assert!(!history.contains(&number), "{number} in the history");
                assert!(!state.contains(&number), "{number} in the state");
            }
        }
    }
    // Replayed with no ledger at all, it is the same World.
    let mut bare = daily.clone();
    bare.metadata = Default::default();
    let replayed = tiny_society::TinySociety::resume_archive(&bare.archive).unwrap();
    assert_eq!(replayed.world().state(), society.world().state());
    let monthly_society = tiny_society::TinySociety::resume_archive(&monthly.archive).unwrap();
    assert_eq!(monthly_society.world().state(), society.world().state());
    let _ = std::fs::remove_dir_all(daily_root);
    let _ = std::fs::remove_dir_all(monthly_root);
}

/// The first sitting passes three days, every later one one: four sittings
/// are six days, and three mornings (none on the harbour's first day).
#[test]
fn sittings_pass_three_days_then_one_each_with_a_morning_each() {
    let branch = play(1_000, 1, 4);
    let world = branch.world();
    let days = world.events_of_kind(&["day_began"]).len() as u32;
    assert_eq!(days, FIRST_SITTING_DAYS + 3);
    let mornings = world.events_of_kind(&[ritual::MORNING_CAME]);
    assert_eq!(mornings.len(), 3, "a morning each later sitting");
    for morning in &mornings {
        // Each follows its day's beginning, and the rest follows it.
        let cause = world.event(morning.caused_by[0]).unwrap();
        assert_eq!(cause.kind, "day_began");
        let parts = world
            .events()
            .iter()
            .filter(|event| event.caused_by.contains(&morning.id))
            .map(|event| event.kind.as_str())
            .collect::<BTreeSet<_>>();
        assert!(parts.contains(ritual::TENDED), "{parts:?}");
        assert!(parts.contains(ritual::FOUND), "{parts:?}");
    }
}

/// Five days of sittings (the think-aloud test's span): every morning
/// brings something new that is the player's (a find for the book, and
/// something tended), the day's question leads when there is one, and no
/// line of any morning comes twice.
#[test]
fn five_mornings_each_bring_something_new_and_never_a_line_twice() {
    let mut ledger = SittingLedger::default();
    let mut branch = TinySocietyBranch::new_world().unwrap();
    let mut lines = Vec::new();
    let mut finds = 0;
    for today in 500..506 {
        let sitting = ledger.sit(today);
        assert!(sitting.new);
        // A morning comes on every day but the harbour's first.
        let came = branch
            .invoke_projection_command(SITTING_BEGINS_COMMAND)
            .is_ok();
        assert_eq!(came, today > 500);
        let snapshot = branch.projection_snapshot();
        if today > 500 {
            let morning = snapshot.morning.clone().expect("this day's morning");
            let find = morning.find.expect("a find");
            finds += 1;
            lines.push(find.line.clone());
            lines.extend(morning.tended.map(|tended| tended.line));
            if let Some(letter) = morning.letter {
                // Read in town at the start of the day, never "come by soon".
                for stock in ["Hello from me.", "Come by soon."] {
                    assert!(!letter.note.contains(stock), "{}", letter.note);
                }
                lines.push(letter.note);
            }
            if let Some(question) = morning.question {
                // The day's question is one the harbour puts now.
                assert!(snapshot
                    .commands
                    .iter()
                    .any(|command| command.question.as_ref() == Some(&question)));
                lines.push(question.prompt);
            }
            // The book keeps every find so far.
            let kept = snapshot
                .book
                .iter()
                .filter(|entry| entry.shelf == ritual::FINDS_SHELF && entry.found)
                .count();
            assert_eq!(kept, finds);
        } else {
            assert!(
                snapshot.morning.is_none(),
                "the first day opens on its welcome"
            );
        }
        for _ in 0..sitting.left {
            pass_one_day(&mut branch);
            ledger.spend_day(today);
        }
    }
    assert_eq!(finds, 5);
    let unique = lines.iter().collect::<BTreeSet<_>>();
    assert_eq!(unique.len(), lines.len(), "a line came twice: {lines:#?}");
    // A sitting begun twice on one World day brings no second morning: the
    // World refuses it, and records nothing.
    branch
        .invoke_projection_command(SITTING_BEGINS_COMMAND)
        .unwrap();
    let before = branch.world().events().len();
    assert!(branch
        .invoke_projection_command(SITTING_BEGINS_COMMAND)
        .is_err());
    assert_eq!(branch.world().events().len(), before);
}
