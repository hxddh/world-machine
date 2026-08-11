use crate::model::{BAKERY, JONAS, LEO, MARA};
use std::error::Error;
use world_core::{ActionRequest, BehaviorRegistry, Event, RuleBehavior, WorldState};

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
    Ok(())
}
