use crate::{actions::text_component, fishing::contract_remaining, model::*};
use society_basic::{integer_component, CASH, EMPLOYER, JOB};
use world_core::{Action, ActionError, ActionRegistry, ActionRequest, EventDraft, StateChange, WorldState};

pub(crate) const HARBOR_FISHING_WAGE: i64 = 25;

pub(crate) fn register_actions(registry: &mut ActionRegistry) -> Result<(), ActionError> {
    registry.register(RecordHarborPayrollExhaustion)?;
    registry.register(SuspendHarborFishing)?;
    Ok(())
}

struct RecordHarborPayrollExhaustion;

impl Action for RecordHarborPayrollExhaustion {
    fn name(&self) -> &'static str {
        "record_harbor_payroll_exhaustion"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        _request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        if contract_remaining(state) != Some(0) {
            return Err(ActionError::Invalid(
                "Harbor still has contracted mainland demand".into(),
            ));
        }
        if text_component(state, JONAS, JOB)? != "fisher" {
            return Err(ActionError::Invalid("Jonas is not actively fishing".into()));
        }
        if state.relation(JONAS_HARBOR_JOB).is_none() {
            return Err(ActionError::Invalid(
                "Jonas has no active Harbor job relation".into(),
            ));
        }
        let cash = integer_component(state, HARBOR, CASH)?;
        if cash >= HARBOR_FISHING_WAGE {
            return Err(ActionError::Invalid(format!(
                "Harbor still has {cash} cash and can cover the next fishing wage"
            )));
        }

        let mut draft = EventDraft::new("harbor_payroll_exhausted");
        draft.targets = vec![HARBOR, JONAS];
        draft.payload.insert("cash_available".into(), cash.into());
        draft
            .payload
            .insert("next_wage".into(), HARBOR_FISHING_WAGE.into());
        Ok(draft)
    }
}

struct SuspendHarborFishing;

impl Action for SuspendHarborFishing {
    fn name(&self) -> &'static str {
        "suspend_harbor_fishing"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        _request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        if text_component(state, JONAS, JOB)? != "fisher" {
            return Err(ActionError::Invalid("Jonas is not actively fishing".into()));
        }
        if state.relation(JONAS_HARBOR_JOB).is_none() {
            return Err(ActionError::Invalid(
                "Jonas has no active Harbor job relation".into(),
            ));
        }

        let mut draft = EventDraft::new("fishing_suspended");
        draft.targets = vec![HARBOR, JONAS, JONAS_BOAT];
        draft.changes = vec![
            StateChange::RemoveRelation(JONAS_HARBOR_JOB),
            StateChange::SetComponent {
                entity: JONAS,
                key: JOB.into(),
                value: "fishing_suspended".into(),
            },
            StateChange::RemoveComponent {
                entity: JONAS,
                key: EMPLOYER.into(),
            },
        ];
        Ok(draft)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{TinySociety, REPAIR_BOAT_COMMAND};

    #[test]
    fn an_unrenewed_contract_suspends_fishing_before_payroll_can_fail_silently() {
        let mut simulation = TinySociety::new().unwrap();
        simulation.run_story().unwrap();
        let mut branch = simulation.branch();
        branch.advance_days(10).unwrap();
        branch
            .invoke_projection_command(REPAIR_BOAT_COMMAND)
            .unwrap();
        branch.advance_days(5).unwrap();
        assert_eq!(contract_remaining(branch.world().state()), Some(0));

        let cursor = branch.visit_cursor();
        branch.advance_days(40).unwrap();
        let events = &branch.world().events()[cursor.event_count..];
        let exhaustion = events
            .iter()
            .find(|event| event.kind == "harbor_payroll_exhausted")
            .expect("Harbor records payroll exhaustion before the next unpaid day");
        let suspended = events
            .iter()
            .find(|event| event.kind == "fishing_suspended")
            .expect("payroll exhaustion suspends fishing");
        assert_eq!(exhaustion.caused_by.len(), 1);
        let last_paid_shift = branch
            .world()
            .event(exhaustion.caused_by[0])
            .expect("exhaustion keeps the last paid shift as its cause");
        assert_eq!(last_paid_shift.kind, "work_shift_completed");
        assert_eq!(last_paid_shift.actor, Some(JONAS));
        assert!(last_paid_shift.targets.contains(&HARBOR));
        assert_eq!(suspended.caused_by, vec![exhaustion.id]);
        assert!(integer_component(branch.world().state(), HARBOR, CASH).unwrap() < HARBOR_FISHING_WAGE);
        assert_eq!(text_component(branch.world().state(), JONAS, JOB).unwrap(), "fishing_suspended");
        assert!(branch.world().state().relation(JONAS_HARBOR_JOB).is_none());

        let snapshot = branch.projection_snapshot();
        assert!(snapshot
            .briefing
            .as_ref()
            .is_some_and(|briefing| briefing
                .items
                .iter()
                .any(|item| item.title == "Harbor suspended fishing")));

        let archive = branch.archive().unwrap();
        let resumed = TinySociety::resume_archive(&archive).unwrap();
        assert_eq!(text_component(resumed.world().state(), JONAS, JOB).unwrap(), "fishing_suspended");
        assert!(resumed.world().state().relation(JONAS_HARBOR_JOB).is_none());

        let cursor = branch.visit_cursor();
        branch.advance_days(2).unwrap();
        assert!(branch.world().events()[cursor.event_count..]
            .iter()
            .all(|event| event.actor != Some(JONAS) || event.kind != "work_shift_completed"));
        assert!(branch.world().events()[cursor.event_count..]
            .iter()
            .all(|event| event.kind != "catch_landed"));
    }
}
