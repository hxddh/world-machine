//! Does each place keep having a story? Sixty periods, played three ways,
//! held to the bar the v0.10 plan set.

use crate::{projection, story, talk, PocketUniverse, NUDGE_COMMAND};
use world_projection::SelectionId;

#[derive(Clone, Copy, Debug)]
enum Policy {
    /// Always says yes, to the first thing on offer.
    Generous,
    /// Always takes the last answer on offer, which is often a no.
    Contrary,
    /// Never answers anything; lets every day pass.
    Absent,
}

struct Played {
    /// Days that offered something more than letting the day pass.
    days_with_a_choice: Vec<bool>,
    /// What was said on each day.
    lines: Vec<Vec<String>>,
    /// Each day's money and spirits gauges.
    gauges: Vec<Vec<(String, f32)>>,
    /// The day the first chapter closed.
    first_chapter: Option<usize>,
    universe: PocketUniverse<crate::PocketMind>,
}

fn said_today(universe: &PocketUniverse<crate::PocketMind>) -> Vec<String> {
    let world = universe.world();
    let now = world.world_time();
    talk::voices(world)
        .into_iter()
        .filter(|voice| {
            let SelectionId::Event(id) = voice.moment else {
                return false;
            };
            world
                .events()
                .iter()
                .any(|event| event.id == id && event.world_time == now)
        })
        .map(|voice| voice.line)
        .collect()
}

fn play(seed: &str, policy: Policy, days: usize) -> Played {
    let mut universe = PocketUniverse::new().unwrap();
    universe.invoke_projection_command(seed).unwrap();
    // The first period passes before anything is counted.
    universe.invoke_projection_command(NUDGE_COMMAND).unwrap();
    let mut played = Played {
        days_with_a_choice: Vec::new(),
        lines: Vec::new(),
        gauges: Vec::new(),
        first_chapter: None,
        universe: PocketUniverse::new().unwrap(),
    };
    for day in 0..days {
        let snapshot = projection::snapshot(universe.world());
        let choices = snapshot
            .commands
            .iter()
            .filter(|command| command.id != NUDGE_COMMAND)
            .map(|command| command.id.clone())
            .collect::<Vec<_>>();
        played.days_with_a_choice.push(!choices.is_empty());
        played.lines.push(said_today(&universe));
        played.gauges.push(
            snapshot
                .gauges
                .iter()
                .filter(|gauge| gauge.id == "trust" || gauge.id == "tension")
                .map(|gauge| (gauge.id.clone(), gauge.value))
                .collect(),
        );
        if played.first_chapter.is_none()
            && universe
                .world()
                .events()
                .iter()
                .any(|event| event.kind == "chapter_ended")
        {
            played.first_chapter = Some(day);
        }
        let pick = match policy {
            Policy::Generous => choices.first(),
            Policy::Contrary => choices.last(),
            Policy::Absent => None,
        };
        if let Some(command) = pick {
            if let Err(error) = universe.invoke_projection_command(command) {
                panic!("{policy:?} period {day}: {command} was offered but failed: {error}");
            }
        }
        universe.invoke_projection_command(NUDGE_COMMAND).unwrap();
    }
    played.universe = universe;
    played
}

fn check(seed: &str, policy: Policy) {
    let played = play(seed, policy, 60);
    let empty = played
        .days_with_a_choice
        .iter()
        .enumerate()
        .filter(|(_, choice)| !**choice)
        .map(|(day, _)| day)
        .collect::<Vec<_>>();
    assert!(
        empty.is_empty(),
        "{policy:?}: days with nothing to decide: {empty:?}"
    );

    for window in played.lines.windows(10) {
        let mut counts = std::collections::BTreeMap::<&str, usize>::new();
        for line in window.iter().flatten() {
            *counts.entry(line).or_default() += 1;
        }
        let worst = counts.into_iter().max_by_key(|(_, count)| *count);
        if let Some((line, count)) = worst {
            assert!(
                count <= 3,
                "{policy:?}: {line:?} said {count} times in ten days"
            );
        }
    }

    for gauge in ["trust", "tension"] {
        let mut run = 0;
        let mut worst = 0;
        for day in &played.gauges {
            let value = day
                .iter()
                .find(|(id, _)| id == gauge)
                .map(|(_, value)| *value)
                .unwrap();
            if !(0.05..=0.95).contains(&value) {
                run += 1;
            } else {
                run = 0;
            }
            worst = worst.max(run);
        }
        assert!(
            worst <= 5,
            "{policy:?}: {gauge} pinned at an end for {worst} days"
        );
    }

    assert!(
        played.first_chapter.is_some_and(|day| day <= 30),
        "{policy:?}: first chapter closed on {:?}",
        played.first_chapter
    );

    // Everything the storyteller did is history: the World replays to the
    // same place without it.
    let world = played.universe.world();
    let replayed = world.replay().unwrap();
    assert_eq!(replayed.state(), world.state());
}

const MARS: &str = crate::SEED_MARS_COLONY_COMMAND;

#[test]
fn a_generous_player_always_has_a_story() {
    check(MARS, Policy::Generous);
}

#[test]
fn a_contrary_player_always_has_a_story() {
    check(MARS, Policy::Contrary);
}

#[test]
fn an_absent_player_always_has_a_story() {
    check(MARS, Policy::Absent);
}

#[test]
fn every_place_has_a_story() {
    check(crate::SEED_1980S_TOWN_COMMAND, Policy::Contrary);
    check(crate::SEED_PENGUIN_CIVILIZATION_COMMAND, Policy::Generous);
}

/// Prints how sixty periods went, for tuning the deck by eye.
#[test]
#[ignore]
fn show_sixty_periods() {
    for policy in [Policy::Generous, Policy::Contrary, Policy::Absent] {
        let played = play(MARS, policy, 60);
        println!("== {policy:?}");
        for (day, gauges) in played.gauges.iter().enumerate() {
            println!(
                "day {day:2} choice={} {:?} {:?}",
                played.days_with_a_choice[day], gauges, played.lines[day]
            );
        }
        for event in played.universe.world().events() {
            if let Some(title) = story::told(played.universe.world(), event) {
                println!("  t={} {title}", event.world_time);
            }
        }
    }
}
