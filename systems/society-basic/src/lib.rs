use world_core::{
    Action, ActionError, ActionRegistry, ActionRequest, EntityId, EventDraft, StateChange, Value,
    WorldState,
};

pub const CASH: &str = "cash";
pub const JOB: &str = "job";
pub const EMPLOYER: &str = "employer";

pub fn register_actions(registry: &mut ActionRegistry) -> Result<(), ActionError> {
    registry.register(WorkShift)?;
    registry.register(TransferCash)?;
    Ok(())
}

pub fn integer_component(
    state: &WorldState,
    entity: EntityId,
    key: &str,
) -> Result<i64, ActionError> {
    match state.entity(entity).and_then(|item| item.component(key)) {
        Some(Value::Integer(value)) => Ok(*value),
        _ => Err(ActionError::Invalid(format!(
            "entity {entity} has no integer component {key}"
        ))),
    }
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

pub struct WorkShift;

impl Action for WorkShift {
    fn name(&self) -> &'static str {
        "work_shift"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let worker = entity_arg(request, "worker")?;
        let workplace = entity_arg(request, "workplace")?;
        let wage = positive_integer_arg(request, "wage")?;
        let worker_cash = integer_component(state, worker, CASH)?;
        let workplace_cash = integer_component(state, workplace, CASH)?;
        if workplace_cash < wage {
            return Err(ActionError::Invalid(format!(
                "workplace {workplace} cannot pay wage {wage}"
            )));
        }

        let mut draft = EventDraft::new("work_shift_completed");
        draft.actor = Some(worker);
        draft.targets = vec![worker, workplace];
        draft.payload.insert("wage".into(), wage.into());
        draft.changes = vec![
            StateChange::SetComponent {
                entity: worker,
                key: CASH.into(),
                value: (worker_cash + wage).into(),
            },
            StateChange::SetComponent {
                entity: workplace,
                key: CASH.into(),
                value: (workplace_cash - wage).into(),
            },
        ];
        Ok(draft)
    }
}

pub struct TransferCash;

impl Action for TransferCash {
    fn name(&self) -> &'static str {
        "transfer_cash"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let from = entity_arg(request, "from")?;
        let to = entity_arg(request, "to")?;
        let amount = positive_integer_arg(request, "amount")?;
        let from_cash = integer_component(state, from, CASH)?;
        let to_cash = integer_component(state, to, CASH)?;
        if from_cash < amount {
            return Err(ActionError::Invalid(format!(
                "entity {from} cannot transfer {amount}"
            )));
        }

        let mut draft = EventDraft::new("cash_transferred");
        draft.actor = Some(from);
        draft.targets = vec![from, to];
        draft.payload.insert("amount".into(), amount.into());
        draft.changes = vec![
            StateChange::SetComponent {
                entity: from,
                key: CASH.into(),
                value: (from_cash - amount).into(),
            },
            StateChange::SetComponent {
                entity: to,
                key: CASH.into(),
                value: (to_cash + amount).into(),
            },
        ];
        Ok(draft)
    }
}
