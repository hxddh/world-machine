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

fn semantic_path_signature(
    snapshot: &world_projection::ProjectionSnapshot,
    event: world_core::EventId,
) -> Vec<(world_projection::SelectionId, String)> {
    snapshot
        .semantic_path(event)
        .into_iter()
        .map(|item| (item.id, item.title.clone()))
        .collect()
}

fn semantic_path_detail_signature(
    snapshot: &world_projection::ProjectionSnapshot,
    event: world_core::EventId,
) -> Vec<(usize, world_projection::SelectionId, String, String)> {
    snapshot
        .semantic_path_details(event)
        .into_iter()
        .map(|(steps, item, effect)| (steps, item.id, item.title.clone(), effect))
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

    let relationship_path = semantic_path_signature(&relationship_snapshot, relationship);
    assert!(relationship_path.len() >= 3);
    assert!(relationship_path
        .iter()
        .all(|(_, title)| title != "Agent Decision Recorded"));
    let shifted = relationship_path
        .iter()
        .position(|(_, title)| title == "Relationship Shifted")
        .expect("the compressed thread should include the materialized relationship shift");
    let partnership = relationship_path
        .iter()
        .position(|(_, title)| title == "Partnership Formed")
        .expect("the latest relationship thread should reach the resolved social arc");
    assert!(shifted < partnership);

    let relationship_details = semantic_path_detail_signature(&relationship_snapshot, relationship);
    assert_eq!(relationship_details.len(), relationship_path.len());
    assert!(relationship_details
        .iter()
        .all(|(_, _, title, _)| title != "Agent Decision Recorded"));
    assert!(
        relationship_details
            .iter()
            .any(|(steps, _, _, _)| *steps > 1),
        "the readable thread should report when supporting causal records were folded"
    );
    let shifted_effect = relationship_details
        .iter()
        .find(|(_, _, title, _)| title == "Relationship Shifted")
        .map(|(_, _, _, effect)| effect)
        .expect("relationship shift should carry recorded semantic evidence");
    assert!(shifted_effect.contains("Trust is"));
    assert!(shifted_effect.contains("Recorded state"));
    assert!(shifted_effect.contains("Trust"));
    let partnership_effect = relationship_details
        .iter()
        .find(|(_, _, title, _)| title == "Partnership Formed")
        .map(|(_, _, _, effect)| effect)
        .expect("resolved social arc should carry its recorded summary");
    assert!(partnership_effect.contains("one expedition crew"));

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
        semantic_path_signature(&reopened_snapshot, relationship),
        semantic_path_signature(&intervention_snapshot, relationship),
        "archive/reopen must reconstruct the same compressed causal thread from persisted Events"
    );
    assert_eq!(
        semantic_path_detail_signature(&reopened_snapshot, relationship),
        semantic_path_detail_signature(&intervention_snapshot, relationship),
        "archive/reopen must reconstruct the same recorded causal explanation"
    );
    assert_eq!(
        influence_signature(&reopened_snapshot, intervention),
        raw_intervention,
        "semantic folding must not mutate or discard the raw causal history"
    );

    Ok(())
}
