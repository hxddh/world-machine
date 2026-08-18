# Next Coding Task — M213 Provider-Neutral Read-Only Tool Set

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
