use crate::AgentObservation;
use std::collections::VecDeque;
use std::error::Error;
use std::fmt;
use world_core::ActionRequest;

#[derive(Clone, Debug, PartialEq)]
pub struct AvailableAction {
    pub description: String,
    pub request: ActionRequest,
}

impl AvailableAction {
    pub fn new(description: impl Into<String>, request: ActionRequest) -> Self {
        Self {
            description: description.into(),
            request,
        }
    }

    pub fn name(&self) -> &str {
        &self.request.action
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentDecision {
    pub action: String,
}

impl AgentDecision {
    pub fn choose(action: impl Into<String>) -> Self {
        Self {
            action: action.into(),
        }
    }
}

#[derive(Debug)]
pub struct AgentRuntimeError {
    message: String,
}

impl AgentRuntimeError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for AgentRuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.message.fmt(f)
    }
}

impl Error for AgentRuntimeError {}

pub trait AgentRuntime {
    fn decide(
        &mut self,
        observation: &AgentObservation,
        actions: &[AvailableAction],
    ) -> Result<AgentDecision, AgentRuntimeError>;
}

#[derive(Clone, Debug, Default)]
pub struct MockAgentRuntime {
    decisions: VecDeque<String>,
    call_count: usize,
}

impl MockAgentRuntime {
    pub fn scripted<I, S>(decisions: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            decisions: decisions.into_iter().map(Into::into).collect(),
            call_count: 0,
        }
    }

    pub fn call_count(&self) -> usize {
        self.call_count
    }
}

impl AgentRuntime for MockAgentRuntime {
    fn decide(
        &mut self,
        _observation: &AgentObservation,
        _actions: &[AvailableAction],
    ) -> Result<AgentDecision, AgentRuntimeError> {
        self.call_count += 1;
        let action = self
            .decisions
            .pop_front()
            .ok_or_else(|| AgentRuntimeError::new("agent runtime has no scripted decision"))?;
        Ok(AgentDecision::choose(action))
    }
}
