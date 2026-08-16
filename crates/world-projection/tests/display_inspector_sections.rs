use world_core::EntityId;
use world_projection::{
    InspectorProjection, InspectorRow, InspectorSection, SelectionId, ENTITY_HISTORY_SECTION,
};

#[test]
fn display_sections_hide_only_machine_entity_history() {
    let inspector = InspectorProjection {
        selection: SelectionId::Entity(EntityId::new(7)),
        title: "Seven".into(),
        subtitle: "Actor".into(),
        sections: vec![
            InspectorSection {
                title: "State".into(),
                rows: vec![InspectorRow {
                    label: "Mood".into(),
                    value: "Calm".into(),
                }],
            },
            InspectorSection {
                title: ENTITY_HISTORY_SECTION.into(),
                rows: vec![InspectorRow {
                    label: "World time 1 · Changed".into(),
                    value: "event-1".into(),
                }],
            },
            InspectorSection {
                title: "Relations".into(),
                rows: vec![InspectorRow {
                    label: "Knows".into(),
                    value: "Two".into(),
                }],
            },
        ],
    };

    let sections = inspector.display_sections().collect::<Vec<_>>();
    assert_eq!(
        sections
            .iter()
            .map(|section| section.title.as_str())
            .collect::<Vec<_>>(),
        vec!["State", "Relations"]
    );
    assert_eq!(sections[0].rows[0].value, "Calm");
    assert_eq!(sections[1].rows[0].value, "Two");
}
