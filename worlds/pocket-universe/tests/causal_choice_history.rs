use pocket_universe::{
    PocketUniverse, BOLD_PATH_COMMAND, NUDGE_COMMAND, OUTWARD_POSTURE_COMMAND,
    SEED_MARS_COLONY_COMMAND, SHARED_PROJECT_COMMAND,
};
use std::error::Error;
use world_core::EventId;

fn latest_event(universe: &PocketUniverse, kind: &str) -> world_core::Event {
    universe
        .world()
        .events()
        .iter()
        .rev()
        .find(|event| event.kind == kind)
        .unwrap_or_else(|| panic!("missing event kind {kind}"))
        .clone()
}

fn why_contains(
    snapshot: &world_projection::ProjectionSnapshot,
    effect: EventId,
    cause: EventId,
) -> bool {
    snapshot
        .why(effect)
        .expect("effect should have a generic Why projection")
        .nodes
        .iter()
        .any(|node| node.event == cause)
}

#[test]
fn later_relationship_shift_directly_names_the_relationship_choice_as_a_cause(
) -> Result<(), Box<dyn Error>> {
    let mut universe = PocketUniverse::new()?;
    universe.invoke_projection_command(SEED_MARS_COLONY_COMMAND)?;
    universe.advance_periods(2)?;
    let choice = universe.invoke_projection_command(SHARED_PROJECT_COMMAND)?;

    universe.invoke_projection_command(NUDGE_COMMAND)?;
    let shifted = latest_event(&universe, "relationship_shifted");

    assert!(shifted.caused_by.contains(&choice));
    let snapshot = universe.projection_snapshot();
    assert!(why_contains(&snapshot, shifted.id, choice));

    let archive = universe.archive()?;
    let reopened = PocketUniverse::resume_archive(&archive)?;
    assert_eq!(
        reopened.projection_snapshot().why(shifted.id),
        snapshot.why(shifted.id),
        "archive/reopen must preserve the exact causal explanation"
    );
    Ok(())
}

#[test]
fn later_growth_directly_names_every_durable_world_input_it_reads() -> Result<(), Box<dyn Error>> {
    let mut universe = PocketUniverse::new()?;
    universe.invoke_projection_command(SEED_MARS_COLONY_COMMAND)?;
    universe.advance_periods(2)?;
    universe.invoke_projection_command(SHARED_PROJECT_COMMAND)?;
    universe.advance_periods(1)?;
    let intervention = universe.invoke_projection_command(BOLD_PATH_COMMAND)?;
    universe.advance_periods(3)?;
    let social_arc = latest_event(&universe, "partnership_formed").id;
    let posture = universe.invoke_projection_command(OUTWARD_POSTURE_COMMAND)?;

    universe.invoke_projection_command(NUDGE_COMMAND)?;
    let growth = latest_event(&universe, "universe_grew");
    for cause in [intervention, social_arc, posture] {
        assert!(growth.caused_by.contains(&cause));
    }
    let snapshot = universe.projection_snapshot();
    for cause in [intervention, social_arc, posture] {
        assert!(why_contains(&snapshot, growth.id, cause));
    }
    Ok(())
}

#[test]
fn background_growth_uses_the_same_durable_causal_inputs_as_manual_continuation(
) -> Result<(), Box<dyn Error>> {
    let mut base = PocketUniverse::new()?;
    base.invoke_projection_command(SEED_MARS_COLONY_COMMAND)?;
    base.advance_periods(2)?;
    base.invoke_projection_command(SHARED_PROJECT_COMMAND)?;
    base.advance_periods(1)?;
    let intervention = base.invoke_projection_command(BOLD_PATH_COMMAND)?;
    base.advance_periods(3)?;
    let social_arc = latest_event(&base, "partnership_formed").id;
    let posture = base.invoke_projection_command(OUTWARD_POSTURE_COMMAND)?;
    let archive = base.archive()?;

    let mut manual = PocketUniverse::resume_archive(&archive)?;
    manual.invoke_projection_command(NUDGE_COMMAND)?;
    let manual_growth = latest_event(&manual, "universe_grew");

    let mut background = PocketUniverse::resume_archive(&archive)?;
    background.advance_periods(1)?;
    let background_growth = latest_event(&background, "universe_grew");

    for cause in [intervention, social_arc, posture] {
        assert!(manual_growth.caused_by.contains(&cause));
        assert!(background_growth.caused_by.contains(&cause));
    }
    assert_eq!(manual_growth.caused_by, background_growth.caused_by);
    Ok(())
}

#[test]
fn agent_decisions_trace_only_the_durable_choices_their_policy_reads() -> Result<(), Box<dyn Error>> {
    let mut universe = PocketUniverse::new()?;
    universe.invoke_projection_command(SEED_MARS_COLONY_COMMAND)?;
    universe.advance_periods(2)?;
    let relationship = universe.invoke_projection_command(SHARED_PROJECT_COMMAND)?;
    universe.advance_periods(1)?;
    universe.invoke_projection_command(BOLD_PATH_COMMAND)?;
    universe.advance_periods(3)?;
    let posture = universe.invoke_projection_command(OUTWARD_POSTURE_COMMAND)?;

    universe.invoke_projection_command(NUDGE_COMMAND)?;
    let decisions = universe
        .world()
        .events()
        .iter()
        .rev()
        .filter(|event| event.kind == "agent_decision_recorded")
        .take(2)
        .collect::<Vec<_>>();
    assert_eq!(decisions.len(), 2);
    let secondary = decisions[0];
    let primary = decisions[1];

    assert!(primary.caused_by.contains(&posture));
    assert!(secondary.caused_by.contains(&posture));
    assert!(secondary.caused_by.contains(&relationship));
    assert!(!primary.caused_by.contains(&relationship));
    Ok(())
}
