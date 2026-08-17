# Next Coding Task — M200 Persistent Machine Query Session

Turn the stable machine evidence-query contract into an efficient long-lived CLI transport by loading one World snapshot once and processing a stream of newline-delimited query requests over stdin.

## Current baseline

The machine investigation surface is semantically stable through M199:

- M185–M198 established typed query DTOs, stable errors, protocol-v1 envelopes, visible selection/detail, state-evidence queries, and a complete causal investigation family;
- M199 proves cross-query causal consistency without requiring product-code changes;
- `world-cli evidence-query <file.world> <request-json|->` already exposes the generic query contract, but each invocation parses/restores the World and then exits.

For an external agent adapter or interactive investigator, process-per-query restore cost is unnecessary and creates avoidable latency.

## Product goal

Add:

```text
world-cli evidence-query-session <file.world>
```

The command restores the World once, takes one immutable `ProjectionSnapshot`, then reads `EvidenceQueryRequest` documents as NDJSON from stdin until EOF.

For every non-empty input line it emits exactly one existing protocol-v1 status envelope followed by `\n`, then flushes stdout before reading/processing further work.

## Transport contract

1. Input is **one complete JSON request per line**. Multi-line pretty-printed JSON is intentionally not part of the session framing.
2. Empty or whitespace-only lines are ignored and produce no response.
3. Responses are strictly ordered and positional: the Nth non-empty valid request produces the Nth envelope.
4. Success and semantic `QueryError` responses use the exact existing one-shot envelope:
   - `{protocol, version:1, status:"ok", response:...}`
   - `{protocol, version:1, status:"error", error:...}`
5. A semantic query error does **not** terminate the session; later records continue.
6. Malformed request JSON remains a transport failure, matching one-shot semantics. It writes no synthetic QueryError envelope, terminates the process nonzero, and reports the existing `invalid evidence query JSON` error on stderr.
7. Any already completed response line must have been flushed before a later malformed record terminates the process.
8. EOF after valid records exits zero.
9. No request IDs are added in M200; ordered NDJSON is sufficient for this sequential transport and avoids changing the v1 envelope.

## Architecture boundary

- Implement session framing only in `world-cli`.
- Reuse `evidence_query_json_from_snapshot` so one-shot and session envelopes cannot drift.
- Load archive, registry session, and snapshot exactly once before the input loop.
- Keep the snapshot immutable/read-only for the lifetime of the session.
- Do not move streaming, stdin/stdout, or process concerns into `world-query`, `world-projection`, or `world-core`.
- Do not expose a full ProjectionSnapshot to in-world AgentRuntime.

## Tests

Prove at minimum with real subprocess tests:

1. multiple NDJSON requests produce multiple ordered protocol-v1 envelopes in one process;
2. blank lines are ignored;
3. a semantic QueryError produces an error envelope and the following valid request still succeeds;
4. malformed JSON after one valid record exits nonzero, reports the transport error on stderr, and preserves exactly the already-completed response line;
5. existing one-shot machine-query subprocess tests remain unchanged and green.

## Validation

Before merge:

- `bash ./scripts/check-boundaries.sh`
- `cargo fmt --all -- --check`
- `cargo test -p world-query`
- `cargo test -p world-cli`
- focused Clippy with warnings denied
- semantic workspace CI and external Pack conformance
- macOS/GPUI only if dependency-path filtering requires it

## Non-goals for M200

Do not add request IDs, concurrency, out-of-order responses, mutation commands, world reload/watch semantics, comparison sessions, TCP/HTTP/WebSocket/MCP, AgentRuntime access, protocol v2, or automatic recovery after malformed JSON.
