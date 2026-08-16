use std::collections::BTreeMap;
use world_core::{
    Entity, EntityId, Event, EventId, Relation, RelationId, StateChange, Value, World, WorldState,
};
use world_projection::{
    inspectors_from_world, timeline_from_world, ProjectionSnapshot, RelationEventEvidence,
    SelectionId,
};

fn snapshot(world: &World) -> ProjectionSnapshot {
    ProjectionSnapshot {
        timeline: timeline_from_world(world),
        inspectors: inspectors_from_world(world),
        ..ProjectionSnapshot::default()
    }
}

fn event(id: u64, kind: &str, changes: Vec<StateChange>) -> Event {
    Event {
        id: EventId::new(id),
        kind: kind.into(),
        world_time: id,
        actor: None,
        targets: Vec::new(),
        caused_by: Vec::new(),
        payload: BTreeMap::new(),
        changes,
    }
}

fn baseline_with_relation() -> (WorldState, EntityId, EntityId, EntityId, RelationId) {
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
        .seed_relation(
            Relation::new(relation, "knows", left, middle)
                .with_property("strength", Value::Integer(1)),
        )
        .expect("relation should seed");
    (baseline, left, middle, right, relation)
}

#[test]
fn relation_evidence_resets_when_the_same_relation_id_starts_a_new_incarnation() {
    let (baseline, _left, middle, right, relation) = baseline_with_relation();
    let history = vec![
        event(
            1,
            "strengthen_old_relation",
            vec![StateChange::SetRelationProperty {
                relation,
                key: "strength".into(),
                value: Value::Integer(2),
            }],
        ),
        event(
            2,
            "remove_old_relation",
            vec![StateChange::RemoveRelation(relation)],
        ),
        event(
            3,
            "create_new_relation",
            vec![StateChange::CreateRelation(Relation::new(
                relation, "supports", middle, right,
            ))],
        ),
        event(
            4,
            "strengthen_new_relation",
            vec![StateChange::SetRelationProperty {
                relation,
                key: "strength".into(),
                value: Value::Integer(9),
            }],
        ),
    ];
    let world = World::from_history(baseline, &history).expect("history should replay");
    let snapshot = snapshot(&world);

    let inspector = snapshot
        .inspector(SelectionId::Relation(relation))
        .expect("latest relation incarnation should be inspectable");
    assert!(inspector.subtitle.contains("Active"));
    assert_eq!(inspector.title, "Supports");
    let relation_rows = &inspector
        .display_sections()
        .find(|section| section.title == "Relation")
        .expect("relation section")
        .rows;
    assert!(relation_rows
        .iter()
        .any(|row| row.value.contains("Entity #2")));
    assert!(relation_rows
        .iter()
        .any(|row| row.value.contains("Entity #3")));

    assert_eq!(
        snapshot
            .relation_history(relation)
            .iter()
            .map(|item| item.id)
            .collect::<Vec<_>>(),
        vec![
            SelectionId::Event(EventId::new(4)),
            SelectionId::Event(EventId::new(3)),
        ]
    );
    assert!(snapshot
        .directly_changed_relations(EventId::new(1))
        .is_empty());
    assert!(snapshot
        .directly_changed_relations(EventId::new(2))
        .is_empty());
    assert_eq!(
        snapshot.directly_changed_relations(EventId::new(3)),
        vec![relation]
    );
    assert_eq!(
        snapshot.directly_changed_relations(EventId::new(4)),
        vec![relation]
    );
    assert_eq!(
        snapshot.relation_event_evidence(),
        vec![
            RelationEventEvidence {
                relation,
                event: EventId::new(3),
            },
            RelationEventEvidence {
                relation,
                event: EventId::new(4),
            },
        ]
    );
}

#[test]
fn removed_relation_keeps_a_tombstone_inspector_and_remove_event_history() {
    let (baseline, _left, _middle, _right, relation) = baseline_with_relation();
    let history = vec![
        event(
            1,
            "change_relation",
            vec![StateChange::SetRelationProperty {
                relation,
                key: "strength".into(),
                value: Value::Integer(2),
            }],
        ),
        event(
            2,
            "remove_relation",
            vec![StateChange::RemoveRelation(relation)],
        ),
    ];
    let world = World::from_history(baseline, &history).expect("history should replay");
    assert!(world.state().relation(relation).is_none());
    let snapshot = snapshot(&world);

    let inspector = snapshot
        .inspector(SelectionId::Relation(relation))
        .expect("removed latest incarnation should keep recorded evidence");
    assert!(inspector.subtitle.contains("Removed"));
    assert_eq!(
        snapshot
            .relation_history(relation)
            .iter()
            .map(|item| item.id)
            .collect::<Vec<_>>(),
        vec![
            SelectionId::Event(EventId::new(2)),
            SelectionId::Event(EventId::new(1)),
        ]
    );
    assert_eq!(
        snapshot.directly_changed_relations(EventId::new(2)),
        vec![relation]
    );
}

#[test]
fn removing_an_entity_records_the_implicit_relation_removal_on_the_relation() {
    let (baseline, left, _middle, _right, relation) = baseline_with_relation();
    let history = vec![event(
        1,
        "remove_person",
        vec![StateChange::RemoveEntity(left)],
    )];
    let world = World::from_history(baseline, &history).expect("history should replay");
    let snapshot = snapshot(&world);

    let inspector = snapshot
        .inspector(SelectionId::Relation(relation))
        .expect("implicitly removed relation should keep a tombstone inspector");
    assert!(inspector.subtitle.contains("Removed"));
    assert_eq!(
        snapshot
            .relation_history(relation)
            .iter()
            .map(|item| item.id)
            .collect::<Vec<_>>(),
        vec![SelectionId::Event(EventId::new(1))]
    );
    assert_eq!(
        snapshot.directly_changed_relations(EventId::new(1)),
        vec![relation]
    );
}
