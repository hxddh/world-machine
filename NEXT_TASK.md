# Next Coding Task — M231 Analyst New Comparison

M214–M230 now provide a complete installed-app path from immutable saved-World evidence through a restricted long-lived Pi analyst, stable provider-neutral turns, native runtime setup and probing, model-state readiness, an in-memory Question → Answer → Evidence transcript, and an explicit recovery path after fatal analyst failure.

## M231 — start a new comparison without closing the analyst window

A healthy Active analyst session is still sticky to its original immutable snapshot pair. Once analysis starts, the right-hand World selector is intentionally locked and the only practical way to compare a different pair is to close the analyst window and reopen it.

Add the smallest explicit healthy-session exit / new-comparison flow above the existing session semantics.

### Product behavior

While a session is Active and not busy:

- expose a clear `New comparison` action;
- keep the current transcript visible until the user explicitly chooses that action;
- do not silently stop or restart analysis;
- do not automatically choose a different World.

When the user chooses `New comparison`:

- close the current `DesktopAnalystSession` through the existing clean shutdown path;
- clear the panel cancellation handle and ended-session transcript;
- return to Setup in the same analyst window;
- preserve the current left/right World selection as the initial Setup selection so the user can keep it or choose another right-hand World;
- preserve any composer draft that has not yet been submitted;
- invalidate the old runtime readiness and perform a fresh installed-runtime recheck before another Start is enabled.

### New-session boundary

A later Start must create a completely new analyst session through the existing M222/M227/M228 path:

1. capture fresh immutable archives for the selected saved Worlds;
2. spawn a fresh restricted turn host/Pi process;
3. run startup probe and model-state readiness;
4. only then expose the new session as Active.

Never reuse the old Pi process, old immutable archive pair, old startup readiness result, or old transcript.

### Transcript semantics

Completed exchanges belong to exactly one immutable snapshot pair.

- show the current transcript while the healthy session remains Active;
- clear it only after the user explicitly starts the new-comparison flow;
- never append exchanges from the later session to the previous session's history;
- no persistent cross-session transcript or session resume.

### Layering and authority

- GPUI owns only panel/product state plus `DesktopAnalystSession` lifecycle calls;
- process shutdown and snapshot cleanup remain below GPUI;
- runtime recheck reuses existing readiness semantics;
- no provider/model picker, credential storage, Pi/RPC protocol change, new tools, or mutation authority.

## Validation

Required gates:

- Active UI exposes `New comparison` only when a session is healthy and not busy;
- choosing it closes the current session instead of dropping a live Pi process implicitly;
- clean shutdown removes the old immutable snapshot pair;
- panel clears the old session/cancellation/history and returns to Setup;
- selected World pair and unsent composer draft are preserved;
- stale runtime readiness is invalidated and rechecked;
- subsequent Start creates/probes a fresh process and fresh archive pair;
- old and new snapshot-pair transcripts are never mixed;
- fatal recovery from M230 remains independent and green;
- existing M219–M230 protocol/model/transcript/runtime regressions remain green;
- Linux boundary/fmt/Clippy/workspace tests remain green;
- full macOS GPUI/desktop tests and `World Machine.app` build/validate/packaged smoke/archive/upload remain green.

## Non-goals

No automatic restart, no cross-session transcript merge, no persistent chat history, no session resume after app restart, no streaming UI, no concurrent turns, no protocol v2, no model/provider picker, no API-key storage, no new tools, and no mutation authority.
