use crate::model::{MARA_BAKERY_JOB, OPERATING_STATUS};
use crate::{actions::text_component, build_action_registry, TinySocietyBranch, BAKERY, MARA};
use society_basic::{integer_component, CASH, JOB};
use std::error::Error;
use world_core::{
    Action, ActionError, ActionRegistry, ActionRequest, EventDraft, EventId, Relation, StateChange,
    WorldState,
};

pub(crate) const LEAN_REOPEN_INVESTMENT: i64 = 60;

pub(crate) fn register_actions(registry: &mut ActionRegistry) -> Result<(), ActionError> {
    registry.register(ReopenBakeryLean)?;
    Ok(())
}

pub(crate) fn reopen_lean(branch: &mut TinySocietyBranch) -> Result<Vec<EventId>, Box<dyn Error>> {
    let closure = branch
        .world
        .events()
        .iter()
        .rev()
        .find(|event| event.kind == "bakery_closed")
        .map(|event| event.id)
        .ok_or_else(|| std::io::Error::other("the bakery has not closed"))?;
    let actions = build_action_registry()?;
    let reopened = branch
        .world
        .execute(
            &actions,
            &ActionRequest::new("reopen_bakery_lean")
                .actor(MARA)
                .caused_by(closure),
        )?
        .id;
    Ok(vec![reopened])
}

struct ReopenBakeryLean;

impl Action for ReopenBakeryLean {
    fn name(&self) -> &'static str {
        "reopen_bakery_lean"
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

        let mara_cash = integer_component(state, MARA, CASH)?;
        if mara_cash < LEAN_REOPEN_INVESTMENT {
            return Err(ActionError::Invalid(format!(
                "Mara needs {LEAN_REOPEN_INVESTMENT} cash to reopen as an owner-run counter"
            )));
        }
        let bakery_cash = integer_component(state, BAKERY, CASH)?;

        let mut draft = EventDraft::new("bakery_reopened_lean");
        draft.actor = Some(MARA);
        draft.targets = vec![BAKERY, MARA];
        draft
            .payload
            .insert("investment".into(), LEAN_REOPEN_INVESTMENT.into());
        draft.payload.insert("model".into(), "owner_run".into());
        draft.changes = vec![
            StateChange::SetComponent {
                entity: MARA,
                key: CASH.into(),
                value: (mara_cash - LEAN_REOPEN_INVESTMENT).into(),
            },
            StateChange::SetComponent {
                entity: BAKERY,
                key: CASH.into(),
                value: (bakery_cash + LEAN_REOPEN_INVESTMENT).into(),
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
                value: "bakery_owner_operator".into(),
            },
        ];
        Ok(draft)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        model::OPERATING_STATUS, TinySociety, LEAN_REOPEN_BAKERY_COMMAND, REOPEN_BAKERY_COMMAND,
    };

    fn long_run_closed_branch() -> TinySocietyBranch {
        let mut society = TinySociety::new().unwrap();
        society.run_story().unwrap();
        let mut branch = society.branch();
        branch.advance_days(120).unwrap();
        assert_eq!(
            text_component(branch.world().state(), BAKERY, OPERATING_STATUS).unwrap(),
            "closed"
        );
        branch
    }

    #[test]
    fn same_closure_can_diverge_into_fragile_traditional_or_stable_lean_recovery() {
        let closed = long_run_closed_branch();
        let original_closure = closed
            .world()
            .events()
            .iter()
            .rev()
            .find(|event| event.kind == "bakery_closed")
            .expect("long-running dismissal branch closes the bakery")
            .id;
        let commands = closed.projection_snapshot().commands;
        assert!(commands
            .iter()
            .any(|command| command.id == REOPEN_BAKERY_COMMAND));
        assert!(commands
            .iter()
            .any(|command| command.id == LEAN_REOPEN_BAKERY_COMMAND));

        let mut traditional = closed.clone();
        traditional
            .invoke_projection_command(REOPEN_BAKERY_COMMAND)
            .unwrap();
        let traditional_cursor = traditional.visit_cursor();
        traditional.advance_days(20).unwrap();
        assert!(
            traditional.world().events()[traditional_cursor.event_count..]
                .iter()
                .any(|event| event.kind == "bakery_closed")
        );

        let mut lean = closed;
        let mara_before = integer_component(lean.world().state(), MARA, CASH).unwrap();
        let bakery_before = integer_component(lean.world().state(), BAKERY, CASH).unwrap();
        let reopened = lean
            .invoke_projection_command(LEAN_REOPEN_BAKERY_COMMAND)
            .unwrap();
        assert_eq!(reopened.len(), 1);
        let event = lean
            .world()
            .event(reopened[0])
            .expect("lean reopen creates an event");
        assert_eq!(event.kind, "bakery_reopened_lean");
        assert_eq!(event.caused_by, vec![original_closure]);
        assert_eq!(
            integer_component(lean.world().state(), MARA, CASH).unwrap(),
            mara_before - LEAN_REOPEN_INVESTMENT
        );
        assert_eq!(
            integer_component(lean.world().state(), BAKERY, CASH).unwrap(),
            bakery_before + LEAN_REOPEN_INVESTMENT
        );
        assert_eq!(
            text_component(lean.world().state(), MARA, JOB).unwrap(),
            "bakery_owner_operator"
        );
        assert!(lean.world().state().relation(MARA_BAKERY_JOB).is_some());

        let lean_cursor = lean.visit_cursor();
        lean.advance_days(20).unwrap();
        let lean_events = &lean.world().events()[lean_cursor.event_count..];
        assert!(!lean_events
            .iter()
            .any(|event| event.kind == "bakery_closed"));
        assert!(!lean_events.iter().any(|event| {
            event.kind == "work_shift_completed"
                && event.actor == Some(MARA)
                && event.targets.contains(&BAKERY)
        }));
        assert_eq!(
            text_component(lean.world().state(), BAKERY, OPERATING_STATUS).unwrap(),
            "open"
        );

        let archive = lean.archive().unwrap();
        let resumed = TinySociety::resume_archive(&archive).unwrap();
        assert_eq!(
            text_component(resumed.world().state(), MARA, JOB).unwrap(),
            "bakery_owner_operator"
        );
        assert_eq!(
            text_component(resumed.world().state(), BAKERY, OPERATING_STATUS).unwrap(),
            "open"
        );
    }
}
