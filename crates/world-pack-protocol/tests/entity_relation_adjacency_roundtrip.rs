use std::collections::BTreeMap;
use world_core::{EntityId, RelationId};
use world_pack_protocol::{
    decode_response, encode_response, PackResponse, PackResponseEnvelope, ProjectionSnapshotWire,
    ProtocolError, PACK_PROTOCOL_VERSION_V1, PACK_PROTOCOL_VERSION_V2,
};
use world_projection::{
    EntityRelationEvidence, InspectorProjection, InspectorRow, InspectorSection,
    ProjectionSnapshot, RelationEndpointRole, SelectionId, RELATION_ENDPOINTS_SECTION,
};

fn adjacency_snapshot() -> (ProjectionSnapshot, EntityId, EntityId, RelationId) {
    let left = EntityId::new(1);
    let right = EntityId::new(2);
    let relation = RelationId::new(5);
    let left_selection = SelectionId::Entity(left);
    let right_selection = SelectionId::Entity(right);
    let relation_selection = SelectionId::Relation(relation);

    let snapshot = ProjectionSnapshot {
        title: "External adjacency world".into(),
        inspectors: BTreeMap::from([
            (
                left_selection,
                InspectorProjection {
                    selection: left_selection,
                    title: "Left".into(),
                    subtitle: "Person".into(),
                    sections: Vec::new(),
                },
            ),
            (
                right_selection,
                InspectorProjection {
                    selection: right_selection,
                    title: "Right".into(),
                    subtitle: "Person".into(),
                    sections: Vec::new(),
                },
            ),
            (
                relation_selection,
                InspectorProjection {
                    selection: relation_selection,
                    title: "Works With".into(),
                    subtitle: "Relation #5 · Active".into(),
                    sections: vec![
                        InspectorSection {
                            title: "Relation".into(),
                            rows: vec![InspectorRow {
                                label: "Status".into(),
                                value: "Active".into(),
                            }],
                        },
                        InspectorSection {
                            title: RELATION_ENDPOINTS_SECTION.into(),
                            rows: vec![
                                InspectorRow {
                                    label: "From".into(),
                                    value: left_selection.stable_key(),
                                },
                                InspectorRow {
                                    label: "To".into(),
                                    value: right_selection.stable_key(),
                                },
                            ],
                        },
                    ],
                },
            ),
        ]),
        ..ProjectionSnapshot::default()
    };
    (snapshot, left, right, relation)
}

#[test]
fn protocol_v2_preserves_typed_entity_relation_adjacency_across_json_wire_round_trip() {
    let (snapshot, left, right, relation) = adjacency_snapshot();
    let envelope = PackResponseEnvelope::for_version(
        PACK_PROTOCOL_VERSION_V2,
        7,
        PackResponse::Snapshot {
            snapshot: ProjectionSnapshotWire::from(&snapshot),
        },
    )
    .expect("v2 should carry Relation adjacency metadata");

    let encoded = encode_response(&envelope).expect("response should encode");
    let decoded = decode_response(&encoded).expect("response should decode");
    let PackResponse::Snapshot { snapshot } = decoded.response else {
        panic!("expected snapshot response");
    };
    let restored = ProjectionSnapshot::try_from(snapshot).expect("snapshot should restore");

    assert_eq!(
        restored.entity_relation_evidence(),
        vec![
            EntityRelationEvidence {
                entity: left,
                relation,
                role: RelationEndpointRole::From,
            },
            EntityRelationEvidence {
                entity: right,
                relation,
                role: RelationEndpointRole::To,
            },
        ]
    );
    assert_eq!(restored.relations_for_entity(left), vec![relation]);
    assert_eq!(restored.relations_for_entity(right), vec![relation]);
    assert_eq!(restored.entities_for_relation(relation), vec![left, right]);

    let inspector = restored
        .inspector(SelectionId::Relation(relation))
        .expect("Relation inspector should survive v2");
    assert_eq!(
        inspector
            .display_sections()
            .map(|section| section.title.as_str())
            .collect::<Vec<_>>(),
        vec!["Relation"]
    );
}

#[test]
fn protocol_v1_rejects_the_same_relation_adjacency_snapshot() {
    let (snapshot, _, _, _) = adjacency_snapshot();
    let error = PackResponseEnvelope::for_version(
        PACK_PROTOCOL_VERSION_V1,
        1,
        PackResponse::Snapshot {
            snapshot: ProjectionSnapshotWire::from(&snapshot),
        },
    )
    .expect_err("v1 must reject Relation selections");

    assert!(matches!(
        error,
        ProtocolError::SelectionNotSupportedInProtocol {
            protocol_version: PACK_PROTOCOL_VERSION_V1,
            selection,
        } if selection == "relation-5"
    ));
}
