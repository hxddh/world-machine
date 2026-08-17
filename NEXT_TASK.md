# Next Coding Task — M197 Executable Causal Continuations

Turn M196 causal frontier metadata into directly executable continuation requests so an external investigator can advance through a large visible causal graph in bounded deterministic windows without reconstructing query arguments.

## Current baseline

The machine causal investigation surface is complete through M196:

- M192: `why` upstream ancestry;
- M193: `influence` downstream traversal;
- M194: shortest `causal-path` plus one shared private `VisibleCausalGraph`;
- M195: bounded bidirectional `causal-neighborhood`;
- M196: explicit truncation and stable upstream/downstream frontier Events;
- all causal semantics remain based only on timeline-visible Events and persisted `caused_by`, separate from state-evidence adjacency;
- JSON/stdin transport remains `world-machine-evidence-query` protocol v1.

## Product problem

M196 tells a caller exactly where a bounded causal window was cut off, but the caller still has to interpret direction, reconstruct a new `causal-neighborhood` request, and avoid generating a zero-depth no-op. That is unnecessary protocol logic for every future agent/tool adapter.

## M197 — executable continuations

Extend `EvidenceCausalNeighborhoodResult` additively with:

- `upstream_continuations: Vec<EvidenceCausalContinuation>`;
- `downstream_continuations: Vec<EvidenceCausalContinuation>`.

Add:

- `EvidenceCausalDirection::{Upstream, Downstream}`;
- `EvidenceCausalContinuation { event, direction, request }` where `request` is an ordinary `EvidenceQueryRequest` that can be serialized and passed directly back to the existing `evidence-query` machine transport.

Mark both continuation arrays `#[serde(default)]` so M196-era protocol-v1 responses remain deserializable.

## Continuation semantics

- There is exactly one continuation per frontier entry, in the same deterministic order as the corresponding frontier.
- An upstream continuation roots at that frontier Event, sets `downstream_depth = 0`, and preserves the caller's non-zero `upstream_depth` as the next window size.
- A downstream continuation roots at that frontier Event, sets `upstream_depth = 0`, and preserves the caller's non-zero `downstream_depth` as the next window size.
- If the original depth was `0`, the continuation depth is promoted to `1`; an executable continuation must always make progress.
- Continuations are suggestions over the same immutable visible ProjectionSnapshot. They do not carry hidden state, visited sets, opaque server tokens, or mutation authority.
- Overlap between separately expanded frontier branches is allowed; stable Event keys let the caller deduplicate across windows.

## Tests

Prove at minimum:

1. upstream and downstream frontier entries produce exact typed continuation requests;
2. each emitted request can be passed directly back to `execute_query` and reveals the next causal window;
3. depth-zero frontiers emit one-hop progressing continuations rather than no-ops;
4. non-zero window sizes are preserved across continuation generation;
5. an M196-shaped response without continuation fields deserializes with empty defaults;
6. a real `world-cli` stdin subprocess can take a continuation emitted by one machine query and replay it as the next machine query;
7. all M192–M196 causal tests remain green.

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

Do not add opaque pagination tokens, server-side continuation state, automatic recursive expansion, causal comparison between worlds, arbitrary graph export, MCP/HTTP/WebSocket, AgentRuntime access, raw mutation payloads, Pack-specific causal inference, or protocol v2.
