use crate::{EntityId, Event, EventId, Relation, RelationId, StateChange, WorldState};
use std::collections::{BTreeMap, BTreeSet};

/// Which recorded events touched each entity and relation, kept up to date
/// as events are recorded so that asking costs the same however long a
/// World has lived.
#[derive(Clone, Debug, Default)]
pub struct HistoryIndex {
    covered: usize,
    endpoints: BTreeMap<RelationId, (EntityId, EntityId)>,
    by_entity: BTreeMap<EntityId, Vec<EventId>>,
    relations: BTreeMap<RelationId, RelationRecord>,
}

/// One life of a relation: how it stands now or stood when it ended, and
/// the events that touched it.
#[derive(Clone, Debug, PartialEq)]
pub struct RelationRecord {
    pub relation: Relation,
    pub active: bool,
    pub event_ids: Vec<EventId>,
}

impl HistoryIndex {
    pub(crate) fn new(baseline: &WorldState) -> Self {
        Self {
            covered: 0,
            endpoints: baseline
                .relations()
                .map(|relation| (relation.id, (relation.from, relation.to)))
                .collect(),
            by_entity: BTreeMap::new(),
            relations: baseline
                .relations()
                .map(|relation| {
                    (
                        relation.id,
                        RelationRecord {
                            relation: relation.clone(),
                            active: true,
                            event_ids: Vec::new(),
                        },
                    )
                })
                .collect(),
        }
    }

    /// How many of the World's events this index has read.
    pub(crate) fn covered(&self) -> usize {
        self.covered
    }

    /// The events that changed an entity since it was last created, oldest
    /// first; a relation's change counts for both its ends.
    pub fn changes_of(&self, entity: EntityId) -> &[EventId] {
        self.by_entity
            .get(&entity)
            .map(Vec::as_slice)
            .unwrap_or_default()
    }

    /// Every relation the World has had, each in its latest life.
    pub fn relations(&self) -> impl Iterator<Item = &RelationRecord> {
        self.relations.values()
    }

    pub(crate) fn read(&mut self, event: &Event) {
        self.covered += 1;
        let mut entities = BTreeSet::new();
        let mut relations = BTreeSet::new();
        for change in &event.changes {
            match change {
                StateChange::CreateEntity(entity) => {
                    self.by_entity.remove(&entity.id);
                    entities.insert(entity.id);
                }
                StateChange::RemoveEntity(entity) => {
                    entities.insert(*entity);
                    for (from, to) in self.endpoints.values().copied() {
                        if from == *entity {
                            entities.insert(to);
                        }
                        if to == *entity {
                            entities.insert(from);
                        }
                    }
                    self.endpoints
                        .retain(|_, (from, to)| *from != *entity && *to != *entity);
                    for (id, record) in &mut self.relations {
                        if record.active
                            && (record.relation.from == *entity || record.relation.to == *entity)
                        {
                            record.active = false;
                            relations.insert(*id);
                        }
                    }
                }
                StateChange::SetComponent { entity, .. }
                | StateChange::RemoveComponent { entity, .. } => {
                    entities.insert(*entity);
                }
                StateChange::CreateRelation(relation) => {
                    entities.insert(relation.from);
                    entities.insert(relation.to);
                    self.endpoints
                        .insert(relation.id, (relation.from, relation.to));
                    self.relations.insert(
                        relation.id,
                        RelationRecord {
                            relation: relation.clone(),
                            active: true,
                            event_ids: Vec::new(),
                        },
                    );
                    relations.insert(relation.id);
                }
                StateChange::RemoveRelation(relation) => {
                    if let Some((from, to)) = self.endpoints.remove(relation) {
                        entities.insert(from);
                        entities.insert(to);
                    }
                    if let Some(record) = self.relations.get_mut(relation) {
                        if record.active {
                            record.active = false;
                            relations.insert(*relation);
                        }
                    }
                }
                StateChange::SetRelationProperty {
                    relation,
                    key,
                    value,
                } => {
                    if let Some((from, to)) = self.endpoints.get(relation).copied() {
                        entities.insert(from);
                        entities.insert(to);
                    }
                    if let Some(record) = self.relations.get_mut(relation) {
                        if record.active {
                            record
                                .relation
                                .properties
                                .insert(key.clone(), value.clone());
                            relations.insert(*relation);
                        }
                    }
                }
                StateChange::RemoveRelationProperty { relation, key } => {
                    if let Some((from, to)) = self.endpoints.get(relation).copied() {
                        entities.insert(from);
                        entities.insert(to);
                    }
                    if let Some(record) = self.relations.get_mut(relation) {
                        if record.active {
                            record.relation.properties.remove(key);
                            relations.insert(*relation);
                        }
                    }
                }
            }
        }
        for entity in entities {
            self.by_entity.entry(entity).or_default().push(event.id);
        }
        for relation in relations {
            if let Some(record) = self.relations.get_mut(&relation) {
                record.event_ids.push(event.id);
            }
        }
    }
}
