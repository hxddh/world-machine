//! What happens to a harbour nobody is answering.
//!
//! Measured before this existed, with `cargo run -p tiny-society --example
//! dump_visits -- 14 4`: eight of fourteen returns had nothing to report. From
//! the third visit onward the World held out exactly one command — "Repair Sea
//! Finch with Leo's backing" — and if nobody pressed it, the World held it out
//! forever. Jonas stayed unemployed at 5 cash, his boat stayed broken, Leo's
//! money stayed earmarked, and every briefing in between counted bread.
//!
//! An offer that never expires is not really an offer, and a World that waits
//! indefinitely is not really living. So Leo's backing has a deadline, the way
//! [Pocket Universe's choices do](../../pocket-universe/src/drift.rs). If
//! nobody answers, Leo puts the money elsewhere and says so. That does not
//! leave the World with nothing: a man with a broken boat and no backing has
//! one thing left worth money, and selling it is the harder question the first
//! question turns into. That one drifts too — a destitute man does not sit
//! beside an unusable boat forever.
//!
//! What drifts is exactly as durable as what is chosen. Selling Sea Finch ends
//! the fishing life whoever ended it, which is the cost of staying away and the
//! reason to come back.

use crate::{
    actions::text_component,
    model::{CONDITION, JONAS, JONAS_BOAT, LEO, PUB},
    social::SEA_FINCH_REPAIR_COST,
};
use society_basic::{integer_component, CASH};
use std::error::Error;
use world_core::{
    Action, ActionError, ActionRegistry, ActionRequest, EntityId, EventDraft, EventId, StateChange,
    Value, World, WorldState,
};

/// Days Leo's backing stands after the repair becomes possible. Long enough
/// that somebody visiting weekly is never overridden — the app advances a
/// World at most seven periods per return — short enough that an abandoned
/// harbour keeps moving.
pub(crate) const BACKING_LAPSES_AFTER_DAYS: u64 = 8;

/// Days Jonas lives beside an unusable boat before selling it himself.
pub(crate) const SALE_DRIFTS_AFTER_DAYS: u64 = 10;

/// What a broken boat fetches. Well under the cost of repairing it, which is
/// the whole shape of the choice: answering was worth more than not answering.
pub(crate) const SEA_FINCH_SCRAP_VALUE: i64 = 30;

pub(crate) const BACKING_STATUS: &str = "backing_status";
pub(crate) const BACKING_OFFERED: &str = "offered";
pub(crate) const BACKING_WITHDRAWN: &str = "withdrawn";

/// Marks who answered, so a briefing can tell somebody what was settled
/// without them. Absent means the person did it.
pub(crate) const DECIDED_BY: &str = "decided_by";
pub(crate) const DECIDED_BY_ARG: &str = "decided_by";
pub(crate) const BY_DRIFT: &str = "drift";

pub(crate) fn register_actions(registry: &mut ActionRegistry) -> Result<(), ActionError> {
    registry.register(WithdrawBacking)?;
    registry.register(SellSeaFinch)?;
    Ok(())
}

/// Whether this request was the World answering for itself.
fn requested_by_drift(request: &ActionRequest) -> bool {
    matches!(
        request.args.get(DECIDED_BY_ARG),
        Some(Value::Text(by)) if by == BY_DRIFT
    )
}

pub(crate) fn record_decider(draft: &mut EventDraft, request: &ActionRequest) {
    if requested_by_drift(request) {
        draft.payload.insert(DECIDED_BY.into(), BY_DRIFT.into());
    }
}

/// Whether an Event was the World deciding for itself.
pub(crate) fn was_drifted(event: &world_core::Event) -> bool {
    matches!(
        event.payload.get(DECIDED_BY),
        Some(Value::Text(by)) if by == BY_DRIFT
    )
}

pub(crate) fn backing_status(state: &WorldState) -> &str {
    text_component(state, JONAS_BOAT, BACKING_STATUS).unwrap_or(BACKING_OFFERED)
}

/// Sea Finch can still be sold: it is broken, it has not already gone, and
/// Jonas still owns it.
pub(crate) fn sea_finch_can_be_sold(state: &WorldState) -> bool {
    text_component(state, JONAS_BOAT, CONDITION).ok() == Some("damaged")
        && backing_status(state) == BACKING_WITHDRAWN
}

/// Answer at most one overdue question per day, so a long absence unfolds as a
/// sequence a returning visitor can read rather than resolving in one jump.
pub(crate) fn resolve_overdue(
    world: &mut World,
    actions: &ActionRegistry,
) -> Result<Vec<EventId>, Box<dyn Error>> {
    let Some((action, actor, cause)) = overdue(world) else {
        return Ok(Vec::new());
    };
    let mut request = ActionRequest::new(action)
        .actor(actor)
        .arg(DECIDED_BY_ARG, BY_DRIFT);
    if let Some(cause) = cause {
        request = request.caused_by(cause);
    }
    Ok(vec![world.execute(actions, &request)?.id])
}

/// The question this World has left open longest past its deadline.
///
/// Deadlines are measured from the Event that opened the question rather than
/// from a component stamped for the purpose, so the reason a thing is overdue
/// stays visible in the history the visitor can already inspect.
fn overdue(world: &World) -> Option<(&'static str, EntityId, Option<EventId>)> {
    let now = world.world_time();
    let day = crate::persistence::WORLD_DAY_TICKS;

    if crate::projection::repair_offer_is_open(world) {
        let opened = world
            .events()
            .iter()
            .find(|event| event.kind == "support_received")?;
        if now.saturating_sub(opened.world_time) >= BACKING_LAPSES_AFTER_DAYS * day {
            return Some(("withdraw_backing", LEO, Some(opened.id)));
        }
        return None;
    }

    if sea_finch_can_be_sold(world.state()) {
        let withdrawn = world
            .events()
            .iter()
            .find(|event| event.kind == "backing_withdrawn")?;
        if now.saturating_sub(withdrawn.world_time) >= SALE_DRIFTS_AFTER_DAYS * day {
            return Some(("sell_sea_finch", JONAS, Some(withdrawn.id)));
        }
        return None;
    }

    if crate::livelihood::work_ask_is_open(world.state()) {
        let asked = world
            .events()
            .iter()
            .rev()
            .find(|event| event.kind == "work_sought")?;
        if now.saturating_sub(asked.world_time)
            >= crate::livelihood::WORK_ANSWER_DRIFTS_AFTER_DAYS * day
        {
            return Some(("take_jonas_on", crate::model::MARA, Some(asked.id)));
        }
    }

    None
}

struct WithdrawBacking;

impl Action for WithdrawBacking {
    fn name(&self) -> &'static str {
        "withdraw_backing"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        if backing_status(state) == BACKING_WITHDRAWN {
            return Err(ActionError::Invalid(
                "Leo has already put his backing elsewhere".into(),
            ));
        }
        if text_component(state, JONAS_BOAT, CONDITION)? != "damaged" {
            return Err(ActionError::Invalid(
                "there is nothing for Leo to be backing".into(),
            ));
        }

        let mut draft = EventDraft::new("backing_withdrawn");
        draft.actor = Some(LEO);
        draft.targets = vec![LEO, JONAS, JONAS_BOAT, PUB];
        draft
            .payload
            .insert("amount".into(), SEA_FINCH_REPAIR_COST.into());
        draft.changes.push(StateChange::SetComponent {
            entity: JONAS_BOAT,
            key: BACKING_STATUS.into(),
            value: BACKING_WITHDRAWN.into(),
        });
        record_decider(&mut draft, request);
        Ok(draft)
    }
}

struct SellSeaFinch;

impl Action for SellSeaFinch {
    fn name(&self) -> &'static str {
        "sell_sea_finch"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        if !sea_finch_can_be_sold(state) {
            return Err(ActionError::Invalid(
                "Sea Finch is not Jonas's to sell for scrap right now".into(),
            ));
        }
        let cash = integer_component(state, JONAS, CASH)?;
        let after = cash
            .checked_add(SEA_FINCH_SCRAP_VALUE)
            .ok_or_else(|| ActionError::Invalid("Jonas cash overflow".into()))?;

        let mut draft = EventDraft::new("boat_sold");
        draft.actor = Some(JONAS);
        draft.targets = vec![JONAS, JONAS_BOAT];
        draft
            .payload
            .insert("amount".into(), SEA_FINCH_SCRAP_VALUE.into());
        draft.changes = vec![
            StateChange::SetComponent {
                entity: JONAS_BOAT,
                key: CONDITION.into(),
                value: "sold".into(),
            },
            // Selling her ends the owning. Leaving the relation standing left
            // Jonas's card reading "Owns · Sea Finch" for the rest of the
            // World, which is the sort of thing that makes a whole app feel
            // like it is not really keeping track.
            StateChange::RemoveRelation(crate::model::JONAS_BOAT_OWNER),
            StateChange::SetComponent {
                entity: JONAS,
                key: CASH.into(),
                value: after.into(),
            },
        ];
        record_decider(&mut draft, request);
        Ok(draft)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{TinySociety, REPAIR_BOAT_COMMAND, SELL_BOAT_COMMAND};

    fn kinds(branch: &crate::TinySocietyBranch) -> Vec<String> {
        branch
            .world()
            .events()
            .iter()
            .map(|event| event.kind.clone())
            .collect()
    }

    #[test]
    fn an_offer_nobody_answers_does_not_stand_forever() {
        let mut society = TinySociety::new().unwrap();
        society.run_story().unwrap();
        let mut branch = society.branch();

        // Far enough for Leo to have helped, which is what opens the offer.
        branch.advance_days(14).unwrap();
        assert!(
            crate::projection::repair_offer_is_open(branch.world()),
            "Leo's backing is on the table once he has helped"
        );

        branch.advance_days(BACKING_LAPSES_AFTER_DAYS + 1).unwrap();
        assert!(
            kinds(&branch).contains(&"backing_withdrawn".to_string()),
            "an unanswered offer lapses"
        );
        assert!(
            !crate::projection::repair_offer_is_open(branch.world()),
            "a lapsed offer stops being offered"
        );
        assert!(
            !branch
                .projection_snapshot()
                .commands
                .iter()
                .any(|command| command.id == REPAIR_BOAT_COMMAND),
            "a lapsed offer is not still a button"
        );
    }

    #[test]
    fn answering_in_time_keeps_the_offer() {
        let mut society = TinySociety::new().unwrap();
        society.run_story().unwrap();
        let mut branch = society.branch();
        branch.advance_days(14).unwrap();
        branch
            .invoke_projection_command(REPAIR_BOAT_COMMAND)
            .unwrap();

        // Well past the deadline the offer would have lapsed at.
        branch.advance_days(BACKING_LAPSES_AFTER_DAYS * 3).unwrap();
        assert!(
            !kinds(&branch).contains(&"backing_withdrawn".to_string()),
            "an answered offer never lapses behind the person who answered it"
        );
    }

    #[test]
    fn a_lapsed_offer_leaves_a_harder_question_rather_than_nothing() {
        let mut society = TinySociety::new().unwrap();
        society.run_story().unwrap();
        let mut branch = society.branch();
        branch
            .advance_days(14 + BACKING_LAPSES_AFTER_DAYS + 1)
            .unwrap();

        let commands = branch.projection_snapshot().commands;
        assert!(
            commands
                .iter()
                .any(|command| command.id == SELL_BOAT_COMMAND),
            "losing the backing opens the question of what the boat is worth, got {:?}",
            commands.iter().map(|c| &c.id).collect::<Vec<_>>()
        );
    }

    #[test]
    fn what_drifts_is_as_durable_as_what_is_chosen() {
        let mut society = TinySociety::new().unwrap();
        society.run_story().unwrap();
        let mut branch = society.branch();
        branch
            .advance_days(14 + BACKING_LAPSES_AFTER_DAYS + SALE_DRIFTS_AFTER_DAYS + 2)
            .unwrap();

        let sold = branch
            .world()
            .events()
            .iter()
            .find(|event| event.kind == "boat_sold")
            .expect("a destitute man does not sit beside an unusable boat forever");
        assert!(was_drifted(sold), "the World answered this one itself");
        assert_eq!(
            text_component(branch.world().state(), JONAS_BOAT, CONDITION).unwrap(),
            "sold"
        );
        assert!(
            !branch
                .projection_snapshot()
                .commands
                .iter()
                .any(|command| command.id == REPAIR_BOAT_COMMAND),
            "a sold boat cannot be repaired, whoever sold her"
        );
    }

    #[test]
    fn the_world_answers_one_question_a_day_rather_than_all_of_them_at_once() {
        let mut society = TinySociety::new().unwrap();
        society.run_story().unwrap();
        let mut branch = society.branch();
        branch
            .advance_days(14 + BACKING_LAPSES_AFTER_DAYS + SALE_DRIFTS_AFTER_DAYS + 12)
            .unwrap();

        let drifted = branch
            .world()
            .events()
            .iter()
            .filter(|event| was_drifted(event))
            .collect::<Vec<_>>();
        assert!(
            drifted.len() >= 2,
            "a long absence reaches more than one default"
        );
        let mut times = drifted
            .iter()
            .map(|event| event.world_time)
            .collect::<Vec<_>>();
        times.dedup();
        assert_eq!(
            times.len(),
            drifted.len(),
            "no two defaults are reached at the same moment, got {times:?}"
        );
    }
}
