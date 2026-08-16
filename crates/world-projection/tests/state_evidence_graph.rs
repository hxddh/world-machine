use std::collections::BTreeMap;
use world_core::{EntityId, EventId, RelationId};
use world_projection::{
    EntityEventEvidence, EntityRelationEvidence, InspectorProjection, InspectorRow,
    InspectorSection, ProjectionSnapshot, RelationEndpointRole, RelationEventEvidence, SelectionId,
    StateEvidenceEdge, TimelineItem, TimelineProjection, ENTITY_HISTORY_SECTION,
    RELATION_ENDPOINTS_SECTION, RELATION_HISTORY_SECTION,
};

fn evidence_snapshot() -> ProjectionSnapshot {
    let one = SelectionId::Entity(EntityId::new(1));
    let two = SelectionId::Entity(EntityId::new(2));
    let relation = SelectionId::Relation(RelationId::new(5));
    let event = SelectionId::Event(EventId::new(9));
    ProjectionSnapshot {
        timeline: TimelineProjection {
            items: vec![TimelineItem {
                id: event,
                world_time: 9,
                title: "Changed".into(),
                subtitle: "Recorded change".into(),
                caused_by: Vec::new(),
            }],
        },
        inspectors: BTreeMap::from([
            (
                one,
                InspectorProjection {
                    selection: one,
                    title: "One".into(),
                    subtitle: "Person".into(),
                    sections: vec![InspectorSection {
                        title: ENTITY_HISTORY_SECTION.into(),
                        rows: vec![InspectorRow {
                            label: "World time 9 · Changed".into(),
                            value: event.stable_key(),
                        }],
                    }],
                },
            ),
            (
                two,
                InspectorProjection {
                    selection: two,
                    title: "Two".into(),
                    subtitle: "Person".into(),
                    sections: Vec::new(),
                },
            ),
            (
                relation,
                InspectorProjection {
                    selection: relation,
                    title: "Knows".into(),
                    subtitle: "Relation #5 · Active".into(),
                    sections: vec![
                        InspectorSection {
                            title: RELATION_ENDPOINTS_SECTION.into(),
                            rows: vec![
                                InspectorRow {
                                    label: "From".into(),
                                    value: one.stable_key(),
                                },
                                InspectorRow {
                                    label: "To".into(),
                                    value: two.stable_key(),
                                },
                            ],
                        },
                        InspectorSection {
                            title: RELATION_HISTORY_SECTION.into(),
                            rows: vec![InspectorRow {
                                label: "World time 9 · Changed".into(),
                                value: event.stable_key(),
                            }],
                        },
                    ],
                },
            ),
        ]),
        ..ProjectionSnapshot::default()
    }
}

#[test]
fn state_evidence_graph_unifies_current_and_recorded_edges() {
    let snapshot = evidence_snapshot();
    let entity_one = EntityId::new(1);
    let entity_two = EntityId::new(2);
    let relation = RelationId::new(5);
    let event = EventId::new(9);

    assert_eq!(
        snapshot.state_evidence_edges(),
        vec![
            StateEvidenceEdge::EntityEvent(EntityEventEvidence {
                entity: entity_one,
                event,
            }),
            StateEvidenceEdge::RelationEvent(RelationEventEvidence { relation, event }),
            StateEvidenceEdge::EntityRelation(EntityRelationEvidence {
                entity: entity_one,
                relation,
                role: RelationEndpointRole::From,
            }),
            StateEvidenceEdge::EntityRelation(EntityRelationEvidence {
                entity: entity_two,
                relation,
                role: RelationEndpointRole::To,
            }),
        ]
    );
}

#[test]
fn state_evidence_neighbors_traverse_any_selection_without_losing_types() {
    let snapshot = evidence_snapshot();
    let one = SelectionId::Entity(EntityId::new(1));
    let two = SelectionId::Entity(EntityId::new(2));
    let relation = SelectionId::Relation(RelationId::new(5));
    let event = SelectionId::Event(EventId::new(9));

    assert_eq!(
        snapshot.state_evidence_neighbors(one),
        vec![relation, event]
    );
    assert_eq!(snapshot.state_evidence_neighbors(two), vec![relation]);
    assert_eq!(
        snapshot.state_evidence_neighbors(relation),
        vec![one, two, event]
    );
    assert_eq!(
        snapshot.state_evidence_neighbors(event),
        vec![one, relation]
    );
}

#[test]
fn edge_other_only_returns_the_opposite_endpoint() {
    let edge = StateEvidenceEdge::EntityRelation(EntityRelationEvidence {
        entity: EntityId::new(1),
        relation: RelationId::new(5),
        role: RelationEndpointRole::From,
    });

    assert_eq!(
        edge.other(SelectionId::Entity(EntityId::new(1))),
        Some(SelectionId::Relation(RelationId::new(5)))
    );
    assert_eq!(
        edge.other(SelectionId::Relation(RelationId::new(5))),
        Some(SelectionId::Entity(EntityId::new(1)))
    );
    assert_eq!(edge.other(SelectionId::Event(EventId::new(9))), None);
}
