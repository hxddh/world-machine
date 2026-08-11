use crate::{actions::text_component, model::*};
use society_basic::{integer_component, CASH, JOB};
use world_core::{
    Action, ActionError, ActionRegistry, ActionRequest, EventDraft, StateChange, Value, WorldState,
};

pub(crate) const DAILY_CATCH_CRATES: i64 = 1;
pub(crate) const DAILY_CATCH_VALUE: i64 = 35;
pub(crate) const MAINLAND_INITIAL_CASH: i64 = 10_000;
pub(crate) const MAINLAND_CONTRACT_CRATES: i64 = 6;
pub(crate) const MAINLAND_CONTRACT_RENEWAL_FEE: i64 = 50;
pub(crate) const CONTRACT_REMAINING: &str = "contract_remaining";

pub(crate) fn register_actions(registry: &mut ActionRegistry) -> Result<(), ActionError> {
    registry.register(LandCatch)?;
    registry.register(SellFish)?;
    registry.register(RecordMainlandContractFulfilled)?;
    registry.register(RenewMainlandContract)?;
    Ok(())
}

pub(crate) fn contract_remaining(state: &WorldState) -> Option<i64> {
    if let Some(value) = state
        .entity(MAINLAND_MARKET)
        .and_then(|entity| entity.component(CONTRACT_REMAINING))
    {
        return match value {
            Value::Integer(remaining) if *remaining >= 0 => Some(*remaining),
            _ => None,
        };
    }

    let buyer_cash = integer_component(state, MAINLAND_MARKET, CASH).ok()?;
    let spent = MAINLAND_INITIAL_CASH.saturating_sub(buyer_cash).max(0);
    let completed_sales = spent / DAILY_CATCH_VALUE;
    Some((MAINLAND_CONTRACT_CRATES - completed_sales).max(0))
}

struct LandCatch;

impl Action for LandCatch {
    fn name(&self) -> &'static str {
        "land_catch"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        _request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        if text_component(state, JONAS, JOB)? != "fisher" {
            return Err(ActionError::Invalid(
                "Jonas is not currently fishing".into(),
            ));
        }
        if text_component(state, JONAS_BOAT, CONDITION)? != "sound" {
            return Err(ActionError::Invalid("Sea Finch is not seaworthy".into()));
        }
        if state.relation(JONAS_HARBOR_JOB).is_none() {
            return Err(ActionError::Invalid(
                "Jonas has no active Harbor fishing relation".into(),
            ));
        }

        let mut draft = EventDraft::new("catch_landed");
        draft.actor = Some(JONAS);
        draft.targets = vec![JONAS, JONAS_BOAT, HARBOR];
        draft
            .payload
            .insert("crates".into(), DAILY_CATCH_CRATES.into());
        Ok(draft)
    }
}

struct SellFish;

impl Action for SellFish {
    fn name(&self) -> &'static str {
        "sell_fish"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        _request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let remaining = contract_remaining(state)
            .ok_or_else(|| ActionError::Invalid("mainland contract state is invalid".into()))?;
        if remaining <= 0 {
            return Err(ActionError::Invalid(
                "the current mainland fish contract is fulfilled".into(),
            ));
        }
        let buyer_cash = integer_component(state, MAINLAND_MARKET, CASH)?;
        if buyer_cash < DAILY_CATCH_VALUE {
            return Err(ActionError::Invalid(
                "Mainland Fish Market cannot afford today's catch".into(),
            ));
        }
        let harbor_cash = integer_component(state, HARBOR, CASH)?;
        let harbor_after = harbor_cash
            .checked_add(DAILY_CATCH_VALUE)
            .ok_or_else(|| ActionError::Invalid("Harbor cash overflow".into()))?;
        let remaining_after = remaining - 1;

        let mut draft = EventDraft::new("fish_sold");
        draft.actor = Some(JONAS);
        draft.targets = vec![JONAS, HARBOR, MAINLAND_MARKET];
        draft
            .payload
            .insert("crates".into(), DAILY_CATCH_CRATES.into());
        draft
            .payload
            .insert("revenue".into(), DAILY_CATCH_VALUE.into());
        draft
            .payload
            .insert(CONTRACT_REMAINING.into(), remaining_after.into());
        draft.changes = vec![
            StateChange::SetComponent {
                entity: MAINLAND_MARKET,
                key: CASH.into(),
                value: (buyer_cash - DAILY_CATCH_VALUE).into(),
            },
            StateChange::SetComponent {
                entity: MAINLAND_MARKET,
                key: CONTRACT_REMAINING.into(),
                value: remaining_after.into(),
            },
            StateChange::SetComponent {
                entity: HARBOR,
                key: CASH.into(),
                value: harbor_after.into(),
            },
        ];
        Ok(draft)
    }
}

struct RecordMainlandContractFulfilled;

impl Action for RecordMainlandContractFulfilled {
    fn name(&self) -> &'static str {
        "record_mainland_contract_fulfilled"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        _request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        if contract_remaining(state) != Some(0) {
            return Err(ActionError::Invalid(
                "the mainland fish contract still has demand".into(),
            ));
        }
        let mut draft = EventDraft::new("mainland_contract_fulfilled");
        draft.targets = vec![HARBOR, MAINLAND_MARKET];
        Ok(draft)
    }
}

struct RenewMainlandContract;

impl Action for RenewMainlandContract {
    fn name(&self) -> &'static str {
        "renew_mainland_contract"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        _request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        if contract_remaining(state) != Some(0) {
            return Err(ActionError::Invalid(
                "the current mainland fish contract is not fulfilled".into(),
            ));
        }
        let harbor_cash = integer_component(state, HARBOR, CASH)?;
        if harbor_cash < MAINLAND_CONTRACT_RENEWAL_FEE {
            return Err(ActionError::Invalid(format!(
                "Harbor needs {MAINLAND_CONTRACT_RENEWAL_FEE} cash to renew the mainland contract"
            )));
        }
        let buyer_cash = integer_component(state, MAINLAND_MARKET, CASH)?;
        let buyer_after = buyer_cash
            .checked_add(MAINLAND_CONTRACT_RENEWAL_FEE)
            .ok_or_else(|| ActionError::Invalid("Mainland market cash overflow".into()))?;

        let mut draft = EventDraft::new("fish_contract_renewed");
        draft.targets = vec![HARBOR, MAINLAND_MARKET];
        draft
            .payload
            .insert("fee".into(), MAINLAND_CONTRACT_RENEWAL_FEE.into());
        draft.payload.insert(
            "contract_crates".into(),
            MAINLAND_CONTRACT_CRATES.into(),
        );
        draft.changes = vec![
            StateChange::SetComponent {
                entity: HARBOR,
                key: CASH.into(),
                value: (harbor_cash - MAINLAND_CONTRACT_RENEWAL_FEE).into(),
            },
            StateChange::SetComponent {
                entity: MAINLAND_MARKET,
                key: CASH.into(),
                value: buyer_after.into(),
            },
            StateChange::SetComponent {
                entity: MAINLAND_MARKET,
                key: CONTRACT_REMAINING.into(),
                value: MAINLAND_CONTRACT_CRATES.into(),
            },
        ];
        Ok(draft)
    }
}
