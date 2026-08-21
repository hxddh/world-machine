# Next Coding Task — M233 Explicit Retry Failed Analyst Question

M214–M232 now provide the installed World Analyst path from immutable saved-World evidence through a restricted long-lived Pi analyst, stable provider-neutral turns, native runtime/model readiness, an in-memory Question → Answer → Evidence transcript, explicit fatal recovery, clean New Comparison lifecycle, and explicit cancellation of a running Ask.

## M233 — retry a retained failed analyst question without disturbing the current draft

M229 introduced `failed_question` so a failed submitted prompt is not lost when the user edits a newer composer draft while an Ask is running. M230 deliberately preserves that failed question across fatal recovery for the same selected World pair. M231 clears it when the user explicitly changes comparison scope, and M232 also uses the same retained-question semantics for a successfully cancelled Ask.

The missing product step is that the native panel can display the retained failed question but cannot explicitly submit it again without the user manually replacing or copying the current composer text. That makes the safety state visible but not actionable.

Add the smallest explicit `Retry failed question` flow above the existing session API. The retry must submit the retained failed question itself and must not overwrite, clear, or otherwise borrow the current composer draft.

### Product behavior

When `failed_question` exists:

- continue displaying the failed-question callout exactly as retained product state;
- expose `Retry failed question` only while the panel has a healthy, idle `PanelPhase::Active` analyst session;
- do not expose or enable retry during Setup, startup probing, an in-flight Ask, M232 cancellation settlement, M231 clean New Comparison shutdown, or Fatal state;
- after M230 recovery, the failed question remains visible through Setup and can become retryable only after a fresh Start successfully reaches Active for the same selected World pair.

When the user chooses `Retry failed question`:

- submit exactly the retained failed-question text through the existing `DesktopAnalystSession::ask` path;
- leave the current composer draft byte-for-byte unchanged, even if it contains a different newer question;
- use the same single-flight, cancellation, transcript, recoverable/fatal error, and immutable snapshot semantics as a normal Ask;
- do not automatically start a session, retry after another failure, or create a second Pi process.

### Completion semantics

On successful retry:

- append the resulting exchange once using the exact retried failed-question prompt;
- clear `failed_question` because that retained failure has now been successfully answered;
- keep the current composer draft unchanged rather than applying the normal composer-clear policy to a prompt that did not originate from the composer.

On recoverable retry failure:

- append no fake successful exchange;
- keep the same failed question retained for another explicit retry;
- keep the current composer draft unchanged;
- keep the session Active if the existing lower session state remains recoverable.

On fatal retry failure:

- append no fake successful exchange;
- retain the failed question and current composer draft;
- enter the existing Fatal UI and require M230 recovery before another session can be started.

If the user chooses M231 `New comparison`, the retained question remains scoped to the old immutable snapshot pair and must still be cleared only after the clean comparison reset succeeds.

### Implementation boundary

Prefer one shared internal Ask completion path rather than duplicating session state handling between composer Ask and failed-question retry. The shared path must make prompt origin explicit enough that composer clearing can never be accidentally applied to a retry prompt.

GPUI must continue to consume only `DesktopAnalystSession` / `DesktopAnalystCancellation` product APIs. Do not move Pi/RPC/process/provider details into the panel and do not add a protocol field for retry; retry is a native product action over the existing M220/M221 `ask` contract.

### Validation

Required gates:

- retry control appears only for idle healthy Active state with a retained failed question;
- retry submits the retained failed question rather than the current composer text;
- a different newer composer draft survives successful, recoverable-failed, and fatal-failed retry unchanged;
- successful retry appends exactly one Question → Answer exchange and clears `failed_question`;
- recoverable retry failure appends no exchange and retains `failed_question`;
- fatal retry failure retains question/draft and enters Fatal;
- M230 recovery preserves the retained question until a later successful retry;
- M231 New Comparison still clears old snapshot-pair `failed_question` only after clean close;
- M232 cancellation remains single-flight and independent;
- existing M219–M232 protocol/model/runtime/transcript/session regressions remain green;
- Linux boundary/fmt/Clippy/workspace/Pack conformance remain green;
- full macOS GPUI/desktop tests and `World Machine.app` build/validate/packaged smoke/archive/upload remain green.

## Non-goals

No automatic retry, no retry timer/backoff, no resumable or paused model turn, no reconnecting an ended Pi process, no cross-comparison failed-question carryover, no persistent chat history, no token streaming, no concurrent turns, no protocol v2, no provider/model picker, no API-key storage, no new analyst tools, and no mutation authority.
