use crate::{EntityId, Event, EventId, Relation, RelationId, StateChange, WorldState};
use std::collections::BTreeMap;

/// Which recorded events touched each entity and relation, kept up to date
/// as events are recorded so that asking costs the same however long a
/// World has lived.
#[derive(Clone, Debug, Default)]
pub struct HistoryIndex {
    covered: usize,
    endpoints: BTreeMap<RelationId, (EntityId, EntityId)>,
    by_entity: BTreeMap<EntityId, Vec<EventId>>,
    by_kind: BTreeMap<String, Vec<EventId>>,
    relations: BTreeMap<RelationId, RelationRecord>,
    /// What the event being read touched, kept between events so reading a
    /// long history does not ask for memory for each of them.
    touched: Touched,
}

#[derive(Clone, Debug, Default)]
struct Touched {
    entities: Vec<EntityId>,
    relations: Vec<RelationId>,
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
            by_kind: BTreeMap::new(),
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
            touched: Touched::default(),
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

    /// The events of one kind, oldest first, so a System can find what it
    /// recorded without reading the whole history.
    pub fn of_kind(&self, kind: &str) -> &[EventId] {
        self.by_kind
            .get(kind)
            .map(Vec::as_slice)
            .unwrap_or_default()
    }

    /// Every relation the World has had, each in its latest life.
    pub fn relations(&self) -> impl Iterator<Item = &RelationRecord> {
        self.relations.values()
    }

    pub(crate) fn read(&mut self, event: &Event) {
        self.covered += 1;
        match self.by_kind.get_mut(event.kind.as_str()) {
            Some(events) => events.push(event.id),
            None => {
                self.by_kind.insert(event.kind.clone(), vec![event.id]);
            }
        }
        let Touched {
            mut entities,
            mut relations,
        } = std::mem::take(&mut self.touched);
        entities.clear();
        relations.clear();
        for change in &event.changes {
            match change {
                StateChange::CreateEntity(entity) => {
                    self.by_entity.remove(&entity.id);
                    entities.push(entity.id);
                }
                StateChange::RemoveEntity(entity) => {
                    entities.push(*entity);
                    for (from, to) in self.endpoints.values().copied() {
                        if from == *entity {
                            entities.push(to);
                        }
                        if to == *entity {
                            entities.push(from);
                        }
                    }
                    self.endpoints
                        .retain(|_, (from, to)| *from != *entity && *to != *entity);
                    for (id, record) in &mut self.relations {
                        if record.active
                            && (record.relation.from == *entity || record.relation.to == *entity)
                        {
                            record.active = false;
                            relations.push(*id);
                        }
                    }
                }
                StateChange::SetComponent { entity, .. }
                | StateChange::RemoveComponent { entity, .. } => {
                    entities.push(*entity);
                }
                StateChange::CreateRelation(relation) => {
                    entities.push(relation.from);
                    entities.push(relation.to);
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
                    relations.push(relation.id);
                }
                StateChange::RemoveRelation(relation) => {
                    if let Some((from, to)) = self.endpoints.remove(relation) {
                        entities.push(from);
                        entities.push(to);
                    }
                    if let Some(record) = self.relations.get_mut(relation) {
                        if record.active {
                            record.active = false;
                            relations.push(*relation);
                        }
                    }
                }
                StateChange::SetRelationProperty {
                    relation,
                    key,
                    value,
                } => {
                    if let Some((from, to)) = self.endpoints.get(relation).copied() {
                        entities.push(from);
                        entities.push(to);
                    }
                    if let Some(record) = self.relations.get_mut(relation) {
                        if record.active {
                            record
                                .relation
                                .properties
                                .insert(key.clone(), value.clone());
                            relations.push(*relation);
                        }
                    }
                }
                StateChange::RemoveRelationProperty { relation, key } => {
                    if let Some((from, to)) = self.endpoints.get(relation).copied() {
                        entities.push(from);
                        entities.push(to);
                    }
                    if let Some(record) = self.relations.get_mut(relation) {
                        if record.active {
                            record.relation.properties.remove(key);
                            relations.push(*relation);
                        }
                    }
                }
            }
        }
        // Each once, in order, as a set of them would give them.
        entities.sort_unstable();
        entities.dedup();
        relations.sort_unstable();
        relations.dedup();
        for entity in &entities {
            match self.by_entity.get_mut(entity) {
                Some(events) => events.push(event.id),
                None => {
                    self.by_entity.insert(*entity, vec![event.id]);
                }
            }
        }
        for relation in &relations {
            if let Some(record) = self.relations.get_mut(relation) {
                record.event_ids.push(event.id);
            }
        }
        self.touched = Touched {
            entities,
            relations,
        };
    }
}
