use std::collections::{BTreeMap, BTreeSet, VecDeque};
use world_core::EventId;

use crate::{InspectorProjection, SelectionId, TimelineItem, TimelineProjection};

pub(crate) fn influence_from_timeline(
    timeline: &TimelineProjection,
    root: EventId,
) -> Vec<(usize, &TimelineItem)> {
    let children = children_from_timeline(timeline);

    let mut visited = BTreeSet::from([root]);
    let mut queue = VecDeque::from([(root, 0_usize)]);
    let mut influenced = Vec::new();
    while let Some((parent, depth)) = queue.pop_front() {
        let Some(next) = children.get(&parent) else {
            continue;
        };
        for item in next {
            let SelectionId::Event(child) = item.id else {
                continue;
            };
            if !visited.insert(child) {
                continue;
            }
            let child_depth = depth + 1;
            influenced.push((child_depth, *item));
            queue.push_back((child, child_depth));
        }
    }

    influenced.sort_by(|(left_depth, left), (right_depth, right)| {
        left_depth
            .cmp(right_depth)
            .then_with(|| right.world_time.cmp(&left.world_time))
            .then_with(|| event_id(right).cmp(&event_id(left)))
    });
    influenced
}

pub(crate) fn semantic_influence_from_snapshot<'a>(
    timeline: &'a TimelineProjection,
    inspectors: &BTreeMap<SelectionId, InspectorProjection>,
    root: EventId,
) -> Vec<(usize, &'a TimelineItem)> {
    influence_from_timeline(timeline, root)
        .into_iter()
        .filter(|(_, item)| inspector_has_world_effect(inspectors.get(&item.id)))
        .collect()
}

pub(crate) fn semantic_path_from_snapshot<'a>(
    timeline: &'a TimelineProjection,
    inspectors: &BTreeMap<SelectionId, InspectorProjection>,
    root: EventId,
) -> Vec<&'a TimelineItem> {
    let by_id = timeline
        .items
        .iter()
        .filter_map(|item| match item.id {
            SelectionId::Event(event) => Some((event, item)),
            SelectionId::Entity(_) => None,
        })
        .collect::<BTreeMap<_, _>>();
    if !by_id.contains_key(&root) {
        return Vec::new();
    }

    let semantic = semantic_influence_from_snapshot(timeline, inspectors, root);
    if semantic.is_empty() {
        return Vec::new();
    }
    let semantic_ids = semantic
        .iter()
        .map(|(_, item)| event_id(item))
        .collect::<BTreeSet<_>>();
    let children = children_from_timeline(timeline);
    let terminal = semantic_ids
        .iter()
        .copied()
        .filter(|event| !has_semantic_descendant(*event, &children, &semantic_ids))
        .max_by(|left, right| {
            let left_item = by_id
                .get(left)
                .expect("semantic influence event must exist in Timeline");
            let right_item = by_id
                .get(right)
                .expect("semantic influence event must exist in Timeline");
            left_item
                .world_time
                .cmp(&right_item.world_time)
                .then_with(|| left.cmp(right))
        });
    let Some(terminal) = terminal else {
        return Vec::new();
    };

    let mut memo = BTreeMap::<EventId, Option<Vec<EventId>>>::new();
    let mut visiting = BTreeSet::new();
    let Some(path) =
        best_semantic_path_to(terminal, root, &by_id, inspectors, &mut memo, &mut visiting)
    else {
        return Vec::new();
    };

    path.into_iter()
        .filter_map(|event| by_id.get(&event).copied())
        .collect()
}

fn children_from_timeline(timeline: &TimelineProjection) -> BTreeMap<EventId, Vec<&TimelineItem>> {
    let mut children = BTreeMap::<EventId, Vec<&TimelineItem>>::new();
    for item in &timeline.items {
        for cause in &item.caused_by {
            children.entry(*cause).or_default().push(item);
        }
    }
    children
}

fn has_semantic_descendant(
    root: EventId,
    children: &BTreeMap<EventId, Vec<&TimelineItem>>,
    semantic: &BTreeSet<EventId>,
) -> bool {
    let mut visited = BTreeSet::from([root]);
    let mut queue = VecDeque::from([root]);
    while let Some(parent) = queue.pop_front() {
        let Some(next) = children.get(&parent) else {
            continue;
        };
        for item in next {
            let child = event_id(item);
            if !visited.insert(child) {
                continue;
            }
            if semantic.contains(&child) {
                return true;
            }
            queue.push_back(child);
        }
    }
    false
}

fn best_semantic_path_to(
    current: EventId,
    root: EventId,
    by_id: &BTreeMap<EventId, &TimelineItem>,
    inspectors: &BTreeMap<SelectionId, InspectorProjection>,
    memo: &mut BTreeMap<EventId, Option<Vec<EventId>>>,
    visiting: &mut BTreeSet<EventId>,
) -> Option<Vec<EventId>> {
    if current == root {
        return Some(Vec::new());
    }
    if let Some(cached) = memo.get(&current) {
        return cached.clone();
    }
    if !visiting.insert(current) {
        return None;
    }

    let result = by_id.get(&current).and_then(|item| {
        let semantic = inspector_has_world_effect(inspectors.get(&item.id));
        let mut best = None::<Vec<EventId>>;
        for cause in &item.caused_by {
            let Some(mut candidate) =
                best_semantic_path_to(*cause, root, by_id, inspectors, memo, visiting)
            else {
                continue;
            };
            if semantic {
                candidate.push(current);
            }
            let should_replace = best.as_ref().map_or(true, |existing| {
                candidate.len() > existing.len()
                    || (candidate.len() == existing.len() && candidate > *existing)
            });
            if should_replace {
                best = Some(candidate);
            }
        }
        best
    });

    visiting.remove(&current);
    memo.insert(current, result.clone());
    result
}

fn inspector_has_world_effect(inspector: Option<&InspectorProjection>) -> bool {
    inspector.is_some_and(|inspector| {
        inspector.sections.iter().any(|section| {
            (section.title == "Changes" && !section.rows.is_empty())
                || (section.title == "Payload"
                    && section.rows.iter().any(|row| {
                        matches!(row.label.as_str(), "Summary" | "Change")
                            && !row.value.trim().is_empty()
                    }))
        })
    })
}

fn event_id(item: &TimelineItem) -> EventId {
    match item.id {
        SelectionId::Event(event) => event,
        SelectionId::Entity(_) => unreachable!("Timeline items must select Events"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{InspectorRow, InspectorSection};

    fn item(id: u64, title: &str, caused_by: &[u64]) -> TimelineItem {
        TimelineItem {
            id: SelectionId::Event(EventId::new(id)),
            world_time: id,
            title: title.into(),
            subtitle: format!("Event #{id}"),
            caused_by: caused_by.iter().copied().map(EventId::new).collect(),
        }
    }

    fn inspector(id: u64, sections: Vec<InspectorSection>) -> InspectorProjection {
        InspectorProjection {
            selection: SelectionId::Event(EventId::new(id)),
            title: format!("Event {id}"),
            subtitle: String::new(),
            sections,
        }
    }

    #[test]
    fn influence_uses_shortest_causal_depth_and_newest_first_within_each_depth() {
        let timeline = TimelineProjection {
            items: vec![
                item(6, "Legacy", &[5]),
                item(5, "Growth", &[3, 4]),
                item(4, "Relationship", &[2]),
                item(3, "Agent", &[1]),
                item(2, "Growth", &[1]),
                item(1, "Choice", &[]),
            ],
        };

        let influenced = influence_from_timeline(&timeline, EventId::new(1));
        let actual = influenced
            .iter()
            .map(|(depth, item)| (*depth, event_id(item)))
            .collect::<Vec<_>>();

        assert_eq!(
            actual,
            vec![
                (1, EventId::new(3)),
                (1, EventId::new(2)),
                (2, EventId::new(5)),
                (2, EventId::new(4)),
                (3, EventId::new(6)),
            ]
        );
    }

    #[test]
    fn influence_does_not_repeat_events_reached_through_multiple_paths() {
        let timeline = TimelineProjection {
            items: vec![
                item(4, "Shared effect", &[2, 3]),
                item(3, "Right", &[1]),
                item(2, "Left", &[1]),
                item(1, "Choice", &[]),
            ],
        };

        let influenced = influence_from_timeline(&timeline, EventId::new(1));
        assert_eq!(
            influenced
                .iter()
                .filter(|(_, item)| item.id == SelectionId::Event(EventId::new(4)))
                .count(),
            1
        );
        assert_eq!(
            influenced
                .iter()
                .find(|(_, item)| item.id == SelectionId::Event(EventId::new(4)))
                .map(|(depth, _)| *depth),
            Some(2)
        );
    }

    #[test]
    fn influence_is_empty_when_nothing_depends_on_the_event() {
        let timeline = TimelineProjection {
            items: vec![item(1, "Choice", &[])],
        };
        assert!(influence_from_timeline(&timeline, EventId::new(1)).is_empty());
    }

    #[test]
    fn semantic_path_prefers_real_intermediate_world_stages_over_a_direct_shortcut() {
        let timeline = TimelineProjection {
            items: vec![
                item(5, "Final Effect", &[1, 4]),
                item(4, "Milestone", &[3]),
                item(3, "Supporting Record", &[2]),
                item(2, "First World Effect", &[1]),
                item(1, "Choice", &[]),
            ],
        };
        let changes = |id| {
            inspector(
                id,
                vec![InspectorSection {
                    title: "Changes".into(),
                    rows: vec![InspectorRow {
                        label: "Entity #1 · Status".into(),
                        value: format!("stage {id}"),
                    }],
                }],
            )
        };
        let inspectors = BTreeMap::from([
            (SelectionId::Event(EventId::new(2)), changes(2)),
            (SelectionId::Event(EventId::new(4)), changes(4)),
            (SelectionId::Event(EventId::new(5)), changes(5)),
        ]);

        let path = semantic_path_from_snapshot(&timeline, &inspectors, EventId::new(1));
        assert_eq!(
            path.iter().map(|item| item.id).collect::<Vec<_>>(),
            vec![
                SelectionId::Event(EventId::new(2)),
                SelectionId::Event(EventId::new(4)),
                SelectionId::Event(EventId::new(5)),
            ]
        );
    }

    #[test]
    fn semantic_influence_keeps_world_changes_and_explicit_summaries_without_kind_rules() {
        let timeline = TimelineProjection {
            items: vec![
                item(4, "Milestone Note", &[3]),
                item(3, "Actor Outcome", &[1, 2]),
                item(2, "Execution Record", &[1]),
                item(1, "Choice", &[]),
            ],
        };
        let inspectors = BTreeMap::from([
            (
                SelectionId::Event(EventId::new(2)),
                inspector(
                    2,
                    vec![InspectorSection {
                        title: "Payload".into(),
                        rows: vec![InspectorRow {
                            label: "Selected Action".into(),
                            value: "care".into(),
                        }],
                    }],
                ),
            ),
            (
                SelectionId::Event(EventId::new(3)),
                inspector(
                    3,
                    vec![InspectorSection {
                        title: "Changes".into(),
                        rows: vec![InspectorRow {
                            label: "World · Status".into(),
                            value: "changed".into(),
                        }],
                    }],
                ),
            ),
            (
                SelectionId::Event(EventId::new(4)),
                inspector(
                    4,
                    vec![InspectorSection {
                        title: "Payload".into(),
                        rows: vec![InspectorRow {
                            label: "Summary".into(),
                            value: "A durable milestone formed.".into(),
                        }],
                    }],
                ),
            ),
        ]);

        let raw = influence_from_timeline(&timeline, EventId::new(1));
        assert!(raw
            .iter()
            .any(|(_, item)| item.id == SelectionId::Event(EventId::new(2))));

        let semantic = semantic_influence_from_snapshot(&timeline, &inspectors, EventId::new(1));
        let ids = semantic.iter().map(|(_, item)| item.id).collect::<Vec<_>>();
        assert_eq!(
            ids,
            vec![
                SelectionId::Event(EventId::new(3)),
                SelectionId::Event(EventId::new(4)),
            ]
        );
    }
}
