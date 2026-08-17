# Next Coding Task — M207 Composable First-Divergence Trace Prefixes

Preserve M204 witness explainability across M205 segmented replay by attaching a deterministic root-to-frontier trace prefix to every first-divergence continuation.

## Current baseline

M203 identifies the earliest bounded causal divergence, M204 gives each edge witness a deterministic directional trace, M205 makes the search resumable at frontier Events, and M206 proves segmented depth/witness semantics match monolithic deeper queries. The remaining composition gap is explanatory: a replayed witness trace begins at the continuation root rather than the original query root.

## M207 — continuation trace prefixes

Extend `EvidenceCausalFirstDivergenceContinuation` additively with `trace_prefix: Vec<String>` using `#[serde(default)]`.

## Semantics

- `trace_prefix` begins at the current request root and ends at the continuation frontier Event.
- Use the same directional traversal semantics as M204 witness traces.
- Restrict the path to Events already visible inside the current bounded neighborhood.
- Choose a shortest path; break equal-length alternatives by typed Event identity using the existing deterministic path helper.
- For a frontier present on both sides, either side must yield the same structural prefix because no divergence was found inside the current window; use a deterministic side choice.
- For one-sided frontier membership, derive the prefix from the side that owns the frontier.
- A zero-depth continuation has prefix `[root]`.
- To rebuild an original-root witness trace after replay, concatenate `trace_prefix` with the replay witness trace while dropping the replay trace's first Event, which is the shared frontier root.

## Compatibility

M205 continuation payloads without `trace_prefix` deserialize with an empty default. No request shape, CLI command, protocol version, server state, AgentRuntime authority, or transport changes.

## Tests

Prove upstream/downstream prefix composition against monolithic M204 traces, zero-depth behavior, typed shortest-path selection in a diamond, and backward M205 deserialization.

## Non-goals

No production recursive scheduler, no automatic trace concatenation API, no opaque cursor, no arbitrary graph export, no MCP/HTTP/WebSocket, no AgentRuntime access, and no protocol v2.
