//! Five players, three years each: the bars for "a story for every player".
//!
//! - **Warm:** answers the first question each day, and makes something
//!   every third day.
//! - **Builder:** the same, but building on a plot every third day when
//!   one is on offer.
//! - **Last:** takes the last answer to each question.
//! - **Never:** never answers, and makes something every third day.
//! - **Absent:** never answers and never makes anything.
//!
//! The bars, for every player over 1,080 days:
//! - no storylet told under the same title more than three times;
//! - a legend cause share of at least 30% for Last and Never;
//! - Never's and Absent's works at a Jaccard distance of at least 0.5;
//! - plots opening in each of the three years;
//! - letters averaging at most 1.5 a week, with some quiet days (and at
//!   most four in any 120).
//!
//! A Pack gives its World to play ([`PlayerWorld`]) and what the report
//! reads of it ([`PlayerFacts`]); the players, the numbers and the bars
//! are the same in every World.

use std::collections::{BTreeMap, BTreeSet};
use world_core::{Event, Value, World};
use world_projection::{ProjectionCommand, ProjectionSnapshot};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Player {
    Warm,
    Builder,
    Last,
    Never,
    Absent,
}

pub const PLAYERS: [Player; 5] = [
    Player::Warm,
    Player::Builder,
    Player::Last,
    Player::Never,
    Player::Absent,
];

/// Three years, of 360 days.
pub const DAYS: usize = 1_080;
pub const YEAR: usize = 360;

/// A World as a player plays it.
pub trait PlayerWorld {
    fn world(&self) -> &World;
    fn snapshot(&self) -> ProjectionSnapshot;
    /// Invokes a command; whether it was refused does not matter here.
    fn invoke(&mut self, command: &str);
    /// The command that lets the day pass.
    fn pass_command(&self) -> &str;
    /// The ids of the plots open to build on now.
    fn open_plots(&self) -> Vec<String>;
    /// Of the deeds of the player's hands on offer (`hands`), the one to
    /// do: something not yet made, else the first.
    fn deed(&self, snapshot: &ProjectionSnapshot, hands: &[&ProjectionCommand]) -> Option<String>;
    /// The film a player would be shown returning now, having last seen
    /// the World when it had `since_events` Events; `None` if the World
    /// has no film.
    fn returned(&self, _since_events: usize) -> Option<world_projection::BriefingProjection> {
        None
    }
}

/// What a player's three years brought.
pub struct Played {
    /// Which run it was: the player, and the place when a Pack has several.
    pub label: String,
    pub player: Player,
    /// Letters written each day.
    pub letters: Vec<usize>,
    /// Days that brought nothing new: nothing found in the book, nothing
    /// to keep, no letter and no chapter's close.
    pub quiet: Vec<bool>,
    /// The day each plot was first open to build on.
    pub plots_opened: BTreeMap<String, usize>,
    pub world: World,
}

/// Plays `world` for `days` days as `player`.
pub fn play(label: String, player: Player, days: usize, world: impl PlayerWorld) -> Played {
    play_watched(label, player, days, world, |_, _, _| {})
}

/// What a watcher is shown of each day a player plays: the day (from 1),
/// the World as the day began (before the player's answers), and the World
/// once the day has passed.
pub struct Day<'a> {
    pub day: usize,
    pub began: &'a ProjectionSnapshot,
    /// How many Events the World had as the day began.
    pub events_before: usize,
}

/// Plays as [`play`] does, showing `watch` each day as it passes: its
/// snapshot as it began, the World after, and the snapshot after.
pub fn play_watched<W: PlayerWorld>(
    label: String,
    player: Player,
    days: usize,
    mut world: W,
    mut watch: impl FnMut(Day<'_>, &W, &ProjectionSnapshot),
) -> Played {
    let mut played = Played {
        label,
        player,
        letters: Vec::new(),
        quiet: Vec::new(),
        plots_opened: BTreeMap::new(),
        world: World::new(Default::default()),
    };
    let mut found = BTreeSet::new();
    let mut news = |snapshot: &ProjectionSnapshot| {
        let mut new = 0;
        for entry in snapshot.book.iter().filter(|entry| entry.found) {
            new += usize::from(found.insert((entry.shelf.clone(), entry.name.clone())));
        }
        (
            new,
            snapshot.keepsakes.len() + snapshot.letters.len() + snapshot.chapters.len(),
        )
    };
    let mut before = world.snapshot();
    news(&before);
    for day in 1..=days {
        for plot in world.open_plots() {
            played.plots_opened.entry(plot).or_insert(day);
        }
        let snapshot = before;
        let events_before = world.world().events().len();
        let pass = world.pass_command().to_string();
        let questions = snapshot
            .commands
            .iter()
            .filter(|command| {
                command.question.is_some() && command.unavailable.is_none() && command.id != pass
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
            world.invoke(&answer.id.clone());
        }
        if player != Player::Absent && day % 3 == 0 {
            let snap = world.snapshot();
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
                world.deed(&snap, &hands)
            });
            if let Some(deed) = deed {
                world.invoke(&deed);
            }
        }
        world.invoke(&pass);
        let after = world.snapshot();
        watch(
            Day {
                day,
                began: &snapshot,
                events_before,
            },
            &world,
            &after,
        );
        let (found_new, kept) = news(&after);
        let kept_before =
            snapshot.keepsakes.len() + snapshot.letters.len() + snapshot.chapters.len();
        played.quiet.push(found_new == 0 && kept <= kept_before);
        played
            .letters
            .push(after.letters.len().saturating_sub(snapshot.letters.len()));
        before = after;
    }
    played.world = world.world().clone();
    played
}

/// Plays every run, five at a time on threads of their own, so a small
/// machine is not swamped.
pub fn play_all<R: Sync>(runs: &[R], play: impl Fn(&R) -> Played + Sync) -> Vec<Played> {
    let mut played = Vec::new();
    for batch in runs.chunks(5) {
        played.extend(std::thread::scope(|scope| {
            batch
                .iter()
                .map(|run| {
                    let play = &play;
                    std::thread::Builder::new()
                        .stack_size(64 << 20)
                        .spawn_scoped(scope, move || play(run))
                        .unwrap()
                })
                .collect::<Vec<_>>()
                .into_iter()
                .map(|handle| handle.join().unwrap())
                .collect::<Vec<_>>()
        }));
    }
    played
}

/// The players `PLAYERS=Never,Absent` asks for, or all five.
pub fn players_asked() -> Vec<Player> {
    let only = std::env::var("PLAYERS").unwrap_or_default();
    PLAYERS
        .into_iter()
        .filter(|player| only.is_empty() || only.contains(&format!("{player:?}")))
        .collect()
}

/// What the report reads of a Pack's World.
pub trait PlayerFacts {
    /// World time in a day.
    fn day_length(&self) -> u64;
    /// The works the World finished.
    fn works(&self, world: &World) -> BTreeSet<String>;
    /// How many legend lines there are, and how many have a cause.
    fn legend_share(&self, world: &World) -> (usize, usize);
    /// How the World tells an Event.
    fn told(&self, world: &World, event: &Event) -> Option<String>;
    fn moments(&self, world: &World) -> usize;
    /// Anything else the Pack prints of a player's World.
    fn more(&self, _world: &World) -> Vec<String> {
        Vec::new()
    }
}

/// The numbers the bars are checked against, for one player.
pub struct Numbers {
    pub most_told: usize,
    pub over_three: usize,
    pub caused_share: f64,
    pub plots_by_year: [usize; 3],
    pub letters_a_week: f64,
    pub quiet_days: usize,
    pub most_quiet_in_120: usize,
    pub works: usize,
    pub born: usize,
    pub died: usize,
}

fn day_of(facts: &impl PlayerFacts, event: &Event) -> usize {
    (event.world_time / facts.day_length()) as usize
}

/// How often each storylet was told under each title.
pub fn titles(facts: &impl PlayerFacts, world: &World) -> BTreeMap<String, usize> {
    let mut titles = BTreeMap::<String, usize>::new();
    for event in world.events_of_kind(&["situation_arose"]) {
        if let Some(told) = facts.told(world, event) {
            *titles.entry(told).or_default() += 1;
        }
    }
    titles
}

/// New kinds of thing in each 30 days.
pub fn fresh_per_30(
    facts: &impl PlayerFacts,
    world: &World,
    is_news: impl Fn(&Event) -> bool,
) -> Vec<usize> {
    let mut seen = BTreeSet::new();
    let mut fresh = vec![0_usize; DAYS / 30];
    for event in world.events() {
        let mut keys = vec![event.kind.clone()];
        for key in ["storylet", "festival", "beat", "bond", "activity"] {
            if let Some(Value::Text(text)) = event.payload.get(key) {
                keys.push(format!("{key}:{text}"));
            }
        }
        if event.kind == "situation_arose" || is_news(event) {
            if let Some(told) = facts.told(world, event) {
                keys.push(format!("told:{told}"));
            }
        }
        for key in keys {
            if seen.insert(key) {
                fresh[day_of(facts, event).min(DAYS - 1) / 30] += 1;
            }
        }
    }
    fresh
}

pub fn jaccard(a: &BTreeSet<String>, b: &BTreeSet<String>) -> f64 {
    let union = a.union(b).count() as f64;
    1.0 - a.intersection(b).count() as f64 / union.max(1.0)
}

/// Prints one player's numbers.
pub fn report(facts: &impl PlayerFacts, played: &Played) -> Numbers {
    let world = &played.world;
    let p = &played.label;
    let per_year = |kinds: &[&str]| {
        let mut years = [0_usize; 3];
        for event in world.events_of_kind(kinds) {
            years[(day_of(facts, event) / YEAR).min(2)] += 1;
        }
        years
    };
    let works = facts.works(world);
    eprintln!("PLAYERS {p} works {} {works:?}", works.len());
    let made = world
        .events_of_kind(&["built_by_hand", "decorated_by_hand", "planted_by_hand"])
        .len();
    let plots_built = world.events_of_kind(&["plot_finished"]).len();
    eprintln!("PLAYERS {p} made by hand {made}, plots finished {plots_built}");
    let taken = world
        .events()
        .iter()
        .filter(|event| event.payload.get("taken_up") == Some(&Value::Bool(true)))
        .count();
    eprintln!(
        "PLAYERS {p} wants taken up by someone else {taken}; hands seen {}; own works taken up {}",
        world.events_of_kind(&["story_marked"]).len(),
        world.events_of_kind(&["own_work_taken_up"]).len()
    );
    for line in facts.more(world) {
        eprintln!("PLAYERS {p} {line}");
    }
    let (born, died) = (per_year(&["born"]), per_year(&["died"]));
    eprintln!(
        "PLAYERS {p} born/yr {born:?} died/yr {died:?} came of age/yr {:?} bonds/yr {:?}",
        per_year(&["came_of_age"]),
        per_year(&["bond_changed"])
    );
    let titles = titles(facts, world);
    let mut top = titles.iter().collect::<Vec<_>>();
    top.sort_by_key(|(title, n)| (std::cmp::Reverse(**n), (*title).clone()));
    let over_three = titles.values().filter(|n| **n > 3).count();
    let most_told = top.first().map_or(0, |(_, n)| **n);
    eprintln!(
        "PLAYERS {p} situations {} distinct titles {} titles>3 {over_three} top {:?}",
        titles.values().sum::<usize>(),
        titles.len(),
        top.iter().take(5).collect::<Vec<_>>()
    );
    let (lines, caused) = facts.legend_share(world);
    let caused_share = caused as f64 / lines.max(1) as f64;
    eprintln!(
        "PLAYERS {p} legend lines {lines} caused {caused} ({:.0}%)",
        caused_share * 100.0
    );
    let mut plots_by_year = [0_usize; 3];
    for day in played.plots_opened.values() {
        plots_by_year[((day - 1) / YEAR).min(2)] += 1;
    }
    eprintln!("PLAYERS {p} plots opened per year {plots_by_year:?}");
    let letters = played.letters.iter().sum::<usize>();
    let letters_a_week = letters as f64 * 7.0 / DAYS as f64;
    let most_week = played
        .letters
        .windows(7)
        .map(|week| week.iter().sum::<usize>())
        .max()
        .unwrap_or(0);
    let weeks = played
        .letters
        .chunks(7)
        .map(|week| week.iter().sum::<usize>())
        .fold([0_usize; 3], |mut counts, n| {
            counts[n.min(2)] += 1;
            counts
        });
    let quiet_days = played.quiet.iter().filter(|quiet| **quiet).count();
    let most_quiet_in_120 = played
        .quiet
        .windows(120)
        .map(|stretch| stretch.iter().filter(|quiet| **quiet).count())
        .max()
        .unwrap_or(0);
    eprintln!(
        "PLAYERS {p} letters {letters} ({letters_a_week:.2} a week, most {most_week} in 7 days; weeks with 0/1/2+ {weeks:?}); quiet days {quiet_days} (most {most_quiet_in_120} in 120)"
    );
    let fresh = fresh_per_30(facts, world, lives::is_news);
    eprintln!(
        "PLAYERS {p} new kinds per 30 days, year 3 {:?}",
        &fresh[24..36]
    );
    eprintln!(
        "PLAYERS {p} moments {} events {}",
        facts.moments(world),
        world.events().len()
    );
    Numbers {
        most_told,
        over_three,
        caused_share,
        plots_by_year,
        letters_a_week,
        quiet_days,
        most_quiet_in_120,
        works: works.len(),
        born: born.iter().sum(),
        died: died.iter().sum(),
    }
}

/// What failed the bars every player is held to.
pub fn failed(played: &Played, numbers: &Numbers) -> Vec<String> {
    let p = &played.label;
    let mut failed = Vec::new();
    if numbers.over_three > 0 {
        failed.push(format!(
            "{p}: {} titles told more than 3 times (most {})",
            numbers.over_three, numbers.most_told
        ));
    }
    if matches!(played.player, Player::Last | Player::Never) && numbers.caused_share < 0.30 {
        failed.push(format!(
            "{p}: legend cause share {:.0}%",
            numbers.caused_share * 100.0
        ));
    }
    if numbers.plots_by_year.contains(&0) {
        failed.push(format!(
            "{p}: plots opened per year {:?}",
            numbers.plots_by_year
        ));
    }
    if numbers.letters_a_week > 1.5 {
        failed.push(format!("{p}: {:.2} letters a week", numbers.letters_a_week));
    }
    if numbers.quiet_days == 0 || numbers.most_quiet_in_120 > 4 {
        failed.push(format!(
            "{p}: {} quiet days, most {} in 120",
            numbers.quiet_days, numbers.most_quiet_in_120
        ));
    }
    failed
}

/// Prints how far apart the works of each pair of players are, and fails
/// Never and Absent closer than 0.5. `played` is one place's players.
pub fn works_apart(facts: &impl PlayerFacts, place: &str, played: &[&Played]) -> Vec<String> {
    let works = played
        .iter()
        .map(|played| (played.player, facts.works(&played.world)))
        .collect::<BTreeMap<_, _>>();
    let at = if place.is_empty() {
        String::new()
    } else {
        format!("{place} ")
    };
    for (index, (a, of_a)) in works.iter().enumerate() {
        for (b, of_b) in works.iter().skip(index + 1) {
            eprintln!(
                "PLAYERS {at}works Jaccard distance {a:?}-{b:?} {:.2}",
                jaccard(of_a, of_b)
            );
        }
    }
    let mut failed = Vec::new();
    if let (Some(never), Some(absent)) = (works.get(&Player::Never), works.get(&Player::Absent)) {
        let distance = jaccard(never, absent);
        if distance < 0.5 {
            failed.push(format!(
                "{at}Never and Absent works at distance {distance:.2}"
            ));
        }
    }
    failed
}
