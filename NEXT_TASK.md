# Next Coding Task — M210 CLI Investigation Adapter

Expose the M209 read-only progressive investigation boundary through `world-cli` without duplicating continuation scheduling or weakening the Projection/AgentRuntime boundary.

## Current baseline

M203–M208 define and prove deterministic, replayable first-divergence semantics. M209 packages those semantics in `world-investigation`, whose production dependency is only `world-query` and whose executor trait prevents the scheduler from reaching `ProjectionSnapshot` directly. The remaining gap is a concrete local adapter that external automation can invoke today.

## M210 — local CLI adapter

Add `world-cli evidence-investigate-compare <left.world> <right.world> <request-json|->`.

The request is an orchestration-layer JSON document:

```json
{"query":"first-divergence","root":"event-7","direction":"upstream","window_depth":2,"max_depth":12}
```

`world-cli` opens the two archives, owns the snapshots locally, implements `ComparisonQueryExecutor`, and delegates all progressive scheduling to `world-investigation`.

## Machine contract

- Emit a separate `world-machine-evidence-investigation` version-1 JSON envelope so orchestration results are not confused with the existing `world-machine-evidence-query` version-1 response DTOs.
- Successful responses contain the M209 absolute result: root, direction, max depth, bounded identity, absolute divergence depth, original-root witnesses, and truncation.
- Underlying `QueryError` values retain their existing stable serialized shape inside the investigation error envelope.
- M209 orchestration contract errors use stable kebab-case error keys.
- Malformed request JSON, unsupported query names, missing/wrong field types, and invalid direction remain CLI transport/input failures: non-zero exit, stderr, no success envelope.
- `-` reads one full JSON document from stdin, matching the existing machine-query commands.

## Boundary rules

- `world-cli` may hold `ProjectionSnapshot`; `world-investigation` still may not.
- The CLI adapter must call `investigate_first_divergence` rather than reimplement replay, offset accumulation, frontier convergence, or trace composition.
- No mutation authority and no AgentRuntime access are introduced.

## Validation

- subprocess test for stdin investigation and a real two-archive first divergence;
- stable investigation envelope and absolute witness trace;
- underlying query error remains a status-error envelope with exit zero;
- malformed JSON remains a non-zero CLI failure;
- `cargo fmt --all -- --check`;
- boundary checks, `world-cli` / `world-investigation` tests, Clippy, full workspace CI, external Pack conformance.

## Non-goals

No Agent tool adapter yet, no MCP/HTTP/WebSocket, no server cursor/session, no protocol-v2 change to evidence queries, no arbitrary graph export, and no mutation APIs.
