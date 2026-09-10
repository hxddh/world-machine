//! What happens to a World nobody is answering.
//!
//! Measured before this existed: a World seeded and then left alone walked
//! itself as far as period 18 — its trouble rose, peaked, and cost it its
//! anchor, and a successor stepped forward — and then stopped there forever,
//! holding three commands nobody was going to press. A World never given even
//! its opening choices never got past `legacy = forming` at all.
//!
//! So every choice the World offers now has a deadline as well as a default.
//! Leaving is itself an answer: the World reaches the default on its own, marks
//! the decision as one it made rather than one it was given, and says so when
//! somebody comes back. A drifted decision is exactly as durable as a chosen
//! one — that is the cost of staying away, and the reason to return.
//!
//! Two things deliberately do not drift. A trouble left unanswered already has
//! a consequence: chapter three takes the anchor, which is a stronger and more
//! legible outcome than quietly picking `hold` for somebody. And a lost anchor
//! is never recovered by drift, because leaving a loss standing is a coherent
//! way for a World to be, while rebuilding is a thing a person chooses.

use super::*;
use world_core::Event;

/// Generations a choice stays open, after it becomes available, before the
/// World answers it. Long enough that an attentive observer is never
/// overridden; short enough that an abandoned World keeps moving.
const DRIFT_AFTER_GENERATIONS: i64 = 4;

/// Generations a successor waits before formalising what has already happened.
/// Chapter four's bands already say the work becomes theirs; past this, saying
/// so out loud is the honest default.
const DRIFT_AFTER_PATIENCE: i64 = 6;

/// The generation at which the World starts offering each choice, mirroring
/// what the projection gates on. Drift measures its deadline from here.
const INTERVENTION_AVAILABLE_AT: i64 = 2;
const POSTURE_AVAILABLE_AT: i64 = 6;

/// Marks who answered. Absent on a decision made before this existed, which
/// reads as the observer's.
pub(crate) const DECIDED_BY: &str = "decided_by";
pub(crate) const DECIDED_BY_ARG: &str = "decided_by";
pub(crate) const BY_DRIFT: &str = "drift";

/// Whether this request was the World answering for itself.
pub(crate) fn requested_by_drift(request: &ActionRequest) -> bool {
    matches!(
        request.args.get(DECIDED_BY_ARG),
        Some(Value::Text(by)) if by == BY_DRIFT
    )
}

/// Record who answered on the Event, so the briefing can tell somebody what
/// was decided without them and replay keeps the distinction.
pub(crate) fn record_decider(draft: &mut EventDraft, request: &ActionRequest) {
    let by = if requested_by_drift(request) {
        BY_DRIFT
    } else {
        "observer"
    };
    draft.payload.insert(DECIDED_BY.into(), by.into());
}

/// Whether an Event was the World deciding for itself.
pub(crate) fn was_drifted(event: &Event) -> bool {
    matches!(
        event.payload.get(DECIDED_BY),
        Some(Value::Text(by)) if by == BY_DRIFT
    )
}

/// Answer at most one open choice per period, so a long absence unfolds as a
/// sequence a returning observer can read rather than resolving in one jump.
pub(crate) fn resolve_stage(
    world: &mut World,
    actions: &ActionRegistry,
    tail: EventId,
) -> Result<EventId, Box<dyn Error>> {
    let Some(action) = overdue_choice(world.state())? else {
        return Ok(tail);
    };
    let request = ActionRequest::new(action)
        .actor(UNIVERSE)
        .arg(DECIDED_BY_ARG, BY_DRIFT)
        .caused_by(tail);
    Ok(world.execute(actions, &request)?.id)
}

/// The choice this World has left open longest past its deadline, if any.
fn overdue_choice(state: &WorldState) -> Result<Option<&'static str>, ActionError> {
    if seed_id_from_state(state)? == UNSEEDED {
        return Ok(None);
    }
    let generation = integer_component(state, UNIVERSE, GENERATION)?;

    let decision = text_component_from_state(state, UNIVERSE, DECISION).unwrap_or_default();
    if decision.is_empty() || decision == "none" {
        if generation >= INTERVENTION_AVAILABLE_AT + DRIFT_AFTER_GENERATIONS {
            return Ok(Some("choose_careful_path"));
        }
        return Ok(None);
    }

    let posture = text_component_from_state(state, UNIVERSE, POSTURE).unwrap_or_default();
    if posture.is_empty() || posture == "none" {
        if generation >= POSTURE_AVAILABLE_AT + DRIFT_AFTER_GENERATIONS {
            return Ok(Some("choose_rooted_posture"));
        }
        return Ok(None);
    }

    if succession::choice_open(&succession::succession_id_from_state(state))
        && succession::succession_patience_from_state(state) >= DRIFT_AFTER_PATIENCE
    {
        return Ok(Some("release_legacy"));
    }
    Ok(None)
}

/// What the World decided while nobody was answering, for the return briefing.
pub(crate) fn drifted_decisions(events: &[Event]) -> Vec<&Event> {
    events.iter().filter(|event| was_drifted(event)).collect()
}

/// How a drifted decision reads to somebody who has just come back.
pub(crate) fn drift_note(event: &Event) -> Option<&'static str> {
    match event.kind.as_str() {
        "universe_intervened" => {
            Some("Nobody chose a direction, so the World took the careful one.")
        }
        "world_posture_chosen" => {
            Some("Nobody chose whether to reach outward, so the World stayed as it was.")
        }
        "legacy_released" => Some(
            "Nobody said whether to hand the legacy on, so the successor kept what they had already made theirs.",
        ),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn universe_state(components: &[(&str, Value)]) -> WorldState {
        let mut state = WorldState::default();
        let mut universe = Entity::new(UNIVERSE, "universe").with_component(SEED, "mars-colony");
        for (key, value) in components {
            universe = universe.with_component(*key, value.clone());
        }
        state.seed_entity(universe).unwrap();
        state
    }

    #[test]
    fn an_attentive_world_is_never_answered_for() {
        // Every choice still inside its deadline is left alone.
        let fresh = universe_state(&[(GENERATION, 1_i64.into())]);
        assert_eq!(overdue_choice(&fresh).unwrap(), None);

        let deciding = universe_state(&[(
            GENERATION,
            (INTERVENTION_AVAILABLE_AT + DRIFT_AFTER_GENERATIONS - 1).into(),
        )]);
        assert_eq!(overdue_choice(&deciding).unwrap(), None);
    }

    #[test]
    fn a_world_left_at_its_opening_choice_takes_the_careful_one() {
        let overdue = universe_state(&[(
            GENERATION,
            (INTERVENTION_AVAILABLE_AT + DRIFT_AFTER_GENERATIONS).into(),
        )]);
        assert_eq!(
            overdue_choice(&overdue).unwrap(),
            Some("choose_careful_path")
        );
    }

    #[test]
    fn a_world_left_at_its_direction_stays_as_it_was() {
        // The posture choice opens at generation 6 and gets the same four
        // generations of grace every choice gets.
        let decided = universe_state(&[
            (
                GENERATION,
                (POSTURE_AVAILABLE_AT + DRIFT_AFTER_GENERATIONS).into(),
            ),
            (DECISION, "careful".into()),
        ]);
        assert_eq!(
            overdue_choice(&decided).unwrap(),
            Some("choose_rooted_posture")
        );

        // ... but not one generation before.
        let recent = universe_state(&[
            (
                GENERATION,
                (POSTURE_AVAILABLE_AT + DRIFT_AFTER_GENERATIONS - 1).into(),
            ),
            (DECISION, "careful".into()),
        ]);
        assert_eq!(overdue_choice(&recent).unwrap(), None);
    }

    #[test]
    fn a_successor_left_waiting_long_enough_keeps_what_is_already_theirs() {
        let settled_enough = |patience: i64| {
            universe_state(&[
                (GENERATION, 30_i64.into()),
                (DECISION, "careful".into()),
                (POSTURE, "rooted".into()),
                (succession::SUCCESSION, "emerging".into()),
                (succession::SUCCESSION_PATIENCE, patience.into()),
            ])
        };
        assert_eq!(overdue_choice(&settled_enough(2)).unwrap(), None);
        assert_eq!(
            overdue_choice(&settled_enough(DRIFT_AFTER_PATIENCE)).unwrap(),
            Some("release_legacy")
        );

        // A settled succession is not answered again.
        let settled = universe_state(&[
            (GENERATION, 30_i64.into()),
            (DECISION, "careful".into()),
            (POSTURE, "rooted".into()),
            (succession::SUCCESSION, "settled".into()),
            (succession::SUCCESSION_PATIENCE, 40_i64.into()),
        ]);
        assert_eq!(overdue_choice(&settled).unwrap(), None);
    }

    #[test]
    fn an_unseeded_world_is_never_answered_for() {
        let mut state = WorldState::default();
        state
            .seed_entity(
                Entity::new(UNIVERSE, "universe")
                    .with_component(SEED, UNSEEDED)
                    .with_component(GENERATION, 99_i64),
            )
            .unwrap();
        assert_eq!(overdue_choice(&state).unwrap(), None);
    }

    #[test]
    fn every_drifted_decision_can_be_explained_to_somebody_who_returns() {
        for kind in [
            "universe_intervened",
            "world_posture_chosen",
            "legacy_released",
        ] {
            let mut event = EventDraft::new(kind);
            event.payload.insert(DECIDED_BY.into(), BY_DRIFT.into());
            let note = drift_note(&Event {
                id: EventId::new(1),
                kind: kind.into(),
                actor: None,
                targets: Vec::new(),
                payload: event.payload.clone(),
                changes: Vec::new(),
                caused_by: Vec::new(),
                world_time: 0,
            });
            assert!(
                note.is_some(),
                "{kind} has no note for a returning observer"
            );
        }
    }
}
