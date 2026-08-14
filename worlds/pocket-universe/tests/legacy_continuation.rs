use pocket_universe::{
    PocketUniverse, BOLD_PATH_COMMAND, NUDGE_COMMAND, OUTWARD_POSTURE_COMMAND,
    ROOTED_POSTURE_COMMAND, SEED_MARS_COLONY_COMMAND, SHARED_PROJECT_COMMAND,
};
use std::error::Error;
use world_compare::compare_snapshots;

fn second_chapter_source() -> Result<PocketUniverse, Box<dyn Error>> {
    let mut source = PocketUniverse::new()?;
    source.invoke_projection_command(SEED_MARS_COLONY_COMMAND)?;
    source.advance_periods(2)?;
    source.invoke_projection_command(SHARED_PROJECT_COMMAND)?;
    source.advance_periods(1)?;
    source.invoke_projection_command(BOLD_PATH_COMMAND)?;
    source.advance_periods(3)?;
    Ok(source)
}

#[test]
fn durable_legacy_changes_how_the_same_continuation_action_is_presented(
) -> Result<(), Box<dyn Error>> {
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

    assert_eq!(left.commands.len(), 1);
    assert_eq!(right.commands.len(), 1);
    let outward_nudge = left.command(NUDGE_COMMAND).expect("outward continuation");
    let rooted_nudge = right.command(NUDGE_COMMAND).expect("rooted continuation");
    assert_eq!(outward_nudge.id, rooted_nudge.id);
    assert!(outward_nudge.title.contains("ridge network"));
    assert!(outward_nudge.detail.contains("ridge"));
    assert!(rooted_nudge.title.contains("habitat commons"));
    assert!(rooted_nudge.detail.contains("commons"));
    assert_ne!(outward_nudge.title, rooted_nudge.title);
    assert_ne!(outward_nudge.detail, rooted_nudge.detail);

    let comparison = compare_snapshots(&left, &right);
    let continuation = comparison
        .commands
        .changed
        .iter()
        .find(|command| command.id == NUDGE_COMMAND)
        .expect("generic comparison should expose the changed continuation action");
    assert_eq!(continuation.left.title, outward_nudge.title);
    assert_eq!(continuation.right.title, rooted_nudge.title);

    Ok(())
}
