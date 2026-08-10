use crate::actions::text_component;
use crate::model::{JONAS_LEO_TRUST, MARA_EMMA_FRIEND, ORDER_STATUS, TEMP_BAKERY_JOB};
use crate::*;
use society_basic::{integer_component, CASH, JOB};

#[test]
fn full_story_is_causal_and_replayable() {
    let mut simulation = TinySociety::new().unwrap();
    simulation.run_story().unwrap();

    let story = simulation.causal_story();
    assert_eq!(story.len(), 8);
    for pair in story.windows(2) {
        assert!(pair[1].caused_by.contains(&pair[0].id));
    }

    let replayed = simulation.world().replay().unwrap();
    assert_eq!(replayed.state(), simulation.world().state());
    assert_eq!(replayed.events(), simulation.world().events());
    assert_eq!(
        text_component(replayed.state(), JONAS, JOB).unwrap(),
        "unemployed"
    );
    assert_eq!(
        text_component(replayed.state(), WEDDING_ORDER, ORDER_STATUS).unwrap(),
        "lost"
    );
}

#[test]
fn fork_before_dismissal_preserves_the_alternative_state() {
    let mut simulation = TinySociety::new().unwrap();
    simulation.run_story().unwrap();

    let dismissal_position = simulation
        .world()
        .events()
        .iter()
        .position(|event| event.kind == "worker_dismissed")
        .unwrap();
    let fork = simulation.world().fork_after(dismissal_position).unwrap();

    assert_eq!(
        text_component(fork.state(), JONAS, JOB).unwrap(),
        "bakery_temp"
    );
    assert!(fork.state().relation(TEMP_BAKERY_JOB).is_some());
    assert_eq!(
        text_component(fork.state(), WEDDING_ORDER, ORDER_STATUS).unwrap(),
        "lost"
    );
    assert!(fork
        .events()
        .iter()
        .all(|event| event.kind != "worker_dismissed"));
}

#[test]
fn routine_work_moves_cash_and_relationships_exist() {
    let mut simulation = TinySociety::new().unwrap();
    let jonas_before = integer_component(simulation.world().state(), JONAS, CASH).unwrap();
    let harbor_before = integer_component(simulation.world().state(), HARBOR, CASH).unwrap();

    simulation.advance_checkpoint(5).unwrap();

    assert_eq!(
        integer_component(simulation.world().state(), JONAS, CASH).unwrap(),
        jonas_before + 25
    );
    assert_eq!(
        integer_component(simulation.world().state(), HARBOR, CASH).unwrap(),
        harbor_before - 25
    );
    assert!(simulation
        .world()
        .state()
        .relation(MARA_EMMA_FRIEND)
        .is_some());
    assert!(simulation
        .world()
        .state()
        .relation(JONAS_LEO_TRUST)
        .is_some());
}
