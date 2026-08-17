# Next Coding Task — M189 Versioned Machine Query Envelope

Version the machine-readable evidence-query transport before external consumers begin depending on an implicit JSON envelope.

## Current baseline

The evidence/query line is complete through M188:

- M173–M178: evidence paths, bounded neighborhoods, durable neighborhood semantics, comparison, and divergence.
- M179: canonical stable selection keys.
- M180–M182: human-readable CLI neighborhood/path/comparison surfaces.
- M183–M184: reusable headless `world-query` and CLI routing through it.
- M185: serializable query/comparison request DTOs, response DTOs, canonical selection-key parsing, and serializable `QueryError`.
- M186: `world-cli` became a real consumer of the `world-query` request boundary.
- M187: `evidence-query` and `evidence-compare-query` added the first machine-readable JSON subprocess surface.
- M188: both machine commands accept request JSON from stdin with `-`, and true subprocess tests pin stdout/stderr/exit behavior.

Do not redo those milestones.

## Product goal

External tools should be able to identify which transport contract produced a response before they interpret `status`, `response`, or `error` fields.

The query semantics remain the M185 `world-query` DTOs. M189 versions only the CLI transport envelope; it must not wrap or fork the query request schema.

## Architecture boundary

1. `world-query` remains transport-neutral and owns query semantics, stable selection keys, result DTOs, and `QueryError`.
2. `world-cli` owns the subprocess envelope and its protocol/version metadata.
3. Do not add transport/version concerns to `world-core`, projection, persistence, Pack protocols, GPUI, or AgentRuntime.
4. Preserve both M187 inline JSON and M188 stdin request forms.
5. Preserve the M188 process contract: semantic query errors are valid protocol responses and exit 0; malformed/transport/archive failures remain nonzero process failures.

## M189 — explicit envelope identity

Add stable metadata to every successful protocol response and every semantic-error protocol response.

Use one shared envelope identity for single-World and comparison queries, for example:

```json
{
  "protocol": "world-machine-evidence-query",
  "version": 1,
  "status": "ok",
  "response": {}
}
```

and:

```json
{
  "protocol": "world-machine-evidence-query",
  "version": 1,
  "status": "error",
  "error": {}
}
```

The exact identifier spelling may be tightened during implementation, but it must be a constant owned in one place in `world-cli` and pinned by tests.

### Compatibility rules

- Existing request JSON remains exactly `EvidenceQueryRequest` or `EvidenceComparisonRequest`; do not add a request wrapper merely for versioning.
- Adding protocol/version metadata to response envelopes must be additive; keep `status`, `response`, and `error` semantics from M187/M188.
- Single-World and comparison commands must emit the same protocol name/version.
- Inline JSON and stdin JSON must emit the same envelope shape.
- Human-readable evidence commands remain unchanged.

## Tests

At minimum prove:

1. neighborhood success envelope carries the pinned protocol identifier and version;
2. shortest-path success uses the same protocol/version;
3. comparison success uses the same protocol/version;
4. serialized semantic `QueryError` envelope uses the same protocol/version;
5. inline and stdin subprocess forms expose identical protocol metadata;
6. malformed JSON is still a process/transport failure and does not emit a fake versioned semantic-error envelope;
7. M188 subprocess exit-code tests remain green.

Prefer assertions in the true subprocess integration test as well as focused helper-level tests so the public stdout contract is what is pinned.

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

## Non-goals for M189

Do not add:

- request-envelope versioning or a second request schema;
- HTTP/WebSocket/MCP transport;
- daemon or streaming mode;
- NDJSON/batching;
- authentication;
- AgentRuntime query access;
- perception-policy changes;
- new evidence semantics;
- Pack-specific query variants.

## Why this is next

M187 and M188 made the evidence-query boundary genuinely consumable across a process boundary. The next mistake would be to let external consumers hard-code an anonymous `{status,...}` object and only discover compatibility problems later. A tiny additive protocol/version marker is cheap now and gives future CLI, tool, RPC, or MCP adapters an explicit compatibility anchor without contaminating `world-query` semantics.
