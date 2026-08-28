# Next Coding Task — M261 Bound Pack Process Request Writes

M260 adds sender-side preflight to the installed Analyst tool client. The next transport gap is the older external Pack process path, but its contract is **not identical** to the Analyst JSONL contract and must not be normalized by assumption.

The post-M260 audit found:

- `crates/world-pack-process::ProcessClient` already bounds **responses** with `DEFAULT_MAX_RESPONSE_BYTES = 16 MiB`;
- `crates/world-pack-server` already bounds incoming **requests** with `DEFAULT_MAX_REQUEST_BYTES = 16 MiB`;
- `ProcessClient::request()` still has **no sender-side request bound**: it `encode_request()`s, then writes the encoded JSON and LF directly to the child;
- the current Pack server receiver's historical limit is a **physical JSONL record limit**: `read_bounded_line(..., DEFAULT_MAX_REQUEST_BYTES)` counts bytes returned by `read_until(b'\n', ...)`, so a terminating LF is currently inside the 16 MiB ceiling;
- therefore M261 must align the sender with the existing Pack v1 receiver contract rather than copying M259/M260's Analyst rule where LF is outside the 64 MiB payload budget.

## M261

Add a fixed sender-side bound to `world-pack-process::ProcessClient` so a Pack request that the existing `world-pack-server` receiver will reject is never handed to the child stdin pipe.

The **existing server contract is authoritative for this milestone**:

- production request wire ceiling remains `world_pack_server::DEFAULT_MAX_REQUEST_BYTES` = **16 MiB**;
- count the UTF-8 bytes of the encoded request **plus the single terminating LF**, because that is what the current Pack receiver bounds;
- do not change protocol v1, request schemas, Pack authority, or the server's current wire semantics just to resemble Analyst framing;
- do not truncate/rewrite a request to fit;
- no CLI/env/runtime setting may raise the production ceiling;
- use one already-encoded request for both size validation and transport; do not call `encode_request()` a second time after preflight.

If a lower-only test seam is needed, keep it private/test-local and never permit a value above the 16 MiB production ceiling.

### Sender ordering and request-id invariant

Today `ProcessClient::request()` reserves the id before encoding and before any write:

```rust
let request_id = self.next_request_id;
self.next_request_id = self.next_request_id.checked_add(1)?;
let envelope = PackRequestEnvelope::for_version(..., request_id, request)?;
let encoded = encode_request(&envelope)?;
write(encoded);
write(b"\n");
```

M261 should prepare the envelope/frame and validate its size **before committing sender correlation state or touching stdin** where practical. A request rejected entirely locally should not make the process session look as if a wire request was dispatched.

Because the request id itself is part of the serialized bytes, a suitable shape is:

1. read the current `next_request_id` without advancing it;
2. construct and encode the envelope once using that id;
3. build/measure the exact wire frame (`encoded UTF-8 bytes + one LF`);
4. reject locally if the complete frame exceeds 16 MiB;
5. only after successful preflight, advance `next_request_id` and dispatch the prepared frame;
6. then retain the existing response correlation/timeout behavior.

If implementation evidence shows consuming an id on a local encode failure is deliberately required by another invariant, document and test that instead of silently changing it. The default expectation is zero wire dispatch => no committed request id.

### Local failure versus transport contamination

An oversized request discovered before dispatch is a **local validation failure**:

- write zero bytes to child stdin;
- do not terminate/kill an otherwise healthy Pack process;
- do not wait for a response that can never arrive;
- do not consume a response belonging to an earlier completed wire request;
- the same `ProcessClient` must remain usable for a later valid request;
- expose a clear local `HostError` that names the Pack request wire limit.

By contrast, an I/O failure after a valid frame starts dispatching remains transport contamination. Preserve the existing fail-closed behavior that terminates the Pack process rather than trying to reuse a potentially partial JSONL stream.

### One framed write

Today payload and LF are separate `write_all()` calls. After local preflight, prefer a single prepared `Vec<u8>`/byte frame containing encoded JSON plus exactly one LF and one `write_all()` before `flush()`.

This does **not** make OS writes atomic; `write_all()` can still partially write before an error. It simply avoids treating the LF as a separate logical dispatch phase. Any write/flush error still requires terminating the child, as today.

`send_shutdown()` should use the same bounded/prepared frame rule or an equivalently proven path. Shutdown is tiny, but leaving a second unbounded request encoder/writer would make the sender invariant incomplete.

### Server receiver semantics to lock before relying on them

The Pack server is already bounded, but it lacks explicit exact-boundary regressions for the request side. Add deterministic tests around `read_bounded_line` / `serve_server_jsonl` that lock the existing contract before changing the sender:

1. a physical request record whose encoded JSON + LF is exactly the configured test limit is accepted;
2. the same frame at limit + 1 is rejected before request dispatch;
3. multibyte JSON content is judged in UTF-8 bytes;
4. no-newline input is bounded rather than accumulated indefinitely;
5. a malformed/oversized request remains process/server-fatal rather than producing a fabricated correlated response;
6. the request limit continues to count the LF inside the current 16 MiB Pack wire ceiling unless a separate protocol milestone explicitly changes that contract.

Prefer a small helper/test limit for unit tests instead of allocating 16 MiB repeatedly. Do not expose the test seam as production configuration.

### Process sender regressions

Add process/client-level regressions proving:

1. a request below the effective wire limit dispatches normally;
2. an exact-limit frame (encoded JSON + one LF) is accepted;
3. limit + 1 is rejected before any stdin bytes are produced;
4. multibyte request content is bounded by UTF-8 bytes, not character count;
5. local oversize does not kill/close the child and a later valid request succeeds on the same process;
6. no request handler/server-side mutation occurs for the rejected request;
7. local rejection does not consume/commit the request id expected by the next dispatched request;
8. accepted requests are encoded once and the same bytes are transported;
9. payload + LF is dispatched as one prepared frame;
10. a transport write/flush failure after dispatch starts still terminates the process;
11. response timeout, response-size bound, protocol-version correlation, request-id correlation, durable probe, content-pin and Pack conformance regressions remain green;
12. `send_shutdown()` cannot bypass the production request ceiling/path invariant.

### Validation

Run at minimum:

- focused `world-pack-process` tests;
- focused `world-pack-server` tests;
- `bash ./scripts/check-boundaries.sh`;
- `cargo fmt --all -- --check`;
- full Linux Clippy/workspace tests/Pack conformance;
- full macOS Library/Packs/GPUI/desktop tests, `World Machine.app` build/validate, packaged Analyst smoke, archive and upload if the normal path filter selects it.

Before merge, require exact-head review to check both sender and receiver together; do not review the new sender bound in isolation from the existing 16 MiB server behavior.

## M260 invariants to preserve

Do not regress the Analyst transport while working on Pack process framing:

- `AnalystJsonlClient` keeps its fixed 64 MiB **serialized JSON payload** ceiling;
- Analyst LF remains outside that payload budget;
- request serialization/UTF-8 encoding/size validation occurs before response-waiter registration;
- oversized or local JSON serialization failures write zero bytes, leave queued responses/waiters untouched, do not kill the child, and allow same-session reuse;
- accepted Analyst requests serialize once and write one prepared payload+LF frame;
- M255 response framing overflow/write-error/abort/SIGTERM→SIGKILL behavior remains unchanged;
- M259 Rust tool-host receiver stays fixed at its 64 MiB payload ceiling with its established CRLF/EOF/lone-CR/strict-UTF-8 behavior.

## Non-goals

No Pack protocol/version/schema change, no change to the current 16 MiB Pack request or response ceilings, no attempt to unify Pack and Analyst framing semantics, no request truncation, no Pack business behavior change, no World/query/UI changes, and no broad shared JSONL transport abstraction refactor.
