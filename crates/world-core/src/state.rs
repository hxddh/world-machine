use crate::{Entity, EntityId, Relation, RelationId, StateChange};
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct WorldState {
    world_time: u64,
    entities: BTreeMap<EntityId, Entity>,
    relations: BTreeMap<RelationId, Relation>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WorldStateError {
    EntityAlreadyExists(EntityId),
    EntityNotFound(EntityId),
    RelationAlreadyExists(RelationId),
    RelationNotFound(RelationId),
}

impl fmt::Display for WorldStateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EntityAlreadyExists(id) => write!(f, "entity already exists: {id}"),
            Self::EntityNotFound(id) => write!(f, "entity not found: {id}"),
            Self::RelationAlreadyExists(id) => write!(f, "relation already exists: {id}"),
            Self::RelationNotFound(id) => write!(f, "relation not found: {id}"),
        }
    }
}

impl Error for WorldStateError {}

impl WorldState {
    pub fn world_time(&self) -> u64 {
        self.world_time
    }

    pub(crate) fn set_world_time(&mut self, world_time: u64) {
        self.world_time = world_time;
    }

    pub fn entity(&self, id: EntityId) -> Option<&Entity> {
        self.entities.get(&id)
    }

    pub fn relation(&self, id: RelationId) -> Option<&Relation> {
        self.relations.get(&id)
    }

    pub fn entities(&self) -> impl Iterator<Item = &Entity> {
        self.entities.values()
    }

    pub fn relations(&self) -> impl Iterator<Item = &Relation> {
        self.relations.values()
    }

    pub fn seed_entity(&mut self, entity: Entity) -> Result<(), WorldStateError> {
        if self.entities.contains_key(&entity.id) {
            return Err(WorldStateError::EntityAlreadyExists(entity.id));
        }
        self.entities.insert(entity.id, entity);
        Ok(())
    }

    pub fn seed_relation(&mut self, relation: Relation) -> Result<(), WorldStateError> {
        if self.relations.contains_key(&relation.id) {
            return Err(WorldStateError::RelationAlreadyExists(relation.id));
        }
        if !self.entities.contains_key(&relation.from) {
            return Err(WorldStateError::EntityNotFound(relation.from));
        }
        if !self.entities.contains_key(&relation.to) {
            return Err(WorldStateError::EntityNotFound(relation.to));
        }
        self.relations.insert(relation.id, relation);
        Ok(())
    }

    pub(crate) fn apply_change(&mut self, change: &StateChange) -> Result<(), WorldStateError> {
        match change {
            StateChange::CreateEntity(entity) => {
                if self.entities.contains_key(&entity.id) {
                    return Err(WorldStateError::EntityAlreadyExists(entity.id));
                }
                self.entities.insert(entity.id, entity.clone());
            }
            StateChange::RemoveEntity(id) => {
                if self.entities.remove(id).is_none() {
                    return Err(WorldStateError::EntityNotFound(*id));
                }
                self.relations.retain(|_, relation| relation.from != *id && relation.to != *id);
            }
            StateChange::SetComponent { entity, key, value } => {
                let target = self
                    .entities
                    .get_mut(entity)
                    .ok_or(WorldStateError::EntityNotFound(*entity))?;
                target.components.insert(key.clone(), value.clone());
            }
            StateChange::RemoveComponent { entity, key } => {
                let target = self
                    .entities
                    .get_mut(entity)
                    .ok_or(WorldStateError::EntityNotFound(*entity))?;
                target.components.remove(key);
            }
            StateChange::CreateRelation(relation) => {
                if self.relations.contains_key(&relation.id) {
                    return Err(WorldStateError::RelationAlreadyExists(relation.id));
                }
                if !self.entities.contains_key(&relation.from) {
                    return Err(WorldStateError::EntityNotFound(relation.from));
                }
                if !self.entities.contains_key(&relation.to) {
                    return Err(WorldStateError::EntityNotFound(relation.to));
                }
                self.relations.insert(relation.id, relation.clone());
            }
            StateChange::RemoveRelation(id) => {
                if self.relations.remove(id).is_none() {
                    return Err(WorldStateError::RelationNotFound(*id));
                }
            }
            StateChange::SetRelationProperty { relation, key, value } => {
                let target = self
                    .relations
                    .get_mut(relation)
                    .ok_or(WorldStateError::RelationNotFound(*relation))?;
                target.properties.insert(key.clone(), value.clone());
            }
            StateChange::RemoveRelationProperty { relation, key } => {
                let target = self
                    .relations
                    .get_mut(relation)
                    .ok_or(WorldStateError::RelationNotFound(*relation))?;
                target.properties.remove(key);
            }
        }
        Ok(())
    }
}
