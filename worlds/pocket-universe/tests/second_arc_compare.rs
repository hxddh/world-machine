use pocket_universe::{
    PocketUniverse, BOLD_PATH_COMMAND, OUTWARD_POSTURE_COMMAND, ROOTED_POSTURE_COMMAND,
    SEED_MARS_COLONY_COMMAND, SHARED_PROJECT_COMMAND,
};
use std::error::Error;
use world_compare::{compare_snapshots, DifferenceKind, EntityDifference};

fn row<'a>(difference: &'a EntityDifference, label: &str) -> Option<(&'a str, &'a str)> {
    difference
        .inspector_rows
        .iter()
        .find(|row| row.key.label == label)
        .and_then(|row| Some((row.left.as_deref()?, row.right.as_deref()?)))
}

#[test]
fn second_arc_is_a_durable_generic_strategy_fork() -> Result<(), Box<dyn Error>> {
    let mut source = PocketUniverse::new()?;
    source.invoke_projection_command(SEED_MARS_COLONY_COMMAND)?;

    source.advance_periods(2)?;
    assert!(source
        .projection_snapshot()
        .command(SHARED_PROJECT_COMMAND)
        .is_some());
    source.invoke_projection_command(SHARED_PROJECT_COMMAND)?;

    source.advance_periods(1)?;
    assert!(source
        .projection_snapshot()
        .command(BOLD_PATH_COMMAND)
        .is_some());
    source.invoke_projection_command(BOLD_PATH_COMMAND)?;

    source.advance_periods(3)?;
    let second_chapter = source.projection_snapshot();
    assert!(second_chapter.command(OUTWARD_POSTURE_COMMAND).is_some());
    assert!(second_chapter.command(ROOTED_POSTURE_COMMAND).is_some());

    let common_archive = source.archive()?;
    let common_event_count = common_archive.events.len();

    let mut outward = PocketUniverse::resume_archive(&common_archive)?;
    let mut rooted = PocketUniverse::resume_archive(&common_archive)?;
    outward.invoke_projection_command(OUTWARD_POSTURE_COMMAND)?;
    rooted.invoke_projection_command(ROOTED_POSTURE_COMMAND)?;
    outward.advance_periods(2)?;
    rooted.advance_periods(2)?;

    // Compare durable histories, not two transient in-memory branches.
    let outward_archive = outward.archive()?;
    let rooted_archive = rooted.archive()?;
    assert_eq!(common_archive.events.len(), common_event_count);
    let outward = PocketUniverse::resume_archive(&outward_archive)?;
    let rooted = PocketUniverse::resume_archive(&rooted_archive)?;

    let left = outward.projection_snapshot();
    let right = rooted.projection_snapshot();
    let comparison = compare_snapshots(&left, &right);

    assert!(!comparison.is_identical());
    assert_eq!(comparison.left.world_time, comparison.right.world_time);

    let universe = comparison
        .entities
        .iter()
        .find(|entity| {
            entity.kind == DifferenceKind::Changed
                && entity.left.as_ref().map(|view| view.title.as_str())
                    == Some("Ares Pocket Colony")
        })
        .expect("generic comparison should expose the changed World entity");
    assert_eq!(row(universe, "Posture"), Some(("outward", "rooted")));

    let nia = comparison
        .entities
        .iter()
        .find(|entity| {
            entity.kind == DifferenceKind::Changed
                && entity.left.as_ref().map(|view| view.title.as_str()) == Some("Nia Chen")
        })
        .expect("posture should produce a visible behavioral difference for Nia");
    assert_ne!(row(nia, "Care count"), None);
    assert_ne!(row(nia, "Explore count"), None);
    assert_ne!(row(nia, "Care count").unwrap().0, row(nia, "Care count").unwrap().1);
    assert_ne!(
        row(nia, "Explore count").unwrap().0,
        row(nia, "Explore count").unwrap().1
    );

    assert!(comparison.timeline.changed.iter().any(|event| {
        event.left.title == "World posture chosen"
            && event.right.title == "World posture chosen"
            && event.left.subtitle != event.right.subtitle
    }));

    let outward_briefing = left.briefing.as_ref().expect("Pocket Universe has a briefing");
    let rooted_briefing = right.briefing.as_ref().expect("Pocket Universe has a briefing");
    assert!(outward_briefing
        .items
        .iter()
        .any(|item| item.title == "World direction · Outward"));
    assert!(rooted_briefing
        .items
        .iter()
        .any(|item| item.title == "World direction · Rooted"));

    Ok(())
}
