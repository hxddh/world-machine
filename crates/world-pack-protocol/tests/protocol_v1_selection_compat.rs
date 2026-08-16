use serde_json::Value;
use world_pack_protocol::{ProjectionSnapshotWire, SelectionIdWire, PACK_PROTOCOL_VERSION_V1};

#[test]
fn protocol_v1_selection_json_shape_stays_entity_or_event() {
    assert_eq!(PACK_PROTOCOL_VERSION_V1, 1);
    assert_eq!(
        serde_json::to_string(&SelectionIdWire::Entity { id: 7 })
            .expect("entity selection should encode"),
        r#"{"type":"entity","id":7}"#
    );
    assert_eq!(
        serde_json::to_string(&SelectionIdWire::Event { id: 9 })
            .expect("event selection should encode"),
        r#"{"type":"event","id":9}"#
    );
}

#[test]
fn relation_selection_is_a_parseable_superset_with_version_semantics_enforced_by_envelopes() {
    let decoded = serde_json::from_str::<SelectionIdWire>(r#"{"type":"relation","id":5}"#)
        .expect("the shared parser must understand the v2 Relation selection shape");
    assert_eq!(decoded, SelectionIdWire::Relation { id: 5 });
}

#[test]
fn protocol_v1_snapshot_keeps_typed_evidence_derived_not_wire_encoded() {
    let value = serde_json::to_value(ProjectionSnapshotWire::default())
        .expect("default snapshot should encode");
    let Value::Object(snapshot) = value else {
        panic!("snapshot wire should encode as a JSON object");
    };

    assert!(!snapshot.contains_key("entity_event_evidence"));
    assert_eq!(
        snapshot.keys().map(String::as_str).collect::<Vec<_>>(),
        vec![
            "briefing",
            "canvas",
            "capabilities",
            "collection",
            "commands",
            "inspectors",
            "timeline",
            "title",
            "why",
            "world_time",
        ]
    );
}
