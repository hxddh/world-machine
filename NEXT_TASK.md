# Next Coding Task — M251 Avoid Duplicate Full Analyst Turn on Ask Completion

Status: implementation complete; exact validation in progress on `agent/m251-retain-analyst-turn-without-clone`.

M214–M250 now provide the installed World Analyst path from immutable saved-World evidence through a restricted long-lived Pi analyst, provider-neutral turns, native runtime/model readiness, complete in-memory Question → Answer → Evidence history, explicit recovery/new-comparison/cancellation/retry/dismiss flows, retained-question evidence scoping, asynchronous catalog loading/refresh, typed drift reconciliation, two-sided selection/Swap, local filtering, stable Pack/document/pair identity, bounded 4096-byte UTF-8-safe UI evidence previews, incremental session-exchange → `PanelTurn` projection, and variable-height virtualized GPUI history rendering.

M248 bounds only the panel's UI copy of tool input/output. The authoritative `DesktopAnalystSession` intentionally retains complete raw `AnalystTurn` evidence. There is still one avoidable full-payload duplication on every successful ask before that retained turn reaches the panel.

## M251 — move successful turns into retained exchanges without cloning them for the panel path

`SessionCore::ask()` currently receives one owned `AnalystTurn` from the process and then clones the complete value into session history before returning the original:

```rust
Ok(turn) => {
    self.exchanges.push(DesktopAnalystExchange {
        prompt: prompt.to_owned(),
        turn: turn.clone(),
    });
    self.state = DesktopAnalystState::Answer {
        turn_index: self.exchanges.len() - 1,
    };
    Ok(turn)
}
```

`AnalystTurn` includes arbitrary `serde_json::Value` tool input/output payloads. The protocol has no small payload bound, so `turn.clone()` can duplicate a large evidence tree.

The native Analyst panel does not consume the successful owned return value. `AnalystPanelView::start_ask()` currently does:

```rust
let result = session.ask(&prompt).map_err(|error| error.to_string());
```

and completion only checks `result.is_ok()` / `result.err()` before projecting authoritative `session.exchanges()`. On the normal panel path, the returned successful `AnalystTurn` is therefore an unnecessary second full raw copy until the background result is consumed.

### Product behavior

- keep exactly one complete authoritative raw `AnalystTurn` per successful exchange in `DesktopAnalystSession`;
- the panel's successful ask path must not request or retain a second owned full turn that it never reads;
- preserve the public owned-return `DesktopAnalystSession::ask()` behavior for callers/tests that actually need the returned `AnalystTurn`;
- preserve exact exchange prompt/turn ordering, `DesktopAnalystState::Answer { turn_index }`, recoverable/fatal behavior and process reuse/shutdown semantics;
- preserve M248 bounded UI projections, M249 incremental projection and M250 virtualized rendering unchanged.

### Implementation boundary

Keep protocol/client behavior unchanged. Do not remove `Clone` from `world_analyst_client::AnalystTurn` and do not change its serialized schema.

Refactor `SessionCore` so its primary successful storage path **moves** the owned process result directly into `DesktopAnalystExchange` instead of cloning it. A narrow private primitive is preferred, for example:

```rust
fn ask_and_retain(
    &mut self,
    prompt: &str,
) -> Result<usize, DesktopAnalystSessionError>
```

On success:

1. compute the new exchange index;
2. push `DesktopAnalystExchange { prompt: prompt.to_owned(), turn }` by move;
3. set `DesktopAnalystState::Answer { turn_index }`;
4. return the cheap index.

Recoverable/fatal branches should remain the same as today and must not manufacture an exchange.

Expose a product-level no-copy method on `DesktopAnalystSession` for orchestration code that only needs successful retention, for example:

```rust
pub fn ask_retained(
    &mut self,
    prompt: &str,
) -> Result<usize, DesktopAnalystSessionError>
```

The exact name may differ, but it must communicate that the successful turn is retained in `exchanges()` rather than returned as another owned payload.

Preserve the existing compatibility API:

```rust
pub fn ask(&mut self, prompt: &str) -> Result<AnalystTurn, DesktopAnalystSessionError>
```

It may call the retained primitive and clone the stored turn **only because that caller explicitly requested an owned `AnalystTurn`**. The avoidable clone must disappear from the retained/panel path, not necessarily from the compatibility API.

Update `AnalystPanelView::start_ask()` to use the retained/no-copy product method and carry only cheap success/error information through the background task. The panel must remain above raw client/process/Pi layers.

### State / correctness invariants

- `DesktopAnalystSession::exchanges()` remains authoritative and stores the complete raw evidence exactly once per successful retained ask;
- successful exchange ordering and prompt association are unchanged;
- `DesktopAnalystState::Answer { turn_index }` still points at the exchange just retained;
- recoverable errors retain the live process and append no exchange;
- fatal errors shut down the process, clean snapshots and append no exchange exactly as today;
- public `DesktopAnalystSession::ask()` continues returning an owned turn with the same content;
- cancellation handle/state behavior is unchanged;
- no Analyst turn protocol/schema/provider/model/timeout changes;
- no evidence truncation or raw-payload dropping in session truth;
- M248 4096-byte panel previews remain the only UI payload bound;
- M249 projection plans/list synchronization and M250 virtual list behavior remain unchanged;
- no Library/catalog/filter/pair/Swap/identity/persistence/Pack/World changes.

### Validation

Required regressions:

- a retained successful ask appends exactly one exchange and reports the matching exchange index;
- two retained successful asks preserve exact order and advance `Answer { turn_index }` from 0 to 1;
- retained recoverable/fatal failures append no fake exchanges and preserve existing shutdown/reuse behavior;
- the compatibility `DesktopAnalystSession::ask()` still returns an owned `AnalystTurn` with exact answer/tool/runtime-error content;
- source regression proves the retained storage primitive moves `turn` into `DesktopAnalystExchange` rather than calling `turn.clone()` there;
- any clone used to implement the owned-return compatibility API occurs only after successful retained storage and is not used by the panel path;
- `AnalystPanelView::start_ask()` uses the retained/no-copy session API rather than `session.ask()` and still bases completion on success/error only;
- cancellation completion behavior is unchanged;
- M248 bounded-preview, M249 incremental-projection and M250 virtualized-history regressions remain green;
- existing analyst session success/recoverable/fatal/archive/cancellation tests remain green;
- Linux boundary/Pi/fmt/Clippy/workspace/Pack gates remain green;
- full macOS Library/Packs/GPUI/desktop tests plus `World Machine.app` build/validate/packaged smoke/archive/upload remain green.

## Non-goals

No history truncation or persistence, no payload-size change, no protocol/client response redesign, no removal of `AnalystTurn: Clone`, no provider/model changes, no new Pi tools, no concurrent asks, no reconnect/resume, no selector/catalog changes, no GPUI changes, and no Pack/World behavior changes.
