use std::collections::BTreeMap;
use world_compare::{compare_snapshots, DifferenceKind};
use world_core::{EntityId, EventId};
use world_projection::{
    CanvasItem, CanvasItemKind, CanvasProjection, CollectionItem, CollectionProjection,
    InspectorProjection, InspectorRow, InspectorSection, ProjectionCommand, ProjectionSnapshot,
    SelectionId, TimelineItem, TimelineProjection,
};

fn entity(id: u64) -> SelectionId {
    SelectionId::Entity(EntityId::new(id))
}

fn event(id: u64) -> SelectionId {
    SelectionId::Event(EventId::new(id))
}

fn inspector(id: SelectionId, status: &str) -> InspectorProjection {
    InspectorProjection {
        selection: id,
        title: format!("Entity {}", id.stable_key()),
        subtitle: "Resident".into(),
        sections: vec![InspectorSection {
            title: "State".into(),
            rows: vec![InspectorRow {
                label: "Status".into(),
                value: status.into(),
            }],
        }],
    }
}

#[test]
fn entity_diff_uses_stable_selection_ids_and_inspector_rows() {
    let one = entity(1);
    let two = entity(2);
    let three = entity(3);
    let left = ProjectionSnapshot {
        collection: CollectionProjection {
            title: "Entities".into(),
            items: vec![
                CollectionItem {
                    id: one,
                    title: "One".into(),
                    subtitle: "active".into(),
                },
                CollectionItem {
                    id: two,
                    title: "Two".into(),
                    subtitle: "present".into(),
                },
            ],
        },
        inspectors: BTreeMap::from([
            (one, inspector(one, "active")),
            (two, inspector(two, "present")),
        ]),
        ..Default::default()
    };
    let right = ProjectionSnapshot {
        collection: CollectionProjection {
            title: "Entities".into(),
            items: vec![
                CollectionItem {
                    id: one,
                    title: "One".into(),
                    subtitle: "paused".into(),
                },
                CollectionItem {
                    id: three,
                    title: "Three".into(),
                    subtitle: "new".into(),
                },
            ],
        },
        inspectors: BTreeMap::from([
            (one, inspector(one, "paused")),
            (three, inspector(three, "new")),
        ]),
        ..Default::default()
    };

    let comparison = compare_snapshots(&left, &right);
    assert_eq!(comparison.entities.len(), 3);
    assert_eq!(comparison.entities[0].id, one);
    assert_eq!(comparison.entities[0].kind, DifferenceKind::Changed);
    assert_eq!(comparison.entities[0].inspector_rows.len(), 1);
    assert_eq!(
        comparison.entities[0].inspector_rows[0].key.section,
        "State"
    );
    assert_eq!(comparison.entities[0].inspector_rows[0].key.label, "Status");
    assert_eq!(
        comparison.entities[0].inspector_rows[0].left.as_deref(),
        Some("active")
    );
    assert_eq!(
        comparison.entities[0].inspector_rows[0].right.as_deref(),
        Some("paused")
    );
    assert_eq!(comparison.entities[1].id, two);
    assert_eq!(comparison.entities[1].kind, DifferenceKind::Removed);
    assert_eq!(comparison.entities[2].id, three);
    assert_eq!(comparison.entities[2].kind, DifferenceKind::Added);
}

#[test]
fn duplicate_inspector_labels_are_compared_by_occurrence() {
    let id = entity(1);
    let build = |values: [&str; 2]| InspectorProjection {
        selection: id,
        title: "Resident".into(),
        subtitle: "Resident".into(),
        sections: vec![InspectorSection {
            title: "Relations".into(),
            rows: values
                .into_iter()
                .map(|value| InspectorRow {
                    label: "Trusts".into(),
                    value: value.into(),
                })
                .collect(),
        }],
    };
    let left = ProjectionSnapshot {
        inspectors: BTreeMap::from([(id, build(["Leo", "Mara"]))]),
        ..Default::default()
    };
    let right = ProjectionSnapshot {
        inspectors: BTreeMap::from([(id, build(["Leo", "Emma"]))]),
        ..Default::default()
    };

    let rows = &compare_snapshots(&left, &right).entities[0].inspector_rows;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].key.occurrence, 1);
    assert_eq!(rows[0].left.as_deref(), Some("Mara"));
    assert_eq!(rows[0].right.as_deref(), Some("Emma"));
}

#[test]
fn timeline_detects_reused_event_ids_with_divergent_history() {
    let shared = event(10);
    let left_only = event(11);
    let right_only = event(12);
    let left = ProjectionSnapshot {
        timeline: TimelineProjection {
            items: vec![
                TimelineItem {
                    id: shared,
                    world_time: 20,
                    title: "Traditional reopen".into(),
                    subtitle: "left".into(),
                    caused_by: vec![EventId::new(5)],
                },
                TimelineItem {
                    id: left_only,
                    world_time: 21,
                    title: "Closed".into(),
                    subtitle: "left only".into(),
                    caused_by: vec![EventId::new(10)],
                },
            ],
        },
        ..Default::default()
    };
    let right = ProjectionSnapshot {
        timeline: TimelineProjection {
            items: vec![
                TimelineItem {
                    id: shared,
                    world_time: 20,
                    title: "Lean reopen".into(),
                    subtitle: "right".into(),
                    caused_by: vec![EventId::new(6)],
                },
                TimelineItem {
                    id: right_only,
                    world_time: 21,
                    title: "Stayed open".into(),
                    subtitle: "right only".into(),
                    caused_by: vec![EventId::new(10)],
                },
            ],
        },
        ..Default::default()
    };

    let timeline = compare_snapshots(&left, &right).timeline;
    assert_eq!(timeline.only_left.len(), 1);
    assert_eq!(timeline.only_left[0].id, left_only);
    assert_eq!(timeline.only_right.len(), 1);
    assert_eq!(timeline.only_right[0].id, right_only);
    assert_eq!(timeline.changed.len(), 1);
    assert_eq!(timeline.changed[0].id, shared);
    assert_eq!(timeline.changed[0].left.caused_by, vec![EventId::new(5)]);
    assert_eq!(timeline.changed[0].right.caused_by, vec![EventId::new(6)]);
}

#[test]
fn commands_are_keyed_by_semantic_command_id() {
    let command = |id: &str, detail: &str| ProjectionCommand {
        id: id.into(),
        title: id.into(),
        detail: detail.into(),
    };
    let left = ProjectionSnapshot {
        commands: vec![command("shared", "left"), command("left", "only")],
        ..Default::default()
    };
    let right = ProjectionSnapshot {
        commands: vec![command("shared", "right"), command("right", "only")],
        ..Default::default()
    };

    let commands = compare_snapshots(&left, &right).commands;
    assert_eq!(commands.only_left.len(), 1);
    assert_eq!(commands.only_left[0].id, "left");
    assert_eq!(commands.only_right.len(), 1);
    assert_eq!(commands.only_right[0].id, "right");
    assert_eq!(commands.changed.len(), 1);
    assert_eq!(commands.changed[0].id, "shared");
}

#[test]
fn canvas_geometry_does_not_create_semantic_differences() {
    let id = entity(1);
    let canvas_item = |x, y| CanvasItem {
        id,
        kind: CanvasItemKind::Actor,
        label: "Resident".into(),
        detail: "fisher".into(),
        x,
        y,
    };
    let left = ProjectionSnapshot {
        canvas: CanvasProjection {
            items: vec![canvas_item(0.1, 0.2)],
        },
        ..Default::default()
    };
    let right = ProjectionSnapshot {
        canvas: CanvasProjection {
            items: vec![canvas_item(0.9, 0.8)],
        },
        ..Default::default()
    };

    assert!(!compare_snapshots(&left, &right).has_differences());
}
