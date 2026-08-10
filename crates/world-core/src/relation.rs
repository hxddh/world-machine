use crate::{EntityId, RelationId, Value};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq)]
pub struct Relation {
    pub id: RelationId,
    pub kind: String,
    pub from: EntityId,
    pub to: EntityId,
    pub properties: BTreeMap<String, Value>,
}

impl Relation {
    pub fn new(id: RelationId, kind: impl Into<String>, from: EntityId, to: EntityId) -> Self {
        Self {
            id,
            kind: kind.into(),
            from,
            to,
            properties: BTreeMap::new(),
        }
    }

    pub fn with_property(mut self, key: impl Into<String>, value: impl Into<Value>) -> Self {
        self.properties.insert(key.into(), value.into());
        self
    }
}
