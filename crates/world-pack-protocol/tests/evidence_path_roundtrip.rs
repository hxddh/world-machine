use std::collections::BTreeMap;
use world_core::{EntityId, EventId, RelationId};
use world_pack_protocol::{
    decode_response, encode_response, PackResponse, PackResponseEnvelope, ProjectionSnapshotWire,
    PACK_PROTOCOL_VERSION_V2,
};
use world_projection::{
    EntityEventEvidence, EntityRelationEvidence, InspectorProjection, InspectorRow,
    InspectorSection, ProjectionSnapshot, RelationEndpointRole, RelationEventEvidence, SelectionId,
    StateEvidenceEdge, StateEvidencePathStep, TimelineItem, TimelineProjection,
    ENTITY_HISTORY_SECTION, RELATION_ENDPOINTS_SECTION, RELATION_HISTORY_SECTION,
};

#[test]
fn protocol_v2_preserves_typed_shortest_evidence_path_across_json_wire_round_trip() {
    let one = SelectionId::Entity(EntityId::new(1));
    let two = SelectionId::Entity(EntityId::new(2));
    let three = SelectionId::Entity(EntityId::new(3));
    let relation = SelectionId::Relation(RelationId::new(5));
    let event = SelectionId::Event(EventId::new(9));
    let snapshot = ProjectionSnapshot {
        timeline: TimelineProjection {
            items: vec![TimelineItem {
                id: event,
                world_time: 9,
                title: "Changed".into(),
                subtitle: "External Pack recorded change".into(),
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
                    sections: Vec::new(),
                },
            ),
            (
                two,
                InspectorProjection {
                    selection: two,
                    title: "Two".into(),
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
                three,
                InspectorProjection {
                    selection: three,
                    title: "Three".into(),
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
                                    value: three.stable_key(),
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
    };

    let envelope = PackResponseEnvelope::for_version(
        PACK_PROTOCOL_VERSION_V2,
        7,
        PackResponse::Snapshot {
            snapshot: ProjectionSnapshotWire::from(&snapshot),
        },
    )
    .expect("v2 should carry evidence graph metadata");
    let encoded = encode_response(&envelope).expect("response should encode");
    let decoded = decode_response(&encoded).expect("response should decode");
    let PackResponse::Snapshot { snapshot } = decoded.response else {
        panic!("expected snapshot response");
    };
    let restored = ProjectionSnapshot::try_from(snapshot).expect("snapshot should restore");

    assert_eq!(
        restored.state_evidence_shortest_path(one, two),
        Some(vec![
            StateEvidencePathStep {
                from: one,
                edge: StateEvidenceEdge::EntityRelation(EntityRelationEvidence {
                    entity: EntityId::new(1),
                    relation: RelationId::new(5),
                    role: RelationEndpointRole::From,
                }),
                to: relation,
            },
            StateEvidencePathStep {
                from: relation,
                edge: StateEvidenceEdge::RelationEvent(RelationEventEvidence {
                    relation: RelationId::new(5),
                    event: EventId::new(9),
                }),
                to: event,
            },
            StateEvidencePathStep {
                from: event,
                edge: StateEvidenceEdge::EntityEvent(EntityEventEvidence {
                    entity: EntityId::new(2),
                    event: EventId::new(9),
                }),
                to: two,
            },
        ])
    );
}
