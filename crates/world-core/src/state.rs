use crate::{Entity, EntityId, Relation, RelationId, StateChange, Value};
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use std::sync::Arc;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct WorldState {
    world_time: u64,
    /// Each entity shared between copies of the state until one of them
    /// changes it, so a copy (to try something on, or to go back to) costs
    /// what the entities it changes cost, not the whole World.
    entities: BTreeMap<EntityId, Arc<Entity>>,
    relations: BTreeMap<RelationId, Relation>,
}

/// How to put back one change [`WorldState::apply_all`] made.
enum Undo {
    Entity(EntityId, Option<Arc<Entity>>),
    Relation(RelationId, Option<Relation>),
    Component(EntityId, String, Option<Value>),
    Property(RelationId, String, Option<Value>),
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
        self.entities.get(&id).map(Arc::as_ref)
    }

    pub fn relation(&self, id: RelationId) -> Option<&Relation> {
        self.relations.get(&id)
    }

    pub fn entities(&self) -> impl Iterator<Item = &Entity> {
        self.entities.values().map(Arc::as_ref)
    }

    pub fn relations(&self) -> impl Iterator<Item = &Relation> {
        self.relations.values()
    }

    pub fn seed_entity(&mut self, entity: Entity) -> Result<(), WorldStateError> {
        if self.entities.contains_key(&entity.id) {
            return Err(WorldStateError::EntityAlreadyExists(entity.id));
        }
        self.entities.insert(entity.id, Arc::new(entity));
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

    /// Applies every change or none: when one does not apply, those before
    /// it are undone, so the state is as it was. Only what the changes
    /// touch is kept aside to undo, never the whole state.
    pub(crate) fn apply_all(&mut self, changes: &[StateChange]) -> Result<(), WorldStateError> {
        let mut undo = Vec::new();
        for change in changes {
            if let Err(error) = self.apply_undoable(change, &mut undo) {
                while let Some(step) = undo.pop() {
                    self.undo(step);
                }
                return Err(error);
            }
        }
        Ok(())
    }

    fn apply_undoable(
        &mut self,
        change: &StateChange,
        undo: &mut Vec<Undo>,
    ) -> Result<(), WorldStateError> {
        match change {
            StateChange::RemoveEntity(id) => {
                let entity = self
                    .entities
                    .remove(id)
                    .ok_or(WorldStateError::EntityNotFound(*id))?;
                let (gone, kept) = std::mem::take(&mut self.relations)
                    .into_iter()
                    .partition(|(_, relation)| relation.from == *id || relation.to == *id);
                self.relations = kept;
                undo.push(Undo::Entity(entity.id, Some(entity)));
                for (_, relation) in gone {
                    undo.push(Undo::Relation(relation.id, Some(relation)));
                }
            }
            StateChange::SetComponent { entity, key, value } => {
                let target = self
                    .entities
                    .get_mut(entity)
                    .ok_or(WorldStateError::EntityNotFound(*entity))?;
                let before = Arc::make_mut(target)
                    .components
                    .insert(key.clone(), value.clone());
                undo.push(Undo::Component(*entity, key.clone(), before));
            }
            StateChange::RemoveComponent { entity, key } => {
                let target = self
                    .entities
                    .get_mut(entity)
                    .ok_or(WorldStateError::EntityNotFound(*entity))?;
                let before = Arc::make_mut(target).components.remove(key);
                undo.push(Undo::Component(*entity, key.clone(), before));
            }
            StateChange::RemoveRelation(id) => {
                let relation = self
                    .relations
                    .remove(id)
                    .ok_or(WorldStateError::RelationNotFound(*id))?;
                undo.push(Undo::Relation(*id, Some(relation)));
            }
            StateChange::SetRelationProperty {
                relation,
                key,
                value,
            } => {
                let target = self
                    .relations
                    .get_mut(relation)
                    .ok_or(WorldStateError::RelationNotFound(*relation))?;
                let before = target.properties.insert(key.clone(), value.clone());
                undo.push(Undo::Property(*relation, key.clone(), before));
            }
            StateChange::RemoveRelationProperty { relation, key } => {
                let target = self
                    .relations
                    .get_mut(relation)
                    .ok_or(WorldStateError::RelationNotFound(*relation))?;
                let before = target.properties.remove(key);
                undo.push(Undo::Property(*relation, key.clone(), before));
            }
            StateChange::CreateEntity(entity) => {
                self.apply_change(change)?;
                undo.push(Undo::Entity(entity.id, None));
            }
            StateChange::CreateRelation(relation) => {
                self.apply_change(change)?;
                undo.push(Undo::Relation(relation.id, None));
            }
        }
        Ok(())
    }

    fn undo(&mut self, step: Undo) {
        match step {
            Undo::Entity(id, before) => match before {
                Some(entity) => {
                    self.entities.insert(id, entity);
                }
                None => {
                    self.entities.remove(&id);
                }
            },
            Undo::Relation(id, before) => match before {
                Some(relation) => {
                    self.relations.insert(id, relation);
                }
                None => {
                    self.relations.remove(&id);
                }
            },
            Undo::Component(id, key, before) => {
                if let Some(entity) = self.entities.get_mut(&id) {
                    let entity = Arc::make_mut(entity);
                    match before {
                        Some(value) => entity.components.insert(key, value),
                        None => entity.components.remove(&key),
                    };
                }
            }
            Undo::Property(id, key, before) => {
                if let Some(relation) = self.relations.get_mut(&id) {
                    match before {
                        Some(value) => relation.properties.insert(key, value),
                        None => relation.properties.remove(&key),
                    };
                }
            }
        }
    }

    pub(crate) fn apply_change(&mut self, change: &StateChange) -> Result<(), WorldStateError> {
        match change {
            StateChange::CreateEntity(entity) => {
                if self.entities.contains_key(&entity.id) {
                    return Err(WorldStateError::EntityAlreadyExists(entity.id));
                }
                self.entities.insert(entity.id, Arc::new(entity.clone()));
            }
            StateChange::RemoveEntity(id) => {
                if self.entities.remove(id).is_none() {
                    return Err(WorldStateError::EntityNotFound(*id));
                }
                self.relations
                    .retain(|_, relation| relation.from != *id && relation.to != *id);
            }
            StateChange::SetComponent { entity, key, value } => {
                let target = self
                    .entities
                    .get_mut(entity)
                    .ok_or(WorldStateError::EntityNotFound(*entity))?;
                Arc::make_mut(target)
                    .components
                    .insert(key.clone(), value.clone());
            }
            StateChange::RemoveComponent { entity, key } => {
                let target = self
                    .entities
                    .get_mut(entity)
                    .ok_or(WorldStateError::EntityNotFound(*entity))?;
                Arc::make_mut(target).components.remove(key);
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
            StateChange::SetRelationProperty {
                relation,
                key,
                value,
            } => {
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
