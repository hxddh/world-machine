# Next Coding Task — M200 Executable Causal Continuations

Make bounded causal investigation directly resumable by embedding executable continuation requests at every causal frontier, on top of the consistency guarantees established through M199.

## Current baseline

The machine causal investigation surface is complete through M199:

- M192: upstream `why`;
- M193: downstream `influence`;
- M194: deterministic shortest `causal-path` and shared private `VisibleCausalGraph`;
- M195: bounded bidirectional `causal-neighborhood`;
- M196: explicit truncation and stable frontier Events;
- M197: induced visible causal edges for bounded neighborhoods;
- M198: self-contained edge payloads for `why` and `influence` traversals;
- M199: cross-query invariant coverage proving these surfaces agree on one visible persisted causal graph;
- protocol remains `world-machine-evidence-query` v1.

## M200 — executable continuations

Extend `EvidenceCausalNeighborhoodResult` additively with `upstream_continuations` and `downstream_continuations`. Each `EvidenceCausalContinuation { event, direction, request }` embeds a normal `EvidenceQueryRequest::CausalNeighborhood` that can be serialized and replayed directly through the existing machine transport.

Emit one continuation per frontier entry in the same deterministic order. Preserve non-zero directional window sizes; promote zero-depth frontiers to a one-hop request so continuation always makes progress. Continued windows retain induced-edge semantics. Both arrays use `#[serde(default)]` to preserve protocol-v1 backward deserialization.

## Tests

Prove exact typed requests, direct re-execution upstream/downstream, zero-depth progress, preserved non-zero window size, induced-edge coexistence, backward deserialization from an edge-bearing payload without continuation fields, and a real two-step `world-cli` stdin replay against the same `.world` file. All M199 cross-query invariants must remain green.

## Validation

- `bash ./scripts/check-boundaries.sh`
- `cargo fmt --all -- --check`
- `cargo test -p world-query`
- `cargo test -p world-cli`
- focused Clippy with warnings denied
- semantic workspace CI and external Pack conformance
- macOS/GPUI only if dependency-path filtering requires it

## Non-goals

Do not add opaque pagination tokens, server-side continuation state, automatic recursive expansion, causal comparison between worlds, arbitrary graph export, MCP/HTTP/WebSocket, AgentRuntime access, raw mutation payloads, Pack-specific causal inference, or protocol v2.
