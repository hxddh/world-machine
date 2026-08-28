# Next Coding Task — M262 Bound Pack Response Queue

M261 is merged and now bounds Pack request writes against the existing 16 MiB physical JSONL receiver contract. The next concrete transport gap is on the opposite direction: `world-pack-process` bounds **each response record**, but the background response reader can still queue an unbounded number of individually-valid records in host memory.

The post-M261 audit found:

- `ProcessClient::request(&mut self, ...)` is single-flight: it sends one request and waits for exactly one correlated response before returning;
- `spawn_response_reader()` runs independently in a background thread and continuously calls `read_bounded_line()`;
- `read_bounded_line(..., DEFAULT_MAX_RESPONSE_BYTES)` already caps each physical response record at 16 MiB;
- however `spawn_response_reader()` forwards those records through `mpsc::channel()`, which is unbounded;
- therefore a malicious or malfunctioning Pack can emit arbitrarily many individually-valid response records and make the host accumulate `N × <=16 MiB` queued strings even though the public client has no legitimate need for an unbounded number of outstanding responses.

## M262

Replace the unbounded Pack response queue with fixed bounded backpressure while preserving the existing response framing, timeout, decode, and correlation semantics.

### Required production shape

Keep this milestone narrow:

1. retain `DEFAULT_MAX_RESPONSE_BYTES = 16 MiB` and the current physical JSONL record semantics;
2. use a fixed bounded response queue with a production capacity of **1**;
3. do not expose queue capacity through CLI, environment, manifest, Pack protocol, or other runtime configuration;
4. keep `ProcessClient::responses` as the consumer side used by the existing `recv_timeout` path;
5. preserve response order exactly;
6. preserve the existing fatal behavior for response I/O errors, oversized records, malformed JSON, protocol-version mismatch, request-id mismatch, timeout, EOF, and disconnected reader;
7. do not change Pack request framing added in M261.

A capacity of 1 intentionally permits one complete response to wait for the host while the reader may hold at most the next record it is currently attempting to hand off. This converts unbounded user-space accumulation into constant-bounded memory plus the operating-system pipe buffer without inventing concurrent Pack response semantics that the client does not support.

### Private reader helper for deterministic tests

Refactor only as far as needed to test the backpressure deterministically. A suitable shape is:

- production `spawn_response_reader(ChildStdout, max_response_bytes)` wraps stdout in `BufReader`;
- a private generic/helper reader loop accepts an `impl BufRead + Send + 'static` (or equivalent private seam);
- the helper creates/uses the fixed-capacity synchronous queue and keeps the existing `read_bounded_line` loop;
- no helper/test seam may become a public production setting.

Do **not** introduce a broad shared JSONL transport abstraction in this milestone.

### Test-first regressions

Before changing production code, add a regression that fails on the current `mpsc::channel()` implementation and requires a fixed bounded/synchronous queue.

Then add deterministic behavior tests proving:

1. queue capacity is fixed at 1 and the production reader cannot fall back to `mpsc::channel()`;
2. with three complete valid response records, the reader may queue the first and may read/hold the second, but it must not consume the third until the host drains capacity;
3. after the host drains one response, the reader resumes and preserves exact record order;
4. dropping the receiver while the reader is blocked on a bounded send causes the reader thread to exit rather than retaining an orphaned blocked producer indefinitely;
5. EOF is still delivered exactly as today;
6. response read errors/oversize remain terminal for the reader and are surfaced to `ProcessClient` through the existing path;
7. existing request timeout, protocol-version correlation, request-id correlation, durable probe, content pin, Pack conformance, and M261 request-bound regressions remain green.

Prefer tiny synthetic records for the queue/backpressure tests. The point is to bound **record count accumulation**, not to repeatedly allocate 16 MiB fixtures.

### Backpressure boundary / non-goal

Bounding stdout consumption can make an already-invalid Pack that floods unsolicited responses hit operating-system pipe backpressure earlier. That is intentional: host memory must not be the overflow buffer for an untrusted child.

Do not expand M262 into stdin write-timeout redesign. The current transport already cannot guarantee a bounded synchronous `write_all()` if a child stops reading stdin; solving write deadlines/cancellation is a separate milestone requiring a wider process-I/O design review. M262 must not silently change that unrelated contract.

## M261 invariants to preserve

- Pack request production ceiling remains 16 MiB **including the terminating LF**;
- request encoding and complete-frame size validation happen before request-id commit or stdin write;
- local request overflow writes zero bytes, does not kill the child, does not consume the request id, and allows same-process reuse;
- accepted Pack requests are encoded once and dispatched as one prepared frame;
- `send_shutdown()` uses the same bounded request preparation path;
- sender/server request ceilings remain equal.

## M260/M259 invariants to preserve

- Analyst client keeps its fixed 64 MiB serialized JSON payload ceiling with LF outside that payload budget;
- Analyst local serialization/overflow failures remain nonfatal and reusable;
- Rust tool-host request receiver keeps its established 64 MiB payload framing/UTF-8 behavior;
- M255 response overflow/write-error/abort/SIGTERM→SIGKILL behavior remains unchanged.

## Validation

Run at minimum:

- focused `world-pack-process` unit/integration tests;
- `bash ./scripts/check-boundaries.sh`;
- `cargo fmt --all -- --check`;
- Linux Clippy with `-D warnings` and full non-GPUI workspace tests;
- external Pack conformance;
- macOS GPUI/desktop/app packaging + packaged Analyst smoke whenever the normal path filter selects it;
- exact-head Codex review with zero unresolved threads before merge.

## Non-goals

No Pack protocol/version/schema change, no change to 16 MiB Pack request/response record ceilings, no response truncation, no Pack business behavior change, no World/query/UI work, no Analyst framing change, no broad transport abstraction, and no stdin write-timeout/cancellation redesign in M262.
