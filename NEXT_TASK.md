# Next Coding Task — M201 Persistent Machine Query Session

Turn the stable machine evidence-query contract into an efficient long-lived CLI transport by restoring one World snapshot once and processing a stream of newline-delimited query requests over stdin.

## Current baseline

The machine investigation surface is complete through M200:

- M185–M198 established typed query DTOs, stable semantic errors, protocol-v1 envelopes, visible selection/detail, state-evidence queries, and the causal investigation family;
- M199 locked the causal surface with cross-query invariants rather than adding new product semantics;
- M200 added executable causal continuations so bounded causal frontiers carry typed replayable `EvidenceQueryRequest::CausalNeighborhood` requests;
- `world-cli evidence-query <file.world> <request-json|->` exposes the generic machine contract, but each invocation still restores the World and exits.

M201 makes repeated investigation efficient without weakening any semantic or visibility boundary.

## Product goal

Add:

```text
world-cli evidence-query-session <file.world>
```

The command restores the World once, takes one immutable `ProjectionSnapshot`, then reads `EvidenceQueryRequest` documents as NDJSON from stdin until EOF.

For every non-empty input line it emits exactly one existing protocol-v1 status envelope followed by `\n`, then flushes stdout immediately so interactive callers can submit the next request without closing stdin.

## Transport contract

1. Input framing is one complete JSON request per line. Multi-line pretty-printed JSON is intentionally outside the session contract.
2. Empty or whitespace-only lines are ignored and produce no response.
3. Responses are ordered and positional: the Nth non-empty valid request produces the Nth envelope.
4. Success and semantic `QueryError` responses reuse the exact existing one-shot v1 envelope.
5. A semantic query error does not terminate the session; later requests continue.
6. Malformed request JSON remains a transport failure, matching one-shot semantics: no synthetic QueryError envelope, nonzero exit, existing stderr diagnostic.
7. Every completed response is flushed before the session waits for another request. A later malformed record cannot erase already completed output.
8. EOF after valid records exits zero.
9. M200 continuation requests are ordinary `EvidenceQueryRequest` values and can be replayed directly inside the same session process.
10. No request IDs are added in M201; sequential NDJSON already has unambiguous positional correlation.

## Architecture boundary

- Session framing belongs only in `world-cli`.
- Reuse `evidence_query_json_from_snapshot` so one-shot and session envelopes cannot drift.
- Load archive, registry session, and snapshot exactly once before the input loop.
- Keep the snapshot immutable/read-only for the entire session.
- Do not move stdin/stdout, buffering, or process concerns into `world-query`, `world-projection`, or `world-core`.
- Do not expose `ProjectionSnapshot` to in-world AgentRuntime.

## Tests

Prove with real subprocess behavior:

1. multiple NDJSON requests produce ordered protocol-v1 envelopes in one process;
2. blank lines are ignored;
3. a semantic QueryError emits an error envelope and a following valid request still succeeds;
4. malformed JSON after one valid record exits nonzero while preserving the already completed response;
5. a truly interactive caller can write one request, receive the flushed response before EOF, extract an M200 causal continuation, replay it through the same process, and receive the continued causal window;
6. existing one-shot, M199 invariant, and M200 continuation tests remain green.

## Validation

Before merge:

- `bash ./scripts/check-boundaries.sh`
- `cargo fmt --all -- --check`
- `cargo test -p world-query`
- `cargo test -p world-cli`
- focused Clippy with warnings denied
- semantic workspace CI and external Pack conformance
- macOS/GPUI only if dependency-path filtering requires it

## Non-goals for M201

Do not add request IDs, concurrency, out-of-order responses, mutation commands, World reload/watch semantics, comparison sessions, TCP/HTTP/WebSocket/MCP, AgentRuntime access, automatic malformed-record recovery, or protocol v2.
