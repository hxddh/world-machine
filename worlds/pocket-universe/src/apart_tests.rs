//! Each place its own (v0.29): the cards a place asks in its first month
//! are mostly its own, and trust is earned over weeks, not in four turns.
//!
//! - At most a fifth of the storylets a place's first month asks about
//!   (by storylet, whatever its words) are asked in another place's first
//!   month too, for a warm player and for a builder.
//! - Trust stays under 10 of 10 through day 7, for the same two players
//!   and for one who answers every card each day.

use crate::players::PLACES;
use crate::{PocketUniverse, NUDGE_COMMAND, RELATIONSHIP, RELATIONSHIP_TRUST};
use std::collections::BTreeSet;
use world_core::Value;
use world_pack_testkit::players::{self, Player};

/// The storylet a card asks about, from its command.
fn storylet_of(command: &str) -> Option<String> {
    let (storylet, _) = crate::story::parse_command(command)?;
    Some(storylet.to_string())
}

fn trust(universe: &PocketUniverse) -> i64 {
    match universe
        .world()
        .state()
        .entity(RELATIONSHIP)
        .and_then(|entity| entity.component(RELATIONSHIP_TRUST))
    {
        Some(Value::Integer(trust)) => *trust,
        _ => 0,
    }
}

/// What a player is asked over `days` in a place: every storylet a card
/// on offer asked about, and trust at the end of each day.
fn asked(seed: &str, player: Player, days: usize) -> (BTreeSet<String>, Vec<i64>) {
    let mut universe = PocketUniverse::new().unwrap();
    universe.invoke_projection_command(seed).unwrap();
    let mut cards = BTreeSet::new();
    let mut trusts = Vec::new();
    players::play_watched(
        String::new(),
        player,
        days,
        universe,
        |day, world, _after| {
            cards.extend(
                day.began
                    .commands
                    .iter()
                    .filter(|command| command.question.is_some() && command.id != NUDGE_COMMAND)
                    .filter_map(|command| storylet_of(&command.id)),
            );
            trusts.push(trust(world));
        },
    );
    (cards, trusts)
}

/// One who answers every card on offer, the first answer to each, every
/// day: the quickest way to trust there is.
fn eager(seed: &str, days: usize) -> Vec<i64> {
    let mut universe = PocketUniverse::new().unwrap();
    universe.invoke_projection_command(seed).unwrap();
    let mut trusts = Vec::new();
    for _ in 0..days {
        let mut asked = BTreeSet::new();
        for _ in 0..12 {
            let snapshot = universe.projection_snapshot();
            let Some(card) = snapshot.commands.iter().find(|command| {
                command.question.is_some()
                    && command.unavailable.is_none()
                    && command.id != NUDGE_COMMAND
                    && storylet_of(&command.id).is_some_and(|storylet| !asked.contains(&storylet))
            }) else {
                break;
            };
            asked.extend(storylet_of(&card.id));
            let _ = universe.invoke_projection_command(&card.id.clone());
        }
        universe.invoke_projection_command(NUDGE_COMMAND).unwrap();
        trusts.push(trust(&universe));
    }
    trusts
}

const FIRST_MONTH: usize = 30;
const FIRST_WEEK: usize = 7;

#[test]
fn a_places_first_month_cards_are_mostly_its_own() {
    let mut failed = Vec::new();
    for player in [Player::Warm, Player::Builder] {
        let asked = PLACES.map(|(place, seed)| (place, asked(seed, player, FIRST_MONTH).0));
        for (place, cards) in &asked {
            for (other, theirs) in &asked {
                if place == other {
                    continue;
                }
                let shared = cards.intersection(theirs).collect::<Vec<_>>();
                let share = shared.len() as f64 / cards.len().max(1) as f64;
                eprintln!(
                    "{player:?} {place}: {} cards, {} shared with {other} ({:.0}%): {shared:?}",
                    cards.len(),
                    shared.len(),
                    share * 100.0
                );
                if share > 0.2 {
                    failed.push(format!(
                        "{player:?}: {place} shares {} of {} first-month cards with {other}: {shared:?}",
                        shared.len(),
                        cards.len()
                    ));
                }
            }
        }
    }
    assert!(failed.is_empty(), "{failed:#?}");
}

#[test]
fn trust_is_not_full_in_the_first_week() {
    let mut failed = Vec::new();
    for (place, seed) in PLACES {
        for player in [Player::Warm, Player::Builder] {
            let (_, trusts) = asked(seed, player, FIRST_WEEK);
            eprintln!("{place} {player:?}: trust by day {trusts:?}");
            if trusts.iter().any(|trust| *trust >= 10) {
                failed.push(format!("{place} {player:?}: {trusts:?}"));
            }
        }
        let trusts = eager(seed, FIRST_WEEK);
        eprintln!("{place} eager: trust by day {trusts:?}");
        if trusts.iter().any(|trust| *trust >= 10) {
            failed.push(format!("{place} eager: {trusts:?}"));
        }
    }
    assert!(failed.is_empty(), "{failed:#?}");
}

/// A new place as it opens: its lamps, the plots open to build on, where
/// its keeper stands, and the first thing said to the player.
fn opened(seed: &str) -> (Vec<f32>, Vec<i32>, f32, String) {
    let mut universe = PocketUniverse::new().unwrap();
    universe.invoke_projection_command(seed).unwrap();
    let snapshot = universe.projection_snapshot();
    let canvas = &snapshot.canvas;
    let lamps = canvas.look.as_ref().unwrap().lamps.clone();
    let plots = canvas
        .plots
        .iter()
        .map(|plot| (plot.px * 100.0).round() as i32)
        .collect();
    let keeper = canvas
        .px_of(world_projection::SelectionId::Entity(crate::SLOT_B))
        .unwrap();
    let hello = universe
        .world()
        .events_of_kind(&["greeted"])
        .first()
        .and_then(|event| match event.payload.get("said") {
            Some(Value::Text(said)) => Some(said.clone()),
            _ => None,
        })
        .unwrap_or_default();
    (lamps, plots, keeper, hello)
}

/// No two places share lamp positions, plots, where the keeper stands or
/// the opening line (the art director's v0.29 brief).
#[test]
fn no_two_places_open_alike() {
    let places = PLACES.map(|(place, seed)| (place, opened(seed)));
    for (place, (lamps, plots, keeper, hello)) in &places {
        eprintln!("{place}: lamps {lamps:?} plots {plots:?} keeper {keeper} hello {hello:?}");
    }
    for (at, (place, (lamps, plots, keeper, hello))) in places.iter().enumerate() {
        assert!(!lamps.is_empty() && !plots.is_empty() && !hello.is_empty());
        for (other, (theirs, their_plots, their_keeper, their_hello)) in &places[at + 1..] {
            let shared_lamps = lamps
                .iter()
                .filter(|lamp| theirs.iter().any(|their| (*lamp - their).abs() < 0.03))
                .count();
            assert_eq!(shared_lamps, 0, "{place} and {other} share lamps");
            let shared_plots = plots
                .iter()
                .filter(|plot| their_plots.contains(plot))
                .count();
            assert!(
                shared_plots * 5 <= plots.len().min(their_plots.len()),
                "{place} and {other} open on the same plots: {plots:?} {their_plots:?}"
            );
            assert!(
                (keeper - their_keeper).abs() > 0.05,
                "{place} and {other} keep their keeper in one spot"
            );
            assert_ne!(
                hello.split(". ").next(),
                their_hello.split(". ").next(),
                "{place} and {other} open on the same line"
            );
        }
    }
}
