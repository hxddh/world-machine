# Next Coding Task — M198 Executable Causal Continuations

Turn the M196 frontier metadata into directly executable continuation requests while preserving M197's self-contained induced-edge payloads.

## Current baseline

The machine causal investigation surface is complete through M197:

- M192: upstream `why`;
- M193: downstream `influence`;
- M194: deterministic shortest `causal-path` and shared private `VisibleCausalGraph`;
- M195: bounded bidirectional `causal-neighborhood`;
- M196: explicit truncation and stable frontier Events;
- M197: the full induced visible causal edge set for every bounded neighborhood;
- causal visibility remains timeline-owned and separate from state-evidence adjacency;
- JSON/stdin transport remains `world-machine-evidence-query` protocol v1.

## Product problem

M196/M197 tell a caller where a bounded causal window stops and give a complete local graph, but the caller still has to interpret direction and reconstruct a new request to continue. Every future agent or tool adapter would otherwise duplicate that protocol logic and could accidentally generate a zero-depth no-op.

## M198 — executable continuations

Extend `EvidenceCausalNeighborhoodResult` additively with:

- `upstream_continuations: Vec<EvidenceCausalContinuation>`;
- `downstream_continuations: Vec<EvidenceCausalContinuation>`.

Add:

- `EvidenceCausalDirection::{Upstream, Downstream}`;
- `EvidenceCausalContinuation { event, direction, request }`, where `request` is an ordinary `EvidenceQueryRequest` that can be serialized and passed directly back to the existing `evidence-query` transport.

Both continuation arrays use `#[serde(default)]` so M197-era protocol-v1 responses remain readable.

## Continuation semantics

- Emit exactly one continuation per frontier entry, in frontier order.
- Upstream continuations root at the frontier Event, set `downstream_depth = 0`, and preserve the caller's non-zero upstream window size.
- Downstream continuations are symmetric.
- If the original directional depth is `0`, promote the continuation window to `1`; a continuation must make progress.
- Each continuation query independently returns M197 induced edges for its own bounded window.
- Continuations carry no hidden state, visited set, opaque server token, mutation authority, or server-side session state.
- Separate continuation branches may overlap; stable Event keys and causal edges let callers deduplicate/merge windows deterministically.

## Tests

Prove at minimum:

1. exact typed upstream/downstream continuation requests;
2. emitted requests execute directly and reveal the next causal window;
3. depth-zero frontier continuations progress by one hop;
4. non-zero directional window sizes are preserved;
5. current M197 induced edges remain present before and after continuation;
6. an M197-shaped v1 payload with `edges` but no continuation fields deserializes with empty defaults;
7. a real two-step `world-cli` stdin subprocess can replay an emitted continuation request against the same `.world` file;
8. all M192–M197 causal tests remain green.

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

Do not add opaque pagination tokens, server-side continuation state, automatic recursive expansion, causal comparison between worlds, arbitrary graph export, MCP/HTTP/WebSocket, AgentRuntime access, raw mutation payloads, Pack-specific causal inference, or protocol v2.
