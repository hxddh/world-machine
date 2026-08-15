use pocket_universe::{
    PocketUniverse, BOLD_PATH_COMMAND, CAREFUL_PATH_COMMAND, OUTWARD_POSTURE_COMMAND,
    RIVALRY_COMMAND, ROOTED_POSTURE_COMMAND, SEED_MARS_COLONY_COMMAND, SHARED_PROJECT_COMMAND,
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
    assert!(compass.detail.starts_with("Why now: "));
    assert!(
        compass.detail.contains("trust ") && compass.detail.contains("tension "),
        "relationship context should expose the current durable relationship pressure"
    );
    assert!(compass
        .detail
        .contains("Its durable direction is still open."));
    let shared = snapshot
        .commands
        .iter()
        .find(|command| command.id == SHARED_PROJECT_COMMAND)
        .expect("shared project should be available");
    let rivalry = snapshot
        .commands
        .iter()
        .find(|command| command.id == RIVALRY_COMMAND)
        .expect("rivalry should be available");
    let nudge = &snapshot.commands[0];
    assert!(shared.detail.contains(
        "Choice signal: trust 2 → 4 · tension 0 → 0; each later relationship shift also gains +1 trust and -1 tension."
    ));
    assert!(rivalry.detail.contains(
        "Choice signal: trust 2 → 2 · tension 0 → 2; each later relationship shift also gains +1 tension."
    ));
    assert!(nudge.detail.contains(
        "Choice signal: one full cycle resolves under current rules: world growth, both actor turns, relationship update, then period consequences."
    ));
    assert!(compass.detail.contains("Choice signals:"));
    assert!(compass.detail.contains(
        "trust 2 → 4 · tension 0 → 0; each later relationship shift also gains +1 trust and -1 tension"
    ));
    assert!(compass.detail.contains(
        "trust 2 → 2 · tension 0 → 2; each later relationship shift also gains +1 tension"
    ));
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
    assert!(compass.detail.starts_with("Why now: "));
    assert!(
        compass
            .detail
            .contains("Generation 3 has reached a larger intervention point."),
        "the compass should explain why the larger intervention is open now"
    );
    assert!(compass.detail.contains("Current thread:"));
    let bold = snapshot
        .commands
        .iter()
        .find(|command| command.id == BOLD_PATH_COMMAND)
        .expect("bold intervention should be available");
    let careful = snapshot
        .commands
        .iter()
        .find(|command| command.id == CAREFUL_PATH_COMMAND)
        .expect("careful intervention should be available");
    assert!(bold.detail.contains(
        "Choice signal: locks the first intervention to Signal expedition; Kestrel's durable status becomes signal expedition."
    ));
    assert!(careful.detail.contains(
        "Choice signal: locks the first intervention to Fortified habitat; Ares Habitat's durable status becomes storm sealed."
    ));
    assert!(compass.detail.contains(
        "locks the first intervention to Signal expedition; Kestrel's durable status becomes signal expedition"
    ));
    assert!(compass.detail.contains(
        "locks the first intervention to Fortified habitat; Ares Habitat's durable status becomes storm sealed"
    ));
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
fn return_compass_explains_why_world_direction_is_open() -> Result<(), Box<dyn Error>> {
    let mut universe = PocketUniverse::new()?;
    universe.invoke_projection_command(SEED_MARS_COLONY_COMMAND)?;
    universe.advance_periods(2)?;
    universe.invoke_projection_command(SHARED_PROJECT_COMMAND)?;
    universe.advance_periods(1)?;
    universe.invoke_projection_command(BOLD_PATH_COMMAND)?;
    let since = universe.world().events().len();
    universe.advance_periods(3)?;

    let snapshot = universe.projection_snapshot_since(Some(since));
    let compass = snapshot
        .briefing
        .as_ref()
        .expect("returning Pocket Universe should expose a Briefing")
        .items
        .iter()
        .find(|item| item.title == "Your turn · World direction")
        .expect("the return compass should explain why the second-arc posture choice is open");

    assert!(compass.detail.starts_with("Why now: "));
    assert!(
        compass
            .detail
            .contains("The first arc has settled as partnership"),
        "posture context should reuse the durable social arc"
    );
    assert!(
        compass.detail.contains("Signal expedition"),
        "posture context should reuse the durable intervention"
    );
    let outward = snapshot
        .commands
        .iter()
        .find(|command| command.id == OUTWARD_POSTURE_COMMAND)
        .expect("outward posture should be available");
    let rooted = snapshot
        .commands
        .iter()
        .find(|command| command.id == ROOTED_POSTURE_COMMAND)
        .expect("rooted posture should be available");
    assert!(outward.detail.contains(
        "Choice signal: sets durable World direction to Outward; later growth and legacy formation read the outward posture."
    ));
    assert!(rooted.detail.contains(
        "Choice signal: sets durable World direction to Rooted; later growth and legacy formation read the rooted posture."
    ));
    assert!(compass.detail.contains(
        "sets durable World direction to Outward; later growth and legacy formation read the outward posture"
    ));
    assert!(compass.detail.contains(
        "sets durable World direction to Rooted; later growth and legacy formation read the rooted posture"
    ));
    let archive = universe.archive()?;
    let reopened = PocketUniverse::resume_archive(&archive)?;
    assert_eq!(
        reopened.projection_snapshot_since(Some(since)),
        snapshot,
        "choice signals should be derived entirely from durable state and current rules"
    );
    for command in &snapshot.commands {
        assert!(
            compass.detail.contains(&command.title),
            "the contextual compass must still name every actually available command: {}",
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
    assert!(compass.detail.starts_with("Why now: "));
    assert!(compass.detail.contains("World legacy · Ridge Network"));
    assert!(compass.detail.contains("1 later cycle"));
    assert!(compass.detail.contains("adaptive cycle 1"));
    assert!(compass.detail.contains(&continuation.title));
    assert!(
        compass.detail.contains(&continuation.detail),
        "when continuation is the only action, the compass should reuse its semantic explanation"
    );

    let archive = universe.archive()?;
    let reopened = PocketUniverse::resume_archive(&archive)?;
    assert_eq!(
        reopened.projection_snapshot_since(Some(since)),
        snapshot,
        "return context should be derived entirely from durable state and event history"
    );

    Ok(())
}
