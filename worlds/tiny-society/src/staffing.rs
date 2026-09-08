//! The second long-run consequence chain: what a lean reopening costs, and
//! what recovered demand can buy back.
//!
//! The first chain runs on trust and spending — Jonas repays Leo, resumes
//! local spending, and his bread money buffers a household's budget cut. This
//! one runs on employment and reads that chain's result. When Mara reopens the
//! bakery as an owner-run counter she is working it alone; every loaf sold
//! across that counter is counted, and once the counter has carried enough
//! trade and the bakery holds a wage in reserve, she takes Mia on. The job the
//! closure cost the island comes back — but only in a World where the harbour
//! recovered far enough for people to be buying bread again.
//!
//! A World where demand never recovers keeps a one-person bakery, durably.

use crate::{
    actions::text_component,
    local_economy::LOCAL_SPENDING_STATUS,
    model::{BAKERY, JONAS, MARA, MIA, MIA_BAKERY_JOB, OPERATING_STATUS},
};
use society_basic::{integer_component, CASH, JOB};
use std::error::Error;
use world_core::{
    Action, ActionError, ActionRegistry, ActionRequest, BehaviorRegistry, Event, EventDraft,
    Relation, RuleBehavior, StateChange, Value, WorldState,
};

/// On the bakery: `lean` while Mara runs the counter alone, `staffed` once she
/// has taken somebody on.
pub(crate) const STAFFING_STATUS: &str = "staffing_status";
/// Loaves the recovered harbour has bought across the owner-run counter since
/// the lean reopening. Baseline island trade is deliberately not counted.
pub(crate) const COUNTER_SALES: &str = "counter_sales";

/// Sales across the lean counter before Mara can consider taking somebody on.
pub(crate) const REHIRE_AFTER_SALES: i64 = 3;
/// Cash the bakery must hold before a second wage is a responsible promise.
pub(crate) const REHIRE_CASH_RESERVE: i64 = 520;
/// Mia's first wage, paid on the day she starts.
pub(crate) const FIRST_WAGE: i64 = 8;

pub(crate) fn register_actions(registry: &mut ActionRegistry) -> Result<(), ActionError> {
    registry.register(RecordCounterSale)?;
    registry.register(HireCounterHelp)?;
    Ok(())
}

pub(crate) fn register_behaviors(registry: &mut BehaviorRegistry) -> Result<(), Box<dyn Error>> {
    registry.register(RuleBehavior::new(
        "owner-run-counter-counts-its-trade",
        ["bread_purchased"],
        |state: &WorldState, event: &Event| {
            if !event.targets.contains(&BAKERY) {
                return Vec::new();
            }
            if text_component(state, BAKERY, STAFFING_STATUS).ok() != Some("lean") {
                return Vec::new();
            }
            // Only recovered demand counts. The island's baseline bread money
            // was there before the closure and proves nothing; what tells Mara
            // the work is back is Jonas spending locally again, which is the
            // end of the first chain.
            if event.actor != Some(JONAS)
                || text_component(state, JONAS, LOCAL_SPENDING_STATUS).ok() != Some("active")
            {
                return Vec::new();
            }
            vec![ActionRequest::new("record_counter_sale").actor(MARA)]
        },
    ))?;
    registry.register(RuleBehavior::new(
        "sustained-counter-trade-brings-a-second-pair-of-hands",
        ["counter_sale_recorded"],
        |state: &WorldState, _event: &Event| {
            if !rehire_is_earned(state) {
                return Vec::new();
            }
            vec![ActionRequest::new("hire_counter_help")
                .actor(MARA)
                .arg("hire", MIA)]
        },
    ))?;
    Ok(())
}

/// Whether the lean counter has carried enough trade, and the bakery holds
/// enough cash, for a second wage to be a promise Mara can keep.
pub(crate) fn rehire_is_earned(state: &WorldState) -> bool {
    if text_component(state, BAKERY, STAFFING_STATUS).ok() != Some("lean") {
        return false;
    }
    if text_component(state, BAKERY, OPERATING_STATUS).ok() != Some("open") {
        return false;
    }
    let sales = integer_component(state, BAKERY, COUNTER_SALES).unwrap_or(0);
    let cash = integer_component(state, BAKERY, CASH).unwrap_or(0);
    sales >= REHIRE_AFTER_SALES && cash >= REHIRE_CASH_RESERVE
}

fn entity_arg(request: &ActionRequest, name: &str) -> Result<world_core::EntityId, ActionError> {
    match request.args.get(name) {
        Some(Value::Entity(id)) => Ok(*id),
        _ => Err(ActionError::Invalid(format!("missing entity arg: {name}"))),
    }
}

struct RecordCounterSale;
struct HireCounterHelp;

impl Action for RecordCounterSale {
    fn name(&self) -> &'static str {
        "record_counter_sale"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        _request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        if text_component(state, BAKERY, STAFFING_STATUS)? != "lean" {
            return Err(ActionError::Invalid(
                "the bakery is not running an owner-run counter".into(),
            ));
        }
        let sales = integer_component(state, BAKERY, COUNTER_SALES)?.saturating_add(1);

        let mut draft = EventDraft::new("counter_sale_recorded");
        draft.actor = Some(MARA);
        draft.targets = vec![BAKERY];
        draft.payload.insert("counter_sales".into(), sales.into());
        draft
            .payload
            .insert("sales_to_rehire".into(), REHIRE_AFTER_SALES.into());
        draft.changes = vec![StateChange::SetComponent {
            entity: BAKERY,
            key: COUNTER_SALES.into(),
            value: sales.into(),
        }];
        Ok(draft)
    }
}

impl Action for HireCounterHelp {
    fn name(&self) -> &'static str {
        "hire_counter_help"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        if !rehire_is_earned(state) {
            return Err(ActionError::Invalid(
                "the counter has not carried enough trade for a second wage".into(),
            ));
        }
        let hire = entity_arg(request, "hire")?;
        if state.relation(MIA_BAKERY_JOB).is_some() {
            return Err(ActionError::Invalid(
                "the bakery already has counter help".into(),
            ));
        }
        let bakery_cash = integer_component(state, BAKERY, CASH)?;
        let hire_cash = integer_component(state, hire, CASH)?;
        let sales = integer_component(state, BAKERY, COUNTER_SALES)?;

        let mut draft = EventDraft::new("counter_help_hired");
        draft.actor = Some(MARA);
        draft.targets = vec![BAKERY, hire];
        draft.payload.insert("counter_sales".into(), sales.into());
        draft.payload.insert("first_wage".into(), FIRST_WAGE.into());
        draft.changes = vec![
            StateChange::SetComponent {
                entity: BAKERY,
                key: STAFFING_STATUS.into(),
                value: "staffed".into(),
            },
            StateChange::SetComponent {
                entity: BAKERY,
                key: CASH.into(),
                value: (bakery_cash - FIRST_WAGE).into(),
            },
            StateChange::SetComponent {
                entity: hire,
                key: CASH.into(),
                value: (hire_cash + FIRST_WAGE).into(),
            },
            StateChange::SetComponent {
                entity: hire,
                key: JOB.into(),
                value: "bakery_assistant".into(),
            },
            StateChange::CreateRelation(Relation::new(MIA_BAKERY_JOB, "works_at", hire, BAKERY)),
        ];
        Ok(draft)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{TinySociety, TinySocietyBranch, LEAN_REOPEN_BAKERY_COMMAND, REPAIR_BOAT_COMMAND};

    fn bakery_state(staffing: Option<&str>, operating: &str, sales: i64, cash: i64) -> WorldState {
        let mut state = WorldState::default();
        let mut bakery = world_core::Entity::new(BAKERY, "location")
            .with_component(OPERATING_STATUS, operating)
            .with_component(COUNTER_SALES, sales)
            .with_component(CASH, cash);
        if let Some(staffing) = staffing {
            bakery = bakery.with_component(STAFFING_STATUS, staffing);
        }
        state.seed_entity(bakery).unwrap();
        state
    }

    /// The World where the first chain completed: Jonas's boat is repaired, he
    /// repays Leo and spends locally again — and the bakery still closed and
    /// reopened lean.
    fn recovered_lean_branch() -> TinySocietyBranch {
        let mut society = TinySociety::new().unwrap();
        society.run_story().unwrap();
        let mut branch = society.branch();
        branch.advance_days(10).unwrap();
        branch
            .invoke_projection_command(REPAIR_BOAT_COMMAND)
            .unwrap();
        branch.advance_days(120).unwrap();
        assert_eq!(
            text_component(branch.world().state(), JONAS, LOCAL_SPENDING_STATUS).unwrap(),
            "active",
            "this branch exists to have the first chain finished"
        );
        branch
            .invoke_projection_command(LEAN_REOPEN_BAKERY_COMMAND)
            .unwrap();
        branch
    }

    /// The same closure and the same lean reopening, in a World where the
    /// harbour never recovered.
    fn unrecovered_lean_branch() -> TinySocietyBranch {
        let mut society = TinySociety::new().unwrap();
        society.run_story().unwrap();
        let mut branch = society.branch();
        branch.advance_days(120).unwrap();
        branch
            .invoke_projection_command(LEAN_REOPEN_BAKERY_COMMAND)
            .unwrap();
        branch
    }

    fn staffing_of(branch: &TinySocietyBranch) -> String {
        text_component(branch.world().state(), BAKERY, STAFFING_STATUS)
            .unwrap()
            .to_owned()
    }

    #[test]
    fn a_lean_reopening_opens_an_owner_run_counter_that_counts_its_trade() {
        let branch = unrecovered_lean_branch();
        assert_eq!(staffing_of(&branch), "lean");
        assert_eq!(
            integer_component(branch.world().state(), BAKERY, COUNTER_SALES).unwrap(),
            0
        );
        // Nobody is hired the moment the shutters go up.
        assert!(branch.world().state().relation(MIA_BAKERY_JOB).is_none());
    }

    #[test]
    fn recovered_demand_across_the_counter_brings_mia_her_first_job() {
        let mut branch = recovered_lean_branch();
        let mia_cash_before = integer_component(branch.world().state(), MIA, CASH).unwrap();
        assert_eq!(staffing_of(&branch), "lean");

        // The hire is a long-run consequence, not an immediate one: the
        // counter has to carry the trade and the bakery has to build the
        // reserve, which takes several visits' worth of days.
        branch.advance_days(40).unwrap();
        assert_eq!(
            staffing_of(&branch),
            "lean",
            "recovered demand alone is not yet a wage Mara can promise"
        );
        assert!(integer_component(branch.world().state(), BAKERY, COUNTER_SALES).unwrap() > 0);

        branch.advance_days(60).unwrap();
        assert_eq!(staffing_of(&branch), "staffed");

        let hired = branch
            .world()
            .events()
            .iter()
            .find(|event| event.kind == "counter_help_hired")
            .expect("a staffed bakery has a hire event");
        assert_eq!(hired.actor, Some(MARA));
        assert!(hired.targets.contains(&MIA));
        assert!(branch.world().state().relation(MIA_BAKERY_JOB).is_some());
        assert_eq!(
            text_component(branch.world().state(), MIA, JOB).unwrap(),
            "bakery_assistant"
        );
        assert_eq!(
            integer_component(branch.world().state(), MIA, CASH).unwrap(),
            mia_cash_before + FIRST_WAGE
        );

        // The chain runs once: later trade does not hire again.
        branch.advance_days(60).unwrap();
        assert_eq!(
            branch
                .world()
                .events()
                .iter()
                .filter(|event| event.kind == "counter_help_hired")
                .count(),
            1
        );

        // The job survives a save and reload, like every other durable fact.
        let archive = branch.archive().unwrap();
        let resumed = TinySociety::resume_archive(&archive).unwrap();
        assert_eq!(
            text_component(resumed.world().state(), MIA, JOB).unwrap(),
            "bakery_assistant"
        );
        assert!(resumed.world().state().relation(MIA_BAKERY_JOB).is_some());
    }

    #[test]
    fn a_bakery_whose_demand_never_recovered_stays_a_one_person_counter() {
        let mut branch = unrecovered_lean_branch();
        branch.advance_days(120).unwrap();

        // Baseline island bread money is not evidence the work came back, so
        // the counter never moves and Mara never promises a second wage.
        assert_eq!(
            integer_component(branch.world().state(), BAKERY, COUNTER_SALES).unwrap(),
            0
        );
        assert_eq!(staffing_of(&branch), "lean");
        assert!(branch.world().state().relation(MIA_BAKERY_JOB).is_none());
        assert!(!branch
            .world()
            .events()
            .iter()
            .any(|event| event.kind == "counter_help_hired"));
    }

    #[test]
    fn a_second_wage_waits_for_both_trade_and_a_reserve() {
        assert!(rehire_is_earned(&bakery_state(
            Some("lean"),
            "open",
            REHIRE_AFTER_SALES,
            REHIRE_CASH_RESERVE
        )));

        // Trade without a reserve is a promise Mara could not keep.
        assert!(!rehire_is_earned(&bakery_state(
            Some("lean"),
            "open",
            REHIRE_AFTER_SALES,
            REHIRE_CASH_RESERVE - 1
        )));
        // A reserve without trade is not evidence the work exists.
        assert!(!rehire_is_earned(&bakery_state(
            Some("lean"),
            "open",
            REHIRE_AFTER_SALES - 1,
            REHIRE_CASH_RESERVE
        )));
    }

    #[test]
    fn only_a_lean_open_bakery_can_take_somebody_on() {
        // Already staffed: the chain runs once.
        assert!(!rehire_is_earned(&bakery_state(
            Some("staffed"),
            "open",
            99,
            9_999
        )));
        // A bakery that never reopened lean has no counter to count.
        assert!(!rehire_is_earned(&bakery_state(None, "open", 99, 9_999)));
        // Closed again before the hire: nobody is taken on.
        assert!(!rehire_is_earned(&bakery_state(
            Some("lean"),
            "closed",
            99,
            9_999
        )));
    }
}
