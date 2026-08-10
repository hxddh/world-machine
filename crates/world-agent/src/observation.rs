use std::collections::BTreeMap;
use world_core::{Entity, EntityId, Event, EventId, Relation, Value};

#[derive(Clone, Debug, PartialEq)]
pub struct ObservedEvent {
    pub id: EventId,
    pub kind: String,
    pub world_time: u64,
    pub actor: Option<EntityId>,
    pub targets: Vec<EntityId>,
    pub caused_by: Vec<EventId>,
    pub payload: BTreeMap<String, Value>,
}

impl From<&Event> for ObservedEvent {
    fn from(event: &Event) -> Self {
        Self {
            id: event.id,
            kind: event.kind.clone(),
            world_time: event.world_time,
            actor: event.actor,
            targets: event.targets.clone(),
            caused_by: event.caused_by.clone(),
            payload: event.payload.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AgentObservation {
    pub actor: EntityId,
    pub world_time: u64,
    pub entities: Vec<Entity>,
    pub relations: Vec<Relation>,
    pub events: Vec<ObservedEvent>,
}
