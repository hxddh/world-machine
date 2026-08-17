# Next Coding Task — M204 First-Divergence Traces

Make each M203 first-divergence edge witness self-explanatory by attaching a deterministic directional Event trace from the comparison root to that exact differing causal edge.

## Current baseline

M203 adds `first-divergence` over the existing protocol-v1 `evidence-compare-query` transport. It reports the earliest bounded structural divergence layer and every left/right-only causal edge at that depth, with typed witness ordering and frontier-aware bounded identity.

## M204 — witness traces

Extend the `edge` form of `EvidenceCausalDivergenceWitness` additively with:

- `trace: Vec<String>` using `#[serde(default)]`.

The trace is a canonical directional traversal beginning at the requested root and ending by traversing the witness edge itself.

## Semantics

- Downstream traces walk root → ... → witness cause → witness effect.
- Upstream traces walk root → ... → witness effect → witness cause, because investigation traverses causal edges in reverse while the stored edge remains cause → effect.
- Restrict prefix search to Events already inside that side's bounded causal neighborhood; traces must not escape the M203 query window to explain a witness.
- Choose a shortest directional prefix; break same-length alternatives by typed Event identity rather than timeline/display ordering.
- Always append the witness edge as the final traversal step, even for cross/cycle edges whose far endpoint was reachable earlier by another route.
- A trace is side-specific. All structure strictly shallower than `divergence_depth` is necessarily shared, but same-depth traces may pass another parallel divergence before terminating at their own witness.
- `root-presence` witnesses remain unchanged and carry no trace.

## Compatibility

- `#[serde(default)]` allows protocol-v1 M203 edge witnesses without `trace` to deserialize as an empty trace.
- Older clients can ignore the additive field.
- No new request, response variant, CLI command, transport, protocol version, AgentRuntime authority, or server-side state.

## Tests

Prove downstream and upstream traces, common-prefix behavior, cross/cycle terminal-edge behavior, typed shortest-path selection, and backward deserialization of M203 witnesses without trace.

## Non-goals

No global recursive divergence search, trace mutation authority, arbitrary graph export, opaque cursor, MCP/HTTP/WebSocket, AgentRuntime access, Pack-specific inference, or protocol v2.
