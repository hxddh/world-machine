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
    assert!(snapshot
        .collection
        .items
        .iter()
        .any(|item| item.id == SelectionId::Entity(DELETED_MESSAGE)));
    assert!(snapshot
        .timeline
        .items
        .iter()
        .any(|item| item.id == SelectionId::Event(recovered)));
    assert!(!snapshot
        .timeline
        .items
        .iter()
        .any(|item| item.id == SelectionId::Event(MESSAGE_DELETED)));
    assert!(snapshot
        .inspectors
        .contains_key(&SelectionId::Entity(DELETED_MESSAGE)));
    assert!(snapshot
        .inspectors
        .contains_key(&SelectionId::Event(recovered)));
    assert!(!snapshot
        .inspectors
        .contains_key(&SelectionId::Event(MESSAGE_DELETED)));

    let recovered_why = snapshot
        .why
        .get(&recovered)
        .expect("recovered evidence should expose its visible cause chain");
    assert_eq!(recovered_why.nodes.len(), 2);
    assert_eq!(recovered_why.nodes[0].event, recovered);
    assert_eq!(recovered_why.nodes[1].event, PROTOTYPE_COPIED);
    assert!(recovered_why.truncated);
}

#[test]
fn recovery_is_rejected_after_the_branch_point_has_passed() {
    let mut world = FutureArchaeologist::new().unwrap();
    world
        .choose_action(&format!("{RESPOND_COMMAND}:public_warning"))
        .unwrap();

    let error = world
        .invoke_projection_command(RECOVER_MESSAGE_COMMAND)
        .unwrap_err();
    assert!(matches!(error, FutureArchaeologistError::InvalidCommand(_)));
}
