use pocket_universe::{
    PocketUniverse, BOLD_PATH_COMMAND, OUTWARD_POSTURE_COMMAND, ROOTED_POSTURE_COMMAND,
    SEED_MARS_COLONY_COMMAND, SHARED_PROJECT_COMMAND,
};
use std::error::Error;
use world_compare::{compare_divergence, compare_snapshots, DifferenceKind, EntityDifference};

fn row<'a>(difference: &'a EntityDifference, label: &str) -> Option<(&'a str, &'a str)> {
    let row = difference
        .inspector_rows
        .iter()
        .find(|row| row.key.label == label)?;
    Some((row.left.as_deref()?, row.right.as_deref()?))
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
    let untouched_source = common_archive.clone();

    let mut outward = PocketUniverse::resume_archive(&common_archive)?;
    let mut rooted = PocketUniverse::resume_archive(&common_archive)?;
    outward.invoke_projection_command(OUTWARD_POSTURE_COMMAND)?;
    rooted.invoke_projection_command(ROOTED_POSTURE_COMMAND)?;
    outward.advance_periods(2)?;
    rooted.advance_periods(2)?;

    // Compare durable histories, not two transient in-memory branches.
    let outward_archive = outward.archive()?;
    let rooted_archive = rooted.archive()?;
    assert_eq!(common_archive, untouched_source);
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
    let care_count = row(nia, "Care Count").expect("Nia's care count should differ");
    let explore_count = row(nia, "Explore Count").expect("Nia's explore count should differ");
    assert_ne!(care_count.0, care_count.1);
    assert_ne!(explore_count.0, explore_count.1);

    assert!(comparison.timeline.changed.iter().any(|event| {
        event.left.title == "World Posture Chosen"
            && event.right.title == "World Posture Chosen"
            && event.left.subtitle != event.right.subtitle
    }));

    let divergence =
        compare_divergence(&left, &right).expect("the two durable second-arc futures must diverge");
    let shared_frontier = divergence
        .shared_frontier
        .as_ref()
        .expect("both futures share the full history before the posture choice");
    assert_ne!(shared_frontier.title, "World Posture Chosen");
    let left_first = divergence
        .left
        .first_difference
        .as_ref()
        .expect("outward future has a first difference");
    let right_first = divergence
        .right
        .first_difference
        .as_ref()
        .expect("rooted future has a first difference");
    assert_eq!(left_first.title, "World Posture Chosen");
    assert_eq!(right_first.title, "World Posture Chosen");
    assert_ne!(left_first.subtitle, right_first.subtitle);
    assert!(!divergence.left.impact.is_empty());
    assert!(!divergence.right.impact.is_empty());
    assert!(divergence
        .left
        .impact
        .iter()
        .all(|stage| stage.event.title != "Agent Decision Recorded"));
    assert!(divergence
        .right
        .impact
        .iter()
        .all(|stage| stage.event.title != "Agent Decision Recorded"));
    assert_ne!(
        divergence
            .left
            .impact
            .iter()
            .map(|stage| stage.effect.as_str())
            .collect::<Vec<_>>(),
        divergence
            .right
            .impact
            .iter()
            .map(|stage| stage.effect.as_str())
            .collect::<Vec<_>>()
    );

    let outward_briefing = left
        .briefing
        .as_ref()
        .expect("Pocket Universe has a briefing");
    let rooted_briefing = right
        .briefing
        .as_ref()
        .expect("Pocket Universe has a briefing");
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
