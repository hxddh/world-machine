# Next Coding Task — M194 Machine Causal Path

Add a stable shortest causal path query between two visible Events and consolidate M192/M193 causal traversal onto one private visible-causal-graph primitive inside `world-query`.

## Current baseline

The machine investigation surface is complete through M193:

- state-evidence discovery, describe, neighborhood, shortest path, and comparison are already machine-readable;
- M192 adds upstream `why` traversal over visible persisted `caused_by` links;
- M193 adds downstream `influence` traversal with BFS minimum depth and deterministic `(world_time, SelectionId)` child ordering;
- CLI transport remains generic `evidence-query` JSON/stdin with protocol v1.

## Product goal

A caller should be able to ask:

```json
{"query":"causal-path","from":"event-1","to":"event-42"}
```

and receive one deterministic shortest visible causal route from cause to effect.

## Architecture boundary

1. `world-query` owns the private visible causal graph and path semantics.
2. Refactor `why` and `influence` to use the same graph helper so visibility/filtering/order cannot drift.
3. The graph is derived only from visible `ProjectionSnapshot.timeline.items` and persisted `caused_by` links.
4. Do not merge causal edges into the state-evidence graph.
5. Do not make inspector-only Events visible.
6. Reuse `EvidenceCausalNode` and protocol v1.
7. Keep `world-cli` transport-only; no new top-level command.

## M194 — `causal-path`

Add request:

```json
{"query":"causal-path","from":"event-1","to":"event-42"}
```

Return `EvidenceCausalPathResult { from, to, nodes }` with path nodes ordered source-to-target and path-relative depths 0..N.

## Path rules

- Both endpoints pass the existing canonical stable-key parser.
- Both endpoints must be timeline-visible Events; canonical wrong kinds use `SelectionKindMismatch`, invisible Events use `SelectionNotVisible`.
- Traverse only downstream `cause -> effect` edges where both endpoints are visible.
- Use BFS for shortest edge count.
- Equal-length paths are resolved by the same deterministic child ordering as M193: `(world_time, SelectionId)` ascending.
- `from == to` returns a one-node path at depth 0.
- If no visible downstream path exists, return stable `NoCausalPath { from, to }`.
- Hidden intermediate Events must never bridge a path.

## Internal causal graph

Introduce a private helper in `world-query` that owns:

- visible Event lookup;
- filtered persisted parent order;
- deterministic downstream children;
- visible `EvidenceCausalNode` materialization.

`why`, `influence`, and `causal-path` must all use it.

## Tests

At minimum prove:

1. request/response serde round-trip;
2. deterministic shortest-path tie-break through a diamond;
3. identity path returns one node;
4. reverse direction returns `NoCausalPath`;
5. hidden intermediate Events cannot create a path;
6. both endpoint validation paths remain stable;
7. `NoCausalPath` has pinned serialized shape;
8. existing M192/M193 tests remain green after refactor;
9. true stdin `world-cli` subprocess emits a v1 typed causal-path response.

## Validation

Before merge:

- `bash ./scripts/check-boundaries.sh`
- `cargo fmt --all -- --check`
- `cargo test -p world-query`
- `cargo test -p world-cli`
- focused Clippy with warnings denied
- semantic workspace CI and external Pack conformance
- macOS/GPUI only if dependency-path filtering requires it

## Non-goals for M194

Do not add arbitrary causal subgraph export, causal comparison between worlds, HTTP/WebSocket/MCP, AgentRuntime access, raw mutation payloads, Pack-specific causal inference, or protocol v2.
