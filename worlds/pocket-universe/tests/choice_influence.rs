use pocket_universe::{
    PocketUniverse, BOLD_PATH_COMMAND, NUDGE_COMMAND, SEED_MARS_COLONY_COMMAND,
    SHARED_PROJECT_COMMAND,
};
use std::error::Error;
use world_projection::SelectionId;

fn influence_values(
    snapshot: &world_projection::ProjectionSnapshot,
    event: world_core::EventId,
) -> Vec<String> {
    snapshot
        .inspector(SelectionId::Event(event))
        .expect("choice Event should have a generic inspector")
        .sections
        .iter()
        .find(|section| section.title == "Influence")
        .expect("choice Event should expose downstream influence")
        .rows
        .iter()
        .map(|row| row.value.clone())
        .collect()
}

#[test]
fn old_choices_show_the_later_events_they_actually_influenced() -> Result<(), Box<dyn Error>> {
    let mut universe = PocketUniverse::new()?;
    universe.invoke_projection_command(SEED_MARS_COLONY_COMMAND)?;
    universe.advance_periods(2)?;
    let relationship = universe.invoke_projection_command(SHARED_PROJECT_COMMAND)?;

    universe.invoke_projection_command(NUDGE_COMMAND)?;
    let relationship_snapshot = universe.projection_snapshot();
    let relationship_values = influence_values(&relationship_snapshot, relationship);
    assert!(relationship_values
        .iter()
        .any(|value| value.contains("Relationship Shifted")));
    assert!(relationship_values
        .iter()
        .any(|value| value.contains("Agent Decision Recorded")));

    let intervention = universe.invoke_projection_command(BOLD_PATH_COMMAND)?;
    universe.advance_periods(2)?;
    let intervention_snapshot = universe.projection_snapshot();
    let intervention_values = influence_values(&intervention_snapshot, intervention);
    assert!(intervention_values
        .iter()
        .any(|value| value.contains("Universe Grew")));

    let archive = universe.archive()?;
    let reopened = PocketUniverse::resume_archive(&archive)?;
    assert_eq!(
        reopened
            .projection_snapshot()
            .inspector(SelectionId::Event(intervention)),
        intervention_snapshot.inspector(SelectionId::Event(intervention)),
        "archive/reopen must reconstruct the same forward influence from persisted causal Events"
    );

    Ok(())
}
