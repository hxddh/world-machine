use crate::model::{MESSAGE_DELETED, PROTOTYPE_COPIED, VISIBLE, WARNING_MESSAGE_SENT};
use crate::*;
use world_core::Value;
use world_projection::SelectionId;

#[test]
fn initial_projection_exposes_evidence_without_leaking_hidden_truth() {
    let world = FutureArchaeologist::new().unwrap();
    let snapshot = world.projection_snapshot();

    assert_eq!(world.world().events().len(), 7);
    assert_eq!(world.world().world_time(), 50);
    assert_eq!(snapshot.collection.items.len(), 5);
    assert_eq!(snapshot.commands.len(), 1);
    assert_eq!(snapshot.commands[0].id, RECOVER_MESSAGE_COMMAND);
    assert!(snapshot
        .collection
        .items
        .iter()
        .all(|item| item.id != SelectionId::Entity(DELETED_MESSAGE)));

    let visible_timeline = snapshot
        .timeline
        .items
        .iter()
        .filter_map(|item| match item.id {
            SelectionId::Event(id) => Some(id),
            SelectionId::Entity(_) | SelectionId::Relation(_) => None,
        })
        .collect::<Vec<_>>();
    assert!(visible_timeline.contains(&PROTOTYPE_COPIED));
    assert!(!visible_timeline.contains(&WARNING_MESSAGE_SENT));
    assert!(!visible_timeline.contains(&MESSAGE_DELETED));
    assert!(!snapshot
        .inspectors
        .contains_key(&SelectionId::Entity(DELETED_MESSAGE)));
    assert!(!snapshot
        .inspectors
        .contains_key(&SelectionId::Event(WARNING_MESSAGE_SENT)));
    assert!(!snapshot
        .inspectors
        .contains_key(&SelectionId::Event(MESSAGE_DELETED)));
    assert!(!snapshot.why.contains_key(&WARNING_MESSAGE_SENT));
    assert!(!snapshot.why.contains_key(&MESSAGE_DELETED));
}

#[test]
fn recovery_reveals_the_deleted_artifact_but_still_truncates_hidden_causes() {
    let mut world = FutureArchaeologist::new().unwrap();

    let recovered = world
        .invoke_projection_command(RECOVER_MESSAGE_COMMAND)
        .unwrap();
    let snapshot = world.projection_snapshot();

    assert_eq!(recovered.0, 8);
    assert!(matches!(
        world
            .world()
            .state()
            .entity(DELETED_MESSAGE)
            .and_then(|entity| entity.component(VISIBLE)),
        Some(Value::Bool(true))
    ));
    assert!(snapshot.commands.is_empty());
    assert!(snapshot
        .collection
        .items
        .iter()
        .any(|item| item.id == SelectionId::Entity(DELETED_MESSAGE)));
    assert!(snapshot
        .timeline
        .items
        .iter()
        .any(|item| item.id == SelectionId::Event(MESSAGE_DELETED)));
    assert!(snapshot
        .timeline
        .items
        .iter()
        .any(|item| item.id == SelectionId::Event(recovered)));
    assert!(snapshot
        .timeline
        .items
        .iter()
        .all(|item| item.id != SelectionId::Event(WARNING_MESSAGE_SENT)));
    assert!(snapshot
        .inspectors
        .contains_key(&SelectionId::Entity(DELETED_MESSAGE)));
    assert!(snapshot
        .inspectors
        .contains_key(&SelectionId::Event(MESSAGE_DELETED)));
    assert!(!snapshot
        .inspectors
        .contains_key(&SelectionId::Event(WARNING_MESSAGE_SENT)));

    let deletion_why = snapshot.why.get(&MESSAGE_DELETED).unwrap();
    assert_eq!(deletion_why.nodes.len(), 1);
    assert_eq!(deletion_why.nodes[0].event, MESSAGE_DELETED);
    assert!(deletion_why.nodes[0].caused_by.is_empty());

    let recovery_why = snapshot.why.get(&recovered).unwrap();
    assert!(recovery_why
        .nodes
        .iter()
        .any(|node| node.event == MESSAGE_DELETED));
    assert!(recovery_why
        .nodes
        .iter()
        .all(|node| node.event != WARNING_MESSAGE_SENT));

    let recovery_event = world.world().event(recovered).unwrap();
    assert_eq!(recovery_event.caused_by, vec![MESSAGE_DELETED]);
}

#[test]
fn fixed_truth_and_recovery_are_replayable() {
    let mut world = FutureArchaeologist::new().unwrap();
    world
        .invoke_projection_command(RECOVER_MESSAGE_COMMAND)
        .unwrap();

    let replayed = world.world().replay().unwrap();

    assert_eq!(replayed.state(), world.world().state());
    assert_eq!(replayed.events(), world.world().events());
}