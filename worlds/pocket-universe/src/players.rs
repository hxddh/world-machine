//! Five players, three years each, in every place: the bars for "a story
//! for every player", as the harbour's own `players` harness has them.
//!
//! - **Warm:** the first answer every time, and something made by hand
//!   every third period.
//! - **Builder:** the same, but building on a plot every third period when
//!   one is on offer.
//! - **Last:** the last answer to the first question on offer.
//! - **Never:** never answers, and makes something every third period.
//! - **Absent:** does nothing at all.
//!
//! The bars, in each place over 1,080 periods: no storylet told under the
//! same title more than three times; a legend cause share of at least 30%
//! for Last and Never; Never's and Absent's works at a Jaccard distance of
//! at least 0.5; plots opening in each of the three years; letters
//! averaging at most 1.5 a week, with some quiet days (at most four in any
//! 120).
//!
//! `cargo test -p pocket-universe --release --lib players -- --ignored --nocapture`;
//! `PLAYERS=Never,Absent` plays only those, `PLACES=mars` only that place.

use crate::{story, PocketUniverse, NUDGE_COMMAND};
use std::collections::{BTreeMap, BTreeSet};
use world_core::World;
use world_projection::SelectionId;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Player {
    Warm,
    Builder,
    Last,
    Never,
    Absent,
}

const PLAYERS: [Player; 5] = [
    Player::Warm,
    Player::Builder,
    Player::Last,
    Player::Never,
    Player::Absent,
];

const PLACES: [(&str, &str); 3] = [
    ("mars", crate::SEED_MARS_COLONY_COMMAND),
    ("maple", crate::SEED_1980S_TOWN_COMMAND),
    ("ice", crate::SEED_PENGUIN_CIVILIZATION_COMMAND),
];

const DAYS: usize = 1_080;
const YEAR: usize = 360;

struct Played {
    place: &'static str,
    player: Player,
    letters: Vec<usize>,
    quiet: Vec<bool>,
    plots_opened: BTreeMap<String, usize>,
    world: World,
}

fn play(place: &'static str, seed: &str, player: Player) -> Played {
    let mut universe = PocketUniverse::new().unwrap();
    universe.invoke_projection_command(seed).unwrap();
    let mut played = Played {
        place,
        player,
        letters: Vec::new(),
        quiet: Vec::new(),
        plots_opened: BTreeMap::new(),
        world: World::new(Default::default()),
    };
    let mut found = BTreeSet::new();
    let mut news = |snapshot: &world_projection::ProjectionSnapshot| {
        let mut new = 0;
        for entry in snapshot.book.iter().filter(|entry| entry.found) {
            new += usize::from(found.insert((entry.shelf.clone(), entry.name.clone())));
        }
        (
            new,
            snapshot.keepsakes.len() + snapshot.letters.len() + snapshot.chapters.len(),
        )
    };
    let mut before = universe.projection_snapshot();
    news(&before);
    for day in 1..=DAYS {
        let state = universe.world().state();
        for plot in hands::open_plots(state, &crate::handwork::kit(state)) {
            played.plots_opened.entry(plot.id).or_insert(day);
        }
        let snapshot = before;
        let questions = snapshot
            .commands
            .iter()
            .filter(|command| {
                command.question.is_some()
                    && command.unavailable.is_none()
                    && command.id != NUDGE_COMMAND
            })
            .collect::<Vec<_>>();
        let answer = match player {
            Player::Warm | Player::Builder => questions.first().copied(),
            Player::Last => questions.first().and_then(|first| {
                questions
                    .iter()
                    .copied()
                    .rfind(|command| command.question == first.question)
            }),
            Player::Never | Player::Absent => None,
        };
        if let Some(answer) = answer {
            let _ = universe.invoke_projection_command(&answer.id.clone());
        }
        if player != Player::Absent && day % 3 == 0 {
            let snap = universe.projection_snapshot();
            let plot = (player == Player::Builder)
                .then(|| {
                    snap.canvas
                        .plots
                        .iter()
                        .flat_map(|plot| plot.offers.iter())
                        .find(|offer| offer.unavailable.is_none())
                        .map(|offer| offer.command.clone())
                })
                .flatten();
            let deed = plot.or_else(|| {
                let made = hands::ever_made(universe.world());
                let hands = snap
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
                hands
                    .iter()
                    .find(|command| !made.iter().any(|thing| command.id.contains(thing.as_str())))
                    .or(hands.first())
                    .map(|command| command.id.clone())
            });
            if let Some(deed) = deed {
                let _ = universe.invoke_projection_command(&deed);
            }
        }
        universe.invoke_projection_command(NUDGE_COMMAND).unwrap();
        let after = universe.projection_snapshot();
        let (found_new, kept) = news(&after);
        let kept_before =
            snapshot.keepsakes.len() + snapshot.letters.len() + snapshot.chapters.len();
        played.quiet.push(found_new == 0 && kept <= kept_before);
        played
            .letters
            .push(after.letters.len().saturating_sub(snapshot.letters.len()));
        before = after;
    }
    played.world = universe.world().clone();
    played
}

fn day_of(event: &world_core::Event) -> usize {
    (event.world_time / crate::BACKGROUND_PERIOD) as usize
}

fn works(world: &World) -> BTreeSet<String> {
    story::goals(world)
        .into_iter()
        .filter(|goal| goal.done >= goal.parts)
        .map(|goal| goal.id)
        .collect()
}

fn legend_share(world: &World) -> (usize, usize) {
    let (mut lines, mut caused) = (0, 0);
    let subjects = world
        .state()
        .entities()
        .filter(|entity| {
            matches!(entity.kind.as_str(), "person" | "penguin")
                || [crate::SLOT_A, crate::SLOT_C].contains(&entity.id)
        })
        .map(|entity| SelectionId::Entity(entity.id))
        .collect::<Vec<_>>();
    for subject in subjects {
        if let Some(legend) = crate::legends::legend(world, subject) {
            lines += legend.lines.len();
            caused += legend
                .lines
                .iter()
                .filter(|line| line.cause.is_some())
                .count();
        }
    }
    (lines, caused)
}

fn jaccard(a: &BTreeSet<String>, b: &BTreeSet<String>) -> f64 {
    let union = a.union(b).count() as f64;
    1.0 - a.intersection(b).count() as f64 / union.max(1.0)
}

/// Prints one player's numbers and returns what failed its bars.
fn report(played: &Played) -> Vec<String> {
    let world = &played.world;
    let p = format!("{} {:?}", played.place, played.player);
    let mut failed = Vec::new();
    let works = works(world);
    eprintln!("PLAYERS {p} works {} {works:?}", works.len());
    let per_year = |kinds: &[&str]| {
        let mut years = [0_usize; 3];
        for event in world.events_of_kind(kinds) {
            years[(day_of(event) / YEAR).min(2)] += 1;
        }
        years
    };
    let taken = world
        .events()
        .iter()
        .filter(|event| event.payload.get("taken_up") == Some(&world_core::Value::Bool(true)))
        .count();
    eprintln!(
        "PLAYERS {p} wants taken up by someone else {taken}; hands seen {}",
        world.events_of_kind(&["story_marked"]).len()
    );
    eprintln!(
        "PLAYERS {p} born/yr {:?} died/yr {:?} made by hand {} plots finished {}",
        per_year(&["born"]),
        per_year(&["died"]),
        world
            .events_of_kind(&["built_by_hand", "decorated_by_hand", "planted_by_hand"])
            .len(),
        world.events_of_kind(&["plot_finished"]).len()
    );
    let mut titles = BTreeMap::<String, usize>::new();
    for event in world.events_of_kind(&["situation_arose"]) {
        if let Some(told) = story::told(world, event) {
            *titles.entry(told).or_default() += 1;
        }
    }
    let mut top = titles.iter().collect::<Vec<_>>();
    top.sort_by_key(|(title, n)| (std::cmp::Reverse(**n), (*title).clone()));
    let over = titles.values().filter(|n| **n > 3).count();
    eprintln!(
        "PLAYERS {p} situations {} distinct titles {} titles>3 {over} top {:?}",
        titles.values().sum::<usize>(),
        titles.len(),
        top.iter().take(4).collect::<Vec<_>>()
    );
    if over > 0 {
        failed.push(format!("{p}: {over} titles told more than 3 times"));
    }
    let (lines, caused) = legend_share(world);
    let share = caused as f64 / lines.max(1) as f64;
    eprintln!(
        "PLAYERS {p} legend lines {lines} caused {caused} ({:.0}%)",
        share * 100.0
    );
    if matches!(played.player, Player::Last | Player::Never) && share < 0.30 {
        failed.push(format!("{p}: legend cause share {:.0}%", share * 100.0));
    }
    let mut plots = [0_usize; 3];
    for day in played.plots_opened.values() {
        plots[((day - 1) / YEAR).min(2)] += 1;
    }
    eprintln!("PLAYERS {p} plots opened per year {plots:?}");
    if plots.contains(&0) {
        failed.push(format!("{p}: plots opened per year {plots:?}"));
    }
    let letters = played.letters.iter().sum::<usize>();
    let a_week = letters as f64 * 7.0 / DAYS as f64;
    let quiet = played.quiet.iter().filter(|quiet| **quiet).count();
    let most_quiet = played
        .quiet
        .windows(120)
        .map(|stretch| stretch.iter().filter(|quiet| **quiet).count())
        .max()
        .unwrap_or(0);
    eprintln!(
        "PLAYERS {p} letters {letters} ({a_week:.2} a week); quiet days {quiet} (most {most_quiet} in 120)"
    );
    if a_week > 1.5 {
        failed.push(format!("{p}: {a_week:.2} letters a week"));
    }
    if quiet == 0 || most_quiet > 4 {
        failed.push(format!("{p}: {quiet} quiet days, most {most_quiet} in 120"));
    }
    eprintln!(
        "PLAYERS {p} moments {} events {}",
        crate::moments::moments(world).len(),
        world.events().len()
    );
    failed
}

#[test]
#[ignore]
fn five_players_three_years_in_every_place() {
    let only = std::env::var("PLAYERS").unwrap_or_default();
    let places = std::env::var("PLACES").unwrap_or_default();
    let runs = PLACES
        .into_iter()
        .filter(|(place, _)| places.is_empty() || places.contains(place))
        .flat_map(|(place, seed)| {
            PLAYERS
                .into_iter()
                .filter(|player| only.is_empty() || only.contains(&format!("{player:?}")))
                .map(move |player| (place, seed, player))
        })
        .collect::<Vec<_>>();
    // Five at a time, so a small machine is not swamped.
    let mut played = Vec::new();
    for batch in runs.chunks(5) {
        played.extend(std::thread::scope(|scope| {
            batch
                .iter()
                .map(|(place, seed, player)| {
                    std::thread::Builder::new()
                        .stack_size(64 << 20)
                        .spawn_scoped(scope, move || play(place, seed, *player))
                        .unwrap()
                })
                .collect::<Vec<_>>()
                .into_iter()
                .map(|handle| handle.join().unwrap())
                .collect::<Vec<_>>()
        }));
    }
    let mut failed = played.iter().flat_map(report).collect::<Vec<_>>();
    for (place, _) in PLACES {
        let of = |player| {
            played
                .iter()
                .find(|played| played.place == place && played.player == player)
                .map(|played| works(&played.world))
        };
        let ways = PLAYERS
            .into_iter()
            .filter_map(|player| Some((player, of(player)?)))
            .collect::<Vec<_>>();
        for (at, (a, of_a)) in ways.iter().enumerate() {
            for (b, of_b) in ways.iter().skip(at + 1) {
                eprintln!(
                    "PLAYERS {place} works Jaccard distance {a:?}-{b:?} {:.2}",
                    jaccard(of_a, of_b)
                );
            }
        }
        if let (Some(never), Some(absent)) = (of(Player::Never), of(Player::Absent)) {
            let distance = jaccard(&never, &absent);
            if distance < 0.5 {
                failed.push(format!(
                    "{place}: Never and Absent works at distance {distance:.2}"
                ));
            }
        }
    }
    assert!(failed.is_empty(), "{failed:#?}");
}
