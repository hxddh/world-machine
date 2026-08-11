use crate::{
    actions::text_component,
    model::{BAKERY, EMMA, INCOME_STATUS, LEO, PUB, SCHOOL},
};
use society_basic::{integer_component, CASH};
use std::error::Error;
use world_core::{
    Action, ActionError, ActionRegistry, ActionRequest, BehaviorRegistry, EntityId, Event,
    EventDraft, RuleBehavior, StateChange, Value, WorldState,
};

pub(crate) const EMERGENCY_SAVINGS: &str = "emergency_savings";
pub(crate) const SAVINGS_BUFFER_THRESHOLD: i64 = 200;

pub(crate) fn register_actions(registry: &mut ActionRegistry) -> Result<(), ActionError> {
    registry.register(RecordIncomeDisrupted)?;
    registry.register(EnterSavingsMode)?;
    Ok(())
}

pub(crate) fn register_behaviors(
    registry: &mut BehaviorRegistry,
) -> Result<(), Box<dyn Error>> {
    registry.register(RuleBehavior::new(
        "payroll-exhaustion-disrupts-household-income",
        ["payroll_reserve_exhausted"],
        |_state: &WorldState, event: &Event| {
            let Some(resident) = event.actor else {
                return Vec::new();
            };
            let workplace = match resident {
                LEO if event.targets.contains(&PUB) => PUB,
                EMMA if event.targets.contains(&SCHOOL) => SCHOOL,
                _ => return Vec::new(),
            };
            vec![ActionRequest::new("record_income_disrupted")
                .actor(resident)
                .arg("resident", resident)
                .arg("workplace", workplace)]
        },
    ))?;
    registry.register(RuleBehavior::new(
        "disrupted-income-eventually-cuts-bread-budget",
        ["bread_purchased"],
        |state: &WorldState, event: &Event| {
            let Some(resident) = event.actor else {
                return Vec::new();
            };
            if !matches!(resident, LEO | EMMA) || !event.targets.contains(&BAKERY) {
                return Vec::new();
            }
            if text_component(state, resident, INCOME_STATUS).ok() != Some("disrupted") {
                return Vec::new();
            }
            if state
                .entity(resident)
                .and_then(|entity| entity.component(EMERGENCY_SAVINGS))
                .is_some()
            {
                return Vec::new();
            }
            let Ok(cash) = integer_component(state, resident, CASH) else {
                return Vec::new();
            };
            if cash > SAVINGS_BUFFER_THRESHOLD {
                return Vec::new();
            }

            vec![ActionRequest::new("enter_savings_mode")
                .actor(resident)
                .arg("resident", resident)]
        },
    ))?;
    Ok(())
}

struct RecordIncomeDisrupted;

impl Action for RecordIncomeDisrupted {
    fn name(&self) -> &'static str {
        "record_income_disrupted"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let resident = entity_arg(request, "resident")?;
        let workplace = entity_arg(request, "workplace")?;
        if !matches!((resident, workplace), (LEO, PUB) | (EMMA, SCHOOL)) {
            return Err(ActionError::Invalid(
                "income disruption is currently defined for Leo/Pub and Emma/School".into(),
            ));
        }
        if state
            .entity(resident)
            .and_then(|entity| entity.component(INCOME_STATUS))
            .is_some_and(|status| status == &Value::Text("disrupted".into()))
        {
            return Err(ActionError::Invalid(
                "resident income is already disrupted".into(),
            ));
        }

        let mut draft = EventDraft::new("income_disrupted");
        draft.actor = Some(resident);
        draft.targets = vec![resident, workplace];
        draft.changes.push(StateChange::SetComponent {
            entity: resident,
            key: INCOME_STATUS.into(),
            value: "disrupted".into(),
        });
        Ok(draft)
    }
}

struct EnterSavingsMode;

impl Action for EnterSavingsMode {
    fn name(&self) -> &'static str {
        "enter_savings_mode"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let resident = entity_arg(request, "resident")?;
        if !matches!(resident, LEO | EMMA) {
            return Err(ActionError::Invalid(
                "savings mode is currently defined for Leo and Emma".into(),
            ));
        }
        if text_component(state, resident, INCOME_STATUS)? != "disrupted" {
            return Err(ActionError::Invalid(
                "resident income is not disrupted".into(),
            ));
        }
        if state
            .entity(resident)
            .and_then(|entity| entity.component(EMERGENCY_SAVINGS))
            .is_some()
        {
            return Err(ActionError::Invalid(
                "resident is already protecting emergency savings".into(),
            ));
        }

        let cash = integer_component(state, resident, CASH)?;
        if cash > SAVINGS_BUFFER_THRESHOLD {
            return Err(ActionError::Invalid(format!(
                "resident still has {cash} cash, above savings threshold {SAVINGS_BUFFER_THRESHOLD}"
            )));
        }

        let mut draft = EventDraft::new("bread_budget_cut");
        draft.actor = Some(resident);
        draft.targets = vec![resident, BAKERY];
        draft.payload.insert("protected_savings".into(), cash.into());
        draft
            .payload
            .insert("threshold".into(), SAVINGS_BUFFER_THRESHOLD.into());
        draft.changes = vec![
            StateChange::SetComponent {
                entity: resident,
                key: CASH.into(),
                value: 0_i64.into(),
            },
            StateChange::SetComponent {
                entity: resident,
                key: EMERGENCY_SAVINGS.into(),
                value: cash.into(),
            },
        ];
        Ok(draft)
    }
}

fn entity_arg(request: &ActionRequest, name: &str) -> Result<EntityId, ActionError> {
    match request.args.get(name) {
        Some(Value::Entity(id)) => Ok(*id),
        _ => Err(ActionError::Invalid(format!("missing entity arg: {name}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{TinySociety, BAKERY};

    fn event_for<'a>(
        society: &'a crate::TinySocietyBranch,
        kind: &str,
        actor: EntityId,
    ) -> &'a Event {
        society
            .world()
            .events()
            .iter()
            .find(|event| event.kind == kind && event.actor == Some(actor))
            .unwrap_or_else(|| panic!("missing {kind} for {actor}"))
    }

    #[test]
    fn thirty_day_equilibrium_survives_income_disruption_because_savings_buffer_spending() {
        let mut society = TinySociety::new().unwrap();
        society.run_story().unwrap();
        let mut branch = society.branch();
        let bakery_before = integer_component(branch.world().state(), BAKERY, CASH).unwrap();

        branch.advance_days(30).unwrap();

        assert!(branch
            .world()
            .events()
            .iter()
            .any(|event| event.kind == "income_disrupted" && event.actor == Some(LEO)));
        assert!(!branch
            .world()
            .events()
            .iter()
            .any(|event| event.kind == "bread_budget_cut"));
        assert_eq!(
            integer_component(branch.world().state(), BAKERY, CASH).unwrap(),
            bakery_before
        );
        assert!(!branch
            .world()
            .events()
            .iter()
            .any(|event| event.kind == "bakery_closed"));
    }

    #[test]
    fn leo_uses_savings_before_cutting_bakery_spending() {
        let mut society = TinySociety::new().unwrap();
        society.run_story().unwrap();
        let mut branch = society.branch();

        branch.advance_days(70).unwrap();

        let reserve = event_for(&branch, "payroll_reserve_exhausted", LEO);
        let disruption = event_for(&branch, "income_disrupted", LEO);
        let cut = event_for(&branch, "bread_budget_cut", LEO);
        assert_eq!(disruption.caused_by, vec![reserve.id]);
        assert!(cut.world_time > disruption.world_time);
        assert!(cut.world_time - disruption.world_time >= 100);
        assert_eq!(cut.caused_by.len(), 1);
        let purchase = branch
            .world()
            .event(cut.caused_by[0])
            .expect("budget cut purchase cause remains in history");
        assert_eq!(purchase.kind, "bread_purchased");
        assert_eq!(purchase.actor, Some(LEO));
        assert_eq!(purchase.world_time, cut.world_time);

        let protected = match cut.payload.get("protected_savings") {
            Some(Value::Integer(value)) => *value,
            other => panic!("unexpected protected savings payload: {other:?}"),
        };
        assert!(protected > 0 && protected <= SAVINGS_BUFFER_THRESHOLD);
        assert_eq!(
            integer_component(branch.world().state(), LEO, CASH).unwrap(),
            0
        );
        assert_eq!(
            integer_component(branch.world().state(), LEO, EMERGENCY_SAVINGS).unwrap(),
            protected
        );
        assert!(branch.world().events().iter().all(|event| {
            !(event.kind == "bread_purchased"
                && event.actor == Some(LEO)
                && event.world_time > cut.world_time)
        }));

        let archive = branch.archive().unwrap();
        let resumed = TinySociety::resume_archive(&archive).unwrap();
        assert_eq!(
            integer_component(resumed.world().state(), LEO, EMERGENCY_SAVINGS).unwrap(),
            protected
        );
    }

    #[test]
    fn delayed_household_cut_eventually_turns_demand_loss_into_bakery_crisis() {
        let mut society = TinySociety::new().unwrap();
        society.run_story().unwrap();
        let mut branch = society.branch();

        branch.advance_days(120).unwrap();

        let leo_cut = event_for(&branch, "bread_budget_cut", LEO);
        let closure = branch
            .world()
            .events()
            .iter()
            .find(|event| event.kind == "bakery_closed")
            .expect("demand loss eventually closes the bakery");
        assert!(closure.world_time > leo_cut.world_time);
        assert_eq!(closure.caused_by.len(), 1);
        assert_eq!(
            branch
                .world()
                .event(closure.caused_by[0])
                .expect("closure cause remains in history")
                .kind,
            "payroll_shortfall"
        );
        assert!(branch
            .world()
            .events()
            .iter()
            .any(|event| event.kind == "income_disrupted" && event.actor == Some(EMMA)));

        let briefing = branch.projection_snapshot();
        let briefing = briefing.briefing.expect("Tiny Society has a briefing");
        assert!(briefing
            .items
            .iter()
            .any(|item| item.title == "Harbor Bakery closed its doors"));
    }
}
