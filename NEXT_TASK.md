# Next Coding Task — M240 Recheck Analyst Runtime After Spawn Drift

M214–M239 now provide the installed World Analyst path from immutable saved-World evidence through a restricted long-lived Pi analyst, provider-neutral turns, native runtime/model readiness, an in-memory Question → Answer → Evidence transcript, explicit recovery/new-comparison/cancellation/retry/dismiss flows, retained-question evidence scoping, background saved-World catalog refresh, asynchronous first-open catalog loading, and automatic catalog reconciliation when a selected World disappears or becomes unreadable between Setup and immutable snapshot capture.

## M240 — refresh stale runtime readiness after process spawn drift

M239 closes the saved-World side of the Setup → Start race while deliberately leaving runtime/process failures distinct. A symmetric runtime race remains: Setup may hold `DesktopAnalystRuntimeReadiness::Ready` with resolved Node/Pi/turn-host/tool-host paths, then one of those executable/runtime files can disappear, lose executable/read permissions, or otherwise become unlaunchable before `DesktopAnalystSession::start` spawns the analyst process.

Today a `DesktopAnalystSessionError::Spawn` returns the panel to Setup with `runtime = None` and the startup error visible, but the user must manually press Recheck before the panel reconstructs current runtime readiness and actionable path controls.

Make spawn-time runtime drift automatically flow into the existing runtime-readiness pipeline once, without retrying Start and without conflating probe/protocol/model failures with filesystem/executable readiness.

### Product behavior

- keep using the typed `DesktopAnalystSessionError` preserved by M239;
- when startup fails specifically with `DesktopAnalystSessionError::Spawn`, return fully to safe idle Setup and immediately rerun the existing asynchronous runtime discovery/readiness check;
- while that check runs, Start and runtime path controls remain unavailable through the existing `runtime_checking` gate;
- if readiness now reports a missing/unexecutable Node, Pi, analyst tool host, launcher dependency, or runtime resource, show the existing actionable `DesktopAnalystRuntimeIssue` and existing path controls;
- if readiness is still Ready, leave the panel ready for a later explicit Start; do not automatically retry session startup;
- do not refresh the saved-World catalog for a Spawn error; M239 owns Library drift independently.

### Error classification boundary

Do **not** broaden automatic runtime recheck to every startup failure.

- `MissingWorld` / `LoadWorld` remain M239 catalog-refresh cases;
- `SameWorld`, archive serialization, snapshot-directory/snapshot-write failures remain visible startup errors;
- `DesktopAnalystSessionError::Client` must remain visible and must **not** be reclassified as simple runtime readiness. Startup probe/client failures can represent selected-model rejection, protocol/correlation contamination, malformed responses, or transport failures that the current filesystem/executable readiness check does not diagnose;
- shutdown/cancel/closed/fatal-session errors remain outside this Setup spawn-drift recovery path.

Do not parse error display strings. Match the typed `Spawn` variant only.

### State and concurrency rules

- reuse `refresh_runtime`; do not create a second runtime discovery implementation;
- the failed startup must relinquish `Starting` and `busy`, clear cancellation/session state, and enter Setup before `refresh_runtime` begins so its existing single-flight guards remain authoritative;
- preserve the M239 startup-completion stale-result gate; an obsolete completion must not start a runtime check or overwrite newer panel state;
- no process/session/cancellation handle may survive the failed startup transition;
- the automatic runtime check is a single reconciliation transition, never retry/backoff and never automatic Start;
- catalog and runtime refreshes must remain mutually exclusive through the existing busy/runtime/catalog gates.

### Implementation boundary

Keep the change in native desktop product state. Do not change analyst protocol v1, provider/model selection, Pi authority, persistence, World truth, Pack behavior, settings format, or introduce a runtime daemon/watcher/global cache.

Prefer a small typed helper such as `startup_error_requires_runtime_refresh` beside M239's catalog classifier, with tests proving the two classifiers are mutually exclusive for all relevant startup variants.

### Validation

Required gates:

- `Spawn` startup failure triggers the existing asynchronous runtime readiness check after Setup is restored;
- MissingWorld/LoadWorld continue to trigger catalog refresh and not runtime refresh;
- Client/probe, snapshot, serialization, SameWorld and other non-Spawn failures remain visible and do not trigger runtime refresh;
- no automatic session retry occurs after readiness becomes Ready again;
- stale startup completion cannot trigger a runtime refresh;
- M230–M239 recovery/new-comparison/cancel/retry/dismiss/evidence-scope/catalog/initial-open/startup-drift regressions remain green;
- Linux boundary/fmt/Clippy/workspace/Pack conformance remain green;
- full macOS GPUI/desktop tests and `World Machine.app` build/validate/packaged smoke/archive/upload remain green.

## Non-goals

No global Library/runtime watcher or cache, no two-sided World selector yet, no automatic pair swapping, no persistent chat history, no automatic startup retry/backoff, no resumable model turn, no reconnect, no streaming, no concurrent turns, no protocol v2, no provider/model picker, no API-key storage, no new analyst tools, and no mutation authority.
