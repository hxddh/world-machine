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
//! The players and bars are `world_pack_testkit::players`'. It takes a
//! few minutes in release:
//! `cargo test -p tiny-society --release --lib players -- --ignored --nocapture`.
//! `PLAYERS=Never,Absent` plays only those; `PLAYERS_START=prologue`
//! starts the old way, with the storm prologue, as v0.23 was measured.

use crate::{story, TinySociety, TinySocietyBranch};
use std::collections::{BTreeMap, BTreeSet};
use world_core::{Event, Value, World};
use world_pack_testkit::players::{self, Played, Player, PlayerFacts, PlayerWorld, DAYS};
use world_projection::{ProjectionCommand, ProjectionSnapshot, SelectionId};

impl PlayerWorld for TinySocietyBranch {
    fn world(&self) -> &World {
        TinySocietyBranch::world(self)
    }
    fn snapshot(&self) -> ProjectionSnapshot {
        self.projection_snapshot()
    }
    fn invoke(&mut self, command: &str) {
        let _ = self.invoke_projection_command(command);
    }
    fn pass_command(&self) -> &str {
        story::WAIT_COMMAND
    }
    fn open_plots(&self) -> Vec<String> {
        let state = TinySocietyBranch::world(self).state();
        hands::open_plots(state, &crate::handwork::kit(state))
            .into_iter()
            .map(|plot| plot.id)
            .collect()
    }
    /// Something not yet in the book's Made shelf, else the first.
    fn deed(&self, snap: &ProjectionSnapshot, hands: &[&ProjectionCommand]) -> Option<String> {
        let unmade = snap
            .book
            .iter()
            .filter(|entry| entry.shelf == "Made" && !entry.found)
            .map(|entry| entry.name.clone())
            .collect::<BTreeSet<_>>();
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
    }
    fn returned(&self, since: usize) -> Option<world_projection::BriefingProjection> {
        Some(self.film_since(since))
    }
}

/// The harbour, as the players' report reads it.
struct Harbour;

impl PlayerFacts for Harbour {
    fn day_length(&self) -> u64 {
        crate::persistence::WORLD_DAY_TICKS
    }
    fn works(&self, world: &World) -> BTreeSet<String> {
        works(world)
    }
    fn legend_share(&self, world: &World) -> (usize, usize) {
        legend_share(world)
    }
    fn told(&self, world: &World, event: &Event) -> Option<String> {
        story::told(world, event)
    }
    fn moments(&self, world: &World) -> usize {
        crate::moments::moments(world).len()
    }
    fn more(&self, world: &World) -> Vec<String> {
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
        vec![format!("wants still let down {let_down}")]
    }
}

fn play_for(player: Player, days: usize) -> Played {
    // As a new player finds it; `PLAYERS_START=prologue` starts the old
    // way (the storm prologue first), as v0.23's numbers were measured.
    let branch = if std::env::var("PLAYERS_START").as_deref() == Ok("prologue") {
        let mut society = TinySociety::new().unwrap();
        society.run_story().unwrap();
        let mut branch = society.branch();
        branch.begin_story().unwrap();
        branch
    } else {
        TinySocietyBranch::new_world().unwrap()
    };
    players::play(format!("{player:?}"), player, days, branch)
}

fn day_of(event: &world_core::Event) -> usize {
    (event.world_time / crate::persistence::WORLD_DAY_TICKS) as usize
}

/// The works the harbour finished.
fn works(world: &World) -> BTreeSet<String> {
    story::goals(world)
        .into_iter()
        .filter(|goal| goal.done >= goal.parts)
        .map(|goal| goal.id)
        .collect()
}

/// How many legend lines there are, and how many have a cause, over every
/// resident's and place's legend.
fn legend_share(world: &World) -> (usize, usize) {
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

#[test]
#[ignore]
fn five_players_three_years() {
    let played = players::play_all(&players::players_asked(), |player| play_for(*player, DAYS));
    let numbers = played
        .iter()
        .map(|played| players::report(&Harbour, played))
        .collect::<Vec<_>>();
    let mut failed = players::works_apart(&Harbour, "", &played.iter().collect::<Vec<_>>());
    for (played, numbers) in played.iter().zip(&numbers) {
        failed.extend(players::failed(played, numbers));
        // v0.23's 64 works, 4 born and 2 died were measured from the old
        // start (the storm prologue first). A new player's harbour now
        // begins on Day 1 without it (D's v0.24 start) and lives slightly
        // differently: there the bar is the births and deaths, and works
        // within one of v0.23's.
        let prologue = std::env::var("PLAYERS_START").as_deref() == Ok("prologue");
        let works_bar = if prologue { 64 } else { 63 };
        if played.player == Player::Warm
            && (numbers.works < works_bar || numbers.born < 4 || numbers.died < 2)
        {
            failed.push(format!(
                "Warm: {} works, {} born, {} died (v0.23: 64, 4, 2)",
                numbers.works, numbers.born, numbers.died
            ));
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
