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
    /// Questions answered, and how many of those answers changed what is
    /// on the scene.
    answered: usize,
    answers_seen: usize,
    universe: PocketUniverse<crate::PocketMind>,
}

/// What the scene shows: everything on it, by name and where it stands.
fn scene(world: &world_core::World) -> std::collections::BTreeSet<String> {
    projection::snapshot(world)
        .canvas
        .items
        .iter()
        .map(|item| format!("{} @ {:?}", item.label, item.at))
        .collect()
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
        answered: 0,
        answers_seen: 0,
        universe: PocketUniverse::new().unwrap(),
    };
    for day in 0..days {
        let snapshot = projection::snapshot(universe.world());
        let choices = snapshot
            .commands
            .iter()
            .filter(|command| {
                command.id != NUDGE_COMMAND
                    && command.unavailable.is_none()
                    && command.hand.is_none()
            })
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
            let before = scene(universe.world());
            if let Err(error) = universe.invoke_projection_command(command) {
                panic!("{policy:?} period {day}: {command} was offered but failed: {error}");
            }
            if snapshot
                .command(command)
                .is_some_and(|command| command.question.is_some())
            {
                played.answered += 1;
                if scene(universe.world()) != before {
                    played.answers_seen += 1;
                }
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

    // The chapters that ended are in the book, each in the World's words,
    // and the goals stand on the horizon.
    let snapshot = projection::snapshot(world);
    assert!(!snapshot.chapters.is_empty());
    assert!(snapshot
        .chapters
        .iter()
        .all(|chapter| !chapter.title.is_empty() && !chapter.summary.is_empty()));
    assert!(!snapshot.goals.is_empty());
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

/// The v0.11 bar: what you choose changes the place, and questions follow
/// from earlier answers.
#[test]
fn what_you_choose_changes_the_place_and_comes_back() {
    let generous = play(MARS, Policy::Generous, 30);
    let contrary = play(MARS, Policy::Contrary, 30);
    // Across both ways of playing, at least half of all answers change the
    // scene and a third of all questions besides the calendar's follow from
    // an earlier answer; neither way of playing falls far below that.
    let mut followed = (0, 0);
    for (policy, played) in [("yes", &generous), ("last answer", &contrary)] {
        let world = played.universe.world();
        eprintln!(
            "{policy}: {} of {} answers changed the scene",
            played.answers_seen, played.answered
        );
        assert!(
            played.answers_seen * 3 >= played.answered,
            "{policy}: only {} of {} answers changed the scene",
            played.answers_seen,
            played.answered
        );
        let mut asked = std::collections::BTreeMap::<String, usize>::new();
        for event in world.events() {
            if event.kind == "situation_arose" {
                if let Some(world_core::Value::Text(id)) = event.payload.get("storylet") {
                    *asked.entry(id.clone()).or_default() += 1;
                }
            }
        }
        // The calendar's days (market day, birthdays) come round by design,
        // so they are left out of the count.
        let all = asked
            .iter()
            .filter(|(id, _)| !story::from_the_calendar(id))
            .map(|(_, count)| count)
            .sum::<usize>();
        let following = asked
            .iter()
            .filter(|(id, _)| story::follows_from_an_answer(id))
            .map(|(_, count)| count)
            .sum::<usize>();
        eprintln!("{policy}: {following} of {all} questions followed from an answer; {asked:?}");
        assert!(
            following * 4 >= all,
            "{policy}: {following} of {all} questions followed from an earlier answer"
        );
        followed.0 += following;
        followed.1 += all;
        for (id, count) in &asked {
            if !story::from_the_calendar(id) {
                assert!(
                    *count <= 2,
                    "{policy}: {id} asked {count} times in 30 periods"
                );
            }
        }
    }
    assert!(
        (generous.answers_seen + contrary.answers_seen) * 2
            >= generous.answered + contrary.answered,
        "fewer than half of all answers changed the scene"
    );
    assert!(
        followed.0 * 3 >= followed.1,
        "{} of {} questions followed from an earlier answer",
        followed.0,
        followed.1
    );
    let names = |played: &Played| {
        let world = played.universe.world();
        projection::snapshot(world)
            .canvas
            .items
            .iter()
            .map(|item| item.label.clone())
            .collect::<std::collections::BTreeSet<_>>()
    };
    let (a, b) = (names(&generous), names(&contrary));
    let differences = a.symmetric_difference(&b).collect::<Vec<_>>();
    eprintln!("differences {differences:?}");
    assert!(
        differences.len() >= 3,
        "yes and last answer end too alike: {differences:?}"
    );
}

#[test]
fn a_new_world_opens_on_a_question() {
    let mut registry = world_host::WorldRegistry::new();
    registry
        .register(crate::pocket_universe_registration())
        .unwrap();
    let mut session = registry.create(crate::POCKET_UNIVERSE_PACK_ID).unwrap();
    let snapshot = session
        .handle(world_projection::ProjectionIntent::InvokeCommand(
            MARS.into(),
        ))
        .unwrap();
    assert!(
        snapshot
            .commands
            .iter()
            .any(|command| command.question.is_some()),
        "{:?}",
        snapshot
            .commands
            .iter()
            .map(|command| &command.title)
            .collect::<Vec<_>>()
    );
}

#[test]
fn a_week_away_lapses_at_most_three_questions() {
    let mut universe = PocketUniverse::new().unwrap();
    universe.invoke_projection_command(MARS).unwrap();
    for _ in 0..3 {
        universe.invoke_projection_command(NUDGE_COMMAND).unwrap();
    }
    let before = universe.world().events().len();
    universe.advance_periods(7).unwrap();
    let world = universe.world();
    let lapsed = world.events()[before..]
        .iter()
        .filter(|event| event.payload.contains_key("lapsed"))
        .count();
    assert!(lapsed <= 3, "{lapsed} questions lapsed in a week away");
}

/// The v0.12 bar: a year in each place never runs out. Played 365 periods,
/// new situations keep coming every month, nobody repeats themselves,
/// people's standing with each other keeps changing, every chapter has a
/// title of its own, and everyone lives every period without being asked.
fn a_year(seed: &str, policy: Policy) {
    let played = play(seed, policy, 365);
    let world = played.universe.world();
    let period = |event: &world_core::Event| event.world_time / crate::BACKGROUND_PERIOD;
    let first = world.events().first().map(period).unwrap_or(0);

    let mut seen = std::collections::BTreeSet::new();
    let mut fresh = vec![0; 13];
    for event in world.events() {
        let key = match event.kind.as_str() {
            "situation_arose" => event.payload.get("storylet"),
            "situation_came_up" => event.payload.get("situation"),
            _ => None,
        };
        if let Some(world_core::Value::Text(key)) = key {
            let month = ((period(event) - first) / 30).min(12) as usize;
            if seen.insert(key.clone()) {
                fresh[month] += 1;
            }
        }
    }
    eprintln!(
        "{seed} {policy:?}: never-seen situations by month {fresh:?}; living here {}",
        crate::life::people(world).len()
    );
    assert!(
        fresh[3..12].iter().all(|count| *count >= 8),
        "{seed} {policy:?}: never-seen situations by month {fresh:?}"
    );

    for window in played.lines.windows(30) {
        let mut counts = std::collections::BTreeMap::<&str, usize>::new();
        for line in window.iter().flatten() {
            *counts.entry(line).or_default() += 1;
        }
        if let Some((line, count)) = counts.into_iter().max_by_key(|(_, count)| *count) {
            assert!(
                count <= 3,
                "{seed} {policy:?}: {line:?} said {count} times in 30 periods"
            );
        }
    }

    let changes = world
        .events()
        .iter()
        .filter(|event| event.kind == "bond_changed")
        .map(|event| period(event) - first)
        .collect::<Vec<_>>();
    // Strangers take a while to arrive; from the fourth month on, how
    // people stand with each other keeps changing.
    for start in 90..335 {
        let count = changes
            .iter()
            .filter(|at| (start..start + 30).contains(*at))
            .count();
        assert!(
            count >= 3,
            "{seed} {policy:?}: only {count} changes between people in periods {start}-{}",
            start + 30
        );
    }

    let titles = projection::snapshot(world)
        .chapters
        .into_iter()
        .map(|chapter| chapter.title)
        .collect::<Vec<_>>();
    let unique = titles.iter().collect::<std::collections::BTreeSet<_>>();
    assert_eq!(unique.len(), titles.len(), "{seed} {policy:?}: {titles:?}");

    let people = crate::life::people(world);
    let now = world.world_time();
    let lived_now = world
        .events()
        .iter()
        .filter(|event| event.kind == "lived" && event.world_time == now)
        .filter_map(|event| event.actor)
        .collect::<std::collections::BTreeSet<_>>();
    assert!(
        people.iter().all(|who| lived_now.contains(who)),
        "{seed} {policy:?}: {people:?} vs {lived_now:?}"
    );
    assert!(people.len() >= 3, "{seed} {policy:?}: nobody came to stay");

    let replayed = world.replay().unwrap();
    assert_eq!(replayed.state(), world.state());
}

#[test]
fn a_year_on_mars_saying_yes_never_runs_out() {
    a_year(MARS, Policy::Generous);
}

#[test]
fn a_year_on_maple_street_saying_no_never_runs_out() {
    a_year(crate::SEED_1980S_TOWN_COMMAND, Policy::Contrary);
}

#[test]
fn a_year_on_icebridge_left_alone_never_runs_out() {
    a_year(crate::SEED_PENGUIN_CIVILIZATION_COMMAND, Policy::Absent);
}

/// The v0.12 bar for the player's own hands, in each place: at least five
/// things to do besides answering, and after a month of building,
/// planting and decorating, at least a third of what stands was put there
/// by the player.
#[test]
fn your_hands_shape_each_place() {
    for seed in [
        MARS,
        crate::SEED_1980S_TOWN_COMMAND,
        crate::SEED_PENGUIN_CIVILIZATION_COMMAND,
    ] {
        let mut universe = PocketUniverse::new().unwrap();
        universe.invoke_projection_command(seed).unwrap();
        universe.invoke_projection_command(NUDGE_COMMAND).unwrap();
        let verbs = projection::snapshot(universe.world())
            .deeds()
            .map(|(_, _, hand)| hand.verb.clone())
            .collect::<std::collections::BTreeSet<_>>();
        assert!(verbs.len() >= 5, "{seed}: {verbs:?}");
        for period in 0..30 {
            let snapshot = projection::snapshot(universe.world());
            let deeds = snapshot
                .deeds()
                .filter(|(_, command, hand)| {
                    command.unavailable.is_none()
                        && ["Build", "Plant", "Decorate"].contains(&hand.verb.as_str())
                })
                .map(|(_, command, _)| command.id.clone())
                .collect::<Vec<_>>();
            if let Some(deed) = deeds.get(period * 7 % deeds.len().max(1)) {
                universe.invoke_projection_command(deed).unwrap();
            }
            if let Some(answer) = snapshot
                .choices()
                .find(|command| command.question.is_some() && command.unavailable.is_none())
            {
                let _ = universe.invoke_projection_command(&answer.id.clone());
            }
            universe.invoke_projection_command(NUDGE_COMMAND).unwrap();
        }
        let state = universe.world().state();
        let standing = storylets::fixtures(state).len();
        let made = hands::made(state).len();
        assert!(
            made * 3 >= standing && made > 0,
            "{seed}: {made} of {standing} things standing were the player's"
        );
    }
}

/// The v0.12 bar for branches, on Mars: split at period 10 by one different
/// answer and played 90 periods the same way, the two colonies meet
/// situations at least a third different, and look different.
#[test]
fn branches_become_different_colonies() {
    let mut universe = PocketUniverse::new().unwrap();
    universe.invoke_projection_command(MARS).unwrap();
    let step = |universe: &mut PocketUniverse<crate::PocketMind>, last: bool| {
        let snapshot = projection::snapshot(universe.world());
        let answers = snapshot
            .choices()
            .filter(|command| command.question.is_some() && command.unavailable.is_none())
            .map(|command| command.id.clone())
            .collect::<Vec<_>>();
        let pick = if last {
            answers.last()
        } else {
            answers.first()
        };
        if let Some(answer) = pick {
            universe.invoke_projection_command(answer).unwrap();
        }
        universe.invoke_projection_command(NUDGE_COMMAND).unwrap();
    };
    for _ in 0..10 {
        step(&mut universe, false);
    }
    let split = universe.world().events().len();
    let archive = universe.archive().unwrap();
    let mut left = universe;
    let mut right = PocketUniverse::resume_archive(&archive).unwrap();
    step(&mut left, false);
    step(&mut right, true);
    for _ in 0..90 {
        step(&mut left, false);
        step(&mut right, false);
    }
    let met = |universe: &PocketUniverse<crate::PocketMind>| {
        universe.world().events()[split..]
            .iter()
            .filter_map(|event| match event.kind.as_str() {
                "situation_arose" => event.payload.get("storylet"),
                "situation_came_up" => event.payload.get("situation"),
                _ => None,
            })
            .filter_map(|key| match key {
                world_core::Value::Text(key) => Some(key.clone()),
                _ => None,
            })
            .collect::<std::collections::BTreeSet<_>>()
    };
    let (a, b) = (met(&left), met(&right));
    let differ = a.symmetric_difference(&b).count();
    let all = a.union(&b).count();
    assert!(
        differ * 3 >= all,
        "only {differ} of {all} situations differ between the branches"
    );
    assert_ne!(scene(left.world()), scene(right.world()));
}
