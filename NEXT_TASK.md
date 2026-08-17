# Next Coding Task — M191 Machine Selection Describe

Complete the basic machine investigation loop by letting callers retrieve structured visible detail for one discovered selection without scraping human-readable inspector output.

## Current baseline

The evidence/query line is complete through M190:

- M173–M178: evidence paths, bounded neighborhoods, durable neighborhood semantics, comparison, and divergence.
- M179: canonical stable selection keys.
- M180–M182: human-readable CLI neighborhood/path/comparison surfaces.
- M183–M184: reusable headless `world-query` and CLI routing through it.
- M185: serializable query/comparison DTOs, centralized stable-key parsing, and serializable `QueryError`.
- M186: `world-cli` became a real consumer of that contract.
- M187: machine-readable JSON query commands.
- M188: stdin-safe subprocess transport with real stdout/stderr/exit-code tests.
- M189: versioned response envelope (`world-machine-evidence-query`, version 1).
- M190: additive `{"query":"selections"}` discovery returns deterministic visible entity/relation/event keys and labels.

Do not redo those milestones.

## Product goal

A machine caller can now discover a valid stable key and traverse its evidence graph, but it still cannot retrieve the visible structured details behind that selection without screen scraping.

M191 adds one additive describe query so the machine workflow becomes:

1. discover selections;
2. describe one visible selection;
3. inspect its neighborhood or shortest evidence path.

## Architecture boundary

1. `world-query` owns describe semantics and DTOs.
2. `world-cli` remains only JSON/subprocess transport; no new top-level command.
3. Parse the stable key through the same centralized query boundary as neighborhood/path.
4. Visibility must match M190/evidence semantics:
   - entity/relation visible only when its inspector is present;
   - event visible only when present in the visible timeline;
   - an event inspector by itself must never make an event visible.
5. Do not expose raw `InspectorProjection.sections` wholesale. Only expose sections already intended for display through `InspectorProjection::display_sections()` so evidence/history/identity support sections remain internal.
6. Do not read raw World/archive state to reconstruct hidden details.
7. Keep the M189 protocol/version unchanged; this is an additive query capability.

## M191 — `describe` query

Extend the existing request enum with an additive variant equivalent to:

```json
{"query":"describe","selection":"entity-7"}
```

Return a typed response containing at minimum:

```rust
EvidenceSelectionDetail {
    selection: String,
    kind: EvidenceSelectionKind,
    title: String,
    subtitle: String,
    sections: Vec<EvidenceDetailSection>,
}

EvidenceDetailSection {
    title: String,
    rows: Vec<EvidenceDetailRow>,
}

EvidenceDetailRow {
    label: String,
    value: String,
}
```

The exact Rust names may be tightened, but keep the JSON generic and transport-neutral.

## Source-of-truth rules

### Entity / relation

- title/subtitle come from the visible inspector;
- detail rows come only from `inspector.display_sections()`;
- therefore internal history/evidence/identity sections remain absent from the machine response.

### Event

- visibility and title/subtitle come from the visible timeline item, matching M190 discovery;
- if a matching event inspector exists, its `display_sections()` may provide Context/Payload/Changes detail rows;
- if no event inspector exists, return the visible timeline metadata with an empty section list rather than inventing data;
- an event inspector without a timeline item must return `SelectionNotVisible`.

## CLI compatibility

Both existing machine transports should automatically accept describe:

```text
world-cli evidence-query <file.world> '{"query":"describe","selection":"entity-7"}'
```

and:

```text
printf '%s' '{"query":"describe","selection":"entity-7"}' | world-cli evidence-query <file.world> -
```

Do not add `world-cli describe`.

## Tests

At minimum prove:

1. serialized describe request executes through `execute_query` and round-trips through `EvidenceQueryResponse` serde;
2. entity detail returns visible title/subtitle and display sections;
3. relation detail excludes `RELATION_HISTORY_SECTION`, `RELATION_ENDPOINTS_SECTION`, and `RELATION_IDENTITY_SECTION`;
4. entity detail excludes `ENTITY_HISTORY_SECTION`;
5. timeline-visible event uses timeline title/subtitle and may expose only display-safe event inspector sections;
6. inspector-only event is rejected as `SelectionNotVisible`;
7. malformed/noncanonical selection keys still return `InvalidSelectionKey` through the existing semantic error path;
8. a true stdin subprocess describe request returns a version-1 success envelope with a typed description;
9. all M190 discovery and existing graph-query tests remain green.

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

## Non-goals for M191

Do not add:

- free-text/fuzzy search;
- pagination/filter syntax;
- HTTP/WebSocket/MCP;
- AgentRuntime query access;
- perception-policy changes;
- raw inspector/evidence support-section export;
- new World/Event semantics;
- Pack-specific detail schemas;
- protocol version 2.

## Why this is next

M190 made the query boundary self-discoverable. The highest-value missing primitive is now structured detail for a discovered node. Adding describe before search, MCP, or AgentRuntime integration yields a complete generic read-only investigation surface while preserving the existing ProjectionSnapshot visibility boundary and avoiding a perception-policy shortcut.
