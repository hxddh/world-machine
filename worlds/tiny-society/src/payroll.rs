#[path = "household.rs"]
mod household;

use crate::model::{EMMA_SCHOOL_JOB, LEO_PUB_JOB, OPERATING_STATUS};
use crate::{EMMA, LEO, PUB, SCHOOL};
use society_basic::{integer_component, CASH, JOB};
use std::error::Error;
use world_core::{
    Action, ActionError, ActionRegistry, ActionRequest, BehaviorRegistry, EntityId, Event,
    EventDraft, RuleBehavior, StateChange, Value, WorldState,
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

        // A workplace that cannot pay stops operating, and the job it cannot
        // pay for ends. Without this the event was a sentence and nothing
        // else: Leo stayed `pub_owner` of a pub with nothing in the till for
        // the rest of the World, the shift scheduler silently skipped him
        // every day forever, and no further event was ever recorded about
        // him. The bakery has always done this on closing; the pub and the
        // school announced the same failure and then changed nothing.
        let (job_relation, ended_job) = match workplace {
            PUB => (LEO_PUB_JOB, "pub_closed"),
            _ => (EMMA_SCHOOL_JOB, "unemployed"),
        };
        draft.changes.push(StateChange::SetComponent {
            entity: workplace,
            key: OPERATING_STATUS.into(),
            value: "closed".into(),
        });
        draft
            .changes
            .push(StateChange::RemoveRelation(job_relation));
        draft.changes.push(StateChange::SetComponent {
            entity: worker,
            key: JOB.into(),
            value: ended_job.into(),
        });
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

#[cfg(test)]
mod ending_a_job_it_cannot_pay {
    use crate::model::OPERATING_STATUS;
    use crate::{TinySociety, EMMA, LEO, PUB, SCHOOL};
    use society_basic::JOB;
    use world_projection::{CanvasItemKind, CanvasItemState};

    /// A workplace that runs out of money does something about it.
    ///
    /// It used to record one sentence and change nothing: Leo stayed
    /// `pub_owner` of a pub with an empty till for the rest of the World, the
    /// shift scheduler skipped him in silence every day, and no further event
    /// about him was ever recorded.
    #[test]
    fn a_workplace_that_cannot_pay_stops_operating_and_the_job_ends() {
        let mut society = TinySociety::new().unwrap();
        society.run_story().unwrap();
        let mut branch = society.branch();
        branch.advance_days(80).unwrap();
        let world = branch.world();

        for (workplace, worker, ended) in [(PUB, LEO, "pub_closed"), (SCHOOL, EMMA, "unemployed")] {
            assert!(
                world
                    .events()
                    .iter()
                    .any(|event| event.kind == "payroll_reserve_exhausted"
                        && event.targets.contains(&workplace)),
                "the reserve ran out and the World said so"
            );
            assert_eq!(
                crate::actions::text_component(world.state(), workplace, OPERATING_STATUS).unwrap(),
                "closed",
                "a workplace that cannot pay is not still open"
            );
            assert_eq!(
                crate::actions::text_component(world.state(), worker, JOB).unwrap(),
                ended,
                "the job it could not pay for has ended"
            );
        }
    }

    /// And the drawing is told, so a shut pub is drawn shut.
    #[test]
    fn a_shut_workplace_reads_as_stopped_on_the_canvas() {
        let mut society = TinySociety::new().unwrap();
        society.run_story().unwrap();
        let mut branch = society.branch();
        branch.advance_days(80).unwrap();

        let canvas = branch.projection_snapshot().canvas;
        let place = |label: &str| {
            canvas
                .items
                .iter()
                .find(|item| item.kind == CanvasItemKind::Place && item.label == label)
                .unwrap_or_else(|| panic!("{label} is on the canvas"))
                .state
        };
        assert_eq!(place("Anchor Pub"), CanvasItemState::Stopped);
        assert_eq!(place("Island School"), CanvasItemState::Stopped);
        assert_eq!(
            place("Harbor"),
            CanvasItemState::Working,
            "the harbour is not a business and is never shut"
        );
    }
}
