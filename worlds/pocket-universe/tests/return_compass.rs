use pocket_universe::{
    PocketUniverse, BOLD_PATH_COMMAND, OUTWARD_POSTURE_COMMAND, SEED_MARS_COLONY_COMMAND,
    SHARED_PROJECT_COMMAND,
};
use std::error::Error;

#[test]
fn return_compass_names_every_current_relationship_action() -> Result<(), Box<dyn Error>> {
    let mut universe = PocketUniverse::new()?;
    universe.invoke_projection_command(SEED_MARS_COLONY_COMMAND)?;
    let since = universe.world().events().len();
    universe.advance_periods(2)?;

    let snapshot = universe.projection_snapshot_since(Some(since));
    let briefing = snapshot
        .briefing
        .as_ref()
        .expect("returning Pocket Universe should expose a Briefing");
    let compass = briefing
        .items
        .iter()
        .find(|item| item.title == "Your turn · Relationship")
        .expect("the return digest should surface the currently open relationship choice");

    assert_eq!(
        snapshot.commands.len(),
        3,
        "nudge plus the two relationship choices"
    );
    for command in &snapshot.commands {
        assert!(
            compass.detail.contains(&command.title),
            "the return compass must be generated from the same current command titles: {}",
            command.title
        );
    }

    Ok(())
}

#[test]
fn return_compass_surfaces_all_simultaneously_open_shaping_choices() -> Result<(), Box<dyn Error>> {
    let mut universe = PocketUniverse::new()?;
    universe.invoke_projection_command(SEED_MARS_COLONY_COMMAND)?;
    let since = universe.world().events().len();
    universe.advance_periods(3)?;

    let snapshot = universe.projection_snapshot_since(Some(since));
    let compass = snapshot
        .briefing
        .as_ref()
        .expect("returning Pocket Universe should expose a Briefing")
        .items
        .iter()
        .find(|item| item.title == "Your turn · Shape the world")
        .expect("relationship and intervention choices should be summarized together");

    assert_eq!(
        snapshot.commands.len(),
        5,
        "one nudge plus two relationship and two intervention choices should be open"
    );
    for command in &snapshot.commands {
        assert!(
            compass.detail.contains(&command.title),
            "every actually available action should appear in the return compass: {}",
            command.title
        );
    }

    Ok(())
}

#[test]
fn return_compass_explains_how_to_continue_a_living_legacy() -> Result<(), Box<dyn Error>> {
    let mut universe = PocketUniverse::new()?;
    universe.invoke_projection_command(SEED_MARS_COLONY_COMMAND)?;
    universe.advance_periods(2)?;
    universe.invoke_projection_command(SHARED_PROJECT_COMMAND)?;
    universe.advance_periods(1)?;
    universe.invoke_projection_command(BOLD_PATH_COMMAND)?;
    universe.advance_periods(3)?;
    universe.invoke_projection_command(OUTWARD_POSTURE_COMMAND)?;
    universe.advance_periods(3)?;

    let since = universe.world().events().len();
    universe.advance_periods(1)?;
    let snapshot = universe.projection_snapshot_since(Some(since));
    let compass = snapshot
        .briefing
        .as_ref()
        .expect("returning Pocket Universe should expose a Briefing")
        .items
        .iter()
        .find(|item| item.title == "Next · Living legacy")
        .expect("a mature World should explain why another cycle is meaningful");

    assert_eq!(
        snapshot.commands.len(),
        1,
        "a mature legacy has one continuation command"
    );
    let continuation = &snapshot.commands[0];
    assert!(compass.detail.contains(&continuation.title));
    assert!(
        compass.detail.contains(&continuation.detail),
        "when continuation is the only action, the compass should reuse its semantic explanation"
    );

    Ok(())
}
