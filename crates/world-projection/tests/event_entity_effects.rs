use std::collections::BTreeMap;
use world_core::{EntityId, EventId};
use world_projection::{
    InspectorProjection, InspectorRow, InspectorSection, ProjectionSnapshot, SelectionId,
    ENTITY_HISTORY_SECTION,
};

fn entity_inspector(entity_id: u64, event_keys: &[String]) -> (SelectionId, InspectorProjection) {
    let selection = SelectionId::Entity(EntityId::new(entity_id));
    (
        selection,
        InspectorProjection {
            selection,
            title: format!("Entity {entity_id}"),
            subtitle: "fixture".into(),
            sections: vec![InspectorSection {
                title: ENTITY_HISTORY_SECTION.into(),
                rows: event_keys
                    .iter()
                    .enumerate()
                    .map(|(index, key)| InspectorRow {
                        label: format!("Recorded change {index}"),
                        value: key.clone(),
                    })
                    .collect(),
            }],
        },
    )
}

#[test]
fn directly_changed_entities_inverts_recorded_entity_history_without_text_parsing() {
    let target_event = EventId::new(7);
    let other_event = EventId::new(8);
    let target_key = SelectionId::Event(target_event).stable_key();
    let other_key = SelectionId::Event(other_event).stable_key();

    let snapshot = ProjectionSnapshot {
        inspectors: BTreeMap::from([
            entity_inspector(3, &[target_key.clone()]),
            entity_inspector(1, &[other_key]),
            entity_inspector(2, &[target_key]),
        ]),
        ..ProjectionSnapshot::default()
    };

    assert_eq!(
        snapshot.directly_changed_entities(target_event),
        vec![EntityId::new(2), EntityId::new(3)]
    );
}
