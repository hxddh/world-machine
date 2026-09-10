> **Status (2026-09-10):** The active plan is Phase VIII in [docs/ROADMAP.md](docs/ROADMAP.md) — `0.5 "A World that speaks for itself"`. Audit-driven transport hardening stays **frozen**. Every entry below states what a user will see change.

# Next Coding Task — 0.5, a World that speaks for itself

`v0.4.0` shipped on 2026-09-10 and made a World's story unbounded. Measuring what it actually reads like:

```text
80 periods of play: 586 lines of prose shown, 114 distinct  ->  81% already read
first repeated line: period 3
two Worlds, same seed, every answer opposite: 46% of their prose is identical
```

The wall at period 13 is gone; the wall at period 3 is not. A World's structure is infinite and its vocabulary is a lookup table of 114 lines, and every new trouble costs eighteen hand-written copy fields per seed.

Separately, three of three shipped `AgentRuntime` implementations are hand-written branching, `AgentDecision` is `{ action: String }`, and the app has no settings window at all — so "the inhabitants act on their own" means an if/else chain, and the only way to give a World a model is an environment variable and a `pi` CLI.

The items below are in dependency order. Everything except item 4's window can be finished from CI.

## The version

1. **Tiny Society ships in the app.** `build-app.sh` bundles `pocket-universe.worldpack` and `micro-company.worldpack` only. The README advertises Tiny Society, `v0.3.0` shipped its second consequence chain, and nobody who downloaded the app has ever been able to open it.
   *User-visible change:* the third World the README promises is actually there.
2. **The narrator seam.** A `world_narrated` Event carrying prose, recorded after a period's consequences resolve, preferred by the briefing and falling back to today's table whenever it is absent, empty, oversized, or slow. Deterministic structure untouched; the table stays the floor. Testable with a scripted narrator, no model involved.
   *User-visible change:* none yet on its own — this is the seam item 3 fills, and it ships with the fallback proven.
3. **A local model writes the news.** *(done)* The seam narrates every line a return shows, a Pi-backed narrator writes them in one request, and the Analyst settings carry a **World voice** switch that routes the already-persisted program into the Pack process. What decides and what narrates are separate settings, because the mind costs fifty-six requests per week-long catch-up and the voice costs one.
   Two measurements from building item 2 shape this. First, the 81% figure counts the whole page and over-states the problem: most of the page is a status panel reporting facts that have not changed, and rewording those every visit would make a stable fact look like it moved. Measured on the loop the product actually has — twenty week-long returns — the repetition that matters sits in the digest's event lines, each stuck at exactly three distinct phrasings (`Pressure is rising` 83%, `The pressure peaked` 82%, `Something was lost` 82%, `Decided without you` 85%), because there are three troubles per seed and one written line per stage. Second, the one-call-per-return budget is per *return*, not per line, so one request can carry every event the digest is about to show rather than only the latest consequence.
   *User-visible change:* what the World tells you happened is written about your World, not picked from a table, and two Worlds with the same seed stop sharing that prose. The status panel keeps saying plainly what is true.
4. **A settings window of its own, and a key for people without a CLI.** The World voice switch currently lives in the Analyst settings because that is where its program already lived; it deserves a place that is not about the Analyst. Where the app says how it reaches a model, what it costs, and that everything still works without one. The key lives in the macOS Keychain, never in a file.
   *User-visible change:* a person who has never opened a terminal can give their World a voice, or knowingly decline and lose nothing but the phrasing.
5. **Ship `v0.5.0`.**

## Only the owner can do these

They have not moved since `v0.2.1` and they outrank everything above if they produce findings.

0. **Apple Developer Program.** Enrol, create the Developer ID Application certificate, add the five secrets in [docs/RELEASE_SIGNING.md](docs/RELEASE_SIGNING.md).
   *User-visible change:* the "Open Anyway" detour disappears from the first launch.
1. **Real-device verification and screenshots.** Install `v0.4.0` on a physical Mac, walk [docs/INSTALL.md](docs/INSTALL.md), capture `docs/screenshots/home.png` and `docs/screenshots/world.png` in light and dark. The CI screenshot job rasterizes no text and cannot substitute.
   *User-visible change:* the README stops describing the app and shows it.
2. **Usability test.** Three to five non-developers, five minutes each. Phase VII and this plan both rest on code measurement: it can prove 81% of lines repeat, not whether a real person minds. Findings win over this file.

## Deliberately not doing

- A model deciding what happens rather than how it reads. Structure stays deterministic: the existing tests keep their meaning, and a bad generation costs a clumsy sentence rather than a broken World.
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
