from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    if old not in text:
        raise SystemExit(f"missing patch anchor: {label}")
    return text.replace(old, new, 1)


lib = Path("crates/world-agent-tools/src/lib.rs")
text = lib.read_text()
text = replace_once(
    text,
    '''pub fn first_divergence_json_tool_descriptor() -> ReadOnlyJsonToolDescriptor {\n    let descriptor = first_divergence_tool_descriptor();\n    ReadOnlyJsonToolDescriptor {\n        name: descriptor.name,\n        description: descriptor.description,\n        read_only: descriptor.read_only,\n        input_schema: first_divergence_input_schema(),\n    }\n}\n\n''',
    '''pub fn first_divergence_json_tool_descriptor() -> ReadOnlyJsonToolDescriptor {\n    let descriptor = first_divergence_tool_descriptor();\n    ReadOnlyJsonToolDescriptor {\n        name: descriptor.name,\n        description: descriptor.description,\n        read_only: descriptor.read_only,\n        input_schema: first_divergence_input_schema(),\n    }\n}\n\npub fn read_only_json_tool_catalog() -> Vec<ReadOnlyJsonToolDescriptor> {\n    let mut descriptors = vec![first_divergence_json_tool_descriptor()];\n    descriptors.sort_by(|left, right| left.name.cmp(right.name));\n    debug_assert!(\n        descriptors\n            .windows(2)\n            .all(|pair| pair[0].name != pair[1].name),\n        "read-only tool names must be unique"\n    );\n    descriptors\n}\n\n''',
    "tool catalog",
)

anchor = '''#[cfg(test)]\nmod tests {\n'''
insert = r'''#[derive(Debug)]
pub enum JsonToolDispatchError<E> {
    UnknownTool(String),
    Invocation {
        name: &'static str,
        source: JsonToolInvocationError<E>,
    },
}

impl<E: fmt::Display> fmt::Display for JsonToolDispatchError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownTool(name) => write!(f, "unknown read-only tool: {name}"),
            Self::Invocation { name, source } => write!(f, "read-only tool {name} failed: {source}"),
        }
    }
}

impl<E> Error for JsonToolDispatchError<E>
where
    E: Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::UnknownTool(_) => None,
            Self::Invocation { source, .. } => Some(source),
        }
    }
}

pub trait ReadOnlyJsonToolSet {
    type ExecutorError;

    fn descriptors(&self) -> Vec<ReadOnlyJsonToolDescriptor>;

    fn invoke_named_json(
        &mut self,
        name: &str,
        input: Value,
    ) -> Result<Value, JsonToolDispatchError<Self::ExecutorError>>;
}

pub struct WorldReadOnlyToolSet<E> {
    first_divergence: FirstDivergenceTool<E>,
}

impl<E> WorldReadOnlyToolSet<E> {
    pub fn new(executor: E) -> Self {
        Self {
            first_divergence: FirstDivergenceTool::new(executor),
        }
    }

    pub fn executor(&self) -> &E {
        self.first_divergence.executor()
    }

    pub fn executor_mut(&mut self) -> &mut E {
        self.first_divergence.executor_mut()
    }

    pub fn into_inner(self) -> E {
        self.first_divergence.into_inner()
    }
}

impl<E> ReadOnlyJsonToolSet for WorldReadOnlyToolSet<E>
where
    E: ComparisonQueryExecutor,
{
    type ExecutorError = E::Error;

    fn descriptors(&self) -> Vec<ReadOnlyJsonToolDescriptor> {
        read_only_json_tool_catalog()
    }

    fn invoke_named_json(
        &mut self,
        name: &str,
        input: Value,
    ) -> Result<Value, JsonToolDispatchError<Self::ExecutorError>> {
        match name {
            FIRST_DIVERGENCE_TOOL_NAME => self
                .first_divergence
                .invoke_json(input)
                .map_err(|source| JsonToolDispatchError::Invocation {
                    name: FIRST_DIVERGENCE_TOOL_NAME,
                    source,
                }),
            _ => Err(JsonToolDispatchError::UnknownTool(name.to_owned())),
        }
    }
}

'''
text = replace_once(text, anchor, insert + anchor, "tool set")

test_anchor = '''    #[test]\n    fn output_is_serializable_without_exposing_executor_or_world_internals() {\n'''
tests = r'''    #[test]
    fn read_only_catalog_is_deterministic_unique_and_read_only() {
        let catalog = read_only_json_tool_catalog();
        let names = catalog
            .iter()
            .map(|descriptor| descriptor.name)
            .collect::<Vec<_>>();
        assert_eq!(names, vec!["world.first-divergence"]);
        assert!(catalog.iter().all(|descriptor| descriptor.read_only));
        assert!(names.windows(2).all(|pair| pair[0] < pair[1]));
    }

    #[test]
    fn tool_set_lists_the_same_catalog_exposed_to_provider_adapters() {
        let tool_set = WorldReadOnlyToolSet::new(ScriptedExecutor::new(vec![]));
        assert_eq!(tool_set.descriptors(), read_only_json_tool_catalog());
    }

    #[test]
    fn tool_set_dispatches_known_name_through_existing_json_tool() {
        let direction = EvidenceCausalDirection::Upstream;
        let mut tool_set = WorldReadOnlyToolSet::new(ScriptedExecutor::new(vec![(
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

        let output = tool_set
            .invoke_named_json(
                FIRST_DIVERGENCE_TOOL_NAME,
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
        assert!(tool_set.executor().script.is_empty());
    }

    #[test]
    fn unknown_tool_name_fails_before_executor_use() {
        let mut tool_set = WorldReadOnlyToolSet::new(ScriptedExecutor::new(vec![]));
        let error = tool_set
            .invoke_named_json("world.mutate", serde_json::json!({}))
            .unwrap_err();
        assert!(matches!(
            error,
            JsonToolDispatchError::UnknownTool(name) if name == "world.mutate"
        ));
        assert!(tool_set.executor().script.is_empty());
    }

'''
text = replace_once(text, test_anchor, tests + test_anchor, "m213 tests")
lib.write_text(text)

Path("NEXT_TASK.md").write_text('''# Next Coding Task — M213 Provider-Neutral Read-Only Tool Set

Add deterministic tool discovery and name-based JSON dispatch on top of the M212 provider-neutral JSON tool contract so each future provider adapter can integrate the World tool surface once instead of wiring every tool independently.

## Current baseline

M211 introduces typed host-side `world.first-divergence`; M212 adds a provider-neutral JSON descriptor/schema and dynamic JSON invocation for that tool. A provider adapter can now register one tool, but it still needs tool-specific wiring. The next boundary is a deterministic read-only tool catalog and generic name dispatch.

## M213 — tool set/catalog

Extend `world-agent-tools` with:

- `read_only_json_tool_catalog()` returning all host-side read-only tool descriptors in stable name order;
- a uniqueness invariant for tool names;
- provider-neutral `ReadOnlyJsonToolSet` trait for discovery and name-based JSON invocation;
- `WorldReadOnlyToolSet<E>` that owns the shared `ComparisonQueryExecutor` authority and dispatches `world.first-divergence` through the existing M212 JSON tool;
- `JsonToolDispatchError` distinguishing unknown tool names from invocation failures.

## Dispatch semantics

- Unknown names fail before the executor is touched.
- Known names must call the existing JSON tool surface; no schema parsing or investigation logic is duplicated in the tool set.
- Catalog order is deterministic and all entries are explicitly read-only.
- The tool set exposes the underlying executor only as the same generic authority already accepted by M209–M212; it gains no additional capabilities.

## Boundary rules

No provider SDK, Projection/Core, in-world `world-agent`/AgentRuntime, filesystem, network, archive loading, or mutation authority enters `world-agent-tools`.

## Validation

- stable unique read-only catalog;
- tool-set discovery exactly matches the public catalog;
- known-name dispatch returns the same JSON output as M212;
- unknown-name dispatch never invokes the executor;
- existing typed and JSON tool tests remain green;
- boundary/fmt/focused Clippy, full workspace CI, external Pack conformance.

## Non-goals

No second World tool yet, no provider-specific adapter, no MCP/HTTP/WebSocket server, no in-world AgentRuntime injection, no mutation tools, and no protocol v2.
''')
