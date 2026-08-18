from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    if old not in text:
        raise SystemExit(f"missing patch anchor: {label}")
    return text.replace(old, new, 1)


cargo = Path("crates/world-agent-tools/Cargo.toml")
text = cargo.read_text()
text = replace_once(
    text,
    '''[dependencies]\nserde = { version = "1", features = ["derive"] }\nworld-investigation = { path = "../world-investigation" }\nworld-query = { path = "../world-query" }\n\n[dev-dependencies]\nserde_json = "1"\n''',
    '''[dependencies]\nserde = { version = "1", features = ["derive"] }\nserde_json = "1"\nworld-investigation = { path = "../world-investigation" }\nworld-query = { path = "../world-query" }\n''',
    "serde-json production dependency",
)
cargo.write_text(text)

lib = Path("crates/world-agent-tools/src/lib.rs")
text = lib.read_text()
text = replace_once(
    text,
    'use serde::{Deserialize, Serialize};\n',
    'use serde::{Deserialize, Serialize};\nuse serde_json::Value;\nuse std::error::Error;\nuse std::fmt;\n',
    "json imports",
)
text = replace_once(
    text,
    '''pub fn first_divergence_tool_descriptor() -> ReadOnlyToolDescriptor {\n    ReadOnlyToolDescriptor {\n        name: FIRST_DIVERGENCE_TOOL_NAME,\n        description: "Find the earliest visible causal divergence between two World histories within a bounded depth and return original-root witness traces.",\n        read_only: true,\n    }\n}\n\n#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]\npub struct FirstDivergenceToolInput {\n''',
    '''pub fn first_divergence_tool_descriptor() -> ReadOnlyToolDescriptor {\n    ReadOnlyToolDescriptor {\n        name: FIRST_DIVERGENCE_TOOL_NAME,\n        description: "Find the earliest visible causal divergence between two World histories within a bounded depth and return original-root witness traces.",\n        read_only: true,\n    }\n}\n\n#[derive(Clone, Debug, PartialEq, Serialize)]\npub struct ReadOnlyJsonToolDescriptor {\n    pub name: &'static str,\n    pub description: &'static str,\n    pub read_only: bool,\n    pub input_schema: Value,\n}\n\npub fn first_divergence_input_schema() -> Value {\n    serde_json::json!({\n        "type": "object",\n        "additionalProperties": false,\n        "properties": {\n            "root": {\n                "type": "string",\n                "description": "Canonical visible Event selection key such as event-7."\n            },\n            "direction": {\n                "type": "string",\n                "enum": ["upstream", "downstream"]\n            },\n            "window_depth": {\n                "type": "integer",\n                "minimum": 1\n            },\n            "max_depth": {\n                "type": "integer",\n                "minimum": 0\n            }\n        },\n        "required": ["root", "direction", "window_depth", "max_depth"]\n    })\n}\n\npub fn first_divergence_json_tool_descriptor() -> ReadOnlyJsonToolDescriptor {\n    let descriptor = first_divergence_tool_descriptor();\n    ReadOnlyJsonToolDescriptor {\n        name: descriptor.name,\n        description: descriptor.description,\n        read_only: descriptor.read_only,\n        input_schema: first_divergence_input_schema(),\n    }\n}\n\n#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]\n#[serde(deny_unknown_fields)]\npub struct FirstDivergenceToolInput {\n''',
    "json descriptor and strict input",
)

anchor = '''#[cfg(test)]\nmod tests {\n'''
insert = r'''#[derive(Debug)]
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

'''
text = replace_once(text, anchor, insert + anchor, "json tool trait")

# Insert M212 tests before the existing output serialization test so helpers stay shared.
test_anchor = '''    #[test]\n    fn output_is_serializable_without_exposing_executor_or_world_internals() {\n'''
tests = r'''    #[test]
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

'''
text = replace_once(text, test_anchor, tests + test_anchor, "m212 tests")
lib.write_text(text)

Path("NEXT_TASK.md").write_text('''# Next Coding Task — M212 Provider-Neutral JSON Tool Contract

Turn the M211 typed read-only investigation tool into a dynamic JSON contract that any external Agent SDK adapter can register without importing World internals or duplicating investigation semantics.

## Current baseline

M209 owns progressive first-divergence orchestration, M210 proves a concrete CLI executor adapter, and M211 adds host-side typed `world.first-divergence` with no in-world `AgentRuntime` or Projection access. The remaining integration gap is the common shape expected by practical Agent SDKs: a tool descriptor with an input schema plus dynamic JSON invocation.

## M212 — JSON tool contract

Extend `world-agent-tools` with:

- serializable `ReadOnlyJsonToolDescriptor`;
- deterministic JSON Schema for `world.first-divergence` input;
- `ReadOnlyJsonTool` trait with provider-neutral `json_descriptor` and `invoke_json`;
- strict JSON input decoding into the existing typed `FirstDivergenceToolInput`;
- JSON output encoding from the existing typed `FirstDivergenceToolOutput`;
- `JsonToolInvocationError` that distinguishes malformed tool input, investigation/executor failures, and output serialization failures.

## Schema semantics

The input schema is an object with no additional properties. It requires `root`, `direction`, `window_depth`, and `max_depth`; direction is `upstream|downstream`, window depth has minimum 1, and maximum investigation depth has minimum 0.

The schema describes the transport contract only. Canonical Event visibility and causal semantics remain validated by the existing machine-query/investigation layers.

## Boundary rules

- No provider SDK types or names enter the tool contract.
- JSON dispatch delegates to typed `invoke`, which delegates to M209; no investigation logic is reimplemented.
- The tool still owns no Projection, archive, filesystem, network, model, or mutation authority.
- The in-world `AgentRuntime` remains unchanged and does not gain this tool automatically.

## Validation

- stable serializable descriptor and deterministic input schema;
- valid JSON dispatch reaches the typed M211/M209 path and returns original-root witnesses;
- invalid direction / unknown fields fail before the executor is used;
- existing typed API remains compatible;
- boundary check, fmt, focused tests/Clippy, full workspace CI and Pack conformance.

## Non-goals

No provider-specific adapter yet, no generic multi-tool registry yet, no MCP/HTTP/WebSocket server, no in-world AgentRuntime tool injection, no mutation tools, and no protocol v2.
''')
