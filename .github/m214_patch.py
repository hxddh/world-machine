from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    if old not in text:
        raise SystemExit(f"missing marker for {label}")
    return text.replace(old, new, 1)


lib_path = Path("crates/world-agent-tools/src/lib.rs")
text = lib_path.read_text()

host = r'''
pub const READ_ONLY_JSON_TOOL_HOST_PROTOCOL: &str = "world-machine-readonly-tools";
pub const READ_ONLY_JSON_TOOL_HOST_VERSION: u64 = 1;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "kebab-case", deny_unknown_fields)]
pub enum ReadOnlyJsonToolHostRequest {
    ListTools,
    Invoke {
        call_id: String,
        tool: String,
        input: Value,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ReadOnlyJsonToolHostErrorKind {
    UnknownTool,
    InvalidInput,
    Investigation,
    OutputSerialization,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ReadOnlyJsonToolHostError {
    pub kind: ReadOnlyJsonToolHostErrorKind,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum ReadOnlyJsonToolHostResponse {
    Catalog {
        tools: Vec<ReadOnlyJsonToolDescriptor>,
    },
    Result {
        call_id: String,
        tool: String,
        output: Value,
    },
    Error {
        call_id: String,
        tool: String,
        error: ReadOnlyJsonToolHostError,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ReadOnlyJsonToolHostEnvelope {
    pub protocol: &'static str,
    pub version: u64,
    #[serde(flatten)]
    pub response: ReadOnlyJsonToolHostResponse,
}

#[derive(Debug)]
pub enum ReadOnlyJsonToolHostProtocolError {
    InvalidRequest(serde_json::Error),
    ResponseSerialization(serde_json::Error),
}

impl fmt::Display for ReadOnlyJsonToolHostProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRequest(error) => write!(f, "invalid read-only tool host request: {error}"),
            Self::ResponseSerialization(error) => {
                write!(f, "failed to serialize read-only tool host response: {error}")
            }
        }
    }
}

impl Error for ReadOnlyJsonToolHostProtocolError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidRequest(error) | Self::ResponseSerialization(error) => Some(error),
        }
    }
}

pub struct ReadOnlyJsonToolHost<E> {
    registry: ReadOnlyJsonToolRegistry<E>,
}

impl<E> ReadOnlyJsonToolHost<E> {
    pub fn new(registry: ReadOnlyJsonToolRegistry<E>) -> Self {
        Self { registry }
    }

    pub fn registry(&self) -> &ReadOnlyJsonToolRegistry<E> {
        &self.registry
    }

    pub fn registry_mut(&mut self) -> &mut ReadOnlyJsonToolRegistry<E> {
        &mut self.registry
    }

    pub fn into_inner(self) -> ReadOnlyJsonToolRegistry<E> {
        self.registry
    }
}

impl<E> ReadOnlyJsonToolHost<E>
where
    E: fmt::Display,
{
    pub fn handle(&mut self, request: ReadOnlyJsonToolHostRequest) -> ReadOnlyJsonToolHostEnvelope {
        let response = match request {
            ReadOnlyJsonToolHostRequest::ListTools => ReadOnlyJsonToolHostResponse::Catalog {
                tools: self.registry.descriptors(),
            },
            ReadOnlyJsonToolHostRequest::Invoke {
                call_id,
                tool,
                input,
            } => match self.registry.dispatch(&tool, input) {
                Ok(output) => ReadOnlyJsonToolHostResponse::Result {
                    call_id,
                    tool,
                    output,
                },
                Err(dispatch_error) => ReadOnlyJsonToolHostResponse::Error {
                    call_id,
                    tool,
                    error: host_error_from_dispatch(&dispatch_error),
                },
            },
        };
        ReadOnlyJsonToolHostEnvelope {
            protocol: READ_ONLY_JSON_TOOL_HOST_PROTOCOL,
            version: READ_ONLY_JSON_TOOL_HOST_VERSION,
            response,
        }
    }

    pub fn handle_json(
        &mut self,
        request: Value,
    ) -> Result<Value, ReadOnlyJsonToolHostProtocolError> {
        let request = serde_json::from_value::<ReadOnlyJsonToolHostRequest>(request)
            .map_err(ReadOnlyJsonToolHostProtocolError::InvalidRequest)?;
        serde_json::to_value(self.handle(request))
            .map_err(ReadOnlyJsonToolHostProtocolError::ResponseSerialization)
    }
}

fn host_error_from_dispatch<E>(error: &JsonToolDispatchError<E>) -> ReadOnlyJsonToolHostError
where
    E: fmt::Display,
{
    let kind = match error {
        JsonToolDispatchError::UnknownTool { .. } => ReadOnlyJsonToolHostErrorKind::UnknownTool,
        JsonToolDispatchError::Invocation { source, .. } => match source {
            JsonToolInvocationError::InvalidInput(_) => ReadOnlyJsonToolHostErrorKind::InvalidInput,
            JsonToolInvocationError::Investigation(_) => {
                ReadOnlyJsonToolHostErrorKind::Investigation
            }
            JsonToolInvocationError::OutputSerialization(_) => {
                ReadOnlyJsonToolHostErrorKind::OutputSerialization
            }
        },
    };
    ReadOnlyJsonToolHostError {
        kind,
        message: error.to_string(),
    }
}

'''
text = replace_once(
    text,
    "\n#[cfg(test)]\nmod tests {",
    "\n" + host + "#[cfg(test)]\nmod tests {",
    "host insertion",
)

host_tests = r'''
    #[test]
    fn host_lists_the_frozen_registry_catalog_in_stable_order() {
        let mut registry = ReadOnlyJsonToolRegistry::<Infallible>::new();
        registry
            .register(StaticJsonTool::new(
                "world.zz-static",
                serde_json::json!({"source": "static"}),
            ))
            .unwrap();
        registry
            .register(FirstDivergenceTool::new(ScriptedExecutor::new(vec![])))
            .unwrap();
        let mut host = ReadOnlyJsonToolHost::new(registry);

        let envelope = host.handle(ReadOnlyJsonToolHostRequest::ListTools);
        assert_eq!(envelope.protocol, "world-machine-readonly-tools");
        assert_eq!(envelope.version, 1);
        match envelope.response {
            ReadOnlyJsonToolHostResponse::Catalog { tools } => assert_eq!(
                tools.into_iter().map(|tool| tool.name).collect::<Vec<_>>(),
                vec!["world.first-divergence", "world.zz-static"]
            ),
            response => panic!("unexpected host response: {response:?}"),
        }
    }

    #[test]
    fn host_json_invocation_echoes_call_identity_and_world_tool_output() {
        let direction = EvidenceCausalDirection::Upstream;
        let mut registry = ReadOnlyJsonToolRegistry::<Infallible>::new();
        registry
            .register(FirstDivergenceTool::new(ScriptedExecutor::new(vec![(
                request("event-2", direction, 1),
                response(
                    "event-2",
                    direction,
                    1,
                    Some(1),
                    vec![witness(&["event-2", "event-1"])],
                    vec![],
                ),
            )])))
            .unwrap();
        let mut host = ReadOnlyJsonToolHost::new(registry);

        let response = host
            .handle_json(serde_json::json!({
                "op": "invoke",
                "call_id": "call-7",
                "tool": "world.first-divergence",
                "input": {
                    "root": "event-2",
                    "direction": "upstream",
                    "window_depth": 1,
                    "max_depth": 1
                }
            }))
            .unwrap();
        assert_eq!(response["protocol"], "world-machine-readonly-tools");
        assert_eq!(response["version"], 1);
        assert_eq!(response["type"], "result");
        assert_eq!(response["call_id"], "call-7");
        assert_eq!(response["tool"], "world.first-divergence");
        assert_eq!(response["output"]["divergence_depth"], 1);
        assert_eq!(
            response["output"]["witnesses"][0]["trace"],
            serde_json::json!(["event-2", "event-1"])
        );
    }

    #[test]
    fn host_maps_unknown_tool_to_a_stable_correlated_error() {
        let registry = ReadOnlyJsonToolRegistry::<Infallible>::new();
        let mut host = ReadOnlyJsonToolHost::new(registry);
        let response = host
            .handle_json(serde_json::json!({
                "op": "invoke",
                "call_id": "call-missing",
                "tool": "world.missing",
                "input": {}
            }))
            .unwrap();
        assert_eq!(response["type"], "error");
        assert_eq!(response["call_id"], "call-missing");
        assert_eq!(response["tool"], "world.missing");
        assert_eq!(response["error"]["kind"], "unknown-tool");
    }

    #[test]
    fn host_maps_invalid_tool_input_without_reaching_the_executor() {
        let mut registry = ReadOnlyJsonToolRegistry::<Infallible>::new();
        registry
            .register(FirstDivergenceTool::new(ScriptedExecutor::new(vec![])))
            .unwrap();
        let mut host = ReadOnlyJsonToolHost::new(registry);
        let response = host
            .handle_json(serde_json::json!({
                "op": "invoke",
                "call_id": "call-invalid",
                "tool": "world.first-divergence",
                "input": {
                    "root": "event-2",
                    "direction": "sideways",
                    "window_depth": 1,
                    "max_depth": 2
                }
            }))
            .unwrap();
        assert_eq!(response["type"], "error");
        assert_eq!(response["error"]["kind"], "invalid-input");
    }

    #[test]
    fn host_protocol_rejects_unknown_request_fields_before_dispatch() {
        let registry = ReadOnlyJsonToolRegistry::<Infallible>::new();
        let mut host = ReadOnlyJsonToolHost::new(registry);
        let error = host
            .handle_json(serde_json::json!({
                "op": "list-tools",
                "mutate": true
            }))
            .unwrap_err();
        assert!(matches!(
            error,
            ReadOnlyJsonToolHostProtocolError::InvalidRequest(_)
        ));
    }

'''
text = replace_once(
    text,
    "    #[test]\n    fn output_is_serializable_without_exposing_executor_or_world_internals() {",
    host_tests + "    #[test]\n    fn output_is_serializable_without_exposing_executor_or_world_internals() {",
    "host tests insertion",
)
lib_path.write_text(text)

Path("NEXT_TASK.md").write_text(r'''# Next Coding Task — M214 Read-Only Analyst Tool Host

Expose the M213 provider-neutral registry through a transport-neutral, correlated JSON host boundary for external analyst agents, while keeping the in-world `AgentRuntime` decision-only and perception-scoped.

## Current baseline

M209 owns progressive investigation, M210 adds the local CLI executor adapter, M211/M212 define typed and JSON read-only tools, and M213 adds deterministic multi-tool registration and dispatch. `world-pi-rpc` remains an in-world decision adapter and explicitly rejects tool execution events, so investigation tools must not be injected there.

## M214 — transport-neutral read-only tool host

Extend `world-agent-tools` with `ReadOnlyJsonToolHost<E>` and a versioned JSON envelope.

Requests are strict, provider-neutral JSON:

- `{"op":"list-tools"}`
- `{"op":"invoke","call_id":"...","tool":"...","input":{...}}`

Responses carry protocol `world-machine-readonly-tools`, version `1`, and one of:

- a deterministic `catalog` from the frozen M213 descriptors;
- a correlated `result` echoing `call_id` and tool name;
- a correlated `error` with stable kind `unknown-tool`, `invalid-input`, `investigation`, or `output-serialization`.

## Error boundary

M213 keeps typed `JsonToolDispatchError<E>` internally. M214 erases that typed error only at the external JSON host boundary, preserving a stable error kind plus diagnostic message. Malformed host requests are protocol failures returned from `handle_json` and never reach registry dispatch.

## Boundary rules

- Do not connect this host to the in-world `world-pi-rpc` / `AgentRuntime`; that path remains decision-only and continues rejecting tool execution.
- The host owns only a read-only registry supplied by its caller. It gains no Projection, archive, filesystem, network, model, or World mutation authority.
- No OpenAI, Anthropic, Pi, MCP, HTTP, or WebSocket SDK/protocol types enter the host contract.
- Tool invocation still flows M214 host -> M213 registry -> M212 JSON tool -> M211 typed tool -> M209 investigation.

## Validation

- stable protocol/version and deterministic catalog;
- correlated call id/tool name on result and error;
- real first-divergence invocation preserves witness trace;
- unknown tool and invalid input map to distinct stable error kinds;
- unknown host request fields fail before dispatch;
- focused fmt/boundary/tests/Clippy plus full workspace CI and external Pack conformance.

## Non-goals

No provider-specific adapter yet, no in-world tool use, no network server, no mutable tools, no server-side investigation cursor/session, and no evidence-query protocol v2.
''')
