# Next Coding Task — M232 Analyst Cancel Running Analysis

M214–M231 now provide the installed World Analyst path from immutable saved-World evidence through a restricted long-lived Pi analyst, stable provider-neutral turns, native runtime/model readiness, an in-memory Question → Answer → Evidence transcript, explicit fatal recovery, and an explicit clean exit from a healthy session into a fresh comparison.

## M232 — cancel a running analyst turn explicitly

A submitted analyst question is currently single-flight and safe, but the native panel gives the user no direct way to stop a long-running turn. The only existing cancellation handle is used for window teardown; an in-progress Ask otherwise runs until completion or timeout.

Add the smallest explicit `Cancel analysis` flow using the existing `DesktopAnalystCancellation` authority. Cancellation intentionally ends the current analyst session; it is not an attempt to keep using the same Pi process after an interrupted model turn.

### Product behavior

While a question is actively being analyzed:

- keep the completed transcript visible;
- keep any newer composer draft intact;
- expose a clear `Cancel analysis` action only when an Ask is actually in flight and a cancellation handle is available;
- do not expose the action during Setup, startup probing, normal idle Active state, fatal recovery, or the M231 clean `New comparison` shutdown.

When the user chooses `Cancel analysis`:

- signal the existing session cancellation handle once;
- do not fabricate a completed exchange for the interrupted question;
- do not automatically retry the question or start another Pi process;
- let the existing in-flight Ask settle through the normal session-fatal path;
- surface the ended/cancelled session in the existing Fatal UI so M230 explicit recovery is the only route to a fresh session.

### User intent and transcript

Cancellation must preserve product intent:

- completed exchanges already in the transcript remain visible while Fatal;
- the interrupted submitted question remains available through the existing failed-question/composer semantics once the Ask settles;
- a newer draft typed while the Ask was running must not be overwritten;
- no interrupted question is appended as a completed Question → Answer exchange.

### Lifecycle boundary

Reuse the existing cancellation handle and fatal cleanup semantics:

1. GPUI requests cancellation through `DesktopAnalystCancellation` only;
2. the restricted analyst process is terminated by the existing session layer;
3. the in-flight Ask resolves as a fatal client/session error;
4. session cleanup removes the immutable snapshot pair;
5. panel enters Fatal;
6. M230 recovery is required before another Start creates fresh snapshots/process/probe/model readiness.

Do not add a second process-control path in GPUI and do not attempt to reconnect or reuse the cancelled Pi process.

### Validation

Required gates:

- cancel control appears only for a real in-flight Ask with a live cancellation handle;
- clicking cancel is idempotent and cannot send repeated termination signals;
- completed transcript remains intact and the interrupted question is not appended as a successful exchange;
- edited composer drafts and failed-question semantics remain safe;
- the Ask settles into Fatal rather than returning the panel to healthy Active;
- fatal process/snapshot cleanup remains owned by the existing desktop session layer;
- M230 recovery and M231 clean New Comparison remain independent and green;
- existing M219–M231 protocol/model/runtime/transcript regressions remain green;
- Linux boundary/fmt/Clippy/workspace tests remain green;
- full macOS GPUI/desktop tests and `World Machine.app` build/validate/packaged smoke/archive/upload remain green.

## Non-goals

No resumable model turn, no pause/resume, no reconnecting a cancelled Pi process, no automatic retry, no cross-session transcript merge, no persistent chat history, no token streaming, no concurrent turns, no protocol v2, no provider/model picker, no API-key storage, no new analyst tools, and no mutation authority.
