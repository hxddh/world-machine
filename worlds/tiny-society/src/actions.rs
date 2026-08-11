use crate::model::*;
use society_basic::{integer_component, CASH, EMPLOYER, JOB};
use world_core::{
    Action, ActionError, ActionRegistry, ActionRequest, EntityId, EventDraft, Relation,
    StateChange, Value, WorldState,
};

pub(crate) fn register(registry: &mut ActionRegistry) -> Result<(), ActionError> {
    registry.register(StormArrives)?;
    registry.register(DamageBoat)?;
    registry.register(RecordIncomeLoss)?;
    registry.register(RequestLoan)?;
    registry.register(AssignTemporaryWork)?;
    registry.register(DeclineTemporaryWork)?;
    registry.register(MissShift)?;
    registry.register(LoseOrder)?;
    registry.register(DismissWorker)?;
    registry.register(BuyBread)?;
    registry.register(RecordPayrollShortfall)?;
    registry.register(CloseBakery)?;
    registry.register(ReopenBakery)?;
    Ok(())
}

pub(crate) fn text_component<'a>(
    state: &'a WorldState,
    entity: EntityId,
    key: &str,
) -> Result<&'a str, ActionError> {
    match state.entity(entity).and_then(|item| item.component(key)) {
        Some(Value::Text(value)) => Ok(value),
        _ => Err(ActionError::Invalid(format!(
            "entity {entity} has no text component {key}"
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

struct StormArrives;

impl Action for StormArrives {
    fn name(&self) -> &'static str {
        "storm_arrives"
    }

    fn evaluate(
        &self,
        _state: &WorldState,
        _request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let mut draft = EventDraft::new("storm_started");
        draft.targets = vec![HARBOR];
        draft.changes.push(StateChange::SetComponent {
            entity: HARBOR,
            key: WEATHER.into(),
            value: "storm".into(),
        });
        Ok(draft)
    }
}

struct DamageBoat;

impl Action for DamageBoat {
    fn name(&self) -> &'static str {
        "damage_boat"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        _request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        if text_component(state, JONAS_BOAT, CONDITION)? != "sound" {
            return Err(ActionError::Invalid(
                "Jonas' boat is already damaged".into(),
            ));
        }
        let mut draft = EventDraft::new("boat_damaged");
        draft.targets = vec![JONAS_BOAT, JONAS];
        draft.changes.push(StateChange::SetComponent {
            entity: JONAS_BOAT,
            key: CONDITION.into(),
            value: "damaged".into(),
        });
        Ok(draft)
    }
}

struct RecordIncomeLoss;

impl Action for RecordIncomeLoss {
    fn name(&self) -> &'static str {
        "record_income_loss"
    }

    fn evaluate(
        &self,
        _state: &WorldState,
        _request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let mut draft = EventDraft::new("income_lost");
        draft.actor = Some(JONAS);
        draft.targets = vec![JONAS];
        draft.changes.push(StateChange::SetComponent {
            entity: JONAS,
            key: INCOME_STATUS.into(),
            value: "lost".into(),
        });
        Ok(draft)
    }
}

struct RequestLoan;

impl Action for RequestLoan {
    fn name(&self) -> &'static str {
        "request_loan"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        if text_component(state, JONAS, INCOME_STATUS)? != "lost" {
            return Err(ActionError::Invalid("income is not lost".into()));
        }
        let lender = match request.args.get("lender") {
            Some(Value::Entity(id)) => *id,
            _ => return Err(ActionError::Invalid("missing lender".into())),
        };
        let amount = match request.args.get("amount") {
            Some(Value::Integer(value)) if *value > 0 => *value,
            _ => return Err(ActionError::Invalid("invalid loan amount".into())),
        };

        let mut draft = EventDraft::new("loan_requested");
        draft.actor = Some(JONAS);
        draft.targets = vec![JONAS, lender];
        draft.payload.insert("amount".into(), amount.into());
        draft.changes.push(StateChange::SetComponent {
            entity: JONAS,
            key: LOAN_STATUS.into(),
            value: "requested".into(),
        });
        Ok(draft)
    }
}

struct AssignTemporaryWork;

impl Action for AssignTemporaryWork {
    fn name(&self) -> &'static str {
        "assign_temporary_work"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        _request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        if text_component(state, JONAS, LOAN_STATUS)? != "requested" {
            return Err(ActionError::Invalid("Jonas did not request help".into()));
        }
        let mut draft = EventDraft::new("temporary_work_assigned");
        draft.actor = Some(MARA);
        draft.targets = vec![JONAS, BAKERY];
        draft.changes = vec![
            StateChange::RemoveRelation(JONAS_HARBOR_JOB),
            StateChange::CreateRelation(Relation::new(TEMP_BAKERY_JOB, "works_at", JONAS, BAKERY)),
            StateChange::SetComponent {
                entity: JONAS,
                key: JOB.into(),
                value: "bakery_temp".into(),
            },
            StateChange::SetComponent {
                entity: JONAS,
                key: EMPLOYER.into(),
                value: BAKERY.into(),
            },
        ];
        Ok(draft)
    }
}

struct DeclineTemporaryWork;

impl Action for DeclineTemporaryWork {
    fn name(&self) -> &'static str {
        "decline_temporary_work"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        _request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        if text_component(state, JONAS, LOAN_STATUS)? != "requested" {
            return Err(ActionError::Invalid("Jonas did not request help".into()));
        }
        let mut draft = EventDraft::new("temporary_work_declined");
        draft.actor = Some(MARA);
        draft.targets = vec![JONAS, BAKERY];
        draft.changes.push(StateChange::SetComponent {
            entity: JONAS,
            key: LOAN_STATUS.into(),
            value: "help_declined".into(),
        });
        Ok(draft)
    }
}

struct MissShift;

impl Action for MissShift {
    fn name(&self) -> &'static str {
        "miss_shift"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        _request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        if text_component(state, JONAS, JOB)? != "bakery_temp" {
            return Err(ActionError::Invalid(
                "Jonas is not a temporary bakery worker".into(),
            ));
        }
        let missed = integer_component(state, JONAS, MISSED_SHIFTS)?;
        let mut draft = EventDraft::new("shift_missed");
        draft.actor = Some(JONAS);
        draft.targets = vec![JONAS, BAKERY];
        draft.changes.push(StateChange::SetComponent {
            entity: JONAS,
            key: MISSED_SHIFTS.into(),
            value: (missed + 1).into(),
        });
        Ok(draft)
    }
}

struct LoseOrder;

impl Action for LoseOrder {
    fn name(&self) -> &'static str {
        "lose_order"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        _request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        if text_component(state, WEDDING_ORDER, ORDER_STATUS)? != "pending" {
            return Err(ActionError::Invalid("order is not pending".into()));
        }
        let bakery_cash = integer_component(state, BAKERY, CASH)?;
        let loss = 80_i64;
        if bakery_cash < loss {
            return Err(ActionError::Invalid(
                "bakery cannot absorb order loss".into(),
            ));
        }

        let mut draft = EventDraft::new("order_lost");
        draft.actor = Some(MARA);
        draft.targets = vec![WEDDING_ORDER, BAKERY];
        draft.payload.insert("loss".into(), loss.into());
        draft.changes = vec![
            StateChange::SetComponent {
                entity: WEDDING_ORDER,
                key: ORDER_STATUS.into(),
                value: "lost".into(),
            },
            StateChange::SetComponent {
                entity: BAKERY,
                key: CASH.into(),
                value: (bakery_cash - loss).into(),
            },
        ];
        Ok(draft)
    }
}

struct DismissWorker;

impl Action for DismissWorker {
    fn name(&self) -> &'static str {
        "dismiss_worker"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        _request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        if text_component(state, WEDDING_ORDER, ORDER_STATUS)? != "lost" {
            return Err(ActionError::Invalid(
                "the bakery has not lost the order".into(),
            ));
        }
        if state.relation(TEMP_BAKERY_JOB).is_none() {
            return Err(ActionError::Invalid("temporary job does not exist".into()));
        }

        let mut draft = EventDraft::new("worker_dismissed");
        draft.actor = Some(MARA);
        draft.targets = vec![JONAS, BAKERY];
        draft.changes = vec![
            StateChange::RemoveRelation(TEMP_BAKERY_JOB),
            StateChange::SetComponent {
                entity: JONAS,
                key: JOB.into(),
                value: "unemployed".into(),
            },
            StateChange::RemoveComponent {
                entity: JONAS,
                key: EMPLOYER.into(),
            },
        ];
        Ok(draft)
    }
}

struct BuyBread;

impl Action for BuyBread {
    fn name(&self) -> &'static str {
        "buy_bread"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        if text_component(state, BAKERY, OPERATING_STATUS)? != "open" {
            return Err(ActionError::Invalid("the bakery is closed".into()));
        }
        let customer = entity_arg(request, "customer")?;
        let amount = positive_integer_arg(request, "amount")?;
        let customer_cash = integer_component(state, customer, CASH)?;
        if customer_cash < amount {
            return Err(ActionError::Invalid(format!(
                "customer {customer} cannot afford bread costing {amount}"
            )));
        }
        let bakery_cash = integer_component(state, BAKERY, CASH)?;

        let mut draft = EventDraft::new("bread_purchased");
        draft.actor = Some(customer);
        draft.targets = vec![customer, BAKERY];
        draft.payload.insert("amount".into(), amount.into());
        draft.changes = vec![
            StateChange::SetComponent {
                entity: customer,
                key: CASH.into(),
                value: (customer_cash - amount).into(),
            },
            StateChange::SetComponent {
                entity: BAKERY,
                key: CASH.into(),
                value: (bakery_cash + amount).into(),
            },
        ];
        Ok(draft)
    }
}

struct RecordPayrollShortfall;

impl Action for RecordPayrollShortfall {
    fn name(&self) -> &'static str {
        "record_payroll_shortfall"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let worker = entity_arg(request, "worker")?;
        let workplace = entity_arg(request, "workplace")?;
        let wage = positive_integer_arg(request, "wage")?;
        if workplace != BAKERY {
            return Err(ActionError::Invalid(
                "Tiny Society currently models payroll crisis only for the bakery".into(),
            ));
        }
        if text_component(state, BAKERY, OPERATING_STATUS)? != "open" {
            return Err(ActionError::Invalid("the bakery is already closed".into()));
        }
        let cash_available = integer_component(state, workplace, CASH)?;
        if cash_available >= wage {
            return Err(ActionError::Invalid(format!(
                "workplace {workplace} can still cover wage {wage}"
            )));
        }

        let mut draft = EventDraft::new("payroll_shortfall");
        draft.targets = vec![worker, workplace];
        draft.payload.insert("required_wage".into(), wage.into());
        draft
            .payload
            .insert("cash_available".into(), cash_available.into());
        Ok(draft)
    }
}

struct CloseBakery;

impl Action for CloseBakery {
    fn name(&self) -> &'static str {
        "close_bakery"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        _request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        if text_component(state, BAKERY, OPERATING_STATUS)? != "open" {
            return Err(ActionError::Invalid("the bakery is already closed".into()));
        }

        let mut draft = EventDraft::new("bakery_closed");
        draft.actor = Some(MARA);
        draft.targets = vec![BAKERY, MARA];
        draft.changes = vec![
            StateChange::SetComponent {
                entity: BAKERY,
                key: OPERATING_STATUS.into(),
                value: "closed".into(),
            },
            StateChange::RemoveRelation(MARA_BAKERY_JOB),
            StateChange::SetComponent {
                entity: MARA,
                key: JOB.into(),
                value: "bakery_closed".into(),
            },
        ];

        if state.relation(TEMP_BAKERY_JOB).is_some() {
            draft.targets.push(JONAS);
            draft.changes.extend([
                StateChange::RemoveRelation(TEMP_BAKERY_JOB),
                StateChange::SetComponent {
                    entity: JONAS,
                    key: JOB.into(),
                    value: "unemployed".into(),
                },
                StateChange::RemoveComponent {
                    entity: JONAS,
                    key: EMPLOYER.into(),
                },
            ]);
        }
        Ok(draft)
    }
}

struct ReopenBakery;

impl Action for ReopenBakery {
    fn name(&self) -> &'static str {
        "reopen_bakery"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        _request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        if text_component(state, BAKERY, OPERATING_STATUS)? != "closed" {
            return Err(ActionError::Invalid("the bakery is not closed".into()));
        }
        if state.relation(MARA_BAKERY_JOB).is_some() {
            return Err(ActionError::Invalid(
                "Mara already has an active bakery job relation".into(),
            ));
        }

        let investment = crate::BAKERY_REOPEN_INVESTMENT;
        let mara_cash = integer_component(state, MARA, CASH)?;
        if mara_cash < investment {
            return Err(ActionError::Invalid(format!(
                "Mara needs {investment} cash to reopen the bakery"
            )));
        }
        let bakery_cash = integer_component(state, BAKERY, CASH)?;

        let mut draft = EventDraft::new("bakery_reopened");
        draft.actor = Some(MARA);
        draft.targets = vec![BAKERY, MARA];
        draft.payload.insert("investment".into(), investment.into());
        draft.changes = vec![
            StateChange::SetComponent {
                entity: MARA,
                key: CASH.into(),
                value: (mara_cash - investment).into(),
            },
            StateChange::SetComponent {
                entity: BAKERY,
                key: CASH.into(),
                value: (bakery_cash + investment).into(),
            },
            StateChange::SetComponent {
                entity: BAKERY,
                key: OPERATING_STATUS.into(),
                value: "open".into(),
            },
            StateChange::CreateRelation(Relation::new(MARA_BAKERY_JOB, "works_at", MARA, BAKERY)),
            StateChange::SetComponent {
                entity: MARA,
                key: JOB.into(),
                value: "baker".into(),
            },
        ];
        Ok(draft)
    }
}
