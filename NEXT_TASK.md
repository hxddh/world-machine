> **Status (2026-09-10):** Phase VIII shipped as `v0.5.0`. The next entries should come from the owner's real-device verification and usability test rather than from this file. Audit-driven transport hardening stays **frozen**. Every entry below states what a user will see change.

> **Read [docs/PRODUCT_STUDY.md](docs/PRODUCT_STUDY.md) first.** It measures the
> shipped Worlds against the products this one competes with, and it disagrees
> with the ordering below: Tiny Society stops producing events at visit 77,
> Micro Company at visit 2, and the one World that keeps going gets
> quadratically slower as it does. Those are the findings this file was waiting
> for; they did not need a device or a stranger to produce.

# Next Coding Task — after v0.5.0

`v0.5.0` shipped on 2026-09-10 and closed Phase VIII. Measured on the loop the product actually has — leave for a week, come back, read the briefing, twenty times over:

```text
  Pressure is rising   18 lines /  3 distinct (83% repeat)  ->  18 distinct (0%)
  The pressure peaked  17 lines /  3 distinct (82% repeat)  ->  17 distinct (0%)
  Something was lost   17 lines /  3 distinct (82% repeat)  ->  17 distinct (0%)

  whole return page:  61% repeat  ->  40% repeat
```

No Pack version moved, so no World was closed by the release.

## Shipped in 0.5

1. **Tiny Society ships in the app.** *(shipped)* Advertised since before `v0.2.0` and unreachable until now.
2. **The narrator seam.** *(shipped)* Prose recorded as a durable Event the briefing prefers, with the built-in copy as the floor under it.
3. **A local model writes the news.** *(shipped)* Every line a return shows, in one request; what decides and what narrates are separate settings because the mind costs fifty-six requests per week-long catch-up and the voice costs one.
4. **A Settings window, and a key for people without a CLI.** *(shipped)* ⌘, opens it; the key lives in the login keychain and never in a process argument.
5. **Ship `v0.5.0`.** *(shipped)*

## Only the owner can do these

They have not moved since `v0.2.1` and they outrank everything below if they produce findings.

0. **Apple Developer Program.** Enrol, create the Developer ID Application certificate, add the five secrets in [docs/RELEASE_SIGNING.md](docs/RELEASE_SIGNING.md).
   *User-visible change:* the "Open Anyway" detour disappears from the first launch.
1. **Real-device verification and screenshots.** Install `v0.5.0` on a physical Mac, walk [docs/INSTALL.md](docs/INSTALL.md), open the new Settings window, and capture `docs/screenshots/home.png` and `docs/screenshots/world.png` in light and dark. The CI screenshot job rasterizes no text and cannot substitute — and nothing in 0.5 could verify a window's *layout* anywhere, only that it compiles and behaves.
   *User-visible change:* the README stops describing the app and shows it.
2. **Usability test.** Three to five non-developers, five minutes each. Two versions have now rested entirely on code measurement. It can prove the lines stopped repeating; it cannot tell you whether a stranger switches the voice on, or whether the prose it produces is actually good.
   *User-visible change:* none directly; it is what turns the rest of this list from guesses into findings.

## Known and waiting, if no findings arrive

3. **Is generated prose actually better?** Everything measured in 0.5 is structural — that the lines differ, not that they read well. Nobody has judged the output of a real model against the hand-written copy. That judgement needs a person, and it may send content work in a different direction entirely.
   *User-visible change:* whatever the judgement finds.
4. **The other Packs still end.** The era engine is Pocket Universe's, and so is the voice. Tiny Society has two consequence chains and then stops; Micro Company has one arc.
   *User-visible change:* a Tiny Society World is still worth opening on the fifth visit.
5. **A second Tiny Society branch worth taking.** There is still only one durable fork deciding both chains.
   *User-visible change:* two different Tiny Society Worlds diverge on more than one choice.

## Deliberately not doing

- A model deciding what happens rather than how it reads. Structure stays deterministic: a bad generation costs a clumsy sentence rather than a broken World, and that is what makes a voice safe to switch on.
- Generating a World's rules, a Pack marketplace or remote catalog, multi-version Packs and World migration, Windows and Linux.
- A background daemon or notifications; that waits on notarization.
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
