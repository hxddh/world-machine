use pocket_universe::{
    PocketUniverse, BOLD_PATH_COMMAND, NUDGE_COMMAND, SEED_MARS_COLONY_COMMAND,
    SHARED_PROJECT_COMMAND,
};
use std::error::Error;

fn influence_signature(
    snapshot: &world_projection::ProjectionSnapshot,
    event: world_core::EventId,
) -> Vec<(usize, world_projection::SelectionId, String)> {
    snapshot
        .influence(event)
        .into_iter()
        .map(|(depth, item)| (depth, item.id, item.title.clone()))
        .collect()
}

#[test]
fn old_choices_expose_the_later_events_they_actually_influenced() -> Result<(), Box<dyn Error>> {
    let mut universe = PocketUniverse::new()?;
    universe.invoke_projection_command(SEED_MARS_COLONY_COMMAND)?;
    universe.advance_periods(2)?;
    let relationship = universe.invoke_projection_command(SHARED_PROJECT_COMMAND)?;

    universe.invoke_projection_command(NUDGE_COMMAND)?;
    let relationship_snapshot = universe.projection_snapshot();
    let relationship_influence = influence_signature(&relationship_snapshot, relationship);
    assert!(relationship_influence
        .iter()
        .any(|(_, _, title)| title == "Relationship Shifted"));
    assert!(relationship_influence
        .iter()
        .any(|(_, _, title)| title == "Agent Decision Recorded"));
    assert!(
        relationship_influence
            .iter()
            .filter(|(depth, _, _)| *depth == 1)
            .count()
            >= 2
    );

    let intervention = universe.invoke_projection_command(BOLD_PATH_COMMAND)?;
    universe.advance_periods(2)?;
    let intervention_snapshot = universe.projection_snapshot();
    let intervention_influence = influence_signature(&intervention_snapshot, intervention);
    assert!(intervention_influence
        .iter()
        .any(|(_, _, title)| title == "Universe Grew"));

    let archive = universe.archive()?;
    let reopened = PocketUniverse::resume_archive(&archive)?;
    assert_eq!(
        influence_signature(&reopened.projection_snapshot(), intervention),
        intervention_influence,
        "archive/reopen must reconstruct the same forward influence from persisted causal Events"
    );

    Ok(())
}
