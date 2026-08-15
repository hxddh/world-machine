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

#[derive(Clone, Debug, PartialEq)]
pub struct SnapshotDivergence {
    pub shared_frontier: Option<TimelineItem>,
    pub left: DivergenceSide,
    pub right: DivergenceSide,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DivergenceSide {
    pub first_difference: Option<TimelineItem>,
    pub impact: Vec<DivergenceImpactStage>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DivergenceImpactStage {
    pub causal_steps: usize,
    pub event: TimelineItem,
    pub effect: String,
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

pub fn compare_divergence(
    left: &ProjectionSnapshot,
    right: &ProjectionSnapshot,
) -> Option<SnapshotDivergence> {
    let left_chronological = left.timeline.items.iter().rev().collect::<Vec<_>>();
    let right_chronological = right.timeline.items.iter().rev().collect::<Vec<_>>();
    let shared_len = left_chronological
        .iter()
        .zip(&right_chronological)
        .take_while(|(left_item, right_item)| {
            same_recorded_timeline_event(left, right, left_item, right_item)
        })
        .count();

    if shared_len == left_chronological.len() && shared_len == right_chronological.len() {
        return None;
    }

    Some(SnapshotDivergence {
        shared_frontier: shared_len
            .checked_sub(1)
            .and_then(|index| left_chronological.get(index))
            .map(|item| (*item).clone()),
        left: divergence_side(left, &left_chronological, shared_len),
        right: divergence_side(right, &right_chronological, shared_len),
    })
}

fn same_recorded_timeline_event(
    left_snapshot: &ProjectionSnapshot,
    right_snapshot: &ProjectionSnapshot,
    left: &TimelineItem,
    right: &TimelineItem,
) -> bool {
    left.id == right.id
        && left.world_time == right.world_time
        && left.title == right.title
        && left.caused_by == right.caused_by
        && recorded_event_evidence(left_snapshot, left.id)
            == recorded_event_evidence(right_snapshot, right.id)
}

fn recorded_event_evidence(
    snapshot: &ProjectionSnapshot,
    selection: SelectionId,
) -> Vec<(String, String, String)> {
    let Some(inspector) = snapshot.inspectors.get(&selection) else {
        return Vec::new();
    };

    inspector
        .sections
        .iter()
        .filter(|section| matches!(section.title.as_str(), "Payload" | "Changes"))
        .flat_map(|section| {
            section
                .rows
                .iter()
                .map(move |row| (section.title.clone(), row.label.clone(), row.value.clone()))
        })
        .collect()
}

fn divergence_side(
    snapshot: &ProjectionSnapshot,
    chronological: &[&TimelineItem],
    shared_len: usize,
) -> DivergenceSide {
    let first_difference = chronological.get(shared_len).map(|item| (*item).clone());
    let impact = first_difference
        .as_ref()
        .and_then(|item| match item.id {
            SelectionId::Event(event) => Some(event),
            SelectionId::Entity(_) => None,
        })
        .map(|event| {
            snapshot
                .semantic_path_details(event)
                .into_iter()
                .map(|(causal_steps, event, effect)| DivergenceImpactStage {
                    causal_steps,
                    event: event.clone(),
                    effect,
                })
                .collect()
        })
        .unwrap_or_default();

    DivergenceSide {
        first_difference,
        impact,
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
        .filter_map(
            |id| match (left_entities.get(&id), right_entities.get(&id)) {
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
            },
        )
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
        let left = snapshot(20, [entity_inspector(7, "40", "baker")], vec![], vec![]);
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

    #[test]
    fn divergence_ignores_current_state_derived_timeline_display_drift_in_shared_history() {
        let left_common = TimelineItem {
            id: SelectionId::Event(EventId::new(1)),
            world_time: 1,
            title: "Common".into(),
            subtitle: "Alice · Event #1".into(),
            caused_by: vec![],
        };
        let right_common = TimelineItem {
            subtitle: "Renamed Alice · Event #1".into(),
            ..left_common.clone()
        };
        let left_first = event(2, "Left choice", 2);
        let right_first = event(2, "Right choice", 2);
        let left = snapshot(2, [], vec![left_first.clone(), left_common.clone()], vec![]);
        let right = snapshot(2, [], vec![right_first.clone(), right_common], vec![]);

        let divergence = compare_divergence(&left, &right).expect("histories diverged");
        assert_eq!(divergence.shared_frontier, Some(left_common));
        assert_eq!(divergence.left.first_difference, Some(left_first));
        assert_eq!(divergence.right.first_difference, Some(right_first));
    }

    #[test]
    fn divergence_detects_same_id_semantic_difference_from_recorded_event_evidence() {
        let item = TimelineItem {
            id: SelectionId::Event(EventId::new(1)),
            world_time: 1,
            title: "Choice Made".into(),
            subtitle: "Event #1".into(),
            caused_by: vec![],
        };
        let inspector = |summary: &str| {
            let selection = SelectionId::Event(EventId::new(1));
            (
                selection,
                InspectorProjection {
                    selection,
                    title: "Choice Made".into(),
                    subtitle: String::new(),
                    sections: vec![InspectorSection {
                        title: "Payload".into(),
                        rows: vec![InspectorRow {
                            label: "Summary".into(),
                            value: summary.into(),
                        }],
                    }],
                },
            )
        };
        let left = snapshot(1, [inspector("Outward")], vec![item.clone()], vec![]);
        let right = snapshot(1, [inspector("Rooted")], vec![item.clone()], vec![]);

        let divergence = compare_divergence(&left, &right).expect("recorded semantics differ");
        assert_eq!(divergence.shared_frontier, None);
        assert_eq!(divergence.left.first_difference, Some(item.clone()));
        assert_eq!(divergence.right.first_difference, Some(item));
    }

    #[test]
    fn divergence_detects_non_summary_recorded_payload_difference() {
        let item = TimelineItem {
            id: SelectionId::Event(EventId::new(1)),
            world_time: 1,
            title: "Choice Made".into(),
            subtitle: "Event #1".into(),
            caused_by: vec![],
        };
        let inspector = |mode: &str| {
            let selection = SelectionId::Event(EventId::new(1));
            (
                selection,
                InspectorProjection {
                    selection,
                    title: "Choice Made".into(),
                    subtitle: String::new(),
                    sections: vec![InspectorSection {
                        title: "Payload".into(),
                        rows: vec![InspectorRow {
                            label: "Mode".into(),
                            value: mode.into(),
                        }],
                    }],
                },
            )
        };
        let left = snapshot(1, [inspector("outward")], vec![item.clone()], vec![]);
        let right = snapshot(1, [inspector("rooted")], vec![item.clone()], vec![]);

        let divergence = compare_divergence(&left, &right).expect("recorded payload differs");
        assert_eq!(divergence.shared_frontier, None);
        assert_eq!(divergence.left.first_difference, Some(item.clone()));
        assert_eq!(divergence.right.first_difference, Some(item));
    }

    #[test]
    fn divergence_ignores_current_state_derived_event_context_drift() {
        let common = TimelineItem {
            id: SelectionId::Event(EventId::new(1)),
            world_time: 1,
            title: "Common".into(),
            subtitle: "Event #1".into(),
            caused_by: vec![],
        };
        let left_first = event(2, "Left choice", 2);
        let right_first = event(2, "Right choice", 2);
        let inspector = |actor: &str| {
            let selection = SelectionId::Event(EventId::new(1));
            (
                selection,
                InspectorProjection {
                    selection,
                    title: "Common".into(),
                    subtitle: String::new(),
                    sections: vec![InspectorSection {
                        title: "Context".into(),
                        rows: vec![InspectorRow {
                            label: "Actor".into(),
                            value: actor.into(),
                        }],
                    }],
                },
            )
        };
        let left = snapshot(
            2,
            [inspector("Alice")],
            vec![left_first.clone(), common.clone()],
            vec![],
        );
        let right = snapshot(
            2,
            [inspector("Renamed Alice")],
            vec![right_first.clone(), common.clone()],
            vec![],
        );

        let divergence = compare_divergence(&left, &right).expect("later histories diverged");
        assert_eq!(divergence.shared_frontier, Some(common));
        assert_eq!(divergence.left.first_difference, Some(left_first));
        assert_eq!(divergence.right.first_difference, Some(right_first));
    }

    #[test]
    fn divergence_uses_the_longest_common_prefix_not_a_later_reconverged_event() {
        let common = event(1, "Common", 1);
        let left_first = event(2, "Left choice", 2);
        let right_first = event(2, "Right choice", 2);
        let reconverged = event(3, "Same later event", 3);
        let left = snapshot(
            3,
            [],
            vec![reconverged.clone(), left_first.clone(), common.clone()],
            vec![],
        );
        let right = snapshot(
            3,
            [],
            vec![reconverged, right_first.clone(), common.clone()],
            vec![],
        );

        let divergence = compare_divergence(&left, &right).expect("histories diverged");
        assert_eq!(divergence.shared_frontier, Some(common));
        assert_eq!(divergence.left.first_difference, Some(left_first));
        assert_eq!(divergence.right.first_difference, Some(right_first));
    }

    #[test]
    fn divergence_reuses_recorded_semantic_impact_from_each_first_difference() {
        let common = TimelineItem {
            id: SelectionId::Event(EventId::new(1)),
            world_time: 1,
            title: "Common".into(),
            subtitle: "Event #1".into(),
            caused_by: vec![],
        };
        let left_first = TimelineItem {
            id: SelectionId::Event(EventId::new(2)),
            world_time: 2,
            title: "Left choice".into(),
            subtitle: "Left choice · Event #2".into(),
            caused_by: vec![EventId::new(1)],
        };
        let left_support = TimelineItem {
            id: SelectionId::Event(EventId::new(3)),
            world_time: 3,
            title: "Supporting record".into(),
            subtitle: "Event #3".into(),
            caused_by: vec![EventId::new(2)],
        };
        let left_effect = TimelineItem {
            id: SelectionId::Event(EventId::new(4)),
            world_time: 4,
            title: "Left effect".into(),
            subtitle: "Left effect · Event #4".into(),
            caused_by: vec![EventId::new(3)],
        };
        let right_first = TimelineItem {
            id: SelectionId::Event(EventId::new(2)),
            world_time: 2,
            title: "Right choice".into(),
            subtitle: "Right choice · Event #2".into(),
            caused_by: vec![EventId::new(1)],
        };
        let right_effect = TimelineItem {
            id: SelectionId::Event(EventId::new(3)),
            world_time: 3,
            title: "Right effect".into(),
            subtitle: "Right effect · Event #3".into(),
            caused_by: vec![EventId::new(2)],
        };

        let effect_inspector = |id: u64, value: &str| {
            let selection = SelectionId::Event(EventId::new(id));
            (
                selection,
                InspectorProjection {
                    selection,
                    title: format!("Event {id}"),
                    subtitle: String::new(),
                    sections: vec![world_projection::InspectorSection {
                        title: "Changes".into(),
                        rows: vec![world_projection::InspectorRow {
                            label: "Entity #1 · State".into(),
                            value: value.into(),
                        }],
                    }],
                },
            )
        };

        let left = snapshot(
            4,
            [effect_inspector(4, "left")],
            vec![
                left_effect.clone(),
                left_support,
                left_first.clone(),
                common.clone(),
            ],
            vec![],
        );
        let right = snapshot(
            3,
            [effect_inspector(3, "right")],
            vec![right_effect.clone(), right_first.clone(), common],
            vec![],
        );

        let divergence = compare_divergence(&left, &right).expect("histories diverged");
        assert_eq!(divergence.left.first_difference, Some(left_first));
        assert_eq!(divergence.right.first_difference, Some(right_first));
        assert_eq!(divergence.left.impact.len(), 1);
        assert_eq!(divergence.left.impact[0].causal_steps, 2);
        assert_eq!(divergence.left.impact[0].event, left_effect);
        assert!(divergence.left.impact[0].effect.contains("left"));
        assert_eq!(divergence.right.impact.len(), 1);
        assert_eq!(divergence.right.impact[0].causal_steps, 1);
        assert_eq!(divergence.right.impact[0].event, right_effect);
        assert!(divergence.right.impact[0].effect.contains("right"));
    }

    #[test]
    fn ancestor_comparison_keeps_the_shared_frontier_and_one_sided_continuation() {
        let first = event(1, "First", 1);
        let frontier = event(2, "Frontier", 2);
        let continuation = event(3, "Continuation", 3);
        let left = snapshot(2, [], vec![frontier.clone(), first.clone()], vec![]);
        let right = snapshot(
            3,
            [],
            vec![continuation.clone(), frontier.clone(), first],
            vec![],
        );

        let divergence = compare_divergence(&left, &right).expect("right side continued");
        assert_eq!(divergence.shared_frontier, Some(frontier));
        assert_eq!(divergence.left.first_difference, None);
        assert_eq!(divergence.right.first_difference, Some(continuation));
    }
}
