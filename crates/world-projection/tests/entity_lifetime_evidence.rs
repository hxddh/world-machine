use std::collections::BTreeMap;
use world_core::{
    Entity, EntityId, Event, EventId, Relation, RelationId, StateChange, Value, World, WorldState,
};
use world_projection::{
    inspectors_from_world, timeline_from_world, ProjectionSnapshot, SelectionId,
};

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

#[test]
fn recreating_entity_id_starts_a_new_evidence_lifetime() {
    let entity = EntityId::new(1);
    let survivor = EntityId::new(2);
    let relation = RelationId::new(5);

    let mut baseline = WorldState::default();
    baseline
        .seed_entity(Entity::new(entity, "old"))
        .expect("old entity should seed");
    baseline
        .seed_entity(Entity::new(survivor, "person"))
        .expect("survivor should seed");
    baseline
        .seed_relation(Relation::new(relation, "knows", entity, survivor))
        .expect("baseline relation should seed");

    let events = vec![
        recorded_event(
            1,
            "set_component",
            StateChange::SetComponent {
                entity,
                key: "generation".into(),
                value: Value::Integer(1),
            },
        ),
        recorded_event(2, "remove_entity", StateChange::RemoveEntity(entity)),
        recorded_event(
            3,
            "create_entity",
            StateChange::CreateEntity(Entity::new(entity, "replacement")),
        ),
        recorded_event(
            4,
            "set_component",
            StateChange::SetComponent {
                entity,
                key: "generation".into(),
                value: Value::Integer(2),
            },
        ),
    ];

    let world = World::from_history(baseline, &events).expect("entity lifetime should replay");
    let snapshot = ProjectionSnapshot {
        timeline: timeline_from_world(&world),
        inspectors: inspectors_from_world(&world),
        ..ProjectionSnapshot::default()
    };

    assert_eq!(
        snapshot
            .entity_history(entity)
            .iter()
            .map(|item| item.id)
            .collect::<Vec<_>>(),
        vec![
            SelectionId::Event(EventId::new(4)),
            SelectionId::Event(EventId::new(3)),
        ]
    );
    assert_eq!(
        snapshot
            .entity_history(survivor)
            .iter()
            .map(|item| item.id)
            .collect::<Vec<_>>(),
        vec![SelectionId::Event(EventId::new(2))]
    );

    assert!(snapshot
        .directly_changed_entities(EventId::new(1))
        .is_empty());
    assert_eq!(
        snapshot.directly_changed_entities(EventId::new(2)),
        vec![survivor]
    );
    assert_eq!(
        snapshot.directly_changed_entities(EventId::new(3)),
        vec![entity]
    );
    assert_eq!(
        snapshot.directly_changed_entities(EventId::new(4)),
        vec![entity]
    );
}

#[test]
fn same_event_remove_and_recreate_keeps_only_new_incarnation_evidence() {
    let entity = EntityId::new(1);
    let survivor = EntityId::new(2);
    let relation = RelationId::new(5);

    let mut baseline = WorldState::default();
    baseline
        .seed_entity(Entity::new(entity, "old"))
        .expect("old entity should seed");
    baseline
        .seed_entity(Entity::new(survivor, "person"))
        .expect("survivor should seed");
    baseline
        .seed_relation(Relation::new(relation, "knows", entity, survivor))
        .expect("baseline relation should seed");

    let old_event = recorded_event(
        1,
        "set_component",
        StateChange::SetComponent {
            entity,
            key: "generation".into(),
            value: Value::Integer(1),
        },
    );
    let transition = Event {
        id: EventId::new(2),
        kind: "replace_entity".into(),
        world_time: 2,
        actor: None,
        targets: Vec::new(),
        caused_by: Vec::new(),
        payload: BTreeMap::new(),
        changes: vec![
            StateChange::RemoveEntity(entity),
            StateChange::CreateEntity(Entity::new(entity, "replacement")),
            StateChange::SetComponent {
                entity,
                key: "generation".into(),
                value: Value::Integer(2),
            },
        ],
    };

    let world = World::from_history(baseline, &[old_event, transition])
        .expect("same-event lifetime transition should replay");
    let snapshot = ProjectionSnapshot {
        timeline: timeline_from_world(&world),
        inspectors: inspectors_from_world(&world),
        ..ProjectionSnapshot::default()
    };

    assert_eq!(
        snapshot
            .entity_history(entity)
            .iter()
            .map(|item| item.id)
            .collect::<Vec<_>>(),
        vec![SelectionId::Event(EventId::new(2))]
    );
    assert_eq!(
        snapshot
            .entity_history(survivor)
            .iter()
            .map(|item| item.id)
            .collect::<Vec<_>>(),
        vec![SelectionId::Event(EventId::new(2))]
    );
    assert!(snapshot
        .directly_changed_entities(EventId::new(1))
        .is_empty());
    assert_eq!(
        snapshot.directly_changed_entities(EventId::new(2)),
        vec![entity, survivor]
    );
}
