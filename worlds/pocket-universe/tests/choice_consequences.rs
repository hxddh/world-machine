//! A choice's consequences are shown before it is made, so they must be
//! what actually happens when it is made.

use pocket_universe::{
    PocketUniverse, BOLD_PATH_COMMAND, CAREFUL_PATH_COMMAND, HOLD_PRESSURE_COMMAND,
    ROOTED_POSTURE_COMMAND, SEED_MARS_COLONY_COMMAND, SHARED_PROJECT_COMMAND,
};
use std::error::Error;
use world_core::Value;
use world_projection::{EffectChange, SelectionId, Tone};

fn status_of(universe: &PocketUniverse, selection: SelectionId) -> Option<String> {
    let SelectionId::Entity(id) = selection else {
        return None;
    };
    match universe.world().state().entity(id)?.component("status") {
        Some(Value::Text(value)) => Some(value.clone()),
        _ => None,
    }
}

#[test]
fn an_intervention_shows_the_state_it_will_set() -> Result<(), Box<dyn Error>> {
    for command in [BOLD_PATH_COMMAND, CAREFUL_PATH_COMMAND] {
        let mut universe = PocketUniverse::new()?;
        universe.invoke_projection_command(SEED_MARS_COLONY_COMMAND)?;
        universe.advance_periods(3)?;

        let shown = universe
            .projection_snapshot()
            .command(command)
            .expect("the intervention is open at generation 3")
            .effects
            .clone();
        assert_eq!(shown.len(), 1, "{command}: {shown:?}");
        let target = shown[0]
            .target
            .expect("an intervention changes something on stage");
        let EffectChange::To(promised) = &shown[0].change else {
            panic!("{command}: an intervention sets a value: {shown:?}");
        };

        universe.invoke_projection_command(command)?;
        assert_eq!(
            status_of(&universe, target).as_deref(),
            Some(promised.as_str())
        );
    }
    Ok(())
}

#[test]
fn a_relationship_choice_shows_which_way_it_moves_them() -> Result<(), Box<dyn Error>> {
    let mut universe = PocketUniverse::new()?;
    universe.invoke_projection_command(SEED_MARS_COLONY_COMMAND)?;
    universe.advance_periods(2)?;
    let effects = universe
        .projection_snapshot()
        .command(SHARED_PROJECT_COMMAND)
        .expect("the relationship choice is open")
        .effects
        .clone();
    assert!(effects
        .iter()
        .any(|effect| effect.label == "Trust" && effect.change == EffectChange::Up));
    assert!(effects
        .iter()
        .any(|effect| effect.label == "Tension" && effect.change == EffectChange::Down));
    Ok(())
}

#[test]
fn rising_trouble_reads_as_a_warning_and_answering_it_as_good_news() -> Result<(), Box<dyn Error>> {
    let mut universe = PocketUniverse::new()?;
    universe.invoke_projection_command(SEED_MARS_COLONY_COMMAND)?;
    universe.advance_periods(3)?;
    universe.invoke_projection_command(BOLD_PATH_COMMAND)?;
    universe.invoke_projection_command(SHARED_PROJECT_COMMAND)?;
    universe.advance_periods(3)?;
    universe.invoke_projection_command(ROOTED_POSTURE_COMMAND)?;
    let mut periods = 0;
    while universe
        .projection_snapshot()
        .command(HOLD_PRESSURE_COMMAND)
        .is_none()
    {
        universe.advance_periods(1)?;
        periods += 1;
        assert!(periods < 30, "the pressure never rose");
    }

    let rising = universe.projection_snapshot();
    let briefing = rising.briefing.as_ref().expect("a briefing");
    assert!(
        briefing.items.iter().any(|item| item.tone == Tone::Warning),
        "{:#?}",
        briefing.items
    );
    let hold = rising.command(HOLD_PRESSURE_COMMAND).expect("hold is open");
    assert!(hold
        .effects
        .iter()
        .any(|effect| effect.label == "Fits the World's direction" && effect.tone == Tone::Good));

    universe.invoke_projection_command(HOLD_PRESSURE_COMMAND)?;
    let held = universe.projection_snapshot();
    assert!(held
        .briefing
        .as_ref()
        .expect("a briefing")
        .items
        .iter()
        .any(|item| item.tone == Tone::Good));
    Ok(())
}
