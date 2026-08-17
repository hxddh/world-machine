# Next Coding Task — M192 Machine Causal Ancestry

Expose persisted Event `caused_by` provenance through the existing machine query contract so external investigators can ask why a visible event happened without scraping timeline UI.

## Current baseline

The machine evidence-query line is complete through M191:

- M173–M178: typed state-evidence paths, neighborhoods, comparison, and divergence.
- M179–M186: canonical stable selection keys and reusable headless query boundary.
- M187–M189: JSON subprocess transport, stdin support, pinned exit semantics, and version-1 response envelopes.
- M190: deterministic visible selection discovery.
- M191: display-safe structured selection detail with timeline-owned Event visibility.

The machine workflow now supports `selections -> describe -> neighborhood / shortest-path`, but causal provenance is still only represented inside timeline projection data.

## Product goal

A caller should be able to ask:

```json
{"query":"why","event":"event-42"}
```

and receive a deterministic visible causal ancestry rooted at that Event.

This is causal provenance, not state-evidence adjacency. Keep those concepts separate.

## Architecture boundary

1. `world-query` owns the machine causal DTO and traversal semantics.
2. `world-cli` remains a thin JSON/subprocess transport; add no new top-level command.
3. Use only `ProjectionSnapshot.timeline.items` and their persisted `caused_by` links. Do not read raw World/archive state or rerun Behaviors/Agents.
4. Root Event visibility is timeline visibility, matching M190/M191.
5. A cause absent from the visible timeline must not appear in output, even if its ID is referenced by a visible item.
6. Do not use inspector existence to make an Event visible.
7. Keep state-evidence graph edges and causal edges as distinct APIs.
8. Keep the M189 envelope protocol/version unchanged; this is additive query capability.

## M192 — `why` query

Extend `EvidenceQueryRequest` with an additive variant equivalent to:

```json
{"query":"why","event":"event-42"}
```

Return a typed response similar to:

```rust
EvidenceWhyResult {
    event: String,
    nodes: Vec<EvidenceWhyNode>,
}

EvidenceWhyNode {
    event: String,
    depth: usize,
    world_time: u64,
    title: String,
    subtitle: String,
    caused_by: Vec<String>,
}
```

The exact Rust names may be tightened, but retain stable keys and generic visible labels.

## Traversal rules

- Parse through the existing canonical stable-key boundary.
- A canonical entity/relation key supplied to `why` is a kind mismatch, not malformed syntax. Add a stable typed semantic error for selection-kind mismatch rather than pretending `entity-1` is an invalid stable key.
- Root must be a timeline-visible Event or return `SelectionNotVisible`.
- Traverse upstream through `caused_by` only when each cause exists in the visible timeline.
- Preserve deterministic causal order from the persisted `caused_by` vector; deduplicate/cycle-protect traversal.
- Root depth is 0; direct causes are depth 1, and so on.
- Each node's exported `caused_by` list must itself be filtered to visible timeline Events so hidden IDs do not leak.

## CLI compatibility

Existing machine transports should accept the new request automatically:

```text
world-cli evidence-query <file.world> '{"query":"why","event":"event-42"}'
```

and:

```text
printf '%s' '{"query":"why","event":"event-42"}' | world-cli evidence-query <file.world> -
```

No `world-cli why` command is needed.

## Tests

At minimum prove:

1. serialized `why` request executes and round-trips through `EvidenceQueryResponse` serde;
2. a three-Event causal chain returns root/direct/indirect causes at depths 0/1/2;
3. multiple persisted causes preserve deterministic order;
4. a referenced cause absent from timeline is omitted both from traversal and exported `caused_by` IDs;
5. cycles cannot loop forever or duplicate nodes;
6. inspector-only Event remains invisible;
7. canonical `entity-N` / `relation-N` requests return the new stable selection-kind-mismatch error;
8. noncanonical `event-07` still returns `InvalidSelectionKey`;
9. a true stdin `world-cli` subprocess request returns a version-1 typed `why` response;
10. M190/M191 and graph-query tests remain green.

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

## Non-goals for M192

Do not add:

- downstream influence traversal yet;
- causal edges into the existing state-evidence graph;
- free-text/fuzzy search;
- HTTP/WebSocket/MCP;
- AgentRuntime access or perception-policy changes;
- raw World/Event mutation data;
- Pack-specific causal semantics;
- protocol version 2.

## Why this is next

M190 and M191 made visible state discoverable and describable; M173–M189 made state-evidence topology traversable. The other fundamental investigative dimension already present in World Machine is persisted causal provenance. Exposing `caused_by` as a separate machine query adds real Future Archaeologist / debugging value without weakening the deterministic World or Agent perception boundaries.
