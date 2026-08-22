# Next Coding Task — M235 Invalidate Failed Question When Comparison Selection Changes

M214–M234 now provide the installed World Analyst path from immutable saved-World evidence through a restricted long-lived Pi analyst, stable provider-neutral turns, native runtime/model readiness, an in-memory Question → Answer → Evidence transcript, explicit fatal recovery, clean New Comparison lifecycle, explicit cancellation, explicit retry of retained failed/cancelled questions, and explicit dismissal of retained failed-question intent.

## M235 — prevent retained failed-question intent from crossing snapshot-pair scope

M230 deliberately preserves `failed_question` across Fatal recovery so the user can recover the runtime and retry the same question against the same selected immutable World pair. M233 makes that retained question explicitly retryable. M231 clears it when New Comparison cleanly resets scope.

There is one remaining scope hole: after M230 recovery, the panel is back in Setup with no live session, and existing `choose_right` allows the user to select a different right-hand saved World. Today that selection change clears `last_error` but leaves `failed_question` intact. A later Start can therefore make a question retained from pair A↔B retryable against pair A↔C.

That is incorrect product state. A failed question is scoped to the immutable snapshot pair on which it was submitted.

### Product behavior

When Setup changes the pending comparison selection to a different World:

- clear any retained `failed_question` immediately because its evidence scope no longer matches;
- leave the composer draft byte-for-byte unchanged;
- leave runtime/readiness state unchanged;
- leave the selected replacement World in place;
- do not start, close, recover, cancel, or otherwise touch an analyst session/process;
- do not fabricate or retain transcript history from the previous pair.

A no-op selection of the already-selected World must not clear anything.

The important recovery case is:

1. an Ask fails fatally for pair A↔B and retains its submitted question;
2. M230 Recover returns the panel to Setup while preserving that retained question;
3. if the user keeps A↔B and starts a fresh session, M233 Retry remains available once Active;
4. if the user changes B to C before Start, the retained A↔B question is cleared immediately and can never become retryable against A↔C.

M231 New Comparison already clears `failed_question` before Setup and remains unchanged. M234 Dismiss remains an independent explicit local action.

### Implementation boundary

Keep this entirely in native panel selection state. Prefer a small pure transition/helper around pending comparison selection so the scope-clearing rule is directly testable rather than coupling it to rendering.

Do not add a protocol field, session method, process command, persistence record, provider-specific behavior, or model call. GPUI must continue to consume only the existing product-level analyst APIs.

### Validation

Required gates:

- changing the right-hand World in Setup clears retained `failed_question`;
- selecting the already-selected right-hand World is a no-op and preserves retained intent;
- composer draft is unaffected by selection changes;
- runtime/readiness is unaffected by selection changes;
- no session/process/cancellation action is introduced;
- keeping the same pair through Fatal → Recover → Start still preserves M233 Retry behavior;
- M231 New Comparison, M232 cancellation, M233 Retry, and M234 Dismiss regressions remain green;
- existing M219–M234 protocol/model/runtime/transcript/session regressions remain green;
- Linux boundary/fmt/Clippy/workspace/Pack conformance remain green;
- full macOS GPUI/desktop tests and `World Machine.app` build/validate/packaged smoke/archive/upload remain green.

## Non-goals

No two-sided World selector yet, no automatic pair swapping, no cross-comparison failed-question carryover, no persistent chat history, no automatic retry/backoff, no resumable model turn, no reconnect, no token streaming, no concurrent turns, no protocol v2, no provider/model picker, no API-key storage, no new analyst tools, and no mutation authority.
