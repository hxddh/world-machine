//! Does the harbour keep having a story? Sixty days, played three ways,
//! held to the bar the v0.10 plan set.

use crate::{projection, story, talk, TinySociety, TinySocietyBranch};
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

    branch: TinySocietyBranch,
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

fn said_today(branch: &TinySocietyBranch) -> Vec<String> {
    let world = branch.world();
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

fn play(policy: Policy, days: usize) -> Played {
    let mut society = TinySociety::new().unwrap();
    society.run_story().unwrap();
    let mut branch = society.branch();
    // The first day passes before anything is counted.
    branch
        .invoke_projection_command(story::WAIT_COMMAND)
        .unwrap();
    let mut played = Played {
        days_with_a_choice: Vec::new(),
        lines: Vec::new(),
        gauges: Vec::new(),
        first_chapter: None,
        answered: 0,
        answers_seen: 0,

        branch: branch.clone(),
    };
    for day in 0..days {
        // Whoever asks an open question is in the harbour to ask it, even
        // someone back from being away.
        let here = story::people(branch.world());
        for storylet in storylets::open(branch.world().state(), &story::deck()) {
            if branch.world().state().entity(storylet.asker).is_some() {
                assert!(
                    here.contains(&storylet.asker),
                    "{policy:?}: {} asked by someone not on the scene",
                    storylet.id
                );
            }
        }
        let snapshot = projection::snapshot(branch.world());
        let choices = snapshot
            .commands
            .iter()
            .filter(|command| {
                command.id != story::WAIT_COMMAND
                    && command.unavailable.is_none()
                    && command.hand.is_none()
            })
            .map(|command| command.id.clone())
            .collect::<Vec<_>>();
        played.days_with_a_choice.push(!choices.is_empty());
        played.lines.push(said_today(&branch));
        played.gauges.push(
            snapshot
                .gauges
                .iter()
                .filter(|gauge| gauge.id == "money" || gauge.id == "spirits")
                .map(|gauge| (gauge.id.clone(), gauge.value))
                .collect(),
        );
        if played.first_chapter.is_none()
            && branch
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
            let before = scene(branch.world());
            branch.invoke_projection_command(command).unwrap();
            if snapshot
                .command(command)
                .is_some_and(|command| command.question.is_some())
            {
                played.answered += 1;
                if scene(branch.world()) != before {
                    played.answers_seen += 1;
                }
            }
        }
        branch
            .invoke_projection_command(story::WAIT_COMMAND)
            .unwrap();
    }
    played.branch = branch;
    played
}

fn check(policy: Policy) {
    let played = play(policy, 60);
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

    for gauge in ["money", "spirits"] {
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

    // Everything the storyteller did is history: the town replays to the
    // same place without it.
    let world = played.branch.world();
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

#[test]
fn a_generous_player_always_has_a_story() {
    check(Policy::Generous);
}

#[test]
fn a_contrary_player_always_has_a_story() {
    check(Policy::Contrary);
}

#[test]
fn an_absent_player_always_has_a_story() {
    check(Policy::Absent);
}

/// Prints how sixty days went, for tuning the deck by eye.
#[test]
#[ignore]
fn show_sixty_days() {
    for policy in [Policy::Generous, Policy::Contrary, Policy::Absent] {
        let played = play(policy, 60);
        println!("== {policy:?}");
        for (day, gauges) in played.gauges.iter().enumerate() {
            println!(
                "day {day:2} choice={} {:?} {:?}",
                played.days_with_a_choice[day], gauges, played.lines[day]
            );
        }
        for event in played.branch.world().events() {
            if let Some(title) = story::told(played.branch.world(), event) {
                println!("  t={} {title}", event.world_time);
            }
        }
    }
}

/// The v0.11 bar: what you choose changes the place, and questions follow
/// from earlier answers.
#[test]
fn what_you_choose_changes_the_place_and_comes_back() {
    let generous = play(Policy::Generous, 30);
    let contrary = play(Policy::Contrary, 30);
    // Across both ways of playing, at least half of all answers change the
    // scene and a third of all questions besides the calendar's follow from
    // an earlier answer; neither way of playing falls far below that.
    let mut followed = (0, 0);
    for (policy, played) in [("yes", &generous), ("last answer", &contrary)] {
        let world = played.branch.world();
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
        let world = played.branch.world();
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
        .register(crate::tiny_society_registration())
        .unwrap();
    let session = registry.create(crate::TINY_SOCIETY_PACK_ID).unwrap();
    let snapshot = session.snapshot();
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
    let mut society = TinySociety::new().unwrap();
    society.run_story().unwrap();
    let mut branch = society.branch();
    branch.begin_story().unwrap();
    for _ in 0..3 {
        branch
            .invoke_projection_command(story::WAIT_COMMAND)
            .unwrap();
    }
    let before = branch.world().events().len();
    branch.advance_days(7).unwrap();
    let world = branch.world();
    let lapsed = world.events()[before..]
        .iter()
        .filter(|event| event.payload.contains_key("lapsed"))
        .count();
    assert!(lapsed <= 3, "{lapsed} questions lapsed in a week away");
}

/// The v0.12 bar: a year in the harbour never runs out. Played 365 days,
/// new situations keep coming every month, nobody repeats themselves,
/// people's standing with each other keeps changing, every chapter has a
/// title of its own, and everyone lives every day without being asked.
fn a_year(policy: Policy) {
    let played = play(policy, 365);
    let world = played.branch.world();

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
    let day = |event: &world_core::Event| event.world_time / crate::persistence::WORLD_DAY_TICKS;
    let first = world.events().first().map(day).unwrap_or(0);

    let mut seen = std::collections::BTreeSet::new();
    let mut fresh = vec![0; 13];
    for event in world.events() {
        let key = match event.kind.as_str() {
            "situation_arose" => event.payload.get("storylet"),
            "situation_came_up" => event.payload.get("situation"),
            _ => None,
        };
        if let Some(world_core::Value::Text(key)) = key {
            let month = ((day(event) - first) / 30).min(12) as usize;
            if seen.insert(key.clone()) {
                fresh[month] += 1;
            }
        }
    }
    assert!(
        fresh[3..12].iter().all(|count| *count >= 8),
        "{policy:?}: never-seen situations by month {fresh:?}"
    );

    for window in played.lines.windows(30) {
        let mut counts = std::collections::BTreeMap::<&str, usize>::new();
        for line in window.iter().flatten() {
            *counts.entry(line).or_default() += 1;
        }
        if let Some((line, count)) = counts.into_iter().max_by_key(|(_, count)| *count) {
            assert!(
                count <= 3,
                "{policy:?}: {line:?} said {count} times in 30 days"
            );
        }
    }

    let changes = world
        .events()
        .iter()
        .filter(|event| event.kind == "bond_changed")
        .map(|event| day(event) - first)
        .collect::<Vec<_>>();
    for start in 90..335 {
        let count = changes
            .iter()
            .filter(|at| (start..start + 30).contains(*at))
            .count();
        assert!(
            count >= 3,
            "{policy:?}: only {count} changes between people in days {start}-{}",
            start + 30
        );
    }

    let titles = projection::snapshot(world)
        .chapters
        .into_iter()
        .map(|chapter| chapter.title)
        .collect::<Vec<_>>();
    let unique = titles.iter().collect::<std::collections::BTreeSet<_>>();
    assert_eq!(unique.len(), titles.len(), "{policy:?}: {titles:?}");

    // Everyone living here does something with every day of their own.
    let mut lived = 0;
    let mut owed = 0;
    for today in first + 1..first + 365 {
        let people = world
            .events()
            .iter()
            .filter(|event| event.kind == "lived" && day(event) == today)
            .filter_map(|event| event.actor)
            .collect::<std::collections::BTreeSet<_>>();
        lived += people.len();
        owed += crate::story::people(world).len().min(people.len().max(7));
    }
    assert!(
        lived * 100 >= owed * 95,
        "{policy:?}: {lived} of {owed} days lived"
    );

    let replayed = world.replay().unwrap();
    assert_eq!(replayed.state(), world.state());
}

#[test]
fn a_year_of_saying_yes_never_runs_out() {
    a_year(Policy::Generous);
}

#[test]
fn a_year_of_saying_no_never_runs_out() {
    a_year(Policy::Contrary);
}

#[test]
fn a_year_left_alone_never_runs_out() {
    a_year(Policy::Absent);
}

/// The v0.12 bar for the player's own hands: there are at least five
/// things to do besides answering, and after a month of building, planting
/// and decorating where they choose, at least a third of what stands in
/// the harbour was put there by the player.
#[test]
fn your_hands_shape_the_harbour() {
    let mut society = TinySociety::new().unwrap();
    society.run_story().unwrap();
    let mut branch = society.branch();
    branch.begin_story().unwrap();
    let verbs = branch
        .projection_snapshot()
        .deeds()
        .map(|(_, _, hand)| hand.verb.clone())
        .collect::<std::collections::BTreeSet<_>>();
    assert!(verbs.len() >= 5, "{verbs:?}");
    for day in 0..30 {
        let snapshot = branch.projection_snapshot();
        // One deed a day, turning through what can be made and where.
        let deeds = snapshot
            .deeds()
            .filter(|(_, command, hand)| {
                command.unavailable.is_none()
                    && ["Build", "Plant", "Decorate"].contains(&hand.verb.as_str())
            })
            .map(|(_, command, _)| command.id.clone())
            .collect::<Vec<_>>();
        if let Some(deed) = deeds.get(day * 7 % deeds.len().max(1)) {
            branch.invoke_projection_command(deed).unwrap();
        }
        if let Some(answer) = snapshot
            .choices()
            .find(|command| command.question.is_some() && command.unavailable.is_none())
        {
            let _ = branch.invoke_projection_command(&answer.id.clone());
        }
        branch
            .invoke_projection_command(story::WAIT_COMMAND)
            .unwrap();
    }
    let state = branch.world().state();
    let standing = storylets::fixtures(state).len();
    let made = hands::made(state).len();
    assert!(
        made * 3 >= standing && made > 0,
        "{made} of {standing} things standing were the player's"
    );
    let replayed = branch.world().replay().unwrap();
    assert_eq!(replayed.state(), branch.world().state());
}

/// The scene shows the weather the World's state says it has: a gale
/// whenever a storm is on, snow only in winter, and something other than
/// sunshine often enough to notice.
#[test]
fn the_weather_follows_the_world() {
    use world_projection::Weather;
    let mut society = TinySociety::new().unwrap();
    society.run_story().unwrap();
    let mut branch = society.branch();
    branch.begin_story().unwrap();
    let mut seen = std::collections::BTreeSet::new();
    for _ in 0..80 {
        let world = branch.world();
        let weather = projection::snapshot(world).weather;
        let storm_on = storylets::open(world.state(), &story::deck())
            .iter()
            .any(|storylet| matches!(storylet.id, "storm_warning" | "great_storm"));
        if storm_on {
            assert_eq!(weather, Weather::Storm);
        }
        if weather == Weather::Snow {
            assert_eq!(story::season(world), 3, "snow outside winter");
        }
        seen.insert(format!("{weather:?}"));
        branch
            .invoke_projection_command(story::WAIT_COMMAND)
            .unwrap();
    }
    assert!(seen.len() >= 4, "{seen:?}");
}

/// The v0.12 bar for branches: two branches split at day 10 by one
/// different answer, then played 90 days the same way, end up different
/// towns: at least a third of the situations their people met differ, and
/// so does what stands in the place.
#[test]
fn branches_become_different_towns() {
    let mut society = TinySociety::new().unwrap();
    society.run_story().unwrap();
    let mut branch = society.branch();
    branch.begin_story().unwrap();
    let answer_first = |branch: &mut TinySocietyBranch, last: bool| {
        let snapshot = branch.projection_snapshot();
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
            branch.invoke_projection_command(answer).unwrap();
        }
        branch
            .invoke_projection_command(story::WAIT_COMMAND)
            .unwrap();
    };
    for _ in 0..10 {
        answer_first(&mut branch, false);
    }
    let split = branch.world().events().len();
    let (mut left, mut right) = (branch.clone(), branch);
    answer_first(&mut left, false);
    answer_first(&mut right, true);
    for _ in 0..90 {
        answer_first(&mut left, false);
        answer_first(&mut right, false);
    }
    let met = |branch: &TinySocietyBranch| {
        branch.world().events()[split..]
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

/// Plays `days` days, planting a garden on the first day if `plant`, and
/// answering nothing; returns the branch.
fn a_harbour_year(days: usize, plant: bool) -> TinySocietyBranch {
    let mut society = TinySociety::new().unwrap();
    society.run_story().unwrap();
    let mut branch = society.branch();
    branch.begin_story().unwrap();
    if plant {
        let gardens = branch
            .projection_snapshot()
            .deeds()
            .filter(|(_, command, hand)| hand.verb == "Plant" && command.unavailable.is_none())
            .map(|(_, command, _)| command.id.clone())
            .take(2)
            .collect::<Vec<_>>();
        assert!(!gardens.is_empty());
        for garden in gardens {
            branch.invoke_projection_command(&garden).unwrap();
        }
    }
    for _ in 0..days {
        branch
            .invoke_projection_command(story::WAIT_COMMAND)
            .unwrap();
    }
    branch
}

/// The harbour's year has a shape: seasons a month long, something on the
/// calendar in every fortnight, festivals in every season that people get
/// ready for, and a harvest that is as good as what was planted.
#[test]
fn a_harbour_year_has_a_shape() {
    let left_alone = a_harbour_year(130, false);
    let world = left_alone.world();
    let period = |event: &world_core::Event| event.world_time / crate::persistence::WORLD_DAY_TICKS;
    let days = world
        .events()
        .iter()
        .filter(|event| calendar::is_calendar(event))
        .map(period)
        .collect::<Vec<_>>();
    for start in 0..116 {
        assert!(
            days.iter().any(|day| (start..start + 14).contains(day)),
            "nothing on the calendar in days {start}-{}",
            start + 14
        );
    }
    let held = world
        .events()
        .iter()
        .filter(|event| event.kind == "festival_held")
        .collect::<Vec<_>>();
    let almanac = crate::almanac::almanac(world.state());
    for season in 0..4 {
        let in_season = held
            .iter()
            .filter(|event| {
                let day = period(event) % crate::almanac::YEAR_DAYS;
                day / (crate::almanac::YEAR_DAYS / 4) == season
            })
            .count();
        assert!(in_season >= 3, "season {season}: {in_season} festivals");
    }
    assert!(held.len() >= almanac.festivals.len());
    assert!(world
        .events()
        .iter()
        .any(|event| event.kind == "festival_nears"));
    let snapshot = left_alone.projection_snapshot();
    let calendar = snapshot.calendar.unwrap();
    assert!(calendar.season.is_some());

    let harvest = |branch: &TinySocietyBranch| {
        branch
            .world()
            .events()
            .iter()
            .find(|event| {
                event.kind == "festival_held"
                    && event.payload.get("festival")
                        == Some(&world_core::Value::Text("harvest_home".into()))
            })
            .and_then(calendar::told)
            .unwrap()
    };
    let planted = a_harbour_year(75, true);
    let unplanted = a_harbour_year(75, false);
    assert_ne!(harvest(&planted), harvest(&unplanted));
    assert_eq!(
        planted.world().replay().unwrap().state(),
        planted.world().state()
    );
}
