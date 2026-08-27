# Next Coding Task — M255 Bound Analyst Tool-Host JSONL Framing

M254 bounds Pi RPC child stdout records. The next independent unbounded boundary is `AnalystJsonlClient.#acceptChunk()` in `integrations/pi/world-machine-analyst-client.mjs`, which consumes the restricted read-only Analyst tool-host stdout inside the Pi extension.

## M255

Today that client starts each receive with `Buffer.concat([this.buffer, chunk])` and only afterwards searches for `\n`. A very large or no-newline tool-host record can therefore grow/copy memory before JSON, protocol, or correlation validation. M254 cannot protect this separate child stream.

Required behavior:

- bound one tool-host response record to **64 MiB payload bytes**;
- newline framing is outside the payload budget; preserve optional trailing CR handling;
- count bytes, not JavaScript characters;
- do not preallocate 64 MiB and do not concatenate the whole incoming chunk before locating newline boundaries;
- preserve arbitrary chunk splits, multiple records in one chunk, ordering, and existing empty-line behavior;
- oversized input must fail before `JSON.parse`, never be truncated, poison `AnalystJsonlClient`, terminate the child, clear queued/pending record storage, and prevent later `listTools()` / `invoke()` reuse;
- framing contamination discovered later in the same synchronous stdout chunk must win over an earlier response line from that chunk: the active request must fail rather than return success and defer the fatal error to the next request;
- contaminated-child termination must be idempotent and must escalate from SIGTERM to SIGKILL after the existing shutdown grace period when the child ignores SIGTERM, including overflow while the client is idle;
- preserve current single-flight, catalog, invoke, remote-error, correlation, abort, shutdown, protocol-version, and provider-neutral behavior. In particular, normal remote tool errors remain recoverable and must not be confused with fatal framing contamination.

Use a constructor-injected small limit for tests while `spawn()` keeps the production default. Keep this change local to `world-machine-analyst-client.mjs` and `integrations/pi/tests/world-machine-analyst-client.test.mjs`; do not introduce a broad shared JSONL abstraction yet.

Required regressions:

1. exact-limit valid JSON + LF;
2. exact-limit valid JSON + CRLF;
3. a valid record split across many chunks;
4. multiple records in one chunk preserve order;
5. newline-terminated oversized payload fails before parsing;
6. no-newline oversized stream fails promptly;
7. oversize poisons/terminates the client and later requests cannot recover;
8. valid-looking bytes following an oversized prefix are ignored as contaminated-stream data;
9. a valid awaited response followed by an oversized record in the same stdout chunk still rejects the active request;
10. idle overflow against a child that ignores SIGTERM escalates to SIGKILL;
11. existing recoverable remote tool errors still allow subsequent requests on the same session;
12. all existing Analyst client tests remain green.

Validation: focused Node tests, `bash ./scripts/check-pi-analyst.sh`, fmt, authoritative Linux boundary/Clippy/workspace/Pack gates, and the full macOS packaged Analyst `.app` gate because `integrations/pi/**` ships with the desktop path.

Known later boundary: `jsonLines(stdin)` in `world-machine-analyst-turn-host.mjs` is still unbounded for Rust-client → turn-host input. Keep it separate after M255. The `world-cli` machine-query stdin `read_to_string` is another later bounded-input candidate.

## Non-goals

No protocol/schema/provider/model changes, no accepted tool-output truncation, no UI/history/World/Pack/query/archive changes, no cumulative turn-evidence budget, and no turn-host stdin change in M255.
