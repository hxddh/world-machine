# Next Coding Task — M203 First Causal Divergence

Locate the earliest visible causal structural divergence between two persisted worlds without exposing raw World state or expanding the protocol surface beyond the existing comparison transport.

## Current baseline

M192–M202 provide a deterministic visible causal graph, single-world traversal/path/neighborhood queries, induced edges, frontiers, executable continuations, structural two-world causal-neighborhood comparison, and replayable comparison continuations through `world-cli evidence-compare-query` protocol v1.

## M203 — first divergence

Add an additive causal comparison request:

`{"query":"first-divergence","root":"event-N","direction":"upstream|downstream","max_depth":D}`

The response identifies the minimum directional depth at which the two visible causal graphs differ and returns every differing visible causal edge at that earliest depth as a deterministic witness set.

## Semantics

- Validate the root with the same Event-only comparison contract used by causal-neighborhood comparison.
- A root visible in only one world is an immediate depth-0 `root-presence` divergence.
- Otherwise traverse only the requested direction and compare induced visible causal edges within `max_depth`.
- Define an edge's divergence depth as the maximum directional BFS depth of its two endpoints, with the root at depth 0.
- Return only witnesses at the minimum differing depth; do not mix later differences into the first-divergence answer.
- Sort same-depth witnesses by typed `(cause EventId, effect EventId, side)` order rather than lexical stable-key order.
- Hidden referenced causes remain invisible and cannot produce witnesses.
- `identical_within_depth=true` means only that no structural divergence is visible inside the requested bound. Return left/right frontiers so callers can distinguish a bounded answer from a globally exhausted graph.

## Compatibility

- Add request/response enum variants only; preserve legacy state-evidence comparison and M201/M202 causal-neighborhood wire shapes exactly.
- Reuse `world-cli evidence-compare-query`; no new command or transport.
- Keep `world-machine-evidence-query` at protocol version 1.

## Tests

Prove downstream and upstream first divergence, root-presence depth 0, deterministic typed witness ordering, bounded identical/frontier semantics, hidden-reference filtering, stable root errors, tagged serde, and real stdin CLI transport.

## Non-goals

No global unbounded auto-search, opaque cursor, recursive server state, arbitrary graph export, AgentRuntime access, raw mutation payloads, MCP/HTTP/WebSocket, Pack-specific inference, or protocol v2.
