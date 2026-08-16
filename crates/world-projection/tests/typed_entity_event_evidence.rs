use std::collections::BTreeMap;
use world_core::{EntityId, EventId};
use world_projection::{
    EntityEventEvidence, InspectorProjection, InspectorRow, InspectorSection, ProjectionSnapshot,
    SelectionId, TimelineItem, TimelineProjection, ENTITY_HISTORY_SECTION,
};

fn inspector(selection: SelectionId, event_keys: &[String]) -> (SelectionId, InspectorProjection) {
    (
        selection,
        InspectorProjection {
            selection,
            title: selection.stable_key(),
            subtitle: "fixture".into(),
            sections: vec![InspectorSection {
                title: ENTITY_HISTORY_SECTION.into(),
                rows: event_keys
                    .iter()
                    .enumerate()
                    .map(|(index, value)| InspectorRow {
                        label: format!("Evidence {index}"),
                        value: value.clone(),
                    })
                    .collect(),
            }],
        },
    )
}

#[test]
fn typed_evidence_is_visible_deduplicated_and_shared_by_navigation_queries() {
    let event_7 = SelectionId::Event(EventId::new(7));
    let event_8 = SelectionId::Event(EventId::new(8));
    let hidden = SelectionId::Event(EventId::new(99));
    let entity_2 = SelectionId::Entity(EntityId::new(2));
    let entity_3 = SelectionId::Entity(EntityId::new(3));
    let entity_4 = SelectionId::Entity(EntityId::new(4));

    let event_7_key = event_7.stable_key();
    let event_8_key = event_8.stable_key();
    let hidden_key = hidden.stable_key();
    let entity_key = entity_2.stable_key();

    let snapshot = ProjectionSnapshot {
        timeline: TimelineProjection {
            items: vec![
                TimelineItem {
                    id: event_8,
                    world_time: 8,
                    title: "Eight".into(),
                    subtitle: "Visible".into(),
                    caused_by: Vec::new(),
                },
                TimelineItem {
                    id: event_7,
                    world_time: 7,
                    title: "Seven".into(),
                    subtitle: "Visible".into(),
                    caused_by: Vec::new(),
                },
            ],
        },
        inspectors: BTreeMap::from([
            inspector(
                entity_2,
                &[event_7_key.clone(), event_8_key.clone()],
            ),
            inspector(
                entity_3,
                &[
                    event_7_key.clone(),
                    event_7_key.clone(),
                    hidden_key.clone(),
                    entity_key,
                ],
            ),
            inspector(entity_4, std::slice::from_ref(&hidden_key)),
            inspector(event_7, &[event_8_key]),
        ]),
        ..ProjectionSnapshot::default()
    };

    assert_eq!(
        snapshot.entity_event_evidence(),
        vec![
            EntityEventEvidence {
                entity: EntityId::new(2),
                event: EventId::new(7),
            },
            EntityEventEvidence {
                entity: EntityId::new(2),
                event: EventId::new(8),
            },
            EntityEventEvidence {
                entity: EntityId::new(3),
                event: EventId::new(7),
            },
        ]
    );
    assert_eq!(
        snapshot
            .entity_history(EntityId::new(2))
            .iter()
            .map(|item| item.id)
            .collect::<Vec<_>>(),
        vec![event_8, event_7]
    );
    assert_eq!(
        snapshot.directly_changed_entities(EventId::new(7)),
        vec![EntityId::new(2), EntityId::new(3)]
    );
    assert!(snapshot
        .directly_changed_entities(EventId::new(99))
        .is_empty());
}
