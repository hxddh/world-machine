# Next Coding Task — M257 Bound Rust Analyst Turn Request Writes

M254 bounds the long-lived Pi RPC child stdout, M255 bounds the restricted Analyst tool-host stdout, and M256 bounds the Node turn-host stdin receiver to **64 MiB of JSON payload bytes per request record**. The next adjacent boundary is the Rust sender in `crates/world-analyst-client/src/lib.rs`: `AnalystTurnClient::transact_with_response_limit()` currently serializes an `AnalystTurnRequest` and immediately writes all encoded bytes to the child pipe before any request-size preflight.

## M257

Add a **64 MiB maximum serialized Analyst turn request payload** on the Rust client side so oversized prompts are rejected locally before crossing the pipe.

The receiver ceiling established by M256 is authoritative for the installed path, so the Rust sender must use the same payload-byte definition:

- limit the serialized JSON bytes only;
- the trailing `\n` written by the transport is framing and is outside the payload budget;
- count serialized UTF-8 bytes, not Rust `String::len()` assumptions or user-visible character count;
- never truncate a prompt or request to fit;
- reject before the first writer `write_all()` / `flush()` call;
- avoid serializing a large accepted request twice merely to preflight it.

### Preserve local-invalid-request semantics

Request-size rejection is a **local request validation failure**, not a transport/session failure.

Existing `invalid_requests_are_rejected_locally_without_consuming_id` behavior is the model:

- the error must be non-fatal according to `is_session_fatal()`;
- the client must remain unpoisoned;
- writer bytes must remain untouched;
- the rejected request must **not consume `next_request_id`**;
- a subsequent valid request on the same client must still use the same request id that the rejected oversized request would have used and must be able to succeed normally.

Do not put the size check only inside the current `transact_with_response_limit()` after `ask()` / `probe()` have incremented `next_request_id`; that would silently violate the existing local-validation id semantics.

### Suggested implementation shape

Keep the change local to `world-analyst-client`.

Add a production constant such as:

```rust
pub const ANALYST_TURN_MAX_REQUEST_BYTES: usize = 64 * 1024 * 1024;
```

Give `AnalystTurnClient` a private request-size limit initialized to that constant. A `#[cfg(test)]` constructor/helper may lower it for small deterministic tests, but do not expose a runtime/user configuration knob that can raise or alter the installed production ceiling.

Prepare and encode the candidate request while `next_request_id` is still unchanged. A shape such as this is appropriate:

```rust
fn encode_bounded_request(
    request: &AnalystTurnRequest,
    max_bytes: usize,
) -> Result<Vec<u8>, AnalystTurnClientError>
```

The helper should serialize once, compare `encoded.len()` with the ceiling, and return the encoded bytes. Only after successful local validation/encoding should `ask()` or `probe()` advance `next_request_id` and dispatch those already-encoded bytes.

Split the current transaction path if needed so the transport stage accepts the encoded request bytes rather than calling `serde_json::to_vec()` again. Preserve all existing response framing, protocol/version/correlation checks, remote-error classification, poisoning, and response-size semantics.

Use either the existing `InvalidRequest` class with a clear request-size message or a dedicated `RequestTooLarge { max_bytes }` variant, but if a new variant is introduced it must be classified non-fatal and must not poison the client.

### Required regressions

1. an encoded request whose JSON payload is exactly the configured small test limit is accepted;
2. the transport still writes exactly one trailing LF outside that payload budget;
3. payload limit + 1 byte is rejected before any writer byte is produced;
4. multibyte prompt content is judged by serialized UTF-8 byte length rather than character count;
5. oversized local request is non-fatal and leaves `is_poisoned() == false`;
6. oversized local request does not consume the request id; the next valid request uses the same id and succeeds;
7. accepted request serialization is reused by the transport rather than serialized a second time;
8. probe continues to use the same bounded request path without changing normal probe behavior;
9. existing response-too-large, malformed-response, protocol/version/correlation, remote-command recovery, and fatal-response poisoning tests remain unchanged and green;
10. `AnalystTurnProcess` must not tear down/poison its child merely because the Rust caller attempted a locally oversized request.

Prefer small injected limits in tests; do not allocate tens of MiB merely to exercise the boundary.

### Validation

Run `cargo fmt --all -- --check`, focused `cargo test -p world-analyst-client`, Clippy/workspace tests, the Pi integration checks, Pack conformance, and the full macOS desktop / packaged Analyst `World Machine.app` gate because this Rust client participates in the shipped Analyst process path.

## M256 invariants to preserve

Do not weaken the receiver while adding sender preflight:

- `runAnalystTurnHost()` retains its existing production runner option surface; there is no input-limit or session-factory runtime option;
- `PiAnalystRpcSession.spawnRestricted` remains the only production session creation path for the turn host;
- the Node input reader keeps its fixed 64 MiB production ceiling, exact-limit LF/CRLF behavior, EOF-tail compatibility, streaming split/multi-record behavior, and fatal cleanup on framing overflow;
- a complete first JSONL request remains independent from a later oversized request even if both arrive in one OS chunk.

## Later audit candidate

After M257, audit `world-cli` machine-query stdin paths that still use whole-input `read_to_string` style accumulation and decide whether they need a separate bounded-input contract. Keep that separate from the Analyst transport milestones.

## Non-goals

No protocol/schema/provider/model changes, no prompt truncation, no Node turn-host receiver change, no response-size change, no UI/history/World/Pack/query/archive behavior change, and no broad shared transport/framing framework refactor.
