use pocket_universe::{
    PocketUniverse, BOLD_PATH_COMMAND, OUTWARD_POSTURE_COMMAND, SEED_MARS_COLONY_COMMAND,
    SHARED_PROJECT_COMMAND,
};
use std::error::Error;
use world_core::EventId;
use world_projection::SelectionId;

#[test]
fn legacy_briefing_selects_its_event_and_exposes_why() -> Result<(), Box<dyn Error>> {
    let mut universe = PocketUniverse::new()?;
    universe.invoke_projection_command(SEED_MARS_COLONY_COMMAND)?;
    universe.advance_periods(2)?;
    universe.invoke_projection_command(SHARED_PROJECT_COMMAND)?;
    universe.advance_periods(1)?;
    universe.invoke_projection_command(BOLD_PATH_COMMAND)?;
    universe.advance_periods(3)?;
    universe.invoke_projection_command(OUTWARD_POSTURE_COMMAND)?;
    universe.advance_periods(3)?;

    let archive = universe.archive()?;
    let legacy_event = archive
        .events
        .iter()
        .find(|event| event.kind == "world_legacy_formed")
        .expect("the World should have formed a legacy");
    let legacy_event_id = EventId::new(legacy_event.id);
    let snapshot = universe.projection_snapshot();
    let legacy_item = snapshot
        .briefing
        .as_ref()
        .expect("Pocket Universe should keep its Briefing")
        .items
        .iter()
        .find(|item| item.title == "World legacy · Ridge Network")
        .expect("the durable legacy should stay visible in Briefing");

    assert_eq!(
        legacy_item.selection,
        Some(SelectionId::Event(legacy_event_id)),
        "clicking the persistent legacy should select the event that formed it"
    );

    let why = snapshot
        .why(legacy_event_id)
        .expect("the selected legacy event should already have a generic Why projection");
    assert_eq!(why.event, legacy_event_id);
    assert_eq!(why.nodes[0].title, "World Legacy Formed");
    assert!(why.nodes[0].subtitle.contains("ridge network"));

    let titles = why
        .nodes
        .iter()
        .map(|node| node.title.as_str())
        .collect::<Vec<_>>();
    assert!(titles.contains(&"World Posture Chosen"));
    assert!(titles.contains(&"Partnership Formed"));
    assert!(titles.contains(&"Universe Intervened"));
    assert!(titles.contains(&"Relationship Shifted"));

    let reopened = PocketUniverse::resume_archive(&archive)?;
    let reopened_snapshot = reopened.projection_snapshot();
    let reopened_legacy = reopened_snapshot
        .briefing
        .as_ref()
        .expect("reopened Pocket Universe should keep its Briefing")
        .items
        .iter()
        .find(|item| item.title == "World legacy · Ridge Network")
        .expect("reopened World should keep its legacy entrypoint");
    assert_eq!(reopened_legacy.selection, legacy_item.selection);
    assert_eq!(
        reopened_snapshot.why(legacy_event_id),
        Some(why),
        "archive/reopen should preserve the same causal explanation"
    );

    Ok(())
}
