use std::collections::BTreeMap;
use world_core::{
    Entity, EntityId, Event, EventId, Relation, RelationId, StateChange, World, WorldState,
};
use world_projection::{
    inspectors_from_world, timeline_from_world, EntityRelationEvidence, InspectorProjection,
    InspectorRow, InspectorSection, ProjectionSnapshot, RelationEndpointRole, SelectionId,
    RELATION_ENDPOINTS_SECTION,
};

fn snapshot(world: &World) -> ProjectionSnapshot {
    ProjectionSnapshot {
        timeline: timeline_from_world(world),
        inspectors: inspectors_from_world(world),
        ..ProjectionSnapshot::default()
    }
}

fn baseline() -> (WorldState, EntityId, EntityId, EntityId, RelationId) {
    let one = EntityId::new(1);
    let two = EntityId::new(2);
    let three = EntityId::new(3);
    let relation = RelationId::new(5);
    let mut state = WorldState::default();
    for entity in [one, two, three] {
        state
            .seed_entity(Entity::new(entity, "person"))
            .expect("entity should seed");
    }
    state
        .seed_relation(Relation::new(relation, "knows", one, two))
        .expect("relation should seed");
    (state, one, two, three, relation)
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

#[test]
fn active_relation_exposes_typed_entity_adjacency() {
    let (baseline, one, two, _three, relation) = baseline();
    let world = World::new(baseline);
    let snapshot = snapshot(&world);

    assert_eq!(
        snapshot.entity_relation_evidence(),
        vec![
            EntityRelationEvidence {
                entity: one,
                relation,
                role: RelationEndpointRole::From,
            },
            EntityRelationEvidence {
                entity: two,
                relation,
                role: RelationEndpointRole::To,
            },
        ]
    );
    assert_eq!(snapshot.relations_for_entity(one), vec![relation]);
    assert_eq!(snapshot.entities_for_relation(relation), vec![one, two]);
}

#[test]
fn self_relation_preserves_both_endpoint_roles_without_duplicating_convenience_results() {
    let entity = EntityId::new(1);
    let relation = RelationId::new(5);
    let mut baseline = WorldState::default();
    baseline
        .seed_entity(Entity::new(entity, "person"))
        .expect("entity should seed");
    baseline
        .seed_relation(Relation::new(relation, "reflects", entity, entity))
        .expect("self relation should seed");
    let snapshot = snapshot(&World::new(baseline));

    assert_eq!(
        snapshot.entity_relation_evidence(),
        vec![
            EntityRelationEvidence {
                entity,
                relation,
                role: RelationEndpointRole::From,
            },
            EntityRelationEvidence {
                entity,
                relation,
                role: RelationEndpointRole::To,
            },
        ]
    );
    assert_eq!(snapshot.relations_for_entity(entity), vec![relation]);
    assert_eq!(snapshot.entities_for_relation(relation), vec![entity]);
}

#[test]
fn removed_relation_tombstone_is_not_current_entity_adjacency() {
    let (baseline, one, two, _three, relation) = baseline();
    let world = World::from_history(
        baseline,
        &[event(
            1,
            "remove_relation",
            vec![StateChange::RemoveRelation(relation)],
        )],
    )
    .expect("history should replay");
    let snapshot = snapshot(&world);

    assert!(snapshot
        .inspector(SelectionId::Relation(relation))
        .expect("tombstone should remain inspectable")
        .subtitle
        .contains("Removed"));
    assert!(snapshot.entity_relation_evidence().is_empty());
    assert!(snapshot.relations_for_entity(one).is_empty());
    assert!(snapshot.relations_for_entity(two).is_empty());
    assert!(snapshot.entities_for_relation(relation).is_empty());
}

#[test]
fn reused_relation_id_exposes_only_latest_active_endpoints() {
    let (baseline, one, two, three, relation) = baseline();
    let world = World::from_history(
        baseline,
        &[
            event(1, "remove_old", vec![StateChange::RemoveRelation(relation)]),
            event(
                2,
                "create_new",
                vec![StateChange::CreateRelation(Relation::new(
                    relation, "supports", two, three,
                ))],
            ),
        ],
    )
    .expect("history should replay");
    let snapshot = snapshot(&world);

    assert!(snapshot.relations_for_entity(one).is_empty());
    assert_eq!(snapshot.relations_for_entity(two), vec![relation]);
    assert_eq!(snapshot.relations_for_entity(three), vec![relation]);
    assert_eq!(snapshot.entities_for_relation(relation), vec![two, three]);
}

#[test]
fn adjacency_does_not_expose_hidden_entity_ids_from_partial_snapshots() {
    let visible = EntityId::new(1);
    let hidden = EntityId::new(99);
    let relation = RelationId::new(5);
    let entity_selection = SelectionId::Entity(visible);
    let relation_selection = SelectionId::Relation(relation);
    let snapshot = ProjectionSnapshot {
        inspectors: BTreeMap::from([
            (
                entity_selection,
                InspectorProjection {
                    selection: entity_selection,
                    title: "Visible".into(),
                    subtitle: "Person".into(),
                    sections: Vec::new(),
                },
            ),
            (
                relation_selection,
                InspectorProjection {
                    selection: relation_selection,
                    title: "Knows".into(),
                    subtitle: "Relation #5 · Active".into(),
                    sections: vec![InspectorSection {
                        title: RELATION_ENDPOINTS_SECTION.into(),
                        rows: vec![
                            InspectorRow {
                                label: "From".into(),
                                value: SelectionId::Entity(visible).stable_key(),
                            },
                            InspectorRow {
                                label: "To".into(),
                                value: SelectionId::Entity(hidden).stable_key(),
                            },
                        ],
                    }],
                },
            ),
        ]),
        ..ProjectionSnapshot::default()
    };

    assert_eq!(
        snapshot.entity_relation_evidence(),
        vec![EntityRelationEvidence {
            entity: visible,
            relation,
            role: RelationEndpointRole::From,
        }]
    );
    assert_eq!(snapshot.entities_for_relation(relation), vec![visible]);
}
