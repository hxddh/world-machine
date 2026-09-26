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
        snapshot
            .commands
            .iter()
            .filter(|command| !command.id.starts_with("pocket-universe.story."))
            .count(),
        3,
        "nudge plus the two relationship choices"
    );
    assert!(
        !compass.detail.contains("Choice signal"),
        "the compass says why the choice is open; what each answer does is the choice's own line"
    );
    assert!(
        compass.detail.contains("finding their footing")
            || compass.detail.contains("rely on each other")
            || compass.detail.contains("not easy with each other"),
        "relationship context says how they stand, in words: {}",
        compass.detail
    );
    assert!(compass.detail.contains("Where it goes is still open."));
    let shared = snapshot
        .commands
        .iter()
        .find(|command| command.id == SHARED_PROJECT_COMMAND)
        .expect("shared project should be available");
    assert!(
        !shared.detail.contains("Choice signal"),
        "{}",
        shared.detail
    );
    let rivalry = snapshot
        .commands
        .iter()
        .find(|command| command.id == RIVALRY_COMMAND)
        .expect("rivalry should be available");
    assert!(
        !rivalry.detail.contains("Choice signal"),
        "{}",
        rivalry.detail
    );
    let nudge = &snapshot.commands[0];
    assert!(!nudge.detail.contains("Choice signal"), "{}", nudge.detail);
    for command in &snapshot.commands {
        assert!(
            !compass.detail.contains(&command.detail),
            "the compass should not repeat what the choice itself says: {}",
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
        snapshot
            .commands
            .iter()
            .filter(|command| !command.id.starts_with("pocket-universe.story."))
            .count(),
        5,
        "one nudge plus two relationship and two intervention choices should be open"
    );
    assert!(
        !compass.detail.contains("Choice signal"),
        "the compass says why the choice is open; what each answer does is the choice's own line"
    );
    assert!(
        compass
            .detail
            .contains("The World has grown enough for a bigger choice."),
        "the compass should explain why the larger intervention is open now"
    );
    assert!(
        compass.detail.len() > "The World has grown enough for a bigger choice.".len(),
        "the compass also says what the World is doing now: {}",
        compass.detail
    );
    let bold = snapshot
        .commands
        .iter()
        .find(|command| command.id == BOLD_PATH_COMMAND)
        .expect("bold intervention should be available");
    assert!(!bold.detail.contains("Choice signal"), "{}", bold.detail);
    let careful = snapshot
        .commands
        .iter()
        .find(|command| command.id == CAREFUL_PATH_COMMAND)
        .expect("careful intervention should be available");
    assert!(
        !careful.detail.contains("Choice signal"),
        "{}",
        careful.detail
    );
    for command in &snapshot.commands {
        assert!(
            !compass.detail.contains(&command.detail),
            "the compass should not repeat what the choice itself says: {}",
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

    assert!(
        !compass.detail.contains("Choice signal"),
        "the compass says why the choice is open; what each answer does is the choice's own line"
    );
    assert!(
        compass
            .detail
            .contains("The first chapter has settled into a partnership"),
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
    assert!(
        !outward.detail.contains("Choice signal"),
        "{}",
        outward.detail
    );
    let rooted = snapshot
        .commands
        .iter()
        .find(|command| command.id == ROOTED_POSTURE_COMMAND)
        .expect("rooted posture should be available");
    assert!(
        !rooted.detail.contains("Choice signal"),
        "{}",
        rooted.detail
    );
    let archive = universe.archive()?;
    let reopened = PocketUniverse::resume_archive(&archive)?;
    assert_eq!(
        reopened.projection_snapshot_since(Some(since)),
        snapshot,
        "choice signals should be derived entirely from durable state and current rules"
    );
    for command in &snapshot.commands {
        assert!(
            !compass.detail.contains(&command.detail),
            "the compass should not repeat what the choice itself says: {}",
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
        snapshot
            .commands
            .iter()
            .filter(|command| !command.id.starts_with("pocket-universe.story."))
            .count(),
        1,
        "a mature legacy has one continuation command"
    );
    let continuation = &snapshot.commands[0];
    assert!(
        !compass.detail.contains("Choice signal"),
        "the compass says why the choice is open; what each answer does is the choice's own line"
    );
    assert!(compass
        .detail
        .contains("Ridge Network keeps growing stronger"));
    assert!(compass.detail.contains("shared upkeep"));
    assert!(
        !compass.detail.contains(&continuation.detail),
        "the continuation explains itself; the compass says why it matters now"
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
