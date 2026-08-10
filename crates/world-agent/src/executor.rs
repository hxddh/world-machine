use crate::{
    AgentDecision, AgentRuntime, AgentRuntimeError, AvailableAction, PerceptionError,
    PerceptionPolicy,
};
use std::error::Error;
use std::fmt;
use world_core::{
    Action, ActionError, ActionRegistry, ActionRequest, EntityId, EventDraft, EventId, Value, World,
    WorldError, WorldState,
};

const RECORD_DECISION_ACTION: &str = "agent.record_decision";

#[derive(Clone, Debug, PartialEq)]
pub struct AgentExecution {
    pub decision: AgentDecision,
    pub decision_event: EventId,
    pub outcome_event: EventId,
}

#[derive(Debug)]
pub enum AgentExecutionError {
    Perception(PerceptionError),
    Runtime(AgentRuntimeError),
    UnavailableAction(String),
    World(WorldError),
}

impl fmt::Display for AgentExecutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Perception(error) => error.fmt(f),
            Self::Runtime(error) => error.fmt(f),
            Self::UnavailableAction(action) => {
                write!(f, "agent selected unavailable action: {action}")
            }
            Self::World(error) => error.fmt(f),
        }
    }
}

impl Error for AgentExecutionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Perception(error) => Some(error),
            Self::Runtime(error) => Some(error),
            Self::World(error) => Some(error),
            Self::UnavailableAction(_) => None,
        }
    }
}

impl From<PerceptionError> for AgentExecutionError {
    fn from(value: PerceptionError) -> Self {
        Self::Perception(value)
    }
}

impl From<AgentRuntimeError> for AgentExecutionError {
    fn from(value: AgentRuntimeError) -> Self {
        Self::Runtime(value)
    }
}

impl From<WorldError> for AgentExecutionError {
    fn from(value: WorldError) -> Self {
        Self::World(value)
    }
}

pub struct AgentExecutor;

impl AgentExecutor {
    pub fn decide_and_execute<R, P>(
        runtime: &mut R,
        perception: &P,
        world: &mut World,
        registry: &ActionRegistry,
        actor: EntityId,
        actions: &[AvailableAction],
        caused_by: &[EventId],
    ) -> Result<AgentExecution, AgentExecutionError>
    where
        R: AgentRuntime,
        P: PerceptionPolicy,
    {
        let observation = perception.observe(world, actor)?;
        let decision = runtime.decide(&observation, actions)?;
        let selected = actions
            .iter()
            .find(|action| action.name() == decision.action)
            .ok_or_else(|| AgentExecutionError::UnavailableAction(decision.action.clone()))?;

        let offered = Value::List(
            actions
                .iter()
                .map(|action| Value::Text(action.name().to_owned()))
                .collect(),
        );
        let visible_entities = Value::List(
            observation
                .entities
                .iter()
                .map(|entity| Value::Entity(entity.id))
                .collect(),
        );
        let mut record = ActionRequest::new(RECORD_DECISION_ACTION)
            .actor(actor)
            .arg("selected_action", decision.action.clone())
            .arg("offered_actions", offered)
            .arg("visible_entities", visible_entities);
        for cause in caused_by {
            record.caused_by.push(*cause);
        }
        let decision_event = world.execute(registry, &record)?.id;

        let mut request = selected.request.clone();
        request.actor = Some(actor);
        for cause in caused_by {
            if !request.caused_by.contains(cause) {
                request.caused_by.push(*cause);
            }
        }
        if !request.caused_by.contains(&decision_event) {
            request.caused_by.push(decision_event);
        }
        let outcome_event = world.execute(registry, &request)?.id;

        Ok(AgentExecution {
            decision,
            decision_event,
            outcome_event,
        })
    }
}

pub fn register_actions(registry: &mut ActionRegistry) -> Result<(), ActionError> {
    registry.register(RecordAgentDecision)
}

struct RecordAgentDecision;

impl Action for RecordAgentDecision {
    fn name(&self) -> &'static str {
        RECORD_DECISION_ACTION
    }

    fn evaluate(
        &self,
        _state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let selected = match request.args.get("selected_action") {
            Some(Value::Text(selected)) => selected.clone(),
            _ => return Err(ActionError::Invalid("missing selected_action".into())),
        };
        let offered = request
            .args
            .get("offered_actions")
            .cloned()
            .ok_or_else(|| ActionError::Invalid("missing offered_actions".into()))?;
        let visible_entities = request
            .args
            .get("visible_entities")
            .cloned()
            .ok_or_else(|| ActionError::Invalid("missing visible_entities".into()))?;

        let mut draft = EventDraft::new("agent_decision_recorded");
        draft
            .payload
            .insert("selected_action".into(), selected.into());
        draft.payload.insert("offered_actions".into(), offered);
        draft
            .payload
            .insert("visible_entities".into(), visible_entities);
        Ok(draft)
    }
}
