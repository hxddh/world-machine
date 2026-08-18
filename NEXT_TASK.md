# Next Coding Task — M216 Reuse Local Investigation Executor in CLI

Remove the remaining duplicate local comparison authority from M210 `world-cli` now that M215 has established `world-investigation-local` as the reusable concrete archive/Projection adapter.

## Current baseline

M209 owns progressive investigation semantics. M210 exposes them through `world-cli` but still contains a private `SnapshotComparisonQueryExecutor` and duplicates archive restoration for investigation calls. M211–M214 build the provider-neutral external analyst tool stack. M215 adds `world-investigation-local` plus the long-lived `world-agent-tool-stdio` process, proving the shared local authority boundary independently.

## M216 — CLI local executor de-duplication

Refactor only the M210 investigation path:

- add `world-investigation-local` to `world-cli` dependencies;
- remove the CLI-private `SnapshotComparisonQueryExecutor`;
- make `evidence_investigate_compare_json_report` construct `LocalArchiveComparisonExecutor::from_archive_paths(left, right)`;
- keep snapshot-based unit helpers by constructing `LocalArchiveComparisonExecutor::new(left.clone(), right.clone())`;
- continue delegating orchestration to `investigate_first_divergence` without duplicating replay, frontier, depth, or trace semantics.

## Compatibility requirements

Preserve the existing command exactly:

`world-cli evidence-investigate-compare <left.world> <right.world> <request-json|->`

Preserve the `world-machine-evidence-investigation` version-1 envelope, stdin behavior, status-error semantics, malformed JSON failure behavior, and existing M210 subprocess tests. Do not change evidence-query protocol v1 or the M214 analyst-host protocol.

## Boundary rules

`world-investigation-local` is the concrete archive/Projection authority. Query-only `world-investigation`, external tool layers, and in-world `AgentRuntime` remain unchanged. No provider SDK, network server, mutable tool, or in-world tool injection is introduced.

## Validation

- existing `machine_investigation_first_divergence` subprocess suite stays unchanged and green;
- focused `world-cli` / `world-investigation-local` tests and Clippy;
- no CLI wire-format diff;
- full workspace CI and external Pack conformance; macOS/GPUI follows normal path filters.

## Non-goals

No provider-specific adapter yet, no MCP/HTTP/WebSocket server, no mutation tools, no AgentRuntime query access, and no protocol v2.
