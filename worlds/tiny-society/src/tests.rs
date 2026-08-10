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
    assert!(simulation.projection_snapshot().commands.is_empty());
}

#[test]
fn branch_before_dismissal_preserves_the_alternative_state() {
    let mut simulation = TinySociety::new().unwrap();
    simulation.run_story().unwrap();

    let dismissal = simulation
        .world()
        .events()
        .iter()
        .find(|event| event.kind == "worker_dismissed")
        .unwrap()
        .id;
    let mut branch = simulation.branch();
    branch.fork_before_event(dismissal).unwrap();

    assert_eq!(
        text_component(branch.world().state(), JONAS, JOB).unwrap(),
        "bakery_temp"
    );
    assert!(branch.world().state().relation(TEMP_BAKERY_JOB).is_some());
    assert_eq!(
        text_component(branch.world().state(), WEDDING_ORDER, ORDER_STATUS).unwrap(),
        "lost"
    );
    assert!(branch
        .world()
        .events()
        .iter()
        .all(|event| event.kind != "worker_dismissed"));

    let snapshot = branch.projection_snapshot();
    assert!(snapshot
        .timeline
        .items
        .iter()
        .all(|item| item.id != SelectionId::Event(dismissal)));
    assert_eq!(snapshot.commands.len(), 1);
    assert_eq!(snapshot.commands[0].id, RETAIN_WORKER_COMMAND);
    assert_eq!(snapshot.commands[0].title, "Give Jonas another chance");
}

#[test]
fn forked_branch_can_diverge_through_projection_command_and_keep_running() {
    let mut simulation = TinySociety::new().unwrap();
    simulation.run_story().unwrap();

    let dismissal = simulation
        .world()
        .events()
        .iter()
        .find(|event| event.kind == "worker_dismissed")
        .unwrap()
        .id;
    let mut branch = simulation.branch();
    branch.fork_before_event(dismissal).unwrap();

    let order_loss = branch
        .world()
        .events()
        .iter()
        .find(|event| event.kind == "order_lost")
        .unwrap()
        .id;
    let jonas_cash_before = integer_component(branch.world().state(), JONAS, CASH).unwrap();

    branch
        .invoke_projection_command(RETAIN_WORKER_COMMAND)
        .unwrap();

    let retained = branch
        .world()
        .events()
        .iter()
        .find(|event| event.kind == "worker_retained")
        .unwrap();
    let future_shift = branch
        .world()
        .events()
        .iter()
        .rev()
        .find(|event| event.kind == "work_shift_completed" && event.actor == Some(JONAS))
        .unwrap();

    assert!(retained.caused_by.contains(&order_loss));
    assert!(future_shift.caused_by.contains(&retained.id));
    assert_eq!(
        text_component(branch.world().state(), JONAS, JOB).unwrap(),
        "bakery_temp"
    );
    assert_eq!(
        integer_component(branch.world().state(), JONAS, CASH).unwrap(),
        jonas_cash_before + 18
    );
    assert_eq!(branch.world().world_time(), 25);
    assert!(branch
        .world()
        .events()
        .iter()
        .all(|event| event.kind != "worker_dismissed"));

    let snapshot = branch.projection_snapshot();
    assert!(snapshot.commands.is_empty());
    let why = snapshot.why(future_shift.id).unwrap();
    assert!(why.nodes.iter().any(|node| node.event == retained.id));
    assert!(why.nodes.iter().any(|node| node.event == order_loss));
    assert!(snapshot
        .briefing
        .as_ref()
        .is_some_and(|briefing| briefing.items.iter().any(|item| {
            item.title == "Mara gave Jonas another chance"
                || item.title == "Jonas completed another bakery shift"
        })));
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
fn projection_snapshot_is_self_contained_selectable_and_causal() {
    let mut simulation = TinySociety::new().unwrap();
    simulation.run_story().unwrap();

    let snapshot = simulation.projection_snapshot();
    let dismissal = simulation
        .world()
        .events()
        .iter()
        .find(|event| event.kind == "worker_dismissed")
        .unwrap();

    assert_eq!(snapshot.collection.items.len(), 8);
    assert_eq!(snapshot.world_time, simulation.world().world_time());
    assert!(snapshot.canvas.items.len() >= 12);
    assert_eq!(
        snapshot.timeline.items.len(),
        simulation.world().events().len()
    );
    assert!(snapshot
        .briefing
        .as_ref()
        .is_some_and(|briefing| briefing.eyebrow == "Society Today" && !briefing.items.is_empty()));
    assert!(snapshot
        .inspector(SelectionId::Entity(MARA))
        .is_some_and(|inspector| inspector.title == "Mara"));

    let why = snapshot.why(dismissal.id).unwrap();
    assert_eq!(why.nodes.first().unwrap().event, dismissal.id);
    assert!(why.nodes.iter().any(|node| node.title == "Storm Started"));
    assert!(why.nodes.iter().any(|node| node.title == "Order Lost"));
}
