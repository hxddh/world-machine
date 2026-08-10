use crate::{EntityId, Value};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq)]
pub struct Entity {
    pub id: EntityId,
    pub kind: String,
    pub components: BTreeMap<String, Value>,
}

impl Entity {
    pub fn new(id: EntityId, kind: impl Into<String>) -> Self {
        Self {
            id,
            kind: kind.into(),
            components: BTreeMap::new(),
        }
    }

    pub fn with_component(mut self, key: impl Into<String>, value: impl Into<Value>) -> Self {
        self.components.insert(key.into(), value.into());
        self
    }

    pub fn component(&self, key: &str) -> Option<&Value> {
        self.components.get(key)
    }
}
