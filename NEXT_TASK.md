# Next Coding Task — M209 Read-Only Investigation Orchestrator

Turn the now-proven M203–M208 machine-query continuation semantics into a reusable orchestration boundary without exposing `ProjectionSnapshot` or embedding transport/provider concerns in `world-query` or `world-agent`.

## Current baseline

M203–M205 provide bounded first-divergence search, witness traces, and replayable continuations. M206–M208 prove that segmented replay preserves absolute divergence depth, witness sets, original-root traces, zero-depth progress, typed tie-breaking, and canonical convergence semantics. External callers can therefore orchestrate investigation safely, but each caller would otherwise have to reimplement the scheduler.

## M209 — `world-investigation`

Add a new `world-investigation` crate whose production dependency is only `world-query`.

The crate exposes:

- a provider-neutral `ComparisonQueryExecutor` trait that accepts ordinary `EvidenceComparisonQueryRequest` DTOs;
- `FirstDivergenceInvestigationRequest { root, direction, window_depth, max_depth }`;
- `investigate_first_divergence`, which incrementally replays first-divergence continuations until it finds the globally earliest divergence or exhausts the requested depth budget;
- a result carrying absolute divergence depth, original-root witness traces, bounded identity, and whether unexplored causal frontier remains beyond `max_depth`.

## Boundary rules

- `world-investigation` must not depend on or name `world-projection`, `ProjectionSnapshot`, `world-core`, `world-agent`, GPUI, model providers, or transport stacks.
- The executor is the only authority boundary. CLI, local in-memory adapters, MCP, or future Agent tools may implement it later without changing the orchestration semantics.
- Validate that returned continuations are self-consistent first-divergence requests before replaying them.
- Require M207 trace prefixes for cross-window original-root explanations; do not silently invent missing paths.
- Keep canonical convergence keyed by `(absolute offset, continuation Event)` so equal search states execute once and retain deterministic typed-order explanation prefixes.
- Retarget the final replay window to the remaining depth budget instead of overscanning past `max_depth`.

## Compatibility

No change to `world-machine-evidence-query` protocol v1, existing request/response DTOs, `world-cli`, AgentRuntime, Pack APIs, or persistence formats. This is orchestration over the existing public machine-query boundary.

## Tests

Prove three-window trace/depth composition, partial final windows, converging-diamond state dedup and canonical traces, bounded truncation vs exhausted identity, invalid zero-size windows, executor error preservation, and unexpected-response rejection.

## Non-goals

No CLI integration yet, no AgentRuntime tool exposure yet, no MCP/HTTP/WebSocket, no server cursor/session, no mutation authority, no arbitrary graph export, and no protocol v2.
