# Next Coding Task — M206 First-Divergence Continuation Consistency

Prove that M205 segmented `first-divergence` replay is semantically equivalent to a single deeper bounded query before building any higher-level search scheduler.

## Current baseline

M203 finds the earliest bounded causal divergence, M204 adds deterministic witness traces, and M205 emits typed replayable frontier continuations with explicit `depth_offset` values. The remaining risk is compositional: a caller may fan out across multiple frontier Events and replay several windows, so the protocol must not miss an earlier divergence, distort absolute depth, or lose same-depth witness branches.

## M206 — consistency invariants

Add a dedicated invariant suite that implements a small test-only segmented search scheduler using only public M205 DTOs and `execute_comparison_query_request`.

## Invariants

- `offset + replay.divergence_depth` must equal the corresponding divergence depth from a monolithic deeper query.
- When several frontier branches diverge at the same minimum absolute depth, the union of segmented witnesses must equal the monolithic witness set at that depth.
- A zero-depth bootstrap followed by promoted one-hop replay must converge to the same result as a monolithic query without non-progressing loops.
- Upstream and downstream segmented searches must both agree with monolithic semantics.
- Hidden referenced Events and visible causal cycles must not create false segmented divergence or infinite frontier replay.
- Duplicate continuation requests reached through converging branches may be de-duplicated by `(absolute offset, serialized request)` without changing the result.

## Scope

This milestone adds no production API surface. It is a proof milestone for the existing protocol-v1 continuation semantics and should exercise only public machine-query DTOs and executors.

## Validation

- `cargo fmt --all -- --check`
- `bash ./scripts/check-boundaries.sh`
- `cargo test -p world-query`
- focused Clippy with warnings denied
- full semantic workspace CI and external Pack conformance

## Non-goals

No production recursive scheduler, no new request or response type, no trace-prefix field, no server cursor/session, no MCP/HTTP/WebSocket, no AgentRuntime access, no arbitrary graph export, and no protocol v2.
