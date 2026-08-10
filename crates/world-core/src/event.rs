use crate::{Entity, EntityId, EventId, Relation, RelationId, Value};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq)]
pub enum StateChange {
    CreateEntity(Entity),
    RemoveEntity(EntityId),
    SetComponent {
        entity: EntityId,
        key: String,
        value: Value,
    },
    RemoveComponent {
        entity: EntityId,
        key: String,
    },
    CreateRelation(Relation),
    RemoveRelation(RelationId),
    SetRelationProperty {
        relation: RelationId,
        key: String,
        value: Value,
    },
    RemoveRelationProperty {
        relation: RelationId,
        key: String,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Event {
    pub id: EventId,
    pub kind: String,
    pub world_time: u64,
    pub actor: Option<EntityId>,
    pub targets: Vec<EntityId>,
    pub caused_by: Vec<EventId>,
    pub payload: BTreeMap<String, Value>,
    pub changes: Vec<StateChange>,
}
