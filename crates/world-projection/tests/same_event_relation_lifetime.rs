use std::collections::BTreeMap;
use world_core::{
    Entity, EntityId, Event, EventId, Relation, RelationId, StateChange, Value, World, WorldState,
};
use world_projection::{
    inspectors_from_world, timeline_from_world, ProjectionSnapshot, SelectionId,
};

fn snapshot(world: &World) -> ProjectionSnapshot {
    ProjectionSnapshot {
        timeline: timeline_from_world(world),
        inspectors: inspectors_from_world(world),
        ..ProjectionSnapshot::default()
    }
}

#[test]
fn same_event_relation_recreation_updates_lifetime_once_in_order() {
    let left = EntityId::new(1);
    let middle = EntityId::new(2);
    let right = EntityId::new(3);
    let relation = RelationId::new(5);

    let mut baseline = WorldState::default();
    for entity in [left, middle, right] {
        baseline
            .seed_entity(Entity::new(entity, "person"))
            .expect("entity should seed");
    }
    baseline
        .seed_relation(Relation::new(relation, "knows", left, middle))
        .expect("baseline relation should seed");

    let transition = Event {
        id: EventId::new(1),
        kind: "replace_relation".into(),
        world_time: 1,
        actor: None,
        targets: Vec::new(),
        caused_by: Vec::new(),
        payload: BTreeMap::new(),
        changes: vec![
            StateChange::RemoveRelation(relation),
            StateChange::CreateRelation(Relation::new(relation, "knows", middle, right)),
            StateChange::SetRelationProperty {
                relation,
                key: "strength".into(),
                value: Value::Integer(1),
            },
        ],
    };
    let follow_up = Event {
        id: EventId::new(2),
        kind: "set_relation_property".into(),
        world_time: 2,
        actor: None,
        targets: Vec::new(),
        caused_by: Vec::new(),
        payload: BTreeMap::new(),
        changes: vec![StateChange::SetRelationProperty {
            relation,
            key: "strength".into(),
            value: Value::Integer(2),
        }],
    };

    let world = World::from_history(baseline, &[transition, follow_up])
        .expect("same-event relation lifetime should replay");
    let snapshot = snapshot(&world);

    assert_eq!(
        snapshot
            .entity_history(left)
            .iter()
            .map(|item| item.id)
            .collect::<Vec<_>>(),
        vec![SelectionId::Event(EventId::new(1))]
    );
    assert_eq!(
        snapshot
            .entity_history(middle)
            .iter()
            .map(|item| item.id)
            .collect::<Vec<_>>(),
        vec![
            SelectionId::Event(EventId::new(2)),
            SelectionId::Event(EventId::new(1)),
        ]
    );
    assert_eq!(
        snapshot
            .entity_history(right)
            .iter()
            .map(|item| item.id)
            .collect::<Vec<_>>(),
        vec![
            SelectionId::Event(EventId::new(2)),
            SelectionId::Event(EventId::new(1)),
        ]
    );

    assert_eq!(
        snapshot.directly_changed_entities(EventId::new(1)),
        vec![left, middle, right]
    );
    assert_eq!(
        snapshot.directly_changed_entities(EventId::new(2)),
        vec![middle, right]
    );
}
