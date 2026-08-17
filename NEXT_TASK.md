# Next Coding Task — M188 Stdin-Safe Machine Query Transport

Harden the M187 machine-readable evidence-query CLI into a subprocess boundary that an Agent/tool adapter can invoke without embedding arbitrary JSON in argv.

## Current baseline

The evidence/query line is complete through M187:

- M173–M178: evidence paths, bounded neighborhoods, durable neighborhood semantics, comparison, and divergence.
- M179: canonical stable selection keys.
- M180–M182: human-readable CLI neighborhood/path/comparison surfaces.
- M183–M184: reusable headless `world-query` and CLI routing through it.
- M185: serializable query/comparison request DTOs, response DTOs, canonical selection-key parsing, and serializable `QueryError`.
- M186: `world-cli` became a real consumer of the `world-query` request boundary instead of parsing selection keys itself.
- M187: machine-readable CLI commands now accept the existing query DTO JSON and return a CLI-owned status envelope while reusing `world-query` semantics and errors.

Do not redo those milestones.

## Product goal

A subprocess caller should be able to pipe exactly one JSON request through stdin and parse exactly one JSON response from stdout. This should be safe for Agent/tool adapters and ordinary process spawning without shell quoting or command-line length concerns.

M188 is transport hardening only. It must not create another query protocol.

## Architecture boundary

1. `world-query` remains the sole owner of evidence-query semantics, stable-key parsing, typed results, and `QueryError`.
2. `world-cli` owns stdin/argv/stdout/stderr and process exit behavior.
3. Do not move transport concerns into `world-core`, `world-host`, `world-projection`, or GPUI.
4. Do not duplicate the M185 request/response schema in a new crate.
5. Query execution remains read-only and replay-safe: no Action dispatch, AgentRuntime invocation, wall clock, or World mutation.
6. Preserve M187 inline-JSON argv compatibility.

## M188 — `-` means request JSON from stdin

Extend both M187 commands:

```text
world-cli evidence-query <file.world> -
world-cli evidence-compare-query <left.world> <right.world> -
```

When the request argument is `-`:

1. Read one complete JSON document from stdin to EOF.
2. Feed those bytes/text into the exact same M187 deserialization and `world-query` execution path used by inline JSON.
3. Emit the same M187 JSON success/error envelope to stdout.
4. Do not add NDJSON, streaming, batches, or multiple-request framing.

Inline JSON must continue to work unchanged:

```text
world-cli evidence-query <file.world> '{"query":"neighborhood","root":"entity-1","max_depth":2}'
```

## Process semantics

Pin the machine-facing behavior explicitly:

- valid JSON + successful query -> exit 0; one success envelope on stdout;
- valid JSON + semantic `QueryError` -> exit 0; one error envelope on stdout;
- malformed request JSON -> nonzero exit; diagnostic on stderr; no JSON success/error envelope pretending the protocol request was valid;
- stdin read failure -> nonzero exit;
- archive read/parse/Pack-open failure -> nonzero exit;
- ordinary human-readable commands keep their existing behavior.

A semantic query failure is a valid protocol response, not a transport crash.

## Implementation guidance

Keep this small. Prefer a tiny request-source abstraction inside `world-cli`, for example inline text vs stdin, rather than duplicating command handlers.

If the current binary-only shape makes subprocess testing awkward, add focused integration tests under `crates/world-cli/tests/` rather than turning `world-cli` into a large library.

## Tests

At minimum prove:

1. `evidence-query <archive> -` accepts a neighborhood request via piped stdin and emits parseable success JSON;
2. stdin shortest-path identity request uses the same response schema;
3. `entity-07` via stdin produces the serialized `invalid-selection-key` semantic error envelope and exits 0;
4. malformed stdin JSON exits nonzero and is distinguishable from a semantic error;
5. comparison query works via stdin and returns a typed comparison result;
6. the original inline-JSON M187 commands remain green;
7. existing human-readable evidence commands remain green.

Prefer at least one true subprocess integration test using the built `world-cli` binary so stdin/stdout/stderr/exit-code behavior is verified rather than inferred from helper functions.

## Validation

Before merge:

- `bash ./scripts/check-boundaries.sh`
- `cargo fmt --all -- --check`
- semantic workspace Clippy with warnings denied
- semantic workspace tests
- `cargo test -p world-query`
- `cargo test -p world-cli`
- external Pack conformance command
- macOS/GPUI and `World Machine.app` artifact validation whenever dependency-path filtering requires it

## Non-goals for M188

Do not add:

- HTTP/WebSocket transport;
- MCP;
- daemon mode;
- NDJSON or batching;
- async streaming;
- authentication/authorization;
- remote archive URLs;
- new evidence semantics;
- Pack-specific queries;
- AgentRuntime execution.

## Why this is next

M187 proved that the query contract can cross a process boundary as JSON, but argv is still an awkward carrier for arbitrary structured requests. Stdin is the smallest transport hardening that makes the subprocess surface comfortable for Agents, tool adapters, scripts, and tests while keeping the same `world-query` thin waist.
