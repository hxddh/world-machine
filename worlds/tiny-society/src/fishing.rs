use crate::{actions::text_component, model::*};
use society_basic::{integer_component, CASH, JOB};
use world_core::{
    Action, ActionError, ActionRegistry, ActionRequest, EventDraft, StateChange, WorldState,
};

pub(crate) const DAILY_CATCH_CRATES: i64 = 1;
pub(crate) const DAILY_CATCH_VALUE: i64 = 35;

pub(crate) fn register_actions(registry: &mut ActionRegistry) -> Result<(), ActionError> {
    registry.register(LandCatch)?;
    registry.register(SellFish)?;
    Ok(())
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
            return Err(ActionError::Invalid("Jonas is not currently fishing".into()));
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

        let mut draft = EventDraft::new("fish_sold");
        draft.actor = Some(JONAS);
        draft.targets = vec![JONAS, HARBOR, MAINLAND_MARKET];
        draft
            .payload
            .insert("crates".into(), DAILY_CATCH_CRATES.into());
        draft
            .payload
            .insert("revenue".into(), DAILY_CATCH_VALUE.into());
        draft.changes = vec![
            StateChange::SetComponent {
                entity: MAINLAND_MARKET,
                key: CASH.into(),
                value: (buyer_cash - DAILY_CATCH_VALUE).into(),
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
