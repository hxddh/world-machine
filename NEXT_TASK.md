# Next Coding Task — M239 Reconcile Analyst Catalog After Pair Startup Drift

M214–M238 now provide the installed World Analyst path from immutable saved-World evidence through a restricted long-lived Pi analyst, provider-neutral turns, native runtime/model readiness, an in-memory Question → Answer → Evidence transcript, explicit recovery/new-comparison/cancellation/retry/dismiss flows, retained-question evidence scoping, background saved-World catalog refresh for fresh Setup transitions, and a first-open path that creates the analyst window before any Library enumeration.

## M239 — refresh a stale pair after World-dependent session startup failure

M238 removes the last synchronous `WorldLibrary::list()` from the analyst click/open path. One race remains between a successful catalog reconciliation and `DesktopAnalystSession::start`: the selected left/right World can be deleted or become unreadable after Setup declared the pair usable but before startup captures its immutable archive snapshots.

Today that startup error returns the panel to Setup while leaving the previously reconciled `documents` snapshot intact. The UI can therefore continue to display and select a pair that startup has just proven is no longer loadable until the user manually presses Recheck.

Make World-dependent startup drift feed back into the existing catalog refresh pipeline automatically, while keeping runtime/process startup failures distinct.

### Product behavior

- keep the typed `DesktopAnalystSessionError` through the background startup task until the GPUI completion handler decides recovery behavior;
- when startup fails because a selected World is missing or cannot be loaded from the Library, return to safe Setup and immediately run the existing background saved-World catalog refresh before selection or Start becomes usable again;
- if the refreshed catalog still contains a valid pair, reconcile the right selection with the existing deterministic M237 policy and then rerun runtime readiness;
- if the selected right disappeared, fall back deterministically and apply the existing M235 failed-question pair-change invalidation semantics;
- if the fixed left anchor disappeared, preserve the existing actionable close/reopen anchor error;
- if fewer than two usable saved Worlds remain, preserve the existing actionable insufficient-catalog error;
- if catalog refresh itself fails, preserve the M237 fail-closed catalog-error behavior with no stale selectable cards;
- runtime/process/probe/snapshot-write/serialization failures must continue to surface as startup errors and must not trigger an unrelated catalog refresh loop.

### State and concurrency rules

- reuse `refresh_saved_world_catalog`; do not introduce a second Library reconciliation path;
- the failed startup must fully relinquish `Starting`/busy state before beginning the refresh so the M237 generation/phase/session gate remains authoritative;
- no stale startup completion may overwrite Active/Fatal/newer state;
- catalog-triggering startup errors must not leave a partially started analyst process/session or cancellation handle behind;
- the automatic refresh is one recovery transition, not retry/backoff: a later startup is still an explicit user Start action;
- preserve M236 evidence-scope authority for successfully captured sessions; this task only handles failure before a live immutable pair exists.

### Implementation boundary

Keep the change in native desktop/session error plumbing. `DesktopAnalystSessionError` is already public; prefer matching its World-dependent variants in the panel instead of parsing display strings or broadening the analyst protocol.

Do not add provider/model behavior, Pi/filesystem authority, persistence, World mutation, Pack-specific logic, global Library watchers/caches, or automatic model-turn retries.

### Validation

Required gates:

- missing-right and missing-left startup failures route through catalog refresh rather than leaving the stale catalog interactive;
- non-Library startup failures remain visible and do not trigger catalog refresh;
- refresh reconciliation preserves an existing valid right, falls back deterministically when the right disappeared, and fails closed when the left/alternate set is unusable;
- no process/session/cancellation handle leaks across startup failure recovery;
- M230–M238 recovery/new-comparison/cancel/retry/dismiss/evidence-scope/catalog/initial-open regressions remain green;
- Linux boundary/fmt/Clippy/workspace/Pack conformance remain green;
- full macOS GPUI/desktop tests and `World Machine.app` build/validate/packaged smoke/archive/upload remain green.

## Non-goals

No global Library watcher/cache, no two-sided World selector yet, no automatic pair swapping, no persistent chat history, no automatic startup retry/backoff, no resumable model turn, no reconnect, no streaming, no concurrent turns, no protocol v2, no provider/model picker, no API-key storage, no new analyst tools, and no mutation authority.
