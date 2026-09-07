//! Chapter three: a seed-specific long-run pressure that arrives after the
//! World's legacy has reinforced itself, gives the observer a bounded window
//! to hold or reach, and otherwise durably costs the World its anchor.
//!
//! Everything here is a validated Action producing an Event with explicit
//! state changes, so replay never re-runs these rules. The kernel knows
//! nothing about pressure; it is Pocket Universe vocabulary only.

use super::*;

/// On `UNIVERSE`: `none` → `warning` → `crisis` → `lost`, or resolved as
/// `held` / `reached` from the open window, or `recovered` after a loss.
pub(crate) const PRESSURE: &str = "pressure";
/// Generation at which the current pressure stage began.
pub(crate) const PRESSURE_GENERATION: &str = "pressure_generation";
/// Durable record of how the World answered: `none`, `aligned`, `strained`,
/// `lost`, or `recovered`.
pub(crate) const PRESSURE_OUTCOME: &str = "pressure_outcome";

/// Pressure rises once the legacy has reinforced this many times.
const RISE_AFTER_LEGACY_CYCLES: i64 = 2;
/// Generations of warning before the pressure peaks into a crisis.
const WARNING_GENERATIONS: i64 = 2;
/// Generations of crisis before the anchor is lost.
const CRISIS_GENERATIONS: i64 = 3;

pub(crate) fn register_actions(actions: &mut ActionRegistry) -> Result<(), ActionError> {
    actions.register(RaisePressure)?;
    actions.register(EscalatePressure)?;
    actions.register(LoseAnchor)?;
    actions.register(HoldThroughPressure)?;
    actions.register(ReachBeyondPressure)?;
    actions.register(RecoverAnchor)?;
    Ok(())
}

/// Advance the pressure stage at most one step per period. Runs after the
/// legacy consequences so the rise reads the cycle count that period wrote.
pub(crate) fn resolve_period_pressure(
    world: &mut World,
    actions: &ActionRegistry,
    tail: EventId,
) -> Result<EventId, Box<dyn Error>> {
    let state = world.state();
    let action = if rise_candidate(state)?.is_some() {
        "raise_pressure"
    } else if escalate_candidate(state)?.is_some() {
        "escalate_pressure"
    } else if loss_candidate(state)?.is_some() {
        "lose_anchor"
    } else {
        return Ok(tail);
    };
    let mut request = ActionRequest::new(action).caused_by(tail);
    for cause in pressure_causes(world) {
        if cause != tail {
            request = request.caused_by(cause);
        }
    }
    Ok(world.execute(actions, &request)?.id)
}

pub(crate) fn pressure_id_from_state(state: &WorldState) -> String {
    match state
        .entity(UNIVERSE)
        .and_then(|entity| entity.component(PRESSURE))
    {
        Some(Value::Text(pressure)) => pressure.clone(),
        _ => "none".into(),
    }
}

fn pressure_generation_from_state(state: &WorldState) -> i64 {
    match state
        .entity(UNIVERSE)
        .and_then(|entity| entity.component(PRESSURE_GENERATION))
    {
        Some(Value::Integer(generation)) => *generation,
        _ => 0,
    }
}

/// True while the observer can still hold or reach.
pub(crate) fn window_open(pressure: &str) -> bool {
    matches!(pressure, "warning" | "crisis")
}

fn pressure_causes(world: &World) -> Vec<EventId> {
    let mut causes = Vec::new();
    for kind in [
        "pressure_rising",
        "pressure_peaked",
        "legacy_reinforced",
        "world_legacy_formed",
        "world_posture_chosen",
    ] {
        if let Some(event) = world.events().iter().rev().find(|event| event.kind == kind) {
            if !causes.contains(&event.id) {
                causes.push(event.id);
            }
        }
    }
    causes
}

fn rise_candidate(state: &WorldState) -> Result<Option<()>, ActionError> {
    if pressure_id_from_state(state) != "none" {
        return Ok(None);
    }
    if legacy::legacy_id_from_state(state)? == "forming" {
        return Ok(None);
    }
    let cycles = integer_component(state, UNIVERSE, LEGACY_CYCLES)?;
    Ok((cycles >= RISE_AFTER_LEGACY_CYCLES).then_some(()))
}

fn escalate_candidate(state: &WorldState) -> Result<Option<()>, ActionError> {
    if pressure_id_from_state(state) != "warning" {
        return Ok(None);
    }
    let generation = integer_component(state, UNIVERSE, GENERATION)?;
    let since = pressure_generation_from_state(state);
    Ok((generation >= since + WARNING_GENERATIONS).then_some(()))
}

fn loss_candidate(state: &WorldState) -> Result<Option<()>, ActionError> {
    if pressure_id_from_state(state) != "crisis" {
        return Ok(None);
    }
    let generation = integer_component(state, UNIVERSE, GENERATION)?;
    let since = pressure_generation_from_state(state);
    Ok((generation >= since + CRISIS_GENERATIONS).then_some(()))
}

/// Seed-specific copy for every stage. The anchor is always `SLOT_A`.
#[derive(Clone, Copy)]
pub(crate) struct PressureCopy {
    pub warning_status: &'static str,
    pub warning_summary: &'static str,
    pub crisis_status: &'static str,
    pub crisis_summary: &'static str,
    pub lost_status: &'static str,
    pub lost_summary: &'static str,
    pub hold_title: &'static str,
    pub hold_detail: &'static str,
    pub hold_status: &'static str,
    pub hold_summary: &'static str,
    pub reach_title: &'static str,
    pub reach_detail: &'static str,
    pub reach_status: &'static str,
    pub reach_summary: &'static str,
    pub recover_title: &'static str,
    pub recover_detail: &'static str,
    pub recover_status: &'static str,
    pub recover_summary: &'static str,
}

pub(crate) fn copy_for_seed(seed: &str) -> PressureCopy {
    match seed {
        "mars-colony" => PressureCopy {
            warning_status: "reclaimer faltering",
            warning_summary: "The water reclaimer is losing efficiency. Nia logs the first shortfall and the colony starts counting sols.",
            crisis_status: "rationing water",
            crisis_summary: "Ares Habitat is rationing water. The reclaimer will fail within a few sols unless the colony acts.",
            lost_status: "lower ring sealed",
            lost_summary: "The reclaimer failed. Ares Habitat sealed its lower ring, and the colony now lives on half its water.",
            hold_title: "Rebuild the reclaimer from what Ares has",
            hold_detail: "Nia strips Kestrel's spare parts to rebuild the water reclaimer inside the habitat. Nothing leaves the ridge.",
            hold_status: "reclaimer rebuilt",
            hold_summary: "Nia rebuilt the water reclaimer from Kestrel's spare parts. Ares held on with what it already had.",
            reach_title: "Send Kestrel for a replacement",
            reach_detail: "Tomas drives Kestrel past the familiar ridge to the relay station for a replacement reclaimer.",
            reach_status: "resupplied from the ridge",
            reach_summary: "Tomas brought a replacement reclaimer back from the relay station. Ares was saved by reaching beyond the ridge.",
            recover_title: "Reopen the sealed ring",
            recover_detail: "Salvage a reclaimer from the sealed ring and reopen it. The routines the colony built will have to be rebuilt too.",
            recover_status: "ring reopened",
            recover_summary: "The colony reopened the sealed ring with a salvaged reclaimer. The habits it had built are gone, and its legacy starts over.",
        },
        "1980s-town" => PressureCopy {
            warning_status: "rent rising",
            warning_summary: "The arcade's landlord posts a rent increase. Lena counts the quarters and the numbers do not reach.",
            crisis_status: "lease ending",
            crisis_summary: "Maple Arcade has thirty days before the lease ends. The shutters could stay down for good.",
            lost_status: "closed",
            lost_summary: "The lease ended. Maple Arcade closed, and the shutters stayed down.",
            hold_title: "Raise the rent from the neighborhood",
            hold_detail: "Lena runs a neighborhood fundraiser to cover the new lease with the people who already show up.",
            hold_status: "neighborhood-funded",
            hold_summary: "The neighborhood covered the new lease. Maple Arcade stayed open on Maple Street's own money.",
            reach_title: "Put the arcade on the air",
            reach_detail: "Max gets K-88 to broadcast the arcade's story and pulls in backers from beyond the neighborhood.",
            reach_status: "backed from outside",
            reach_summary: "K-88 carried the arcade's story past Maple Street, and outside backers covered the lease.",
            recover_title: "Reopen under new hands",
            recover_detail: "Reopen the arcade with new owners. The old regulars' routines are gone and will have to form again.",
            recover_status: "reopened",
            recover_summary: "Maple Arcade reopened under new hands. The old regulars' routines are gone, and its legacy starts over.",
        },
        "penguin-civilization" => PressureCopy {
            warning_status: "span cracked",
            warning_summary: "A crack runs across the third span of the ice bridge. Piko marks it and the council starts to worry.",
            crisis_status: "bridge closed",
            crisis_summary: "The bridge is closed at moonrise. The outer colonies are cut off until the span is dealt with.",
            lost_status: "span collapsed",
            lost_summary: "The third span collapsed. Icebridge is an island until someone raises a new one.",
            hold_title: "Rebuild the span through the dark season",
            hold_detail: "Piko rebuilds the cracked span with packed snow and Icebridge's own hands, one moonrise at a time.",
            hold_status: "span rebuilt",
            hold_summary: "Piko rebuilt the cracked span with packed snow. Icebridge held its bridge with its own hands.",
            reach_title: "Send for the outer builders",
            reach_detail: "The Aurora Council sends for the outer colonies' bridge builders to shore up the span.",
            reach_status: "outer builders arrived",
            reach_summary: "The outer colonies' builders shored up the span. Icebridge kept its bridge by reaching beyond it.",
            recover_title: "Raise a new span",
            recover_detail: "Raise a new span after the collapse. The winter routines that crossed the old one will have to be rebuilt.",
            recover_status: "new span raised",
            recover_summary: "Icebridge raised a new span after the collapse. The winter routines that crossed the old one are gone, and its legacy starts over.",
        },
        _ => PressureCopy {
            warning_status: "under strain",
            warning_summary: "Something the World depends on is starting to fail.",
            crisis_status: "failing",
            crisis_summary: "Something the World depends on will fail soon unless the World acts.",
            lost_status: "lost",
            lost_summary: "Something the World depended on has been lost.",
            hold_title: "Hold with what the World has",
            hold_detail: "Answer the pressure from inside the World.",
            hold_status: "held",
            hold_summary: "The World held with what it already had.",
            reach_title: "Reach beyond the World",
            reach_detail: "Answer the pressure by reaching outside the World.",
            reach_status: "reached",
            reach_summary: "The World was saved by reaching beyond itself.",
            recover_title: "Recover what was lost",
            recover_detail: "Rebuild after the loss at the cost of what the World had built.",
            recover_status: "recovered",
            recover_summary: "The World recovered what it lost, and its legacy starts over.",
        },
    }
}

fn stage_draft(
    state: &WorldState,
    kind: &str,
    stage: &str,
    status: &str,
    summary: &str,
    outcome: Option<&str>,
) -> Result<EventDraft, ActionError> {
    let generation = integer_component(state, UNIVERSE, GENERATION)?;
    let seed = seed_id_from_state(state)?;
    let mut draft = EventDraft::new(kind);
    draft.targets = vec![UNIVERSE, SLOT_A];
    draft.payload.insert("seed".into(), seed.into());
    draft.payload.insert("stage".into(), stage.into());
    draft.payload.insert("summary".into(), summary.into());
    draft.changes = vec![
        StateChange::SetComponent {
            entity: UNIVERSE,
            key: PRESSURE.into(),
            value: stage.into(),
        },
        StateChange::SetComponent {
            entity: UNIVERSE,
            key: PRESSURE_GENERATION.into(),
            value: generation.into(),
        },
        StateChange::SetComponent {
            entity: SLOT_A,
            key: "status".into(),
            value: status.into(),
        },
        StateChange::SetComponent {
            entity: UNIVERSE,
            key: LAST_CHANGE.into(),
            value: summary.into(),
        },
    ];
    if let Some(outcome) = outcome {
        draft.payload.insert("outcome".into(), outcome.into());
        draft.changes.push(StateChange::SetComponent {
            entity: UNIVERSE,
            key: PRESSURE_OUTCOME.into(),
            value: outcome.into(),
        });
    }
    Ok(draft)
}

struct RaisePressure;
struct EscalatePressure;
struct LoseAnchor;
struct HoldThroughPressure;
struct ReachBeyondPressure;
struct RecoverAnchor;

impl Action for RaisePressure {
    fn name(&self) -> &'static str {
        "raise_pressure"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        _request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        rise_candidate(state)?.ok_or_else(|| {
            ActionError::Invalid(
                "this World's legacy has not settled enough for pressure to rise".into(),
            )
        })?;
        let copy = copy_for_seed(&seed_id_from_state(state)?);
        stage_draft(
            state,
            "pressure_rising",
            "warning",
            copy.warning_status,
            copy.warning_summary,
            None,
        )
    }
}

impl Action for EscalatePressure {
    fn name(&self) -> &'static str {
        "escalate_pressure"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        _request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        escalate_candidate(state)?.ok_or_else(|| {
            ActionError::Invalid("this World's pressure is not ready to peak".into())
        })?;
        let copy = copy_for_seed(&seed_id_from_state(state)?);
        stage_draft(
            state,
            "pressure_peaked",
            "crisis",
            copy.crisis_status,
            copy.crisis_summary,
            None,
        )
    }
}

impl Action for LoseAnchor {
    fn name(&self) -> &'static str {
        "lose_anchor"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        _request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        loss_candidate(state)?.ok_or_else(|| {
            ActionError::Invalid("this World's crisis has not run out of time".into())
        })?;
        let copy = copy_for_seed(&seed_id_from_state(state)?);
        stage_draft(
            state,
            "anchor_lost",
            "lost",
            copy.lost_status,
            copy.lost_summary,
            Some("lost"),
        )
    }
}

fn answer_draft(state: &WorldState, reach: bool) -> Result<EventDraft, ActionError> {
    let seed = seed_id_from_state(state)?;
    if seed == UNSEEDED {
        return Err(ActionError::Invalid(
            "choose a Pocket Universe seed before answering its pressure".into(),
        ));
    }
    let pressure = pressure_id_from_state(state);
    if !window_open(&pressure) {
        return Err(ActionError::Invalid(format!(
            "this World has no open pressure to answer (pressure is {pressure})"
        )));
    }
    let posture = posture_id_from_state(state)?;
    let aligned = matches!(
        (reach, posture.as_str()),
        (true, "outward") | (false, "rooted")
    );
    let outcome = if aligned { "aligned" } else { "strained" };
    let copy = copy_for_seed(&seed);
    let (kind, stage, status, summary) = if reach {
        (
            "pressure_reached",
            "reached",
            copy.reach_status,
            copy.reach_summary,
        )
    } else {
        ("pressure_held", "held", copy.hold_status, copy.hold_summary)
    };
    let mut draft = stage_draft(state, kind, stage, status, summary, Some(outcome))?;
    draft.payload.insert("posture".into(), posture.into());
    draft
        .payload
        .insert("answered_from".into(), pressure.into());
    Ok(draft)
}

impl Action for HoldThroughPressure {
    fn name(&self) -> &'static str {
        "hold_through_pressure"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        _request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        answer_draft(state, false)
    }
}

impl Action for ReachBeyondPressure {
    fn name(&self) -> &'static str {
        "reach_beyond_pressure"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        _request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        answer_draft(state, true)
    }
}

impl Action for RecoverAnchor {
    fn name(&self) -> &'static str {
        "recover_anchor"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        _request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let pressure = pressure_id_from_state(state);
        if pressure != "lost" {
            return Err(ActionError::Invalid(format!(
                "this World has nothing to recover (pressure is {pressure})"
            )));
        }
        let copy = copy_for_seed(&seed_id_from_state(state)?);
        let mut draft = stage_draft(
            state,
            "anchor_recovered",
            "recovered",
            copy.recover_status,
            copy.recover_summary,
            Some("recovered"),
        )?;
        // Recovery is durable and costly: the legacy has to reinforce itself
        // again from nothing.
        draft.changes.push(StateChange::SetComponent {
            entity: UNIVERSE,
            key: LEGACY_CYCLES.into(),
            value: 0_i64.into(),
        });
        Ok(draft)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_seed_has_distinct_copy_for_every_stage() {
        for seed in ["mars-colony", "1980s-town", "penguin-civilization"] {
            let copy = copy_for_seed(seed);
            let statuses = [
                copy.warning_status,
                copy.crisis_status,
                copy.lost_status,
                copy.hold_status,
                copy.reach_status,
                copy.recover_status,
            ];
            for (index, status) in statuses.iter().enumerate() {
                assert!(!status.is_empty());
                assert!(
                    !statuses[index + 1..].contains(status),
                    "{seed} reuses status {status}"
                );
            }
            assert_ne!(copy.hold_title, copy.reach_title);
        }
    }

    #[test]
    fn window_is_open_only_while_the_observer_can_still_answer() {
        assert!(window_open("warning"));
        assert!(window_open("crisis"));
        for closed in ["none", "held", "reached", "lost", "recovered"] {
            assert!(!window_open(closed), "{closed} should be closed");
        }
    }
}
