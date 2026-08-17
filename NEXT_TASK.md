# Next Coding Task — M197 Causal Neighborhood Edges

Make bounded causal neighborhoods self-contained graph payloads by exporting all persisted visible causal edges induced by the Events already returned in the local window.

## Current baseline

The machine causal investigation surface is complete through M196:

- M192: upstream `why`;
- M193: downstream `influence`;
- M194: deterministic shortest `causal-path` and shared private `VisibleCausalGraph`;
- M195: bounded bidirectional `causal-neighborhood`;
- M196: explicit truncation/frontier metadata for bounded windows;
- causal visibility remains timeline-owned and separate from state-evidence adjacency;
- protocol remains `world-machine-evidence-query` v1.

## Product problem

M195/M196 return enough node metadata to infer some edges from each node's `caused_by`, but a consumer must still reconstruct the graph and decide which references are inside the bounded window. This is error-prone, especially for diamonds, cross-branch links, cycles, and duplicated persisted parent IDs.

## M197 — explicit induced causal edges

Extend `EvidenceCausalNeighborhoodResult` additively with:

```rust
#[serde(default)]
pub edges: Vec<EvidenceCausalEdge>
```

and add:

```rust
pub struct EvidenceCausalEdge {
    pub cause: String,
    pub effect: String,
}
```

The serde default preserves v1 backward deserialization for M196-era payloads.

## Edge semantics

- The returned Event set is the union of the root, upstream nodes, and downstream nodes.
- `edges` is the full induced directed causal subgraph over that Event set, not merely BFS traversal-tree edges.
- An edge exists only when the effect's persisted `caused_by` contains the cause and both endpoints are timeline-visible and included in the returned window.
- Hidden Events and visible Events outside the requested window never appear as edge endpoints.
- Duplicate persisted parent IDs emit one edge, keeping the first persisted parent position for ordering.
- Effects are ordered deterministically by `(world_time, SelectionId)` ascending; within an effect, causes preserve persisted visible parent order.
- Cycles and self-edges are represented faithfully when both endpoints are included.
- `EvidenceCausalNode.caused_by` remains unchanged and may still name visible causes outside the bounded window; `edges` is specifically the self-contained local graph.

## Internal implementation

Add a private `VisibleCausalGraph::induced_edges(included)` helper so edge filtering, deduplication, and ordering remain centralized with the causal graph semantics.

## Tests

Prove at minimum:

1. a bounded neighborhood exports all induced edges, including a cross-branch edge that was not needed by BFS discovery;
2. edge order is stable despite shuffled timeline input;
3. hidden and out-of-window Event endpoints do not leak;
4. duplicate persisted parent IDs produce one edge;
5. M196-shaped v1 payloads without `edges` deserialize with an empty default;
6. all M192–M196 causal tests and existing CLI subprocess tests remain green.

## Validation

Before merge:

- `bash ./scripts/check-boundaries.sh`
- `cargo fmt --all -- --check`
- `cargo test -p world-query`
- `cargo test -p world-cli`
- focused Clippy with warnings denied
- semantic workspace CI and external Pack conformance
- macOS/GPUI only if dependency-path filtering requires it

## Non-goals for M197

Do not export edges to omitted frontier neighbors, add arbitrary causal graph dumps, add causal comparison between worlds, change `caused_by`, add pagination, MCP/HTTP/WebSocket, AgentRuntime access, raw mutation payloads, Pack-specific causal inference, or protocol v2.
