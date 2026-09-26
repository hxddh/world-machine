use crate::{AgentObservation, ObservedEvent};
use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;
use world_core::{EntityId, World};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PerceptionError {
    ActorNotFound(EntityId),
}

impl fmt::Display for PerceptionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ActorNotFound(actor) => write!(f, "agent actor does not exist: {actor}"),
        }
    }
}

impl Error for PerceptionError {}

pub trait PerceptionPolicy {
    fn observe(&self, world: &World, actor: EntityId) -> Result<AgentObservation, PerceptionError>;
}

#[derive(Clone, Debug, Default)]
pub struct ScopedPerception {
    visible_entities: BTreeSet<EntityId>,
    recent_events: Option<usize>,
}

impl ScopedPerception {
    pub fn self_only() -> Self {
        Self::default()
    }

    pub fn new<I>(visible_entities: I) -> Self
    where
        I: IntoIterator<Item = EntityId>,
    {
        Self {
            visible_entities: visible_entities.into_iter().collect(),
            recent_events: None,
        }
    }

    /// Sees only the latest `count` of the events it would otherwise see,
    /// so what an actor is shown stays the same size however long the
    /// World has lived.
    pub fn with_recent_events(mut self, count: usize) -> Self {
        self.recent_events = Some(count);
        self
    }
}

impl PerceptionPolicy for ScopedPerception {
    fn observe(&self, world: &World, actor: EntityId) -> Result<AgentObservation, PerceptionError> {
        if world.state().entity(actor).is_none() {
            return Err(PerceptionError::ActorNotFound(actor));
        }

        let mut visible = self.visible_entities.clone();
        visible.insert(actor);

        let entities = world
            .state()
            .entities()
            .filter(|entity| visible.contains(&entity.id))
            .cloned()
            .collect();
        let relations = world
            .state()
            .relations()
            .filter(|relation| visible.contains(&relation.from) && visible.contains(&relation.to))
            .cloned()
            .collect();
        let seen = |event: &&world_core::Event| {
            event.actor.is_some_and(|id| visible.contains(&id))
                || event.targets.iter().any(|id| visible.contains(id))
        };
        let events = match self.recent_events {
            Some(count) => {
                let mut latest = world
                    .events()
                    .iter()
                    .rev()
                    .filter(seen)
                    .take(count)
                    .map(ObservedEvent::from)
                    .collect::<Vec<_>>();
                latest.reverse();
                latest
            }
            None => world
                .events()
                .iter()
                .filter(seen)
                .map(ObservedEvent::from)
                .collect(),
        };

        Ok(AgentObservation {
            actor,
            world_time: world.world_time(),
            entities,
            relations,
            events,
        })
    }
}
