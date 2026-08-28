# Next Coding Task — M263 Bound Pack Request Write Deadline

M262 is merged and bounds the Pack response queue to fixed capacity 1. The remaining transport hang is on the request write itself: `ProcessClient::request()` prepares a bounded frame, then performs synchronous `ChildStdin::write_all(&frame)` + `flush()` before `recv_timeout(self.request_timeout)` starts.

The existing `hung_process_is_timed_out_and_terminated` regression does not cover this gap: its fixture first reads the request and only then hangs before responding. A Pack that never reads stdin can instead fill the operating-system pipe with one otherwise-valid request and block the host before the current response timeout is ever consulted.

## M263

Make the existing Pack `request_timeout` bound the complete transport transaction from request dispatch through correlated response receipt, without changing Pack framing, protocol, or valid single-flight behavior.

### Required behavior

Keep this milestone transport-only:

1. retain the M261 fixed 16 MiB Pack request physical-frame ceiling, including the terminating LF;
2. retain prepare/encode/size validation before request-id commit or transport I/O;
3. once a prepared valid frame is committed for dispatch, start one request deadline before any potentially blocking stdin write;
4. the same deadline budget covers both request write/flush and the subsequent correlated response wait — do not silently grant one full timeout to each phase;
5. if request dispatch cannot complete before the deadline, terminate/reap the contaminated Pack process and return the existing timeout-class session failure rather than waiting forever;
6. a partial or timed-out write is fatal: do not attempt same-process reuse and do not consume any response as if the request had been cleanly dispatched;
7. preserve existing fatal behavior for write errors, response I/O/oversize, malformed JSON, protocol-version mismatch, request-id mismatch, response timeout, EOF, and reader disconnect;
8. preserve normal remote `PackResponse::Error` behavior and all valid single-flight request/response ordering;
9. do not add a user/manifest/CLI/env timeout knob in this milestone; keep the existing internal `request_timeout` seam;
10. do not let a new writer helper create an unbounded queue or an unreapable per-request worker-thread leak.

### Lifecycle constraint

A naive `thread::spawn(write_all)` plus `recv_timeout` is not sufficient if timeout merely detaches the blocked writer. Any writer helper must have a bounded ownership/cleanup story after timeout. If stdin ownership moves off `ProcessClient`, `Drop`/shutdown must not reintroduce an unbounded synchronous write path.

Do not broaden this milestone into Pack process-tree sandboxing unless the implementation demonstrably requires it to make writer cleanup correct. If descendant pipe inheritance prevents bounded cleanup, stop and make that lifecycle prerequisite explicit rather than masking it with a detached thread.

### Test-first regressions

Before production changes:

- pin with a source-order contract that `ProcessClient::request()` no longer directly performs the synchronous `ChildStdin` write before consulting `request_timeout`;
- require the request timeout to participate before the existing response `recv_timeout` path.

Then add real process behavior coverage proving:

1. a fixture that never reads stdin receives a legal request large enough to fill the pipe, yet the host returns within the configured request deadline instead of hanging;
2. timeout cleanup leaves the direct child reaped;
3. a normal request written within budget still receives and correlates its response;
4. a slow-but-within-budget write leaves only the remaining deadline for response wait;
5. write failure remains fatal and does not fabricate a correlated response;
6. M261 exact-limit / local-overflow / request-id reuse tests remain green;
7. M262 bounded response queue, EOF/error, correlation, durable probe, Pack conformance, and macOS packaged-app gates remain green.

The real no-read fixture must have an external watchdog only for test cleanup so the old implementation fails promptly without hanging CI. The production pass condition must be that `ProcessClient` returns before that watchdog intervenes.

## M262 invariants to preserve

- response queue capacity remains fixed at 1;
- no fallback to unbounded `mpsc::channel()` in the production response-reader path;
- response record order is exact;
- per-record response framing remains fixed at 16 MiB;
- dropping the response receiver releases a blocked bounded sender;
- response EOF/read-error/oversize/correlation/timeout semantics remain unchanged.

## M261 invariants to preserve

- request production ceiling remains 16 MiB including LF;
- complete-frame preflight happens before request-id commit or stdin dispatch;
- local oversize writes zero bytes, is nonfatal, does not consume request id, and permits same-process reuse;
- accepted requests are encoded once;
- `send_shutdown()` remains on the bounded request-preparation path.

## Validation

Run at minimum:

- focused `world-pack-process` unit/integration tests;
- `bash ./scripts/check-boundaries.sh`;
- `bash ./scripts/check-pi-analyst.sh`;
- `cargo fmt --all -- --check`;
- Linux Clippy with `-D warnings` and full non-GPUI workspace tests;
- external Pack conformance;
- macOS GPUI/desktop/app packaging + packaged Analyst smoke whenever selected by the normal path filter;
- exact-head Codex review with zero unresolved review threads before merge.

## Non-goals

No Pack protocol/version/schema change, no change to 16 MiB request/response ceilings, no response truncation, no World/query/UI/Analyst behavior change, no broad shared JSONL transport abstraction, no timeout configuration surface, and no speculative process-tree redesign without a demonstrated lifecycle need.
