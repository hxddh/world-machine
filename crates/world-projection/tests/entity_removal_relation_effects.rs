use std::collections::BTreeMap;
use world_core::{Entity, EntityId, Event, EventId, Relation, RelationId, StateChange, World, WorldState};
use world_projection::{inspectors_from_world, timeline_from_world, ProjectionSnapshot, SelectionId};

fn recorded_event(id: u64, kind: &str, change: StateChange) -> Event {
    Event {
        id: EventId::new(id),
        kind: kind.into(),
        world_time: id,
        actor: None,
        targets: Vec::new(),
        caused_by: Vec::new(),
        payload: BTreeMap::new(),
        changes: vec![change],
    }
}

fn snapshot(world: &World) -> ProjectionSnapshot {
    ProjectionSnapshot {
        timeline: timeline_from_world(world),
        inspectors: inspectors_from_world(world),
        ..ProjectionSnapshot::default()
    }
}

#[test]
fn removing_entity_records_surviving_endpoint_of_baseline_relation() {
    let removed = EntityId::new(1);
    let survivor = EntityId::new(2);
    let relation = RelationId::new(5);

    let mut baseline = WorldState::default();
    baseline
        .seed_entity(Entity::new(removed, "person"))
        .expect("removed entity should seed");
    baseline
        .seed_entity(Entity::new(survivor, "person"))
        .expect("surviving entity should seed");
    baseline
        .seed_relation(Relation::new(relation, "knows", removed, survivor))
        .expect("baseline relation should seed");

    let event = recorded_event(1, "remove_entity", StateChange::RemoveEntity(removed));
    let world = World::from_history(baseline, &[event]).expect("entity removal should replay");
    let snapshot = snapshot(&world);

    assert!(snapshot.inspector(SelectionId::Entity(removed)).is_none());
    assert_eq!(
        snapshot
            .entity_history(survivor)
            .iter()
            .map(|item| item.id)
            .collect::<Vec<_>>(),
        vec![SelectionId::Event(EventId::new(1))]
    );
    assert_eq!(
        snapshot.directly_changed_entities(EventId::new(1)),
        vec![survivor]
    );
}

#[test]
fn removing_entity_records_surviving_endpoint_of_event_created_relation() {
    let survivor = EntityId::new(2);
    let removed = EntityId::new(3);
    let relation = RelationId::new(5);

    let mut baseline = WorldState::default();
    baseline
        .seed_entity(Entity::new(survivor, "person"))
        .expect("surviving entity should seed");
    baseline
        .seed_entity(Entity::new(removed, "person"))
        .expect("removed entity should seed");

    let events = vec![
        recorded_event(
            1,
            "create_relation",
            StateChange::CreateRelation(Relation::new(relation, "knows", survivor, removed)),
        ),
        recorded_event(2, "remove_entity", StateChange::RemoveEntity(removed)),
    ];
    let world = World::from_history(baseline, &events).expect("relation lifecycle should replay");
    let snapshot = snapshot(&world);

    assert_eq!(
        snapshot
            .entity_history(survivor)
            .iter()
            .map(|item| item.id)
            .collect::<Vec<_>>(),
        vec![
            SelectionId::Event(EventId::new(2)),
            SelectionId::Event(EventId::new(1)),
        ]
    );
    assert_eq!(
        snapshot.directly_changed_entities(EventId::new(2)),
        vec![survivor]
    );
}
