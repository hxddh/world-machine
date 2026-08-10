use serde_json::Value;
use std::error::Error;
use std::fmt;

const DECISION_PREFIX: &str = "WORLD_ACTION:";

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PiRpcProtocolError {
    InvalidJson(String),
    ToolExecutionAttempt(String),
    UiRequest,
    FailedResponse(String),
    MissingSuccessfulResponse,
    MissingDecisionText,
    InvalidDecisionFormat,
}

impl fmt::Display for PiRpcProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidJson(message) => write!(f, "invalid Pi RPC JSON: {message}"),
            Self::ToolExecutionAttempt(tool) => {
                write!(f, "Pi attempted forbidden tool execution: {tool}")
            }
            Self::UiRequest => write!(f, "Pi requested extension UI in decision-only mode"),
            Self::FailedResponse(message) => write!(f, "Pi RPC prompt failed: {message}"),
            Self::MissingSuccessfulResponse => {
                write!(
                    f,
                    "Pi RPC stream ended without a successful prompt response"
                )
            }
            Self::MissingDecisionText => write!(f, "Pi RPC produced no assistant decision text"),
            Self::InvalidDecisionFormat => write!(
                f,
                "Pi decision must be exactly one line: WORLD_ACTION:<action-name>"
            ),
        }
    }
}

impl Error for PiRpcProtocolError {}

#[derive(Default)]
pub struct PiRpcEventParser {
    text: String,
    saw_success: bool,
}

impl PiRpcEventParser {
    pub fn push_line(&mut self, line: &str) -> Result<(), PiRpcProtocolError> {
        let value: Value = serde_json::from_str(line)
            .map_err(|error| PiRpcProtocolError::InvalidJson(error.to_string()))?;
        let event_type = value
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or_default();

        match event_type {
            "message_update" => {
                if let Some(update) = value.get("assistantMessageEvent") {
                    if update.get("type").and_then(Value::as_str) == Some("text_delta") {
                        if let Some(delta) = update.get("delta").and_then(Value::as_str) {
                            self.text.push_str(delta);
                        }
                    }
                }
            }
            "text_delta" => {
                if let Some(delta) = value
                    .get("delta")
                    .or_else(|| value.get("data").and_then(|data| data.get("text")))
                    .and_then(Value::as_str)
                {
                    self.text.push_str(delta);
                }
            }
            "tool_execution_start" => {
                let tool = value
                    .get("toolName")
                    .or_else(|| value.get("tool"))
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
                    .to_owned();
                return Err(PiRpcProtocolError::ToolExecutionAttempt(tool));
            }
            "extension_ui_request" => return Err(PiRpcProtocolError::UiRequest),
            "response" => {
                if value.get("command").and_then(Value::as_str) == Some("prompt") {
                    if value.get("success").and_then(Value::as_bool) == Some(true) {
                        self.saw_success = true;
                    } else {
                        let message = value
                            .get("error")
                            .and_then(Value::as_str)
                            .unwrap_or("unknown error")
                            .to_owned();
                        return Err(PiRpcProtocolError::FailedResponse(message));
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    pub fn finish(self) -> Result<String, PiRpcProtocolError> {
        if !self.saw_success {
            return Err(PiRpcProtocolError::MissingSuccessfulResponse);
        }
        if self.text.trim().is_empty() {
            return Err(PiRpcProtocolError::MissingDecisionText);
        }
        Ok(self.text)
    }
}

pub fn parse_decision(output: &str) -> Result<String, PiRpcProtocolError> {
    let trimmed = output.trim();
    if trimmed.contains('\n') || !trimmed.starts_with(DECISION_PREFIX) {
        return Err(PiRpcProtocolError::InvalidDecisionFormat);
    }
    let action = trimmed[DECISION_PREFIX.len()..].trim();
    if action.is_empty()
        || action.chars().any(char::is_whitespace)
        || action.contains(':')
        || action.contains('`')
    {
        return Err(PiRpcProtocolError::InvalidDecisionFormat);
    }
    Ok(action.to_owned())
}
