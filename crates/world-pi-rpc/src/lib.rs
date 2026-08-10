mod prompt;
mod protocol;
mod transport;

use prompt::DecisionPrompt;
use std::error::Error;
use std::fmt;
use world_agent::{
    AgentDecision, AgentObservation, AgentRuntime, AgentRuntimeError, AvailableAction,
};

pub use protocol::{parse_decision, PiRpcEventParser, PiRpcProtocolError};
pub use transport::{PiCommand, PiRpcTransport, PiRpcTransportError, ProcessPiRpcTransport};

pub struct PiRpcRuntime<T> {
    transport: T,
}

impl<T> PiRpcRuntime<T> {
    pub fn new(transport: T) -> Self {
        Self { transport }
    }

    pub fn transport(&self) -> &T {
        &self.transport
    }

    pub fn transport_mut(&mut self) -> &mut T {
        &mut self.transport
    }
}

#[derive(Debug)]
enum PiRuntimeDecisionError {
    Transport(PiRpcTransportError),
    Protocol(PiRpcProtocolError),
    UnavailableAction(String),
}

impl fmt::Display for PiRuntimeDecisionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Transport(error) => error.fmt(f),
            Self::Protocol(error) => error.fmt(f),
            Self::UnavailableAction(action) => {
                write!(f, "Pi selected an action that was not offered: {action}")
            }
        }
    }
}

impl Error for PiRuntimeDecisionError {}

impl<T> AgentRuntime for PiRpcRuntime<T>
where
    T: PiRpcTransport,
{
    fn decide(
        &mut self,
        observation: &AgentObservation,
        actions: &[AvailableAction],
    ) -> Result<AgentDecision, AgentRuntimeError> {
        if actions.is_empty() {
            return Err(AgentRuntimeError::new(
                "Pi cannot decide without available actions",
            ));
        }

        let prompt = DecisionPrompt::new(observation, actions).render();
        let response = self
            .transport
            .complete(&prompt)
            .map_err(PiRuntimeDecisionError::Transport)
            .map_err(|error| AgentRuntimeError::new(error.to_string()))?;
        let action = parse_decision(&response)
            .map_err(PiRuntimeDecisionError::Protocol)
            .map_err(|error| AgentRuntimeError::new(error.to_string()))?;

        if !actions.iter().any(|candidate| candidate.name() == action) {
            return Err(AgentRuntimeError::new(
                PiRuntimeDecisionError::UnavailableAction(action).to_string(),
            ));
        }

        Ok(AgentDecision::choose(action))
    }
}

#[cfg(test)]
mod tests;
