//! Thirty sittings without a repeat (1.0 P4): a new harbour played as a
//! player plays it, its first sitting and twenty-nine more, one a day, as
//! the host's sitting ledger gives them. Every morning card brings
//! something new: no letter, nothing looked after and no find is ever told
//! twice; what is looked after moves round the harbour; letters are worded
//! in turn, and at least a quarter of them cite what the player did.

use std::collections::{BTreeMap, BTreeSet};
use tiny_society::TinySocietyBranch;
use world_observer::{SittingLedger, FIRST_SITTING_DAYS};
use world_projection::{CommandRole, Morning, SelectionId, SITTING_BEGINS_COMMAND};

/// The sittings played: the first, and twenty-nine more.
const SITTINGS: i64 = 30;

/// The kinds of Event that are something the player did: made, moved,
/// named or gave with their own hands, an answer, a favour.
const PLAYERS: [&str; 9] = [
    "built_by_hand",
    "decorated_by_hand",
    "planted_by_hand",
    "moved_by_hand",
    "named",
    "gift_given",
    "invited_out",
    "situation_answered",
    "favour_done",
];

/// What a player does with a day: answers what they are asked, every
/// third day makes something with their own hands, then lets the day
/// pass by its own card.
fn play_day(branch: &mut TinySocietyBranch, day: u64) {
    let snapshot = branch.projection_snapshot();
    if let Some(answer) = snapshot.commands.iter().find(|command| {
        command.question.is_some()
            && command.unavailable.is_none()
            && command.role != Some(CommandRole::PassesTime)
    }) {
        let _ = branch.invoke_projection_command(&answer.id.clone());
    }
    if day.is_multiple_of(3) {
        let snapshot = branch.projection_snapshot();
        let deeds = snapshot
            .commands
            .iter()
            .filter(|command| {
                command.unavailable.is_none()
                    && command.role != Some(CommandRole::PassesTime)
                    && command
                        .hand
                        .as_ref()
                        .is_some_and(|hand| hand.verb != "Undo" && hand.verb != "Suggest")
            })
            .collect::<Vec<_>>();
        if !deeds.is_empty() {
            let deed = deeds[(day as usize / 3) % deeds.len()].id.clone();
            let _ = branch.invoke_projection_command(&deed);
        }
    }
    let pass = branch
        .projection_snapshot()
        .commands
        .into_iter()
        .find(|command| command.role == Some(CommandRole::PassesTime))
        .expect("the day's card");
    branch.invoke_projection_command(&pass.id).unwrap();
}

/// Every morning card of the sittings, in order, with how many finds the
/// book held after each; and the World as they left it.
fn sittings() -> (Vec<(Morning, usize)>, TinySocietyBranch) {
    let mut ledger = SittingLedger::default();
    let mut branch = TinySocietyBranch::new_world().unwrap();
    let mut mornings = Vec::new();
    let mut day = 0;
    let read = |branch: &TinySocietyBranch, mornings: &mut Vec<(Morning, usize)>| {
        let snapshot = branch.projection_snapshot();
        let finds = snapshot
            .book
            .iter()
            .filter(|entry| entry.shelf == ritual::FINDS_SHELF && entry.found)
            .count();
        mornings.push((snapshot.morning.expect("this day's morning"), finds));
    };
    for today in 900..900 + SITTINGS {
        let sitting = ledger.sit(today);
        assert!(sitting.new);
        let came = branch
            .invoke_projection_command(SITTING_BEGINS_COMMAND)
            .is_ok();
        assert_eq!(came, today > 900, "a morning every sitting but the first");
        if came {
            read(&branch, &mut mornings);
        }
        for left in (0..sitting.left).rev() {
            day += 1;
            play_day(&mut branch, day);
            ledger.spend_day(today);
            // The first sitting's later days open on their mornings as
            // they come.
            if today == 900 && left > 0 {
                read(&branch, &mut mornings);
            }
        }
    }
    (mornings, branch)
}

fn first_sentence(note: &str) -> &str {
    note.split_inclusive(['.', '!', '?'])
        .next()
        .unwrap_or(note)
        .trim()
}

fn last_sentence(note: &str) -> &str {
    note.trim_end_matches(['.', '!', '?'])
        .rsplit(['.', '!', '?'])
        .next()
        .unwrap_or(note)
        .trim()
}

#[test]
fn thirty_sittings_bring_something_new_every_morning_and_never_a_line_twice() {
    let (mornings, branch) = sittings();
    let world = branch.world();
    assert_eq!(
        mornings.len() as i64,
        i64::from(FIRST_SITTING_DAYS) - 1 + SITTINGS - 1,
        "a morning each later day of the first sitting, and each later sitting"
    );

    // Something new every sitting: a find the book never had, and
    // something looked after.
    for (n, (morning, finds)) in mornings.iter().enumerate() {
        assert!(morning.find.is_some(), "morning {n} found nothing");
        assert!(morning.tended.is_some(), "morning {n} looked after nothing");
        assert!(morning.letter.is_some(), "morning {n} brought no letter");
        assert_eq!(*finds, n + 1, "morning {n}: the book grows by one");
    }

    // No line of any morning card, verbatim, twice.
    let lines = mornings
        .iter()
        .flat_map(|(morning, _)| {
            morning
                .letter
                .iter()
                .map(|letter| letter.note.clone())
                .chain(morning.tended.iter().map(|tended| tended.line.clone()))
                .chain(morning.find.iter().map(|find| find.line.clone()))
        })
        .collect::<Vec<_>>();
    let mut seen = BTreeSet::new();
    let twice = lines
        .iter()
        .filter(|line| !seen.insert(line.as_str()))
        .collect::<Vec<_>>();
    assert!(twice.is_empty(), "told twice: {twice:#?}");

    // What is looked after moves round the harbour: never the same thing
    // two mornings running, nor anything more than one morning in four;
    // the player's things and the harbour's places both have their turn.
    let tended = mornings
        .iter()
        .filter_map(|(morning, _)| morning.tended.as_ref()?.about)
        .collect::<Vec<_>>();
    assert_eq!(tended.len(), mornings.len());
    for two in tended.windows(2) {
        assert_ne!(two[0], two[1], "the same thing two mornings running");
    }
    let mut times = BTreeMap::<SelectionId, usize>::new();
    for about in &tended {
        *times.entry(*about).or_default() += 1;
    }
    let most = times.values().max().copied().unwrap_or(0);
    assert!(
        most * 4 <= tended.len(),
        "one thing {most} times: {times:?}"
    );
    assert!(
        times.len() >= 10,
        "only {} things looked after",
        times.len()
    );
    let theirs = mornings
        .iter()
        .filter(|(morning, _)| {
            morning
                .tended
                .as_ref()
                .is_some_and(|tended| tended.line.contains(" your "))
        })
        .count();
    assert!(
        theirs * 4 >= mornings.len() && theirs * 4 <= mornings.len() * 3,
        "{theirs} of {} about the player's things",
        mornings.len()
    );

    // The finds turn up all round the harbour.
    let places = mornings
        .iter()
        .filter_map(|(morning, _)| morning.find.as_ref()?.about)
        .collect::<BTreeSet<_>>();
    assert!(places.len() >= 4, "finds only at {places:?}");

    // At least a quarter of the letters cite what the player did, and say
    // so by naming it as their cause.
    let letters = mornings
        .iter()
        .filter_map(|(morning, _)| morning.letter.as_ref())
        .collect::<Vec<_>>();
    let citing = letters
        .iter()
        .filter(|letter| {
            let SelectionId::Event(id) = letter.moment else {
                return false;
            };
            world.event(id).is_some_and(|event| {
                event
                    .caused_by
                    .iter()
                    .filter_map(|cause| world.event(*cause))
                    .any(|cause| PLAYERS.contains(&cause.kind.as_str()))
            })
        })
        .count();
    assert!(
        citing * 4 >= letters.len(),
        "{citing} of {} letters cite the player",
        letters.len()
    );

    // Letters are worded in turn: no two open and close alike, no two
    // running open, lead or close the same way, and no opening comes
    // round more than twice.
    let shapes = letters
        .iter()
        .map(|letter| {
            let note = letter.note.as_str();
            let opening = first_sentence(note);
            let body = note[opening.len()..].trim();
            let lead = body
                .split_whitespace()
                .take(3)
                .collect::<Vec<_>>()
                .join(" ");
            (opening, lead, last_sentence(note))
        })
        .collect::<Vec<_>>();
    let pairs = shapes
        .iter()
        .map(|(opening, _, closing)| (*opening, *closing))
        .collect::<BTreeSet<_>>();
    assert_eq!(pairs.len(), shapes.len(), "{shapes:#?}");
    for two in shapes.windows(2) {
        assert_ne!(two[0].0, two[1].0, "the same opening twice running");
        assert_ne!(two[0].1, two[1].1, "the same lead twice running");
        assert_ne!(two[0].2, two[1].2, "the same closing twice running");
    }
    let mut openings = BTreeMap::<&str, usize>::new();
    for (opening, _, _) in &shapes {
        *openings.entry(opening).or_default() += 1;
    }
    assert!(openings.values().all(|n| *n <= 2), "{openings:#?}");
    assert!(openings.len() >= 15, "{openings:#?}");
    let closings = shapes
        .iter()
        .map(|(_, _, closing)| *closing)
        .collect::<BTreeSet<_>>();
    assert!(closings.len() >= 12, "{closings:#?}");
    println!(
        "{} mornings: {citing} of {} letters cite the player; {} things looked after, \
         {theirs} mornings the player's; {} distinct lines",
        mornings.len(),
        letters.len(),
        times.len(),
        seen.len()
    );
}
