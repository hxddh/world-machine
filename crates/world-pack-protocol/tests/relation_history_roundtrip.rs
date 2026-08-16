use std::collections::BTreeMap;
use world_core::{EventId, RelationId};
use world_pack_protocol::{
    decode_response, encode_response, PackResponse, PackResponseEnvelope, ProjectionSnapshotWire,
    ProtocolError, PACK_PROTOCOL_VERSION_V1, PACK_PROTOCOL_VERSION_V2,
};
use world_projection::{
    InspectorProjection, InspectorRow, InspectorSection, ProjectionSnapshot, RelationEventEvidence,
    SelectionId, TimelineItem, TimelineProjection, RELATION_HISTORY_SECTION,
};

fn relation_evidence_snapshot() -> (ProjectionSnapshot, TimelineItem, RelationId, EventId) {
    let relation_id = RelationId::new(5);
    let event_id = EventId::new(9);
    let relation = SelectionId::Relation(relation_id);
    let event = SelectionId::Event(event_id);
    let event_item = TimelineItem {
        id: event,
        world_time: 41,
        title: "Relation changed".into(),
        subtitle: "External Pack relation event".into(),
        caused_by: vec![EventId::new(8)],
    };
    let snapshot = ProjectionSnapshot {
        title: "External Relation World".into(),
        world_time: 42,
        timeline: TimelineProjection {
            items: vec![event_item.clone()],
        },
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
                        title: RELATION_HISTORY_SECTION.into(),
                        rows: vec![InspectorRow {
                            label: "World time 41 · Relation changed".into(),
                            value: event.stable_key(),
                        }],
                    },
                ],
            },
        )]),
        ..ProjectionSnapshot::default()
    };
    (snapshot, event_item, relation_id, event_id)
}

#[test]
fn protocol_v2_preserves_typed_relation_evidence_across_json_wire_round_trip() {
    let (snapshot, event_item, relation_id, event_id) = relation_evidence_snapshot();
    let envelope = PackResponseEnvelope::for_version(
        PACK_PROTOCOL_VERSION_V2,
        7,
        PackResponse::Snapshot {
            snapshot: ProjectionSnapshotWire::from(&snapshot),
        },
    )
    .expect("v2 should carry Relation evidence");

    let encoded = encode_response(&envelope).expect("response should encode");
    let decoded = decode_response(&encoded).expect("response should decode");
    let PackResponse::Snapshot { snapshot } = decoded.response else {
        panic!("expected snapshot response");
    };
    let restored = ProjectionSnapshot::try_from(snapshot).expect("snapshot should restore");

    assert_eq!(
        restored.relation_event_evidence(),
        vec![RelationEventEvidence {
            relation: relation_id,
            event: event_id,
        }]
    );
    assert_eq!(restored.relation_history(relation_id), vec![&event_item]);
    assert_eq!(
        restored.directly_changed_relations(event_id),
        vec![relation_id]
    );

    let inspector = restored
        .inspector(SelectionId::Relation(relation_id))
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
fn protocol_v1_rejects_the_same_relation_evidence_snapshot() {
    let (snapshot, _, _, _) = relation_evidence_snapshot();
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
