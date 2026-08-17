# Next Coding Task — M198 Self-Contained Causal Traversals

Make the unbounded `why` and `influence` machine results self-contained causal graph payloads by reusing the explicit induced-edge semantics introduced for bounded neighborhoods in M197.

## Current baseline

The causal machine-query surface is complete through M197:

- M192: upstream `why`;
- M193: downstream `influence`;
- M194: deterministic shortest `causal-path` plus shared private `VisibleCausalGraph`;
- M195: bounded bidirectional `causal-neighborhood`;
- M196: frontier/truncation metadata;
- M197: explicit full induced causal `edges` for the bounded neighborhood;
- all visibility comes from timeline-visible Events and persisted `caused_by` links;
- protocol remains `world-machine-evidence-query` v1.

## Product problem

`why` and especially `influence` still expose only nodes plus each node's raw visible `caused_by` list. For an influence closure, a returned Event may have an additional visible co-cause that is not reachable from the requested root. A consumer must therefore infer which references belong to the returned graph and which are external context.

## M198 — traversal edges

Extend both result DTOs additively:

```rust
pub struct EvidenceWhyResult {
    pub event: String,
    pub nodes: Vec<EvidenceCausalNode>,
    #[serde(default)]
    pub edges: Vec<EvidenceCausalEdge>,
}

pub struct EvidenceInfluenceResult {
    pub event: String,
    pub nodes: Vec<EvidenceCausalNode>,
    #[serde(default)]
    pub edges: Vec<EvidenceCausalEdge>,
}
```

Use the M197 `VisibleCausalGraph::induced_edges` helper over each traversal's discovered Event set.

## Semantics

- `why.edges` is the full induced graph over the visible upstream ancestry closure including the root.
- `influence.edges` is the full induced graph over the visible downstream descendant closure including the root.
- A visible external co-cause may remain in an `EvidenceCausalNode.caused_by` list for context but must not appear as an edge unless that Event is itself in the traversal closure.
- Edge order and duplicate-parent handling are exactly M197 semantics; do not introduce query-specific ordering.
- Hidden Events never appear as edge endpoints.
- The fields are additive with `#[serde(default)]` so M192/M193-era v1 responses remain readable by newer clients.
- `causal-path` remains a path, not an induced subgraph, and is intentionally unchanged.

## Tests

Prove at minimum:

1. `why` returns the full visible ancestry induced graph;
2. `influence` excludes a visible external co-cause from `edges` while preserving it in node `caused_by` context;
3. edge ordering is stable despite shuffled timeline input;
4. legacy v1 `why` and `influence` responses without `edges` deserialize with empty defaults;
5. all M192–M197 causal tests and existing CLI subprocess tests remain green.

## Validation

Before merge:

- `bash ./scripts/check-boundaries.sh`
- `cargo fmt --all -- --check`
- `cargo test -p world-query`
- `cargo test -p world-cli`
- focused Clippy with warnings denied
- semantic workspace CI and external Pack conformance
- macOS/GPUI only if dependency-path filtering requires it

## Non-goals for M198

Do not change `causal-path`, expose omitted neighbors as edges, add arbitrary graph dumps, add causal comparison between worlds, add pagination, MCP/HTTP/WebSocket, AgentRuntime access, raw mutation payloads, Pack-specific causal inference, or protocol v2.
