# Next Coding Task — M199 Cross-Query Causal Consistency

Harden the causal machine-query thin waist by proving that `why`, `influence`, `causal-path`, and `causal-neighborhood` are different views of one visible persisted causal graph rather than four independently drifting semantics.

## Current baseline

The causal machine-query surface is complete through M198:

- M192: upstream `why`;
- M193: downstream `influence`;
- M194: deterministic shortest `causal-path` and shared private `VisibleCausalGraph`;
- M195: bounded bidirectional `causal-neighborhood`;
- M196: frontier/truncation metadata;
- M197: explicit induced `edges` for bounded neighborhoods;
- M198: explicit induced `edges` for `why` and `influence`;
- all causal visibility comes only from timeline-visible Events plus persisted `TimelineItem.caused_by`;
- protocol remains `world-machine-evidence-query` v1.

## Product problem

Each causal query now has solid local tests, but consumers depend on stronger global guarantees. A path query must not disagree with influence reachability; a bounded neighborhood must not assign different depths than unbounded traversals; frontier metadata must describe exactly what the depth bound omitted; and the same visible graph must remain stable if timeline presentation order changes.

M199 adds those cross-query invariants before any additional transport or AgentRuntime integration.

## M199 — semantic invariant suite

Add one integration test module built around deterministic visible causal fixtures containing:

- a chain;
- a diamond with equal-length paths;
- a visible external co-cause;
- a hidden referenced Event ID;
- a directed causal cycle;
- disconnected causal components.

Do not add a new dependency such as property-testing infrastructure yet. Table-driven deterministic fixtures are sufficient for this milestone and avoid widening the dependency path.

## Required invariants

1. **Reachability duality**
   - For every pair of visible Events A/B, `B` appears in `influence(A)` iff `A` appears in `why(B)`.
   - Self reachability remains true because traversal results include the root at depth 0.

2. **Path/reachability equivalence**
   - `causal-path(A,B)` succeeds iff B is in `influence(A)`.
   - Every adjacent path pair must be an actual persisted edge in `influence(A).edges`.
   - Path depths are exactly `0..N` and endpoints are A/B.

3. **Bounded-prefix equivalence**
   - `causal-neighborhood(root, up, down).upstream` is exactly the `why(root)` nodes with `0 < depth <= up`, in the same order and with the same minimum depths.
   - `downstream` is the corresponding prefix of `influence(root)`.
   - The neighborhood root equals the root node exposed by both unbounded traversals.

4. **Induced-edge consistency**
   - Neighborhood `edges` must be exactly the persisted visible causal edges whose cause and effect are both in the returned node union.
   - No duplicate graph edges.

5. **Frontier exactness**
   - Upstream frontier is exactly the depth-boundary node set with at least one omitted visible parent.
   - Downstream frontier is exactly the depth-boundary node set with at least one omitted visible child.
   - `*_truncated` is exactly equivalent to a non-empty corresponding frontier.

6. **Visibility safety**
   - Hidden referenced Event IDs never surface in nodes, `caused_by`, edges, path output, frontier metadata, or serialized causal query responses.

7. **Cycle safety**
   - `why` and `influence` return each Event once.
   - The root is not reinserted into neighborhood side lists.
   - `causal-path` remains finite and shortest in a cycle.

8. **Input-order determinism**
   - Reordering `ProjectionSnapshot.timeline.items` without changing Event contents must not alter any causal query response.

## Implementation rule

Prefer a **test-only M199**. Do not change product code merely to make the milestone appear larger. If an invariant test exposes a real semantic inconsistency, fix only that shared causal-graph behavior and keep the query surface unchanged.

## Validation

Before merge:

- `bash ./scripts/check-boundaries.sh`
- `cargo fmt --all -- --check`
- `cargo test -p world-query`
- `cargo test -p world-cli`
- focused Clippy with warnings denied
- semantic workspace CI and external Pack conformance
- macOS/GPUI only if dependency-path filtering requires it

## Non-goals for M199

Do not add new query variants or DTO fields, protocol v2, pagination, arbitrary graph export, causal comparison between worlds, MCP/HTTP/WebSocket, AgentRuntime access, raw mutation payloads, Pack-specific causal inference, or new property-testing dependencies.
