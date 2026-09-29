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
    universe: PocketUniverse,
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

fn said_today(universe: &PocketUniverse) -> Vec<String> {
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
    // A third of all questions besides the calendar's follow from an
    // earlier answer, across every place and both ways of playing.
    let mut followed = (0, 0);
    for seed in [
        MARS,
        crate::SEED_1980S_TOWN_COMMAND,
        crate::SEED_PENGUIN_CIVILIZATION_COMMAND,
    ] {
        let (following, all) = choices_change_and_come_back(seed);
        followed.0 += following;
        followed.1 += all;
    }
    assert!(
        followed.0 * 3 >= followed.1,
        "{} of {} questions followed from an earlier answer",
        followed.0,
        followed.1
    );
}

/// In one place: how many questions followed from an earlier answer, of
/// how many, checking the rest of the bar as it goes.
fn choices_change_and_come_back(seed: &str) -> (usize, usize) {
    let generous = play(seed, Policy::Generous, 30);
    let contrary = play(seed, Policy::Contrary, 30);
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
        "{seed}: yes and last answer end too alike: {differences:?}"
    );
    followed
}

/// The v0.15 bar, as v0.19.1 keeps it: a new World opens on the place and
/// its first question, asked just after the hello. The first deed can be
/// done at once, someone nearby says what they make of it, and the
/// question still waits.
#[test]
fn a_new_world_opens_on_the_place() {
    let mut registry = world_host::WorldRegistry::new();
    registry
        .register(crate::pocket_universe_registration())
        .unwrap();
    let questions = |snapshot: &world_projection::ProjectionSnapshot| {
        snapshot
            .commands
            .iter()
            .filter(|command| command.question.is_some())
            .map(|command| command.title.clone())
            .collect::<Vec<_>>()
    };
    for seed in [
        MARS,
        crate::SEED_1980S_TOWN_COMMAND,
        crate::SEED_PENGUIN_CIVILIZATION_COMMAND,
    ] {
        let mut session = registry.create(crate::POCKET_UNIVERSE_PACK_ID).unwrap();
        let opened = session
            .handle(world_projection::ProjectionIntent::InvokeCommand(
                seed.into(),
            ))
            .unwrap();
        assert!(
            !questions(&opened).is_empty(),
            "{seed}: no question as it opens"
        );
        let deed = opened
            .commands
            .iter()
            .find(|command| {
                command.unavailable.is_none()
                    && command
                        .hand
                        .as_ref()
                        .is_some_and(|hand| hand.verb == "Build")
            })
            .unwrap_or_else(|| panic!("{seed}: something to build at once"))
            .id
            .clone();
        let after = session
            .handle(world_projection::ProjectionIntent::InvokeCommand(deed))
            .unwrap_or_else(|error| panic!("{seed}: the first deed failed: {error}"));
        assert!(
            !questions(&after).is_empty(),
            "{seed}: no question after the first deed"
        );
        let heard = after
            .voices
            .iter()
            .filter(|voice| !opened.voices.contains(voice))
            .count();
        assert!(heard >= 1, "{seed}: nobody said anything about it");
    }
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
    // A player who says yes finishes the place's goals and climbs its
    // ladder of works, with only the latest still under way.
    if matches!(policy, Policy::Generous) {
        let goals = crate::story::goals(world);
        let unfinished = goals
            .iter()
            .filter(|goal| !goal.finished())
            .map(|goal| format!("{} {} of {}", goal.label, goal.done, goal.parts))
            .collect::<Vec<_>>();
        assert!(unfinished.len() <= 1, "{seed}: {unfinished:?}");
        // The place's three goals and at least eight works.
        assert!(
            goals.len() >= 11,
            "{seed}: only {} goals in a year",
            goals.len()
        );
    }

    // A festival told in its second year is told against its first.
    let mut told = std::collections::BTreeMap::<String, Vec<String>>::new();
    for event in world
        .events()
        .iter()
        .filter(|event| event.kind == "festival_held")
    {
        if let (Some(world_core::Value::Text(id)), Some(world_core::Value::Text(line))) =
            (event.payload.get("festival"), event.payload.get("told"))
        {
            told.entry(id.clone()).or_default().push(line.clone());
        }
    }
    let twice = told.values().filter(|lines| lines.len() >= 2).count();
    assert!(twice >= 10, "only {twice} festivals came round twice");
    for (id, lines) in &told {
        if lines.len() >= 2 {
            assert_ne!(
                lines[0], lines[1],
                "{id} told the same way two years running"
            );
        }
    }
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
    // people stand with each other keeps changing. Left alone, with nobody
    // asking or answering, it changes more slowly (a month can be quiet),
    // but it still changes.
    // Fewer, weightier changes (v0.22): at least one a season, at most
    // one in six periods.
    let (fewest, span) = (1, 60);
    assert!(
        changes.len() <= 61,
        "{seed} {policy:?}: {} changes between people in a year",
        changes.len()
    );
    for start in 90..365 - span {
        let count = changes
            .iter()
            .filter(|at| (start..start + span).contains(*at))
            .count();
        assert!(
            count >= fewest,
            "{seed} {policy:?}: only {count} changes between people in periods {start}-{}",
            start + span
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

    // The v0.15 bar: a question that comes round again never uses the
    // words it was asked in last time.
    let mut last_words = std::collections::BTreeMap::<String, String>::new();
    for event in world
        .events()
        .iter()
        .filter(|event| event.kind == "situation_arose")
    {
        let (Some(world_core::Value::Text(id)), Some((_, words))) =
            (event.payload.get("storylet"), story::line(world, event))
        else {
            continue;
        };
        if let Some(before) = last_words.insert(id.clone(), words.clone()) {
            assert_ne!(
                before, words,
                "{seed} {policy:?}: {id} asked in the same words again"
            );
        }
    }

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
    let step = |universe: &mut PocketUniverse, last: bool| {
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
    let met = |universe: &PocketUniverse| {
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

/// Times a year-old World's snapshot and turn; run in a release build with
/// `--ignored --nocapture`.
#[test]
#[ignore]
fn time_a_year_old_world() {
    let played = play(MARS, Policy::Generous, 365);
    let world = played.universe.world();
    world.history_index();
    for _ in 0..3 {
        let started = std::time::Instant::now();
        let snapshot = projection::snapshot(world);
        eprintln!(
            "snapshot {:?} ({} events, {} history items)",
            started.elapsed(),
            world.events().len(),
            snapshot.timeline.items.len()
        );
    }
    let started = std::time::Instant::now();
    played.universe.projection_snapshot();
    eprintln!("session snapshot {:?}", started.elapsed());
    let time = |label: &str, f: &dyn Fn()| {
        let started = std::time::Instant::now();
        f();
        eprintln!("  {label} {:?}", started.elapsed());
    };
    let kit = crate::speech::kit(world.state());
    time("recollections", &|| {
        for who in crate::life::people(world) {
            conversation::recollection(world, &kit, who);
        }
    });
    time("keepsakes", &|| {
        lives::keepsakes(world);
    });
    time("commands", &|| {
        story::commands(world);
    });
    time("talks", &|| {
        crate::talk::talks(world, &story::commands(world));
    });
    time("voices", &|| {
        talk::voices(world);
    });
}

/// What the book, the letter box and the daily round find through the
/// World's index of its history is what reading every event finds, all
/// through a year in which the player answers, plants and makes things:
/// on the World the year left, on that World replayed, and on the World
/// as it stood at fifty points along the way.
#[test]
fn the_index_finds_what_reading_every_event_finds_for_a_year() {
    let mut universe = a_year_of(crate::SEED_1980S_TOWN_COMMAND, 0, true);
    for day in 0..365_usize {
        let snapshot = projection::snapshot(universe.world());
        let answer = snapshot
            .commands
            .iter()
            .find(|command| {
                command.question.is_some()
                    && command.unavailable.is_none()
                    && command.id != NUDGE_COMMAND
            })
            .map(|command| command.id.clone());
        let deeds = snapshot
            .deeds()
            .filter(|(_, command, hand)| command.unavailable.is_none() && hand.verb != "Undo")
            .map(|(_, command, _)| command.id.clone())
            .collect::<Vec<_>>();
        if let Some(answer) = answer {
            let _ = universe.invoke_projection_command(&answer);
        }
        if day % 3 == 0 && !deeds.is_empty() {
            let _ = universe.invoke_projection_command(&deeds[day / 3 % deeds.len()]);
        }
        universe.invoke_projection_command(NUDGE_COMMAND).unwrap();
    }
    let world = universe.world();
    let check = |world: &world_core::World| {
        lives::scanned::compare(world, &crate::life::cast(world.state())).unwrap();
    };
    check(world);
    let replayed = world.replay().unwrap();
    assert_eq!(replayed.state(), world.state());
    check(&replayed);
    let events = world.events().len();
    for at in (1..=50).map(|at| events * at / 50) {
        check(&world.fork_after(at).unwrap());
    }
    let kept = lives::keepsakes(world);
    assert!(kept.iter().any(|kept| world
        .event(kept.event)
        .is_some_and(|event| event.kind == "enjoyed")));
    assert!(!lives::letters(world).is_empty());
}

/// Plays `periods` periods of a seed, planting on the first if `plant`,
/// answering nothing.
fn a_year_of(seed: &str, periods: usize, plant: bool) -> PocketUniverse {
    let mut universe = PocketUniverse::new().unwrap();
    universe.invoke_projection_command(seed).unwrap();
    if plant {
        let gardens = projection::snapshot(universe.world())
            .deeds()
            .filter(|(_, command, hand)| hand.verb == "Plant" && command.unavailable.is_none())
            .map(|(_, command, _)| command.id.clone())
            .take(2)
            .collect::<Vec<_>>();
        assert!(!gardens.is_empty(), "{seed}");
        for garden in gardens {
            universe.invoke_projection_command(&garden).unwrap();
        }
    }
    for _ in 0..periods {
        universe.invoke_projection_command(NUDGE_COMMAND).unwrap();
    }
    universe
}

/// Every place's year has a shape: something on its calendar in every
/// fortnight, festivals in every season, and a harvest that is as good as
/// what was planted.
#[test]
fn every_place_has_a_year_with_a_shape() {
    for seed in [
        MARS,
        crate::SEED_1980S_TOWN_COMMAND,
        crate::SEED_PENGUIN_CIVILIZATION_COMMAND,
    ] {
        let universe = a_year_of(seed, 125, false);
        let world = universe.world();
        let period = |event: &world_core::Event| event.world_time / crate::BACKGROUND_PERIOD;
        let first = world.events().first().map(period).unwrap_or(0);
        let days = world
            .events()
            .iter()
            .filter(|event| calendar::is_calendar(event))
            .map(period)
            .collect::<Vec<_>>();
        for start in first..first + 110 {
            assert!(
                days.iter().any(|day| (start..start + 14).contains(day)),
                "{seed}: nothing on the calendar in periods {start}-{}",
                start + 14
            );
        }
        let held = world
            .events()
            .iter()
            .filter(|event| event.kind == "festival_held")
            .map(|event| period(event) % crate::almanac::YEAR / (crate::almanac::YEAR / 4))
            .collect::<Vec<_>>();
        for season in 0..4 {
            let count = held.iter().filter(|held| **held == season).count();
            assert!(count >= 3, "{seed}: season {season} has {count} festivals");
        }
        let calendar = projection::snapshot(world).calendar.unwrap();
        assert!(calendar.season.is_some(), "{seed}");

        let harvest = |universe: &PocketUniverse| {
            universe
                .world()
                .events()
                .iter()
                .find(|event| {
                    event.kind == "festival_held"
                        && matches!(
                            event.payload.get("festival"),
                            Some(world_core::Value::Text(festival))
                                if festival.ends_with("harvest") || festival == "pie_contest"
                        )
                })
                .and_then(calendar::told)
                .unwrap()
        };
        let planted = a_year_of(seed, 75, true);
        let unplanted = a_year_of(seed, 75, false);
        assert_ne!(harvest(&planted), harvest(&unplanted), "{seed}");
    }
}

const SEEDS: [&str; 3] = [
    MARS,
    crate::SEED_1980S_TOWN_COMMAND,
    crate::SEED_PENGUIN_CIVILIZATION_COMMAND,
];

fn begun(seed: &str) -> PocketUniverse {
    let mut universe = PocketUniverse::new().unwrap();
    universe.invoke_projection_command(seed).unwrap();
    universe.invoke_projection_command(NUDGE_COMMAND).unwrap();
    universe
}

/// The situations people put to the player, by kind and asker.
fn came_up(events: &[world_core::Event]) -> Vec<(String, world_core::EntityId)> {
    events
        .iter()
        .filter(|event| event.kind == "situation_came_up")
        .filter_map(|event| match (event.payload.get("kind"), event.actor) {
            (Some(world_core::Value::Text(kind)), Some(who)) => Some((kind.clone(), who)),
            _ => None,
        })
        .collect()
}

/// The v0.15 bar: befriending someone opens their doors, each once, in
/// every place.
#[test]
fn a_warm_friendship_opens_doors_in_every_place() {
    for seed in SEEDS {
        let mut universe = begun(seed);
        let friend = crate::life::people(universe.world())[0];
        for _ in 0..120 {
            universe.say(friend, "you're wonderful").unwrap();
            let commands = projection::snapshot(universe.world()).commands;
            if let Some(gift) = commands.iter().find(|command| {
                command.unavailable.is_none()
                    && command.hand.as_ref().is_some_and(|hand| {
                        hand.verb == "Give" && hand.at == Some(SelectionId::Entity(friend))
                    })
            }) {
                universe.invoke_projection_command(&gift.id).unwrap();
            }
            let mut asked = std::collections::BTreeSet::new();
            let answers = projection::snapshot(universe.world())
                .commands
                .into_iter()
                .filter(|command| {
                    command.asker == Some(SelectionId::Entity(friend))
                        && command.unavailable.is_none()
                })
                .filter_map(|command| {
                    asked
                        .insert(command.question.as_ref()?.id.clone())
                        .then_some(command.id)
                })
                .collect::<Vec<_>>();
            for answer in answers {
                universe.invoke_projection_command(&answer).unwrap();
            }
            universe.invoke_projection_command(NUDGE_COMMAND).unwrap();
        }
        let world = universe.world();
        let doors = came_up(world.events())
            .into_iter()
            .filter(|(kind, who)| {
                *who == friend && matches!(kind.as_str(), "confide" | "favour" | "keepsake")
            })
            .map(|(kind, _)| kind)
            .collect::<Vec<_>>();
        assert_eq!(doors, vec!["confide", "favour", "keepsake"], "{seed}");
        assert!(
            lives::keepsakes(world)
                .iter()
                .any(|kept| kept.from == friend),
            "{seed}"
        );
    }
}

/// The v0.15 bar: every return, of one period to seven, ends on something
/// someone left the player to keep, unless a week has already brought as
/// many as it may.
#[test]
fn every_return_brings_a_keepsake_in_every_place() {
    for seed in SEEDS {
        let mut universe = begun(seed);
        for periods in 1..=7 {
            let since = universe.world().events().len();
            universe.advance_periods(periods).unwrap();
            let snapshot = universe.projection_snapshot_since(Some(since));
            let last = snapshot
                .briefing
                .as_ref()
                .and_then(|briefing| {
                    briefing
                        .items
                        .iter()
                        .rev()
                        .find(|item| item.kind == world_projection::BriefingItemKind::Beat)
                })
                .map(|item| item.title.clone())
                .unwrap_or_default();
            // Unless the week has already brought as many things to keep
            // as it may, so that each still means something.
            let cast = crate::life::cast(universe.world().state());
            let room = lives::room_for_keepsake(universe.world(), &cast);
            assert!(
                last.contains(" left you ") || !room,
                "{seed}: a return of {periods} ended on {last:?}"
            );
        }
    }
}

/// The v0.15 bar: someone sees what the player made, says so, and brings
/// it up the next period.
#[test]
fn someone_sees_what_you_made_and_remembers_it_in_every_place() {
    for seed in SEEDS {
        let mut universe = begun(seed);
        let build = projection::snapshot(universe.world())
            .commands
            .into_iter()
            .find(|command| {
                command.unavailable.is_none()
                    && command
                        .hand
                        .as_ref()
                        .is_some_and(|hand| hand.verb == "Build")
            })
            .unwrap_or_else(|| panic!("{seed}: something to build"));
        let thing = build.hand.as_ref().unwrap().thing.to_lowercase();
        universe.invoke_projection_command(&build.id).unwrap();
        let who = universe
            .world()
            .events()
            .iter()
            .rev()
            .find(|event| event.kind == "reacted")
            .and_then(|event| event.actor)
            .unwrap_or_else(|| panic!("{seed}: nobody said anything about it"));
        universe.invoke_projection_command(NUDGE_COMMAND).unwrap();
        let world = universe.world();
        let recalled = conversation::recollection(world, &crate::speech::kit(world.state()), who)
            .unwrap_or_default();
        assert!(
            recalled.contains(&format!("{thing} you built")),
            "{seed}: {recalled:?}"
        );
    }
}

/// The v0.16 bar: everyone on the scene has an outline of their own, in
/// every place, and the drawings a snapshot carries stay within what the
/// app takes.
#[test]
fn everyone_on_the_scene_has_an_outline_of_their_own_in_every_place() {
    for seed in SEEDS {
        let played = play(seed, Policy::Generous, 120);
        let snapshot = projection::snapshot(played.universe.world());
        assert!(
            snapshot.drawings.len() <= 64,
            "{seed}: {}",
            snapshot.drawings.len()
        );
        let people = snapshot
            .canvas
            .items
            .iter()
            .filter(|item| item.kind == world_projection::CanvasItemKind::Actor)
            .collect::<Vec<_>>();
        assert!(people.len() >= 4, "{seed}: {} people", people.len());
        let mut outlines = std::collections::BTreeMap::new();
        for item in people {
            let drawing = snapshot
                .drawing_of(item)
                .unwrap_or_else(|| panic!("{seed}: {} has no drawing", item.label));
            assert!(drawing.is_drawable(), "{seed}: {}", drawing.id);
            if let Some(other) = outlines.insert(drawing.silhouette(), item.label.clone()) {
                panic!("{seed}: {} and {other} share an outline", item.label);
            }
        }
    }
}

/// A newcomer's first session in each place, on a clock: choosing a place
/// takes 8 s, someone takes 3 s to walk up, reading takes a second for
/// every 15 characters, and finding something to make takes 12 s. Someone
/// greets them within 20 s, the first choice is theirs within 60 s, and
/// they hold a keepsake within 5 minutes.
#[test]
fn a_first_session_greets_offers_and_gives_in_time_in_every_place() {
    let read = |text: &str| text.chars().count() as f32 / 15.0;
    let mut registry = world_host::WorldRegistry::new();
    registry
        .register(crate::pocket_universe_registration())
        .unwrap();
    for seed in [
        MARS,
        crate::SEED_1980S_TOWN_COMMAND,
        crate::SEED_PENGUIN_CIVILIZATION_COMMAND,
    ] {
        let mut session = registry.create(crate::POCKET_UNIVERSE_PACK_ID).unwrap();
        let mut clock = 2.0 + 8.0;
        let opened = session
            .handle(world_projection::ProjectionIntent::InvokeCommand(
                seed.into(),
            ))
            .unwrap();
        // The hello the World records, not any line with "I'm " in it,
        // among what people say as it opens (a new World is the same
        // World every time).
        let recorded = {
            let mut universe = PocketUniverse::new().unwrap();
            universe.invoke_projection_command(seed).unwrap();
            let greeted = universe
                .world()
                .events()
                .iter()
                .find(|event| event.kind == "greeted")
                .unwrap_or_else(|| panic!("{seed}: nobody greets the newcomer"))
                .clone();
            let (who, line) = lives::said(&greeted).expect("the greeting is said");
            world_projection::Voice {
                moment: world_projection::SelectionId::Event(greeted.id),
                speaker: world_projection::SelectionId::Entity(who),
                line,
            }
        };
        let greeting = opened
            .voices
            .iter()
            .find(|voice| **voice == recorded)
            .unwrap_or_else(|| panic!("{seed}: nobody says hello: {:?}", opened.voices));
        // Nobody has been away, and something is asked from the start.
        let briefing = opened.briefing.as_ref().expect("a briefing");
        assert!(!briefing.returned, "{seed}");
        assert!(
            !briefing.title.to_lowercase().contains("away"),
            "{seed}: {}",
            briefing.title
        );
        assert!(
            opened
                .commands
                .iter()
                .any(|command| command.question.is_some()),
            "{seed}: a new World opens on its first question"
        );
        // It is said now, and first: the window shows what is said now,
        // news before everyday talk, in order.
        let now = |voice: &world_projection::Voice| {
            opened
                .timeline
                .items
                .iter()
                .find(|item| item.id == voice.moment)
                .filter(|item| item.world_time == opened.world_time)
                .map(|item| item.routine)
        };
        assert_eq!(now(greeting), Some(false), "the greeting is news, said now");
        // The window says what is said now in turn, news first, each for
        // 4.6 s a page of two lines.
        let turn =
            |voice: &world_projection::Voice| 4.6 * voice.line.chars().count().div_ceil(72) as f32;
        let before: f32 = opened
            .voices
            .iter()
            .filter(|voice| now(voice) == Some(false))
            .take_while(|voice| *voice != greeting)
            .map(turn)
            .sum();
        assert_eq!(before, 0.0, "{seed}: the hello is the first thing said");
        clock += before;
        clock += 3.0;
        assert!(clock <= 20.0, "{seed}: greeted at {clock} s");
        println!("{seed}: greeted at {clock} s");
        clock += read(&greeting.line);
        let deed = opened
            .commands
            .iter()
            .find(|command| command.unavailable.is_none() && command.hand.is_some())
            .unwrap_or_else(|| panic!("{seed}: something to make at once"))
            .id
            .clone();
        clock += 12.0;
        assert!(clock <= 60.0, "{seed}: first choice at {clock} s");
        println!("{seed}: first choice at {clock} s");
        let after = session
            .handle(world_projection::ProjectionIntent::InvokeCommand(deed))
            .unwrap();
        let keepsake = after
            .keepsakes
            .first()
            .unwrap_or_else(|| panic!("{seed}: a keepsake for the first deed"));
        clock += 3.0 + read(&keepsake.what) + read(&keepsake.note);
        assert!(clock <= 300.0, "{seed}: first keepsake at {clock} s");
        println!("{seed}: first keepsake at {clock} s");
    }
}

/// Every card fits in two lines: a title in about 90 characters and its
/// detail in about 116, across 150 periods of answering in each place.
#[test]
fn every_card_fits_in_two_lines_in_every_place() {
    let mut long = std::collections::BTreeSet::new();
    for seed in [
        MARS,
        crate::SEED_1980S_TOWN_COMMAND,
        crate::SEED_PENGUIN_CIVILIZATION_COMMAND,
    ] {
        let mut universe = PocketUniverse::new().unwrap();
        universe.invoke_projection_command(seed).unwrap();
        for _ in 0..150 {
            let snapshot = projection::snapshot(universe.world());
            for command in snapshot.commands.iter().filter(|c| c.question.is_some()) {
                if command.title.chars().count() > 90 {
                    long.insert(command.title.clone());
                }
                if command.detail.chars().count() > 116 {
                    long.insert(command.detail.clone());
                }
            }
            let pick = snapshot
                .commands
                .iter()
                .find(|c| c.question.is_some() && c.unavailable.is_none())
                .map(|c| c.id.clone());
            if let Some(pick) = pick {
                universe.invoke_projection_command(&pick).unwrap();
            }
            universe.invoke_projection_command(NUDGE_COMMAND).unwrap();
        }
    }
    assert!(long.is_empty(), "longer than two lines: {long:#?}");
}

/// The goals on the horizon can be finished: a player who says yes builds
/// every one within a year, in every place. Three years of play, so run
/// by hand; Mars is checked on every run by its year-long test.
#[test]
#[ignore]
fn a_careful_player_finishes_every_goal_in_a_year() {
    for seed in [
        MARS,
        crate::SEED_1980S_TOWN_COMMAND,
        crate::SEED_PENGUIN_CIVILIZATION_COMMAND,
    ] {
        let played = play(seed, Policy::Generous, 365);
        let goals = crate::story::goals(played.universe.world());
        assert!(!goals.is_empty(), "{seed}");
        let unfinished = goals
            .iter()
            .filter(|goal| !goal.finished())
            .map(|goal| format!("{} {} of {}", goal.label, goal.done, goal.parts))
            .collect::<Vec<_>>();
        assert!(unfinished.is_empty(), "{seed}: {unfinished:?}");
    }
}

/// Something new every period for a month in every place, for a player
/// who only answers the first question on offer and makes something every
/// third period: something to find in the book, something to keep, a
/// letter, or a chapter's close, and at least four keepsakes or letters in
/// the month.
#[test]
fn something_new_every_period_for_a_month_in_every_place() {
    for seed in [
        MARS,
        crate::SEED_1980S_TOWN_COMMAND,
        crate::SEED_PENGUIN_CIVILIZATION_COMMAND,
    ] {
        let mut universe = PocketUniverse::new().unwrap();
        universe.invoke_projection_command(seed).unwrap();
        let news = |universe: &PocketUniverse| {
            let snapshot = universe.projection_snapshot();
            (
                snapshot.book.iter().filter(|entry| entry.found).count()
                    + snapshot.keepsakes.len()
                    + snapshot.letters.len()
                    + snapshot.chapters.len(),
                snapshot.keepsakes.len() + snapshot.letters.len(),
            )
        };
        let (mut before, _) = news(&universe);
        let mut quiet = Vec::new();
        for period in 1..=30 {
            let snapshot = universe.projection_snapshot();
            if let Some(answer) = snapshot.commands.iter().find(|command| {
                command.question.is_some()
                    && command.unavailable.is_none()
                    && command.id != NUDGE_COMMAND
            }) {
                let _ = universe.invoke_projection_command(&answer.id.clone());
            }
            if period % 3 == 0 {
                if let Some(deed) = snapshot
                    .commands
                    .iter()
                    .find(|command| command.hand.is_some() && command.unavailable.is_none())
                {
                    let _ = universe.invoke_projection_command(&deed.id.clone());
                }
            }
            universe.invoke_projection_command(NUDGE_COMMAND).unwrap();
            let (now, _) = news(&universe);
            if now <= before {
                quiet.push(period);
            }
            before = now;
        }
        assert!(quiet.is_empty(), "{seed}: nothing new in periods {quiet:?}");
        let (_, keepsakes) = news(&universe);
        assert!(
            keepsakes >= 4,
            "{seed}: {keepsakes} keepsakes and letters in a month"
        );
    }
}

/// Nothing is asked over and over: a player who answers the first open
/// question every period and builds every third, for a year in each place,
/// is asked no one question (by its words) more than six times.
#[test]
fn no_question_is_asked_more_than_six_times_a_year() {
    for seed in [
        MARS,
        crate::SEED_1980S_TOWN_COMMAND,
        crate::SEED_PENGUIN_CIVILIZATION_COMMAND,
    ] {
        let mut universe = PocketUniverse::new().unwrap();
        universe.invoke_projection_command(seed).unwrap();
        let mut opened = std::collections::BTreeMap::<String, usize>::new();
        let mut open_before = std::collections::BTreeSet::<String>::new();
        for period in 0..365 {
            let snapshot = universe.projection_snapshot();
            let open_now = snapshot
                .commands
                .iter()
                .filter_map(|command| command.question.as_ref())
                .map(|question| question.prompt.clone())
                .collect::<std::collections::BTreeSet<_>>();
            for prompt in open_now.difference(&open_before) {
                *opened.entry(prompt.clone()).or_default() += 1;
            }
            if let Some(answer) = snapshot.commands.iter().find(|command| {
                command.question.is_some()
                    && command.unavailable.is_none()
                    && command.id != NUDGE_COMMAND
            }) {
                universe.invoke_projection_command(&answer.id).unwrap();
            }
            if period % 3 == 0 {
                if let Some(deed) = snapshot
                    .commands
                    .iter()
                    .find(|command| command.hand.is_some() && command.unavailable.is_none())
                {
                    let _ = universe.invoke_projection_command(&deed.id);
                }
            }
            // What is still open after the answer stays the same question.
            open_before = universe
                .projection_snapshot()
                .commands
                .iter()
                .filter_map(|command| command.question.as_ref())
                .map(|question| question.prompt.clone())
                .collect();
            universe.invoke_projection_command(NUDGE_COMMAND).unwrap();
        }
        let too_often = opened
            .iter()
            .filter(|(_, times)| **times > 6)
            .collect::<Vec<_>>();
        assert!(
            too_often.is_empty(),
            "{seed}: asked too often: {too_often:#?}"
        );
    }
}

/// What a warm player's day brought, as the long runs read it.
pub(crate) struct Day {
    /// Every line said on the day.
    pub(crate) lines: Vec<String>,
    /// How many of those lines had never been heard before.
    pub(crate) new_lines: usize,
    /// Goals and works finished by the day's end, in the deck's order.
    pub(crate) finished: Vec<bool>,
    /// The works (by goal) asked for on the day.
    pub(crate) asked: Vec<String>,
    /// Letters written to the player on the day.
    pub(crate) letters: usize,
    /// Whether the day brought nothing new, as Tiny Society's long run
    /// counts it: nothing found for the book (a first is), nothing to
    /// keep, no letter and no chapter's close.
    pub(crate) quiet: bool,
}

/// A warm player's `days` in a place, as the Tiny Society second-year
/// player plays: each day they answer the first question on offer, every
/// third day they make something (something not yet made, if they can),
/// and they let the day pass. Read cheaply, without a whole snapshot a
/// day, so three years stay quick.
pub(crate) fn warm(seed: &str, days: usize) -> (PocketUniverse, Vec<Day>) {
    let mut universe = PocketUniverse::new().unwrap();
    universe.invoke_projection_command(seed).unwrap();
    let deck = story::deck();
    let place = crate::places::Place::of(universe.world().state()).unwrap();
    let goals = story::goal_ids(place);
    let mut heard = std::collections::BTreeSet::new();
    let news = |world: &world_core::World| {
        crate::book::book(world)
            .iter()
            .filter(|entry| entry.found)
            .count()
            + lives::keepsakes(world).len()
            + lives::letters(world).len()
            + storylets::chapters_ended(world).len()
    };
    let mut before = news(universe.world());
    let mut out = Vec::new();
    for day in 1..=days {
        let world = universe.world();
        let now = world.world_time();
        let events = world.events();
        let today = events.partition_point(|event| event.world_time < now);
        let today = &events[today..];
        let ids = today
            .iter()
            .map(|event| event.id)
            .collect::<std::collections::BTreeSet<_>>();
        let lines = talk::voices(world)
            .into_iter()
            .filter(|voice| matches!(voice.moment, SelectionId::Event(id) if ids.contains(&id)))
            .map(|voice| voice.line)
            .collect::<Vec<_>>();
        let new_lines = lines
            .iter()
            .filter(|line| heard.insert((*line).clone()))
            .count();
        let asked = today
            .iter()
            .filter(|event| event.kind == "situation_arose")
            .filter_map(|event| match event.payload.get("storylet") {
                Some(world_core::Value::Text(id)) => id.strip_prefix("work_").map(str::to_string),
                _ => None,
            })
            .collect();
        let letters = today
            .iter()
            .filter(|event| event.kind == "letter_written")
            .count();
        let now_news = news(world);
        let quiet = now_news <= before;
        before = now_news;
        out.push(Day {
            lines,
            new_lines,
            finished: goals
                .iter()
                .map(|goal| storylets::finished(world.state(), &deck, goal))
                .collect(),
            asked,
            letters,
            quiet,
        });

        let commands = story::commands(world);
        let answer = commands
            .iter()
            .find(|command| command.question.is_some() && command.unavailable.is_none())
            .map(|command| command.id.clone());
        let deed = (day % 3 == 0).then(|| {
            let made = hands::ever_made(world);
            let kit = crate::handwork::kit(world.state());
            let unmade = kit
                .things
                .iter()
                .filter(|thing| !made.contains(thing.id))
                .map(|thing| thing.name)
                .collect::<std::collections::BTreeSet<_>>();
            let hands = crate::handwork::commands(world)
                .into_iter()
                .chain(crate::life::suggestions(world))
                .filter(|command| command.unavailable.is_none())
                .filter(|command| {
                    command
                        .hand
                        .as_ref()
                        .is_some_and(|hand| hand.verb != "Undo")
                })
                .collect::<Vec<_>>();
            hands
                .iter()
                .find(|command| unmade.contains(command.hand.as_ref().unwrap().thing.as_str()))
                .or(hands.first())
                .map(|command| command.id.clone())
        });
        if let Some(answer) = answer {
            let _ = universe.invoke_projection_command(&answer);
        }
        if let Some(Some(deed)) = deed {
            let _ = universe.invoke_projection_command(&deed);
        }
        if let Err(error) = universe.invoke_projection_command(NUDGE_COMMAND) {
            panic!("{seed}: day {day} would not pass: {error}");
        }
    }
    (universe, out)
}

/// Three years of the v0.19 bars, days counted from the first: nine of
/// each place's years.
pub(crate) const THREE_YEARS: usize = 1_080;

/// The v0.19 bars over three years of a warm player, in one place: a work
/// finishes at least every 60 days, no rung of the ladder waits more than
/// 45 days to be asked, at most two letters in any seven days, and at
/// most four quiet days in any 120.
fn three_years(seed: &str) -> ThreeYears {
    let (universe, days) = warm(seed, THREE_YEARS);
    let place = crate::places::Place::of(universe.world().state()).unwrap();
    let goals = story::goal_ids(place);

    // The day each goal and work was finished.
    let finished_on = |goal: usize| days.iter().position(|day| day.finished[goal]);
    let mut finishes = (0..goals.len()).filter_map(finished_on).collect::<Vec<_>>();
    finishes.sort_unstable();
    let mut gaps = Vec::new();
    let mut last = 0;
    for day in finishes.iter().copied().chain([days.len()]) {
        gaps.push((last, day - last));
        last = day;
    }
    let longest = gaps.iter().max_by_key(|(_, gap)| *gap).copied().unwrap();

    // How long each rung waited to be asked: from the day it could be
    // asked (the rung before it finished) or last was, until it is asked
    // again, while it stands unfinished.
    let mut longest_wait = (String::new(), 0, 0);
    for (index, goal) in goals.iter().enumerate().skip(3) {
        let open_from = if index == 3 {
            (0..3)
                .filter_map(finished_on)
                .max()
                .filter(|_| (0..3).all(|g| finished_on(g).is_some()))
        } else {
            finished_on(index - 1)
        };
        let Some(open_from) = open_from else {
            continue;
        };
        let until = finished_on(index).unwrap_or(days.len());
        let mut since = open_from;
        for (at, day) in days.iter().enumerate().take(until).skip(open_from) {
            if day.asked.iter().any(|asked| asked == goal) {
                since = at;
            }
            if at - since > longest_wait.1 {
                longest_wait = (goal.to_string(), at - since, since);
            }
        }
    }

    let most_letters = days
        .windows(7)
        .map(|week| week.iter().map(|day| day.letters).sum::<usize>())
        .max()
        .unwrap_or(0);
    let most_quiet = days
        .windows(120)
        .map(|window| window.iter().filter(|day| day.quiet).count())
        .max()
        .unwrap_or(0);
    eprintln!(
        "{seed}: {} of {} goals finished; longest without a finish {} days from day {}; \
         longest a rung waited to be asked {} days ({} from day {}); \
         most letters in 7 days {most_letters}; most quiet days in 120 {most_quiet}; \
         {} letters and {} firsts in all",
        finishes.len(),
        goals.len(),
        longest.1,
        longest.0,
        longest_wait.1,
        longest_wait.0,
        longest_wait.2,
        days.iter().map(|day| day.letters).sum::<usize>(),
        lives::firsts(universe.world()).len(),
    );
    assert!(
        longest.1 <= 60,
        "{seed}: {} days without a work finished, from day {} (finished on {finishes:?})",
        longest.1,
        longest.0
    );
    assert!(
        longest_wait.1 <= 45,
        "{seed}: {} waited {} days to be asked, from day {}",
        longest_wait.0,
        longest_wait.1,
        longest_wait.2
    );
    assert!(
        most_letters <= 2,
        "{seed}: {most_letters} letters in a week"
    );
    assert!(
        most_quiet <= 4,
        "{seed}: {most_quiet} quiet days in 120 (on days {:?})",
        days.iter()
            .enumerate()
            .filter(|(_, day)| day.quiet)
            .map(|(at, _)| at + 1)
            .collect::<Vec<_>>()
    );
    let world = universe.world();
    assert_eq!(world.replay().unwrap().state(), world.state());
    let period = |event: &world_core::Event| event.world_time / crate::BACKGROUND_PERIOD;
    ThreeYears {
        works: story::works_finished(world.state()),
        turned_on: world
            .events()
            .iter()
            .filter(|event| event.kind == "year_turned")
            .map(period)
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect(),
        festivals: world
            .events()
            .iter()
            .filter(|event| event.kind == "festival_held")
            .count(),
    }
}

/// How a place stands after three years of a warm player.
#[derive(Debug)]
struct ThreeYears {
    /// Works finished: its ladder's and its people's own.
    works: i64,
    /// The periods its years turned on.
    turned_on: Vec<u64>,
    /// Festivals held.
    festivals: usize,
}

/// Three years in every place keep moving, and each place in its own
/// way: none ends with the works, the turns or the festivals of another.
#[test]
fn three_years_in_every_place_keep_moving_each_in_its_own_way() {
    let places = std::thread::scope(|scope| {
        [
            MARS,
            crate::SEED_1980S_TOWN_COMMAND,
            crate::SEED_PENGUIN_CIVILIZATION_COMMAND,
        ]
        .map(|seed| scope.spawn(move || three_years(seed)))
        .map(|place| place.join().unwrap())
    });
    eprintln!("{places:#?}");
    for (at, one) in places.iter().enumerate() {
        for other in &places[at + 1..] {
            assert_ne!(one.works, other.works, "{places:#?}");
            assert_ne!(one.turned_on, other.turned_on, "{places:#?}");
            assert_ne!(one.festivals, other.festivals, "{places:#?}");
        }
        // Its years keep turning to the end of the three, and people still
        // come and go through them.
        assert!(
            one.turned_on.iter().any(|day| *day >= 960),
            "years stop turning: {:?}",
            one.turned_on
        );
    }
}

/// How a scripted player answers, as the review's players do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Scripted {
    /// The first answer on offer, every day.
    First,
    /// The last answer of the first question open, often a no.
    Last,
    /// Never answers, but makes something every third day.
    Never,
    /// Does nothing at all.
    Absent,
}

/// How a place stands after a scripted player has kept it.
#[derive(Clone, Debug, PartialEq)]
struct Kept {
    /// Works finished: its ladder's and its people's own.
    works: i64,
    /// Who lives there, and how many came and went.
    people: Vec<String>,
    arrived: usize,
    departed: usize,
    /// What it has put by, and how the pair stand.
    stores: i64,
    trust: i64,
    tension: i64,
    /// What everyone living there thinks of everyone else, added up, and
    /// how many couples there are.
    regard_between: i64,
    couples: usize,
}

/// `days` of a scripted player in a place.
fn kept(seed: &str, policy: Scripted, days: usize) -> Kept {
    let mut universe = PocketUniverse::new().unwrap();
    universe.invoke_projection_command(seed).unwrap();
    for day in 1..=days {
        let world = universe.world();
        let questions = story::commands(world)
            .into_iter()
            .filter(|command| command.question.is_some() && command.unavailable.is_none())
            .collect::<Vec<_>>();
        let answer = match policy {
            Scripted::First => questions.first().map(|command| command.id.clone()),
            Scripted::Last => questions.first().and_then(|first| {
                let asked = first.question.as_ref().map(|question| question.id.clone());
                questions
                    .iter()
                    .rfind(|command| command.question.as_ref().map(|q| q.id.clone()) == asked)
                    .map(|command| command.id.clone())
            }),
            Scripted::Never | Scripted::Absent => None,
        };
        let deed = (policy != Scripted::Absent && day % 3 == 0)
            .then(|| {
                let made = hands::ever_made(world);
                let kit = crate::handwork::kit(world.state());
                let unmade = kit
                    .things
                    .iter()
                    .filter(|thing| !made.contains(thing.id))
                    .map(|thing| thing.name)
                    .collect::<std::collections::BTreeSet<_>>();
                let hands = crate::handwork::commands(world)
                    .into_iter()
                    .chain(crate::life::suggestions(world))
                    .filter(|command| command.unavailable.is_none())
                    .filter(|command| {
                        command
                            .hand
                            .as_ref()
                            .is_some_and(|hand| hand.verb != "Undo")
                    })
                    .collect::<Vec<_>>();
                hands
                    .iter()
                    .find(|command| unmade.contains(command.hand.as_ref().unwrap().thing.as_str()))
                    .or(hands.first())
                    .map(|command| command.id.clone())
            })
            .flatten();
        if let Some(answer) = answer {
            let _ = universe.invoke_projection_command(&answer);
        }
        if let Some(deed) = deed {
            let _ = universe.invoke_projection_command(&deed);
        }
        universe.invoke_projection_command(NUDGE_COMMAND).unwrap();
    }
    let world = universe.world();
    let state = world.state();
    let living = crate::life::people(world);
    let everyone = crate::life::newcomers(state);
    let bond = |key: &str| match state
        .entity(crate::RELATIONSHIP)
        .and_then(|bond| bond.component(key))
    {
        Some(world_core::Value::Integer(value)) => *value,
        _ => 0,
    };
    Kept {
        works: story::works_finished(state),
        people: living
            .iter()
            .map(|person| lives::name(state, *person))
            .collect(),
        arrived: everyone.len(),
        departed: everyone
            .iter()
            .filter(|person| lives::gone(state, **person))
            .count(),
        stores: crate::years::stores(state),
        trust: bond(crate::RELATIONSHIP_TRUST),
        tension: bond(crate::RELATIONSHIP_TENSION),
        regard_between: living
            .iter()
            .flat_map(|a| living.iter().map(move |b| (*a, *b)))
            .filter(|(a, b)| a != b)
            .map(|(a, b)| lives::opinion(state, a, b))
            .sum(),
        couples: living
            .iter()
            .filter(|person| lives::partner(state, **person).is_some())
            .count()
            / 2,
    }
}

/// Every way of keeping a place, side by side.
fn four_ways(seed: &str, days: usize) -> [(Scripted, Kept); 4] {
    std::thread::scope(|scope| {
        [
            Scripted::First,
            Scripted::Last,
            Scripted::Never,
            Scripted::Absent,
        ]
        .map(|policy| (policy, scope.spawn(move || kept(seed, policy, days))))
        .map(|(policy, kept)| (policy, kept.join().unwrap()))
    })
}

/// Four ways of keeping a place for a season already make four places:
/// no two of them stand the same way.
#[test]
fn four_ways_of_keeping_a_place_part_within_a_season() {
    for seed in [
        MARS,
        crate::SEED_1980S_TOWN_COMMAND,
        crate::SEED_PENGUIN_CIVILIZATION_COMMAND,
    ] {
        let ways = four_ways(seed, 120);
        eprintln!("{seed}: {ways:#?}");
        for (at, (one, kept)) in ways.iter().enumerate() {
            for (other, other_kept) in &ways[at + 1..] {
                assert_ne!(
                    kept, other_kept,
                    "{seed}: {one:?} and {other:?} kept the same place"
                );
            }
        }
        // Whoever answers first builds; whoever is not there does not
        // lend a hand, and the people make nothing of their own.
        let works = |policy| ways.iter().find(|(way, _)| *way == policy).unwrap().1.works;
        assert!(
            works(Scripted::First) > works(Scripted::Absent),
            "{seed}: {ways:#?}"
        );
    }
}

/// The review's bar: after three years, the four ways of keeping a place
/// differ on its works, its people, what it has put by and how its people
/// get on, and someone who never says yes still sees works finished, their
/// own way.
#[test]
#[ignore]
fn four_ways_of_keeping_a_place_make_four_places_in_three_years() {
    for seed in [
        MARS,
        crate::SEED_1980S_TOWN_COMMAND,
        crate::SEED_PENGUIN_CIVILIZATION_COMMAND,
    ] {
        let ways = four_ways(seed, THREE_YEARS);
        eprintln!("{seed}: {ways:#?}");
        for (at, (one, kept)) in ways.iter().enumerate() {
            for (other, other_kept) in &ways[at + 1..] {
                let why = format!("{seed}: {one:?} and {other:?}: {kept:#?} {other_kept:#?}");
                assert_ne!(kept.works, other_kept.works, "works: {why}");
                assert_ne!(
                    (&kept.people, kept.arrived, kept.departed),
                    (&other_kept.people, other_kept.arrived, other_kept.departed),
                    "people: {why}"
                );
                assert_ne!(
                    (kept.stores, kept.trust, kept.tension),
                    (other_kept.stores, other_kept.trust, other_kept.tension),
                    "stores: {why}"
                );
                assert_ne!(
                    (kept.regard_between, kept.couples),
                    (other_kept.regard_between, other_kept.couples),
                    "friendships: {why}"
                );
            }
            if *one != Scripted::First {
                assert!(
                    kept.works >= 10,
                    "{seed}: {one:?} finished {} works",
                    kept.works
                );
            }
        }
    }
}

/// Three years of lives in each place, kept warmly: at least two born in
/// each, as the place has them (a baby under the dome, a baby on Maple
/// Street, chicks in Icebridge's thaw); fewer, weightier changes between
/// people; and no storylet asked under one title more than three times.
#[test]
#[ignore]
fn three_years_of_lives_in_every_place() {
    let places = std::thread::scope(|scope| {
        [
            MARS,
            crate::SEED_1980S_TOWN_COMMAND,
            crate::SEED_PENGUIN_CIVILIZATION_COMMAND,
        ]
        .map(|seed| {
            scope.spawn(move || {
                let (universe, _) = warm(seed, THREE_YEARS);
                (seed, universe.world().clone())
            })
        })
        .map(|place| place.join().unwrap())
    });
    for (seed, world) in &places {
        let count = |kind: &str| world.events_of_kind(&[kind]).len();
        let born = count("born");
        let bonds = world
            .events_of_kind(&["bond_changed"])
            .into_iter()
            .map(|event| (event.world_time / crate::BACKGROUND_PERIOD) as usize)
            .collect::<Vec<_>>();
        let most_bonds = (0..=THREE_YEARS - 360)
            .map(|start| {
                bonds
                    .iter()
                    .filter(|at| (start..start + 360).contains(*at))
                    .count()
            })
            .max()
            .unwrap_or(0);
        let mut titles = std::collections::BTreeMap::<String, usize>::new();
        for event in world.events_of_kind(&["situation_arose"]) {
            if let Some(told) = story::told(world, event) {
                *titles.entry(told).or_default() += 1;
            }
        }
        let repeated = titles
            .into_iter()
            .filter(|(_, times)| *times > 3)
            .collect::<Vec<_>>();
        eprintln!(
            "{seed}: born {born}, died {}, came of age {}, retired {}, memorials {}; at most {most_bonds} changes between people in 360 days; titles over 3: {repeated:?}",
            count("died"),
            count("came_of_age"),
            count("retired"),
            count("memorial_placed"),
        );
        for event in world.events_of_kind(&["born", "died", "came_of_age"]) {
            eprintln!(
                "  {}: {}",
                event.world_time / crate::BACKGROUND_PERIOD,
                lives::told(event).unwrap_or_default()
            );
        }
        assert!(born >= 2, "{seed}: {born} born");
        assert!(most_bonds <= 61, "{seed}: {most_bonds}");
        assert!(repeated.is_empty(), "{seed}: {repeated:?}");
        assert_eq!(world.replay().unwrap().state(), world.state());
    }
}
