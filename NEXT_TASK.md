# Next Coding Task — M201 Causal Neighborhood Comparison

Extend the existing two-world machine comparison transport so saved worlds and sibling futures can compare a bounded visible causal neighborhood without conflating it with the state-evidence graph.

## Current baseline

The machine causal surface is complete through M200:

- `why`, `influence`, deterministic `causal-path`;
- bounded bidirectional `causal-neighborhood` with truncation, frontiers, induced edges, and executable continuations;
- self-contained traversal edges for `why` / `influence`;
- cross-query invariant coverage proving all causal surfaces agree on one timeline-visible persisted causal graph;
- existing `evidence-compare-query` currently accepts the legacy untagged state-evidence request `{ root, max_depth }` and emits a raw `EvidenceComparisonResult` inside protocol-v1 status envelopes.

## M201 — bounded causal comparison

Preserve the legacy state-evidence comparison wire shape exactly, while extending `evidence-compare-query` with the tagged request:

`{ "query": "causal-neighborhood", "root": "event-N", "upstream_depth": U, "downstream_depth": D }`

A causal comparison is a structural comparison of the requested bounded visible causal window. Compare:

- Event membership and directional position (`is_root`, upstream depth, downstream depth);
- induced visible `cause -> effect` edges;
- canonical upstream/downstream frontier membership.

Do not compare display titles/subtitles or causal structure outside the requested window. Hidden referenced Event IDs remain invisible.

## Compatibility contract

- Introduce an untagged machine comparison request wrapper that accepts the legacy `{ root, max_depth }` shape unchanged and the new tagged causal request.
- Legacy requests must serialize and respond exactly as before; do not wrap old `EvidenceComparisonResult` in a new result tag.
- New causal responses use a tagged `result: "causal-neighborhood"` payload.
- Keep `world-machine-evidence-query` protocol version 1.
- Keep the human `evidence-compare` command and `execute_comparison_query` legacy API unchanged.

## Causal comparison semantics

- Root must be a canonical Event key.
- If the root is visible in neither world, return `SelectionNotVisibleInEitherWorld`.
- If visible in only one world, comparison succeeds and reports one-sided root/window differences.
- Node differences are typed `left-only`, `right-only`, or `changed`; `changed` means the same Event occupies a different bounded directional position/depth.
- Edge differences are set differences over induced visible causal edges.
- Frontier lists are canonicalized by typed Event identity before comparison so UI/traversal ordering cannot create false structural differences.
- `identical` means node positions, induced edges, and canonical frontier membership are all identical.

## Tests

Prove at minimum:

1. legacy request and response JSON shapes remain byte-structure compatible;
2. tagged causal request/response round-trip;
3. upstream/downstream node and edge divergence;
4. changed causal depth and cycle positions;
5. hidden references remain invisible while frontier differences remain semantic;
6. one-sided root success, neither-side error, kind mismatch, invalid stable key;
7. same-world comparison is identical;
8. a real stdin `world-cli evidence-compare-query` executes the tagged causal request through the existing v1 transport;
9. all M199/M200 causal consistency and continuation tests remain green.

## Validation

- `bash ./scripts/check-boundaries.sh`
- `cargo fmt --all -- --check`
- `cargo test -p world-query`
- `cargo test -p world-cli`
- focused Clippy with warnings denied
- semantic workspace CI and external Pack conformance
- macOS/GPUI only if dependency-path filtering requires it

## Non-goals

Do not compare arbitrary unbounded causal graphs, display metadata, state-evidence and causal graphs in one result, raw mutation payloads, AgentRuntime perception, MCP/HTTP/WebSocket, server-side comparison state, or protocol v2.
