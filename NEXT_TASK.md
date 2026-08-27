# Next Coding Task — M254 Bound Pi Analyst RPC Record Framing

M214–M253 now provide the installed World Analyst path through immutable saved-World evidence, restricted long-lived Pi execution, provider-neutral turns, complete retained session evidence, bounded 4096-byte UI previews, incremental/virtualized panel history, retained successful turns, a bounded 64 MiB outer Analyst turn response frame, and collision-proof concurrent `world-cli` integration-test fixtures.

M252 bounded the Rust `world-analyst-client` response frame, but an earlier process boundary in the real Analyst path is still unbounded. `AnalystTurnHost` imports and uses `PiAnalystRpcSession` from `integrations/pi/world-machine-analyst-rpc.mjs`. That session consumes the long-lived Pi child process stdout.

## M254 — bound one Pi RPC JSONL record before buffering/parsing it

Today `PiAnalystRpcSession.#acceptChunk()` begins with:

```js
this.buffer = Buffer.concat([this.buffer, chunk]);
while (true) {
  const newline = this.buffer.indexOf(0x0a);
  if (newline < 0) return;
  // ...
}
```

If Pi, an extension, or a downstream tool causes stdout to emit a very large record or an indefinitely long byte stream without `\n`, the Node process repeatedly grows/copies `this.buffer` with no ceiling. M252 cannot protect this allocation: the Rust client only sees the later `AnalystTurnHost` response after `PiAnalystRpcSession` has already framed, parsed, accumulated, and normalized Pi output.

This is a transport-boundary problem, not a model/provider or turn-schema change.

### Required behavior

Add a production per-record ceiling for Pi RPC child stdout:

- default maximum: **64 MiB of JSON record payload bytes**;
- framing `\n` is not counted against the payload limit;
- preserve the existing optional trailing `\r` handling, so a max-size payload followed by `\r\n` is not rejected only because of the framing `\r`;
- measure bytes, not JavaScript string/code-point length;
- do not preallocate 64 MiB;
- never concatenate an entire incoming chunk before determining where its newline boundaries are;
- a chunk containing several normal records must keep all later records intact and in order;
- a record split across arbitrary child `data` chunks must remain valid;
- empty JSONL lines keep the current ignore behavior.

An oversized record must:

- fail before `JSON.parse`;
- surface as a fatal Pi RPC framing/protocol failure (`PiAnalystRpcProtocolError` is appropriate; do not invent a recoverable command error);
- poison the `PiAnalystRpcSession` so later `probe()` / `prompt()` calls cannot reuse it;
- terminate the contaminated child process rather than continuing to drain an attacker-controlled oversized record;
- release any buffered record chunks/references when the session is finished.

Do not truncate a record and attempt to parse it.

### Implementation shape

Keep the change primarily in `integrations/pi/world-machine-analyst-rpc.mjs`.

A test-injectable constructor limit is recommended, for example:

```js
const DEFAULT_MAX_RPC_RECORD_BYTES = 64 * 1024 * 1024;

constructor(child, { maxRecordBytes = DEFAULT_MAX_RPC_RECORD_BYTES } = {}) {
  // validate positive safe integer
}
```

`spawnRestricted()` should continue using the production default without exposing a new user-facing option.

Avoid the current repeated whole-buffer `Buffer.concat`. Prefer a bounded record accumulator that:

1. scans each incoming `Buffer` for `\n` before appending bytes;
2. retains only pieces belonging to the current incomplete record;
3. tracks pending byte count in O(1);
4. permits at most `maxRecordBytes + 1` pending raw bytes only to allow a possible final `\r` before `\n`;
5. concatenates the bounded pieces once when a complete record is available;
6. strips the optional final `\r`, then verifies the resulting payload is `<= maxRecordBytes` before UTF-8 conversion / `JSON.parse`.

If the pending stream exceeds the maximum possible valid record before a newline arrives, fail immediately; do not wait for the newline and do not drain the rest of the hostile frame.

### Required regressions

Extend `integrations/pi/tests/world-machine-analyst-rpc.test.mjs` using the existing fake child-process pattern and a small injected limit. Cover at least:

1. a valid record exactly at the byte limit followed by `\n` is accepted;
2. the exact-limit payload with `\r\n` remains accepted;
3. a record split across many stdout chunks remains accepted;
4. multiple records delivered in one stdout chunk remain ordered and usable;
5. a newline-terminated payload larger than the limit fails before JSON parsing;
6. a no-newline stream exceeding the limit fails promptly instead of waiting for prompt timeout/EOF;
7. oversize failure poisons the session and the child is terminated;
8. a valid record after an oversized prefix is never treated as recovery from the contaminated session;
9. all existing probe, prompt reuse, command-error, correlation, timeout, EOF, abort, and extension-error tests remain green.

The tests should not allocate tens of MiB: inject a small record limit.

### Validation

Run at minimum:

- `node --test integrations/pi/tests/world-machine-analyst-rpc.test.mjs integrations/pi/tests/world-machine-analyst-rpc-abort.test.mjs`;
- `bash ./scripts/check-pi-analyst.sh`;
- `cargo fmt --all -- --check`;
- Linux boundary / Clippy / workspace tests / external Pack conformance;
- because `integrations/pi/**` is already in the GPUI/macOS path filter, exact-head CI must also run the full macOS Library/Packs/GPUI/desktop tests and `World Machine.app` build/validate/packaged smoke/archive/upload.

### Scope audit / known follow-ups

The same audit found two other unbounded JSONL accumulation sites:

- `AnalystJsonlClient.#acceptChunk()` in `integrations/pi/world-machine-analyst-client.mjs` (Pi extension → restricted tool-host stdout);
- `jsonLines(stdin)` in `integrations/pi/world-machine-analyst-turn-host.mjs` (Rust client → turn-host stdin).

They are real follow-up boundaries, but do **not** mix them into M254. M254 should first make the long-lived Pi RPC child-output boundary correct, bounded, fatal-on-contamination, and well tested. Follow with separate milestones so each direction has explicit framing semantics and regression coverage.

Also do not add a cumulative Analyst-turn evidence budget in M254. Per-record Pi framing and cumulative turn accumulation are separate concerns.

## Non-goals

No Analyst turn protocol v1/schema changes, no provider/model/Pi command changes, no tool-result truncation, no UI preview changes, no World/Pack/query semantics changes, no archive changes, no `world-cli` stdin limit in this milestone, no global Node stream monkey-patching, and no broad JSONL utility refactor solely for deduplication.
