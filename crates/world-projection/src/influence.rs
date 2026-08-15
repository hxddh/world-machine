use std::collections::{BTreeMap, BTreeSet, VecDeque};
use world_core::EventId;

use crate::{InspectorProjection, SelectionId, TimelineItem, TimelineProjection};

pub(crate) fn influence_from_timeline(
    timeline: &TimelineProjection,
    root: EventId,
) -> Vec<(usize, &TimelineItem)> {
    let mut children = BTreeMap::<EventId, Vec<&TimelineItem>>::new();
    for item in &timeline.items {
        for cause in &item.caused_by {
            children.entry(*cause).or_default().push(item);
        }
    }

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
        let ids = semantic
            .iter()
            .map(|(_, item)| item.id)
            .collect::<Vec<_>>();
        assert_eq!(
            ids,
            vec![
                SelectionId::Event(EventId::new(3)),
                SelectionId::Event(EventId::new(4)),
            ]
        );
    }
}
