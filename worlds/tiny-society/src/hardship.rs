//! What running out of money looks like from outside.
//!
//! The simulation already moved Jonas from 85 cash to 5 over twenty periods,
//! but it recorded nothing along the way: the only events were the daily
//! living cost being paid, which is background hum. A visitor returning to the
//! World was told "the world moved forward" while the one thing actually
//! happening to the person the story is about went unnamed.
//!
//! This module records the crossings. Draining savings is `hardship_began`;
//! no longer being able to cover a day at all is `living_cost_unmet`;
//! climbing back out is `hardship_eased`. Each fires once per spell, so they
//! stay news rather than becoming another daily counter.

use crate::{
    actions::text_component,
    model::{HARDSHIP_STATUS, JONAS},
    social::JONAS_DAILY_LIVING_COST,
};
use society_basic::{integer_component, CASH};
use std::error::Error;
use world_core::{
    Action, ActionError, ActionRegistry, ActionRequest, BehaviorRegistry, Event, EventDraft,
    RuleBehavior, StateChange, WorldState,
};

/// A week of living costs. Below this Jonas is spending savings he cannot
/// replace, which is the moment worth telling someone about — well before he
/// is destitute, and well before he asks Leo for help at
/// [`JONAS_SUPPORT_THRESHOLD`](crate::social::JONAS_SUPPORT_THRESHOLD).
pub(crate) const HARDSHIP_THRESHOLD: i64 = 7 * JONAS_DAILY_LIVING_COST;

pub(crate) const STEADY: &str = "steady";
pub(crate) const STRAINED: &str = "strained";
pub(crate) const DESTITUTE: &str = "destitute";

pub(crate) fn register_actions(registry: &mut ActionRegistry) -> Result<(), ActionError> {
    registry.register(RecordHardship)?;
    registry.register(RecordUnmetLivingCost)?;
    registry.register(RecordHardshipEased)?;
    Ok(())
}

/// The status an entity is in when nothing has been recorded about it yet.
pub(crate) fn hardship_status(state: &WorldState) -> &str {
    text_component(state, JONAS, HARDSHIP_STATUS).unwrap_or(STEADY)
}

/// True when the world should record that Jonas can no longer cover a day.
///
/// The scheduler asks this rather than the action, because a scheduled action
/// that fails aborts the whole advance: the decision has to be made before the
/// request is queued.
pub(crate) fn should_record_unmet_living_cost(state: &WorldState) -> bool {
    let cannot_pay =
        integer_component(state, JONAS, CASH).is_ok_and(|cash| cash < JONAS_DAILY_LIVING_COST);
    cannot_pay && hardship_status(state) != DESTITUTE
}

pub(crate) fn register_behaviors(registry: &mut BehaviorRegistry) -> Result<(), Box<dyn Error>> {
    registry.register(RuleBehavior::new(
        "savings-running-out-is-recorded",
        ["living_cost_paid"],
        |state: &WorldState, event: &Event| {
            if event.actor != Some(JONAS) {
                return Vec::new();
            }
            let Ok(cash) = integer_component(state, JONAS, CASH) else {
                return Vec::new();
            };
            if cash < HARDSHIP_THRESHOLD && hardship_status(state) == STEADY {
                vec![ActionRequest::new("record_hardship").actor(JONAS)]
            } else {
                Vec::new()
            }
        },
    ))?;
    registry.register(RuleBehavior::new(
        "recovered-income-ends-hardship",
        ["fish_sold", "support_received", "work_shift_completed"],
        |state: &WorldState, event: &Event| {
            if event.actor != Some(JONAS) {
                return Vec::new();
            }
            let Ok(cash) = integer_component(state, JONAS, CASH) else {
                return Vec::new();
            };
            if cash >= HARDSHIP_THRESHOLD && hardship_status(state) != STEADY {
                vec![ActionRequest::new("record_hardship_eased").actor(JONAS)]
            } else {
                Vec::new()
            }
        },
    ))?;
    Ok(())
}

struct RecordHardship;

impl Action for RecordHardship {
    fn name(&self) -> &'static str {
        "record_hardship"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        _request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let cash = integer_component(state, JONAS, CASH)?;
        if cash >= HARDSHIP_THRESHOLD {
            return Err(ActionError::Invalid(format!(
                "Jonas still has {cash} cash, which is not yet hardship"
            )));
        }
        if hardship_status(state) != STEADY {
            return Err(ActionError::Invalid(
                "Jonas is already recorded as being in difficulty".into(),
            ));
        }

        let mut draft = EventDraft::new("hardship_began");
        draft.actor = Some(JONAS);
        draft.targets = vec![JONAS];
        draft.payload.insert("cash_remaining".into(), cash.into());
        draft.payload.insert(
            "days_of_cover".into(),
            (cash / JONAS_DAILY_LIVING_COST).into(),
        );
        draft.changes.push(StateChange::SetComponent {
            entity: JONAS,
            key: HARDSHIP_STATUS.into(),
            value: STRAINED.into(),
        });
        Ok(draft)
    }
}

struct RecordUnmetLivingCost;

impl Action for RecordUnmetLivingCost {
    fn name(&self) -> &'static str {
        "record_unmet_living_cost"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        _request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let cash = integer_component(state, JONAS, CASH)?;
        if cash >= JONAS_DAILY_LIVING_COST {
            return Err(ActionError::Invalid(format!(
                "Jonas can still cover today with {cash} cash"
            )));
        }
        if hardship_status(state) == DESTITUTE {
            return Err(ActionError::Invalid(
                "Jonas is already recorded as unable to cover his days".into(),
            ));
        }

        let mut draft = EventDraft::new("living_cost_unmet");
        draft.actor = Some(JONAS);
        draft.targets = vec![JONAS];
        draft.payload.insert("cash_remaining".into(), cash.into());
        draft
            .payload
            .insert("shortfall".into(), (JONAS_DAILY_LIVING_COST - cash).into());
        draft.changes.push(StateChange::SetComponent {
            entity: JONAS,
            key: HARDSHIP_STATUS.into(),
            value: DESTITUTE.into(),
        });
        Ok(draft)
    }
}

struct RecordHardshipEased;

impl Action for RecordHardshipEased {
    fn name(&self) -> &'static str {
        "record_hardship_eased"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        _request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let cash = integer_component(state, JONAS, CASH)?;
        if cash < HARDSHIP_THRESHOLD {
            return Err(ActionError::Invalid(format!(
                "Jonas has {cash} cash, which is not yet back to a week of cover"
            )));
        }
        if hardship_status(state) == STEADY {
            return Err(ActionError::Invalid(
                "Jonas is not currently recorded as being in difficulty".into(),
            ));
        }

        let mut draft = EventDraft::new("hardship_eased");
        draft.actor = Some(JONAS);
        draft.targets = vec![JONAS];
        draft.payload.insert("cash_remaining".into(), cash.into());
        draft.changes.push(StateChange::SetComponent {
            entity: JONAS,
            key: HARDSHIP_STATUS.into(),
            value: STEADY.into(),
        });
        Ok(draft)
    }
}
