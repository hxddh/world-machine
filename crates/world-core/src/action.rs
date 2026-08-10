use crate::{EntityId, EventId, StateChange, Value, WorldState};
use std::collections::{BTreeMap, HashMap};
use std::error::Error;
use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub struct ActionRequest {
    pub actor: Option<EntityId>,
    pub action: String,
    pub args: BTreeMap<String, Value>,
    pub caused_by: Vec<EventId>,
}

impl ActionRequest {
    pub fn new(action: impl Into<String>) -> Self {
        Self {
            actor: None,
            action: action.into(),
            args: BTreeMap::new(),
            caused_by: Vec::new(),
        }
    }

    pub fn actor(mut self, actor: EntityId) -> Self {
        self.actor = Some(actor);
        self
    }

    pub fn arg(mut self, key: impl Into<String>, value: impl Into<Value>) -> Self {
        self.args.insert(key.into(), value.into());
        self
    }

    pub fn caused_by(mut self, event: EventId) -> Self {
        self.caused_by.push(event);
        self
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct EventDraft {
    pub kind: String,
    pub actor: Option<EntityId>,
    pub targets: Vec<EntityId>,
    pub caused_by: Vec<EventId>,
    pub payload: BTreeMap<String, Value>,
    pub changes: Vec<StateChange>,
}

impl EventDraft {
    pub fn new(kind: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            actor: None,
            targets: Vec::new(),
            caused_by: Vec::new(),
            payload: BTreeMap::new(),
            changes: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ActionError {
    UnknownAction(String),
    Invalid(String),
}

impl fmt::Display for ActionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownAction(name) => write!(f, "unknown action: {name}"),
            Self::Invalid(message) => message.fmt(f),
        }
    }
}

impl Error for ActionError {}

pub trait Action: Send + Sync {
    fn name(&self) -> &'static str;

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError>;
}

#[derive(Default)]
pub struct ActionRegistry {
    actions: HashMap<String, Box<dyn Action>>,
}

impl ActionRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register<A>(&mut self, action: A) -> Result<(), ActionError>
    where
        A: Action + 'static,
    {
        let name = action.name().to_owned();
        if self.actions.contains_key(&name) {
            return Err(ActionError::Invalid(format!(
                "action already registered: {name}"
            )));
        }
        self.actions.insert(name, Box::new(action));
        Ok(())
    }

    pub fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let action = self
            .actions
            .get(&request.action)
            .ok_or_else(|| ActionError::UnknownAction(request.action.clone()))?;
        action.evaluate(state, request)
    }
}
