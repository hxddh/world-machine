use std::collections::{BTreeMap, BTreeSet, VecDeque};
use world_core::EventId;

use crate::{SelectionId, TimelineItem, TimelineProjection};

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

fn event_id(item: &TimelineItem) -> EventId {
    match item.id {
        SelectionId::Event(event) => event,
        SelectionId::Entity(_) => unreachable!("Timeline items must select Events"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: u64, title: &str, caused_by: &[u64]) -> TimelineItem {
        TimelineItem {
            id: SelectionId::Event(EventId::new(id)),
            world_time: id,
            title: title.into(),
            subtitle: format!("Event #{id}"),
            caused_by: caused_by.iter().copied().map(EventId::new).collect(),
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
}
