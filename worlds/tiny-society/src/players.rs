//! Five players, three years each, side by side: the bars for "a story
//! for every player".
//!
//! - **Warm:** the first answer every time, and something made by hand
//!   every third day (`long_run`'s player).
//! - **Builder:** the same, but building on a plot every third day when
//!   one is on offer.
//! - **Last:** the last answer to the first question on offer.
//! - **Never:** never answers, and makes something every third day.
//! - **Absent:** does nothing at all.
//!
//! The bars, for every player over 1,080 days:
//! - no storylet told under the same title more than three times;
//! - a legend cause share of at least 30% for Last and Never;
//! - Never's and Absent's works at a Jaccard distance of at least 0.5;
//! - plots opening in each of the three years;
//! - letters averaging at most 1.5 a week, with some quiet days (and at
//!   most four in any 120, as `long_run` asks);
//! - the Warm player's v0.23 numbers kept: 64 works, four births and two
//!   deaths.
//!
//! It takes a few minutes in release:
//! `cargo test -p tiny-society --release --lib players -- --ignored --nocapture`.
//! `PLAYERS=Never,Absent` plays only those; `PLAYERS_START=prologue`
//! starts the old way, with the storm prologue, as v0.23 was measured.

use crate::{story, TinySociety};
use std::collections::{BTreeMap, BTreeSet};
use world_core::{Value, World};
use world_projection::SelectionId;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Player {
    Warm,
    Builder,
    Last,
    Never,
    Absent,
}

pub(crate) const PLAYERS: [Player; 5] = [
    Player::Warm,
    Player::Builder,
    Player::Last,
    Player::Never,
    Player::Absent,
];

const DAYS: usize = 1_080;
const YEAR: usize = 360;

/// What a player's three years brought.
pub(crate) struct Played {
    pub(crate) player: Player,
    /// Letters written each day.
    letters: Vec<usize>,
    /// Days that brought nothing new: nothing found in the book, nothing
    /// to keep, no letter and no chapter's close.
    quiet: Vec<bool>,
    /// The day each plot was first open to build on.
    plots_opened: BTreeMap<String, usize>,
    pub(crate) world: World,
}

fn play(player: Player) -> Played {
    play_for(player, DAYS)
}

fn play_for(player: Player, days: usize) -> Played {
    // As a new player finds it; `PLAYERS_START=prologue` starts the old
    // way (the storm prologue first), as v0.23's numbers were measured.
    let mut branch = if std::env::var("PLAYERS_START").as_deref() == Ok("prologue") {
        let mut society = TinySociety::new().unwrap();
        society.run_story().unwrap();
        let mut branch = society.branch();
        branch.begin_story().unwrap();
        branch
    } else {
        crate::TinySocietyBranch::new_world().unwrap()
    };
    let mut played = Played {
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
    let mut before = branch.projection_snapshot();
    news(&before);
    for day in 1..=days {
        let state = branch.world().state();
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
                    && command.id != story::WAIT_COMMAND
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
            let _ = branch.invoke_projection_command(&answer.id.clone());
        }
        if player != Player::Absent && day % 3 == 0 {
            let snap = branch.projection_snapshot();
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
                let unmade = snap
                    .book
                    .iter()
                    .filter(|entry| entry.shelf == "Made" && !entry.found)
                    .map(|entry| entry.name.clone())
                    .collect::<BTreeSet<_>>();
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
                    .find(|command| {
                        command
                            .hand
                            .as_ref()
                            .is_some_and(|hand| unmade.contains(&hand.thing))
                    })
                    .or(hands.first())
                    .map(|command| command.id.clone())
            });
            if let Some(deed) = deed {
                let _ = branch.invoke_projection_command(&deed);
            }
        }
        branch
            .invoke_projection_command(story::WAIT_COMMAND)
            .unwrap();
        let after = branch.projection_snapshot();
        let (found_new, kept) = news(&after);
        let kept_before =
            snapshot.keepsakes.len() + snapshot.letters.len() + snapshot.chapters.len();
        played.quiet.push(found_new == 0 && kept <= kept_before);
        played
            .letters
            .push(after.letters.len().saturating_sub(snapshot.letters.len()));
        before = after;
    }
    played.world = branch.world().clone();
    played
}

pub(crate) fn play_all(players: &[Player]) -> Vec<Played> {
    std::thread::scope(|scope| {
        let handles = players
            .iter()
            .map(|player| {
                let player = *player;
                std::thread::Builder::new()
                    .stack_size(64 << 20)
                    .spawn_scoped(scope, move || play(player))
                    .unwrap()
            })
            .collect::<Vec<_>>();
        handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect()
    })
}

fn day_of(event: &world_core::Event) -> usize {
    (event.world_time / crate::persistence::WORLD_DAY_TICKS) as usize
}

/// The works the harbour finished.
pub(crate) fn works(world: &World) -> BTreeSet<String> {
    story::goals(world)
        .into_iter()
        .filter(|goal| goal.done >= goal.parts)
        .map(|goal| goal.id)
        .collect()
}

/// How many legend lines there are, and how many have a cause, over every
/// resident's and place's legend.
pub(crate) fn legend_share(world: &World) -> (usize, usize) {
    let (mut lines, mut caused) = (0, 0);
    let mut kinds = BTreeMap::<String, (usize, usize)>::new();
    let subjects = world
        .state()
        .entities()
        .filter(|entity| matches!(entity.kind.as_str(), "resident" | "location"))
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
            for line in &legend.lines {
                let kind = line
                    .event
                    .and_then(|id| world.event(id))
                    .map_or_else(String::new, |event| event.kind.clone());
                let counts = kinds.entry(kind).or_default();
                counts.0 += 1;
                counts.1 += usize::from(line.cause.is_some());
            }
        }
    }
    let mut kinds = kinds.into_iter().collect::<Vec<_>>();
    kinds.sort_by_key(|(_, (n, _))| std::cmp::Reverse(*n));
    eprintln!(
        "PLAYERS legend lines by kind (lines, caused): {:?}",
        kinds.iter().take(14).collect::<Vec<_>>()
    );
    (lines, caused)
}

/// How often each storylet was told under each title.
pub(crate) fn titles(world: &World) -> BTreeMap<String, usize> {
    let mut titles = BTreeMap::<String, usize>::new();
    for event in world.events_of_kind(&["situation_arose"]) {
        if let Some(told) = story::told(world, event) {
            *titles.entry(told).or_default() += 1;
        }
    }
    titles
}

/// New kinds of thing in each 30 days, as `three_years_of_lives` counts
/// them.
fn fresh_per_30(world: &World) -> Vec<usize> {
    let mut seen = BTreeSet::new();
    let mut fresh = vec![0_usize; DAYS / 30];
    for event in world.events() {
        let mut keys = vec![event.kind.clone()];
        for key in ["storylet", "festival", "beat", "bond", "activity"] {
            if let Some(Value::Text(text)) = event.payload.get(key) {
                keys.push(format!("{key}:{text}"));
            }
        }
        if event.kind == "situation_arose" || lives::is_news(event) {
            if let Some(told) = story::told(world, event) {
                keys.push(format!("told:{told}"));
            }
        }
        for key in keys {
            if seen.insert(key) {
                fresh[day_of(event).min(DAYS - 1) / 30] += 1;
            }
        }
    }
    fresh
}

pub(crate) fn jaccard(a: &BTreeSet<String>, b: &BTreeSet<String>) -> f64 {
    let union = a.union(b).count() as f64;
    1.0 - a.intersection(b).count() as f64 / union.max(1.0)
}

/// The numbers the bars are checked against, for one player.
pub(crate) struct Numbers {
    pub(crate) most_told: usize,
    pub(crate) over_three: usize,
    pub(crate) caused_share: f64,
    pub(crate) plots_by_year: [usize; 3],
    pub(crate) letters_a_week: f64,
    pub(crate) quiet_days: usize,
    pub(crate) most_quiet_in_120: usize,
    pub(crate) works: usize,
    pub(crate) born: usize,
    pub(crate) died: usize,
}

pub(crate) fn report(played: &Played) -> Numbers {
    let world = &played.world;
    let p = format!("{:?}", played.player);
    let per_year = |kinds: &[&str]| {
        let mut years = [0_usize; 3];
        for event in world.events_of_kind(kinds) {
            years[(day_of(event) / YEAR).min(2)] += 1;
        }
        years
    };
    let works = works(world);
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
    let let_down = world
        .state()
        .entity(crate::story::STORY)
        .map_or(0, |story| {
            story
                .components
                .keys()
                .filter(|key| key.starts_with("story.letdown."))
                .count()
        });
    eprintln!(
        "PLAYERS {p} wants taken up by someone else {taken}; still let down {let_down}; hands seen {}; own works taken up {}",
        world.events_of_kind(&["story_marked"]).len(),
        world.events_of_kind(&["own_work_taken_up"]).len()
    );
    let (born, died) = (per_year(&["born"]), per_year(&["died"]));
    eprintln!(
        "PLAYERS {p} born/yr {born:?} died/yr {died:?} came of age/yr {:?} bonds/yr {:?}",
        per_year(&["came_of_age"]),
        per_year(&["bond_changed"])
    );
    let titles = titles(world);
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
    let (lines, caused) = legend_share(world);
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
    let fresh = fresh_per_30(world);
    eprintln!(
        "PLAYERS {p} new kinds per 30 days, year 3 {:?}",
        &fresh[24..36]
    );
    let moments = crate::moments::moments(world).len();
    eprintln!(
        "PLAYERS {p} moments {moments} events {}",
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

#[test]
#[ignore]
fn five_players_three_years() {
    let only = std::env::var("PLAYERS").unwrap_or_default();
    let players = PLAYERS
        .into_iter()
        .filter(|player| only.is_empty() || only.contains(&format!("{player:?}")))
        .collect::<Vec<_>>();
    let played = play_all(&players);
    let numbers = played.iter().map(report).collect::<Vec<_>>();
    let works = played
        .iter()
        .map(|played| (played.player, works(&played.world)))
        .collect::<BTreeMap<_, _>>();
    for (at, (a, of_a)) in works.iter().enumerate() {
        for (b, of_b) in works.iter().skip(at + 1) {
            eprintln!(
                "PLAYERS works Jaccard distance {a:?}-{b:?} {:.2}",
                jaccard(of_a, of_b)
            );
        }
    }
    let mut failed = Vec::new();
    for (played, numbers) in played.iter().zip(&numbers) {
        let p = played.player;
        if numbers.over_three > 0 {
            failed.push(format!(
                "{p:?}: {} titles told more than 3 times (most {})",
                numbers.over_three, numbers.most_told
            ));
        }
        if matches!(p, Player::Last | Player::Never) && numbers.caused_share < 0.30 {
            failed.push(format!(
                "{p:?}: legend cause share {:.0}%",
                numbers.caused_share * 100.0
            ));
        }
        if numbers.plots_by_year.contains(&0) {
            failed.push(format!(
                "{p:?}: plots opened per year {:?}",
                numbers.plots_by_year
            ));
        }
        if numbers.letters_a_week > 1.5 {
            failed.push(format!(
                "{p:?}: {:.2} letters a week",
                numbers.letters_a_week
            ));
        }
        if numbers.quiet_days == 0 || numbers.most_quiet_in_120 > 4 {
            failed.push(format!(
                "{p:?}: {} quiet days, most {} in 120",
                numbers.quiet_days, numbers.most_quiet_in_120
            ));
        }
        // v0.23's 64 works, 4 born and 2 died were measured from the old
        // start (the storm prologue first). A new player's harbour now
        // begins on Day 1 without it (D's v0.24 start) and lives slightly
        // differently: there the bar is the births and deaths, and works
        // within one of v0.23's.
        let prologue = std::env::var("PLAYERS_START").as_deref() == Ok("prologue");
        let works_bar = if prologue { 64 } else { 63 };
        if p == Player::Warm && (numbers.works < works_bar || numbers.born < 4 || numbers.died < 2)
        {
            failed.push(format!(
                "Warm: {} works, {} born, {} died (v0.23: 64, 4, 2)",
                numbers.works, numbers.born, numbers.died
            ));
        }
    }
    if let (Some(never), Some(absent)) = (works.get(&Player::Never), works.get(&Player::Absent)) {
        let distance = jaccard(never, absent);
        if distance < 0.5 {
            failed.push(format!("Never and Absent works at distance {distance:.2}"));
        }
    }
    assert!(failed.is_empty(), "{failed:#?}");
}

/// A player who never answers, for four months: a want let down never
/// comes up again as it was; someone else takes it up, caused by its being
/// let down, and that is told in both their legends with its cause; and
/// what the player makes sets which of its own works the harbour takes up.
#[test]
fn a_want_let_down_is_taken_up_and_never_asked_again_as_it_was() {
    let played = play_for(Player::Never, 120);
    let world = &played.world;
    let deck = story::deck();
    let wants = deck
        .storylets
        .iter()
        .filter(|storylet| storylet.want)
        .map(|storylet| storylet.id)
        .collect::<BTreeSet<_>>();
    // Each want's story, in order: asked, let down, taken up.
    let mut let_down = BTreeSet::new();
    let mut taken = Vec::new();
    for event in world.events() {
        let Some(Value::Text(id)) = event.payload.get("storylet") else {
            continue;
        };
        if !wants.contains(id.as_str()) {
            continue;
        }
        if event.kind == "situation_arose" {
            assert!(
                !let_down.contains(id),
                "{id} came up again as it was on day {}",
                day_of(event)
            );
        } else if event.payload.get("taken_up") == Some(&Value::Bool(true)) {
            assert!(let_down.remove(id), "{id} taken up without being let down");
            taken.push(event.clone());
        } else if event.payload.get("lapsed") == Some(&Value::Bool(true))
            || story::answer_refuses(event)
        {
            let_down.insert(id.clone());
        }
    }
    assert!(!taken.is_empty(), "nobody took anything up");
    let first = &taken[0];
    let cause = world.event(first.caused_by[0]).unwrap();
    assert_eq!(cause.payload.get("lapsed"), Some(&Value::Bool(true)));
    let told = story::told(world, first).unwrap();
    assert!(told.contains(" took it on, and "), "{told}");
    assert!(!told.contains('{'), "{told}");
    let helper = first.actor.unwrap();
    let legend = crate::legends::legend(world, SelectionId::Entity(helper)).unwrap();
    let line = legend
        .lines
        .iter()
        .find(|line| line.event == Some(first.id))
        .unwrap_or_else(|| panic!("{told} in {legend:?}"));
    assert_eq!(line.cause, Some(cause.id));
    assert!(
        line.because
            .as_deref()
            .is_some_and(|because| because.ends_with(" was left waiting")),
        "{line:?}"
    );
    // What the player made set the harbour on its own works.
    let chosen = world.events_of_kind(&["own_work_taken_up"]);
    assert!(!chosen.is_empty());
    assert_eq!(world.replay().unwrap().state(), world.state());
    // Each of these new lines reads whole in Chinese.
    let mut catalog = world_i18n::Catalog::parse(include_str!(
        "../../../crates/world-builtins/locales/systems.zh-Hans.tsv"
    ));
    catalog.extend(crate::ZH_HANS);
    let mut lines = taken
        .iter()
        .filter_map(|event| story::told(world, event))
        .chain(
            world
                .events_of_kind(&["plot_cleared"])
                .into_iter()
                .filter_map(|event| story::told(world, event)),
        )
        .collect::<Vec<_>>();
    lines.push(line.because.clone().unwrap());
    lines.push("after a day spent together".into());
    lines.push("after you said “It can wait”".into());
    assert!(lines
        .iter()
        .any(|line| line.starts_with("A new plot was cleared by ")));
    for line in lines {
        let shown = catalog
            .translate(&line)
            .unwrap_or_else(|| panic!("no Chinese for {line:?}"));
        assert!(
            !["took it on", "after", "plot", "waiting"]
                .iter()
                .any(|english| shown.contains(english)),
            "{line:?} shown as {shown:?}"
        );
    }
}
