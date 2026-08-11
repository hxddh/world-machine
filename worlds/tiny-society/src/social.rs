use crate::{actions::text_component, model::*};
use society_basic::{integer_component, CASH};
use world_core::{
    Action, ActionError, ActionRegistry, ActionRequest, EntityId, EventDraft, StateChange, Value,
    WorldState,
};

pub(crate) const JONAS_DAILY_LIVING_COST: i64 = 8;
pub(crate) const JONAS_SUPPORT_THRESHOLD: i64 = 20;
pub(crate) const LEO_SUPPORT_AMOUNT: i64 = 40;
pub(crate) const LEO_SUPPORT_TRUST_GAIN: i64 = 8;

pub(crate) fn register_actions(registry: &mut ActionRegistry) -> Result<(), ActionError> {
    registry.register(PayLivingCost)?;
    registry.register(RequestSupport)?;
    registry.register(ProvideSupport)?;
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
        let relation = state
            .relation(JONAS_LEO_TRUST)
            .ok_or_else(|| ActionError::Invalid("Jonas and Leo have no trust relation".into()))?;
        let trust_before = match relation.properties.get("trust") {
            Some(Value::Integer(value)) => *value,
            _ => {
                return Err(ActionError::Invalid(
                    "Jonas and Leo trust relation has no integer trust score".into(),
                ));
            }
        };
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
