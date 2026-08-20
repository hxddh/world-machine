# Next Coding Task — M230 Analyst Fatal-Session Recovery

M214–M229 now provide a complete installed-app path from immutable saved-World evidence through a restricted long-lived Pi analyst, stable provider-neutral turns, native runtime setup and probing, model-state readiness, and an in-memory product transcript that keeps each accepted user question paired with its completed answer and evidence.

## M230 — recover explicitly from a fatal analyst session

A fatal analyst failure is currently a dead end in the native panel. The process and immutable snapshots are correctly torn down, but the panel remains in `Fatal`, Ask is disabled, and the only practical escape is closing the analyst window. M229 now preserves the failed question in the composer, so the next step is to give that preserved user intent a safe recovery path.

Add the smallest explicit recovery flow above the existing fatal-session semantics.

### Product behavior

When a session becomes fatal:

- keep the fatal message visible;
- keep the completed transcript from that ended session visible while the panel remains in Fatal;
- keep the failed or edited composer text intact;
- expose a clear recovery action that returns the panel to runtime/setup flow without closing the window.

Recovery is explicit. Do not automatically retry a model request or silently restart Pi.

### New-session boundary

The recovery action must discard the ended `DesktopAnalystSession`, clear its cancellation handle, and re-run installed runtime readiness before another Start is allowed.

A later Start creates a completely new analyst session using the existing M222/M227/M228 path:

1. capture a fresh immutable archive pair for the currently selected saved Worlds;
2. spawn a fresh restricted turn host/Pi process;
3. run startup probe and model-state gate;
4. only then expose the new session as Active.

Do not reuse the fatal process, old archive snapshots, or old startup readiness result.

### Transcript semantics

Do not mix completed exchanges from different immutable snapshot pairs into one conversation history.

The ended session's transcript may remain visible while the user is looking at the Fatal state. Once the user explicitly enters recovery/setup for a new session, clear that ended-session history before the next session begins. Preserve the composer text so the failed question can be retried after successful startup.

### Layering and authority

- GPUI continues to own only panel/product state and `DesktopAnalystSession`, never Node/Pi process internals.
- Fatal process shutdown and snapshot cleanup remain owned below GPUI.
- Runtime recheck reuses M225/M226 readiness and M228 model semantics; no provider/model picker or credential storage.
- No transcript persistence, session resume, automatic retries, token streaming, concurrent asks, new tools, or mutation authority.

## Validation

Required gates:

- fatal ask still tears down the process/snapshots and appends no completed exchange;
- failed prompt remains available in the composer;
- Fatal UI exposes an explicit recovery action while keeping the ended transcript visible;
- recovery clears the ended session/cancellation/history but preserves the composer and selected World pair;
- recovery invalidates old runtime readiness and performs a fresh readiness check;
- a subsequent Start creates/probes a fresh analyst process and fresh snapshots rather than reusing the fatal session;
- completed exchanges from the old and new snapshot pairs are never mixed;
- existing M219–M229 protocol, model-state, transcript, packaged-runtime, and desktop-session regressions remain green;
- Linux boundary/fmt/Clippy/workspace tests remain green;
- full macOS GPUI/desktop tests and `World Machine.app` build/validate/packaged smoke/archive/upload remain green.

## Non-goals

No automatic retry, no reconnecting the fatal Pi process, no cross-session transcript merge, no persistent chat history, no session resume after app restart, no streaming UI, no concurrent turns, no protocol v2, no model/provider picker, no API-key storage, no new tools, and no mutation authority.
