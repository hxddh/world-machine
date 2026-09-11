//! People are where their work is, and they move when it changes.
//!
//! Every resident's `location` used to be written once, when the World was
//! seeded, and never again. Mara stood in front of her own shut bakery for
//! the rest of the World; Jonas stood at a harbour he no longer had a boat
//! in. A drawing of the town could only ever change because the *things* in
//! it changed, never because anybody walked, and a reader who noticed that
//! people never moved was noticing something true.
//!
//! Where somebody should be is not guessed from their job title. The World
//! already records employment as a `works_at` relation, kept up to date by
//! everything that hires, dismisses or closes, so a resident stands at the
//! place they work at. Somebody with no such record is left where they are
//! until the place itself shuts, and then they are on the quay, which is
//! where a town without work collects.
//!
//! That second rule is not tidiness. Mia is a student: she is at the school
//! and has no employment relation at all, and a rule that read "no work means
//! the quay" marched a child down to the docks on the first morning. The test
//! below caught it before it shipped.

use crate::model::HARBOR;
use crate::model::LOCATION;
use std::error::Error;
use world_core::{
    Action, ActionError, ActionRegistry, ActionRequest, EntityId, EventDraft, EventId, StateChange,
    Value, World, WorldState,
};

pub(crate) const WORKS_AT: &str = "works_at";

pub(crate) fn register_actions(registry: &mut ActionRegistry) -> Result<(), ActionError> {
    registry.register(Move)
}

/// Where a resident belongs today, or `None` when the World has no opinion
/// and they should be left alone.
fn belongs_at(state: &WorldState, resident: EntityId) -> Option<EntityId> {
    if let Some(workplace) = state
        .relations()
        .find(|relation| relation.kind == WORKS_AT && relation.from == resident)
        .map(|relation| relation.to)
    {
        return Some(workplace);
    }
    // No work recorded. Stay put unless the place itself has shut.
    let here = standing_at(state, resident)?;
    let shut = matches!(
        state.entity(here).and_then(|place| place.component(crate::model::OPERATING_STATUS)),
        Some(Value::Text(status)) if status != "open"
    );
    shut.then_some(HARBOR)
}

fn standing_at(state: &WorldState, resident: EntityId) -> Option<EntityId> {
    match state.entity(resident)?.component(LOCATION) {
        Some(Value::Entity(place)) => Some(*place),
        _ => None,
    }
}

/// Move anyone whose whereabouts no longer match their work.
///
/// Checked once a day alongside the other things people do because of how
/// things stand rather than in reaction to one Event. Everyone who needs to
/// move moves the same day: when the bakery shuts, both the baker and the
/// counter hand leave it, and a picture that moved only one of them would be
/// lying about the other.
pub(crate) fn follow_work(
    world: &mut World,
    actions: &ActionRegistry,
) -> Result<Vec<EventId>, Box<dyn Error>> {
    let moves: Vec<(EntityId, EntityId)> = crate::projection::RESIDENTS
        .iter()
        .filter_map(|resident| {
            let belongs = belongs_at(world.state(), *resident)?;
            (standing_at(world.state(), *resident)? != belongs).then_some((*resident, belongs))
        })
        .collect();

    let mut events = Vec::new();
    for (resident, destination) in moves {
        events.push(
            world
                .execute(
                    actions,
                    &ActionRequest::new("move_resident")
                        .actor(resident)
                        .arg("resident", resident)
                        .arg("destination", destination),
                )?
                .id,
        );
    }
    Ok(events)
}

struct Move;

impl Action for Move {
    fn name(&self) -> &'static str {
        "move_resident"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let resident = entity_arg(request, "resident")?;
        let destination = entity_arg(request, "destination")?;
        let from = standing_at(state, resident).ok_or_else(|| {
            ActionError::Invalid(format!("resident {resident} is nowhere to begin with"))
        })?;
        if from == destination {
            return Err(ActionError::Invalid(format!(
                "resident {resident} is already there"
            )));
        }

        let mut draft = EventDraft::new("resident_moved");
        draft.actor = Some(resident);
        draft.targets = vec![resident, destination];
        draft.payload.insert("from".into(), Value::Entity(from));
        draft
            .payload
            .insert("to".into(), Value::Entity(destination));
        draft.changes.push(StateChange::SetComponent {
            entity: resident,
            key: LOCATION.into(),
            value: Value::Entity(destination),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{BAKERY, HARBOR, MARA};
    use crate::TinySociety;

    fn where_is(branch: &crate::TinySocietyBranch, resident: EntityId) -> EntityId {
        standing_at(branch.world().state(), resident).expect("a resident is somewhere")
    }

    /// The baker does not stand at a bakery that has shut.
    #[test]
    fn work_ending_moves_the_person_who_did_it() {
        let mut society = TinySociety::new().unwrap();
        society.run_story().unwrap();
        let mut branch = society.branch();

        assert_eq!(where_is(&branch, MARA), BAKERY, "she starts at her shop");

        branch.advance_days(80).unwrap();
        assert_eq!(
            where_is(&branch, MARA),
            HARBOR,
            "the bakery shut, so she is on the quay"
        );
        assert!(
            branch
                .world()
                .events()
                .iter()
                .any(|event| event.kind == "resident_moved" && event.actor == Some(MARA)),
            "and the World recorded her moving, rather than the component quietly changing"
        );
    }

    /// Whereabouts follow the employment the World records, not a table of job
    /// titles kept in step by hand.
    #[test]
    fn a_resident_stands_where_the_world_says_they_work() {
        let mut society = TinySociety::new().unwrap();
        society.run_story().unwrap();
        let branch = society.branch();
        let state = branch.world().state();

        for resident in crate::projection::RESIDENTS {
            let Some(expected) = belongs_at(state, resident) else {
                continue;
            };
            assert_eq!(
                where_is(&branch, resident),
                expected,
                "resident {resident} is not where their work is"
            );
        }
    }

    /// A student is not an unemployed adult.
    ///
    /// Mia has no employment relation, and the first version of this rule read
    /// "no work means the quay", which sent her to the docks on day one.
    #[test]
    fn a_child_at_school_is_left_where_she_is() {
        use crate::model::{MIA, SCHOOL};
        let mut society = TinySociety::new().unwrap();
        society.run_story().unwrap();
        let mut branch = society.branch();

        branch.advance_days(20).unwrap();
        assert_eq!(
            where_is(&branch, MIA),
            SCHOOL,
            "the school is open and she is at it"
        );

        // Once the school itself shuts there is nothing to be at, and she is
        // on the quay with everyone else.
        branch.advance_days(70).unwrap();
        assert_eq!(where_is(&branch, MIA), HARBOR);
    }

    /// Nobody is left standing inside a shuttered building.
    ///
    /// A screenshot caught this before any test did: the pub was drawn shut
    /// and Sofia was still in it, alone, while the other seven residents had
    /// walked down to the quay. The closure ended the job it happened to name
    /// and left hers recorded, so the World still believed she worked there.
    #[test]
    fn a_shut_building_is_empty() {
        use crate::model::OPERATING_STATUS;
        let mut society = TinySociety::new().unwrap();
        society.run_story().unwrap();
        let mut branch = society.branch();
        branch.advance_days(90).unwrap();

        let state = branch.world().state();
        for resident in crate::projection::RESIDENTS {
            let here = standing_at(state, resident).expect("a resident is somewhere");
            let shut = matches!(
                state.entity(here).and_then(|place| place.component(OPERATING_STATUS)),
                Some(Value::Text(status)) if status != "open"
            );
            assert!(
                !shut,
                "resident {resident} is standing inside a building that has closed"
            );
        }
    }
}
