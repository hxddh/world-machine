use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::error::Error;
use std::fmt;
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, ChildStdout, Command, ExitStatus, Stdio};

pub const ANALYST_TURN_PROTOCOL: &str = "world-machine-analyst-turns";
pub const ANALYST_TURN_PROTOCOL_VERSION: u64 = 1;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "kebab-case")]
pub enum AnalystTurnRequest {
    Ask {
        id: String,
        prompt: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        timeout_ms: Option<u64>,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnalystTurn {
    pub request_id: String,
    pub text: Option<String>,
    pub tool_calls: Vec<AnalystToolCall>,
    pub runtime_errors: Vec<AnalystRuntimeError>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnalystToolCall {
    pub call_id: String,
    pub tool: String,
    pub input: Value,
    pub output: Value,
    pub is_error: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AnalystRuntimeErrorKind {
    Extension,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnalystRuntimeError {
    pub kind: AnalystRuntimeErrorKind,
    pub source: Option<String>,
    pub message: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AnalystRemoteErrorKind {
    Command,
    Protocol,
    Transport,
    Internal,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnalystRemoteError {
    pub kind: AnalystRemoteErrorKind,
    pub fatal: bool,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum AnalystTurnResponse {
    Result {
        id: String,
        turn: AnalystTurn,
    },
    Error {
        id: String,
        error: AnalystRemoteError,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AnalystTurnEnvelope {
    pub protocol: String,
    pub version: u64,
    #[serde(flatten)]
    pub response: AnalystTurnResponse,
}

#[derive(Debug)]
pub enum AnalystTurnClientError {
    SerializeRequest(serde_json::Error),
    WriteRequest(io::Error),
    ReadResponse(io::Error),
    UnexpectedEof,
    InvalidResponseJson(serde_json::Error),
    InvalidResponseShape(String),
    ProtocolMismatch { actual: String },
    VersionMismatch { actual: u64 },
    CorrelationMismatch { expected: String, actual: String },
    RemoteCommand(AnalystRemoteError),
    RemoteFatal(AnalystRemoteError),
    Poisoned,
}

impl AnalystTurnClientError {
    pub fn is_session_fatal(&self) -> bool {
        !matches!(self, Self::SerializeRequest(_) | Self::RemoteCommand(_))
    }
}

impl fmt::Display for AnalystTurnClientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SerializeRequest(error) => {
                write!(f, "failed to serialize analyst turn request: {error}")
            }
            Self::WriteRequest(error) => write!(f, "failed to write analyst turn request: {error}"),
            Self::ReadResponse(error) => write!(f, "failed to read analyst turn response: {error}"),
            Self::UnexpectedEof => write!(f, "analyst turn host closed before responding"),
            Self::InvalidResponseJson(error) => {
                write!(f, "invalid analyst turn response JSON: {error}")
            }
            Self::InvalidResponseShape(message) => {
                write!(f, "invalid analyst turn response shape: {message}")
            }
            Self::ProtocolMismatch { actual } => write!(
                f,
                "unexpected analyst turn protocol `{actual}`; expected `{ANALYST_TURN_PROTOCOL}`"
            ),
            Self::VersionMismatch { actual } => write!(
                f,
                "unexpected analyst turn protocol version {actual}; expected {ANALYST_TURN_PROTOCOL_VERSION}"
            ),
            Self::CorrelationMismatch { expected, actual } => write!(
                f,
                "analyst turn response correlation mismatch: expected id={expected}, got id={actual}"
            ),
            Self::RemoteCommand(error) => {
                write!(f, "analyst turn command rejected: {}", error.message)
            }
            Self::RemoteFatal(error) => write!(
                f,
                "analyst turn host failed ({:?}): {}",
                error.kind, error.message
            ),
            Self::Poisoned => write!(f, "analyst turn client session is poisoned"),
        }
    }
}

impl Error for AnalystTurnClientError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::SerializeRequest(error) | Self::InvalidResponseJson(error) => Some(error),
            Self::WriteRequest(error) | Self::ReadResponse(error) => Some(error),
            Self::UnexpectedEof
            | Self::InvalidResponseShape(_)
            | Self::ProtocolMismatch { .. }
            | Self::VersionMismatch { .. }
            | Self::CorrelationMismatch { .. }
            | Self::RemoteCommand(_)
            | Self::RemoteFatal(_)
            | Self::Poisoned => None,
        }
    }
}

pub struct AnalystTurnClient<R, W> {
    reader: R,
    writer: W,
    next_request_id: u64,
    poisoned: bool,
}

impl<R, W> AnalystTurnClient<R, W> {
    pub fn new(reader: R, writer: W) -> Self {
        Self {
            reader,
            writer,
            next_request_id: 1,
            poisoned: false,
        }
    }

    pub fn is_poisoned(&self) -> bool {
        self.poisoned
    }

    pub fn into_parts(self) -> (R, W) {
        (self.reader, self.writer)
    }
}

impl<R, W> AnalystTurnClient<R, W>
where
    R: BufRead,
    W: Write,
{
    pub fn ask(
        &mut self,
        prompt: impl Into<String>,
        timeout_ms: Option<u64>,
    ) -> Result<AnalystTurn, AnalystTurnClientError> {
        if self.poisoned {
            return Err(AnalystTurnClientError::Poisoned);
        }

        let id = format!("world-rust-analyst-{}", self.next_request_id);
        self.next_request_id += 1;
        let request = AnalystTurnRequest::Ask {
            id: id.clone(),
            prompt: prompt.into(),
            timeout_ms,
        };

        let encoded =
            serde_json::to_vec(&request).map_err(AnalystTurnClientError::SerializeRequest)?;
        if let Err(error) = self.writer.write_all(&encoded) {
            self.poisoned = true;
            return Err(AnalystTurnClientError::WriteRequest(error));
        }
        if let Err(error) = self.writer.write_all(b"\n") {
            self.poisoned = true;
            return Err(AnalystTurnClientError::WriteRequest(error));
        }
        if let Err(error) = self.writer.flush() {
            self.poisoned = true;
            return Err(AnalystTurnClientError::WriteRequest(error));
        }

        let mut line = String::new();
        let read = match self.reader.read_line(&mut line) {
            Ok(read) => read,
            Err(error) => {
                self.poisoned = true;
                return Err(AnalystTurnClientError::ReadResponse(error));
            }
        };
        if read == 0 {
            self.poisoned = true;
            return Err(AnalystTurnClientError::UnexpectedEof);
        }

        let value: Value = match serde_json::from_str(&line) {
            Ok(value) => value,
            Err(error) => {
                self.poisoned = true;
                return Err(AnalystTurnClientError::InvalidResponseJson(error));
            }
        };
        if let Err(error) = validate_response_shape(&value) {
            self.poisoned = true;
            return Err(error);
        }
        let envelope: AnalystTurnEnvelope = match serde_json::from_value(value) {
            Ok(envelope) => envelope,
            Err(error) => {
                self.poisoned = true;
                return Err(AnalystTurnClientError::InvalidResponseJson(error));
            }
        };

        if envelope.protocol != ANALYST_TURN_PROTOCOL {
            self.poisoned = true;
            return Err(AnalystTurnClientError::ProtocolMismatch {
                actual: envelope.protocol,
            });
        }
        if envelope.version != ANALYST_TURN_PROTOCOL_VERSION {
            self.poisoned = true;
            return Err(AnalystTurnClientError::VersionMismatch {
                actual: envelope.version,
            });
        }

        match envelope.response {
            AnalystTurnResponse::Result { id: actual, turn } => {
                if actual != id {
                    self.poisoned = true;
                    return Err(AnalystTurnClientError::CorrelationMismatch {
                        expected: id,
                        actual,
                    });
                }
                Ok(turn)
            }
            AnalystTurnResponse::Error { id: actual, error } => {
                if actual != id {
                    self.poisoned = true;
                    return Err(AnalystTurnClientError::CorrelationMismatch {
                        expected: id,
                        actual,
                    });
                }
                match (error.kind, error.fatal) {
                    (AnalystRemoteErrorKind::Command, false) => {
                        Err(AnalystTurnClientError::RemoteCommand(error))
                    }
                    (AnalystRemoteErrorKind::Protocol, true)
                    | (AnalystRemoteErrorKind::Transport, true)
                    | (AnalystRemoteErrorKind::Internal, true)
                    | (AnalystRemoteErrorKind::Command, true) => {
                        self.poisoned = true;
                        Err(AnalystTurnClientError::RemoteFatal(error))
                    }
                    (_, false) => {
                        self.poisoned = true;
                        Err(AnalystTurnClientError::InvalidResponseShape(format!(
                            "non-command remote error {:?} cannot be non-fatal",
                            error.kind
                        )))
                    }
                }
            }
        }
    }
}

fn validate_response_shape(value: &Value) -> Result<(), AnalystTurnClientError> {
    let object = value.as_object().ok_or_else(|| {
        AnalystTurnClientError::InvalidResponseShape("response must be a JSON object".into())
    })?;
    let response_type = object.get("type").and_then(Value::as_str).ok_or_else(|| {
        AnalystTurnClientError::InvalidResponseShape("response requires string `type`".into())
    })?;
    let allowed = match response_type {
        "result" => ["protocol", "version", "type", "id", "turn"].as_slice(),
        "error" => ["protocol", "version", "type", "id", "error"].as_slice(),
        other => {
            return Err(AnalystTurnClientError::InvalidResponseShape(format!(
                "unsupported response type `{other}`"
            )))
        }
    };
    if let Some(field) = object
        .keys()
        .find(|field| !allowed.contains(&field.as_str()))
    {
        return Err(AnalystTurnClientError::InvalidResponseShape(format!(
            "unknown top-level field `{field}`"
        )));
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AnalystTurnProcessConfig {
    pub node_program: PathBuf,
    pub turn_host_script: PathBuf,
    pub left_archive: PathBuf,
    pub right_archive: PathBuf,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub thinking: Option<String>,
    pub pi_program: Option<PathBuf>,
    pub analyst_program: Option<PathBuf>,
}

impl AnalystTurnProcessConfig {
    pub fn new(
        turn_host_script: impl Into<PathBuf>,
        left_archive: impl Into<PathBuf>,
        right_archive: impl Into<PathBuf>,
    ) -> Self {
        Self {
            node_program: PathBuf::from("node"),
            turn_host_script: turn_host_script.into(),
            left_archive: left_archive.into(),
            right_archive: right_archive.into(),
            provider: None,
            model: None,
            thinking: None,
            pi_program: None,
            analyst_program: None,
        }
    }
}

pub struct AnalystTurnProcess {
    child: Child,
    client: Option<AnalystTurnClient<BufReader<ChildStdout>, BufWriter<ChildStdin>>>,
}

impl AnalystTurnProcess {
    pub fn spawn(config: &AnalystTurnProcessConfig) -> io::Result<Self> {
        let mut command = Command::new(&config.node_program);
        command
            .arg(&config.turn_host_script)
            .arg(&config.left_archive)
            .arg(&config.right_archive)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());
        if let Some(provider) = &config.provider {
            command.arg("--provider").arg(provider);
        }
        if let Some(model) = &config.model {
            command.arg("--model").arg(model);
        }
        if let Some(thinking) = &config.thinking {
            command.arg("--thinking").arg(thinking);
        }
        if let Some(pi_program) = &config.pi_program {
            command.env("PI_PROGRAM", pi_program);
        }
        if let Some(analyst_program) = &config.analyst_program {
            command.env("WORLD_MACHINE_ANALYST_PROGRAM", analyst_program);
        }

        let mut child = command.spawn()?;
        let Some(stdin) = child.stdin.take() else {
            terminate_child(&mut child);
            return Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "analyst turn host did not expose stdin",
            ));
        };
        let Some(stdout) = child.stdout.take() else {
            drop(stdin);
            terminate_child(&mut child);
            return Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "analyst turn host did not expose stdout",
            ));
        };

        Ok(Self {
            child,
            client: Some(AnalystTurnClient::new(
                BufReader::new(stdout),
                BufWriter::new(stdin),
            )),
        })
    }

    pub fn id(&self) -> u32 {
        self.child.id()
    }

    pub fn ask(
        &mut self,
        prompt: impl Into<String>,
        timeout_ms: Option<u64>,
    ) -> Result<AnalystTurn, AnalystTurnClientError> {
        let result = self
            .client
            .as_mut()
            .expect("analyst client remains available until shutdown")
            .ask(prompt, timeout_ms);
        let poisoned = self
            .client
            .as_ref()
            .is_some_and(AnalystTurnClient::is_poisoned);
        if poisoned {
            self.client.take();
            terminate_child(&mut self.child);
        }
        result
    }

    pub fn shutdown(mut self) -> io::Result<ExitStatus> {
        self.client.take();
        self.child.wait()
    }
}

impl Drop for AnalystTurnProcess {
    fn drop(&mut self) {
        self.client.take();
        match self.child.try_wait() {
            Ok(Some(_)) => {}
            Ok(None) | Err(_) => terminate_child(&mut self.child),
        }
    }
}

fn terminate_child(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn response_client(value: Value) -> AnalystTurnClient<Cursor<Vec<u8>>, Vec<u8>> {
        let mut bytes = serde_json::to_vec(&value).unwrap();
        bytes.push(b'\n');
        AnalystTurnClient::new(Cursor::new(bytes), Vec::new())
    }

    fn success_response(id: &str) -> Value {
        serde_json::json!({
            "protocol": ANALYST_TURN_PROTOCOL,
            "version": ANALYST_TURN_PROTOCOL_VERSION,
            "type": "result",
            "id": id,
            "turn": {
                "request_id": "world-analyst-1",
                "text": "answer",
                "tool_calls": [{
                    "call_id": "tool-1",
                    "tool": "world.first-divergence",
                    "input": {"root": "event-7"},
                    "output": {"divergence_depth": 1},
                    "is_error": false
                }],
                "runtime_errors": []
            }
        })
    }

    #[test]
    fn ask_serializes_request_and_decodes_provider_neutral_turn() {
        let mut client = response_client(success_response("world-rust-analyst-1"));
        let turn = client.ask("What changed?", Some(5000)).unwrap();

        assert_eq!(turn.text.as_deref(), Some("answer"));
        assert_eq!(turn.tool_calls[0].tool, "world.first-divergence");
        assert_eq!(turn.tool_calls[0].output["divergence_depth"], 1);
        assert!(!client.is_poisoned());

        let (_, written) = client.into_parts();
        let request: Value = serde_json::from_slice(&written[..written.len() - 1]).unwrap();
        assert_eq!(request["op"], "ask");
        assert_eq!(request["id"], "world-rust-analyst-1");
        assert_eq!(request["prompt"], "What changed?");
        assert_eq!(request["timeout_ms"], 5000);
    }

    #[test]
    fn nonfatal_command_error_does_not_poison_session() {
        let mut client = response_client(serde_json::json!({
            "protocol": ANALYST_TURN_PROTOCOL,
            "version": 1,
            "type": "error",
            "id": "world-rust-analyst-1",
            "error": {"kind": "command", "fatal": false, "message": "busy"}
        }));
        let error = client.ask("question", None).unwrap_err();
        assert!(matches!(error, AnalystTurnClientError::RemoteCommand(_)));
        assert!(!client.is_poisoned());
    }

    #[test]
    fn fatal_remote_error_poison_session() {
        let mut client = response_client(serde_json::json!({
            "protocol": ANALYST_TURN_PROTOCOL,
            "version": 1,
            "type": "error",
            "id": "world-rust-analyst-1",
            "error": {"kind": "transport", "fatal": true, "message": "eof"}
        }));
        let error = client.ask("question", None).unwrap_err();
        assert!(matches!(error, AnalystTurnClientError::RemoteFatal(_)));
        assert!(client.is_poisoned());
        assert!(matches!(
            client.ask("again", None).unwrap_err(),
            AnalystTurnClientError::Poisoned
        ));
    }

    #[test]
    fn protocol_version_and_correlation_fail_closed() {
        for (response, expected) in [
            (
                serde_json::json!({
                    "protocol": "other",
                    "version": 1,
                    "type": "result",
                    "id": "world-rust-analyst-1",
                    "turn": {"request_id":"x","text":null,"tool_calls":[],"runtime_errors":[]}
                }),
                "protocol",
            ),
            (
                serde_json::json!({
                    "protocol": ANALYST_TURN_PROTOCOL,
                    "version": 2,
                    "type": "result",
                    "id": "world-rust-analyst-1",
                    "turn": {"request_id":"x","text":null,"tool_calls":[],"runtime_errors":[]}
                }),
                "version",
            ),
            (success_response("wrong-id"), "correlation"),
        ] {
            let mut client = response_client(response);
            let message = client.ask("question", None).unwrap_err().to_string();
            assert!(message.contains(expected));
            assert!(client.is_poisoned());
        }
    }

    #[test]
    fn raw_provider_or_pi_fields_are_rejected() {
        let mut raw_event = success_response("world-rust-analyst-1");
        raw_event["turn"]["events"] = serde_json::json!(["agent_settled"]);
        let mut client = response_client(raw_event);
        assert!(matches!(
            client.ask("question", None).unwrap_err(),
            AnalystTurnClientError::InvalidResponseJson(_)
        ));
        assert!(client.is_poisoned());

        let mut raw_tool = success_response("world-rust-analyst-1");
        raw_tool["turn"]["tool_calls"][0]["toolName"] = serde_json::json!("world_first_divergence");
        let mut client = response_client(raw_tool);
        assert!(matches!(
            client.ask("question", None).unwrap_err(),
            AnalystTurnClientError::InvalidResponseJson(_)
        ));
    }

    #[test]
    fn unknown_top_level_response_fields_are_rejected() {
        let mut response = success_response("world-rust-analyst-1");
        response["provider_event"] = serde_json::json!(true);
        let mut client = response_client(response);
        assert!(matches!(
            client.ask("question", None).unwrap_err(),
            AnalystTurnClientError::InvalidResponseShape(_)
        ));
    }
}
