from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    if old not in text:
        raise SystemExit(f"missing marker for {label}")
    return text.replace(old, new, 1)


lib_path = Path("crates/world-agent-tools/src/lib.rs")
text = lib_path.read_text()
text = replace_once(
    text,
    "use serde_json::Value;\nuse std::error::Error;",
    "use serde_json::Value;\nuse std::collections::BTreeMap;\nuse std::error::Error;",
    "BTreeMap import",
)

registry = r'''
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReadOnlyJsonToolRegistryError {
    DuplicateTool { name: String },
}

impl fmt::Display for ReadOnlyJsonToolRegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateTool { name } => write!(f, "read-only JSON tool is already registered: {name}"),
        }
    }
}

impl Error for ReadOnlyJsonToolRegistryError {}

#[derive(Debug)]
pub enum JsonToolDispatchError<E> {
    UnknownTool { name: String },
    Invocation {
        tool: String,
        source: JsonToolInvocationError<E>,
    },
}

impl<E: fmt::Display> fmt::Display for JsonToolDispatchError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownTool { name } => write!(f, "unknown read-only JSON tool: {name}"),
            Self::Invocation { tool, source } => write!(f, "read-only JSON tool {tool} failed: {source}"),
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
        self.tools.get(name).map(|registered| &registered.descriptor)
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
        let registered = self
            .tools
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

'''
text = replace_once(
    text,
    "\n#[cfg(test)]\nmod tests {",
    "\n" + registry + "#[cfg(test)]\nmod tests {",
    "registry insertion",
)

helper = r'''
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

'''
text = replace_once(
    text,
    "    fn request(\n",
    helper + "    fn request(\n",
    "test helper insertion",
)

registry_tests = r'''
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
            registry.dispatch("world.static", serde_json::json!({})).unwrap(),
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
            registry.dispatch("world.static", serde_json::json!({})).unwrap(),
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

'''
text = replace_once(
    text,
    "    #[test]\n    fn output_is_serializable_without_exposing_executor_or_world_internals() {",
    registry_tests + "    #[test]\n    fn output_is_serializable_without_exposing_executor_or_world_internals() {",
    "registry tests insertion",
)
lib_path.write_text(text)

Path("NEXT_TASK.md").write_text(r'''# Next Coding Task — M213 Provider-Neutral Multi-Tool Registry

Turn the M212 provider-neutral JSON tool contract into a deterministic host-side registry that can expose and dispatch multiple read-only World tools without introducing provider SDKs or weakening World authority boundaries.

## Current baseline

M209 owns progressive first-divergence orchestration, M210 provides a local CLI executor adapter, M211 introduces the host-side typed `world.first-divergence` tool, and M212 adds provider-neutral JSON descriptor/schema plus dynamic invocation. The remaining host integration gap is a stable collection boundary: an Agent host needs to enumerate tools once and dispatch tool calls by stable name.

## M213 — read-only JSON tool registry

Extend `world-agent-tools` with `ReadOnlyJsonToolRegistry<E>`.

The registry:

- accepts any `'static` `ReadOnlyJsonTool<ExecutorError = E>`;
- freezes each tool descriptor at registration time;
- stores tools by stable name in a `BTreeMap` so descriptor enumeration is deterministic;
- rejects duplicate tool names instead of replacing an existing tool;
- provides exact-name descriptor lookup and JSON dispatch;
- distinguishes `UnknownTool` from a named tool invocation failure while preserving the typed `JsonToolInvocationError<E>` source.

## Error-type boundary

A registry intentionally has one host-normalized executor error type `E`. This preserves typed invocation errors instead of erasing them to strings. Provider or transport adapters that combine multiple authority sources can normalize their own underlying errors into one host error before tool registration.

## Boundary rules

- Registry membership is host configuration only; registry mutation never mutates a World.
- No provider SDK, MCP/HTTP/WebSocket, Projection, archive, filesystem, network, model, or World mutation authority enters `world-agent-tools`.
- Dispatch delegates to each M212 `ReadOnlyJsonTool`; `world.first-divergence` still delegates through M211 to M209.
- The in-world `AgentRuntime` and `AgentObservation` surfaces remain unchanged.

## Validation

- deterministic lexicographic descriptor enumeration independent of registration order;
- descriptor schema is frozen and available by exact name;
- duplicate registration fails and does not replace the original tool;
- unknown tool dispatch is distinct from invocation failure;
- real `world.first-divergence` registry dispatch preserves its JSON result and witness trace;
- boundary check, fmt, focused tests/Clippy, full workspace CI and external Pack conformance.

## Non-goals

No provider-specific adapter yet, no cross-error type erasure, no mutable World tools, no generic network service, no protocol v2, and no automatic injection into the in-world `AgentRuntime`.
''')
