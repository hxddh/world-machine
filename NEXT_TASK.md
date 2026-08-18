# Next Coding Task — M213 Provider-Neutral Multi-Tool Registry

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
