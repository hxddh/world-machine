use std::collections::BTreeMap;
use world_core::{EntityId, RelationId};
use world_pack_protocol::{
    decode_response, encode_response, PackResponse, PackResponseEnvelope, ProjectionSnapshotWire,
    PACK_PROTOCOL_VERSION_V2,
};
use world_projection::{
    InspectorProjection, InspectorRow, InspectorSection, ProjectionSnapshot, RelationIdentity,
    SelectionId, RELATION_IDENTITY_SECTION,
};

#[test]
fn protocol_v2_preserves_removed_relation_identity_without_visible_endpoints() {
    let from = EntityId::new(7);
    let to = EntityId::new(9);
    let relation_id = RelationId::new(5);
    let relation = SelectionId::Relation(relation_id);
    let snapshot = ProjectionSnapshot {
        title: "External Relation Identity World".into(),
        inspectors: BTreeMap::from([(
            relation,
            InspectorProjection {
                selection: relation,
                title: "Works With".into(),
                subtitle: "Relation #5 · Removed".into(),
                sections: vec![
                    InspectorSection {
                        title: "Relation".into(),
                        rows: vec![InspectorRow {
                            label: "Status".into(),
                            value: "Removed".into(),
                        }],
                    },
                    InspectorSection {
                        title: RELATION_IDENTITY_SECTION.into(),
                        rows: vec![
                            InspectorRow {
                                label: "From".into(),
                                value: SelectionId::Entity(from).stable_key(),
                            },
                            InspectorRow {
                                label: "To".into(),
                                value: SelectionId::Entity(to).stable_key(),
                            },
                        ],
                    },
                ],
            },
        )]),
        ..ProjectionSnapshot::default()
    };

    let envelope = PackResponseEnvelope::for_version(
        PACK_PROTOCOL_VERSION_V2,
        7,
        PackResponse::Snapshot {
            snapshot: ProjectionSnapshotWire::from(&snapshot),
        },
    )
    .expect("v2 should carry stable Relation identity metadata");
    let encoded = encode_response(&envelope).expect("response should encode");
    let decoded = decode_response(&encoded).expect("response should decode");
    let PackResponse::Snapshot { snapshot } = decoded.response else {
        panic!("expected snapshot response");
    };
    let restored = ProjectionSnapshot::try_from(snapshot).expect("snapshot should restore");

    assert_eq!(
        restored.relation_identity(relation_id),
        Some(RelationIdentity { from, to })
    );
    assert!(restored.entities_for_relation(relation_id).is_empty());
    let inspector = restored
        .inspector(relation)
        .expect("removed Relation inspector should survive v2");
    assert_eq!(
        inspector
            .display_sections()
            .map(|section| section.title.as_str())
            .collect::<Vec<_>>(),
        vec!["Relation"]
    );
}
