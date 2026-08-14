use pocket_universe::{
    PocketUniverse, BOLD_PATH_COMMAND, NUDGE_COMMAND, OUTWARD_POSTURE_COMMAND,
    ROOTED_POSTURE_COMMAND, SEED_MARS_COLONY_COMMAND, SHARED_PROJECT_COMMAND,
};
use std::error::Error;
use world_compare::{compare_snapshots, DifferenceKind, EntityDifference};
use world_persistence::ArchivedValue;

fn row<'a>(difference: &'a EntityDifference, label: &str) -> Option<(&'a str, &'a str)> {
    let row = difference
        .inspector_rows
        .iter()
        .find(|row| row.key.label == label)?;
    Some((row.left.as_deref()?, row.right.as_deref()?))
}

fn second_chapter_source() -> Result<PocketUniverse, Box<dyn Error>> {
    let mut source = PocketUniverse::new()?;
    source.invoke_projection_command(SEED_MARS_COLONY_COMMAND)?;
    source.advance_periods(2)?;
    source.invoke_projection_command(SHARED_PROJECT_COMMAND)?;
    source.advance_periods(1)?;
    source.invoke_projection_command(BOLD_PATH_COMMAND)?;
    source.advance_periods(3)?;
    assert!(source
        .projection_snapshot()
        .command(OUTWARD_POSTURE_COMMAND)
        .is_some());
    Ok(source)
}

#[test]
fn legacy_forms_after_lived_posture_and_survives_reopen() -> Result<(), Box<dyn Error>> {
    let mut universe = second_chapter_source()?;
    universe.invoke_projection_command(OUTWARD_POSTURE_COMMAND)?;

    universe.advance_periods(2)?;
    assert!(!universe
        .archive()?
        .events
        .iter()
        .any(|event| event.kind == "world_legacy_formed"));

    universe.advance_periods(1)?;
    let archive = universe.archive()?;
    let legacy = archive
        .events
        .iter()
        .find(|event| event.kind == "world_legacy_formed")
        .expect("legacy should form after three lived periods under the posture");
    assert_eq!(
        legacy.payload.get("legacy"),
        Some(&ArchivedValue::Text("ridge-network".into()))
    );
    let summary = match legacy.payload.get("summary") {
        Some(ArchivedValue::Text(summary)) => summary,
        other => panic!("expected semantic legacy summary, got {other:?}"),
    };
    assert!(summary.contains("signal expedition"));
    assert!(summary.contains("care /"));
    assert!(summary.contains("explore"));

    let cause_kinds = legacy
        .caused_by
        .iter()
        .filter_map(|id| archive.events.iter().find(|event| event.id == *id))
        .map(|event| event.kind.as_str())
        .collect::<Vec<_>>();
    assert!(cause_kinds.contains(&"world_posture_chosen"));
    assert!(cause_kinds.contains(&"partnership_formed"));
    assert!(cause_kinds.contains(&"universe_intervened"));
    assert!(cause_kinds.contains(&"relationship_shifted"));

    let snapshot = universe.projection_snapshot();
    let briefing = snapshot
        .briefing
        .as_ref()
        .expect("Pocket Universe should keep its persistent briefing");
    assert!(briefing
        .items
        .iter()
        .any(|item| item.title == "World legacy · Ridge Network"));

    let mut reopened = PocketUniverse::resume_archive(&archive)?;
    assert_eq!(reopened.archive()?, archive);
    assert_eq!(reopened.projection_snapshot(), snapshot);

    reopened.invoke_projection_command(NUDGE_COMMAND)?;
    let after = reopened.archive()?;
    let growth = after
        .events
        .iter()
        .rev()
        .find(|event| event.kind == "universe_grew")
        .expect("the reopened World should keep growing");
    let change = match growth.payload.get("change") {
        Some(ArchivedValue::Text(change)) => change,
        other => panic!("expected growth change text, got {other:?}"),
    };
    assert!(change.contains("ridge network"));

    Ok(())
}

#[test]
fn posture_forks_compound_into_different_emergent_legacies() -> Result<(), Box<dyn Error>> {
    let source = second_chapter_source()?;
    let common_archive = source.archive()?;

    let mut outward = PocketUniverse::resume_archive(&common_archive)?;
    let mut rooted = PocketUniverse::resume_archive(&common_archive)?;
    outward.invoke_projection_command(OUTWARD_POSTURE_COMMAND)?;
    rooted.invoke_projection_command(ROOTED_POSTURE_COMMAND)?;
    outward.advance_periods(3)?;
    rooted.advance_periods(3)?;

    let outward_archive = outward.archive()?;
    let rooted_archive = rooted.archive()?;
    let outward = PocketUniverse::resume_archive(&outward_archive)?;
    let rooted = PocketUniverse::resume_archive(&rooted_archive)?;
    let left = outward.projection_snapshot();
    let right = rooted.projection_snapshot();
    let comparison = compare_snapshots(&left, &right);

    let universe = comparison
        .entities
        .iter()
        .find(|entity| {
            entity.kind == DifferenceKind::Changed
                && entity.left.as_ref().map(|view| view.title.as_str())
                    == Some("Ares Pocket Colony")
        })
        .expect("generic comparison should expose the changed World entity");
    assert_eq!(
        row(universe, "Legacy"),
        Some(("ridge-network", "habitat-commons"))
    );

    assert!(comparison.timeline.changed.iter().any(|event| {
        event.left.title == "World Legacy Formed"
            && event.right.title == "World Legacy Formed"
            && event.left.subtitle != event.right.subtitle
    }));

    let outward_briefing = left.briefing.as_ref().expect("left briefing");
    let rooted_briefing = right.briefing.as_ref().expect("right briefing");
    assert!(outward_briefing
        .items
        .iter()
        .any(|item| item.title == "World legacy · Ridge Network"));
    assert!(rooted_briefing
        .items
        .iter()
        .any(|item| item.title == "World legacy · Habitat Commons"));

    Ok(())
}
