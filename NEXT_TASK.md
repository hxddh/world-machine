# Next Coding Task — M260 Bound Analyst Tool Request Writes

M255 bounded the read-only tool-host stdout consumed by `AnalystJsonlClient`, and M259 now bounds the opposite Rust tool-host stdin direction to a fixed **64 MiB JSON payload-byte ceiling per request record**. The installed Node producer still has no sender-side preflight.

Today `integrations/pi/world-machine-analyst-client.mjs::AnalystJsonlClient.#writeLine()` does:

```js
const line = `${JSON.stringify(value)}\n`;
await new Promise((resolve, reject) => {
  this.child.stdin.write(line, "utf8", ...);
});
```

An oversized tool invocation is therefore serialized and handed to the pipe even though the M259 receiver will reject it. Add a local sender-side ceiling so an invalid request never crosses the process boundary.

## M260

Add a fixed **64 MiB maximum serialized JSON request payload** to `AnalystJsonlClient` before any stdin write.

The M259 receiver contract is authoritative:

- count the UTF-8 bytes of the serialized JSON payload;
- the trailing LF is framing and is outside the payload budget;
- do not truncate or rewrite tool input to fit;
- accepted requests should be serialized once and the same encoded payload should be used for transport rather than stringifying twice;
- production must not expose a CLI/env/runtime knob that can raise the 64 MiB ceiling.

A lower-only test seam is acceptable if needed for deterministic small tests, but it must never permit a limit above the production ceiling. Prefer keeping that seam local to the existing `AnalystJsonlClient` constructor/test harness rather than adding new user-facing configuration.

### Critical waiter-ordering invariant

Do **not** implement the size check only inside the current `#writeLine()` after `#roundTrip()` has already called `#nextLine()`.

Today `#roundTrip()` effectively does:

```js
const responsePromise = this.#nextLine();
try {
  await this.#writeLine(request);
} catch (error) {
  responsePromise.catch(() => {});
  ...
}
```

That ordering already has a local-encoding edge case: if `JSON.stringify()` throws before any request is written, the response waiter remains queued even though merely attaching `.catch()` observes its eventual rejection. A later valid response can be delivered to that stale waiter instead of the current call.

M260 must prepare/serialize/size-check the request **before registering a response waiter**. Oversized input and other purely local serialization failures must leave `this.waiters`, `this.lines`, the child pipe, and the session correlation state untouched.

A suitable shape is:

```js
const encoded = encodeBoundedRequest(request, maxRequestBytes);
const responsePromise = this.#nextLine();
await this.#writeEncodedLine(encoded);
```

where the encoder returns the already-serialized UTF-8 payload (string or Buffer) and transport appends exactly one LF without re-stringifying.

### Local failure semantics

An oversized request is a **local validation/serialization failure**, not framing contamination:

- do not set `closedError` or `framingError`;
- do not kill/terminate the child;
- do not create or leave a pending response waiter;
- do not consume any response already queued for a different completed request;
- do not write any request bytes;
- after the rejected request, a valid `listTools()` / `invoke()` on the same client must still succeed;
- existing remote tool errors stay correlated and recoverable exactly as today.

Likewise, a local `JSON.stringify` failure such as a `BigInt` or circular input must not leave a stale waiter. M260 may fix this adjacent existing ordering bug as part of moving all local request encoding before `#nextLine()`.

Transport write failures **after** a valid encoded request starts dispatching retain the existing M255 safety behavior; do not weaken the pending-promise observation or framing-contamination precedence fixes.

### Encoding / write shape

Avoid a second JSON serialization. One reasonable implementation is:

1. `JSON.stringify(request)` once;
2. encode that string to UTF-8 bytes (`Buffer.from(...)` or equivalent);
3. compare byte length with 64 MiB;
4. only after success, register the response waiter;
5. write the accepted payload plus one LF.

If payload and LF are written in separate low-level writes, explicitly reason about partial-frame failure: a payload write succeeding while the LF write fails contaminates the child stdin stream and must not be treated like a harmless local validation failure. A single framed write after preflight is simpler if it can be done without another JSON serialization.

### Required regressions

Use a small lower-only test limit; do not allocate tens of MiB in ordinary unit tests.

1. serialized payload below the test limit is dispatched normally;
2. exact-limit serialized payload is accepted and exactly one LF is written outside the payload budget;
3. limit + 1 serialized payload is rejected before any child stdin bytes are produced;
4. multibyte tool input is judged by UTF-8 byte length, not JavaScript string/code-point count;
5. oversized `invoke()` is local/non-fatal: child is not killed and client is not closed/poisoned;
6. no request callback/server-side parse occurs for the rejected oversized request;
7. a valid request immediately after an oversized request succeeds on the same child, proving no stale response waiter remains;
8. a local `JSON.stringify` failure likewise leaves no stale waiter and the next valid request succeeds;
9. `listTools()` and `invoke()` both use the same bounded preparation path;
10. accepted request is serialized once; transport does not call `JSON.stringify` again;
11. existing remote-tool-error recovery remains reusable;
12. existing response framing overflow, same-chunk fatal-wins, write-error observation, abort and SIGTERM→SIGKILL regressions remain green.

A source regression is reasonable if needed to lock the ordering: request preparation must appear before `#nextLine()` in the active round-trip path.

### Validation

Run:

- `node --test integrations/pi/tests/world-machine-analyst-client.test.mjs`;
- `bash ./scripts/check-pi-analyst.sh`;
- `cargo fmt --all -- --check`;
- full Linux boundary/Clippy/workspace tests/Pack conformance;
- full macOS Library/Packs/GPUI/desktop tests, `World Machine.app` build/validate, packaged Analyst smoke, archive and upload.

Because `integrations/pi/**` is in the existing macOS path filter, normal M260 PR CI should automatically execute the packaged-app gate.

## M259 invariants to preserve

Do not weaken the Rust receiver while adding sender preflight:

- `world-agent-tool-stdio` keeps its fixed 64 MiB production record ceiling;
- LF and CRLF framing compatibility remain outside the payload budget as established by M259;
- EOF-tail/lone-CR compatibility and strict UTF-8 behavior remain unchanged;
- no-newline overflow remains prompt/fatal without waiting for EOF once impossible;
- physical line numbers still include ignored blank lines;
- a valid earlier request stays independently committed/flushed when a later record overflows;
- M259 retains its real process-level production-boundary regression that writes 64 MiB + 1 while stdin remains open and requires the child to fail before EOF with no fabricated stdout response.

## Later audit

`world-pack-process` already has a bounded **response** reader (`DEFAULT_MAX_RESPONSE_BYTES = 16 MiB`). After M260, separately audit the Pack request sender and Pack server receiver together before choosing another transport-hardening milestone. Do not assume the response-side bound alone covers the opposite direction.

## Non-goals

No protocol/version/schema changes, no tool-input truncation, no Rust receiver changes, no response ceiling changes, no provider/model behavior changes, no World/Pack/query/UI changes, and no broad JSONL transport abstraction refactor.
