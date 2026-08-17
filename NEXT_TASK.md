# Next Coding Task — M199 Executable Causal Continuations

Turn causal frontier metadata into directly executable continuation requests while preserving both M197 neighborhood edges and M198 traversal-edge semantics.

## Current baseline

The machine causal investigation surface is complete through M198:

- M192: upstream `why`;
- M193: downstream `influence`;
- M194: deterministic shortest `causal-path` and shared private `VisibleCausalGraph`;
- M195: bounded bidirectional `causal-neighborhood`;
- M196: explicit truncation and stable frontier Events;
- M197: full induced visible causal edges for bounded neighborhoods;
- M198: self-contained induced edge payloads for `why` and `influence` traversals;
- causal visibility remains timeline-owned and separate from state-evidence adjacency;
- JSON/stdin transport remains `world-machine-evidence-query` protocol v1.

## M199 — executable continuations

Extend `EvidenceCausalNeighborhoodResult` additively with `upstream_continuations` and `downstream_continuations`, using typed `EvidenceCausalContinuation { event, direction, request }` values whose embedded `EvidenceQueryRequest::CausalNeighborhood` can be serialized and replayed directly through the existing machine transport.

Continuations preserve non-zero directional window sizes, promote zero-depth frontiers to a one-hop request so they always make progress, retain M197 induced edges in each continued window, and use `#[serde(default)]` so M198-era protocol-v1 responses remain readable.

## Tests

Prove exact typed requests, direct re-execution in both directions, zero-depth progress, preserved non-zero windows, coexistence with induced edges, backward deserialization from an edge-bearing payload without continuation fields, and a real two-step `world-cli` stdin replay against the same `.world` file.

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
