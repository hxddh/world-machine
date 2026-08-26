# Next Coding Task — M250 Virtualize Analyst History Rendering

M214–M249 now provide the installed World Analyst path from immutable saved-World evidence through a restricted long-lived Pi analyst, provider-neutral turns, native runtime/model readiness, in-memory Question → Answer → Evidence history, explicit recovery/new-comparison/cancellation/retry/dismiss flows, retained-question evidence scoping, asynchronous catalog loading/refresh, typed saved-World/runtime drift reconciliation, two-sided selection/Swap, local filtering, Pack/document identity, stable selected/pair identity, bounded 4096-byte UTF-8-safe evidence previews, and incremental session-exchange → `PanelTurn` projection.

M249 removes repeated re-projection of all prior turns after each ask. The remaining long-history scaling cost is now in rendering rather than data projection.

## M250 — virtualize the variable-height Analyst history without truncating it

`AnalystPanelView::render_active()` currently builds the complete history tree on every render:

```rust
for (index, turn) in self.history.iter().enumerate() {
    history = history.child(render_turn(index, turn));
}
```

Every `PanelTurn` can have a different height because answer length, evidence-call count, bounded input/output preview length and runtime-error count vary. As history grows, every unrelated panel notification still recreates every turn card even when most are far outside the scroll viewport.

The exact GPUI revision already pinned by `world-machine-desktop` (`zed` rev `4e8057d74db3570b3bd419ff296eb84c35b3a5a3`) provides `gpui::list` + `ListState` specifically for efficiently rendering large numbers of differently sized elements. Its state is stored on the owning view, supports `splice` / `reset`, and supports `FollowMode::Tail` so appended chat-like content follows the end only while the user is already following it.

Do not use `uniform_list`: Analyst turn heights are not uniform.

### Product behavior

- retain **all** `PanelTurn` values in `history`; M250 is rendering virtualization, not a visible-history limit;
- keep exact Question → Answer → Evidence ordering and all existing turn-card content/labels;
- render only the variable-height history items needed for the viewport plus GPUI overdraw;
- new successful turns should remain visible automatically while the history is following the tail;
- if the user scrolls upward, later appends must not yank the viewport back to the bottom; GPUI Tail follow should resume naturally when the user returns to the end;
- keep the current empty-history guidance, composer, pair identity, status, Fatal recovery and New comparison UI unchanged;
- do not add pagination, “load older”, collapse/expand, maximum turn count or transcript persistence.

### Implementation boundary

Keep M250 in `apps/world-machine-desktop/src/analyst_panel.rs`. Do not modify GPUI itself or change the pinned dependency revision.

Add a persistent history-list state to `AnalystPanelView`, for example:

```rust
history_list: ListState
```

Initialize it with the current history count `0`, `ListAlignment::Top`, a modest pixel overdraw, then enable `FollowMode::Tail`. The exact overdraw value is presentation tuning only; avoid `measure_all()` because the purpose is to avoid measuring/rendering the complete long history up front.

Render history with the pinned variable-height API and the standard GPUI view-state pattern:

```rust
list(
    self.history_list.clone(),
    cx.processor(|this, index, _window, _cx| this.render_history_entry(index)),
)
```

The item processor must read the indexed `PanelTurn` from view state. **Do not clone the entire `history` vector into the `'static` list renderer**, because that would reintroduce O(total history) copying on every panel render.

Keep `render_turn(index, turn)` as the single card renderer. A small row wrapper is fine for spacing, but use spacing whose height does not depend on whether the row is currently last; appending a new item must not silently change the previously measured old-tail item's height.

### Keep `ListState` count synchronized with M249 projection

The list state's item count must track `history.len()` on every history lifecycle path.

- panel construction: list count is `0`;
- successful session startup/full `snapshot_history`: `reset(history.len())` and start/follow the tail;
- M249 `HistoryProjectionPlan::Noop`: no list mutation when counts already match;
- `AppendFrom(start)`: when list count matches the old history prefix, `splice(start..start, newly_added_count)` rather than resetting all measured rows;
- `Rebuild`: reset to the rebuilt history length;
- if list count and the expected M249 prefix are detectably inconsistent, defensively `reset(history.len())` instead of applying an invalid splice;
- New comparison and Fatal recovery: after the existing history clear, reset list count to `0`;
- cancellation completion still skips history synchronization and therefore must not manufacture a list mutation.

It is acceptable to make `sync_history_projection()` return its existing `HistoryProjectionPlan`, or to introduce a small pure helper that derives the required `ListState` mutation from the projection plan and before/after lengths. Keep data projection and list-cache synchronization explicit rather than coupling `ListState` into session/core code.

### State / correctness invariants

- `DesktopAnalystSession::exchanges()` and `history` remain the complete authoritative session/UI projection respectively;
- M249 fast-path projection semantics and fallback rebuild semantics remain unchanged;
- M248 4096-byte input/output previews and truncation labels remain unchanged;
- user scroll position is owned by GPUI `ListState`; appending while the user has scrolled up must preserve that position;
- no Question/Answer/tool/runtime-error content changes;
- no retry/cancel/failed-question/evidence-scope/Fatal/New comparison behavior changes;
- no protocol/provider/model/Pi/process/runtime/readiness changes;
- no Library/catalog/filter/pair/Swap/Start/identity changes;
- no persistence, Pack or World authority changes;
- keep `analyst_panel.rs` above raw client/process/Pi layers.

### Validation

Required regressions:

- source/render regression proves `render_active()` no longer eagerly loops through every `self.history` turn and instead uses GPUI `list` with the persistent `history_list` state;
- the list item renderer indexes the live view history rather than cloning the complete history into the renderer;
- empty history still renders the existing guidance instead of an empty list;
- list-state mutation planning covers no-op, single append, multiple append, rebuild, and stale-count fallback;
- normal append uses `splice` and does not reset already measured prefix rows;
- startup/full rebuild synchronizes list count with `history.len()`;
- New comparison and Fatal recovery clear both history and list item count;
- cancellation path still performs no history/list append;
- Tail follow is enabled; no manual unconditional `scroll_to_end()` is performed on every append that would override a user who scrolled upward;
- M249 incremental projection regressions remain green;
- M248 bounded-preview regressions remain green;
- existing analyst session/retry/cancel/evidence-scope/catalog/pair/identity regressions remain green;
- Linux boundary/Pi/fmt/Clippy/workspace/Pack gates remain green;
- full macOS Library/Packs/GPUI/desktop tests plus `World Machine.app` build/validate/packaged smoke/archive/upload remain green.

## Non-goals

No history truncation or pagination, no maximum turn count, no persisted analyst history, no per-turn editing/deletion/reordering, no transcript export, no full tool-payload expansion, no change to M248 preview size, no protocol/session mutation, no GPUI fork or revision bump, no provider/model changes, no new Pi tools, no selector/catalog changes, and no Pack/World behavior changes.
