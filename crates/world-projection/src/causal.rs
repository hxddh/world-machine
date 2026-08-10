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

pub fn why_from_world(world: &World, event: EventId) -> Option<WhyProjection> {
    world.event(event)?;

    let mut visited = BTreeSet::new();
    let mut nodes = Vec::new();
    visit(world, event, 0, &mut visited, &mut nodes);

    Some(WhyProjection { event, nodes })
}

pub fn why_map_from_world(world: &World) -> BTreeMap<EventId, WhyProjection> {
    world
        .events()
        .iter()
        .filter_map(|event| why_from_world(world, event.id).map(|why| (event.id, why)))
        .collect()
}

fn visit(
    world: &World,
    event_id: EventId,
    depth: usize,
    visited: &mut BTreeSet<EventId>,
    nodes: &mut Vec<WhyNode>,
) {
    if !visited.insert(event_id) {
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
        visit(world, *cause, depth + 1, visited, nodes);
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
}
