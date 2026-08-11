#[path = "resilience.rs"]
mod resilience;

use crate::{
    actions::text_component,
    model::{BAKERY, JONAS, OPERATING_STATUS, SUPPORT_STATUS},
};
use society_basic::{integer_component, CASH, JOB};
use std::error::Error;
use world_core::{
    Action, ActionError, ActionRegistry, ActionRequest, BehaviorRegistry, Event, EventDraft,
    RuleBehavior, StateChange, Value, WorldState,
};

pub(crate) const LOCAL_SPENDING_STATUS: &str = "local_spending_status";
pub(crate) const RECOVERED_BREAD_BUDGET: i64 = 6;

pub(crate) fn register_actions(registry: &mut ActionRegistry) -> Result<(), ActionError> {
    registry.register(ResumeLocalSpending)?;
    resilience::register_actions(registry)?;
    Ok(())
}

pub(crate) fn register_behaviors(registry: &mut BehaviorRegistry) -> Result<(), Box<dyn Error>> {
    registry.register(RuleBehavior::new(
        "repaid-support-restores-local-spending",
        ["support_repaid"],
        |_state: &WorldState, event: &Event| {
            if event.actor == Some(JONAS) {
                vec![ActionRequest::new("resume_local_spending").actor(JONAS)]
            } else {
                Vec::new()
            }
        },
    ))?;
    registry.register(RuleBehavior::new(
        "recovered-jonas-buys-bread",
        ["living_cost_paid"],
        |state: &WorldState, event: &Event| {
            if event.actor != Some(JONAS) {
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
            if !integer_component(state, JONAS, CASH)
                .is_ok_and(|cash| cash >= RECOVERED_BREAD_BUDGET)
            {
                return Vec::new();
            }

            vec![ActionRequest::new("buy_bread")
                .actor(JONAS)
                .arg("customer", JONAS)
                .arg("amount", RECOVERED_BREAD_BUDGET)]
        },
    ))?;
    resilience::register_behaviors(registry)?;
    Ok(())
}

struct ResumeLocalSpending;

impl Action for ResumeLocalSpending {
    fn name(&self) -> &'static str {
        "resume_local_spending"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        _request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        if text_component(state, JONAS, SUPPORT_STATUS)? != "repaid" {
            return Err(ActionError::Invalid(
                "Jonas has not completed the support cycle yet".into(),
            ));
        }
        if state
            .entity(JONAS)
            .and_then(|entity| entity.component(LOCAL_SPENDING_STATUS))
            .is_some_and(|status| status == &Value::Text("active".into()))
        {
            return Err(ActionError::Invalid(
                "Jonas has already resumed local spending".into(),
            ));
        }

        let mut draft = EventDraft::new("local_spending_resumed");
        draft.actor = Some(JONAS);
        draft.targets = vec![JONAS, BAKERY];
        draft
            .payload
            .insert("daily_bread_budget".into(), RECOVERED_BREAD_BUDGET.into());
        draft.changes.push(StateChange::SetComponent {
            entity: JONAS,
            key: LOCAL_SPENDING_STATUS.into(),
            value: "active".into(),
        });
        Ok(draft)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{TinySociety, REPAIR_BOAT_COMMAND};

    fn local_spending_status(world: &world_core::World) -> Option<&str> {
        match world
            .state()
            .entity(JONAS)?
            .component(LOCAL_SPENDING_STATUS)?
        {
            Value::Text(status) => Some(status.as_str()),
            _ => None,
        }
    }

    #[test]
    fn restored_income_spills_into_bakery_demand_after_repayment() {
        let mut society = TinySociety::new().unwrap();
        society.run_story().unwrap();
        let mut branch = society.branch();
        branch.advance_days(10).unwrap();
        branch
            .invoke_projection_command(REPAIR_BOAT_COMMAND)
            .unwrap();

        let before_repayment = branch.visit_cursor();
        branch.advance_days(3).unwrap();
        assert_eq!(local_spending_status(branch.world()), None);
        assert!(branch.world().events()[before_repayment.event_count..]
            .iter()
            .all(|event| !(event.kind == "bread_purchased" && event.actor == Some(JONAS))));

        branch.advance_days(1).unwrap();
        let recovery_events = &branch.world().events()[before_repayment.event_count..];
        let repayment = recovery_events
            .iter()
            .find(|event| event.kind == "support_repaid")
            .expect("restored fishing repays Leo before local spending resumes");
        let resumed = recovery_events
            .iter()
            .find(|event| event.kind == "local_spending_resumed")
            .expect("repayment restores Jonas's local spending habit");
        assert_eq!(resumed.caused_by, vec![repayment.id]);
        assert_eq!(
            resumed.payload.get("daily_bread_budget"),
            Some(&Value::Integer(RECOVERED_BREAD_BUDGET))
        );
        assert_eq!(local_spending_status(branch.world()), Some("active"));
        assert!(recovery_events
            .iter()
            .all(|event| !(event.kind == "bread_purchased" && event.actor == Some(JONAS))));

        let cursor = branch.visit_cursor();
        let bakery_before = integer_component(branch.world().state(), BAKERY, CASH).unwrap();
        let jonas_before = integer_component(branch.world().state(), JONAS, CASH).unwrap();

        branch.advance_days(3).unwrap();

        let new_events = &branch.world().events()[cursor.event_count..];
        let purchases = new_events
            .iter()
            .filter(|event| event.kind == "bread_purchased" && event.actor == Some(JONAS))
            .collect::<Vec<_>>();
        assert_eq!(purchases.len(), 3);
        assert!(purchases.iter().all(|event| {
            event.targets.contains(&BAKERY)
                && event.payload.get("amount") == Some(&Value::Integer(RECOVERED_BREAD_BUDGET))
                && event.caused_by.len() == 1
                && branch
                    .world()
                    .event(event.caused_by[0])
                    .is_some_and(|cause| {
                        cause.kind == "living_cost_paid" && cause.actor == Some(JONAS)
                    })
        }));
        assert_eq!(
            integer_component(branch.world().state(), BAKERY, CASH).unwrap(),
            bakery_before + 3 * RECOVERED_BREAD_BUDGET
        );
        assert_eq!(
            integer_component(branch.world().state(), JONAS, CASH).unwrap(),
            jonas_before
                + 3 * (25 - crate::social::JONAS_DAILY_LIVING_COST - RECOVERED_BREAD_BUDGET)
        );

        let briefing = branch
            .projection_snapshot_since(cursor)
            .briefing
            .expect("Tiny Society has a return briefing");
        let bakery_activity = briefing
            .items
            .iter()
            .find(|item| item.title == "Harbor Bakery had customers")
            .expect("recovered Jonas is included in Bakery demand");
        assert!(bakery_activity.detail.contains("Jonas"));
        assert!(bakery_activity.detail.contains("78 revenue"));

        let archive = branch.archive().unwrap();
        let resumed_society = TinySociety::resume_archive(&archive).unwrap();
        let mut resumed_branch = resumed_society.branch();
        assert_eq!(
            local_spending_status(resumed_branch.world()),
            Some("active")
        );
        let resumed_cursor = resumed_branch.visit_cursor();
        resumed_branch.advance_days(1).unwrap();
        assert!(
            resumed_branch.world().events()[resumed_cursor.event_count..]
                .iter()
                .any(|event| event.kind == "bread_purchased" && event.actor == Some(JONAS))
        );
    }
}
