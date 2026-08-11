#[path = "household.rs"]
mod household;

use crate::{EMMA, LEO, PUB, SCHOOL};
use society_basic::{integer_component, CASH};
use std::error::Error;
use world_core::{
    Action, ActionError, ActionRegistry, ActionRequest, BehaviorRegistry, EntityId, Event,
    EventDraft, RuleBehavior, Value, WorldState,
};

pub(crate) fn register_actions(registry: &mut ActionRegistry) -> Result<(), ActionError> {
    registry.register(RecordPayrollReserveExhausted)?;
    household::register_actions(registry)?;
    Ok(())
}

pub(crate) fn register_behaviors(registry: &mut BehaviorRegistry) -> Result<(), Box<dyn Error>> {
    registry.register(RuleBehavior::new(
        "institution-payroll-reserve-exhausted",
        ["work_shift_completed"],
        |state: &WorldState, event: &Event| {
            let Some(worker) = event.actor else {
                return Vec::new();
            };
            let workplace = match worker {
                LEO if event.targets.contains(&PUB) => PUB,
                EMMA if event.targets.contains(&SCHOOL) => SCHOOL,
                _ => return Vec::new(),
            };
            let Some(Value::Integer(wage)) = event.payload.get("wage") else {
                return Vec::new();
            };
            let Ok(cash_available) = integer_component(state, workplace, CASH) else {
                return Vec::new();
            };
            if cash_available >= *wage {
                return Vec::new();
            }

            vec![ActionRequest::new("record_payroll_reserve_exhausted")
                .actor(worker)
                .arg("worker", worker)
                .arg("workplace", workplace)
                .arg("next_wage", *wage)]
        },
    ))?;
    household::register_behaviors(registry)?;
    Ok(())
}

struct RecordPayrollReserveExhausted;

impl Action for RecordPayrollReserveExhausted {
    fn name(&self) -> &'static str {
        "record_payroll_reserve_exhausted"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let worker = entity_arg(request, "worker")?;
        let workplace = entity_arg(request, "workplace")?;
        let next_wage = positive_integer_arg(request, "next_wage")?;
        if !matches!((worker, workplace), (LEO, PUB) | (EMMA, SCHOOL)) {
            return Err(ActionError::Invalid(
                "payroll reserve tracking is currently defined for Leo/Pub and Emma/School".into(),
            ));
        }

        let cash_available = integer_component(state, workplace, CASH)?;
        if cash_available >= next_wage {
            return Err(ActionError::Invalid(format!(
                "workplace {workplace} can still cover the next wage {next_wage}"
            )));
        }

        let mut draft = EventDraft::new("payroll_reserve_exhausted");
        draft.actor = Some(worker);
        draft.targets = vec![worker, workplace];
        draft.payload.insert("next_wage".into(), next_wage.into());
        draft
            .payload
            .insert("cash_available".into(), cash_available.into());
        Ok(draft)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{TinySociety, BAKERY};

    #[test]
    fn pub_reserve_exhaustion_is_one_shot_while_leo_keeps_buying_from_savings() {
        let mut society = TinySociety::new().unwrap();
        society.run_story().unwrap();
        let mut branch = society.branch();
        let cursor = branch.visit_cursor();
        let bakery_before = integer_component(branch.world().state(), BAKERY, CASH).unwrap();

        branch.advance_days(30).unwrap();

        let new_events = &branch.world().events()[cursor.event_count..];
        let exhausted = new_events
            .iter()
            .filter(|event| {
                event.kind == "payroll_reserve_exhausted"
                    && event.actor == Some(LEO)
                    && event.targets.contains(&PUB)
            })
            .collect::<Vec<_>>();
        assert_eq!(exhausted.len(), 1);
        let exhausted = exhausted[0];
        assert_eq!(exhausted.caused_by.len(), 1);
        let cause = branch
            .world()
            .event(exhausted.caused_by[0])
            .expect("reserve exhaustion cause remains in history");
        assert_eq!(cause.kind, "work_shift_completed");
        assert_eq!(cause.actor, Some(LEO));
        assert!(cause.targets.contains(&PUB));
        assert!(matches!(
            exhausted.payload.get("cash_available"),
            Some(Value::Integer(cash)) if *cash < 22
        ));
        assert_eq!(
            exhausted.payload.get("next_wage"),
            Some(&Value::Integer(22))
        );

        assert!(new_events.iter().all(|event| {
            !(event.kind == "work_shift_completed"
                && event.actor == Some(LEO)
                && event.targets.contains(&PUB)
                && event.world_time > exhausted.world_time)
        }));
        assert!(new_events.iter().any(|event| {
            event.kind == "bread_purchased"
                && event.actor == Some(LEO)
                && event.world_time > exhausted.world_time
        }));
        assert_eq!(
            integer_component(branch.world().state(), BAKERY, CASH).unwrap(),
            bakery_before
        );
        assert!(!new_events.iter().any(|event| event.kind == "bakery_closed"));

        let briefing = branch
            .projection_snapshot_since(cursor)
            .briefing
            .expect("Tiny Society has a return briefing");
        assert!(briefing
            .items
            .iter()
            .any(|item| item.title == "Anchor Pub exhausted its payroll reserve"));

        let archive = branch.archive().unwrap();
        let resumed = TinySociety::resume_archive(&archive).unwrap();
        assert_eq!(
            resumed
                .world()
                .events()
                .iter()
                .filter(|event| {
                    event.kind == "payroll_reserve_exhausted"
                        && event.actor == Some(LEO)
                        && event.targets.contains(&PUB)
                })
                .count(),
            1
        );
    }

    #[test]
    fn school_reserve_exhaustion_is_also_recorded_once() {
        let mut society = TinySociety::new().unwrap();
        society.run_story().unwrap();
        let mut branch = society.branch();

        branch.advance_days(60).unwrap();

        assert_eq!(
            branch
                .world()
                .events()
                .iter()
                .filter(|event| {
                    event.kind == "payroll_reserve_exhausted"
                        && event.actor == Some(EMMA)
                        && event.targets.contains(&SCHOOL)
                })
                .count(),
            1
        );
    }
}
