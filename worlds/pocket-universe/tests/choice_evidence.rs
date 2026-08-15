use pocket_universe::{
    PocketUniverse, BOLD_PATH_COMMAND, OUTWARD_POSTURE_COMMAND, SEED_MARS_COLONY_COMMAND,
    SHARED_PROJECT_COMMAND,
};
use std::error::Error;
use world_core::{StateChange, Value};
use world_projection::SelectionId;

fn choice_evidence<'a>(
    snapshot: &'a world_projection::ProjectionSnapshot,
    title: &str,
) -> &'a world_projection::BriefingItem {
    snapshot
        .briefing
        .as_ref()
        .expect("Pocket Universe should expose a Briefing")
        .items
        .iter()
        .find(|item| item.title == title)
        .unwrap_or_else(|| panic!("missing choice evidence item: {title}"))
}

#[test]
fn relationship_choice_signal_is_verified_by_the_recorded_event() -> Result<(), Box<dyn Error>> {
    let mut universe = PocketUniverse::new()?;
    universe.invoke_projection_command(SEED_MARS_COLONY_COMMAND)?;
    universe.advance_periods(2)?;

    let before = universe.projection_snapshot();
    let signal = before
        .command(SHARED_PROJECT_COMMAND)
        .expect("shared project should be available")
        .detail
        .clone();
    assert!(signal.contains("trust 2 → 4 · tension 0 → 0"));

    let event_id = universe.invoke_projection_command(SHARED_PROJECT_COMMAND)?;
    let event = universe
        .world()
        .events()
        .iter()
        .find(|event| event.id == event_id)
        .expect("relationship choice should record an Event");
    assert_eq!(event.kind, "relationship_steered");
    assert!(event.changes.iter().any(|change| matches!(
        change,
        StateChange::SetComponent { key, value: Value::Integer(4), .. } if key == "trust"
    )));
    assert!(event.changes.iter().any(|change| matches!(
        change,
        StateChange::SetComponent { key, value: Value::Integer(0), .. } if key == "tension"
    )));
    assert!(event.changes.iter().any(|change| matches!(
        change,
        StateChange::SetComponent { key, value: Value::Text(value), .. }
            if key == "direction" && value == "shared-project"
    )));

    let after = universe.projection_snapshot();
    let evidence = choice_evidence(&after, "Choice evidence · Shared project");
    assert_eq!(evidence.selection, Some(SelectionId::Event(event_id)));
    assert!(evidence.detail.contains("trust 2 → 4 · tension 0 → 0"));
    assert!(evidence
        .detail
        .contains("Durable direction = shared project"));
    assert!(evidence.detail.contains("add +1 trust and -1 tension"));
    assert!(after.inspector(SelectionId::Event(event_id)).is_some());
    assert!(after.why(event_id).is_some());

    Ok(())
}

#[test]
fn intervention_choice_evidence_uses_the_event_statechange_not_current_copy(
) -> Result<(), Box<dyn Error>> {
    let mut universe = PocketUniverse::new()?;
    universe.invoke_projection_command(SEED_MARS_COLONY_COMMAND)?;
    universe.advance_periods(3)?;

    let before = universe.projection_snapshot();
    let signal = before
        .command(BOLD_PATH_COMMAND)
        .expect("bold intervention should be available")
        .detail
        .clone();
    assert!(signal.contains("Kestrel's durable status becomes signal expedition"));

    let event_id = universe.invoke_projection_command(BOLD_PATH_COMMAND)?;
    let event = universe
        .world()
        .events()
        .iter()
        .find(|event| event.id == event_id)
        .expect("intervention should record an Event");
    assert!(event.changes.iter().any(|change| matches!(
        change,
        StateChange::SetComponent { key, value: Value::Text(value), .. }
            if key == "decision" && value == "follow-signal"
    )));
    assert!(event.changes.iter().any(|change| matches!(
        change,
        StateChange::SetComponent { key, value: Value::Text(value), .. }
            if key == "status" && value == "signal expedition"
    )));

    let after = universe.projection_snapshot();
    let evidence = choice_evidence(&after, "Choice evidence · Signal expedition");
    assert_eq!(evidence.selection, Some(SelectionId::Event(event_id)));
    assert!(evidence
        .detail
        .contains("first intervention = Signal expedition"));
    assert!(evidence
        .detail
        .contains("Kestrel Rover · status = signal expedition"));
    assert!(evidence
        .detail
        .contains("Later growth reads this durable intervention"));

    Ok(())
}

#[test]
fn posture_choice_evidence_survives_archive_and_reopen() -> Result<(), Box<dyn Error>> {
    let mut universe = PocketUniverse::new()?;
    universe.invoke_projection_command(SEED_MARS_COLONY_COMMAND)?;
    universe.advance_periods(2)?;
    universe.invoke_projection_command(SHARED_PROJECT_COMMAND)?;
    universe.advance_periods(1)?;
    universe.invoke_projection_command(BOLD_PATH_COMMAND)?;
    universe.advance_periods(3)?;

    let before = universe.projection_snapshot();
    let signal = before
        .command(OUTWARD_POSTURE_COMMAND)
        .expect("outward posture should be available")
        .detail
        .clone();
    assert!(signal.contains("later growth and legacy formation read the outward posture"));

    let event_id = universe.invoke_projection_command(OUTWARD_POSTURE_COMMAND)?;
    let event = universe
        .world()
        .events()
        .iter()
        .find(|event| event.id == event_id)
        .expect("posture choice should record an Event");
    assert!(event.changes.iter().any(|change| matches!(
        change,
        StateChange::SetComponent { key, value: Value::Text(value), .. }
            if key == "posture" && value == "outward"
    )));
    assert!(event.changes.iter().any(|change| matches!(
        change,
        StateChange::SetComponent { key, value: Value::Integer(6), .. }
            if key == "posture_generation"
    )));

    let after = universe.projection_snapshot();
    let evidence = choice_evidence(&after, "Choice evidence · Outward");
    assert_eq!(evidence.selection, Some(SelectionId::Event(event_id)));
    assert!(evidence
        .detail
        .contains("World direction = Outward at generation 6"));
    assert!(evidence
        .detail
        .contains("Later growth and legacy formation read this durable posture"));

    let archive = universe.archive()?;
    let reopened = PocketUniverse::resume_archive(&archive)?;
    assert_eq!(
        reopened.projection_snapshot(),
        after,
        "choice evidence should be reconstructed from the immutable Event log after reopen"
    );

    Ok(())
}
