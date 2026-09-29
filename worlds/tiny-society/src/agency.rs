//! Your choices make the town. Four players keep the harbour: one takes
//! the first answer to every question, one the last (most often "not
//! now"), one never answers but makes something every third day, and one
//! is never there at all. Each town ends somewhere of its own: in what it
//! built, who lives there, its money, how its people feel about each other
//! and who is walking out together. Saying no, or saying nothing, leads
//! somewhere too: a harbour let down builds smaller things of its own, a
//! little at a time.
//!
//! The three-year run takes a few minutes in a debug build, so it is
//! ignored by default: run it with `cargo test -p tiny-society agency --
//! --ignored`. The first four months run every time.

use crate::{story, TinySociety};
use std::collections::BTreeSet;
use world_core::{EntityId, Value};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Player {
    /// Takes the first answer to every question.
    First,
    /// Takes the last answer of the first question: most often "not now".
    Last,
    /// Never answers, but makes something every third day.
    Never,
    /// Is never there: the days only pass.
    Absent,
}

const PLAYERS: [Player; 4] = [Player::First, Player::Last, Player::Never, Player::Absent];

/// How a town stands.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Town {
    /// Works finished: the pier, the lamp, the ladder's and the harbour's
    /// own.
    works: usize,
    /// Who lives there.
    people: BTreeSet<String>,
    /// Money in the town, the mainland market's aside.
    money: i64,
    /// The mean of what everyone living there thinks of everyone else, in
    /// hundredths.
    opinion: i64,
    /// Couples walking out together.
    couples: usize,
    /// Newcomers who came to stay, and those of them who left again.
    arrived: BTreeSet<String>,
    left: BTreeSet<String>,
}

fn play(player: Player, days: usize) -> Town {
    let mut society = TinySociety::new().unwrap();
    society.run_story().unwrap();
    let mut branch = society.branch();
    branch.begin_story().unwrap();
    for day in 1..=days {
        let snapshot = branch.projection_snapshot();
        let questions = snapshot
            .commands
            .iter()
            .filter(|command| {
                command.question.is_some()
                    && command.unavailable.is_none()
                    && command.id != story::WAIT_COMMAND
            })
            .collect::<Vec<_>>();
        let answer = match player {
            Player::First => questions.first().copied(),
            Player::Last => questions.first().and_then(|first| {
                questions
                    .iter()
                    .copied()
                    .rfind(|command| command.question == first.question)
            }),
            Player::Never | Player::Absent => None,
        };
        if let Some(answer) = answer {
            let _ = branch.invoke_projection_command(&answer.id.clone());
        }
        if player != Player::Absent && day % 3 == 0 {
            let unmade = snapshot
                .book
                .iter()
                .filter(|entry| entry.shelf == "Made" && !entry.found)
                .map(|entry| entry.name.clone())
                .collect::<BTreeSet<_>>();
            let hands = snapshot
                .commands
                .iter()
                .filter(|command| command.unavailable.is_none())
                .filter(|command| {
                    command
                        .hand
                        .as_ref()
                        .is_some_and(|hand| hand.verb != "Undo")
                })
                .collect::<Vec<_>>();
            let deed = hands
                .iter()
                .find(|command| unmade.contains(&command.hand.as_ref().unwrap().thing))
                .or(hands.first());
            if let Some(deed) = deed {
                let _ = branch.invoke_projection_command(&deed.id.clone());
            }
        }
        branch
            .invoke_projection_command(story::WAIT_COMMAND)
            .unwrap();
    }
    let world = branch.world();
    let replayed = world.replay().unwrap();
    assert_eq!(
        replayed.state(),
        world.state(),
        "{player:?} replays exactly"
    );
    town(world)
}

fn town(world: &world_core::World) -> Town {
    let state = world.state();
    let people = story::people(world);
    let mut sum = 0;
    let mut pairs = 0;
    for a in &people {
        for b in &people {
            if a != b {
                sum += lives::opinion(state, *a, *b);
                pairs += 1;
            }
        }
    }
    let visitors = crate::life::cast().visitors.unwrap();
    let came = (visitors.first..visitors.first + visitors.room)
        .map(EntityId::new)
        .chain([story::ADA, story::IVO])
        .filter(|id| state.entity(*id).is_some())
        .collect::<Vec<_>>();
    Town {
        works: story::goals(world)
            .iter()
            .filter(|goal| goal.finished())
            .count(),
        people: people
            .iter()
            .map(|person| lives::name(state, *person))
            .collect(),
        money: state
            .entities()
            .filter(|entity| entity.id != crate::model::MAINLAND_MARKET)
            .filter_map(|entity| match entity.component(society_basic::CASH) {
                Some(Value::Integer(cash)) => Some(*cash),
                _ => None,
            })
            .sum(),
        opinion: sum * 100 / pairs.max(1),
        couples: people
            .iter()
            .filter(|person| {
                lives::partner(state, **person).is_some_and(|partner| people.contains(&partner))
            })
            .count()
            / 2,
        arrived: came
            .iter()
            .map(|person| lives::name(state, *person))
            .collect(),
        left: came
            .iter()
            .filter(|person| lives::gone(state, **person))
            .map(|person| lives::name(state, *person))
            .collect(),
    }
}

/// Every player's town after `days`, played side by side.
fn towns(days: usize) -> Vec<(Player, Town)> {
    std::thread::scope(|scope| {
        let runs = PLAYERS
            .map(|player| scope.spawn(move || (player, play(player, days))))
            .into_iter()
            .collect::<Vec<_>>();
        runs.into_iter()
            .map(|run| run.join().unwrap())
            .collect::<Vec<_>>()
    })
}

/// Asserts that every two towns differ on what `of` reads.
fn all_differ<T: PartialEq + std::fmt::Debug>(
    towns: &[(Player, Town)],
    what: &str,
    of: impl Fn(&Town) -> T,
) {
    for (at, (a, town_a)) in towns.iter().enumerate() {
        for (b, town_b) in &towns[at + 1..] {
            assert_ne!(
                of(town_a),
                of(town_b),
                "{a:?} and {b:?} end with the same {what}"
            );
        }
    }
}

#[test]
#[ignore]
fn four_players_make_four_towns_in_three_years() {
    let towns = towns(1_080);
    for (player, town) in &towns {
        eprintln!(
            "{player:?}: {} works, {} people ({} came, {} left: {:?}), money {}, mean opinion {:.2}, {} couples; {:?}",
            town.works,
            town.people.len(),
            town.arrived.len(),
            town.left.len(),
            town.left,
            town.money,
            town.opinion as f64 / 100.0,
            town.couples,
            town.people
        );
    }
    all_differ(&towns, "number of works", |town| town.works);
    all_differ(&towns, "people", |town| town.people.clone());
    all_differ(&towns, "money", |town| town.money);
    all_differ(&towns, "mean opinion", |town| town.opinion);
    all_differ(&towns, "number of couples", |town| town.couples);
    // Who comes and who goes depends on the player.
    all_differ(&towns, "comings and goings", |town| {
        (town.arrived.clone(), town.left.clone())
    });
    // A town that is never told yes still builds.
    for (player, town) in &towns[1..] {
        assert!(
            town.works >= 10,
            "{player:?} finished only {} works",
            town.works
        );
    }
}

#[test]
fn the_four_towns_part_ways_within_four_months() {
    let towns = towns(120);
    for (player, town) in &towns {
        eprintln!("{player:?} at day 120: {town:?}");
    }
    // No two towns are the same town.
    for (at, (a, town_a)) in towns.iter().enumerate() {
        for (b, town_b) in &towns[at + 1..] {
            assert_ne!(town_a, town_b, "{a:?} and {b:?} are the same town");
        }
    }
    all_differ(&towns, "money", |town| town.money);
    all_differ(&towns, "mean opinion", |town| town.opinion);
    // Saying yes builds the most; the harbour starts its own works when it
    // is let down, and sooner when the player makes things.
    let works = |player| {
        towns
            .iter()
            .find(|(who, _)| *who == player)
            .map(|(_, town)| town.works)
            .unwrap()
    };
    assert!(works(Player::First) > works(Player::Absent));
}
