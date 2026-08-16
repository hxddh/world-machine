use std::collections::BTreeMap;
use world_core::{EntityId, EventId};
use world_pack_protocol::ProjectionSnapshotWire;
use world_projection::{
    InspectorProjection, InspectorRow, InspectorSection, ProjectionSnapshot, SelectionId,
    TimelineItem, TimelineProjection, ENTITY_HISTORY_SECTION,
};

#[test]
fn entity_evidence_edges_survive_pack_json_wire_round_trip() {
    let entity_id = EntityId::new(7);
    let event_id = EventId::new(9);
    let entity = SelectionId::Entity(entity_id);
    let event = SelectionId::Event(event_id);
    let event_item = TimelineItem {
        id: event,
        world_time: 41,
        title: "Changed".into(),
        subtitle: "External Pack event".into(),
        caused_by: vec![EventId::new(8)],
    };

    let snapshot = ProjectionSnapshot {
        title: "External World".into(),
        world_time: 42,
        timeline: TimelineProjection {
            items: vec![event_item.clone()],
        },
        inspectors: BTreeMap::from([(
            entity,
            InspectorProjection {
                selection: entity,
                title: "Seven".into(),
                subtitle: "Actor".into(),
                sections: vec![InspectorSection {
                    title: ENTITY_HISTORY_SECTION.into(),
                    rows: vec![InspectorRow {
                        label: "World time 41 · Changed".into(),
                        value: event.stable_key(),
                    }],
                }],
            },
        )]),
        ..ProjectionSnapshot::default()
    };

    let wire = ProjectionSnapshotWire::from(&snapshot);
    let json = serde_json::to_string(&wire).expect("Pack snapshot should encode");
    let decoded: ProjectionSnapshotWire =
        serde_json::from_str(&json).expect("Pack snapshot should decode");
    let restored = ProjectionSnapshot::try_from(decoded).expect("wire snapshot should restore");

    let history = restored.entity_history(entity_id);
    assert_eq!(history.len(), 1);
    assert_eq!(history[0], &event_item);
    assert_eq!(
        restored.directly_changed_entities(event_id),
        vec![entity_id]
    );
}
