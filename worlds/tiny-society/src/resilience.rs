use crate::{
    actions::text_component,
    local_economy::{LOCAL_SPENDING_STATUS, RECOVERED_BREAD_BUDGET},
    model::{BAKERY, EMMA, JONAS, LEO, OPERATING_STATUS},
};
use society_basic::JOB;
use std::error::Error;
use world_core::{
    Action, ActionError, ActionRegistry, ActionRequest, BehaviorRegistry, EntityId, Event,
    EventDraft, RuleBehavior, Value, WorldState,
};

pub(crate) fn register_actions(registry: &mut ActionRegistry) -> Result<(), ActionError> {
    registry.register(RecordLocalDemandBuffer)?;
    Ok(())
}

pub(crate) fn register_behaviors(registry: &mut BehaviorRegistry) -> Result<(), Box<dyn Error>> {
    registry.register(RuleBehavior::new(
        "recovered-local-demand-buffers-household-cut",
        ["bread_budget_cut"],
        |state: &WorldState, event: &Event| {
            let Some(household) = event.actor else {
                return Vec::new();
            };
            if !matches!(household, LEO | EMMA) || !event.targets.contains(&BAKERY) {
                return Vec::new();
            }
            if text_component(state, JONAS, LOCAL_SPENDING_STATUS).ok() != Some("active") {
                return Vec::new();
            }
            if text_component(state, JONAS, JOB).ok() != Some("fisher") {
                return Vec::new();
            }
            if text_component(state, BAKERY, OPERATING_STATUS).ok() != Some("open") {
                return Vec::new();
            }

            vec![ActionRequest::new("record_local_demand_buffer")
                .actor(JONAS)
                .arg("household", household)]
        },
    ))?;
    Ok(())
}

fn entity_arg(request: &ActionRequest, name: &str) -> Result<EntityId, ActionError> {
    match request.args.get(name) {
        Some(Value::Entity(id)) => Ok(*id),
        _ => Err(ActionError::Invalid(format!("missing entity arg: {name}"))),
    }
}

struct RecordLocalDemandBuffer;

impl Action for RecordLocalDemandBuffer {
    fn name(&self) -> &'static str {
        "record_local_demand_buffer"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let household = entity_arg(request, "household")?;
        if !matches!(household, LEO | EMMA) {
            return Err(ActionError::Invalid(
                "local demand buffering is currently defined for Leo/Emma household cuts".into(),
            ));
        }
        if text_component(state, JONAS, LOCAL_SPENDING_STATUS)? != "active" {
            return Err(ActionError::Invalid(
                "Jonas has not resumed local spending".into(),
            ));
        }
        if text_component(state, JONAS, JOB)? != "fisher" {
            return Err(ActionError::Invalid(
                "Jonas is no longer earning from restored fishing".into(),
            ));
        }
        if text_component(state, BAKERY, OPERATING_STATUS)? != "open" {
            return Err(ActionError::Invalid(
                "the bakery is already closed".into(),
            ));
        }

        let mut draft = EventDraft::new("local_demand_buffered");
        draft.actor = Some(JONAS);
        draft.targets = vec![JONAS, BAKERY, household];
        draft
            .payload
            .insert("daily_recovered_demand".into(), RECOVERED_BREAD_BUDGET.into());
        Ok(draft)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{TinySociety, REPAIR_BOAT_COMMAND};

    #[test]
    fn restored_local_demand_is_recorded_and_delays_bakery_closure() {
        let mut society = TinySociety::new().unwrap();
        society.run_story().unwrap();

        let mut baseline = society.branch();
        let mut recovered = society.branch();
        baseline.advance_days(10).unwrap();
        recovered.advance_days(10).unwrap();
        recovered
            .invoke_projection_command(REPAIR_BOAT_COMMAND)
            .unwrap();

        let mut baseline_closure_time = None;
        for _ in 0..150 {
            baseline.advance_days(1).unwrap();
            recovered.advance_days(1).unwrap();

            if let Some(closure) = baseline
                .world()
                .events()
                .iter()
                .find(|event| event.kind == "bakery_closed")
            {
                baseline_closure_time = Some(closure.world_time);
                break;
            }
        }
        let baseline_closure_time =
            baseline_closure_time.expect("household demand loss eventually closes baseline Bakery");

        assert!(baseline
            .world()
            .events()
            .iter()
            .all(|event| event.kind != "local_demand_buffered"));
        assert!(recovered.world().events().iter().all(|event| {
            event.kind != "bakery_closed" || event.world_time > baseline_closure_time
        }));
        assert_eq!(
            text_component(recovered.world().state(), BAKERY, OPERATING_STATUS).unwrap(),
            "open"
        );

        let buffered = recovered
            .world()
            .events()
            .iter()
            .find(|event| event.kind == "local_demand_buffered")
            .expect("recovered Jonas buffers a later household demand cut");
        assert_eq!(buffered.actor, Some(JONAS));
        assert!(buffered.targets.contains(&BAKERY));
        assert!(buffered.targets.iter().any(|target| matches!(*target, LEO | EMMA)));
        assert_eq!(
            buffered.payload.get("daily_recovered_demand"),
            Some(&Value::Integer(RECOVERED_BREAD_BUDGET))
        );
        assert_eq!(buffered.caused_by.len(), 1);
        let cut = recovered
            .world()
            .event(buffered.caused_by[0])
            .expect("buffering cause remains in history");
        assert_eq!(cut.kind, "bread_budget_cut");
        assert!(matches!(cut.actor, Some(LEO) | Some(EMMA)));

        let archive = recovered.archive().unwrap();
        let resumed = TinySociety::resume_archive(&archive).unwrap();
        assert!(resumed
            .world()
            .events()
            .iter()
            .any(|event| event.kind == "local_demand_buffered" && event.id == buffered.id));
    }
}
