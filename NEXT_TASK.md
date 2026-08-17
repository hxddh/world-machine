# Next Coding Task — M190 Evidence Selection Discovery

Make the machine-readable evidence-query surface self-discoverable: external tools must be able to learn which stable selection keys are currently queryable before issuing neighborhood or shortest-path requests.

## Current baseline

The evidence/query line is complete through M189:

- M173–M178: evidence paths, bounded neighborhoods, durable neighborhood semantics, comparison, and divergence.
- M179: canonical stable selection keys.
- M180–M182: human-readable CLI neighborhood/path/comparison surfaces.
- M183–M184: reusable headless `world-query` and CLI routing through it.
- M185: serializable request/response DTOs, centralized stable-key parsing, and serializable `QueryError`.
- M186: `world-cli` became a real consumer of the `world-query` contract.
- M187: machine-readable JSON query commands.
- M188: stdin-safe subprocess transport with true exit/stdout/stderr integration tests.
- M189: versioned CLI response envelope (`world-machine-evidence-query`, version 1).

Do not redo those milestones.

## Product goal

Today a machine caller can ask about `entity-7`, `relation-12`, or `event-31`, but it has no typed way to discover which keys exist or what they represent. That forces screen scraping, prior knowledge, or guessing.

M190 adds a read-only discovery query to the existing `EvidenceQueryRequest` contract. It must use the same ProjectionSnapshot visibility boundary as neighborhood/path queries and must not expose hidden World state.

## Architecture boundary

1. `world-query` owns discovery semantics and its typed DTOs.
2. `world-cli` remains a thin JSON/subprocess transport; do not create a separate discovery command or duplicate discovery logic there.
3. A selection is discoverable only if it is already query-visible under current evidence semantics:
   - entities/relations must be visible through the snapshot inspectors;
   - events must be visible through the snapshot timeline.
4. Event inspectors alone must not make an event discoverable if the event is absent from the visible timeline.
5. Do not expose raw World, Pack internals, hidden entities, hidden relations, or Agent perception-bypassing data.
6. Keep the M189 envelope protocol/version unchanged; this is an additive query capability, not a transport break.

## M190 — `selections` query

Extend the existing request enum with an additive variant equivalent to:

```json
{"query":"selections"}
```

Return an existing-style typed response variant containing a deterministic list of queryable selections.

Each item should contain at minimum:

- canonical stable key (`entity-N`, `relation-N`, or `event-N`);
- typed kind (`entity`, `relation`, `event`);
- human-readable title;
- human-readable subtitle/detail already present in the visible ProjectionSnapshot.

Suggested DTO shape:

```rust
EvidenceSelectionIndex {
    selections: Vec<EvidenceSelection>,
}

EvidenceSelection {
    selection: String,
    kind: EvidenceSelectionKind,
    title: String,
    subtitle: String,
}
```

The exact Rust names may be tightened during implementation, but keep the JSON compact and generic.

## Determinism and visibility

- Output ordering must be deterministic and based on typed `SelectionId`, not hash-map iteration or lexicographic string accidents (`event-10` before `event-2`).
- Deduplicate by typed selection id.
- Entity/relation labels come from visible inspectors.
- Event labels come from visible timeline items.
- Do not infer or reconstruct hidden selections from causal links, relation endpoints, archive contents, or World state.

## CLI compatibility

The existing M187/M188 machine command should accept the new request automatically:

```text
world-cli evidence-query <file.world> '{"query":"selections"}'
```

and through stdin:

```text
printf '%s' '{"query":"selections"}' | world-cli evidence-query <file.world> -
```

No new top-level CLI command is needed.

## Tests

At minimum prove:

1. serialized `{"query":"selections"}` executes through `execute_query`;
2. entities, relations, and timeline-visible events return canonical stable keys and typed kinds;
3. output ordering is deterministic by typed selection id;
4. an event present only in inspectors but absent from timeline is not discoverable;
5. discovery results round-trip through `EvidenceQueryResponse` serde;
6. a true `world-cli` subprocess request via stdin returns a version-1 success envelope containing a non-empty typed selection index;
7. existing neighborhood/path/comparison/error tests remain green.

## Validation

Before merge:

- `bash ./scripts/check-boundaries.sh`
- `cargo fmt --all -- --check`
- semantic workspace Clippy with warnings denied
- semantic workspace tests
- `cargo test -p world-query`
- `cargo test -p world-cli`
- external Pack conformance command
- macOS/GPUI only when dependency-path filtering requires it

## Non-goals for M190

Do not add:

- fuzzy search or free-text retrieval;
- pagination/filter syntax;
- HTTP/WebSocket/MCP transport;
- AgentRuntime access;
- changes to `ScopedPerception`;
- new World state or Event semantics;
- Pack-specific selection types;
- request-envelope versioning.

## Why this is next

M185–M189 created a robust machine query boundary, but it is not yet independently usable: callers need a valid root key before they can ask anything. Selection discovery closes that usability gap while staying entirely within the already-visible ProjectionSnapshot boundary. It is also the safer prerequisite before deciding how an in-world Agent may receive scoped query access, because discovery can be tested without weakening perception policy.
