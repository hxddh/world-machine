# Next Coding Task — M202 Executable Causal Comparison Continuations

Make bounded two-world causal comparison directly resumable by emitting typed replayable comparison requests at every left/right frontier.

## Current baseline

The machine causal investigation surface is complete through M201:

- M192–M200 provide single-world causal discovery, traversal, bounded neighborhoods, induced edges, frontiers, executable continuations, and cross-query invariants;
- M201 extends the existing protocol-v1 `evidence-compare-query` transport with tagged bounded causal-neighborhood structural comparison while preserving the legacy state-evidence compare wire shape exactly;
- causal comparison supports one-sided roots, so an Event present in only one world can still be investigated as a structural divergence.

## M202 — comparison continuations

Extend `EvidenceCausalNeighborhoodComparisonResult` additively with:

- `upstream_continuations: Vec<EvidenceCausalComparisonContinuation>`;
- `downstream_continuations: Vec<EvidenceCausalComparisonContinuation>`.

Each continuation contains:

- the canonical frontier Event key;
- `EvidenceCausalDirection`;
- `left_frontier` / `right_frontier` membership flags;
- an ordinary `EvidenceComparisonQueryRequest` that can be serialized and replayed directly through `evidence-compare-query`.

## Semantics

- Build continuations from the typed union of the left/right canonical frontier sets, one continuation per unique Event in typed Event order.
- Preserve whether the frontier is present on the left, right, or both sides.
- Preserve the original non-zero directional comparison window size.
- Promote a zero-depth frontier to a one-hop continuation so replay always makes progress.
- The opposite direction is set to depth zero.
- One-sided frontier Events are valid continuation roots because M201 comparison already supports roots visible in either world.
- Continuations carry no hidden state, visited set, opaque token, mutation authority, or server-side session state.
- `identical` remains a property of structural node/edge/frontier equality; continuation arrays are derived metadata and do not independently affect it.

## Compatibility

- Mark both new continuation arrays `#[serde(default)]` so M201 protocol-v1 causal comparison responses remain readable.
- Do not change legacy state-evidence comparison wire shapes.
- Keep `world-machine-evidence-query` at protocol version 1.

## Tests

Prove at minimum:

1. one-sided frontier emits a directly executable continuation with correct side flags;
2. replay reveals the next one-sided node/edge divergence;
3. distinct left/right frontier Events form a deterministic typed union;
4. a shared frontier emits one continuation with both side flags;
5. zero-depth continuations progress by one hop and non-zero window size is preserved;
6. M201 causal comparison payloads without continuation fields deserialize with empty defaults;
7. a real two-step stdin `world-cli evidence-compare-query` replay succeeds through the existing protocol-v1 transport;
8. all M199–M201 consistency, continuation, legacy comparison, and causal comparison tests remain green.

## Validation

- `bash ./scripts/check-boundaries.sh`
- `cargo fmt --all -- --check`
- `cargo test -p world-query`
- `cargo test -p world-cli`
- focused Clippy with warnings denied
- semantic workspace CI and external Pack conformance
- macOS/GPUI only if dependency-path filtering requires it

## Non-goals

Do not add automatic recursive comparison, opaque pagination tokens, server-side continuation state, arbitrary graph export, raw mutation payloads, AgentRuntime access, MCP/HTTP/WebSocket, Pack-specific causal inference, or protocol v2.
