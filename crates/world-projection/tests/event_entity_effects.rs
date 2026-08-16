use std::collections::BTreeMap;
use world_core::{
    Entity, EntityId, Event, EventId, Relation, RelationId, StateChange, Value, World, WorldState,
};
use world_projection::{
    inspectors_from_world, timeline_from_world, InspectorProjection, InspectorRow, InspectorSection,
    ProjectionSnapshot, SelectionId, TimelineItem, TimelineProjection, ENTITY_HISTORY_SECTION,
};

fn entity_inspector(entity_id: u64, event_keys: &[String]) -> (SelectionId, InspectorProjection) {
    let selection = SelectionId::Entity(EntityId::new(entity_id));
    (
        selection,
        InspectorProjection {
            selection,
            title: format!("Entity {entity_id}"),
            subtitle: "fixture".into(),
            sections: vec![InspectorSection {
                title: ENTITY_HISTORY_SECTION.into(),
                rows: event_keys
                    .iter()
                    .enumerate()
                    .map(|(index, key)| InspectorRow {
                        label: format!("Recorded change {index}"),
                        value: key.clone(),
                    })
                    .collect(),
            }],
        },
    )
}

#[test]
fn directly_changed_entities_inverts_only_visible_recorded_entity_history() {
    let target_event = EventId::new(7);
    let other_event = EventId::new(8);
    let hidden_event = EventId::new(99);
    let target_key = SelectionId::Event(target_event).stable_key();
    let other_key = SelectionId::Event(other_event).stable_key();
    let hidden_key = SelectionId::Event(hidden_event).stable_key();

    let snapshot = ProjectionSnapshot {
        timeline: TimelineProjection {
            items: vec![TimelineItem {
                id: SelectionId::Event(target_event),
                world_time: 7,
                title: "Target event".into(),
                subtitle: "Visible recorded event".into(),
                caused_by: Vec::new(),
            }],
        },
        inspectors: BTreeMap::from([
            entity_inspector(3, std::slice::from_ref(&target_key)),
            entity_inspector(1, &[other_key]),
            entity_inspector(2, &[target_key]),
            entity_inspector(4, &[hidden_key]),
        ]),
        ..ProjectionSnapshot::default()
    };

    assert_eq!(
        snapshot.directly_changed_entities(target_event),
        vec![EntityId::new(2), EntityId::new(3)]
    );
    assert!(snapshot.directly_changed_entities(hidden_event).is_empty());
}

#[test]
fn relation_creation_records_only_endpoints_known_from_the_immutable_event() {
    let left = EntityId::new(1);
    let right = EntityId::new(2);
    let relation_id = RelationId::new(5);

    let mut baseline = WorldState::default();
    baseline
        .seed_entity(Entity::new(left, "person"))
        .expect("left entity should seed");
    baseline
        .seed_entity(Entity::new(right, "person"))
        .expect("right entity should seed");

    let events = vec![
        Event {
            id: EventId::new(1),
            kind: "create_relation".into(),
            world_time: 1,
            actor: None,
            targets: Vec::new(),
            caused_by: Vec::new(),
            payload: BTreeMap::new(),
            changes: vec![StateChange::CreateRelation(Relation::new(
                relation_id,
                "knows",
                left,
                right,
            ))],
        },
        Event {
            id: EventId::new(2),
            kind: "set_relation_property".into(),
            world_time: 2,
            actor: None,
            targets: Vec::new(),
            caused_by: Vec::new(),
            payload: BTreeMap::new(),
            changes: vec![StateChange::SetRelationProperty {
                relation: relation_id,
                key: "strength".into(),
                value: Value::Integer(1),
            }],
        },
        Event {
            id: EventId::new(3),
            kind: "remove_relation_property".into(),
            world_time: 3,
            actor: None,
            targets: Vec::new(),
            caused_by: Vec::new(),
            payload: BTreeMap::new(),
            changes: vec![StateChange::RemoveRelationProperty {
                relation: relation_id,
                key: "strength".into(),
            }],
        },
        Event {
            id: EventId::new(4),
            kind: "remove_relation".into(),
            world_time: 4,
            actor: None,
            targets: Vec::new(),
            caused_by: Vec::new(),
            payload: BTreeMap::new(),
            changes: vec![StateChange::RemoveRelation(relation_id)],
        },
    ];

    let world = World::from_history(baseline, &events).expect("relation history should replay");
    let snapshot = ProjectionSnapshot {
        timeline: timeline_from_world(&world),
        inspectors: inspectors_from_world(&world),
        ..ProjectionSnapshot::default()
    };

    for entity in [left, right] {
        assert_eq!(
            snapshot
                .entity_history(entity)
                .iter()
                .map(|item| item.id)
                .collect::<Vec<_>>(),
            vec![SelectionId::Event(EventId::new(1))]
        );
    }
    assert_eq!(
        snapshot.directly_changed_entities(EventId::new(1)),
        vec![left, right]
    );
    for event in [EventId::new(2), EventId::new(3), EventId::new(4)] {
        assert!(snapshot.directly_changed_entities(event).is_empty());
    }
}
