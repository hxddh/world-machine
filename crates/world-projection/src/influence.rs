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
    semantic_path_details_from_snapshot(timeline, inspectors, root)
        .into_iter()
        .map(|(_, item, _)| item)
        .collect()
}

pub(crate) fn semantic_path_details_from_snapshot<'a>(
    timeline: &'a TimelineProjection,
    inspectors: &BTreeMap<SelectionId, InspectorProjection>,
    root: EventId,
) -> Vec<(usize, &'a TimelineItem, String)> {
    let by_id = timeline
        .items
        .iter()
        .filter_map(|item| match item.id {
            SelectionId::Event(event) => Some((event, item)),
            SelectionId::Entity(_) => None,
        })
        .collect::<BTreeMap<_, _>>();
    let full_path = selected_path_event_ids(timeline, inspectors, root, &by_id);
    if full_path.is_empty() {
        return Vec::new();
    }

    let mut causal_steps = 0_usize;
    let mut details = Vec::new();
    for event in full_path {
        causal_steps += 1;
        let Some(item) = by_id.get(&event).copied() else {
            return Vec::new();
        };
        if !inspector_has_world_effect(inspectors.get(&item.id)) {
            continue;
        }
        let effect = semantic_effect_from_snapshot(timeline, inspectors, event)
            .unwrap_or_else(|| item.subtitle.clone());
        details.push((causal_steps, item, effect));
        causal_steps = 0;
    }
    details
}

fn selected_path_event_ids(
    timeline: &TimelineProjection,
    inspectors: &BTreeMap<SelectionId, InspectorProjection>,
    root: EventId,
    by_id: &BTreeMap<EventId, &TimelineItem>,
) -> Vec<EventId> {
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
    let mut descendant_memo = BTreeMap::new();
    let mut descendant_visiting = BTreeSet::new();
    let terminal = semantic_ids
        .iter()
        .copied()
        .filter(|event| {
            !has_semantic_descendant(
                *event,
                &children,
                &semantic_ids,
                &mut descendant_memo,
                &mut descendant_visiting,
            )
        })
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

    let mut memo = BTreeMap::<EventId, Option<BestPathState>>::new();
    let mut visiting = BTreeSet::new();
    if best_semantic_path_state(terminal, root, by_id, inspectors, &mut memo, &mut visiting)
        .is_none()
    {
        return Vec::new();
    }

    let mut path = Vec::new();
    let mut current = terminal;
    while current != root {
        path.push(current);
        let Some(state) = memo.get(&current).and_then(|state| *state) else {
            return Vec::new();
        };
        let Some(predecessor) = state.predecessor else {
            return Vec::new();
        };
        current = predecessor;
    }
    path.reverse();
    path
}

fn semantic_effect_from_snapshot(
    timeline: &TimelineProjection,
    inspectors: &BTreeMap<SelectionId, InspectorProjection>,
    event: EventId,
) -> Option<String> {
    let inspector = inspectors.get(&SelectionId::Event(event))?;
    let payload = inspector
        .sections
        .iter()
        .find(|section| section.title == "Payload");
    let summary = payload.and_then(|section| {
        section.rows.iter().find_map(|row| {
            matches!(row.label.as_str(), "Summary" | "Change")
                .then(|| row.value.trim())
                .filter(|value| !value.is_empty())
        })
    });
    let payload_labels = payload
        .map(|section| {
            section
                .rows
                .iter()
                .filter(|row| !matches!(row.label.as_str(), "Summary" | "Change"))
                .map(|row| row.label.as_str())
                .collect::<BTreeSet<_>>()
        })
        .unwrap_or_default();
    let changes = inspector
        .sections
        .iter()
        .find(|section| section.title == "Changes")
        .map(|section| section.rows.as_slice())
        .unwrap_or_default();

    let matched = changes
        .iter()
        .filter(|row| payload_labels.contains(change_field_label(&row.label)))
        .collect::<Vec<_>>();
    let evidence_rows = if !matched.is_empty() {
        matched
    } else if summary.is_none() {
        changes.iter().collect()
    } else {
        Vec::new()
    };
    let evidence = evidence_rows
        .iter()
        .take(2)
        .map(|row| recorded_transition(timeline, inspectors, event, row))
        .collect::<Vec<_>>();
    let hidden = evidence_rows.len().saturating_sub(evidence.len());

    match (summary, evidence.is_empty()) {
        (Some(summary), true) => Some(summary.to_string()),
        (Some(summary), false) => {
            let mut text = format!("{summary} · Recorded state · {}", evidence.join(" · "));
            if hidden > 0 {
                text.push_str(&format!(" · +{hidden} more recorded changes"));
            }
            Some(text)
        }
        (None, false) => {
            let mut text = format!("Recorded state · {}", evidence.join(" · "));
            if hidden > 0 {
                text.push_str(&format!(" · +{hidden} more recorded changes"));
            }
            Some(text)
        }
        (None, true) => None,
    }
}

fn change_field_label(label: &str) -> &str {
    label.rsplit_once(" · ").map_or(label, |(_, field)| field)
}

fn recorded_transition(
    timeline: &TimelineProjection,
    inspectors: &BTreeMap<SelectionId, InspectorProjection>,
    event: EventId,
    row: &crate::InspectorRow,
) -> String {
    match previous_recorded_value(timeline, inspectors, event, &row.label) {
        Some(previous) if previous != row.value => {
            format!("{} {previous} → {}", row.label, row.value)
        }
        Some(_) => format!("{} = {}", row.label, row.value),
        None => format!("{} → {}", row.label, row.value),
    }
}

fn previous_recorded_value(
    timeline: &TimelineProjection,
    inspectors: &BTreeMap<SelectionId, InspectorProjection>,
    event: EventId,
    label: &str,
) -> Option<String> {
    let current = timeline
        .items
        .iter()
        .position(|item| item.id == SelectionId::Event(event))?;
    timeline.items.iter().skip(current + 1).find_map(|item| {
        inspectors.get(&item.id).and_then(|inspector| {
            inspector
                .sections
                .iter()
                .find(|section| section.title == "Changes")
                .and_then(|section| section.rows.iter().find(|row| row.label == label))
                .map(|row| row.value.clone())
        })
    })
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
    memo: &mut BTreeMap<EventId, bool>,
    visiting: &mut BTreeSet<EventId>,
) -> bool {
    if let Some(cached) = memo.get(&root) {
        return *cached;
    }
    if !visiting.insert(root) {
        return false;
    }

    let result = children.get(&root).is_some_and(|next| {
        next.iter().any(|item| {
            let child = event_id(item);
            semantic.contains(&child)
                || has_semantic_descendant(child, children, semantic, memo, visiting)
        })
    });

    visiting.remove(&root);
    memo.insert(root, result);
    result
}

#[derive(Clone, Copy, Debug)]
struct BestPathState {
    semantic_count: usize,
    predecessor: Option<EventId>,
}

fn best_semantic_path_state(
    current: EventId,
    root: EventId,
    by_id: &BTreeMap<EventId, &TimelineItem>,
    inspectors: &BTreeMap<SelectionId, InspectorProjection>,
    memo: &mut BTreeMap<EventId, Option<BestPathState>>,
    visiting: &mut BTreeSet<EventId>,
) -> Option<BestPathState> {
    if current == root {
        return Some(BestPathState {
            semantic_count: 0,
            predecessor: None,
        });
    }
    if let Some(cached) = memo.get(&current) {
        return *cached;
    }
    if !visiting.insert(current) {
        return None;
    }

    let result = by_id.get(&current).and_then(|item| {
        let current_is_semantic = inspector_has_world_effect(inspectors.get(&item.id));
        let mut best = None::<(EventId, usize)>;
        for cause in &item.caused_by {
            let Some(previous) =
                best_semantic_path_state(*cause, root, by_id, inspectors, memo, visiting)
            else {
                continue;
            };
            let semantic_count = previous.semantic_count + usize::from(current_is_semantic);
            let should_replace = best.is_none_or(|(best_cause, best_count)| {
                semantic_count > best_count || (semantic_count == best_count && *cause > best_cause)
            });
            if should_replace {
                best = Some((*cause, semantic_count));
            }
        }
        best.map(|(predecessor, semantic_count)| BestPathState {
            semantic_count,
            predecessor: Some(predecessor),
        })
    });

    visiting.remove(&current);
    memo.insert(current, result);
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
    fn semantic_path_details_explain_recorded_effects_and_fold_supporting_spans() {
        let timeline = TimelineProjection {
            items: vec![
                item(5, "Final Effect", &[1, 4]),
                item(4, "Milestone", &[3]),
                item(3, "Supporting Record", &[2]),
                item(2, "First World Effect", &[1]),
                item(1, "Choice", &[]),
            ],
        };
        let change = |id, value: &str| {
            inspector(
                id,
                vec![InspectorSection {
                    title: "Changes".into(),
                    rows: vec![InspectorRow {
                        label: "Entity #1 · Status".into(),
                        value: value.into(),
                    }],
                }],
            )
        };
        let inspectors = BTreeMap::from([
            (SelectionId::Event(EventId::new(1)), change(1, "before")),
            (SelectionId::Event(EventId::new(2)), change(2, "first")),
            (SelectionId::Event(EventId::new(4)), change(4, "milestone")),
            (SelectionId::Event(EventId::new(5)), change(5, "final")),
        ]);

        let details = semantic_path_details_from_snapshot(&timeline, &inspectors, EventId::new(1));
        assert_eq!(details.len(), 3);
        assert_eq!(details[0].0, 1);
        assert_eq!(
            details[1].0, 2,
            "one supporting record should be folded between visible stages"
        );
        assert_eq!(details[2].0, 1);
        assert_eq!(details[0].1.id, SelectionId::Event(EventId::new(2)));
        assert_eq!(details[1].1.id, SelectionId::Event(EventId::new(4)));
        assert_eq!(details[2].1.id, SelectionId::Event(EventId::new(5)));
        assert!(details[0].2.contains("before → first"));
        assert!(details[1].2.contains("first → milestone"));
        assert!(details[2].2.contains("milestone → final"));
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
