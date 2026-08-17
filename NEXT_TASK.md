# Next Coding Task — M193 Machine Causal Influence

Expose downstream causal influence through the existing machine query contract so external investigators can ask what visible Events were influenced by a visible root Event.

## Current baseline

The machine investigation surface is complete through M192:

- M173–M189: canonical machine evidence queries, comparison, JSON subprocess/stdin transport, stable semantic errors, and protocol v1 envelopes.
- M190: deterministic visible selection discovery.
- M191: display-safe structured selection detail with timeline-owned Event visibility.
- M192: `why` causal ancestry over visible persisted `TimelineItem.caused_by`, with hidden-cause filtering, cycle protection, and breadth-first minimum-depth semantics.

The workflow now supports `selections -> describe -> neighborhood / shortest-path / why`. M193 adds the downstream half of persisted causal investigation without merging causal edges into the state-evidence graph.

## Product goal

A caller should be able to ask:

```json
{"query":"influence","event":"event-42"}
```

and receive a deterministic visible downstream causal traversal rooted at that Event.

## Architecture boundary

1. `world-query` owns the machine influence DTO and traversal semantics.
2. Reuse the generic M192 `EvidenceCausalNode`; do not create a second almost-identical causal node shape.
3. `world-cli` remains thin JSON/subprocess transport; no new top-level command.
4. Derive influence only from visible `ProjectionSnapshot.timeline.items` and their persisted `caused_by` links.
5. Never traverse or export Events absent from the visible timeline.
6. Keep causal traversal separate from state-evidence adjacency and inspector visibility.
7. Keep protocol identity/version at `world-machine-evidence-query` v1; this is additive.
8. Do not expose the full projection to AgentRuntime.

## M193 — `influence` query

Extend `EvidenceQueryRequest` with:

```json
{"query":"influence","event":"event-42"}
```

Return `EvidenceInfluenceResult { event, nodes }` using `Vec<EvidenceCausalNode>`.

## Traversal rules

- Parse through the existing canonical stable-key boundary.
- Canonical entity/relation roots return the existing `SelectionKindMismatch { expected: event }` error.
- Root must be a timeline-visible Event or return `SelectionNotVisible`.
- Build child adjacency by reversing visible persisted `caused_by` edges only when both endpoints are visible timeline Events.
- Root depth is 0. Direct effects are depth 1. Use BFS so multiply reachable Events get minimum causal depth.
- Deduplicate and cycle-protect traversal.
- There is no persisted child vector, so direct children use stable `(world_time, SelectionId)` ascending order. This intentionally avoids coupling the machine contract to timeline presentation order.
- Each exported node retains its persisted `caused_by` order, filtered to visible timeline Events.

## Tests

Prove at minimum:

1. serialized `influence` request/response serde round-trip;
2. chain/branch/diamond traversal returns minimum BFS depth;
3. same-depth child order is deterministic by world time then typed selection ID, independent of input timeline ordering;
4. hidden referenced causes do not leak through exported node metadata or become adjacency roots;
5. cycles do not duplicate or loop;
6. canonical wrong-kind, malformed stable key, and invisible Event errors remain stable;
7. a real stdin `world-cli` subprocess emits the v1 typed influence response;
8. all M190–M192 query behavior remains green.

## Validation

Before merge:

- `bash ./scripts/check-boundaries.sh`
- `cargo fmt --all -- --check`
- `cargo test -p world-query`
- `cargo test -p world-cli`
- focused Clippy with warnings denied
- semantic workspace CI and external Pack conformance
- macOS/GPUI only if dependency-path filtering requires it

## Non-goals for M193

Do not add causal path-between-events queries, free-text search, HTTP/WebSocket/MCP, AgentRuntime access, raw World/Event mutation data, Pack-specific causal semantics, or protocol v2.
