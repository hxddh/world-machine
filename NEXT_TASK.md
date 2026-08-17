# Next Coding Task — M205 Executable First-Divergence Continuations

Make bounded two-world `first-divergence` search directly resumable at every unresolved causal frontier without adding server-side state.

## Current baseline

M203 locates the earliest visible causal divergence in one direction and M204 attaches deterministic root-to-witness traces. A bounded search can still end with `identical_within_depth = true` while one or both worlds expose a frontier, which currently leaves the caller to construct follow-up searches manually.

## M205 — first-divergence continuations

Extend `EvidenceCausalFirstDivergenceResult` additively with `continuations: Vec<EvidenceCausalFirstDivergenceContinuation>` using `#[serde(default)]` compatibility.

Each continuation carries the canonical frontier Event, direction, left/right frontier membership, a `depth_offset`, and an ordinary replayable `EvidenceComparisonQueryRequest::Causal(FirstDivergence { ... })`.

## Semantics

- Emit continuations only when no divergence was found in the current bounded window.
- Build one continuation per Event in the typed union of left/right frontiers.
- Preserve whether each frontier belongs to the left world, right world, or both.
- For a non-zero window, re-root at the frontier and preserve that window size.
- For a zero-depth window, reuse the current root but promote replay to one hop so it always makes progress.
- `depth_offset` is the distance from the current request root to the continuation root. Add it to a replay response's relative `divergence_depth` to map the result back to the current request root; sum offsets across repeated replays.
- A one-sided frontier remains executable because first-divergence already supports roots visible in either world.
- Stop emitting deeper continuations as soon as a divergence is found; the earliest divergence is already resolved for that branch.

## Compatibility

No request-shape change, protocol bump, CLI command, cursor, visited set, server session, AgentRuntime access, or transport. Existing M204 result payloads without `continuations` deserialize with an empty default.

## Tests

Prove side-aware frontier replay, depth-offset arithmetic, zero-depth progress, typed continuation ordering, suppression after divergence, backward M204 deserialization, and a real two-step stdin `world-cli evidence-compare-query` replay.

## Non-goals

No automatic global recursive search scheduler, opaque cursor, arbitrary graph export, MCP/HTTP/WebSocket, mutation authority, Pack-specific inference, protocol v2, or unrestricted AgentRuntime projection access.
