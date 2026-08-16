use std::collections::BTreeMap;
use world_core::{
    Entity, EntityId, Event, EventId, Relation, RelationId, StateChange, World, WorldState,
};
use world_projection::{inspectors_from_world, ProjectionSnapshot, RelationIdentity, SelectionId};

fn event(id: u64, changes: Vec<StateChange>) -> Event {
    Event {
        id: EventId::new(id),
        kind: "relation_change".into(),
        world_time: id,
        actor: None,
        targets: Vec::new(),
        caused_by: Vec::new(),
        payload: BTreeMap::new(),
        changes,
    }
}

#[test]
fn relation_identity_is_stable_for_active_and_removed_latest_incarnations() {
    let one = EntityId::new(1);
    let two = EntityId::new(2);
    let relation = RelationId::new(5);
    let mut baseline = WorldState::default();
    baseline.seed_entity(Entity::new(one, "person")).unwrap();
    baseline.seed_entity(Entity::new(two, "person")).unwrap();
    baseline
        .seed_relation(Relation::new(relation, "knows", one, two))
        .unwrap();

    let active_world = World::new(baseline.clone());
    let active = ProjectionSnapshot {
        inspectors: inspectors_from_world(&active_world),
        ..ProjectionSnapshot::default()
    };
    assert_eq!(
        active.relation_identity(relation),
        Some(RelationIdentity { from: one, to: two })
    );

    let removed_world = World::from_history(
        baseline,
        &[event(1, vec![StateChange::RemoveRelation(relation)])],
    )
    .unwrap();
    let removed = ProjectionSnapshot {
        inspectors: inspectors_from_world(&removed_world),
        ..ProjectionSnapshot::default()
    };
    assert!(removed
        .inspector(SelectionId::Relation(relation))
        .unwrap()
        .subtitle
        .contains("Removed"));
    assert_eq!(
        removed.relation_identity(relation),
        Some(RelationIdentity { from: one, to: two })
    );
}
