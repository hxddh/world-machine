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

fn semantic_influence_signature(
    snapshot: &world_projection::ProjectionSnapshot,
    event: world_core::EventId,
) -> Vec<(usize, world_projection::SelectionId, String)> {
    snapshot
        .semantic_influence(event)
        .into_iter()
        .map(|(depth, item)| (depth, item.id, item.title.clone()))
        .collect()
}

#[test]
fn old_choices_expose_semantic_world_effects_without_erasing_supporting_history(
) -> Result<(), Box<dyn Error>> {
    let mut universe = PocketUniverse::new()?;
    universe.invoke_projection_command(SEED_MARS_COLONY_COMMAND)?;
    universe.advance_periods(2)?;
    let relationship = universe.invoke_projection_command(SHARED_PROJECT_COMMAND)?;

    universe.invoke_projection_command(NUDGE_COMMAND)?;
    let relationship_snapshot = universe.projection_snapshot();
    let raw_relationship = influence_signature(&relationship_snapshot, relationship);
    let semantic_relationship = semantic_influence_signature(&relationship_snapshot, relationship);

    assert!(raw_relationship
        .iter()
        .any(|(_, _, title)| title == "Agent Decision Recorded"));
    assert!(semantic_relationship
        .iter()
        .all(|(_, _, title)| title != "Agent Decision Recorded"));
    assert!(semantic_relationship
        .iter()
        .any(|(_, _, title)| title == "Relationship Shifted"));
    assert!(semantic_relationship.len() < raw_relationship.len());

    let intervention = universe.invoke_projection_command(BOLD_PATH_COMMAND)?;
    universe.advance_periods(2)?;
    let intervention_snapshot = universe.projection_snapshot();
    let raw_intervention = influence_signature(&intervention_snapshot, intervention);
    let semantic_intervention = semantic_influence_signature(&intervention_snapshot, intervention);
    assert!(semantic_intervention
        .iter()
        .any(|(_, _, title)| title == "Universe Grew"));
    assert!(semantic_intervention.len() <= raw_intervention.len());

    let archive = universe.archive()?;
    let reopened = PocketUniverse::resume_archive(&archive)?;
    let reopened_snapshot = reopened.projection_snapshot();
    assert_eq!(
        semantic_influence_signature(&reopened_snapshot, intervention),
        semantic_intervention,
        "archive/reopen must reconstruct the same semantic influence from persisted Events"
    );
    assert_eq!(
        influence_signature(&reopened_snapshot, intervention),
        raw_intervention,
        "semantic folding must not mutate or discard the raw causal history"
    );

    Ok(())
}
