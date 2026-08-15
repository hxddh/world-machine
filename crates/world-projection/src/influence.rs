use std::collections::{BTreeMap, VecDeque};
use world_core::{Event, EventId, World};

use crate::{event_summary, humanize, InspectorRow};

const MAX_VISIBLE_EFFECT_KINDS: usize = 6;

#[derive(Clone, Debug)]
struct EffectGroup<'a> {
    kind: String,
    min_depth: usize,
    count: usize,
    latest: &'a Event,
}

pub(crate) fn influence_rows(world: &World, root: EventId) -> Vec<InspectorRow> {
    let depths = descendant_depths(world, root);
    if depths.is_empty() {
        return Vec::new();
    }

    let total = depths.len();
    let direct = depths.values().filter(|depth| **depth == 1).count();
    let max_depth = depths.values().copied().max().unwrap_or(1);
    let mut groups = Vec::<EffectGroup<'_>>::new();

    for event in world.events() {
        let Some(depth) = depths.get(&event.id).copied() else {
            continue;
        };
        if let Some(group) = groups.iter_mut().find(|group| group.kind == event.kind) {
            group.min_depth = group.min_depth.min(depth);
            group.count += 1;
            group.latest = event;
        } else {
            groups.push(EffectGroup {
                kind: event.kind.clone(),
                min_depth: depth,
                count: 1,
                latest: event,
            });
        }
    }

    groups.sort_by_key(|group| group.min_depth);

    let mut rows = vec![InspectorRow {
        label: "Reach".into(),
        value: format!(
            "{} later {} · {} direct · {} causal {}",
            total,
            if total == 1 { "Event" } else { "Events" },
            direct,
            max_depth,
            if max_depth == 1 { "step" } else { "steps" },
        ),
    }];

    for group in groups.iter().take(MAX_VISIBLE_EFFECT_KINDS) {
        let label = if group.min_depth == 1 {
            "Direct effect".to_string()
        } else {
            format!("Later · {} steps", group.min_depth)
        };
        let title = humanize(&group.kind);
        let summary = event_summary(group.latest, world);
        let value = if group.count == 1 {
            format!("{title} · {summary}")
        } else {
            format!("{title} · {} Events · latest {summary}", group.count)
        };
        rows.push(InspectorRow { label, value });
    }

    if groups.len() > MAX_VISIBLE_EFFECT_KINDS {
        rows.push(InspectorRow {
            label: "More effects".into(),
            value: format!(
                "{} additional effect types not shown",
                groups.len() - MAX_VISIBLE_EFFECT_KINDS
            ),
        });
    }

    rows
}

fn descendant_depths(world: &World, root: EventId) -> BTreeMap<EventId, usize> {
    let mut children = BTreeMap::<EventId, Vec<EventId>>::new();
    for event in world.events() {
        for cause in &event.caused_by {
            children.entry(*cause).or_default().push(event.id);
        }
    }

    let mut depths = BTreeMap::new();
    let mut queue = VecDeque::from([(root, 0_usize)]);
    while let Some((parent, depth)) = queue.pop_front() {
        let Some(next) = children.get(&parent) else {
            continue;
        };
        for child in next {
            if depths.contains_key(child) || *child == root {
                continue;
            }
            let child_depth = depth + 1;
            depths.insert(*child, child_depth);
            queue.push_back((*child, child_depth));
        }
    }
    depths
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use world_core::{Entity, EntityId, Event, WorldState};

    fn world_with(events: Vec<Event>) -> World {
        let mut state = WorldState::default();
        state
            .seed_entity(Entity::new(EntityId::new(1), "world").with_component("name", "World"))
            .unwrap();
        World::from_history(state, &events).unwrap()
    }

    fn event(id: u64, kind: &str, caused_by: &[u64]) -> Event {
        Event {
            id: EventId::new(id),
            kind: kind.into(),
            world_time: id,
            actor: None,
            targets: vec![],
            caused_by: caused_by.iter().copied().map(EventId::new).collect(),
            payload: BTreeMap::new(),
            changes: vec![],
        }
    }

    #[test]
    fn influence_groups_repeated_effect_kinds_and_preserves_shortest_depth() {
        let world = world_with(vec![
            event(1, "choice_made", &[]),
            event(2, "relationship_shifted", &[1]),
            event(3, "relationship_shifted", &[1]),
            event(4, "agent_decision_recorded", &[1]),
            event(5, "universe_grew", &[2]),
            event(6, "world_legacy_formed", &[5]),
        ]);

        let rows = influence_rows(&world, EventId::new(1));
        assert_eq!(rows[0].label, "Reach");
        assert_eq!(rows[0].value, "5 later Events · 3 direct · 3 causal steps");
        assert_eq!(rows[1].label, "Direct effect");
        assert!(rows[1].value.contains("Relationship Shifted · 2 Events"));
        assert!(rows[1].value.contains("Event #3"));
        assert_eq!(rows[2].label, "Direct effect");
        assert!(rows[2].value.contains("Agent Decision Recorded"));
        assert_eq!(rows[3].label, "Later · 2 steps");
        assert!(rows[3].value.contains("Universe Grew"));
        assert_eq!(rows[4].label, "Later · 3 steps");
        assert!(rows[4].value.contains("World Legacy Formed"));
    }

    #[test]
    fn influence_is_bounded_by_effect_kind_with_explicit_overflow() {
        let mut events = vec![event(1, "choice_made", &[])];
        for id in 2..=9 {
            events.push(event(id, &format!("effect_{id}"), &[1]));
        }
        let world = world_with(events);

        let rows = influence_rows(&world, EventId::new(1));
        assert_eq!(rows[0].value, "8 later Events · 8 direct · 1 causal step");
        assert_eq!(rows.len(), 8);
        assert_eq!(rows.last().unwrap().label, "More effects");
        assert_eq!(
            rows.last().unwrap().value,
            "2 additional effect types not shown"
        );
    }

    #[test]
    fn events_without_descendants_do_not_render_empty_influence() {
        let world = world_with(vec![event(1, "choice_made", &[])]);
        assert!(influence_rows(&world, EventId::new(1)).is_empty());
    }
}
