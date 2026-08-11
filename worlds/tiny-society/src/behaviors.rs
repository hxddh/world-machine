use crate::{
    fishing::DAILY_CATCH_VALUE,
    model::{BAKERY, HARBOR, JONAS, LEO, MAINLAND_MARKET, MARA, SUPPORT_STATUS},
    social::{JONAS_SUPPORT_THRESHOLD, LEO_SUPPORT_AMOUNT},
};
use society_basic::{integer_component, CASH};
use std::error::Error;
use world_core::{ActionRequest, BehaviorRegistry, Event, RuleBehavior, Value, WorldState};

pub(crate) fn register(registry: &mut BehaviorRegistry) -> Result<(), Box<dyn Error>> {
    registry.register(RuleBehavior::new(
        "storm-damages-boat",
        ["storm_started"],
        |_state: &WorldState, _event: &Event| vec![ActionRequest::new("damage_boat")],
    ))?;
    registry.register(RuleBehavior::new(
        "damage-removes-income",
        ["boat_damaged"],
        |_state: &WorldState, _event: &Event| {
            vec![ActionRequest::new("record_income_loss").actor(JONAS)]
        },
    ))?;
    registry.register(RuleBehavior::new(
        "income-loss-prompts-loan",
        ["income_lost"],
        |_state: &WorldState, _event: &Event| {
            vec![ActionRequest::new("request_loan")
                .actor(JONAS)
                .arg("lender", LEO)
                .arg("amount", 40_i64)]
        },
    ))?;
    registry.register(RuleBehavior::new(
        "fishing-shift-lands-catch",
        ["work_shift_completed"],
        |_state: &WorldState, event: &Event| {
            if event.actor == Some(JONAS) && event.targets.contains(&HARBOR) {
                vec![ActionRequest::new("land_catch").actor(JONAS)]
            } else {
                Vec::new()
            }
        },
    ))?;
    registry.register(RuleBehavior::new(
        "landed-catch-sells-to-mainland",
        ["catch_landed"],
        |state: &WorldState, event: &Event| {
            let can_buy = integer_component(state, MAINLAND_MARKET, CASH)
                .is_ok_and(|cash| cash >= DAILY_CATCH_VALUE);
            if event.actor == Some(JONAS) && event.targets.contains(&HARBOR) && can_buy {
                vec![ActionRequest::new("sell_fish").actor(JONAS)]
            } else {
                Vec::new()
            }
        },
    ))?;
    registry.register(RuleBehavior::new(
        "missed-shift-loses-order",
        ["shift_missed"],
        |_state: &WorldState, _event: &Event| vec![ActionRequest::new("lose_order").actor(MARA)],
    ))?;
    registry.register(RuleBehavior::new(
        "lost-order-causes-dismissal",
        ["order_lost"],
        |_state: &WorldState, _event: &Event| {
            vec![ActionRequest::new("dismiss_worker").actor(MARA)]
        },
    ))?;
    registry.register(RuleBehavior::new(
        "jonas-low-cash-prompts-support",
        ["living_cost_paid"],
        |state: &WorldState, event: &Event| {
            if event.actor != Some(JONAS) {
                return Vec::new();
            }
            let cash = integer_component(state, JONAS, CASH).ok();
            let status = state
                .entity(JONAS)
                .and_then(|entity| entity.component(SUPPORT_STATUS));
            match (cash, status) {
                (Some(cash), Some(Value::Text(status)))
                    if cash <= JONAS_SUPPORT_THRESHOLD && status == "none" =>
                {
                    vec![ActionRequest::new("request_support")
                        .actor(JONAS)
                        .arg("resident", JONAS)
                        .arg("supporter", LEO)]
                }
                _ => Vec::new(),
            }
        },
    ))?;
    registry.register(RuleBehavior::new(
        "leo-answers-support-request",
        ["support_requested"],
        |_state: &WorldState, event: &Event| {
            if event.actor == Some(JONAS) && event.targets.contains(&LEO) {
                vec![ActionRequest::new("provide_support")
                    .actor(LEO)
                    .arg("resident", JONAS)
                    .arg("supporter", LEO)
                    .arg("amount", LEO_SUPPORT_AMOUNT)]
            } else {
                Vec::new()
            }
        },
    ))?;
    registry.register(RuleBehavior::new(
        "bakery-payroll-shortfall-closes-bakery",
        ["payroll_shortfall"],
        |_state: &WorldState, event: &Event| {
            if event.targets.contains(&BAKERY) {
                vec![ActionRequest::new("close_bakery").actor(MARA)]
            } else {
                Vec::new()
            }
        },
    ))?;
    crate::payroll::register_behaviors(registry)?;
    crate::reciprocity::register_behaviors(registry)?;
    crate::local_economy::register_behaviors(registry)?;
    Ok(())
}
