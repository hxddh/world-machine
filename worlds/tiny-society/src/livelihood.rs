//! What a man does when the boat is gone.
//!
//! Selling Sea Finch closed the question the World had been holding open, and
//! left a worse one behind: Jonas is unemployed, has no trade, and lives in a
//! town with one bakery. Without this chain the World simply stopped there —
//! measured, seven of fourteen returns had nothing to report, and five of them
//! were this exact dead end, with no choice on offer at all.
//!
//! So he asks Mara for work, and whether she takes him back is the next thing
//! somebody can decide. Left unanswered it drifts, and it drifts to yes: a
//! small town with a trading bakery and a destitute neighbour does not leave
//! him standing outside indefinitely, and a default of no would put the World
//! straight back into the dead end this chain exists to end.
//!
//! Saying yes is not free, and nothing here makes it free. The bakery takes on
//! a second daily wage against the same island trade it was already living on,
//! and the payroll chain that has been in this Pack since the beginning does
//! the rest on its own: the till drains, the reserve runs out, and Harbor
//! Bakery closes. That is not a punishment written into this module, it is
//! what the existing model does with one more wage, and it is what turns the
//! end of the fishing life into the beginning of the bakery's.

use crate::{
    actions::text_component,
    model::{BAKERY, JONAS, MARA, OPERATING_STATUS, TEMP_BAKERY_JOB},
};
use society_basic::{EMPLOYER, JOB};
use std::error::Error;
use world_core::{
    Action, ActionError, ActionRegistry, ActionRequest, BehaviorRegistry, Event, EventDraft,
    Relation, RuleBehavior, StateChange, WorldState,
};

/// On Jonas: `sought` while the ask is open, `taken_on` once it is answered.
pub(crate) const WORK_REQUEST_STATUS: &str = "work_request_status";
pub(crate) const SOUGHT: &str = "sought";
pub(crate) const TAKEN_ON: &str = "taken_on";

/// Jonas's daily wage on the counter — the same one the bakery paid him during
/// the opening story, so taking him back costs what it cost then.
pub(crate) const COUNTER_WAGE: i64 = 18;

/// Days the ask stands before Mara answers it herself.
pub(crate) const WORK_ANSWER_DRIFTS_AFTER_DAYS: u64 = 6;

pub(crate) fn register_actions(registry: &mut ActionRegistry) -> Result<(), ActionError> {
    registry.register(SeekWork)?;
    registry.register(TakeJonasOn)?;
    Ok(())
}

pub(crate) fn register_behaviors(registry: &mut BehaviorRegistry) -> Result<(), Box<dyn Error>> {
    registry.register(RuleBehavior::new(
        "a-man-without-a-boat-asks-for-work",
        ["boat_sold"],
        |state: &WorldState, event: &Event| {
            if event.actor != Some(JONAS) {
                return Vec::new();
            }
            if !work_can_be_sought(state) {
                return Vec::new();
            }
            vec![ActionRequest::new("seek_work").actor(JONAS)]
        },
    ))?;
    Ok(())
}

fn work_request_status(state: &WorldState) -> &str {
    text_component(state, JONAS, WORK_REQUEST_STATUS).unwrap_or("none")
}

fn work_can_be_sought(state: &WorldState) -> bool {
    work_request_status(state) == "none"
        && text_component(state, JONAS, JOB).ok() == Some("unemployed")
        && text_component(state, BAKERY, OPERATING_STATUS).ok() == Some("open")
}

/// Whether Jonas's ask is still waiting on an answer.
pub(crate) fn work_ask_is_open(state: &WorldState) -> bool {
    work_request_status(state) == SOUGHT
        && text_component(state, BAKERY, OPERATING_STATUS).ok() == Some("open")
}

struct SeekWork;

impl Action for SeekWork {
    fn name(&self) -> &'static str {
        "seek_work"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        _request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        if !work_can_be_sought(state) {
            return Err(ActionError::Invalid(
                "Jonas has no reason to be asking the bakery for work".into(),
            ));
        }

        let mut draft = EventDraft::new("work_sought");
        draft.actor = Some(JONAS);
        draft.targets = vec![JONAS, MARA, BAKERY];
        draft.changes.push(StateChange::SetComponent {
            entity: JONAS,
            key: WORK_REQUEST_STATUS.into(),
            value: SOUGHT.into(),
        });
        Ok(draft)
    }
}

struct TakeJonasOn;

impl Action for TakeJonasOn {
    fn name(&self) -> &'static str {
        "take_jonas_on"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        if !work_ask_is_open(state) {
            return Err(ActionError::Invalid(
                "there is no open ask for Mara to answer".into(),
            ));
        }
        if state.relation(TEMP_BAKERY_JOB).is_some() {
            return Err(ActionError::Invalid(
                "Jonas already holds the bakery job".into(),
            ));
        }

        let mut draft = EventDraft::new("jonas_taken_on");
        draft.actor = Some(MARA);
        draft.targets = vec![JONAS, MARA, BAKERY];
        draft.payload.insert("wage".into(), COUNTER_WAGE.into());
        draft.changes = vec![
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
            StateChange::SetComponent {
                entity: JONAS,
                key: WORK_REQUEST_STATUS.into(),
                value: TAKEN_ON.into(),
            },
        ];
        crate::drift::record_decider(&mut draft, request);
        Ok(draft)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{TinySociety, TAKE_JONAS_ON_COMMAND};
    use society_basic::{integer_component, CASH};

    /// Advance a day at a time until the World reaches a state, rather than
    /// guessing how many days that takes. The deadlines compose — the backing
    /// lapses a while after Leo helps, the sale drifts a while after that —
    /// and arithmetic over those constants gets the answer wrong, which is how
    /// a test ends up asserting against a World that has already moved past
    /// the thing it meant to look at.
    fn advance_until(
        branch: &mut crate::TinySocietyBranch,
        limit: u64,
        reached: impl Fn(&crate::TinySocietyBranch) -> bool,
    ) {
        for _ in 0..limit {
            if reached(branch) {
                return;
            }
            branch.advance_days(1).unwrap();
        }
        panic!("the World never reached the state this test is about within {limit} days");
    }

    fn a_man_without_a_boat() -> crate::TinySocietyBranch {
        let mut society = TinySociety::new().unwrap();
        society.run_story().unwrap();
        let mut branch = society.branch();
        advance_until(&mut branch, 90, |branch| {
            work_ask_is_open(branch.world().state())
        });
        branch
    }

    #[test]
    fn selling_the_boat_leaves_an_open_question_rather_than_a_dead_end() {
        let branch = a_man_without_a_boat();

        assert!(
            branch
                .world()
                .events()
                .iter()
                .any(|event| event.kind == "work_sought"),
            "a man with no boat asks for work"
        );
        let commands = branch.projection_snapshot().commands;
        assert!(
            commands
                .iter()
                .any(|command| command.id == TAKE_JONAS_ON_COMMAND),
            "and whether he gets it is somebody's to decide, got {:?}",
            commands.iter().map(|c| &c.id).collect::<Vec<_>>()
        );
    }

    #[test]
    fn an_unanswered_ask_is_answered_by_the_town() {
        let mut branch = a_man_without_a_boat();
        branch
            .advance_days(WORK_ANSWER_DRIFTS_AFTER_DAYS + 1)
            .unwrap();

        let taken_on = branch
            .world()
            .events()
            .iter()
            .find(|event| event.kind == "jonas_taken_on")
            .expect("a small town does not leave a destitute neighbour standing outside");
        assert!(crate::drift::was_drifted(taken_on));
        assert_eq!(
            text_component(branch.world().state(), JONAS, JOB).unwrap(),
            "bakery_temp"
        );
    }

    #[test]
    fn the_bakery_carries_the_second_wage_and_eventually_cannot() {
        let mut branch = a_man_without_a_boat();
        branch
            .advance_days(WORK_ANSWER_DRIFTS_AFTER_DAYS + 1)
            .unwrap();
        assert_eq!(
            text_component(branch.world().state(), JONAS, JOB).unwrap(),
            "bakery_temp"
        );

        let till_when_hired = integer_component(branch.world().state(), BAKERY, CASH).unwrap();
        branch.advance_days(60).unwrap();
        let till_later = integer_component(branch.world().state(), BAKERY, CASH).unwrap();
        assert!(
            till_later < till_when_hired,
            "a second wage against the same trade drains the till: {till_when_hired} -> {till_later}"
        );
        assert!(
            branch
                .world()
                .events()
                .iter()
                .any(|event| event.kind == "bakery_closed"),
            "and the payroll chain that was always here does the rest"
        );
    }

    #[test]
    fn a_closed_bakery_is_not_still_asking_jonas_to_wait() {
        let mut branch = a_man_without_a_boat();
        assert!(work_ask_is_open(branch.world().state()));

        // Nothing offers work it cannot pay for.
        branch.advance_days(200).unwrap();
        if text_component(branch.world().state(), BAKERY, OPERATING_STATUS).unwrap() == "closed" {
            assert!(
                !branch
                    .projection_snapshot()
                    .commands
                    .iter()
                    .any(|command| command.id == TAKE_JONAS_ON_COMMAND),
                "a closed bakery does not offer a counter job"
            );
        }
    }
}
