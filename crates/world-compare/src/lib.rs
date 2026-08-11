use std::collections::{BTreeMap, BTreeSet};
use world_projection::{
    CanvasItemKind, InspectorProjection, ProjectionCommand, ProjectionSnapshot, SelectionId,
    TimelineItem,
};

#[derive(Clone, Debug, PartialEq)]
pub struct SnapshotComparison {
    pub left: ComparisonSide,
    pub right: ComparisonSide,
    pub entities: Vec<EntityDifference>,
    pub timeline: TimelineDifference,
    pub commands: CommandDifference,
}

impl SnapshotComparison {
    pub fn has_differences(&self) -> bool {
        self.left != self.right
            || !self.entities.is_empty()
            || !self.timeline.only_left.is_empty()
            || !self.timeline.only_right.is_empty()
            || !self.timeline.changed.is_empty()
            || !self.commands.only_left.is_empty()
            || !self.commands.only_right.is_empty()
            || !self.commands.changed.is_empty()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComparisonSide {
    pub title: String,
    pub world_time: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DifferenceKind {
    Added,
    Removed,
    Changed,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EntityDifference {
    pub id: SelectionId,
    pub kind: DifferenceKind,
    pub left: Option<EntitySummary>,
    pub right: Option<EntitySummary>,
    pub inspector_rows: Vec<InspectorRowDifference>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EntitySummary {
    pub title: String,
    pub subtitle: String,
    pub canvas_kind: Option<CanvasItemKind>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct InspectorRowKey {
    pub section: String,
    pub label: String,
    pub occurrence: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InspectorRowDifference {
    pub key: InspectorRowKey,
    pub left: Option<String>,
    pub right: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TimelineDifference {
    pub only_left: Vec<TimelineItem>,
    pub only_right: Vec<TimelineItem>,
    pub changed: Vec<TimelineItemDifference>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TimelineItemDifference {
    pub id: SelectionId,
    pub left: TimelineItem,
    pub right: TimelineItem,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CommandDifference {
    pub only_left: Vec<ProjectionCommand>,
    pub only_right: Vec<ProjectionCommand>,
    pub changed: Vec<CommandChange>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandChange {
    pub id: String,
    pub left: ProjectionCommand,
    pub right: ProjectionCommand,
}

pub fn compare_snapshots(
    left: &ProjectionSnapshot,
    right: &ProjectionSnapshot,
) -> SnapshotComparison {
    SnapshotComparison {
        left: ComparisonSide {
            title: left.title.clone(),
            world_time: left.world_time,
        },
        right: ComparisonSide {
            title: right.title.clone(),
            world_time: right.world_time,
        },
        entities: compare_entities(left, right),
        timeline: compare_timeline(left, right),
        commands: compare_commands(left, right),
    }
}

fn compare_entities(
    left: &ProjectionSnapshot,
    right: &ProjectionSnapshot,
) -> Vec<EntityDifference> {
    let left_ids = visible_entity_ids(left);
    let right_ids = visible_entity_ids(right);
    let ids = left_ids
        .union(&right_ids)
        .copied()
        .collect::<Vec<SelectionId>>();

    ids.into_iter()
        .filter_map(|id| {
            let left_present = left_ids.contains(&id);
            let right_present = right_ids.contains(&id);
            let left_summary = left_present.then(|| entity_summary(left, id));
            let right_summary = right_present.then(|| entity_summary(right, id));

            match (left_present, right_present) {
                (true, false) => Some(EntityDifference {
                    id,
                    kind: DifferenceKind::Removed,
                    left: left_summary,
                    right: None,
                    inspector_rows: Vec::new(),
                }),
                (false, true) => Some(EntityDifference {
                    id,
                    kind: DifferenceKind::Added,
                    left: None,
                    right: right_summary,
                    inspector_rows: Vec::new(),
                }),
                (true, true) => {
                    let inspector_rows =
                        compare_inspector_rows(left.inspectors.get(&id), right.inspectors.get(&id));
                    if left_summary != right_summary || !inspector_rows.is_empty() {
                        Some(EntityDifference {
                            id,
                            kind: DifferenceKind::Changed,
                            left: left_summary,
                            right: right_summary,
                            inspector_rows,
                        })
                    } else {
                        None
                    }
                }
                (false, false) => None,
            }
        })
        .collect()
}

fn visible_entity_ids(snapshot: &ProjectionSnapshot) -> BTreeSet<SelectionId> {
    snapshot
        .collection
        .items
        .iter()
        .map(|item| item.id)
        .chain(snapshot.canvas.items.iter().map(|item| item.id))
        .chain(snapshot.inspectors.keys().copied())
        .filter(|id| matches!(id, SelectionId::Entity(_)))
        .collect()
}

fn entity_summary(snapshot: &ProjectionSnapshot, id: SelectionId) -> EntitySummary {
    let collection = snapshot.collection.items.iter().find(|item| item.id == id);
    let inspector = snapshot.inspectors.get(&id);
    let canvas = snapshot.canvas.items.iter().find(|item| item.id == id);

    let (title, subtitle) = if let Some(item) = collection {
        (item.title.clone(), item.subtitle.clone())
    } else if let Some(inspector) = inspector {
        (inspector.title.clone(), inspector.subtitle.clone())
    } else if let Some(item) = canvas {
        (item.label.clone(), item.detail.clone())
    } else {
        (id.stable_key(), String::new())
    };

    EntitySummary {
        title,
        subtitle,
        canvas_kind: canvas.map(|item| item.kind),
    }
}

fn compare_inspector_rows(
    left: Option<&InspectorProjection>,
    right: Option<&InspectorProjection>,
) -> Vec<InspectorRowDifference> {
    let left = left.map(flatten_inspector_rows).unwrap_or_default();
    let right = right.map(flatten_inspector_rows).unwrap_or_default();
    let keys = left
        .keys()
        .chain(right.keys())
        .cloned()
        .collect::<BTreeSet<_>>();

    keys.into_iter()
        .filter_map(|key| {
            let left_value = left.get(&key).cloned();
            let right_value = right.get(&key).cloned();
            (left_value != right_value).then_some(InspectorRowDifference {
                key,
                left: left_value,
                right: right_value,
            })
        })
        .collect()
}

fn flatten_inspector_rows(inspector: &InspectorProjection) -> BTreeMap<InspectorRowKey, String> {
    let mut occurrences = BTreeMap::<(String, String), usize>::new();
    let mut rows = BTreeMap::new();

    for section in &inspector.sections {
        for row in &section.rows {
            let base = (section.title.clone(), row.label.clone());
            let occurrence = occurrences.entry(base.clone()).or_default();
            let key = InspectorRowKey {
                section: base.0,
                label: base.1,
                occurrence: *occurrence,
            };
            *occurrence += 1;
            rows.insert(key, row.value.clone());
        }
    }

    rows
}

fn compare_timeline(left: &ProjectionSnapshot, right: &ProjectionSnapshot) -> TimelineDifference {
    let left_items = timeline_by_id(left);
    let right_items = timeline_by_id(right);
    let ids = left_items
        .keys()
        .chain(right_items.keys())
        .copied()
        .collect::<BTreeSet<_>>();
    let mut difference = TimelineDifference::default();

    for id in ids {
        match (left_items.get(&id), right_items.get(&id)) {
            (Some(left_item), None) => difference.only_left.push((*left_item).clone()),
            (None, Some(right_item)) => difference.only_right.push((*right_item).clone()),
            (Some(left_item), Some(right_item)) if *left_item != *right_item => {
                difference.changed.push(TimelineItemDifference {
                    id,
                    left: (*left_item).clone(),
                    right: (*right_item).clone(),
                });
            }
            _ => {}
        }
    }

    difference
}

fn timeline_by_id(snapshot: &ProjectionSnapshot) -> BTreeMap<SelectionId, &TimelineItem> {
    let mut items = BTreeMap::new();
    for item in &snapshot.timeline.items {
        items.entry(item.id).or_insert(item);
    }
    items
}

fn compare_commands(left: &ProjectionSnapshot, right: &ProjectionSnapshot) -> CommandDifference {
    let left_commands = commands_by_id(left);
    let right_commands = commands_by_id(right);
    let ids = left_commands
        .keys()
        .chain(right_commands.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut difference = CommandDifference::default();

    for id in ids {
        match (left_commands.get(&id), right_commands.get(&id)) {
            (Some(left_command), None) => difference.only_left.push((*left_command).clone()),
            (None, Some(right_command)) => difference.only_right.push((*right_command).clone()),
            (Some(left_command), Some(right_command)) if *left_command != *right_command => {
                difference.changed.push(CommandChange {
                    id,
                    left: (*left_command).clone(),
                    right: (*right_command).clone(),
                });
            }
            _ => {}
        }
    }

    difference
}

fn commands_by_id(snapshot: &ProjectionSnapshot) -> BTreeMap<String, &ProjectionCommand> {
    let mut commands = BTreeMap::new();
    for command in &snapshot.commands {
        commands.entry(command.id.clone()).or_insert(command);
    }
    commands
}

#[cfg(test)]
mod tests {
    use super::*;
    use world_core::{EntityId, EventId};
    use world_projection::{
        CanvasItem, CanvasItemKind, CollectionItem, InspectorProjection, InspectorRow,
        InspectorSection, ProjectionCommand, TimelineItem,
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
        let mut left = ProjectionSnapshot::default();
        left.collection.items = vec![
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
        ];
        left.inspectors.insert(one, inspector(one, "active"));
        left.inspectors.insert(two, inspector(two, "present"));

        let mut right = ProjectionSnapshot::default();
        right.collection.items = vec![
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
        ];
        right.inspectors.insert(one, inspector(one, "paused"));
        right.inspectors.insert(three, inspector(three, "new"));

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
        let mut left = ProjectionSnapshot::default();
        left.inspectors.insert(id, build(["Leo", "Mara"]));
        let mut right = ProjectionSnapshot::default();
        right.inspectors.insert(id, build(["Leo", "Emma"]));

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
        let mut left = ProjectionSnapshot::default();
        left.timeline.items = vec![
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
        ];
        let mut right = ProjectionSnapshot::default();
        right.timeline.items = vec![
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
        ];

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
        let mut left = ProjectionSnapshot::default();
        left.commands = vec![command("shared", "left"), command("left", "only")];
        let mut right = ProjectionSnapshot::default();
        right.commands = vec![command("shared", "right"), command("right", "only")];

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
        let mut left = ProjectionSnapshot::default();
        left.canvas.items = vec![canvas_item(0.1, 0.2)];
        let mut right = ProjectionSnapshot::default();
        right.canvas.items = vec![canvas_item(0.9, 0.8)];

        assert!(!compare_snapshots(&left, &right).has_differences());
    }
}
