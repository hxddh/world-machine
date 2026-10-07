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
//! The players and bars are `world_pack_testkit::players`'.
//! `cargo test -p pocket-universe --release --lib players -- --ignored --nocapture`;
//! `PLAYERS=Never,Absent` plays only those, `PLACES=mars` only that place.

use crate::{story, PocketUniverse, NUDGE_COMMAND};
use std::collections::BTreeSet;
use world_core::{Event, World};
use world_pack_testkit::players::{self, Played, Player, PlayerFacts, PlayerWorld, DAYS};
use world_projection::{ProjectionCommand, ProjectionSnapshot, SelectionId};

pub(crate) const PLACES: [(&str, &str); 3] = [
    ("mars", crate::SEED_MARS_COLONY_COMMAND),
    ("maple", crate::SEED_1980S_TOWN_COMMAND),
    ("ice", crate::SEED_PENGUIN_CIVILIZATION_COMMAND),
];

impl PlayerWorld for PocketUniverse {
    fn world(&self) -> &World {
        PocketUniverse::world(self)
    }
    fn snapshot(&self) -> ProjectionSnapshot {
        self.projection_snapshot()
    }
    fn invoke(&mut self, command: &str) {
        let _ = self.invoke_projection_command(command);
    }
    fn pass_command(&self) -> &str {
        NUDGE_COMMAND
    }
    fn open_plots(&self) -> Vec<String> {
        let state = PocketUniverse::world(self).state();
        hands::open_plots(state, &crate::handwork::kit(state))
            .into_iter()
            .map(|plot| plot.id)
            .collect()
    }
    /// Something not made yet, else the first.
    fn deed(&self, _: &ProjectionSnapshot, hands: &[&ProjectionCommand]) -> Option<String> {
        let made = hands::ever_made(PocketUniverse::world(self));
        hands
            .iter()
            .find(|command| !made.iter().any(|thing| command.id.contains(thing.as_str())))
            .or(hands.first())
            .map(|command| command.id.clone())
    }
    fn returned(&self, since: usize) -> Option<world_projection::BriefingProjection> {
        self.projection_snapshot_since(Some(since)).briefing
    }
}

/// Each place, as the players' report reads it.
struct Here;

impl PlayerFacts for Here {
    fn day_length(&self) -> u64 {
        crate::BACKGROUND_PERIOD
    }
    fn works(&self, world: &World) -> BTreeSet<String> {
        story::goals(world)
            .into_iter()
            .filter(|goal| goal.done >= goal.parts)
            .map(|goal| goal.id)
            .collect()
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
}

fn play(place: &'static str, seed: &str, player: Player) -> Played {
    let mut universe = PocketUniverse::new().unwrap();
    universe.invoke_projection_command(seed).unwrap();
    players::play(format!("{place} {player:?}"), player, DAYS, universe)
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

#[test]
#[ignore]
fn five_players_three_years_in_every_place() {
    let places = std::env::var("PLACES").unwrap_or_default();
    let places = PLACES
        .into_iter()
        .filter(|(place, _)| places.is_empty() || places.contains(place))
        .collect::<Vec<_>>();
    let runs = places
        .iter()
        .flat_map(|(place, seed)| {
            players::players_asked()
                .into_iter()
                .map(move |player| (*place, *seed, player))
        })
        .collect::<Vec<_>>();
    let played = players::play_all(&runs, |(place, seed, player)| play(place, seed, *player));
    let mut failed = Vec::new();
    for played in &played {
        let numbers = players::report(&Here, played);
        failed.extend(players::failed(played, &numbers));
        // Somebody grows up on Ares in three years, whoever plays (none
        // did before v0.27: see `years::CADET_AGE`).
        let grown = played.world.events_of_kind(&["came_of_age"]).len();
        if played.label.starts_with("mars ") && grown == 0 {
            failed.push(format!("{}: nobody came of age", played.label));
        }
    }
    for (place, _) in places {
        let of_place = played
            .iter()
            .filter(|played| played.label.starts_with(&format!("{place} ")))
            .collect::<Vec<_>>();
        failed.extend(players::works_apart(&Here, place, &of_place));
    }
    assert!(failed.is_empty(), "{failed:#?}");
}
