use std::collections::{BTreeMap, BTreeSet};
use world_projection::{
    InspectorProjection, ProjectionCommand, ProjectionSnapshot, SelectionId, TimelineItem,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SnapshotSide {
    pub title: String,
    pub world_time: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SnapshotComparison {
    pub left: SnapshotSide,
    pub right: SnapshotSide,
    pub entities: Vec<EntityDifference>,
    pub timeline: TimelineDifference,
    pub commands: CommandDifference,
}

impl SnapshotComparison {
    pub fn is_identical(&self) -> bool {
        self.left == self.right
            && self.entities.is_empty()
            && self.timeline.is_empty()
            && self.commands.is_empty()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DifferenceKind {
    LeftOnly,
    RightOnly,
    Changed,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EntityDifference {
    pub id: SelectionId,
    pub kind: DifferenceKind,
    pub left: Option<EntityView>,
    pub right: Option<EntityView>,
    pub inspector_rows: Vec<InspectorRowDifference>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EntityView {
    pub title: String,
    pub subtitle: String,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct InspectorRowKey {
    pub section: String,
    pub label: String,
    pub ordinal: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct InspectorRowDifference {
    pub key: InspectorRowKey,
    pub kind: DifferenceKind,
    pub left: Option<String>,
    pub right: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TimelineDifference {
    pub left_only: Vec<TimelineItem>,
    pub right_only: Vec<TimelineItem>,
    pub changed: Vec<ChangedTimelineItem>,
}

impl TimelineDifference {
    pub fn is_empty(&self) -> bool {
        self.left_only.is_empty() && self.right_only.is_empty() && self.changed.is_empty()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChangedTimelineItem {
    pub id: SelectionId,
    pub left: TimelineItem,
    pub right: TimelineItem,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CommandDifference {
    pub left_only: Vec<ProjectionCommand>,
    pub right_only: Vec<ProjectionCommand>,
    pub changed: Vec<ChangedCommand>,
}

impl CommandDifference {
    pub fn is_empty(&self) -> bool {
        self.left_only.is_empty() && self.right_only.is_empty() && self.changed.is_empty()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChangedCommand {
    pub id: String,
    pub left: ProjectionCommand,
    pub right: ProjectionCommand,
}

pub fn compare_snapshots(
    left: &ProjectionSnapshot,
    right: &ProjectionSnapshot,
) -> SnapshotComparison {
    SnapshotComparison {
        left: SnapshotSide {
            title: left.title.clone(),
            world_time: left.world_time,
        },
        right: SnapshotSide {
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
    let left_entities = entity_inspectors(left);
    let right_entities = entity_inspectors(right);
    let ids = left_entities
        .keys()
        .chain(right_entities.keys())
        .copied()
        .collect::<BTreeSet<_>>();

    ids.into_iter()
        .filter_map(|id| match (left_entities.get(&id), right_entities.get(&id)) {
            (Some(left), Some(right)) if *left == *right => None,
            (Some(left), Some(right)) => Some(EntityDifference {
                id,
                kind: DifferenceKind::Changed,
                left: Some(entity_view(left)),
                right: Some(entity_view(right)),
                inspector_rows: compare_inspector_rows(left, right),
            }),
            (Some(left), None) => Some(EntityDifference {
                id,
                kind: DifferenceKind::LeftOnly,
                left: Some(entity_view(left)),
                right: None,
                inspector_rows: Vec::new(),
            }),
            (None, Some(right)) => Some(EntityDifference {
                id,
                kind: DifferenceKind::RightOnly,
                left: None,
                right: Some(entity_view(right)),
                inspector_rows: Vec::new(),
            }),
            (None, None) => None,
        })
        .collect()
}

fn entity_inspectors(snapshot: &ProjectionSnapshot) -> BTreeMap<SelectionId, &InspectorProjection> {
    snapshot
        .inspectors
        .iter()
        .filter_map(|(id, inspector)| match id {
            SelectionId::Entity(_) => Some((*id, inspector)),
            SelectionId::Event(_) => None,
        })
        .collect()
}

fn entity_view(inspector: &InspectorProjection) -> EntityView {
    EntityView {
        title: inspector.title.clone(),
        subtitle: inspector.subtitle.clone(),
    }
}

fn compare_inspector_rows(
    left: &InspectorProjection,
    right: &InspectorProjection,
) -> Vec<InspectorRowDifference> {
    let left_rows = indexed_rows(left);
    let right_rows = indexed_rows(right);
    let keys = left_rows
        .keys()
        .chain(right_rows.keys())
        .cloned()
        .collect::<BTreeSet<_>>();

    keys.into_iter()
        .filter_map(|key| match (left_rows.get(&key), right_rows.get(&key)) {
            (Some(left), Some(right)) if left == right => None,
            (Some(left), Some(right)) => Some(InspectorRowDifference {
                key,
                kind: DifferenceKind::Changed,
                left: Some((*left).clone()),
                right: Some((*right).clone()),
            }),
            (Some(left), None) => Some(InspectorRowDifference {
                key,
                kind: DifferenceKind::LeftOnly,
                left: Some((*left).clone()),
                right: None,
            }),
            (None, Some(right)) => Some(InspectorRowDifference {
                key,
                kind: DifferenceKind::RightOnly,
                left: None,
                right: Some((*right).clone()),
            }),
            (None, None) => None,
        })
        .collect()
}

fn indexed_rows(inspector: &InspectorProjection) -> BTreeMap<InspectorRowKey, &String> {
    let mut rows = BTreeMap::new();
    let mut duplicates = BTreeMap::<(String, String), usize>::new();

    for section in &inspector.sections {
        for row in &section.rows {
            let duplicate_key = (section.title.clone(), row.label.clone());
            let ordinal = duplicates.entry(duplicate_key.clone()).or_default();
            let key = InspectorRowKey {
                section: duplicate_key.0,
                label: duplicate_key.1,
                ordinal: *ordinal,
            };
            *ordinal += 1;
            rows.insert(key, &row.value);
        }
    }

    rows
}

fn compare_timeline(left: &ProjectionSnapshot, right: &ProjectionSnapshot) -> TimelineDifference {
    let left_items = left
        .timeline
        .items
        .iter()
        .map(|item| (item.id, item))
        .collect::<BTreeMap<_, _>>();
    let right_items = right
        .timeline
        .items
        .iter()
        .map(|item| (item.id, item))
        .collect::<BTreeMap<_, _>>();
    let ids = left_items
        .keys()
        .chain(right_items.keys())
        .copied()
        .collect::<BTreeSet<_>>();

    let mut difference = TimelineDifference::default();
    for id in ids {
        match (left_items.get(&id), right_items.get(&id)) {
            (Some(left), Some(right)) if *left == *right => {}
            (Some(left), Some(right)) => difference.changed.push(ChangedTimelineItem {
                id,
                left: (*left).clone(),
                right: (*right).clone(),
            }),
            (Some(left), None) => difference.left_only.push((*left).clone()),
            (None, Some(right)) => difference.right_only.push((*right).clone()),
            (None, None) => {}
        }
    }
    difference
}

fn compare_commands(left: &ProjectionSnapshot, right: &ProjectionSnapshot) -> CommandDifference {
    let left_commands = left
        .commands
        .iter()
        .map(|command| (command.id.as_str(), command))
        .collect::<BTreeMap<_, _>>();
    let right_commands = right
        .commands
        .iter()
        .map(|command| (command.id.as_str(), command))
        .collect::<BTreeMap<_, _>>();
    let ids = left_commands
        .keys()
        .chain(right_commands.keys())
        .copied()
        .collect::<BTreeSet<_>>();

    let mut difference = CommandDifference::default();
    for id in ids {
        match (left_commands.get(id), right_commands.get(id)) {
            (Some(left), Some(right)) if *left == *right => {}
            (Some(left), Some(right)) => difference.changed.push(ChangedCommand {
                id: id.to_string(),
                left: (*left).clone(),
                right: (*right).clone(),
            }),
            (Some(left), None) => difference.left_only.push((*left).clone()),
            (None, Some(right)) => difference.right_only.push((*right).clone()),
            (None, None) => {}
        }
    }
    difference
}

#[cfg(test)]
mod tests {
    use super::*;
    use world_core::{EntityId, EventId};
    use world_projection::{InspectorRow, InspectorSection, TimelineProjection};

    fn entity_inspector(id: u64, cash: &str, job: &str) -> (SelectionId, InspectorProjection) {
        let id = SelectionId::Entity(EntityId::new(id));
        (
            id,
            InspectorProjection {
                selection: id,
                title: format!("Resident {id:?}"),
                subtitle: "resident".into(),
                sections: vec![InspectorSection {
                    title: "State".into(),
                    rows: vec![
                        InspectorRow {
                            label: "cash".into(),
                            value: cash.into(),
                        },
                        InspectorRow {
                            label: "job".into(),
                            value: job.into(),
                        },
                    ],
                }],
            },
        )
    }

    fn event(id: u64, title: &str, world_time: u64) -> TimelineItem {
        TimelineItem {
            id: SelectionId::Event(EventId::new(id)),
            world_time,
            title: title.into(),
            subtitle: format!("{title} summary"),
            caused_by: Vec::new(),
        }
    }

    fn snapshot(
        world_time: u64,
        inspectors: impl IntoIterator<Item = (SelectionId, InspectorProjection)>,
        timeline: Vec<TimelineItem>,
        commands: Vec<ProjectionCommand>,
    ) -> ProjectionSnapshot {
        ProjectionSnapshot {
            title: "Synthetic World".into(),
            world_time,
            inspectors: inspectors.into_iter().collect(),
            timeline: TimelineProjection { items: timeline },
            commands,
            ..ProjectionSnapshot::default()
        }
    }

    #[test]
    fn identical_snapshots_have_no_differences() {
        let left = snapshot(
            10,
            [entity_inspector(1, "100", "baker")],
            vec![event(1, "opened", 10)],
            vec![ProjectionCommand {
                id: "world.act".into(),
                title: "Act".into(),
                detail: "Do something".into(),
            }],
        );

        let comparison = compare_snapshots(&left, &left);

        assert!(comparison.is_identical());
    }

    #[test]
    fn entity_rows_are_compared_inside_stable_entity_identity() {
        let left = snapshot(
            20,
            [entity_inspector(7, "40", "baker")],
            vec![],
            vec![],
        );
        let right = snapshot(
            20,
            [entity_inspector(7, "120", "owner_operator")],
            vec![],
            vec![],
        );

        let comparison = compare_snapshots(&left, &right);

        assert_eq!(comparison.entities.len(), 1);
        let entity = &comparison.entities[0];
        assert_eq!(entity.id, SelectionId::Entity(EntityId::new(7)));
        assert_eq!(entity.kind, DifferenceKind::Changed);
        assert_eq!(entity.inspector_rows.len(), 2);
        assert!(entity.inspector_rows.iter().any(|row| {
            row.key.label == "cash"
                && row.left.as_deref() == Some("40")
                && row.right.as_deref() == Some("120")
        }));
        assert!(entity.inspector_rows.iter().any(|row| {
            row.key.label == "job"
                && row.left.as_deref() == Some("baker")
                && row.right.as_deref() == Some("owner_operator")
        }));
    }

    #[test]
    fn equal_event_id_with_divergent_content_is_changed_not_equal() {
        let left = snapshot(30, [], vec![event(12, "traditional reopen", 30)], vec![]);
        let right = snapshot(30, [], vec![event(12, "lean reopen", 30)], vec![]);

        let comparison = compare_snapshots(&left, &right);

        assert!(comparison.timeline.left_only.is_empty());
        assert!(comparison.timeline.right_only.is_empty());
        assert_eq!(comparison.timeline.changed.len(), 1);
        assert_eq!(
            comparison.timeline.changed[0].id,
            SelectionId::Event(EventId::new(12))
        );
        assert_eq!(
            comparison.timeline.changed[0].left.title,
            "traditional reopen"
        );
        assert_eq!(comparison.timeline.changed[0].right.title, "lean reopen");
    }

    #[test]
    fn only_left_and_only_right_events_and_commands_are_stable() {
        let left = snapshot(
            40,
            [],
            vec![event(1, "common", 10), event(2, "left only", 20)],
            vec![ProjectionCommand {
                id: "left.action".into(),
                title: "Left".into(),
                detail: "left".into(),
            }],
        );
        let right = snapshot(
            40,
            [],
            vec![event(1, "common", 10), event(3, "right only", 20)],
            vec![ProjectionCommand {
                id: "right.action".into(),
                title: "Right".into(),
                detail: "right".into(),
            }],
        );

        let comparison = compare_snapshots(&left, &right);

        assert_eq!(comparison.timeline.left_only.len(), 1);
        assert_eq!(
            comparison.timeline.left_only[0].id,
            SelectionId::Event(EventId::new(2))
        );
        assert_eq!(comparison.timeline.right_only.len(), 1);
        assert_eq!(
            comparison.timeline.right_only[0].id,
            SelectionId::Event(EventId::new(3))
        );
        assert_eq!(comparison.commands.left_only[0].id, "left.action");
        assert_eq!(comparison.commands.right_only[0].id, "right.action");
    }

    #[test]
    fn added_and_removed_entities_are_reported() {
        let left = snapshot(0, [entity_inspector(1, "10", "left")], vec![], vec![]);
        let right = snapshot(0, [entity_inspector(2, "10", "right")], vec![], vec![]);

        let comparison = compare_snapshots(&left, &right);

        assert_eq!(comparison.entities.len(), 2);
        assert_eq!(comparison.entities[0].kind, DifferenceKind::LeftOnly);
        assert_eq!(comparison.entities[1].kind, DifferenceKind::RightOnly);
    }
}
