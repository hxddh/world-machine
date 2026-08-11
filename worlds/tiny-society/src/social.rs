use crate::{actions::text_component, model::*};
use society_basic::{integer_component, CASH, EMPLOYER, JOB};
use world_core::{
    Action, ActionError, ActionRegistry, ActionRequest, EntityId, EventDraft, Relation,
    StateChange, Value, WorldState,
};

pub(crate) const JONAS_DAILY_LIVING_COST: i64 = 8;
pub(crate) const JONAS_SUPPORT_THRESHOLD: i64 = 20;
pub(crate) const LEO_SUPPORT_AMOUNT: i64 = 40;
pub(crate) const LEO_SUPPORT_TRUST_GAIN: i64 = 8;
pub(crate) const SEA_FINCH_REPAIR_COST: i64 = 50;
pub(crate) const SEA_FINCH_REPAIR_TRUST: i64 = 84;

pub(crate) fn register_actions(registry: &mut ActionRegistry) -> Result<(), ActionError> {
    registry.register(PayLivingCost)?;
    registry.register(RequestSupport)?;
    registry.register(ProvideSupport)?;
    registry.register(RepairJonasBoat)?;
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

struct PayLivingCost;

impl Action for PayLivingCost {
    fn name(&self) -> &'static str {
        "pay_living_cost"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let resident = entity_arg(request, "resident")?;
        let amount = positive_integer_arg(request, "amount")?;
        let cash = integer_component(state, resident, CASH)?;
        if cash < amount {
            return Err(ActionError::Invalid(format!(
                "resident {resident} cannot cover living cost {amount}"
            )));
        }

        let mut draft = EventDraft::new("living_cost_paid");
        draft.actor = Some(resident);
        draft.targets = vec![resident];
        draft.payload.insert("amount".into(), amount.into());
        draft.changes.push(StateChange::SetComponent {
            entity: resident,
            key: CASH.into(),
            value: (cash - amount).into(),
        });
        Ok(draft)
    }
}

struct RequestSupport;

impl Action for RequestSupport {
    fn name(&self) -> &'static str {
        "request_support"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let resident = entity_arg(request, "resident")?;
        let supporter = entity_arg(request, "supporter")?;
        if resident != JONAS || supporter != LEO {
            return Err(ActionError::Invalid(
                "Tiny Society currently models this support path only for Jonas and Leo".into(),
            ));
        }
        if text_component(state, JONAS, SUPPORT_STATUS)? != "none" {
            return Err(ActionError::Invalid(
                "Jonas has already activated his support network".into(),
            ));
        }
        let cash = integer_component(state, JONAS, CASH)?;
        if cash > JONAS_SUPPORT_THRESHOLD {
            return Err(ActionError::Invalid(format!(
                "Jonas still has {cash} cash and does not need support yet"
            )));
        }

        let mut draft = EventDraft::new("support_requested");
        draft.actor = Some(JONAS);
        draft.targets = vec![JONAS, LEO];
        draft.payload.insert("cash_available".into(), cash.into());
        draft.changes.push(StateChange::SetComponent {
            entity: JONAS,
            key: SUPPORT_STATUS.into(),
            value: "requested".into(),
        });
        Ok(draft)
    }
}

struct ProvideSupport;

impl Action for ProvideSupport {
    fn name(&self) -> &'static str {
        "provide_support"
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
                "Tiny Society currently models this support path only for Jonas and Leo".into(),
            ));
        }
        if text_component(state, JONAS, SUPPORT_STATUS)? != "requested" {
            return Err(ActionError::Invalid(
                "Jonas has not requested support".into(),
            ));
        }

        let leo_cash = integer_component(state, LEO, CASH)?;
        if leo_cash < amount {
            return Err(ActionError::Invalid(format!(
                "Leo cannot provide support amount {amount}"
            )));
        }
        let jonas_cash = integer_component(state, JONAS, CASH)?;
        let jonas_after = jonas_cash
            .checked_add(amount)
            .ok_or_else(|| ActionError::Invalid("Jonas cash overflow".into()))?;
        let trust_before = jonas_leo_trust(state)?;
        let trust_after = trust_before.saturating_add(LEO_SUPPORT_TRUST_GAIN).min(100);

        let mut draft = EventDraft::new("support_received");
        draft.actor = Some(LEO);
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
                entity: LEO,
                key: CASH.into(),
                value: (leo_cash - amount).into(),
            },
            StateChange::SetComponent {
                entity: JONAS,
                key: CASH.into(),
                value: jonas_after.into(),
            },
            StateChange::SetRelationProperty {
                relation: JONAS_LEO_TRUST,
                key: "trust".into(),
                value: trust_after.into(),
            },
            StateChange::SetComponent {
                entity: JONAS,
                key: SUPPORT_STATUS.into(),
                value: "received".into(),
            },
        ];
        Ok(draft)
    }
}

struct RepairJonasBoat;

impl Action for RepairJonasBoat {
    fn name(&self) -> &'static str {
        "repair_jonas_boat"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        _request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        if text_component(state, JONAS_BOAT, CONDITION)? != "damaged" {
            return Err(ActionError::Invalid("Sea Finch is not damaged".into()));
        }
        if text_component(state, JONAS, JOB)? != "unemployed" {
            return Err(ActionError::Invalid(
                "Jonas must be unemployed before returning to fishing".into(),
            ));
        }
        if text_component(state, JONAS, SUPPORT_STATUS)? != "received" {
            return Err(ActionError::Invalid(
                "Jonas has not yet activated Leo's support".into(),
            ));
        }
        if state.relation(JONAS_HARBOR_JOB).is_some() {
            return Err(ActionError::Invalid(
                "Jonas already has an active Harbor job relation".into(),
            ));
        }

        let trust = jonas_leo_trust(state)?;
        if trust < SEA_FINCH_REPAIR_TRUST {
            return Err(ActionError::Invalid(format!(
                "Jonas and Leo need trust {SEA_FINCH_REPAIR_TRUST} to finance the repair"
            )));
        }
        let leo_cash = integer_component(state, LEO, CASH)?;
        if leo_cash < SEA_FINCH_REPAIR_COST {
            return Err(ActionError::Invalid(format!(
                "Leo needs {SEA_FINCH_REPAIR_COST} cash to finance the repair"
            )));
        }
        let evan_cash = integer_component(state, EVAN, CASH)?;
        let evan_after = evan_cash
            .checked_add(SEA_FINCH_REPAIR_COST)
            .ok_or_else(|| ActionError::Invalid("Evan cash overflow".into()))?;

        let mut draft = EventDraft::new("boat_repaired");
        draft.actor = Some(LEO);
        draft.targets = vec![JONAS, LEO, EVAN, JONAS_BOAT, HARBOR];
        draft
            .payload
            .insert("repair_cost".into(), SEA_FINCH_REPAIR_COST.into());
        draft
            .payload
            .insert("trust_required".into(), SEA_FINCH_REPAIR_TRUST.into());
        draft.payload.insert("trust_observed".into(), trust.into());
        draft.changes = vec![
            StateChange::SetComponent {
                entity: LEO,
                key: CASH.into(),
                value: (leo_cash - SEA_FINCH_REPAIR_COST).into(),
            },
            StateChange::SetComponent {
                entity: EVAN,
                key: CASH.into(),
                value: evan_after.into(),
            },
            StateChange::SetComponent {
                entity: JONAS_BOAT,
                key: CONDITION.into(),
                value: "sound".into(),
            },
            StateChange::CreateRelation(Relation::new(JONAS_HARBOR_JOB, "works_at", JONAS, HARBOR)),
            StateChange::SetComponent {
                entity: JONAS,
                key: JOB.into(),
                value: "fisher".into(),
            },
            StateChange::SetComponent {
                entity: JONAS,
                key: EMPLOYER.into(),
                value: HARBOR.into(),
            },
        ];
        Ok(draft)
    }
}
