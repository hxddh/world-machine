use std::collections::BTreeMap;
use world_core::{
    Entity, EntityId, Event, EventId, Relation, RelationId, StateChange, Value, World, WorldState,
};
use world_projection::{
    inspectors_from_world, timeline_from_world, InspectorProjection, InspectorRow,
    InspectorSection, ProjectionSnapshot, SelectionId, TimelineItem, TimelineProjection,
    ENTITY_HISTORY_SECTION,
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
fn relation_lifetime_effects_follow_baseline_and_relation_id_reuse() {
    let left = EntityId::new(1);
    let middle = EntityId::new(2);
    let right = EntityId::new(3);
    let relation_id = RelationId::new(5);

    let mut baseline = WorldState::default();
    for entity in [left, middle, right] {
        baseline
            .seed_entity(Entity::new(entity, "person"))
            .expect("entity should seed");
    }
    baseline
        .seed_relation(Relation::new(relation_id, "knows", left, middle))
        .expect("baseline relation should seed");

    let events = vec![
        recorded_event(
            1,
            "set_relation_property",
            StateChange::SetRelationProperty {
                relation: relation_id,
                key: "strength".into(),
                value: Value::Integer(1),
            },
        ),
        recorded_event(
            2,
            "remove_relation_property",
            StateChange::RemoveRelationProperty {
                relation: relation_id,
                key: "strength".into(),
            },
        ),
        recorded_event(
            3,
            "remove_relation",
            StateChange::RemoveRelation(relation_id),
        ),
        recorded_event(
            4,
            "create_relation",
            StateChange::CreateRelation(Relation::new(
                relation_id,
                "knows",
                middle,
                right,
            )),
        ),
        recorded_event(
            5,
            "set_relation_property",
            StateChange::SetRelationProperty {
                relation: relation_id,
                key: "strength".into(),
                value: Value::Integer(2),
            },
        ),
        recorded_event(
            6,
            "remove_relation_property",
            StateChange::RemoveRelationProperty {
                relation: relation_id,
                key: "strength".into(),
            },
        ),
        recorded_event(
            7,
            "remove_relation",
            StateChange::RemoveRelation(relation_id),
        ),
    ];

    let world = World::from_history(baseline, &events).expect("relation history should replay");
    let snapshot = ProjectionSnapshot {
        timeline: timeline_from_world(&world),
        inspectors: inspectors_from_world(&world),
        ..ProjectionSnapshot::default()
    };

    assert_eq!(
        snapshot
            .entity_history(left)
            .iter()
            .map(|item| item.id)
            .collect::<Vec<_>>(),
        vec![
            SelectionId::Event(EventId::new(3)),
            SelectionId::Event(EventId::new(2)),
            SelectionId::Event(EventId::new(1)),
        ]
    );
    assert_eq!(
        snapshot
            .entity_history(middle)
            .iter()
            .map(|item| item.id)
            .collect::<Vec<_>>(),
        (1..=7)
            .rev()
            .map(|id| SelectionId::Event(EventId::new(id)))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        snapshot
            .entity_history(right)
            .iter()
            .map(|item| item.id)
            .collect::<Vec<_>>(),
        (4..=7)
            .rev()
            .map(|id| SelectionId::Event(EventId::new(id)))
            .collect::<Vec<_>>()
    );

    for (event, entities) in [
        (EventId::new(1), vec![left, middle]),
        (EventId::new(2), vec![left, middle]),
        (EventId::new(3), vec![left, middle]),
        (EventId::new(4), vec![middle, right]),
        (EventId::new(5), vec![middle, right]),
        (EventId::new(6), vec![middle, right]),
        (EventId::new(7), vec![middle, right]),
    ] {
        assert_eq!(snapshot.directly_changed_entities(event), entities);
    }
}
