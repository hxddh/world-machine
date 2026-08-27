# Next Coding Task — M252 Bound Analyst Turn Response Framing

M214–M251 now provide the installed World Analyst path from immutable saved-World evidence through a restricted long-lived Pi analyst, provider-neutral turns, native runtime/model readiness, complete authoritative in-memory Question → Answer → Evidence exchanges, explicit recovery/new-comparison/cancellation/retry/dismiss flows, retained-question evidence scoping, asynchronous catalog loading/refresh, typed drift reconciliation, two-sided selection/Swap, local filtering, stable Pack/document/pair identity, bounded 4096-byte UTF-8-safe UI evidence previews, incremental panel history projection, variable-height virtualized GPUI history rendering, and a retained/no-copy panel ask path that moves each successful raw `AnalystTurn` directly into session exchanges.

M248 intentionally bounds only the panel's display copy. M251 intentionally keeps one complete accepted raw `AnalystTurn` per successful exchange. The remaining unbounded memory path is earlier, at the NDJSON transport framing boundary before any protocol or JSON validation happens.

## M252 — cap one Analyst transport response before JSON/protocol validation

`crates/world-analyst-client/src/lib.rs` currently reads a complete host response with an unbounded `String`:

```rust
let mut line = String::new();
let read = match self.reader.read_line(&mut line) {
    Ok(read) => read,
    Err(error) => {
        self.poisoned = true;
        return Err(AnalystTurnClientError::ReadResponse(error));
    }
};
if read == 0 {
    self.poisoned = true;
    return Err(AnalystTurnClientError::UnexpectedEof);
}

let value: Value = serde_json::from_str(&line)?;
```

A malformed, buggy, or compromised turn host can therefore emit an arbitrarily large line and force the Rust client to keep allocating until newline/EOF **before** response shape, protocol version, correlation ID, or JSON validity is checked.

This is a transport-framing safety problem, not a UI preview problem. Accepted responses must remain complete; oversized transport frames should be rejected rather than truncated into a fake valid turn.

### Product behavior

- impose a generous **64 MiB maximum JSON payload size per Analyst response frame**;
- the size limit excludes the optional trailing `\n` delimiter, so exactly 64 MiB of JSON plus one newline is valid;
- preserve current support for a valid EOF-terminated JSON response that has no final newline, as long as it is within the limit;
- a response whose JSON payload exceeds the ceiling fails closed with a dedicated client error and poisons the Analyst client/session;
- do **not** truncate an oversized response and attempt to parse it;
- accepted responses below/equal to the ceiling continue through the exact existing JSON shape, protocol/version, correlation, and remote-error validation;
- accepted `AnalystTurn` values remain complete raw evidence and continue to be retained exactly once by M251;
- no protocol version bump and no turn schema changes.

Define a public transport constant alongside the existing protocol constants, for example:

```rust
pub const ANALYST_TURN_MAX_RESPONSE_BYTES: usize = 64 * 1024 * 1024;
```

The 64 MiB ceiling is deliberately far above normal response sizes and far above the 4096-byte M248 UI preview. It is a runaway-frame guard, not an intended evidence-size target.

### Bounded framing implementation

Do not call `BufRead::read_line` into an unbounded `String`.

Extract a small private helper whose limit is parameterized for unit tests, for example:

```rust
fn read_bounded_response_line<R: BufRead>(
    reader: &mut R,
    max_bytes: usize,
) -> Result<Option<Vec<u8>>, AnalystTurnClientError>
```

Recommended semantics:

- read at most `max_bytes + 1` bytes from the underlying `BufRead` using a limited reader and `read_until(b'\n', ...)`;
- if nothing is read, return `Ok(None)` so the existing transaction path maps that to `UnexpectedEof`;
- if the collected bytes end in `\n`, treat that delimiter as framing and exclude it from the payload-byte count;
- therefore `max_bytes` JSON bytes + `\n` is allowed;
- if there is no newline because the underlying reader reached EOF, a payload of exactly `max_bytes` is also allowed;
- if `max_bytes + 1` payload bytes are observed before newline/EOF, return the dedicated oversized-frame error;
- after an oversized frame, the client is poisoned, so there is no need to drain the remainder of the hostile frame;
- the helper's collected buffer length must be bounded by `max_bytes + 1`; do not preallocate the full 64 MiB on every normal response.

`std::io::Take<&mut R>` supports the required bounded read while preserving bytes after a normal newline for the next transaction. Keep the normal multi-request long-lived session behavior intact.

After framing, parse directly from bytes with `serde_json::from_slice(&line)` rather than first constructing another UTF-8 `String`. JSON parsing still validates UTF-8 and should map malformed content to the existing `InvalidResponseJson` path.

It is fine to split the transaction function so production always uses `ANALYST_TURN_MAX_RESPONSE_BYTES` while unit tests can exercise a tiny limit without allocating tens of MiB, for example:

```rust
fn transact(...) -> Result<AnalystTurnResponse, AnalystTurnClientError> {
    self.transact_with_response_limit(
        request,
        expected_id,
        ANALYST_TURN_MAX_RESPONSE_BYTES,
    )
}
```

Do not expose a runtime/user setting for this slice.

### Error semantics

Add a distinct error such as:

```rust
ResponseTooLarge { max_bytes: usize }
```

Requirements:

- `Display` clearly states that the Analyst turn response exceeded the transport limit;
- `Error::source()` returns `None` for it;
- it is session-fatal under the existing `is_session_fatal()` classification;
- the client sets `poisoned = true` before returning it;
- `AnalystTurnProcess` should therefore continue using its existing poisoned-client teardown path without new process policy.

Do not turn an oversized response into `RemoteCommand`, because the frame could not be trusted/decoded enough to classify it as a provider-neutral command rejection.

### State / correctness invariants

- protocol remains `world-machine-analyst-turns` version 1;
- `AnalystTurn`, `AnalystToolCall`, runtime errors and remote errors keep their exact wire schema;
- strict top-level response shape validation remains after bounded framing;
- protocol version and request/response correlation remain strict and unchanged;
- nonfatal correlated command errors remain reusable/non-poisoning;
- malformed JSON, invalid shape, protocol/version/correlation failures remain poisoning/fatal exactly as today;
- accepted full raw evidence is not truncated or summarized;
- M251 retained/no-copy ownership remains unchanged;
- M248 panel 4096-byte previews remain the only presentation-size bound;
- no desktop/session/Library/Pack/World behavior changes should be required.

### Validation

Required regressions in `world-analyst-client`:

- bounded framing accepts a payload below the supplied test limit;
- exactly `limit` payload bytes followed by newline is accepted;
- exactly `limit` payload bytes followed by EOF is accepted, preserving current EOF-terminated response behavior;
- `limit + 1` payload bytes are rejected as oversized;
- the oversized helper path reads/collects no more than `limit + 1` bytes, so tests prove bounded framing without constructing a 64 MiB fixture;
- a normal newline stops the first bounded read and leaves a second response frame available for the next read/transaction;
- an oversized transaction returns the dedicated error, poisons the client, and the next ask/probe returns `Poisoned`;
- a valid provider-neutral result under the limit decodes exactly as today;
- malformed JSON, unknown fields, protocol/version/correlation, remote command/fatal, invalid-request, process teardown and reusable-session regressions remain green;
- M251 desktop retained/no-copy tests remain green;
- Linux boundary/Pi/fmt/Clippy/workspace/Pack gates remain green;
- full macOS Library/Packs/GPUI/desktop tests plus `World Machine.app` build/validate/packaged smoke/archive/upload remain green.

## Non-goals

No UI history limit, no evidence truncation, no 4096-byte preview change, no provider/model changes, no response compression, no streaming partial `AnalystTurn`, no protocol v2, no runtime-configurable frame limit, no request-size redesign, no new Pi tools, no reconnect/resume, no session concurrency changes, no selector/catalog changes, and no Pack/World behavior changes.
