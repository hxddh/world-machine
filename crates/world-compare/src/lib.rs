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
