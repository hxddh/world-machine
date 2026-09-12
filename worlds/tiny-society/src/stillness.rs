//! When the town has nothing left to do, it says so.
//!
//! Harbour Town's only export is Jonas's catch. Once the boat is gone the
//! money that came in from the mainland stops, every workplace drains its
//! reserve, and the day arrives when no shift can be paid for, no bread can
//! be bought and nobody can afford to live. That ending is the World working
//! as designed — the README says the choice about the boat decides it.
//!
//! What was not by design is that the World then went quiet without a word.
//! It recorded its last event and carried on advancing time for as long as
//! anyone asked, producing nothing, while the return briefing kept showing
//! the same stale card and two offers that looked as live as they had on the
//! first day. A person coming back could not tell a town that had settled
//! from one that was about to do something.
//!
//! Coming to rest is a thing that happens to the town, so it is recorded like
//! anything else that happens to the town. It is not a tombstone: reopening
//! the bakery wakes the World again, and when the money runs out a second
//! time it comes to rest a second time.

use std::error::Error;
use world_core::{Action, ActionError, ActionRegistry, ActionRequest, EventDraft, WorldState};

/// The kind recorded when a day passes and the World does nothing at all.
pub(crate) const CAME_TO_REST: &str = "world_came_to_rest";

pub(crate) fn register_actions(registry: &mut ActionRegistry) -> Result<(), ActionError> {
    registry.register(RecordCameToRest)
}

struct RecordCameToRest;

impl Action for RecordCameToRest {
    fn name(&self) -> &'static str {
        "record_world_came_to_rest"
    }

    fn evaluate(
        &self,
        _state: &WorldState,
        _request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        Ok(EventDraft::new(CAME_TO_REST))
    }
}

/// Record that the town has settled, if a day has just passed in which it did
/// nothing and it was not already at rest.
///
/// Called with the events a single day produced. An empty day is the whole
/// test: a World with work to do produces something, every time.
pub(crate) fn note_if_at_rest(
    world: &mut world_core::World,
    actions: &ActionRegistry,
    day_produced_events: bool,
) -> Result<Option<world_core::EventId>, Box<dyn Error>> {
    if day_produced_events {
        return Ok(None);
    }
    // Already resting. Saying it again every day would make a quiet town the
    // noisiest thing in the history.
    if world
        .events()
        .last()
        .is_some_and(|event| event.kind == CAME_TO_REST)
    {
        return Ok(None);
    }
    let request = ActionRequest::new("record_world_came_to_rest");
    Ok(Some(world.execute(actions, &request)?.id))
}

#[cfg(test)]
mod tests {
    use super::CAME_TO_REST;
    use crate::{TinySociety, REOPEN_BAKERY_COMMAND};

    /// Run until the town has nothing left to do, rather than to a day
    /// number. How long Harbour Town lasts is a property of its economy and
    /// changes whenever that does — it moved from world time 790 to 860 the
    /// day the pub got customers — so a test that names a day is a test that
    /// has to be re-guessed, and re-guessing is indistinguishable from tuning
    /// it until it agrees.
    fn until_it_settles(branch: &mut crate::TinySocietyBranch) {
        for _ in 0..400 {
            if rests(branch) > 0 {
                return;
            }
            branch.advance_days(1).unwrap();
        }
        panic!("this World never came to rest");
    }

    fn rests(branch: &crate::TinySocietyBranch) -> usize {
        branch
            .world()
            .events()
            .iter()
            .filter(|event| event.kind == CAME_TO_REST)
            .count()
    }

    /// A town with nothing left to do says so, and says it once.
    #[test]
    fn a_settled_town_says_so_rather_than_going_quiet() {
        let mut society = TinySociety::new().unwrap();
        society.run_story().unwrap();
        let mut branch = society.branch();
        until_it_settles(&mut branch);

        assert_eq!(rests(&branch), 1, "the town settled and said so");
        let settled = branch.world().events().len();

        // Left alone for a very long time it stays settled and stays quiet:
        // a resting town is not the noisiest thing in its own history.
        branch.advance_days(200).unwrap();
        assert_eq!(rests(&branch), 1);
        assert_eq!(
            branch.world().events().len(),
            settled,
            "nothing further is recorded while the town rests"
        );

        let briefing = branch
            .projection_snapshot()
            .briefing
            .expect("a return briefing");
        assert!(
            briefing
                .items
                .iter()
                .any(|item| item.title == "Harbour Town has come to rest"),
            "the return says the town has settled: {:?}",
            briefing.items.iter().map(|i| &i.title).collect::<Vec<_>>()
        );
    }

    /// Coming to rest is something that happens, not the end of the World.
    ///
    /// Reopening the bakery wakes the town; when that money runs out it
    /// settles a second time, and the second settling is recorded too.
    #[test]
    fn a_resting_town_can_be_woken_and_can_settle_again() {
        let mut society = TinySociety::new().unwrap();
        society.run_story().unwrap();
        let mut branch = society.branch();
        until_it_settles(&mut branch);
        assert_eq!(rests(&branch), 1);
        let asleep = branch.world().events().len();

        branch
            .invoke_projection_command(REOPEN_BAKERY_COMMAND)
            .expect("the offer is still on the table");
        branch.advance_days(60).unwrap();

        assert!(
            branch.world().events().len() > asleep + 1,
            "reopening put the town back to work"
        );
        assert_eq!(
            rests(&branch),
            2,
            "and when that ran out it settled again, which is recorded"
        );
    }
}
