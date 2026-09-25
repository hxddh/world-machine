use world_core::RelationId;
use world_pack_protocol::{
    decode_response, encode_response, InspectorProjectionWire, PackResponse, PackResponseEnvelope,
    ProjectionSnapshotWire, ProtocolDecodeError, ProtocolError, SelectionIdWire,
    PACK_PROTOCOL_VERSION_V1, PACK_PROTOCOL_VERSION_V2,
};
use world_projection::{ProjectionSnapshot, SelectionId};

fn relation_snapshot() -> ProjectionSnapshotWire {
    ProjectionSnapshotWire {
        inspectors: vec![InspectorProjectionWire {
            selection: SelectionIdWire::Relation { id: 5 },
            title: "Works with".into(),
            subtitle: "Relation".into(),
            sections: Vec::new(),
        }],
        ..ProjectionSnapshotWire::default()
    }
}

#[test]
fn protocol_v1_envelope_rejects_relation_selection_even_though_parser_understands_it() {
    let response = PackResponse::Snapshot {
        snapshot: relation_snapshot(),
    };
    let error = PackResponseEnvelope::for_version(PACK_PROTOCOL_VERSION_V1, 1, response)
        .expect_err("v1 must reject a v2-only Relation selection");

    assert_eq!(
        error,
        ProtocolError::SelectionNotSupportedInProtocol {
            protocol_version: PACK_PROTOCOL_VERSION_V1,
            selection: "relation-5".into(),
        }
    );
}

#[test]
fn protocol_v2_envelope_round_trips_relation_selection_and_restores_projection_identity() {
    let envelope = PackResponseEnvelope::for_version(
        PACK_PROTOCOL_VERSION_V2,
        7,
        PackResponse::Snapshot {
            snapshot: relation_snapshot(),
        },
    )
    .expect("v2 must allow Relation selections");

    let encoded = encode_response(&envelope).unwrap();
    let decoded = decode_response(&encoded).unwrap();
    let PackResponse::Snapshot { snapshot } = decoded.response else {
        panic!("expected snapshot response");
    };
    let restored = ProjectionSnapshot::try_from(snapshot).unwrap();
    let relation = SelectionId::Relation(RelationId::new(5));

    assert_eq!(relation.stable_key(), "relation-5");
    assert_eq!(restored.inspector(relation).unwrap().title, "Works with");
}

#[test]
fn decoding_a_v1_relation_snapshot_fails_at_protocol_validation_not_json_parsing() {
    let envelope = PackResponseEnvelope {
        protocol_version: PACK_PROTOCOL_VERSION_V1,
        request_id: 9,
        response: PackResponse::Snapshot {
            snapshot: relation_snapshot(),
        },
    };
    let json = serde_json::to_string(&envelope).unwrap();
    let error = decode_response(&json).expect_err("v1 Relation snapshot must be rejected");

    assert!(matches!(
        error,
        ProtocolDecodeError::Protocol(ProtocolError::SelectionNotSupportedInProtocol {
            protocol_version: PACK_PROTOCOL_VERSION_V1,
            selection,
        }) if selection == "relation-5"
    ));
}

#[test]
fn protocol_v1_rejects_a_relation_as_who_asks_or_where_someone_stands() {
    let with = |patch: serde_json::Value| {
        let mut snapshot = serde_json::to_value(ProjectionSnapshotWire::default()).unwrap();
        for (key, value) in patch.as_object().unwrap() {
            snapshot[key] = value.clone();
        }
        serde_json::from_value::<ProjectionSnapshotWire>(snapshot).unwrap()
    };
    let asked = with(serde_json::json!({
        "commands": [{
            "id": "together",
            "title": "Together",
            "detail": "",
            "asker": { "type": "relation", "id": 5 }
        }]
    }));
    let standing = with(serde_json::json!({
        "canvas": { "links": [], "items": [{
            "id": { "type": "entity", "id": 1 },
            "kind": "actor",
            "label": "Nia",
            "detail": "",
            "x": 0.0,
            "y": 0.0,
            "at": { "type": "relation", "id": 5 }
        }]}
    }));
    let marked = with(serde_json::json!({
        "canvas": { "items": [], "links": [], "marks": [{
            "label": "Loop",
            "selection": { "type": "relation", "id": 5 }
        }]}
    }));

    for snapshot in [asked, standing, marked] {
        let error = PackResponseEnvelope::for_version(
            PACK_PROTOCOL_VERSION_V1,
            1,
            PackResponse::Snapshot {
                snapshot: snapshot.clone(),
            },
        )
        .expect_err("v1 must reject a Relation wherever it appears");
        assert_eq!(
            error,
            ProtocolError::SelectionNotSupportedInProtocol {
                protocol_version: PACK_PROTOCOL_VERSION_V1,
                selection: "relation-5".into(),
            }
        );
        PackResponseEnvelope::for_version(
            PACK_PROTOCOL_VERSION_V2,
            1,
            PackResponse::Snapshot { snapshot },
        )
        .expect("v2 allows it");
    }
}
