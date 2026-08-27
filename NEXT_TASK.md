# Next Coding Task — M256 Bound Analyst Turn-Host Stdin Framing

M254 bounds long-lived Pi RPC child stdout and M255 bounds the restricted read-only tool-host stdout consumed by `AnalystJsonlClient`. The remaining unbounded JSONL boundary in the installed Analyst path is `jsonLines(stdin)` in `integrations/pi/world-machine-analyst-turn-host.mjs`, which receives Rust `world-analyst-client` requests.

## M256

Today `jsonLines()` repeatedly does `Buffer.concat([buffer, Buffer.from(chunk)])` before looking for `\n`. An oversized or indefinitely unterminated stdin record can therefore grow/copy an unbounded buffer before request JSON/shape validation.

Add a production **64 MiB payload-byte ceiling per turn-host input record** with these semantics:

- count bytes, not JavaScript characters;
- LF is framing and does not consume payload budget;
- preserve the existing optional trailing CR behavior, including max-size payload + CRLF;
- do not preallocate 64 MiB and do not concatenate the complete incoming chunk before finding newline boundaries;
- preserve arbitrary chunk splits, multiple records in order, and ignored empty lines;
- preserve the existing EOF-tail compatibility: a final non-empty record without a trailing LF is still accepted when its payload is within the limit, with optional final CR stripped;
- an oversized newline-terminated record, oversized no-newline stream, or oversized EOF tail must fail before `JSON.parse` / `host.handle` for that record; never truncate and parse;
- framing overflow is a fatal **turn-host input** failure. Do not fabricate a correlated protocol response because the oversized request may not contain a complete/trustworthy id. Let `runAnalystTurnHost()` unwind through its existing `finally` so the restricted Pi session is shut down and the process exits non-zero;
- release bounded accumulator references on failure/end.

The Rust producer has been audited: `AnalystTurnClient::transact_with_response_limit()` serializes each request, writes the encoded bytes, then explicitly writes `\n` and flushes. Production requests are therefore newline-terminated, but the parser's existing EOF-tail compatibility must still be preserved as parser behavior.

### Important semantic distinction from M254/M255

Do **not** retroactively invalidate a previously complete request only because a later, separate request in the same Node stdin chunk is oversized. JSONL request records are independent commands. A complete record may be yielded/handled; framing contamination becomes fatal when the oversized record itself is reached. Avoid making behavior depend on whether the OS happened to coalesce two independent requests into one `data` chunk.

This differs from M254/M255 response-side races, where a later framing failure in the same synchronous receive chunk had to prevent an in-flight caller from returning a success before the fatal session state was visible.

### Implementation shape

Keep the change primarily in `world-machine-analyst-turn-host.mjs` and its tests. A private/test-injectable limit for `jsonLines()` or `runAnalystTurnHost()` is appropriate; the installed CLI path must keep the production default and must not gain a new user-facing limit option. Do not introduce a broad shared JSONL utility merely to deduplicate M254–M256.

Use bounded record accumulation similar in byte/framing rules to M254/M255, adapted for an async iterable input and EOF-tail completion. Do not queue an entire input chunk worth of parsed lines before yielding: retain streaming behavior and bounded per-record state.

### Required regressions

1. exact-limit request + LF is accepted;
2. exact-limit request + CRLF is accepted;
3. exact-limit request at EOF without LF is accepted;
4. a request split across many input chunks is accepted;
5. multiple complete requests from one chunk remain ordered;
6. empty lines retain current ignore behavior;
7. newline-terminated oversized input fails before JSON parsing/handling that record;
8. no-newline stream fails as soon as it can no longer become a valid record, without waiting for EOF;
9. oversized EOF tail fails;
10. a valid first request followed by an oversized second request may complete the first request, then fatally stops before dispatching the oversized second request;
11. framing failure runs turn-host cleanup so the restricted Pi child/session does not survive;
12. existing probe, two-ask Pi reuse, command-error recovery, strict request shape, and SIGTERM termination tests remain green.

Tests must inject a small limit rather than allocate tens of MiB.

### Validation

Run focused turn-host Node tests, `bash ./scripts/check-pi-analyst.sh`, fmt, authoritative Linux boundary/Clippy/workspace/Pack gates, and the full macOS Library/Packs/GPUI/desktop/`World Machine.app` build + packaged Analyst smoke/archive/upload path.

## Later audit candidates

After M256, audit the Rust request writer for an optional **pre-write request-size ceiling** so an oversized prompt can be rejected before crossing the pipe, rather than relying only on the receiving Node boundary. Keep that separate from M256 so receiver framing correctness lands first. The `world-cli` machine-query stdin `read_to_string` boundary is another later bounded-input candidate.

## Non-goals

No Analyst protocol/schema/provider/model changes, no prompt/tool-output truncation, no cumulative turn budget, no Rust request-size preflight in M256, no UI/history/World/Pack/query/archive changes, and no broad JSONL framework refactor.
