# Next Coding Task — M249 Incremental Analyst History Projection

Status: implementation complete; M248 bounded-projection regression aligned after helper extraction; exact validation restarted on `agent/m249-incremental-analyst-history-projection`.

M214–M248 now provide the installed World Analyst path from immutable saved-World evidence through a restricted long-lived Pi analyst, provider-neutral turns, native runtime/model readiness, in-memory Question → Answer → Evidence history, explicit recovery/new-comparison/cancellation/retry/dismiss flows, retained-question evidence scoping, asynchronous catalog loading/refresh, typed saved-World/runtime drift reconciliation, two-sided selection/Swap, local filtering, Pack/document identity, stable selected/pair identity, and bounded 4096-byte UTF-8-safe UI previews for evidence-tool input/output.

M248 keeps the complete authoritative `AnalystTurn` / tool evidence in `DesktopAnalystSession` while bounding only the panel's presentation copy. The remaining history-projection inefficiency is independent of payload size.

## M249 — append newly completed exchanges instead of rebuilding every prior PanelTurn

After every non-cancelled ask completion, `AnalystPanelView::start_ask()` currently executes:

`this.history = snapshot_history(&session);`

`snapshot_history()` walks **all** `session.exchanges()` and recreates every `PanelTurn`, including reserializing the already-bounded M248 tool previews for all prior successful turns. With N successful turns, the panel repeats prior projection work on every completion, making cumulative projection/copy work quadratic in the number of turns. A recoverable ask error appends no exchange in `DesktopAnalystSession`, yet the current code still rebuilds the unchanged history.

The session already exposes exchanges in append-only success order. Use that property to incrementally synchronize the UI projection without changing the visible history or session semantics.

### Product behavior

- after a normal successful ask, append only exchanges that are not already represented in `history`;
- in the common case of exactly one newly successful turn, create exactly one new `PanelTurn` and leave all earlier `PanelTurn` values untouched;
- if a nonfatal/recoverable ask error adds no session exchange, history synchronization is a no-op rather than recreating prior turns;
- preserve exact Question → Answer → Evidence ordering and all M248 bounded-preview semantics;
- preserve the existing cancellation rule: when `cancel_requested` is true, the completion path must **not** synchronize history, even if the underlying ask raced to completion;
- preserve New comparison and Fatal recovery behavior: both still clear the panel history as today;
- do not add pagination, history virtualization, collapse state or a visible history limit in this slice.

### Implementation boundary

Keep M249 inside `apps/world-machine-desktop/src/analyst_panel.rs` presentation/orchestration.

Prefer extracting the per-exchange projection used by `snapshot_history()`, for example:

- `panel_turn_from_exchange(exchange: &DesktopAnalystExchange) -> PanelTurn`

`DesktopAnalystExchange` is the session-layer value already exposed by `world_machine_desktop::analyst_session`; do not name or import raw `world_analyst_client` types in the panel.

Then keep `snapshot_history()` as the authoritative full-rebuild fallback and add an incremental synchronizer, for example:

- `sync_history_projection(history: &mut Vec<PanelTurn>, session: &DesktopAnalystSession)`

The normal synchronization path should be O(number of newly missing exchanges), not O(total exchanges).

Use a small explicit projection plan so the fast path does not scan or rebuild all prior turns. The panel has no supported per-turn edit/reorder/delete operation while a session is Active, so `history.len()` is the primary cursor. Before appending, make a constant-time consistency check against the last retained exchange where possible (at minimum exchange count and the last projected question/prompt). If the detectable prefix contract is violated, fall back to `snapshot_history(session)` rather than attempting to repair a possibly stale projection incrementally.

Required behavior for the plan:

- `history.len() == 0` with existing exchanges: append from exchange 0;
- `history.len() < exchanges.len()` and the retained tail still matches the corresponding exchange: append from `history.len()`;
- `history.len() == exchanges.len()` and the retained tail matches: no-op;
- `history.len() > exchanges.len()`: full rebuild;
- a detectable retained-tail mismatch: full rebuild;
- if multiple exchanges are ever missing, append all of them in order rather than assuming exactly one;
- do not compare by title/tool text or other lossy presentation data; the prompt/question tail check is only a defensive consistency sentinel for a projection that product code otherwise keeps append-only.

### State / correctness invariants

- `DesktopAnalystSession::exchanges()` remains authoritative; do not mutate, remove, reorder or persist exchanges from the panel;
- `SessionCore::ask()` behavior remains unchanged: only successful asks append exchanges; recoverable/fatal errors do not manufacture exchanges;
- cancellation completion continues to skip history synchronization exactly as today;
- failed-question retention/scope, retry policy, composer clearing and last-error handling remain unchanged;
- Fatal transition, New comparison, recovery and session close/cancellation semantics remain unchanged;
- M248's 4096-byte streaming preview writer and truncated-label behavior remain unchanged;
- no protocol/provider/model/Pi/process/runtime/readiness changes;
- no Library/catalog/filter/pair/Swap/Start/identity changes;
- no persistence, Pack or World authority changes;
- keep `analyst_panel.rs` above raw client/process/Pi layers; the existing source boundary test must continue to pass.

### Validation

Required regressions:

- empty history plus one exchange chooses append-from-0;
- an aligned history prefix with one additional exchange chooses append from the existing history length;
- an aligned history whose length already equals exchange count produces a no-op plan;
- history longer than the session exchange list chooses full rebuild;
- a last-question/prompt mismatch chooses full rebuild;
- a synchronizer appends multiple missing projected exchanges in exact order;
- previously projected `PanelTurn` values are preserved on the normal append path rather than recreated;
- a recoverable/no-new-exchange completion leaves the existing history unchanged;
- the existing cancellation path still does not call/synchronize history when `cancel_requested` is true;
- full fallback rebuild remains behaviorally identical to the existing `snapshot_history()` output;
- M248 short/exact-limit/large/UTF-8/input-output-independent preview regressions remain green;
- existing session success/nonfatal/fatal exchange-count regressions remain green;
- existing retry/cancel/evidence-scope/catalog/pair/identity regressions remain green;
- Linux boundary/Pi/fmt/Clippy/workspace/Pack gates remain green;
- full macOS Library/Packs/GPUI/desktop tests plus `World Machine.app` build/validate/packaged smoke/archive/upload remain green.

## Non-goals

No history virtualization, no maximum turn count, no persisted analyst history, no per-turn editing/deletion/reordering, no transcript export, no lazy/full tool-payload expansion, no M248 preview-size change, no protocol/session exchange mutation, no provider/model changes, no new Pi tools, no selector/catalog changes, and no Pack/World behavior changes.
