use std::collections::{BTreeMap, BTreeSet};
use world_core::{EventId, World};

use crate::{event_summary, humanize};

#[derive(Clone, Debug, PartialEq)]
pub struct WhyProjection {
    pub event: EventId,
    pub nodes: Vec<WhyNode>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WhyNode {
    pub event: EventId,
    pub depth: usize,
    pub world_time: u64,
    pub title: String,
    pub subtitle: String,
    pub caused_by: Vec<EventId>,
}

/// How many links of a chain a snapshot carries for every event: more than
/// a player is shown, and few enough that a long-lived World's snapshot
/// costs the same as a young one's.
pub const WHY_CHAIN_LIMIT: usize = 12;

pub fn why_from_world(world: &World, event: EventId) -> Option<WhyProjection> {
    why_up_to(world, event, usize::MAX)
}

fn why_up_to(world: &World, event: EventId, limit: usize) -> Option<WhyProjection> {
    world.event(event)?;

    // A cut chain keeps the nearest causes: the first `limit` found going
    // out from the event one step at a time, told in the same order as the
    // whole chain would be.
    let kept = (limit != usize::MAX).then(|| nearest_causes(world, event, limit));
    let mut visited = BTreeSet::new();
    let mut nodes = Vec::new();
    visit(world, event, 0, kept.as_ref(), &mut visited, &mut nodes);

    Some(WhyProjection { event, nodes })
}

fn nearest_causes(world: &World, event: EventId, limit: usize) -> BTreeSet<EventId> {
    let mut kept = BTreeSet::from([event]);
    let mut frontier = std::collections::VecDeque::from([event]);
    while let Some(id) = frontier.pop_front() {
        let Some(event) = world.event(id) else {
            continue;
        };
        for cause in &event.caused_by {
            if kept.len() >= limit {
                return kept;
            }
            if kept.insert(*cause) {
                frontier.push_back(*cause);
            }
        }
    }
    kept
}

/// The chain of causes of each of the World's latest events, each cut at
/// [`WHY_CHAIN_LIMIT`] links.
pub fn why_map_from_world(world: &World) -> BTreeMap<EventId, WhyProjection> {
    crate::recent_events(world)
        .iter()
        .filter_map(|event| why_up_to(world, event.id, WHY_CHAIN_LIMIT).map(|why| (event.id, why)))
        .collect()
}

fn visit(
    world: &World,
    event_id: EventId,
    depth: usize,
    kept: Option<&BTreeSet<EventId>>,
    visited: &mut BTreeSet<EventId>,
    nodes: &mut Vec<WhyNode>,
) {
    if kept.is_some_and(|kept| !kept.contains(&event_id)) || !visited.insert(event_id) {
        return;
    }

    let Some(event) = world.event(event_id) else {
        return;
    };

    nodes.push(WhyNode {
        event: event.id,
        depth,
        world_time: event.world_time,
        title: humanize(&event.kind),
        subtitle: event_summary(event, world),
        caused_by: event.caused_by.clone(),
    });

    for cause in &event.caused_by {
        visit(world, *cause, depth + 1, kept, visited, nodes);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use world_core::{Entity, EntityId, Event, WorldState};

    #[test]
    fn why_projection_walks_persisted_causes() {
        let mut state = WorldState::default();
        state
            .seed_entity(
                Entity::new(EntityId::new(1), "workspace").with_component("name", "Workspace"),
            )
            .unwrap();

        let events = vec![
            Event {
                id: EventId::new(1),
                kind: "root_cause".into(),
                world_time: 0,
                actor: Some(EntityId::new(1)),
                targets: vec![],
                caused_by: vec![],
                payload: BTreeMap::new(),
                changes: vec![],
            },
            Event {
                id: EventId::new(2),
                kind: "intermediate_effect".into(),
                world_time: 0,
                actor: Some(EntityId::new(1)),
                targets: vec![],
                caused_by: vec![EventId::new(1)],
                payload: BTreeMap::new(),
                changes: vec![],
            },
            Event {
                id: EventId::new(3),
                kind: "final_effect".into(),
                world_time: 0,
                actor: Some(EntityId::new(1)),
                targets: vec![],
                caused_by: vec![EventId::new(2)],
                payload: BTreeMap::new(),
                changes: vec![],
            },
        ];
        let world = world_core::World::from_history(state, &events).unwrap();

        let why = why_from_world(&world, EventId::new(3)).unwrap();

        assert_eq!(why.nodes.len(), 3);
        assert_eq!(why.nodes[0].event, EventId::new(3));
        assert_eq!(why.nodes[0].depth, 0);
        assert_eq!(why.nodes[1].event, EventId::new(2));
        assert_eq!(why.nodes[1].depth, 1);
        assert_eq!(why.nodes[2].event, EventId::new(1));
        assert_eq!(why.nodes[2].depth, 2);
    }

    #[test]
    fn a_long_history_carries_short_chains_for_its_latest_events_only() {
        let mut state = WorldState::default();
        state
            .seed_entity(Entity::new(EntityId::new(1), "workspace"))
            .unwrap();
        // Every event caused by the one before it: one long chain.
        let events = (1..=1_000u64)
            .map(|id| Event {
                id: EventId::new(id),
                kind: "step".into(),
                world_time: id,
                actor: Some(EntityId::new(1)),
                targets: vec![],
                caused_by: if id > 1 {
                    vec![EventId::new(id - 1)]
                } else {
                    vec![]
                },
                payload: BTreeMap::new(),
                changes: vec![],
            })
            .collect::<Vec<_>>();
        let world = world_core::World::from_history(state, &events).unwrap();

        let map = why_map_from_world(&world);
        assert_eq!(map.len(), crate::RECENT_EVENTS);
        assert!(map.contains_key(&EventId::new(1_000)));
        assert!(!map.contains_key(&EventId::new(1)));
        assert!(map.values().all(|why| why.nodes.len() <= WHY_CHAIN_LIMIT));
        // Asked about on its own, an event still tells its whole chain.
        assert_eq!(
            why_from_world(&world, EventId::new(1_000))
                .unwrap()
                .nodes
                .len(),
            1_000
        );
    }
}
