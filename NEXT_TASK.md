> **Status (2026-09-08):** Audit-driven transport hardening stays **frozen**; M263 (#266) was the last such milestone. The active plan is Phase VI in [docs/ROADMAP.md](docs/ROADMAP.md), whose progress table lists what is done and what remains. Every entry below states what a user will see change.

# Next Coding Task — after v0.3.0

`v0.3.0` shipped on 2026-09-08. The list below is ordered by what it changes for somebody using the app, and each entry says whether it can be finished from CI alone.

## Only the owner can do these

0. **Apple Developer Program.** Enrol, create the Developer ID Application certificate, and add the five secrets in [docs/RELEASE_SIGNING.md](docs/RELEASE_SIGNING.md). The next release is then a normal macOS app: download, open, use. Nothing else on this list removes the first-launch dialog, and no packaging trick substitutes for it.
   *User-visible change:* the "Open Anyway" detour disappears from the first launch.
1. **Real-device verification and screenshots.** Install `v0.3.0` on a physical Mac (macOS 14 and 15 if possible), walk [docs/INSTALL.md](docs/INSTALL.md), open Home and a World window in light and dark, and confirm nothing lays out past the window edge. Capture `docs/screenshots/home.png` and `docs/screenshots/world.png`. The CI screenshot job draws layout but rasterizes no text inside Apple Virtualization, so this cannot be automated — do not spend more time on `screenshots.yml`.
   *User-visible change:* the README stops describing the app and shows it.
2. **Usability test.** Three to five non-developers, five minutes each, from download to a progressing World. Every stall becomes an entry here with the user-visible change it fixes.
   *User-visible change:* none directly; it is what turns the rest of this list from guesses into findings.

Anything the owner reports from 1 or 2 outranks everything below.

## Can be finished without a Mac in hand

3. **Name and remove Worlds from Home.** *(done, shipped in `v0.2.2`)* Branching produces Worlds quickly, and until now every one of them was listed under its World Pack's title with only a file id to tell them apart, with no way to remove one except in the Finder. Every World card now carries **Rename** and **Remove**; removal moves the file into a `Removed` folder rather than deleting it. One unreadable file in the Worlds folder no longer hides every other World.
   *User-visible change:* My Worlds becomes a list of named Worlds the owner can prune.
4. **A World window that says which World it is.** *(done, shipped in `v0.2.2`)* The window title, header, save and reload lines, and the file name Save As suggests now carry the World's name, with the durable file id kept beside it.
   *User-visible change:* the renamed World is called by its name everywhere it appears.
5. **Sort and find in My Worlds.** *(done, shipped in `v0.3.0`)* Past six Worlds the list carries Find a World and an Order switch.
   *User-visible change:* a long list of Worlds stays usable.
6. **A fourth Pocket Universe chapter, and a second Tiny Society consequence chain.** *(done, shipped in `v0.3.0`)* Pocket Universe 0.17 adds succession, with no deadline and a successor whose habits deepen while the observer waits. Tiny Society 0.2 adds an employment chain that reads the first chain's result: recovered demand across a lean counter earns Mia her first job.
   *User-visible change:* `While you were away` keeps having something to say on the fifth visit.
7. **Window size and position remembered between launches.** *(done, shipped in `v0.3.0`)*
   *User-visible change:* the app opens where it was left.

## What is left after v0.3.0

Everything on the CI-only list is shipped. The next entries should come from the owner's real-device verification and the usability test rather than from this file, per the rule at the end of [docs/ROADMAP.md](docs/ROADMAP.md). Two things are known and waiting:

8. **Carrying a World across a Pack version change.** `v0.3.0` moved both Pack versions and closed every Pocket Universe and Tiny Society World saved before it. The versioning model already allows several versions of one Pack to be installed at once, so the app could ship the previous version alongside the new one and let old Worlds keep their old rules. Until it does, every content release costs everybody their Worlds.
   *User-visible change:* a World keeps opening after an update that changes its Pack's rules.
9. **A second Tiny Society branch worth taking.** The employment chain reads the first chain's outcome; there is still only one durable fork (repair the boat or not) that decides it.
   *User-visible change:* two different Tiny Society Worlds diverge on more than one choice.

## Deliberately not doing

- Further transport, scheduler, or Pack-protocol hardening without a reported failure (see the Phase VI freeze).
- Any new runtime primitive that no two Worlds would use (see the end of [docs/ROADMAP.md](docs/ROADMAP.md)).
- Homebrew, `curl | sh`, or other install routes around notarization; the owner has ruled these out and none of them removes the Gatekeeper step.

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
