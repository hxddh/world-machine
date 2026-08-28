# Next Coding Task — M259 Bound Read-Only Tool-Host Stdin JSONL Framing

M255 bounded the read-only tool-host **stdout** consumed by `integrations/pi/world-machine-analyst-client.mjs`, but the other direction of the same process boundary remains unbounded. `crates/world-agent-tool-stdio/src/main.rs::serve_json_lines()` currently uses:

```rust
for (index, line) in reader.lines().enumerate() {
    let line_number = index + 1;
    let line = line.map_err(StdioAdapterError::Read)?;
    ...
}
```

`BufRead::lines()` internally accumulates one complete line before returning it, so a buggy/malicious producer can send an oversized or indefinitely unterminated request and make the long-lived tool host grow memory without a production cap before UTF-8/JSON/host-request validation.

The installed producer has been audited. `AnalystJsonlClient.#writeLine()` serializes requests as `JSON.stringify(value) + "\n"`, so normal production requests are LF-terminated. The receiver's existing compatibility semantics must nevertheless remain stable for direct/test callers.

## M259

Add a fixed **64 MiB JSON payload-byte ceiling per stdin request record** in `world-agent-tool-stdio` before UTF-8/JSON parsing.

This is receiver-side framing hardening only. Keep the protocol `world-machine-readonly-tools` v1 and all tool request/response schemas unchanged.

### Preserve current `BufRead::lines()` semantics

The replacement bounded reader must preserve the meaningful behavior of Rust `BufRead::lines()`:

- count bytes, not Rust/Unicode characters;
- LF is framing and does not consume the JSON payload budget;
- for CRLF records, the single `\r` immediately before LF is framing compatibility and is removed just as `lines()` does, so **exact 64 MiB JSON payload + CRLF is allowed**;
- a final non-empty EOF-tail without LF remains accepted when within the payload limit;
- importantly, a lone trailing `\r` at EOF is **not** CRLF framing and must remain part of the payload, matching `lines()`; do not silently strip it;
- preserve arbitrary underlying `BufRead` chunk splits;
- preserve multiple records in order;
- preserve current blank-line behavior: physical lines still advance line numbering, and records where `line.trim().is_empty()` are ignored;
- invalid UTF-8 remains a fatal stdin read/input failure; do not use lossy conversion.

### Overflow semantics

- never truncate and parse an oversized record;
- reject before `serde_json::from_str()` and before `host.handle_json()` for that record;
- a newline-terminated payload larger than the limit must fail;
- an EOF-tail larger than the limit must fail;
- an unterminated stream must fail once it can no longer become a valid record rather than waiting indefinitely for EOF;
- preserve one byte of conditional CR headroom: at `max + 1` raw bytes without LF, if the last byte is not `\r`, overflow is already certain and should fail immediately; if it is `\r`, one more byte may be needed to distinguish a valid exact-limit `payload + CRLF` from an oversized EOF/continued record;
- bounded accumulation should retain at most the payload ceiling plus the minimal CR framing headroom; do not preallocate 64 MiB for normal requests.

Framing overflow is a fatal stdio-adapter input failure. Do **not** fabricate a correlated protocol response because a partial/oversized record may not contain a trustworthy `call_id` or request shape. Let `serve_json_lines()` return an error so the binary exits non-zero through the existing top-level error path.

A previously complete independent request remains complete: if request 1 has already been parsed, handled, written and flushed, and request 2 is oversized, request 1's response must remain emitted before the process fatally stops on request 2. Behavior must not depend on whether the OS happened to coalesce both requests into one underlying read buffer.

### Suggested implementation shape

Keep the implementation local to `crates/world-agent-tool-stdio/src/main.rs`; do not introduce a broad shared JSONL framework.

Add a fixed production constant such as:

```rust
const MAX_TOOL_REQUEST_BYTES: usize = 64 * 1024 * 1024;
```

Replace `reader.lines()` with a small private bounded record reader over `BufRead`. A `fill_buf()` / `consume()` loop is appropriate because it can:

- scan for LF before copying arbitrary amounts;
- enforce the precise `max + optional CR` headroom rule;
- stop promptly on an unterminated overflow;
- preserve bytes until strict UTF-8 conversion;
- avoid consuming bytes belonging to the next record.

A test-only/helper `max_bytes` parameter may lower the limit for deterministic tests. The production `serve_json_lines()` path must always use the fixed 64 MiB constant; do not add CLI/env/runtime configuration that can raise or alter it.

Preserve physical line numbers. If the bounded helper returns one physical record at a time, increment the line counter before blank-line filtering just as the existing `enumerate()` does. A dedicated `RecordTooLarge { line, max_bytes }` adapter error is reasonable; invalid UTF-8 may continue to map through `StdioAdapterError::Read` with `io::ErrorKind::InvalidData` or an equivalent clear fatal input error.

### Required regressions

Use a small helper limit; do not allocate tens of MiB.

1. below-limit request + LF is accepted;
2. exact-limit payload + LF is accepted;
3. exact-limit payload + CRLF is accepted and the framing CR is removed;
4. exact-limit payload at EOF without LF is accepted;
5. an EOF-tail ending in a lone `\r` preserves that `\r` as payload rather than stripping it;
6. a request split across many underlying chunks is reconstructed exactly;
7. multiple requests from one buffer remain ordered;
8. blank/whitespace-only lines remain ignored while still preserving physical line numbers for later errors;
9. newline-terminated limit + 1 payload bytes fail before JSON/host dispatch;
10. no-newline overflow fails promptly once impossible, without requiring EOF;
11. exact-limit payload followed by a pending `\r` can wait for the next byte and succeeds if that byte is LF;
12. oversized EOF tail fails;
13. invalid UTF-8 within the size limit is rejected rather than lossily decoded;
14. a valid first request followed by an oversized second request emits/flushed the first response, then fails before dispatching the second;
15. existing list-tools/invoke protocol behavior and blank-line framing test remain green.

Add source/regression coverage showing production `serve_json_lines()` no longer calls `reader.lines()` for request framing and uses the fixed production ceiling.

### Validation

Run:

- `cargo fmt --all -- --check`;
- `cargo test -p world-agent-tool-stdio`;
- package Clippy with `-D warnings`;
- `bash ./scripts/check-boundaries.sh`;
- `bash ./scripts/check-pi-analyst.sh`, because the installed Analyst path launches this tool host;
- full workspace Linux tests + Pack conformance;
- full macOS Library/Packs/GPUI/desktop/`World Machine.app` build, packaged Analyst smoke, archive and artifact upload.

Because `crates/world-agent-tool-stdio/**` is already included in CI's `gpui` path filter, normal M259 PR CI should automatically exercise the full macOS packaged gate.

## M258 invariants to preserve

Do not mix machine-query CLI changes into M259:

- `world-cli` stdin `-` requests remain bounded to 64 MiB raw EOF-document bytes via the M258 helper;
- M258 has a process-level regression that streams the production 64 MiB + 1 boundary with parent stdin still open and requires the child to fail before EOF, emit no success envelope, and fail before archive/query execution; preserve that coverage;
- direct inline world-cli request JSON remains unchanged;
- query protocols/schemas and archive behavior remain unchanged.

## Later adjacent audit

After M259, audit the **producer side** of this same tool boundary separately. `AnalystJsonlClient.#writeLine()` currently does `JSON.stringify(value) + "\n"` with no pre-write request-size ceiling. Once the Rust receiver ceiling is authoritative, a later milestone can decide whether to reject oversized tool requests locally before crossing the pipe, preserving client/session semantics. Do not bundle that sender change into M259.

## Non-goals

No protocol/version/schema changes, no tool-input truncation, no tool-output changes, no Node sender preflight in M259, no query/World/Pack/UI changes, no broad shared JSONL abstraction, and no unrelated refactors.
