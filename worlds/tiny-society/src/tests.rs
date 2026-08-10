use crate::actions::text_component;
use crate::model::{JONAS_LEO_TRUST, MARA_EMMA_FRIEND, ORDER_STATUS, TEMP_BAKERY_JOB};
use crate::*;
use society_basic::{integer_component, CASH, JOB};
use world_agent::MockAgentRuntime;
use world_projection::SelectionId;

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

#[test]
fn mara_decision_runs_through_provider_neutral_agent_runtime() {
    let mut simulation = TinySociety::new().unwrap();
    let mut runtime = MockAgentRuntime::scripted(["assign_temporary_work"]);

    simulation.run_story_with_runtime(&mut runtime).unwrap();

    assert_eq!(runtime.call_count(), 1);
    let loan = simulation
        .world()
        .events()
        .iter()
        .find(|event| event.kind == "loan_requested")
        .unwrap();
    let decision = simulation
        .world()
        .events()
        .iter()
        .find(|event| event.kind == "agent_decision_recorded")
        .unwrap();
    let assignment = simulation
        .world()
        .events()
        .iter()
        .find(|event| event.kind == "temporary_work_assigned")
        .unwrap();

    assert!(decision.caused_by.contains(&loan.id));
    assert!(assignment.caused_by.contains(&loan.id));
    assert!(assignment.caused_by.contains(&decision.id));
}

#[test]
fn projection_snapshot_is_self_contained_and_selectable() {
    let mut simulation = TinySociety::new().unwrap();
    simulation.run_story().unwrap();

    let snapshot = simulation.projection_snapshot();

    assert_eq!(snapshot.collection.items.len(), 8);
    assert_eq!(snapshot.world_time, simulation.world().world_time());
    assert!(snapshot.canvas.items.len() >= 12);
    assert_eq!(snapshot.timeline.items.len(), simulation.world().events().len());
    assert!(snapshot
        .inspector(SelectionId::Entity(MARA))
        .is_some_and(|inspector| inspector.title == "Mara"));
    assert!(snapshot.timeline.items.iter().any(|item| {
        snapshot
            .inspector(item.id)
            .is_some_and(|inspector| inspector.selection == item.id)
    }));
}
