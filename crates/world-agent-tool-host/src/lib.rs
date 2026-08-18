use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::error::Error;
use std::fmt;
use world_agent_tools::{
    JsonToolDispatchError, JsonToolInvocationError, ReadOnlyJsonToolDescriptor,
    ReadOnlyJsonToolRegistry,
};

pub const READ_ONLY_JSON_TOOL_HOST_PROTOCOL: &str = "world-machine-readonly-tools";
pub const READ_ONLY_JSON_TOOL_HOST_VERSION: u64 = 1;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "kebab-case")]
pub enum ReadOnlyJsonToolHostRequest {
    ListTools,
    Invoke {
        call_id: String,
        tool: String,
        input: Value,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReadOnlyJsonToolHostDescriptor {
    pub name: String,
    pub description: String,
    pub read_only: bool,
    pub input_schema: Value,
}

impl From<ReadOnlyJsonToolDescriptor> for ReadOnlyJsonToolHostDescriptor {
    fn from(descriptor: ReadOnlyJsonToolDescriptor) -> Self {
        Self {
            name: descriptor.name.to_owned(),
            description: descriptor.description.to_owned(),
            read_only: descriptor.read_only,
            input_schema: descriptor.input_schema,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ReadOnlyJsonToolHostErrorKind {
    UnknownTool,
    InvalidInput,
    Investigation,
    OutputSerialization,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ReadOnlyJsonToolHostError {
    pub kind: ReadOnlyJsonToolHostErrorKind,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum ReadOnlyJsonToolHostResponse {
    Catalog {
        tools: Vec<ReadOnlyJsonToolHostDescriptor>,
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

impl ReadOnlyJsonToolHostResponse {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Catalog { .. } => "catalog",
            Self::Result { .. } => "result",
            Self::Error { .. } => "error",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReadOnlyJsonToolHostEnvelope {
    pub protocol: String,
    pub version: u64,
    #[serde(flatten)]
    pub response: ReadOnlyJsonToolHostResponse,
}

#[derive(Debug)]
pub enum ReadOnlyJsonToolHostProtocolError {
    InvalidRequest(String),
    ResponseSerialization(serde_json::Error),
}

impl fmt::Display for ReadOnlyJsonToolHostProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRequest(message) => {
                write!(f, "invalid read-only tool host request: {message}")
            }
            Self::ResponseSerialization(error) => {
                write!(
                    f,
                    "failed to serialize read-only tool host response: {error}"
                )
            }
        }
    }
}

impl Error for ReadOnlyJsonToolHostProtocolError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidRequest(_) => None,
            Self::ResponseSerialization(error) => Some(error),
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
                tools: self
                    .registry
                    .descriptors()
                    .into_iter()
                    .map(ReadOnlyJsonToolHostDescriptor::from)
                    .collect(),
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
            protocol: READ_ONLY_JSON_TOOL_HOST_PROTOCOL.to_owned(),
            version: READ_ONLY_JSON_TOOL_HOST_VERSION,
            response,
        }
    }

    pub fn handle_json(
        &mut self,
        request: Value,
    ) -> Result<Value, ReadOnlyJsonToolHostProtocolError> {
        validate_request_shape(&request)?;
        let request =
            serde_json::from_value::<ReadOnlyJsonToolHostRequest>(request).map_err(|error| {
                ReadOnlyJsonToolHostProtocolError::InvalidRequest(error.to_string())
            })?;
        serde_json::to_value(self.handle(request))
            .map_err(ReadOnlyJsonToolHostProtocolError::ResponseSerialization)
    }
}

fn validate_request_shape(request: &Value) -> Result<(), ReadOnlyJsonToolHostProtocolError> {
    let object = request.as_object().ok_or_else(|| {
        ReadOnlyJsonToolHostProtocolError::InvalidRequest(
            "request must be a JSON object".to_owned(),
        )
    })?;
    let operation = object.get("op").and_then(Value::as_str).ok_or_else(|| {
        ReadOnlyJsonToolHostProtocolError::InvalidRequest(
            "request must contain a string `op` field".to_owned(),
        )
    })?;
    let allowed = match operation {
        "list-tools" => &["op"][..],
        "invoke" => &["op", "call_id", "tool", "input"][..],
        other => {
            return Err(ReadOnlyJsonToolHostProtocolError::InvalidRequest(format!(
                "unsupported operation `{other}`"
            )))
        }
    };
    if let Some(field) = object
        .keys()
        .find(|field| !allowed.contains(&field.as_str()))
    {
        return Err(ReadOnlyJsonToolHostProtocolError::InvalidRequest(format!(
            "unknown field `{field}` for `{operation}` request"
        )));
    }
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::convert::Infallible;
    use world_agent_tools::{FirstDivergenceTool, ReadOnlyJsonTool};
    use world_investigation::ComparisonQueryExecutor;
    use world_query::{
        Difference, EvidenceCausalComparisonRequest, EvidenceCausalComparisonResponse,
        EvidenceCausalDirection, EvidenceCausalDivergenceWitness, EvidenceCausalEdge,
        EvidenceCausalFirstDivergenceResult, EvidenceComparisonQueryRequest,
        EvidenceComparisonQueryResponse,
    };

    struct StaticJsonTool {
        name: &'static str,
        output: Value,
    }

    impl StaticJsonTool {
        fn new(name: &'static str, output: Value) -> Self {
            Self { name, output }
        }
    }

    impl ReadOnlyJsonTool for StaticJsonTool {
        type ExecutorError = Infallible;

        fn json_descriptor(&self) -> ReadOnlyJsonToolDescriptor {
            ReadOnlyJsonToolDescriptor {
                name: self.name,
                description: "Static host test tool.",
                read_only: true,
                input_schema: serde_json::json!({
                    "type": "object",
                    "additionalProperties": false
                }),
            }
        }

        fn invoke_json(
            &mut self,
            _input: Value,
        ) -> Result<Value, JsonToolInvocationError<Self::ExecutorError>> {
            Ok(self.output.clone())
        }
    }

    struct ScriptedExecutor {
        script: VecDeque<(
            EvidenceComparisonQueryRequest,
            EvidenceComparisonQueryResponse,
        )>,
    }

    impl ScriptedExecutor {
        fn new(
            script: Vec<(
                EvidenceComparisonQueryRequest,
                EvidenceComparisonQueryResponse,
            )>,
        ) -> Self {
            Self {
                script: script.into(),
            }
        }
    }

    impl ComparisonQueryExecutor for ScriptedExecutor {
        type Error = Infallible;

        fn execute(
            &mut self,
            request: &EvidenceComparisonQueryRequest,
        ) -> Result<EvidenceComparisonQueryResponse, Self::Error> {
            let (expected, response) = self
                .script
                .pop_front()
                .expect("every host tool query should have a scripted response");
            assert_eq!(&expected, request);
            Ok(response)
        }
    }

    fn request(
        root: &str,
        direction: EvidenceCausalDirection,
        max_depth: usize,
    ) -> EvidenceComparisonQueryRequest {
        EvidenceComparisonQueryRequest::Causal(EvidenceCausalComparisonRequest::FirstDivergence {
            root: root.into(),
            direction,
            max_depth,
        })
    }

    fn response(
        root: &str,
        direction: EvidenceCausalDirection,
        max_depth: usize,
        divergence_depth: usize,
    ) -> EvidenceComparisonQueryResponse {
        EvidenceComparisonQueryResponse::Causal(EvidenceCausalComparisonResponse::FirstDivergence {
            value: EvidenceCausalFirstDivergenceResult {
                root: root.into(),
                direction,
                max_depth,
                identical_within_depth: false,
                divergence_depth: Some(divergence_depth),
                witnesses: vec![EvidenceCausalDivergenceWitness::Edge {
                    difference: Difference::LeftOnly,
                    edge: EvidenceCausalEdge {
                        cause: "event-1".into(),
                        effect: "event-2".into(),
                    },
                    trace: vec!["event-2".into(), "event-1".into()],
                }],
                left_frontier: vec![],
                right_frontier: vec![],
                continuations: vec![],
            },
        })
    }

    #[test]
    fn host_lists_frozen_registry_catalog_in_stable_order() {
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
    fn host_envelope_round_trips_through_owned_wire_types() {
        let mut registry = ReadOnlyJsonToolRegistry::<Infallible>::new();
        registry
            .register(StaticJsonTool::new(
                "world.static",
                serde_json::json!({"ok": true}),
            ))
            .unwrap();
        let mut host = ReadOnlyJsonToolHost::new(registry);
        let envelope = host.handle(ReadOnlyJsonToolHostRequest::ListTools);

        let json = serde_json::to_string(&envelope).unwrap();
        let decoded: ReadOnlyJsonToolHostEnvelope = serde_json::from_str(&json).unwrap();

        assert_eq!(decoded, envelope);
    }

    #[test]
    fn host_json_invocation_echoes_call_identity_and_world_tool_output() {
        let direction = EvidenceCausalDirection::Upstream;
        let mut registry = ReadOnlyJsonToolRegistry::<Infallible>::new();
        registry
            .register(FirstDivergenceTool::new(ScriptedExecutor::new(vec![(
                request("event-2", direction, 1),
                response("event-2", direction, 1, 1),
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
    fn host_maps_unknown_tool_to_stable_correlated_error() {
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
    fn host_maps_invalid_tool_input_without_reaching_executor() {
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
        assert_eq!(response["call_id"], "call-invalid");
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
}
