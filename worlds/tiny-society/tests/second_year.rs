//! A warm player's first sixteen months in the harbour: they answer the
//! first question each day, make something new every third day, and let
//! the day pass. The second year must not be the first again.

use std::collections::{BTreeMap, BTreeSet};
use world_projection::ProjectionIntent::InvokeCommand;

const DAYS: usize = 480;
const PASS: &str = "tiny-society.let-day-pass";

#[test]
fn a_warm_players_second_year() {
    let mut registry = world_host::WorldRegistry::new();
    registry
        .register(tiny_society::tiny_society_registration())
        .unwrap();
    let mut session = registry.create(tiny_society::TINY_SOCIETY_PACK_ID).unwrap();
    let mut asked: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    let mut open_before = BTreeSet::new();
    let mut heard = BTreeSet::new();
    let (mut year_two_lines, mut year_two_new) = (0, 0);
    let mut kept = Vec::new();
    let mut idle = Vec::new();
    let mut book_at_a_year = (0, 0);
    for day in 1..=DAYS {
        let snapshot = session.snapshot();
        // What was said today, and whether it was heard before.
        for voice in snapshot.voices.iter().filter(|voice| {
            snapshot
                .timeline
                .items
                .iter()
                .any(|item| item.id == voice.moment && item.world_time == snapshot.world_time)
        }) {
            let new = heard.insert(voice.line.clone());
            if day > 365 {
                year_two_lines += 1;
                year_two_new += usize::from(new);
            }
        }
        // A question is asked when it comes on offer.
        let open = snapshot
            .commands
            .iter()
            .filter_map(|command| command.question.as_ref())
            .map(|question| question.prompt.clone())
            .collect::<BTreeSet<_>>();
        for prompt in open.difference(&open_before) {
            asked.entry(prompt.clone()).or_default().push(day);
        }
        open_before = open;
        kept.push(snapshot.keepsakes.len());
        if !snapshot.goals.iter().any(|goal| goal.done < goal.parts) {
            idle.push(day);
        }
        if day == 365 {
            book_at_a_year = (
                snapshot.book.iter().filter(|entry| entry.found).count(),
                snapshot.book.len(),
            );
            let silent = snapshot
                .book
                .iter()
                .filter(|entry| !entry.found && entry.hint.trim().is_empty())
                .count();
            assert_eq!(silent, 0, "every silhouette says how to find it");
        }
        // The warm player answers, makes something new, and waits a day.
        if let Some(answer) = snapshot
            .commands
            .iter()
            .find(|c| c.question.is_some() && c.unavailable.is_none() && c.id != PASS)
        {
            let _ = session.handle(InvokeCommand(answer.id.clone()));
        }
        if day % 3 == 0 {
            let unmade = snapshot
                .book
                .iter()
                .filter(|entry| entry.shelf == "Made" && !entry.found)
                .map(|entry| entry.name.clone())
                .collect::<BTreeSet<_>>();
            let hands = snapshot
                .commands
                .iter()
                .filter(|c| c.unavailable.is_none())
                .filter(|c| c.hand.as_ref().is_some_and(|hand| hand.verb != "Undo"))
                .collect::<Vec<_>>();
            let deed = hands
                .iter()
                .find(|c| unmade.contains(&c.hand.as_ref().unwrap().thing))
                .or(hands.first());
            if let Some(deed) = deed {
                let _ = session.handle(InvokeCommand(deed.id.clone()));
            }
        }
        session.handle(InvokeCommand(PASS.into())).unwrap();
    }

    assert!(idle.is_empty(), "nothing to work towards on days {idle:?}");
    let (found, all) = book_at_a_year;
    assert!(
        found * 10 >= all * 9,
        "only {found} of {all} of the book found in a year"
    );
    let most_in_a_week = kept.windows(8).map(|week| week[7] - week[0]).max().unwrap();
    assert!(most_in_a_week <= 3, "{most_in_a_week} keepsakes in a week");
    let share = year_two_new as f64 / year_two_lines.max(1) as f64;
    assert!(
        share >= 0.6,
        "only {:.0}% of the second year's lines are new",
        share * 100.0
    );
    for (prompt, days) in &asked {
        let most = days
            .iter()
            .map(|first| {
                days.iter()
                    .filter(|day| (*first..*first + 365).contains(*day))
                    .count()
            })
            .max()
            .unwrap_or(0);
        assert!(most <= 6, "asked {most} times in a year: {prompt}");
    }
}
