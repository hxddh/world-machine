# Next Coding Task — M234 Explicit Dismiss Failed Analyst Question

M214–M233 now provide the installed World Analyst path from immutable saved-World evidence through a restricted long-lived Pi analyst, stable provider-neutral turns, native runtime/model readiness, an in-memory Question → Answer → Evidence transcript, explicit fatal recovery, clean New Comparison lifecycle, explicit cancellation of a running Ask, and explicit retry of a retained failed/cancelled question without disturbing a newer composer draft.

## M234 — explicitly dismiss a retained failed question without changing analyst state

M229 introduced `failed_question` as durable in-panel intent so an unsuccessful submitted prompt is not lost. M230 preserves it across fatal recovery, M231 clears it only when a clean New Comparison changes snapshot scope, M232 retains a successfully cancelled Ask, and M233 makes the retained question actionable through explicit Retry.

The remaining product gap is intentional abandonment. Once a failed question is retained, a user who no longer wants to retry it has no direct way to remove the callout without starting a new comparison. The retained question is user intent, not session/runtime state, so it should have its own explicit discard action.

Add the smallest native `Dismiss failed question` action. Dismissal must clear only the retained failed-question intent and must not restart, close, recover, cancel, or otherwise mutate the analyst session.

### Product behavior

When `failed_question` exists:

- continue displaying the retained failed-question callout;
- expose `Dismiss failed question` whenever no analyst lifecycle operation is currently busy;
- dismissal may remain available in idle Setup and Fatal states because it is a local intent change and does not require a live analyst session;
- do not expose or enable dismissal during startup, an in-flight composer Ask, M233 Retry, M232 cancellation settlement, or M231 clean New Comparison shutdown;
- M233 `Retry failed question` keeps its stricter requirement of an idle healthy Active session with a live session.

When the user chooses `Dismiss failed question`:

- set only `failed_question` to `None`;
- leave the composer draft byte-for-byte unchanged;
- leave transcript/history unchanged;
- leave `PanelPhase`, `last_error`, runtime/readiness state, selected Worlds, live session, and cancellation authority unchanged;
- do not issue any lower session/process/RPC/provider call;
- do not automatically start, recover, close, retry, or create another Pi process.

After dismissal:

- the failed-question callout and Retry/Dismiss actions disappear immediately;
- a later failed or successfully cancelled composer Ask can create a new retained failed question normally;
- successful normal Ask behavior remains unchanged;
- M230 recovery, M231 New Comparison, M232 cancellation, and M233 Retry semantics remain unchanged.

### Implementation boundary

Keep dismissal entirely in `analyst_panel.rs` as a native product-state action over `failed_question`. Do not add a `DesktopAnalystSession` method, protocol field, process command, persistence record, or provider-specific behavior for dismissal.

Prefer a small pure eligibility helper so visibility rules are directly testable. The action itself should have no side effects beyond clearing the retained question and notifying GPUI.

### Validation

Required gates:

- Dismiss appears only while a retained failed question exists and the panel is not busy;
- Dismiss is available in idle Active, Setup, and Fatal states without requiring a live session;
- Dismiss is unavailable during startup, Ask, Retry, cancellation settlement, and New Comparison close;
- Dismiss clears only `failed_question`;
- composer draft, transcript/history, phase, errors, runtime/readiness, World selection, session, and cancellation state are unchanged;
- after dismissal, a later failed/cancelled composer Ask can retain a new failed question normally;
- M233 Retry still uses the exact retained question and preserves the composer across success/recoverable/fatal/cancel paths;
- M230 recovery, M231 New Comparison, and M232 cancellation regressions remain green;
- existing M219–M233 protocol/model/runtime/transcript/session regressions remain green;
- Linux boundary/fmt/Clippy/workspace/Pack conformance remain green;
- full macOS GPUI/desktop tests and `World Machine.app` build/validate/packaged smoke/archive/upload remain green.

## Non-goals

No bulk transcript clearing, no automatic expiration of failed questions, no confirmation dialog, no persistent chat history, no automatic retry/backoff, no resumable or paused model turn, no reconnecting an ended Pi process, no cross-comparison failed-question carryover, no token streaming, no concurrent turns, no protocol v2, no provider/model picker, no API-key storage, no new analyst tools, and no mutation authority.
