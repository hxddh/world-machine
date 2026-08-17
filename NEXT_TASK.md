# Next Coding Task — M195 Machine Causal Neighborhood

Expose a bounded bidirectional causal context query so external investigators can inspect the visible causes and visible effects around one Event in a single machine request.

## Current baseline

The machine investigation surface is complete through M194:

- M190–M191: visible selection discovery and display-safe describe;
- state-evidence neighborhood / shortest-path / comparison remain available as a separate graph family;
- M192: unbounded upstream `why` over visible persisted causal links;
- M193: unbounded downstream `influence` with deterministic child ordering;
- M194: deterministic shortest `causal-path`, plus one private `VisibleCausalGraph` shared by all causal queries;
- generic JSON/stdin CLI transport remains protocol `world-machine-evidence-query` v1.

## Product goal

A caller should be able to ask:

```json
{
  "query":"causal-neighborhood",
  "root":"event-42",
  "upstream_depth":2,
  "downstream_depth":2
}
```

and receive one bounded local causal context without issuing and reconciling separate `why` and `influence` requests.

## Architecture boundary

1. Implement only in `world-query`; `world-cli` remains generic transport.
2. Reuse the M194 private `VisibleCausalGraph` and `EvidenceCausalNode`.
3. Read only timeline-visible Events and persisted `TimelineItem.caused_by` links.
4. Keep causal traversal separate from state-evidence adjacency and inspector visibility.
5. Do not expose ProjectionSnapshot to AgentRuntime.
6. Keep protocol v1; the new request/response is additive.

## M195 — `causal-neighborhood`

Add request:

```json
{"query":"causal-neighborhood","root":"event-42","upstream_depth":2,"downstream_depth":2}
```

Return `EvidenceCausalNeighborhoodResult` with:

- `root: EvidenceCausalNode` at depth 0;
- the requested `upstream_depth` and `downstream_depth`;
- `upstream: Vec<EvidenceCausalNode>` excluding the root;
- `downstream: Vec<EvidenceCausalNode>` excluding the root.

## Traversal rules

- Root uses the existing canonical stable-key parser and timeline-visible Event validation.
- Upstream and downstream depth limits are independent; zero disables that side.
- Upstream traversal is BFS, preserving each Event's persisted visible parent order.
- Downstream traversal is BFS, preserving M193/M194 `(world_time, SelectionId)` child order.
- Depth is minimum causal edge distance from root in that direction.
- Each direction deduplicates independently and cycle-protects with the root pre-discovered.
- In an actual causal cycle, the same non-root Event may legitimately appear once in each direction; direction is represented by membership in `upstream` versus `downstream`.
- `EvidenceCausalNode.caused_by` keeps its existing contract: persisted order filtered to timeline-visible Events, even if a referenced visible cause lies outside the requested depth window.
- Hidden Events never appear in traversal or `caused_by` metadata.

## Tests

Prove at minimum:

1. request/response serde round-trip;
2. independent upstream/downstream bounds;
3. persisted upstream order and stable downstream order;
4. minimum BFS depth through branching;
5. zero-depth sides return no contextual nodes while retaining the root;
6. hidden cause IDs are filtered;
7. cycles do not duplicate the root or loop;
8. canonical wrong-kind, malformed key, and invisible Event errors remain stable;
9. existing M192–M194 causal tests remain green;
10. a real stdin `world-cli` subprocess emits the v1 typed causal-neighborhood response.

## Validation

Before merge:

- `bash ./scripts/check-boundaries.sh`
- `cargo fmt --all -- --check`
- `cargo test -p world-query`
- `cargo test -p world-cli`
- focused Clippy with warnings denied
- semantic workspace CI and external Pack conformance
- macOS/GPUI only if dependency-path filtering requires it

## Non-goals for M195

Do not add causal graph comparison between worlds, arbitrary graph export, search/filter, pagination, HTTP/WebSocket/MCP, AgentRuntime access, raw mutation payloads, Pack-specific causal inference, or protocol v2.
