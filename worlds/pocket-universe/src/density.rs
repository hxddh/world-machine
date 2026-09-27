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

/// The v0.15 bar: a new World opens on the place, not a card. The first
/// deed can be done at once, someone nearby says what they make of it,
/// and the first question comes straight after.
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
            questions(&opened).is_empty(),
            "{seed}: {:?}",
            questions(&opened)
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

fn begun(seed: &str) -> PocketUniverse<crate::PocketMind> {
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
/// someone left the player to keep.
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
            assert!(
                last.contains(" left you "),
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
        let greeting = opened
            .voices
            .iter()
            .find(|voice| voice.line.contains("I'm "))
            .unwrap_or_else(|| panic!("{seed}: nobody says hello: {:?}", opened.voices));
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
        clock += before;
        clock += 3.0;
        assert!(clock <= 20.0, "{seed}: greeted at {clock} s");
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
        let after = session
            .handle(world_projection::ProjectionIntent::InvokeCommand(deed))
            .unwrap();
        let keepsake = after
            .keepsakes
            .first()
            .unwrap_or_else(|| panic!("{seed}: a keepsake for the first deed"));
        clock += 3.0 + read(&keepsake.what) + read(&keepsake.note);
        assert!(clock <= 300.0, "{seed}: first keepsake at {clock} s");
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
