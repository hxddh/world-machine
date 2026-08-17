# Next Coding Task — M196 Causal Neighborhood Frontier

Make bounded causal neighborhoods explicitly report whether their visible upstream/downstream context was truncated by the requested depth and which included boundary Events can be expanded next.

## Current baseline

The machine causal investigation surface is complete through M195:

- M192: `why` upstream ancestry;
- M193: `influence` downstream traversal;
- M194: shortest `causal-path` plus one shared private `VisibleCausalGraph`;
- M195: bounded bidirectional `causal-neighborhood` with independent upstream/downstream depths;
- all causal queries use timeline-visible Events and persisted `caused_by`, separate from the state-evidence graph;
- JSON/stdin transport remains protocol `world-machine-evidence-query` v1.

## Product problem

A bounded M195 result currently tells a caller what was returned, but not whether the requested depth omitted additional visible causal context. An agent can therefore mistake a finite window for a complete explanation or influence history.

## M196 — frontier metadata

Extend `EvidenceCausalNeighborhoodResult` additively with:

- `upstream_truncated: bool`;
- `downstream_truncated: bool`;
- `upstream_frontier: Vec<String>`;
- `downstream_frontier: Vec<String>`.

Mark all four fields `#[serde(default)]` so a newer v1 client can still deserialize a response emitted by an M195-era v1 server.

## Frontier semantics

- A frontier entry is an Event already included at the requested depth boundary that has at least one additional timeline-visible neighbor in that direction which was not discovered inside the requested window.
- Frontier order follows the existing traversal order: persisted parent order/BFS upstream and `(world_time, SelectionId)` child order/BFS downstream.
- `*_truncated` is exactly whether the corresponding frontier is non-empty.
- At depth 0, the root itself is the frontier if that direction has additional visible context.
- Hidden Events never create frontier entries.
- Already-discovered neighbors, including cycle edges back into the window, do not count as truncation.
- When the requested window reaches all visible causal context in a direction, the frontier is empty and `*_truncated` is false.

## Compatibility

This remains protocol v1 because the response fields are additive. New fields must default on deserialization so old v1 payloads remain readable.

## Tests

Prove at minimum:

1. exact upstream/downstream frontier nodes at a one-hop boundary;
2. deeper complete windows clear truncation/frontier metadata;
3. depth 0 correctly uses the root as frontier when more context exists;
4. cycles do not produce false frontier after all visible neighbors are discovered;
5. hidden Events do not create frontier;
6. an M195-shaped response without the new fields still deserializes with safe defaults;
7. all M192–M195 causal tests and the M195 stdin subprocess test remain green.

## Validation

Before merge:

- `bash ./scripts/check-boundaries.sh`
- `cargo fmt --all -- --check`
- `cargo test -p world-query`
- `cargo test -p world-cli`
- focused Clippy with warnings denied
- semantic workspace CI and external Pack conformance
- macOS/GPUI only if dependency-path filtering requires it

## Non-goals for M196

Do not add pagination tokens, automatic recursive expansion, causal comparison between worlds, arbitrary graph export, MCP/HTTP/WebSocket, AgentRuntime access, raw mutation payloads, Pack-specific causal inference, or protocol v2.
