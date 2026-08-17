# Next Coding Task — M187 Machine-Readable Evidence Query Surface

Turn the reusable evidence-query contract into a real machine-facing subprocess boundary without creating a second query protocol.

## Current baseline

The evidence/query line is now complete through M186:

- M173–M178 established shortest evidence paths, bounded evidence neighborhoods, durable neighborhood semantics, comparison, and divergence surfaces.
- M179 added canonical stable selection keys.
- M180–M182 exposed neighborhood, shortest-path, and neighborhood-comparison queries through `world-cli`.
- M183 extracted reusable headless evidence queries into `world-query`.
- M184 routed the existing CLI evidence reports through `world-query`.
- M185 added serializable request/response/comparison DTOs, centralized canonical selection-key parsing, and stable serializable `QueryError` shapes.
- M186 made `world-cli` the first real consumer of that contract: the CLI now keeps selection keys as strings and delegates their semantic parsing/validation to `world-query`.

Do not redo any of those milestones.

## Product goal

A subprocess, Agent adapter, test harness, or future server should be able to send one typed evidence-query request and receive machine-readable JSON without scraping the existing human-readable CLI reports.

The JSON surface must be a thin transport adapter over `world-query`. It must not become a new source of query semantics.

## Architecture boundary

Keep the existing thin waist intact:

1. `world-core` remains deterministic World truth and must not depend on query transport, JSON CLI concerns, GPUI, or Pack-specific concepts.
2. `world-query` owns evidence-query semantics, canonical selection-key parsing, typed result DTOs, and `QueryError`.
3. `world-cli` may own argv/stdin/stdout transport details and JSON framing.
4. Do not duplicate `entity-N` / `relation-N` / `event-N` parsing in the CLI.
5. Do not manufacture Events, causes, selections, or comparison results in the transport layer.
6. Query execution must remain read-only: no Action dispatch, AgentRuntime invocation, wall clock, filesystem mutation beyond reading the requested archive, or GPUI state.
7. Tiny Society may be an acceptance Pack, but no Tiny Society semantic name belongs in the query protocol.

## M187 — JSON evidence query commands

Add the first explicit machine-readable CLI surface.

Preferred command shape:

```text
world-cli evidence-query <file.world> '<request-json>'
world-cli evidence-compare-query <left.world> <right.world> '<request-json>'
```

The exact spelling may change if implementation pressure reveals a cleaner CLI shape, but preserve the semantic contract below.

### Single-World query

1. Deserialize the request directly as `world_query::EvidenceQueryRequest`.
2. Open the archive through the normal Pack registry/session path.
3. Execute through `world_query::execute_query` only.
4. On success, emit JSON containing the existing `EvidenceQueryResponse` without re-rendering it into display strings.
5. On a semantic query failure, emit the existing serialized `QueryError` inside an explicit, documented JSON status envelope owned by the CLI transport.
6. Malformed JSON should remain distinguishable from a valid request that produces a semantic `QueryError`.

### Comparison query

1. Deserialize the request directly as `world_query::EvidenceComparisonRequest`.
2. Open the left and right archives independently through the normal registry/session path.
3. Execute through `world_query::execute_comparison_query` only.
4. Emit the existing `EvidenceComparisonResult` on success.
5. Reuse the same transport-level success/error envelope shape as the single-World command.

### Compatibility

- Keep the existing human-readable `evidence`, `evidence-path`, and `evidence-compare` commands unchanged.
- Do not make existing scripts parse JSON unless they opt into the new machine-readable commands.
- Do not change `.world` / `.worldpack` persistence formats.
- Do not add a server process merely to expose this slice.

## Tests

Use generic/synthetic snapshots wherever possible.

At minimum prove:

1. a serialized neighborhood request reaches `execute_query` and returns a JSON success envelope;
2. a serialized shortest-path request reaches the same execution path;
3. a noncanonical key such as `entity-07` produces the serialized `invalid-selection-key` `QueryError`, not a CLI-local parser error;
4. malformed request JSON is distinguishable from semantic query failure;
5. comparison JSON returns the existing typed comparison result;
6. output can be parsed back into the expected DTO/error shape without screen-string scraping;
7. current human-readable evidence command tests remain green.

## Validation

Before merge:

- `bash ./scripts/check-boundaries.sh`
- `cargo fmt --all -- --check`
- semantic workspace Clippy with warnings denied
- semantic workspace tests
- `cargo test -p world-query`
- `cargo test -p world-cli`
- external Pack conformance command
- macOS/GPUI jobs should remain skipped unless their actual dependency paths change

## Non-goals for M187

Do **not** add these yet:

- HTTP/WebSocket server;
- MCP server/client;
- long-running daemon;
- streaming/NDJSON batch protocol;
- authentication/authorization;
- remote archive fetching;
- new evidence graph semantics;
- new Pack-specific query types;
- AgentRuntime execution.

Those should be justified by a real consumer after the subprocess contract is proven.

## Why this is next

M185 made the query boundary serializable, and M186 proved an existing product surface can consume that boundary without owning selection-key semantics. The remaining gap is transport: external tools still have to scrape human-readable CLI output.

M187 closes that gap with the smallest useful machine interface. If this stays thin, the same `world-query` contract can later sit behind a CLI, an Agent tool, an RPC endpoint, or an MCP adapter without changing World truth or creating competing query semantics.
