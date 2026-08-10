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
        }
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
        let events = world
            .events()
            .iter()
            .filter(|event| {
                event.actor.is_some_and(|id| visible.contains(&id))
                    || event.targets.iter().any(|id| visible.contains(id))
            })
            .map(ObservedEvent::from)
            .collect();

        Ok(AgentObservation {
            actor,
            world_time: world.world_time(),
            entities,
            relations,
            events,
        })
    }
}
