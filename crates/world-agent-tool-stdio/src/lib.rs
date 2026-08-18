use serde_json::Value;
use std::error::Error;
use std::ffi::OsStr;
use std::fmt;
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, ExitStatus, Stdio};
use world_agent_tool_host::{
    ReadOnlyJsonToolHostDescriptor, ReadOnlyJsonToolHostEnvelope, ReadOnlyJsonToolHostError,
    ReadOnlyJsonToolHostRequest, ReadOnlyJsonToolHostResponse, READ_ONLY_JSON_TOOL_HOST_PROTOCOL,
    READ_ONLY_JSON_TOOL_HOST_VERSION,
};

pub struct ReadOnlyJsonToolStdioClient<R, W> {
    reader: R,
    writer: W,
}

impl<R, W> ReadOnlyJsonToolStdioClient<R, W> {
    pub fn new(reader: R, writer: W) -> Self {
        Self { reader, writer }
    }

    pub fn into_parts(self) -> (R, W) {
        (self.reader, self.writer)
    }
}

impl<R, W> ReadOnlyJsonToolStdioClient<R, W>
where
    R: BufRead,
    W: Write,
{
    pub fn list_tools(
        &mut self,
    ) -> Result<Vec<ReadOnlyJsonToolHostDescriptor>, ReadOnlyJsonToolStdioClientError> {
        let envelope = self.round_trip(&ReadOnlyJsonToolHostRequest::ListTools)?;
        match envelope.response {
            ReadOnlyJsonToolHostResponse::Catalog { tools } => Ok(tools),
            response => Err(ReadOnlyJsonToolStdioClientError::UnexpectedResponse {
                expected: "catalog",
                actual: response.kind(),
            }),
        }
    }

    pub fn invoke(
        &mut self,
        call_id: impl Into<String>,
        tool: impl Into<String>,
        input: Value,
    ) -> Result<Value, ReadOnlyJsonToolStdioClientError> {
        let call_id = call_id.into();
        let tool = tool.into();
        let envelope = self.round_trip(&ReadOnlyJsonToolHostRequest::Invoke {
            call_id: call_id.clone(),
            tool: tool.clone(),
            input,
        })?;
        match envelope.response {
            ReadOnlyJsonToolHostResponse::Result {
                call_id: actual_call_id,
                tool: actual_tool,
                output,
            } => {
                validate_correlation(&call_id, &tool, &actual_call_id, &actual_tool)?;
                Ok(output)
            }
            ReadOnlyJsonToolHostResponse::Error {
                call_id: actual_call_id,
                tool: actual_tool,
                error,
            } => {
                validate_correlation(&call_id, &tool, &actual_call_id, &actual_tool)?;
                Err(ReadOnlyJsonToolStdioClientError::RemoteTool {
                    call_id,
                    tool,
                    error,
                })
            }
            response => Err(ReadOnlyJsonToolStdioClientError::UnexpectedResponse {
                expected: "result-or-error",
                actual: response.kind(),
            }),
        }
    }

    fn round_trip(
        &mut self,
        request: &ReadOnlyJsonToolHostRequest,
    ) -> Result<ReadOnlyJsonToolHostEnvelope, ReadOnlyJsonToolStdioClientError> {
        let encoded = serde_json::to_vec(request)
            .map_err(ReadOnlyJsonToolStdioClientError::SerializeRequest)?;
        self.writer
            .write_all(&encoded)
            .map_err(ReadOnlyJsonToolStdioClientError::WriteRequest)?;
        self.writer
            .write_all(b"\n")
            .map_err(ReadOnlyJsonToolStdioClientError::WriteRequest)?;
        self.writer
            .flush()
            .map_err(ReadOnlyJsonToolStdioClientError::WriteRequest)?;

        let mut line = String::new();
        let read = self
            .reader
            .read_line(&mut line)
            .map_err(ReadOnlyJsonToolStdioClientError::ReadResponse)?;
        if read == 0 {
            return Err(ReadOnlyJsonToolStdioClientError::UnexpectedEof);
        }
        let envelope: ReadOnlyJsonToolHostEnvelope = serde_json::from_str(&line)
            .map_err(ReadOnlyJsonToolStdioClientError::InvalidResponseJson)?;
        if envelope.protocol != READ_ONLY_JSON_TOOL_HOST_PROTOCOL {
            return Err(ReadOnlyJsonToolStdioClientError::ProtocolMismatch {
                actual: envelope.protocol,
            });
        }
        if envelope.version != READ_ONLY_JSON_TOOL_HOST_VERSION {
            return Err(ReadOnlyJsonToolStdioClientError::VersionMismatch {
                actual: envelope.version,
            });
        }
        Ok(envelope)
    }
}

fn validate_correlation(
    expected_call_id: &str,
    expected_tool: &str,
    actual_call_id: &str,
    actual_tool: &str,
) -> Result<(), ReadOnlyJsonToolStdioClientError> {
    if expected_call_id == actual_call_id && expected_tool == actual_tool {
        return Ok(());
    }
    Err(ReadOnlyJsonToolStdioClientError::CorrelationMismatch {
        expected_call_id: expected_call_id.to_owned(),
        expected_tool: expected_tool.to_owned(),
        actual_call_id: actual_call_id.to_owned(),
        actual_tool: actual_tool.to_owned(),
    })
}

#[derive(Debug)]
pub enum ReadOnlyJsonToolStdioClientError {
    SerializeRequest(serde_json::Error),
    WriteRequest(io::Error),
    ReadResponse(io::Error),
    UnexpectedEof,
    InvalidResponseJson(serde_json::Error),
    ProtocolMismatch {
        actual: String,
    },
    VersionMismatch {
        actual: u64,
    },
    UnexpectedResponse {
        expected: &'static str,
        actual: &'static str,
    },
    CorrelationMismatch {
        expected_call_id: String,
        expected_tool: String,
        actual_call_id: String,
        actual_tool: String,
    },
    RemoteTool {
        call_id: String,
        tool: String,
        error: ReadOnlyJsonToolHostError,
    },
}

impl fmt::Display for ReadOnlyJsonToolStdioClientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SerializeRequest(error) => {
                write!(f, "failed to serialize read-only tool request: {error}")
            }
            Self::WriteRequest(error) => write!(f, "failed to write read-only tool request: {error}"),
            Self::ReadResponse(error) => write!(f, "failed to read read-only tool response: {error}"),
            Self::UnexpectedEof => write!(f, "read-only tool stdio process closed before responding"),
            Self::InvalidResponseJson(error) => {
                write!(f, "invalid read-only tool response JSON: {error}")
            }
            Self::ProtocolMismatch { actual } => write!(
                f,
                "unexpected read-only tool protocol `{actual}`; expected `{READ_ONLY_JSON_TOOL_HOST_PROTOCOL}`"
            ),
            Self::VersionMismatch { actual } => write!(
                f,
                "unexpected read-only tool protocol version {actual}; expected {READ_ONLY_JSON_TOOL_HOST_VERSION}"
            ),
            Self::UnexpectedResponse { expected, actual } => write!(
                f,
                "unexpected read-only tool response `{actual}`; expected `{expected}`"
            ),
            Self::CorrelationMismatch {
                expected_call_id,
                expected_tool,
                actual_call_id,
                actual_tool,
            } => write!(
                f,
                "read-only tool response correlation mismatch: expected call_id={expected_call_id} tool={expected_tool}, got call_id={actual_call_id} tool={actual_tool}"
            ),
            Self::RemoteTool {
                call_id,
                tool,
                error,
            } => write!(
                f,
                "read-only tool call {call_id} ({tool}) failed remotely: {}",
                error.message
            ),
        }
    }
}

impl Error for ReadOnlyJsonToolStdioClientError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::SerializeRequest(error) | Self::InvalidResponseJson(error) => Some(error),
            Self::WriteRequest(error) | Self::ReadResponse(error) => Some(error),
            Self::UnexpectedEof
            | Self::ProtocolMismatch { .. }
            | Self::VersionMismatch { .. }
            | Self::UnexpectedResponse { .. }
            | Self::CorrelationMismatch { .. }
            | Self::RemoteTool { .. } => None,
        }
    }
}

pub struct ReadOnlyJsonToolStdioProcess {
    child: Child,
    client: Option<ReadOnlyJsonToolStdioClient<BufReader<ChildStdout>, BufWriter<ChildStdin>>>,
}

impl ReadOnlyJsonToolStdioProcess {
    pub fn spawn(
        program: impl AsRef<OsStr>,
        left_archive: impl AsRef<OsStr>,
        right_archive: impl AsRef<OsStr>,
    ) -> io::Result<Self> {
        let mut child = Command::new(program)
            .arg(left_archive)
            .arg(right_archive)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()?;
        let Some(stdin) = child.stdin.take() else {
            let _ = child.kill();
            let _ = child.wait();
            return Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "read-only tool stdio process did not expose stdin",
            ));
        };
        let Some(stdout) = child.stdout.take() else {
            drop(stdin);
            let _ = child.kill();
            let _ = child.wait();
            return Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "read-only tool stdio process did not expose stdout",
            ));
        };
        Ok(Self {
            child,
            client: Some(ReadOnlyJsonToolStdioClient::new(
                BufReader::new(stdout),
                BufWriter::new(stdin),
            )),
        })
    }

    pub fn id(&self) -> u32 {
        self.child.id()
    }

    pub fn list_tools(
        &mut self,
    ) -> Result<Vec<ReadOnlyJsonToolHostDescriptor>, ReadOnlyJsonToolStdioClientError> {
        self.client_mut().list_tools()
    }

    pub fn invoke(
        &mut self,
        call_id: impl Into<String>,
        tool: impl Into<String>,
        input: Value,
    ) -> Result<Value, ReadOnlyJsonToolStdioClientError> {
        self.client_mut().invoke(call_id, tool, input)
    }

    pub fn shutdown(mut self) -> io::Result<ExitStatus> {
        self.client.take();
        self.child.wait()
    }

    fn client_mut(
        &mut self,
    ) -> &mut ReadOnlyJsonToolStdioClient<BufReader<ChildStdout>, BufWriter<ChildStdin>> {
        self.client
            .as_mut()
            .expect("stdio client remains available until shutdown")
    }
}

impl Drop for ReadOnlyJsonToolStdioProcess {
    fn drop(&mut self) {
        self.client.take();
        match self.child.try_wait() {
            Ok(Some(_)) => {}
            Ok(None) | Err(_) => {
                let _ = self.child.kill();
                let _ = self.child.wait();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use world_agent_tool_host::ReadOnlyJsonToolHostErrorKind;

    fn client_with_response(
        response: Value,
    ) -> ReadOnlyJsonToolStdioClient<Cursor<Vec<u8>>, Vec<u8>> {
        let mut bytes = serde_json::to_vec(&response).unwrap();
        bytes.push(b'\n');
        ReadOnlyJsonToolStdioClient::new(Cursor::new(bytes), Vec::new())
    }

    #[test]
    fn list_tools_round_trip_emits_request_and_decodes_catalog() {
        let mut client = client_with_response(serde_json::json!({
            "protocol": READ_ONLY_JSON_TOOL_HOST_PROTOCOL,
            "version": READ_ONLY_JSON_TOOL_HOST_VERSION,
            "type": "catalog",
            "tools": [{
                "name": "world.first-divergence",
                "description": "Find divergence.",
                "read_only": true,
                "input_schema": {"type": "object"}
            }]
        }));

        let tools = client.list_tools().unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "world.first-divergence");
        assert!(tools[0].read_only);

        let (_, written) = client.into_parts();
        assert_eq!(
            String::from_utf8(written).unwrap(),
            "{\"op\":\"list-tools\"}\n"
        );
    }

    #[test]
    fn invoke_validates_correlation_and_returns_output() {
        let mut client = client_with_response(serde_json::json!({
            "protocol": READ_ONLY_JSON_TOOL_HOST_PROTOCOL,
            "version": READ_ONLY_JSON_TOOL_HOST_VERSION,
            "type": "result",
            "call_id": "call-7",
            "tool": "world.first-divergence",
            "output": {"divergence_depth": 2}
        }));

        let output = client
            .invoke(
                "call-7",
                "world.first-divergence",
                serde_json::json!({"root": "event-4"}),
            )
            .unwrap();
        assert_eq!(output["divergence_depth"], 2);
    }

    #[test]
    fn invoke_preserves_correlated_remote_tool_error() {
        let mut client = client_with_response(serde_json::json!({
            "protocol": READ_ONLY_JSON_TOOL_HOST_PROTOCOL,
            "version": READ_ONLY_JSON_TOOL_HOST_VERSION,
            "type": "error",
            "call_id": "call-missing",
            "tool": "world.missing",
            "error": {
                "kind": "unknown-tool",
                "message": "unknown read-only JSON tool: world.missing"
            }
        }));

        let error = client
            .invoke("call-missing", "world.missing", serde_json::json!({}))
            .unwrap_err();
        let ReadOnlyJsonToolStdioClientError::RemoteTool { error, .. } = error else {
            panic!("expected remote tool error")
        };
        assert_eq!(error.kind, ReadOnlyJsonToolHostErrorKind::UnknownTool);
    }

    #[test]
    fn response_protocol_and_version_are_strict() {
        let mut wrong_protocol = client_with_response(serde_json::json!({
            "protocol": "other-tools",
            "version": READ_ONLY_JSON_TOOL_HOST_VERSION,
            "type": "catalog",
            "tools": []
        }));
        assert!(matches!(
            wrong_protocol.list_tools().unwrap_err(),
            ReadOnlyJsonToolStdioClientError::ProtocolMismatch { .. }
        ));

        let mut wrong_version = client_with_response(serde_json::json!({
            "protocol": READ_ONLY_JSON_TOOL_HOST_PROTOCOL,
            "version": 2,
            "type": "catalog",
            "tools": []
        }));
        assert!(matches!(
            wrong_version.list_tools().unwrap_err(),
            ReadOnlyJsonToolStdioClientError::VersionMismatch { actual: 2 }
        ));
    }

    #[test]
    fn invoke_rejects_mismatched_call_identity() {
        let mut client = client_with_response(serde_json::json!({
            "protocol": READ_ONLY_JSON_TOOL_HOST_PROTOCOL,
            "version": READ_ONLY_JSON_TOOL_HOST_VERSION,
            "type": "result",
            "call_id": "other-call",
            "tool": "world.other",
            "output": {}
        }));

        let error = client
            .invoke("call-1", "world.first-divergence", serde_json::json!({}))
            .unwrap_err();
        assert!(matches!(
            error,
            ReadOnlyJsonToolStdioClientError::CorrelationMismatch { .. }
        ));
    }

    #[test]
    fn eof_before_response_is_transport_failure() {
        let mut client = ReadOnlyJsonToolStdioClient::new(Cursor::new(Vec::new()), Vec::new());
        assert!(matches!(
            client.list_tools().unwrap_err(),
            ReadOnlyJsonToolStdioClientError::UnexpectedEof
        ));
    }
}
