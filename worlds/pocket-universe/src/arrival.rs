//! A new player's arrival in a place: its first days are kind. A place
//! seeded from this version on records the period it began
//! (`pocket.arrived`), so what is here applies only to such Worlds: a
//! World seeded before keeps its history and its rules exactly.

use crate::{BACKGROUND_PERIOD, UNIVERSE};
use world_core::{StateChange, Value, WorldState};

/// Set on the universe when a new player's place was seeded: the period
/// it began (0 for the beginning).
pub(crate) const ARRIVED: &str = "pocket.arrived";

/// How many periods a new player's first days last: the first session and
/// a little more, in which nothing sad or unkind is said.
pub(crate) const FIRST_DAYS: u64 = 5;

/// The period now, from 0 at the beginning.
pub(crate) fn today(state: &WorldState) -> u64 {
    state.world_time() / BACKGROUND_PERIOD
}

/// The period a new player's place began, when it was seeded this way.
pub(crate) fn arrived(state: &WorldState) -> Option<u64> {
    match state.entity(UNIVERSE)?.component(ARRIVED)? {
        Value::Integer(day) => u64::try_from(*day).ok(),
        _ => None,
    }
}

/// Whether a new player is still in their first days.
pub(crate) fn first_days(state: &WorldState) -> bool {
    arrived(state).is_some_and(|day| today(state) < day + FIRST_DAYS)
}

/// What seeding a place records so its first days can be kept kind.
pub(crate) fn marked(state: &WorldState) -> StateChange {
    StateChange::SetComponent {
        entity: UNIVERSE,
        key: ARRIVED.into(),
        value: Value::Integer(today(state) as i64),
    }
}

/// What the one who comes over to a new player says first, in each
/// place's own words: no two places open on the same line.
pub(crate) fn hello(state: &WorldState) -> &'static str {
    match crate::places::Place::of(state) {
        Some(crate::places::Place::Ares) => {
            "Airlock's sealed behind you. I'm {name}. Breathe easy, you're inside."
        }
        Some(crate::places::Place::Maple) => {
            "Hey, new kid on the street! I'm {name}. Arcade's open till ten."
        }
        Some(crate::places::Place::Ice) => {
            "Careful, the causeway's slippy! I'm {name}. Come and get warm."
        }
        None => "",
    }
}
