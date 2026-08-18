use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use world_investigation::{
    investigate_first_divergence, ComparisonQueryExecutor, FirstDivergenceInvestigationRequest,
    InvestigationError,
};
use world_query::{EvidenceCausalDirection, EvidenceCausalDivergenceWitness};

pub const FIRST_DIVERGENCE_TOOL_NAME: &str = "world.first-divergence";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReadOnlyToolDescriptor {
    pub name: &'static str,
    pub description: &'static str,
    pub read_only: bool,
}

pub fn first_divergence_tool_descriptor() -> ReadOnlyToolDescriptor {
    ReadOnlyToolDescriptor {
        name: FIRST_DIVERGENCE_TOOL_NAME,
        description: "Find the earliest visible causal divergence between two World histories within a bounded depth and return original-root witness traces.",
        read_only: true,
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ReadOnlyJsonToolDescriptor {
    pub name: &'static str,
    pub description: &'static str,
    pub read_only: bool,
    pub input_schema: Value,
}

pub fn first_divergence_input_schema() -> Value {
    serde_json::json!({
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "root": {
                "type": "string",
                "description": "Canonical visible Event selection key such as event-7."
            },
            "direction": {
                "type": "string",
                "enum": ["upstream", "downstream"]
            },
            "window_depth": {
                "type": "integer",
                "minimum": 1
            },
            "max_depth": {
                "type": "integer",
                "minimum": 0
            }
        },
        "required": ["root", "direction", "window_depth", "max_depth"]
    })
}

pub fn first_divergence_json_tool_descriptor() -> ReadOnlyJsonToolDescriptor {
    let descriptor = first_divergence_tool_descriptor();
    ReadOnlyJsonToolDescriptor {
        name: descriptor.name,
        description: descriptor.description,
        read_only: descriptor.read_only,
        input_schema: first_divergence_input_schema(),
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FirstDivergenceToolInput {
    pub root: String,
    pub direction: EvidenceCausalDirection,
    pub window_depth: usize,
    pub max_depth: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct FirstDivergenceToolOutput {
    pub root: String,
    pub direction: EvidenceCausalDirection,
    pub max_depth: usize,
    pub identical_within_depth: bool,
    pub divergence_depth: Option<usize>,
    pub witnesses: Vec<EvidenceCausalDivergenceWitness>,
    pub truncated: bool,
}

pub struct FirstDivergenceTool<E> {
    executor: E,
}

impl<E> FirstDivergenceTool<E> {
    pub fn new(executor: E) -> Self {
        Self { executor }
    }

    pub fn descriptor(&self) -> ReadOnlyToolDescriptor {
        first_divergence_tool_descriptor()
    }

    pub fn executor(&self) -> &E {
        &self.executor
    }

    pub fn executor_mut(&mut self) -> &mut E {
        &mut self.executor
    }

    pub fn into_inner(self) -> E {
        self.executor
    }
}

impl<E> FirstDivergenceTool<E>
where
    E: ComparisonQueryExecutor,
{
    pub fn invoke(
        &mut self,
        input: &FirstDivergenceToolInput,
    ) -> Result<FirstDivergenceToolOutput, InvestigationError<E::Error>> {
        let result = investigate_first_divergence(
            &mut self.executor,
            &FirstDivergenceInvestigationRequest {
                root: input.root.clone(),
                direction: input.direction,
                window_depth: input.window_depth,
                max_depth: input.max_depth,
            },
        )?;
        Ok(FirstDivergenceToolOutput {
            root: result.root,
            direction: result.direction,
            max_depth: result.max_depth,
            identical_within_depth: result.identical_within_depth,
            divergence_depth: result.divergence_depth,
            witnesses: result.witnesses,
            truncated: result.truncated,
        })
    }
}

#[derive(Debug)]
pub enum JsonToolInvocationError<E> {
    InvalidInput(serde_json::Error),
    Investigation(InvestigationError<E>),
    OutputSerialization(serde_json::Error),
}

impl<E: fmt::Display> fmt::Display for JsonToolInvocationError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInput(error) => write!(f, "invalid tool input: {error}"),
            Self::Investigation(error) => error.fmt(f),
            Self::OutputSerialization(error) => {
                write!(f, "failed to serialize tool output: {error}")
            }
        }
    }
}

impl<E> Error for JsonToolInvocationError<E>
where
    E: Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidInput(error) | Self::OutputSerialization(error) => Some(error),
            Self::Investigation(error) => Some(error),
        }
    }
}

pub trait ReadOnlyJsonTool {
    type ExecutorError;

    fn json_descriptor(&self) -> ReadOnlyJsonToolDescriptor;

    fn invoke_json(
        &mut self,
        input: Value,
    ) -> Result<Value, JsonToolInvocationError<Self::ExecutorError>>;
}

impl<E> ReadOnlyJsonTool for FirstDivergenceTool<E>
where
    E: ComparisonQueryExecutor,
{
    type ExecutorError = E::Error;

    fn json_descriptor(&self) -> ReadOnlyJsonToolDescriptor {
        first_divergence_json_tool_descriptor()
    }

    fn invoke_json(
        &mut self,
        input: Value,
    ) -> Result<Value, JsonToolInvocationError<Self::ExecutorError>> {
        let input = serde_json::from_value::<FirstDivergenceToolInput>(input)
            .map_err(JsonToolInvocationError::InvalidInput)?;
        let output = self
            .invoke(&input)
            .map_err(JsonToolInvocationError::Investigation)?;
        serde_json::to_value(output).map_err(JsonToolInvocationError::OutputSerialization)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReadOnlyJsonToolRegistryError {
    DuplicateTool { name: String },
}

impl fmt::Display for ReadOnlyJsonToolRegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateTool { name } => {
                write!(f, "read-only JSON tool is already registered: {name}")
            }
        }
    }
}

impl Error for ReadOnlyJsonToolRegistryError {}

#[derive(Debug)]
pub enum JsonToolDispatchError<E> {
    UnknownTool {
        name: String,
    },
    Invocation {
        tool: String,
        source: JsonToolInvocationError<E>,
    },
}

impl<E: fmt::Display> fmt::Display for JsonToolDispatchError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownTool { name } => write!(f, "unknown read-only JSON tool: {name}"),
            Self::Invocation { tool, source } => {
                write!(f, "read-only JSON tool {tool} failed: {source}")
            }
        }
    }
}

impl<E> Error for JsonToolDispatchError<E>
where
    E: Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::UnknownTool { .. } => None,
            Self::Invocation { source, .. } => Some(source),
        }
    }
}

struct RegisteredReadOnlyJsonTool<E> {
    descriptor: ReadOnlyJsonToolDescriptor,
    tool: Box<dyn ReadOnlyJsonTool<ExecutorError = E>>,
}

pub struct ReadOnlyJsonToolRegistry<E> {
    tools: BTreeMap<&'static str, RegisteredReadOnlyJsonTool<E>>,
}

impl<E> Default for ReadOnlyJsonToolRegistry<E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<E> ReadOnlyJsonToolRegistry<E> {
    pub fn new() -> Self {
        Self {
            tools: BTreeMap::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.tools.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }

    pub fn contains(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }

    pub fn register<T>(&mut self, tool: T) -> Result<(), ReadOnlyJsonToolRegistryError>
    where
        T: ReadOnlyJsonTool<ExecutorError = E> + 'static,
    {
        let descriptor = tool.json_descriptor();
        if self.tools.contains_key(descriptor.name) {
            return Err(ReadOnlyJsonToolRegistryError::DuplicateTool {
                name: descriptor.name.to_owned(),
            });
        }
        self.tools.insert(
            descriptor.name,
            RegisteredReadOnlyJsonTool {
                descriptor,
                tool: Box::new(tool),
            },
        );
        Ok(())
    }

    pub fn descriptor(&self, name: &str) -> Option<&ReadOnlyJsonToolDescriptor> {
        self.tools
            .get(name)
            .map(|registered| &registered.descriptor)
    }

    pub fn descriptors(&self) -> Vec<ReadOnlyJsonToolDescriptor> {
        self.tools
            .values()
            .map(|registered| registered.descriptor.clone())
            .collect()
    }

    pub fn dispatch(
        &mut self,
        name: &str,
        input: Value,
    ) -> Result<Value, JsonToolDispatchError<E>> {
        let registered =
            self.tools
                .get_mut(name)
                .ok_or_else(|| JsonToolDispatchError::UnknownTool {
                    name: name.to_owned(),
                })?;
        registered
            .tool
            .invoke_json(input)
            .map_err(|source| JsonToolDispatchError::Invocation {
                tool: name.to_owned(),
                source,
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::convert::Infallible;
    use world_query::{
        Difference, EvidenceCausalComparisonRequest, EvidenceCausalComparisonResponse,
        EvidenceCausalEdge, EvidenceCausalFirstDivergenceContinuation,
        EvidenceCausalFirstDivergenceResult, EvidenceComparisonQueryRequest,
        EvidenceComparisonQueryResponse,
    };

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
                .expect("every tool query should have a scripted response");
            assert_eq!(&expected, request);
            Ok(response)
        }
    }

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
                description: "Static test tool.",
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
        divergence_depth: Option<usize>,
        witnesses: Vec<EvidenceCausalDivergenceWitness>,
        continuations: Vec<EvidenceCausalFirstDivergenceContinuation>,
    ) -> EvidenceComparisonQueryResponse {
        EvidenceComparisonQueryResponse::Causal(EvidenceCausalComparisonResponse::FirstDivergence {
            value: EvidenceCausalFirstDivergenceResult {
                root: root.into(),
                direction,
                max_depth,
                identical_within_depth: divergence_depth.is_none(),
                divergence_depth,
                witnesses,
                left_frontier: vec![],
                right_frontier: vec![],
                continuations,
            },
        })
    }

    fn continuation(
        event: &str,
        direction: EvidenceCausalDirection,
        depth_offset: usize,
        trace_prefix: &[&str],
        replay_depth: usize,
    ) -> EvidenceCausalFirstDivergenceContinuation {
        EvidenceCausalFirstDivergenceContinuation {
            event: event.into(),
            direction,
            left_frontier: true,
            right_frontier: true,
            depth_offset,
            trace_prefix: trace_prefix.iter().map(|event| (*event).into()).collect(),
            request: request(event, direction, replay_depth),
        }
    }

    fn witness(trace: &[&str]) -> EvidenceCausalDivergenceWitness {
        EvidenceCausalDivergenceWitness::Edge {
            difference: Difference::LeftOnly,
            edge: EvidenceCausalEdge {
                cause: "event-1".into(),
                effect: "event-2".into(),
            },
            trace: trace.iter().map(|event| (*event).into()).collect(),
        }
    }

    #[test]
    fn descriptor_is_stably_named_and_explicitly_read_only() {
        let descriptor = first_divergence_tool_descriptor();
        assert_eq!(descriptor.name, "world.first-divergence");
        assert!(descriptor.read_only);
        assert!(!descriptor.description.is_empty());
    }

    #[test]
    fn typed_input_has_stable_provider_neutral_json_shape() {
        let input = FirstDivergenceToolInput {
            root: "event-4".into(),
            direction: EvidenceCausalDirection::Upstream,
            window_depth: 1,
            max_depth: 3,
        };
        assert_eq!(
            serde_json::to_value(&input).unwrap(),
            serde_json::json!({
                "root": "event-4",
                "direction": "upstream",
                "window_depth": 1,
                "max_depth": 3,
            })
        );
        assert_eq!(
            serde_json::from_value::<FirstDivergenceToolInput>(
                serde_json::to_value(&input).unwrap()
            )
            .unwrap(),
            input
        );
    }

    #[test]
    fn tool_reuses_investigation_scheduler_and_returns_original_root_trace() {
        let direction = EvidenceCausalDirection::Upstream;
        let mut tool = FirstDivergenceTool::new(ScriptedExecutor::new(vec![
            (
                request("event-4", direction, 1),
                response(
                    "event-4",
                    direction,
                    1,
                    None,
                    vec![],
                    vec![continuation(
                        "event-3",
                        direction,
                        1,
                        &["event-4", "event-3"],
                        1,
                    )],
                ),
            ),
            (
                request("event-3", direction, 1),
                response(
                    "event-3",
                    direction,
                    1,
                    None,
                    vec![],
                    vec![continuation(
                        "event-2",
                        direction,
                        1,
                        &["event-3", "event-2"],
                        1,
                    )],
                ),
            ),
            (
                request("event-2", direction, 1),
                response(
                    "event-2",
                    direction,
                    1,
                    Some(1),
                    vec![witness(&["event-2", "event-1"])],
                    vec![],
                ),
            ),
        ]));

        let output = tool
            .invoke(&FirstDivergenceToolInput {
                root: "event-4".into(),
                direction,
                window_depth: 1,
                max_depth: 3,
            })
            .unwrap();

        assert_eq!(output.divergence_depth, Some(3));
        assert!(!output.identical_within_depth);
        assert!(!output.truncated);
        assert_eq!(
            output.witnesses,
            vec![witness(&["event-4", "event-3", "event-2", "event-1"])]
        );
        assert!(tool.executor().script.is_empty());
    }

    #[test]
    fn json_descriptor_has_stable_provider_neutral_schema() {
        let descriptor = first_divergence_json_tool_descriptor();
        assert_eq!(descriptor.name, "world.first-divergence");
        assert!(descriptor.read_only);
        assert_eq!(descriptor.input_schema["type"], "object");
        assert_eq!(descriptor.input_schema["additionalProperties"], false);
        assert_eq!(
            descriptor.input_schema["properties"]["direction"]["enum"],
            serde_json::json!(["upstream", "downstream"])
        );
        assert_eq!(
            descriptor.input_schema["properties"]["window_depth"]["minimum"],
            1
        );
        assert_eq!(
            descriptor.input_schema["required"],
            serde_json::json!(["root", "direction", "window_depth", "max_depth"])
        );
        let serialized = serde_json::to_value(descriptor).unwrap();
        assert_eq!(serialized["name"], "world.first-divergence");
        assert_eq!(serialized["read_only"], true);
    }

    #[test]
    fn json_tool_dispatches_valid_input_through_typed_investigation() {
        let direction = EvidenceCausalDirection::Upstream;
        let mut tool = FirstDivergenceTool::new(ScriptedExecutor::new(vec![(
            request("event-2", direction, 1),
            response(
                "event-2",
                direction,
                1,
                Some(1),
                vec![witness(&["event-2", "event-1"])],
                vec![],
            ),
        )]));

        let output = tool
            .invoke_json(serde_json::json!({
                "root": "event-2",
                "direction": "upstream",
                "window_depth": 1,
                "max_depth": 1,
            }))
            .unwrap();

        assert_eq!(output["root"], "event-2");
        assert_eq!(output["direction"], "upstream");
        assert_eq!(output["divergence_depth"], 1);
        assert_eq!(
            output["witnesses"][0]["trace"],
            serde_json::json!(["event-2", "event-1"])
        );
        assert!(tool.executor().script.is_empty());
    }

    #[test]
    fn json_tool_rejects_unknown_or_malformed_fields_before_executor_use() {
        let mut tool = FirstDivergenceTool::new(ScriptedExecutor::new(vec![]));
        let error = tool
            .invoke_json(serde_json::json!({
                "root": "event-2",
                "direction": "sideways",
                "window_depth": 1,
                "max_depth": 2,
                "mutate": true,
            }))
            .unwrap_err();
        assert!(matches!(error, JsonToolInvocationError::InvalidInput(_)));
        assert!(tool.executor().script.is_empty());
    }

    #[test]
    fn registry_freezes_descriptors_in_deterministic_name_order() {
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

        assert_eq!(registry.len(), 2);
        assert!(registry.contains("world.first-divergence"));
        assert_eq!(
            registry
                .descriptors()
                .into_iter()
                .map(|descriptor| descriptor.name)
                .collect::<Vec<_>>(),
            vec!["world.first-divergence", "world.zz-static"]
        );
        assert_eq!(
            registry
                .descriptor("world.first-divergence")
                .unwrap()
                .input_schema["additionalProperties"],
            false
        );
    }

    #[test]
    fn registry_rejects_duplicate_names_without_replacing_the_original_tool() {
        let mut registry = ReadOnlyJsonToolRegistry::<Infallible>::new();
        registry
            .register(StaticJsonTool::new(
                "world.static",
                serde_json::json!({"value": 1}),
            ))
            .unwrap();
        let error = registry
            .register(StaticJsonTool::new(
                "world.static",
                serde_json::json!({"value": 2}),
            ))
            .unwrap_err();
        assert_eq!(
            error,
            ReadOnlyJsonToolRegistryError::DuplicateTool {
                name: "world.static".into()
            }
        );
        assert_eq!(
            registry
                .dispatch("world.static", serde_json::json!({}))
                .unwrap(),
            serde_json::json!({"value": 1})
        );
    }

    #[test]
    fn registry_reports_unknown_tool_before_any_registered_dispatch() {
        let mut registry = ReadOnlyJsonToolRegistry::<Infallible>::new();
        registry
            .register(StaticJsonTool::new(
                "world.static",
                serde_json::json!({"value": 1}),
            ))
            .unwrap();
        let error = registry
            .dispatch("world.missing", serde_json::json!({}))
            .unwrap_err();
        assert!(matches!(
            error,
            JsonToolDispatchError::UnknownTool { ref name } if name == "world.missing"
        ));
        assert_eq!(
            registry
                .dispatch("world.static", serde_json::json!({}))
                .unwrap(),
            serde_json::json!({"value": 1})
        );
    }

    #[test]
    fn registry_dispatches_first_divergence_through_the_existing_json_tool() {
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

        let output = registry
            .dispatch(
                "world.first-divergence",
                serde_json::json!({
                    "root": "event-2",
                    "direction": "upstream",
                    "window_depth": 1,
                    "max_depth": 1,
                }),
            )
            .unwrap();
        assert_eq!(output["divergence_depth"], 1);
        assert_eq!(
            output["witnesses"][0]["trace"],
            serde_json::json!(["event-2", "event-1"])
        );
    }

    #[test]
    fn output_is_serializable_without_exposing_executor_or_world_internals() {
        let output = FirstDivergenceToolOutput {
            root: "event-1".into(),
            direction: EvidenceCausalDirection::Downstream,
            max_depth: 2,
            identical_within_depth: true,
            divergence_depth: None,
            witnesses: vec![],
            truncated: false,
        };
        let value = serde_json::to_value(output).unwrap();
        assert_eq!(value["root"], "event-1");
        assert_eq!(value["direction"], "downstream");
        assert_eq!(value["identical_within_depth"], true);
        assert_eq!(value["divergence_depth"], serde_json::Value::Null);
    }
}
