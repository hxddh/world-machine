# Next Coding Task — M211 Read-Only Agent Tool Boundary

Expose the proven M209/M210 progressive investigation capability as a provider-neutral host-side agent tool without weakening the in-world `AgentRuntime` perception boundary.

## Current baseline

M209 provides `world-investigation`, a scheduler that depends only on public `world-query` DTOs. M210 proves a concrete local adapter can own `ProjectionSnapshot` while delegating all progressive search semantics through `ComparisonQueryExecutor`. The remaining gap is a reusable typed tool surface for external agent hosts.

## M211 — `world-agent-tools`

Add a new `world-agent-tools` crate with a read-only `world.first-divergence` tool.

The crate exposes:

- a stable `ReadOnlyToolDescriptor` with `read_only = true`;
- serializable `FirstDivergenceToolInput { root, direction, window_depth, max_depth }`;
- serializable `FirstDivergenceToolOutput` carrying the absolute M209 result;
- `FirstDivergenceTool<E>` parameterized only by the existing `ComparisonQueryExecutor` authority boundary;
- `invoke`, which maps the typed input to `investigate_first_divergence` and does not reimplement continuation replay, convergence, depth accounting, or trace composition.

## Boundary rules

- `world-agent-tools` production dependencies are limited to `serde`, `world-investigation`, and `world-query`.
- It must not depend on or name Projection/Core truth, `world-agent` / in-world `AgentRuntime`, GPUI, model-provider SDKs, or transport stacks.
- The tool executor is supplied by a host adapter. The tool itself has no archive, snapshot, filesystem, network, mutation, or model authority.
- This does not add tools to the in-world `AgentRuntime`; `AgentObservation` remains the only perception surface there.

## Compatibility

No change to evidence-query protocol v1, investigation envelope v1, CLI commands, Pack APIs, persistence formats, or AgentRuntime interfaces.

## Validation

- stable read-only descriptor and typed JSON input shape;
- progressive multi-window invocation proves reuse of M209 and preserves original-root witness traces;
- serializable output contains no executor or world-internal state;
- hard dependency/content boundary checks;
- `cargo fmt --all -- --check`;
- `cargo test -p world-agent-tools` and `cargo test -p world-investigation`;
- focused Clippy with warnings denied;
- full workspace CI, external Pack conformance, and macOS/GPUI validation when lockfile/workspace paths trigger it.

## Non-goals

No provider-specific SDK integration, no MCP/HTTP/WebSocket adapter, no in-world AgentRuntime query access, no `ProjectionSnapshot` exposure, no mutation tools, no server cursor/session, and no protocol v2.
