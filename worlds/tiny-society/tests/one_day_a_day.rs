//! One day a day (v0.30 S4): sittings as the observer's ledger gives them,
//! played through the harbour as the app plays them. The World is told only
//! that a sitting began, and passes its days by its own card; the player's
//! clock never enters what it records.

use std::collections::BTreeSet;
use std::path::PathBuf;
use tiny_society::TinySocietyBranch;
use world_observer::{ObserverKey, ObserverStore, FIRST_SITTING_DAYS};
use world_projection::{CommandRole, SITTING_BEGINS_COMMAND};

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
fn play(first: i64, gap: i64, sittings: i64, label: &str) -> TinySocietyBranch {
    let root = temp_root(label);
    let store = ObserverStore::new(root.clone());
    let key = ObserverKey::new("library:one-day-a-day").unwrap();
    let mut branch = TinySocietyBranch::new_world().unwrap();
    for n in 0..sittings {
        let today = first + n * gap;
        // The app opens the World twice the same day: one sitting.
        for _ in 0..2 {
            let sitting = store.sit(&key, today).unwrap();
            if sitting.new {
                // The harbour's own morning, if it has one today.
                branch
                    .invoke_projection_command(SITTING_BEGINS_COMMAND)
                    .unwrap();
            }
            for _ in 0..sitting.left {
                pass_one_day(&mut branch);
                store.spend_day(&key, today).unwrap();
            }
        }
    }
    let _ = std::fs::remove_dir_all(root);
    branch
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
    let daily = play(one, 1, 4, "daily");
    let monthly = play(two, 30, 4, "monthly");
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

/// The first sitting passes three days, every later one one: four sittings
/// are six days, and three mornings (none on the harbour's first day).
#[test]
fn sittings_pass_three_days_then_one_each_with_a_morning_each() {
    let branch = play(1_000, 1, 4, "count");
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
    let root = temp_root("five");
    let store = ObserverStore::new(root.clone());
    let key = ObserverKey::new("library:five-mornings").unwrap();
    let mut branch = TinySocietyBranch::new_world().unwrap();
    let mut lines = Vec::new();
    let mut finds = 0;
    for today in 500..506 {
        let sitting = store.sit(&key, today).unwrap();
        assert!(sitting.new);
        branch
            .invoke_projection_command(SITTING_BEGINS_COMMAND)
            .unwrap();
        let snapshot = branch.projection_snapshot();
        if today > 500 {
            let morning = snapshot.morning.clone().expect("this day's morning");
            let find = morning.find.expect("a find");
            finds += 1;
            lines.push(find.line.clone());
            lines.extend(morning.tended.map(|tended| tended.line));
            lines.extend(morning.letter.map(|letter| letter.note));
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
            store.spend_day(&key, today).unwrap();
        }
    }
    assert_eq!(finds, 5);
    let unique = lines.iter().collect::<BTreeSet<_>>();
    assert_eq!(unique.len(), lines.len(), "a line came twice: {lines:#?}");
    // A sitting begun twice on one World day brings no second morning.
    branch
        .invoke_projection_command(SITTING_BEGINS_COMMAND)
        .unwrap();
    let before = branch.world().events().len();
    branch
        .invoke_projection_command(SITTING_BEGINS_COMMAND)
        .unwrap();
    assert_eq!(branch.world().events().len(), before);
    let _ = std::fs::remove_dir_all(root);
}
