# Next Coding Task — M208 Multi-Window Divergence Trace Composition

Prove that M205 depth offsets and M207 trace prefixes compose together across repeated `first-divergence` continuation replay, so a segmented search preserves both the absolute divergence depth and the original-root witness explanation.

## Current baseline

M203 finds the earliest bounded causal divergence, M204 attaches deterministic witness traces, M205 emits replayable frontier continuations with `depth_offset`, M206 proves segmented depth and witness-set equivalence, and M207 adds deterministic root-to-frontier `trace_prefix` values. The remaining risk is multi-window composition: correct offsets and individually correct prefixes could still combine into a trace that differs from a single deeper query.

## M208 — composition invariants

Add a test-only segmented search scheduler that carries both an accumulated absolute offset and an accumulated original-root trace prefix. Use only public protocol-v1 DTOs and `execute_comparison_query_request`.

## Invariants

- Across three or more replay windows, accumulated `depth_offset + divergence_depth` equals the monolithic deeper-query divergence depth.
- Repeated `trace_prefix` composition followed by the replay witness trace reconstructs the exact monolithic M204 witness trace from the original root.
- Upstream and downstream traversal obey the same composition law.
- Parallel frontier branches that diverge at the same minimum absolute depth preserve the complete witness set and each witness keeps an original-root trace.
- A zero-depth bootstrap does not duplicate the root when trace prefixes are composed.
- Typed shortest-path selection remains stable across replay boundaries, including a diamond where multiple equal-length routes reach the same frontier.

## Scope

This milestone adds no production API surface. It proves the combined M205/M207 protocol semantics before any production-level recursive investigation adapter is introduced.

## Validation

- `cargo fmt --all -- --check`
- `bash ./scripts/check-boundaries.sh`
- `cargo test -p world-query`
- focused Clippy with warnings denied
- full semantic workspace CI and external Pack conformance

## Non-goals

No production recursive scheduler, no new request/response DTO, no automatic server-side trace assembly, no cursor/session state, no MCP/HTTP/WebSocket, no AgentRuntime access, no arbitrary graph export, and no protocol v2.
