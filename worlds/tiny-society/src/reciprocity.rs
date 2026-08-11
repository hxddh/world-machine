use crate::{
    actions::text_component,
    model::{HARBOR, JONAS, JONAS_LEO_TRUST, LEO, SUPPORT_STATUS},
    social::LEO_SUPPORT_AMOUNT,
};
use society_basic::{integer_component, CASH};
use std::error::Error;
use world_core::{
    Action, ActionError, ActionRegistry, ActionRequest, BehaviorRegistry, EntityId, Event,
    EventDraft, RuleBehavior, StateChange, Value, WorldState,
};

pub(crate) const JONAS_REPAYMENT_CASH_THRESHOLD: i64 = 100;
pub(crate) const RECIPROCITY_TRUST_GAIN: i64 = 4;

pub(crate) fn register_actions(registry: &mut ActionRegistry) -> Result<(), ActionError> {
    registry.register(RepaySupport)?;
    Ok(())
}

pub(crate) fn register_behaviors(registry: &mut BehaviorRegistry) -> Result<(), Box<dyn Error>> {
    registry.register(RuleBehavior::new(
        "restored-fishing-repays-leo-support",
        ["fish_sold"],
        |state: &WorldState, event: &Event| {
            if event.actor != Some(JONAS) || !event.targets.contains(&HARBOR) {
                return Vec::new();
            }
            let cash = integer_component(state, JONAS, CASH).ok();
            let status = state
                .entity(JONAS)
                .and_then(|entity| entity.component(SUPPORT_STATUS));
            match (cash, status) {
                (Some(cash), Some(Value::Text(status)))
                    if cash >= JONAS_REPAYMENT_CASH_THRESHOLD && status == "received" =>
                {
                    vec![ActionRequest::new("repay_support")
                        .actor(JONAS)
                        .arg("resident", JONAS)
                        .arg("supporter", LEO)
                        .arg("amount", LEO_SUPPORT_AMOUNT)]
                }
                _ => Vec::new(),
            }
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

fn positive_integer_arg(request: &ActionRequest, name: &str) -> Result<i64, ActionError> {
    match request.args.get(name) {
        Some(Value::Integer(value)) if *value > 0 => Ok(*value),
        _ => Err(ActionError::Invalid(format!(
            "{name} must be a positive integer"
        ))),
    }
}

fn jonas_leo_trust(state: &WorldState) -> Result<i64, ActionError> {
    let relation = state
        .relation(JONAS_LEO_TRUST)
        .ok_or_else(|| ActionError::Invalid("Jonas and Leo have no trust relation".into()))?;
    match relation.properties.get("trust") {
        Some(Value::Integer(value)) => Ok(*value),
        _ => Err(ActionError::Invalid(
            "Jonas and Leo trust relation has no integer trust score".into(),
        )),
    }
}

struct RepaySupport;

impl Action for RepaySupport {
    fn name(&self) -> &'static str {
        "repay_support"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let resident = entity_arg(request, "resident")?;
        let supporter = entity_arg(request, "supporter")?;
        let amount = positive_integer_arg(request, "amount")?;
        if resident != JONAS || supporter != LEO {
            return Err(ActionError::Invalid(
                "Tiny Society currently models reciprocal support only for Jonas and Leo".into(),
            ));
        }
        if amount != LEO_SUPPORT_AMOUNT {
            return Err(ActionError::Invalid(format!(
                "Jonas must repay the original support amount {LEO_SUPPORT_AMOUNT}"
            )));
        }
        if text_component(state, JONAS, SUPPORT_STATUS)? != "received" {
            return Err(ActionError::Invalid(
                "Jonas has no outstanding received support to repay".into(),
            ));
        }

        let jonas_cash = integer_component(state, JONAS, CASH)?;
        if jonas_cash < JONAS_REPAYMENT_CASH_THRESHOLD {
            return Err(ActionError::Invalid(format!(
                "Jonas needs at least {JONAS_REPAYMENT_CASH_THRESHOLD} cash before repaying support"
            )));
        }
        let leo_cash = integer_component(state, LEO, CASH)?;
        let leo_after = leo_cash
            .checked_add(amount)
            .ok_or_else(|| ActionError::Invalid("Leo cash overflow".into()))?;
        let trust_before = jonas_leo_trust(state)?;
        let trust_after = trust_before.saturating_add(RECIPROCITY_TRUST_GAIN).min(100);

        let mut draft = EventDraft::new("support_repaid");
        draft.actor = Some(JONAS);
        draft.targets = vec![JONAS, LEO];
        draft.payload.insert("amount".into(), amount.into());
        draft
            .payload
            .insert("trust_before".into(), trust_before.into());
        draft
            .payload
            .insert("trust_after".into(), trust_after.into());
        draft.changes = vec![
            StateChange::SetComponent {
                entity: JONAS,
                key: CASH.into(),
                value: (jonas_cash - amount).into(),
            },
            StateChange::SetComponent {
                entity: LEO,
                key: CASH.into(),
                value: leo_after.into(),
            },
            StateChange::SetRelationProperty {
                relation: JONAS_LEO_TRUST,
                key: "trust".into(),
                value: trust_after.into(),
            },
            StateChange::SetComponent {
                entity: JONAS,
                key: SUPPORT_STATUS.into(),
                value: "repaid".into(),
            },
        ];
        Ok(draft)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{TinySociety, REPAIR_BOAT_COMMAND};
    use world_core::World;

    fn support_status(world: &World) -> Option<&str> {
        match world.state().entity(JONAS)?.component(SUPPORT_STATUS)? {
            Value::Text(status) => Some(status.as_str()),
            _ => None,
        }
    }

    fn trust(world: &World) -> Option<i64> {
        match world
            .state()
            .relation(JONAS_LEO_TRUST)?
            .properties
            .get("trust")?
        {
            Value::Integer(value) => Some(*value),
            _ => None,
        }
    }

    #[test]
    fn restored_fishing_repays_leo_once_and_persists_reciprocity() {
        let mut society = TinySociety::new().unwrap();
        society.run_story().unwrap();
        let mut branch = society.branch();
        branch.advance_days(10).unwrap();
        assert_eq!(support_status(branch.world()), Some("received"));
        assert_eq!(trust(branch.world()), Some(84));

        branch
            .invoke_projection_command(REPAIR_BOAT_COMMAND)
            .unwrap();
        let cursor = branch.visit_cursor();
        let jonas_before = integer_component(branch.world().state(), JONAS, CASH).unwrap();
        let leo_before = integer_component(branch.world().state(), LEO, CASH).unwrap();

        branch.advance_days(3).unwrap();
        assert_eq!(support_status(branch.world()), Some("received"));
        assert!(branch.world().events()[cursor.event_count..]
            .iter()
            .all(|event| event.kind != "support_repaid"));

        branch.advance_days(1).unwrap();
        let new_events = &branch.world().events()[cursor.event_count..];
        let repayment = new_events
            .iter()
            .find(|event| event.kind == "support_repaid")
            .expect("restored fishing eventually repays Leo");
        assert_eq!(repayment.actor, Some(JONAS));
        assert_eq!(repayment.targets, vec![JONAS, LEO]);
        assert_eq!(repayment.payload.get("amount"), Some(&Value::Integer(40)));
        assert_eq!(
            repayment.payload.get("trust_before"),
            Some(&Value::Integer(84))
        );
        assert_eq!(
            repayment.payload.get("trust_after"),
            Some(&Value::Integer(88))
        );
        assert_eq!(repayment.caused_by.len(), 1);
        assert_eq!(
            branch
                .world()
                .event(repayment.caused_by[0])
                .expect("repayment cause remains in history")
                .kind,
            "fish_sold"
        );
        assert_eq!(support_status(branch.world()), Some("repaid"));
        assert_eq!(trust(branch.world()), Some(88));
        assert_eq!(
            integer_component(branch.world().state(), JONAS, CASH).unwrap(),
            jonas_before + 4 * (25 - crate::social::JONAS_DAILY_LIVING_COST) - LEO_SUPPORT_AMOUNT
        );
        assert_eq!(
            integer_component(branch.world().state(), LEO, CASH).unwrap(),
            leo_before + 4 * (22 - 11) + LEO_SUPPORT_AMOUNT
        );

        let briefing = branch
            .projection_snapshot_since(cursor)
            .briefing
            .expect("Tiny Society has a return briefing");
        assert!(briefing
            .items
            .iter()
            .any(|item| item.title == "Jonas repaid Leo after returning to sea"));

        let archive = branch.archive().unwrap();
        let resumed = TinySociety::resume_archive(&archive).unwrap();
        assert_eq!(support_status(resumed.world()), Some("repaid"));
        assert_eq!(trust(resumed.world()), Some(88));

        let repayment_count = branch
            .world()
            .events()
            .iter()
            .filter(|event| event.kind == "support_repaid")
            .count();
        branch.advance_days(2).unwrap();
        assert_eq!(
            branch
                .world()
                .events()
                .iter()
                .filter(|event| event.kind == "support_repaid")
                .count(),
            repayment_count
        );
    }
}
