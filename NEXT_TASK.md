# Next Coding Task — M237 Refresh Saved-World Catalog for a Fresh Analyst Comparison

M214–M236 now provide the installed World Analyst path from immutable saved-World evidence through a restricted long-lived Pi analyst, stable provider-neutral turns, native runtime/model readiness, an in-memory Question → Answer → Evidence transcript, explicit fatal recovery, clean New Comparison lifecycle, explicit cancellation/retry/dismissal of retained failed questions, and evidence-scope protection so a retained question can never be retried against changed archive snapshots.

## M237 — refresh the saved-World catalog before a fresh comparison

The analyst panel currently snapshots `WorldLibrary::list()` only when the window first opens. M231 can return the same window to Setup through `New comparison`, and M230 can return to Setup through Fatal recovery, but neither path refreshes `documents`.

That creates stale native product state after the Library changes elsewhere while the analyst window stays open: newly created/imported Worlds do not appear, deleted Worlds remain selectable, and titles/summaries/timestamps shown in Setup can be stale. Session startup still loads the current archives by id and therefore fails safely, but the selection surface itself is no longer an accurate view of the Library.

Add a narrow background catalog refresh for fresh Setup transitions.

### Product behavior

- when `New comparison` closes successfully, reload the saved-World summaries before presenting the next usable comparison selection;
- when Fatal recovery returns to Setup, reload the saved-World summaries as part of the fresh recovery path;
- keep the current left document fixed as the analyst anchor;
- if the currently selected right World still exists and differs from left, preserve it;
- if the selected right World disappeared, choose the same deterministic `default_right_for` policy used when the panel first opens;
- if fewer than two saved Worlds remain, stay in Setup with an actionable error and do not start a session;
- newly created/imported Worlds and updated display metadata become visible without closing the analyst window;
- no active analyst session may observe its `documents` catalog mutate underneath it.

### Concurrency and state rules

- Library listing must run off the GPUI render/update path;
- the refresh must be single-flight with existing analyst lifecycle work;
- stale asynchronous refresh completion must not overwrite a later state transition;
- while catalog refresh is running, Start and World-selection actions are unavailable;
- runtime readiness refresh and catalog refresh may be sequenced or coordinated, but Setup must not expose a Start action until both current catalog and runtime readiness are usable;
- refreshing summaries alone must not invalidate a retained failed question when both evidence ids remain the same; M236 fresh-session evidence-scope comparison remains the authority for actual archive-content changes;
- if refresh must replace a missing selected right World, clear the retained failed question and its evidence scope because the pair id changed, reusing the M235 selection-scope rule.

### Implementation boundary

Keep Library enumeration and selection reconciliation in the desktop product layer. Do not add analyst protocol fields, provider/model calls, filesystem authority to Pi, new World truth, or Pack-specific UI.

Prefer a small pure reconciliation helper over embedding right-selection rules inside async callbacks. Reuse `default_right_for` and the M235 failed-question invalidation semantics where possible.

### Validation

Required gates:

- New Comparison sees Worlds added after the analyst window opened;
- Fatal recovery sees updated Library summaries;
- a still-existing selected right World remains selected;
- a deleted selected right World falls back deterministically and invalidates retained pair-scoped failed intent;
- fewer than two available Worlds leaves Setup safe and non-startable with a clear error;
- stale async completion cannot overwrite Active/Fatal/newer refresh state;
- no active session catalog mutation;
- M230–M236 recovery/new-comparison/cancel/retry/dismiss/snapshot-scope regressions remain green;
- existing M219+ protocol/model/runtime/session regressions remain green;
- Linux boundary/fmt/Clippy/workspace/Pack conformance remain green;
- full macOS GPUI/desktop tests and `World Machine.app` build/validate/packaged smoke/archive/upload remain green.

## Non-goals

No two-sided World selector yet, no automatic pair swapping, no persistent chat history, no automatic retry/backoff, no resumable or paused model turn, no reconnect, no streaming, no concurrent turns, no protocol v2, no provider/model picker, no API-key storage, no new analyst tools, and no mutation authority.
