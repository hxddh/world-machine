use crate::actions::text_component;
use crate::model::{
    CONDITION, JONAS_HARBOR_JOB, JONAS_LEO_TRUST, MAINLAND_MARKET, MARA_EMMA_FRIEND, ORDER_STATUS,
    TEMP_BAKERY_JOB,
};
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
fn save_resume_restores_pending_world_and_briefs_only_new_events() {
    let mut simulation = TinySociety::new().unwrap();
    simulation.advance_checkpoint(5).unwrap();
    let cursor = simulation.visit_cursor();
    let json = simulation.archive_json().unwrap();

    let mut resumed = TinySociety::resume_json(&json).unwrap();

    assert_eq!(resumed.world().world_time(), 5);
    assert_eq!(resumed.world().state(), simulation.world().state());
    assert_eq!(resumed.world().events(), simulation.world().events());
    assert_eq!(
        resumed
            .world()
            .scheduler()
            .pending()
            .map(|item| (item.world_time, item.request.action.as_str()))
            .collect::<Vec<_>>(),
        simulation
            .world()
            .scheduler()
            .pending()
            .map(|item| (item.world_time, item.request.action.as_str()))
            .collect::<Vec<_>>()
    );

    resumed.advance_checkpoint(10).unwrap();
    let snapshot = resumed.projection_snapshot_since(cursor);
    let briefing = snapshot.briefing.as_ref().unwrap();

    assert_eq!(briefing.title, "While you were away");
    assert!(briefing
        .items
        .iter()
        .any(|item| item.title == "A storm reached the harbor"));
    assert!(briefing
        .items
        .iter()
        .any(|item| item.title == "Jonas asked Leo for a loan"));
    assert!(briefing
        .items
        .iter()
        .all(|item| item.detail.contains("World time 10")));
}

#[test]
fn visit_cursor_reports_when_nothing_changed() {
    let simulation = TinySociety::new().unwrap();
    let snapshot = simulation.projection_snapshot_since(simulation.visit_cursor());
    let briefing = snapshot.briefing.as_ref().unwrap();

    assert_eq!(briefing.title, "While you were away");
    assert_eq!(briefing.items.len(), 1);
    assert_eq!(briefing.items[0].title, "No new events");
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
fn forked_world_can_be_saved_and_reopened_with_its_choice_intact() {
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
    let json = branch.archive_json().unwrap();

    let resumed = TinySociety::resume_json(&json).unwrap();
    let snapshot = resumed.projection_snapshot();

    assert_eq!(resumed.world().world_time(), 20);
    assert!(resumed
        .world()
        .events()
        .iter()
        .all(|event| event.kind != "worker_dismissed"));
    assert_eq!(snapshot.commands.len(), 1);
    assert_eq!(snapshot.commands[0].id, RETAIN_WORKER_COMMAND);
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
fn social_support_unlocks_a_durable_return_to_fishing() {
    let mut simulation = TinySociety::new().unwrap();
    simulation.run_story().unwrap();
    let mut branch = simulation.branch();

    assert!(branch
        .projection_snapshot()
        .commands
        .iter()
        .all(|command| command.id != REPAIR_BOAT_COMMAND));

    branch.advance_days(10).unwrap();
    let support = branch
        .world()
        .events()
        .iter()
        .rev()
        .find(|event| event.kind == "support_received")
        .expect("dismissal branch activates Leo's support")
        .id;
    let snapshot = branch.projection_snapshot();
    assert!(snapshot
        .commands
        .iter()
        .any(|command| command.id == REPAIR_BOAT_COMMAND));

    let leo_before = integer_component(branch.world().state(), LEO, CASH).unwrap();
    let evan_before = integer_component(branch.world().state(), EVAN, CASH).unwrap();
    let repaired = branch
        .invoke_projection_command(REPAIR_BOAT_COMMAND)
        .unwrap();

    assert_eq!(repaired.len(), 1);
    let repair = branch.world().event(repaired[0]).unwrap();
    assert_eq!(repair.kind, "boat_repaired");
    assert_eq!(repair.caused_by, vec![support]);
    assert_eq!(repair.actor, Some(LEO));
    assert_eq!(
        integer_component(branch.world().state(), LEO, CASH).unwrap(),
        leo_before - crate::social::SEA_FINCH_REPAIR_COST
    );
    assert_eq!(
        integer_component(branch.world().state(), EVAN, CASH).unwrap(),
        evan_before + crate::social::SEA_FINCH_REPAIR_COST
    );
    assert_eq!(
        text_component(branch.world().state(), JONAS_BOAT, CONDITION).unwrap(),
        "sound"
    );
    assert_eq!(
        text_component(branch.world().state(), JONAS, JOB).unwrap(),
        "fisher"
    );
    assert!(branch.world().state().relation(JONAS_HARBOR_JOB).is_some());

    let snapshot = branch.projection_snapshot();
    assert!(snapshot
        .commands
        .iter()
        .all(|command| command.id != REPAIR_BOAT_COMMAND));
    assert!(snapshot
        .canvas
        .items
        .iter()
        .find(|item| item.id == SelectionId::Entity(JONAS_BOAT))
        .is_some_and(|item| item.detail == "asset · sound"));
    let why = snapshot.why(repair.id).unwrap();
    assert!(why.nodes.iter().any(|node| node.event == support));

    let json = branch.archive_json().unwrap();
    let resumed = TinySociety::resume_json(&json).unwrap();
    assert_eq!(
        text_component(resumed.world().state(), JONAS_BOAT, CONDITION).unwrap(),
        "sound"
    );
    assert_eq!(
        text_component(resumed.world().state(), JONAS, JOB).unwrap(),
        "fisher"
    );
    assert!(resumed.world().state().relation(JONAS_HARBOR_JOB).is_some());

    let cursor = branch.visit_cursor();
    let jonas_before = integer_component(branch.world().state(), JONAS, CASH).unwrap();
    let harbor_before = integer_component(branch.world().state(), HARBOR, CASH).unwrap();
    let market_before = integer_component(branch.world().state(), MAINLAND_MARKET, CASH).unwrap();
    branch.advance_days(1).unwrap();
    let new_events = &branch.world().events()[cursor.event_count..];
    let shift = new_events
        .iter()
        .find(|event| {
            event.kind == "work_shift_completed"
                && event.actor == Some(JONAS)
                && event.targets.contains(&HARBOR)
        })
        .expect("repaired Jonas completes a Harbor shift");
    let catch = new_events
        .iter()
        .find(|event| event.kind == "catch_landed")
        .expect("fishing shift lands a catch");
    let sale = new_events
        .iter()
        .find(|event| event.kind == "fish_sold")
        .expect("landed catch sells to the mainland");
    assert_eq!(catch.caused_by, vec![shift.id]);
    assert_eq!(sale.caused_by, vec![catch.id]);
    assert_eq!(
        integer_component(branch.world().state(), JONAS, CASH).unwrap(),
        jonas_before + 25 - crate::social::JONAS_DAILY_LIVING_COST
    );
    assert_eq!(
        integer_component(branch.world().state(), HARBOR, CASH).unwrap(),
        harbor_before + crate::fishing::DAILY_CATCH_VALUE - 25
    );
    assert_eq!(
        integer_component(branch.world().state(), MAINLAND_MARKET, CASH).unwrap(),
        market_before - crate::fishing::DAILY_CATCH_VALUE
    );
}

#[test]
fn routine_fishing_pays_jonas_and_earns_export_revenue() {
    let mut simulation = TinySociety::new().unwrap();
    let jonas_before = integer_component(simulation.world().state(), JONAS, CASH).unwrap();
    let harbor_before = integer_component(simulation.world().state(), HARBOR, CASH).unwrap();
    let market_before = integer_component(simulation.world().state(), MAINLAND_MARKET, CASH).unwrap();

    simulation.advance_checkpoint(5).unwrap();

    let shift = simulation
        .world()
        .events()
        .iter()
        .find(|event| {
            event.kind == "work_shift_completed"
                && event.actor == Some(JONAS)
                && event.targets.contains(&HARBOR)
        })
        .expect("Jonas completes the scheduled Harbor shift");
    let catch = simulation
        .world()
        .events()
        .iter()
        .find(|event| event.kind == "catch_landed")
        .expect("Harbor shift lands a catch");
    let sale = simulation
        .world()
        .events()
        .iter()
        .find(|event| event.kind == "fish_sold")
        .expect("catch sells to mainland demand");

    assert_eq!(catch.caused_by, vec![shift.id]);
    assert_eq!(sale.caused_by, vec![catch.id]);
    assert_eq!(
        sale.payload.get("revenue"),
        Some(&Value::Integer(crate::fishing::DAILY_CATCH_VALUE))
    );
    assert_eq!(
        integer_component(simulation.world().state(), JONAS, CASH).unwrap(),
        jonas_before + 25
    );
    assert_eq!(
        integer_component(simulation.world().state(), HARBOR, CASH).unwrap(),
        harbor_before + crate::fishing::DAILY_CATCH_VALUE - 25
    );
    assert_eq!(
        integer_component(simulation.world().state(), MAINLAND_MARKET, CASH).unwrap(),
        market_before - crate::fishing::DAILY_CATCH_VALUE
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

    let snapshot = simulation.projection_snapshot();
    assert!(snapshot
        .briefing
        .as_ref()
        .is_some_and(|briefing| briefing
            .items
            .iter()
            .any(|item| item.title == "Jonas's catch reached the mainland")));
    assert!(snapshot
        .canvas
        .items
        .iter()
        .find(|item| item.id == SelectionId::Entity(HARBOR))
        .is_some_and(|item| item.detail == "Place · cash 810"));
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
