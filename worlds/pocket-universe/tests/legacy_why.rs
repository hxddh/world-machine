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
    // The chain opens on the legacy as History tells it.
    assert!(why.nodes[0].title.contains("ridge network"));
    assert!(why.nodes[0].subtitle.contains("ridge network"));

    // What the chain passes through, by recorded kind rather than wording.
    let kinds_in = |archive: &world_persistence::WorldArchive,
                    why: &world_projection::WhyProjection| {
        why.nodes
            .iter()
            .filter_map(|node| {
                archive
                    .events
                    .iter()
                    .find(|event| event.id == node.event.0)
                    .map(|event| event.kind.clone())
            })
            .collect::<Vec<_>>()
    };
    let kinds = kinds_in(&archive, why);
    for kind in [
        "world_posture_chosen",
        "partnership_formed",
        "universe_intervened",
        "relationship_shifted",
    ] {
        assert!(kinds.iter().any(|seen| seen == kind), "missing {kind}");
    }

    let mut reopened = PocketUniverse::resume_archive(&archive)?;
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

    reopened.advance_periods(1)?;
    let reinforced_archive = reopened.archive()?;
    let reinforced = reinforced_archive
        .events
        .iter()
        .rev()
        .find(|event| event.kind == "legacy_reinforced")
        .expect("the following period should reinforce the legacy");
    let reinforced_event_id = EventId::new(reinforced.id);
    let reinforced_snapshot = reopened.projection_snapshot();
    let reinforced_legacy_item = reinforced_snapshot
        .briefing
        .as_ref()
        .expect("reinforced Pocket Universe should keep its Briefing")
        .items
        .iter()
        .find(|item| item.title == "World legacy · Ridge Network")
        .expect("the living legacy should remain visible in Briefing");
    assert_eq!(
        reinforced_legacy_item.selection,
        Some(SelectionId::Event(reinforced_event_id)),
        "after reinforcement, the persistent legacy should open its latest living event"
    );
    assert!(
        reinforced_legacy_item.detail.contains("grew stronger"),
        "the persistent legacy should describe how it last grew"
    );

    let reinforced_why = reinforced_snapshot
        .why(reinforced_event_id)
        .expect("legacy reinforcement should have a generic Why projection");
    let reinforced_kinds = kinds_in(&reinforced_archive, reinforced_why);
    assert_eq!(reinforced_kinds[0], "legacy_reinforced");
    assert!(reinforced_kinds
        .iter()
        .any(|kind| kind == "world_legacy_formed"));
    assert!(reinforced_kinds
        .iter()
        .any(|kind| kind == "relationship_shifted"));

    let reopened_again = PocketUniverse::resume_archive(&reinforced_archive)?;
    let reopened_again_snapshot = reopened_again.projection_snapshot();
    let reopened_again_legacy = reopened_again_snapshot
        .briefing
        .as_ref()
        .expect("reopened reinforced World should keep its Briefing")
        .items
        .iter()
        .find(|item| item.title == "World legacy · Ridge Network")
        .expect("reopened reinforced World should keep its living legacy entrypoint");
    assert_eq!(reopened_again_legacy, reinforced_legacy_item);
    assert_eq!(
        reopened_again_snapshot.why(reinforced_event_id),
        Some(reinforced_why),
        "archive/reopen should preserve the reinforcement explanation"
    );

    Ok(())
}
