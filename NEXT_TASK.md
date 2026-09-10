> **Status (2026-09-10):** The active plan is Phase VII in [docs/ROADMAP.md](docs/ROADMAP.md) — `0.4 "Worlds that don't end"`. Audit-driven transport hardening stays **frozen**. Every entry below states what a user will see change.

# Next Coding Task — 0.4, the era engine

`v0.3.0` shipped on 2026-09-08 and completed every CI-only item on the old list. Measuring what it actually produced showed the problem worth a whole version:

**A World's entire content is thirteen periods, and then it is inert forever.** Taking every choice as it appears, all four Pocket Universe chapters complete by period 13; for the next forty-seven periods the command list is exactly `["pocket-universe.nudge"]`. Separately, a World that is never given a choice never moves at all — sixty periods leave it at `legacy=forming`. The product's premise is persistent Worlds that keep living; the measurement says they stop.

The items below build the era engine that removes that wall, in dependency order. A fifth chapter is explicitly **not** on this list: it would move the wall, not remove it.

## The version

1. **The era engine.** *(in progress)* Make the four chapters the stages of one era, and make eras loop: succession settling begins the next era with the successor as the new anchor-keeper. Carry a `pressure_kind` so each era can face a different threat, and select it deterministically from a per-seed pool that never repeats back to back. Give each seed a second threat so the second era genuinely differs.
   *User-visible change:* a World that finished its story has a next thing to decide instead of one dead button.
2. **A third threat per seed, and history read back.** Fill the pool to three per seed, and make an era's reading depend on the eras before it — a World that released three times running offers a different fourth era than one that entrusted three times.
   *User-visible change:* the fourth era does not read like the first.
3. **Drift: a World that moves without you.** Every decision point gets a deadline and a default. Staying away means the World decides, durably, and the return briefing says which decisions it made for you.
   *User-visible change:* leaving a World alone changes it, instead of pausing it.
4. **Pacing and the era briefing.** Re-tune the six-hour period and seven-period catch-up cap for unbounded content, and make `While you were away` summarize an era rather than list periods.
   *User-visible change:* coming back after a week is different from coming back after two days.
5. **Ship `v0.4.0`.**

## Only the owner can do these

They have not moved since `v0.2.1` and they still outrank everything above if they produce findings.

0. **Apple Developer Program.** Enrol, create the Developer ID Application certificate, add the five secrets in [docs/RELEASE_SIGNING.md](docs/RELEASE_SIGNING.md).
   *User-visible change:* the "Open Anyway" detour disappears from the first launch.
1. **Real-device verification and screenshots.** Install the current release on a physical Mac, walk [docs/INSTALL.md](docs/INSTALL.md), capture `docs/screenshots/home.png` and `docs/screenshots/world.png` in light and dark. The CI screenshot job rasterizes no text and cannot substitute.
   *User-visible change:* the README stops describing the app and shows it.
2. **Usability test.** Three to five non-developers, five minutes each. The plan above rests on code measurement: it can prove a World goes inert at period 13, but not whether a real person leaves at period 3. Findings win over this file.

## Deliberately not doing

- Multi-version Packs, World migration, frozen crates, Pack protocol changes: Phase VII keeps content additive instead, so version bumps stay rare.
- A background daemon or notifications. Pillar 3 fixes "the World does not move"; "the World does not move while the app is closed" is a macOS background-execution and notarization problem and waits for notarization.
- AI-generated content, Windows and Linux, a Pack marketplace.
- Further transport, scheduler, or Pack-protocol hardening without a reported failure.

The previous milestone text is kept below for reference.

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
