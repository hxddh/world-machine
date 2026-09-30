//! A new player's arrival in the harbour: the World opens on Day 1, in
//! fair weather, with a hello before any question, and the first days are
//! kind. The harbour's old storm, and Jonas losing his place at the
//! bakery, are a story Worlds made before this one keep telling; a new
//! harbour begins without them.
//!
//! A World that began this way records it (`arrived`), so everything
//! here applies only to such Worlds: a World made before keeps its
//! history and its rules exactly.

use crate::{HARBOR, JONAS};
use society_basic::CASH;
use world_core::{
    Action, ActionError, ActionRegistry, ActionRequest, EventDraft, StateChange, Value, WorldState,
};

/// Set on the harbour when a new player arrived: the day they came.
pub(crate) const ARRIVED: &str = "arrived";

/// How many days a new player's first days last: the first session and a
/// little more, in which nothing sad or unkind is said.
pub(crate) const FIRST_DAYS: u64 = 5;

/// What Jonas has put by when a new player arrives: a fortnight's living
/// and a little more, so the first days are not a scramble.
const STARTING_SAVINGS: i64 = crate::hardship::HARDSHIP_EASED_THRESHOLD + 8;

/// The day a new player arrived, when the World began with one.
pub(crate) fn arrived(state: &WorldState) -> Option<u64> {
    match state.entity(HARBOR)?.component(ARRIVED)? {
        Value::Integer(day) => u64::try_from(*day).ok(),
        _ => None,
    }
}

/// The harbour's day now, counted from 1.
pub(crate) fn today(state: &WorldState) -> u64 {
    state
        .world_time()
        .div_ceil(crate::persistence::WORLD_DAY_TICKS)
        .max(1)
}

/// Whether a new player is still in their first days.
pub(crate) fn first_days(state: &WorldState) -> bool {
    arrived(state).is_some_and(|day| today(state) < day + FIRST_DAYS)
}

/// A new player arrives: recorded once, at the start of a new World.
struct Arrive;

impl Action for Arrive {
    fn name(&self) -> &'static str {
        "harbour_arrived"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        _request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        if arrived(state).is_some() || state.entity(HARBOR).is_none() {
            return Err(ActionError::Invalid("already arrived".into()));
        }
        let mut draft = EventDraft::new("arrived");
        draft.targets = vec![HARBOR];
        draft.changes = vec![StateChange::SetComponent {
            entity: HARBOR,
            key: ARRIVED.into(),
            value: Value::Integer(today(state) as i64),
        }];
        // A new harbour's fisher starts the season with a good catch put
        // by, not scraping by from the first morning.
        let cash = society_basic::integer_component(state, JONAS, CASH).unwrap_or(0);
        if state.entity(JONAS).is_some() && cash < STARTING_SAVINGS {
            draft.changes.push(StateChange::SetComponent {
                entity: JONAS,
                key: CASH.into(),
                value: Value::Integer(STARTING_SAVINGS),
            });
        }
        Ok(draft)
    }
}

pub(crate) fn register_actions(registry: &mut ActionRegistry) -> Result<(), ActionError> {
    registry.register(Arrive)
}
