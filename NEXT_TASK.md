# Next Coding Task — M212 Provider-Neutral JSON Tool Contract

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
