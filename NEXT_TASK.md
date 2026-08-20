# Next Coding Task — M229 Analyst Conversation Transcript

M214–M228 now provide a complete installed-app path from immutable saved-World evidence through a restricted long-lived Pi analyst, a stable provider-neutral turn protocol, strict Rust process ownership, native runtime setup, packaged validation, functional startup probing, and a model-state gate that prevents the known `model: null` false-ready case before the first question is sent.

## M229 — retain the user's questions with completed analyst turns

The native analyst panel is now usable across multiple sequential asks, but its visible history is incomplete: the composer clears the question before dispatch, while `DesktopAnalystSession` retains only returned `AnalystTurn` values. After several asks, the panel renders answers and evidence calls without the question that produced each answer.

Add the smallest product-owned conversation transcript needed to keep user intent paired with the provider-neutral result.

### Product model

Introduce an immutable completed exchange shape below GPUI, owned by the desktop analyst session. Each completed exchange contains:

- the exact non-empty user prompt accepted for that ask;
- the corresponding provider-neutral `AnalystTurn` returned by M220.

On a successful ask, append the prompt and turn atomically and preserve their order for the lifetime of the session. A failed ask must not fabricate a completed exchange.

Keep the existing process/session state machine and single-flight semantics. If retaining `turns()` is useful for compatibility, derive or maintain it without allowing prompt/turn ordering to diverge.

### Native analyst surface

Render every completed exchange as a question/answer pair:

- clearly distinguish the user's question from the analyst answer;
- keep existing evidence-call and runtime-error disclosure attached to the same exchange;
- preserve chronological order across follow-up asks;
- do not expose raw Pi/provider events or internal request envelopes.

Do not require persistence across app launches in this milestone.

### Failed asks

The panel currently clears the composer before the background ask completes. Do not silently lose the user's text when an ask fails before producing a completed exchange. Preserve enough product state to let the user see or retry the failed question without creating a fake successful history item.

Fatal-session behavior and snapshot/process cleanup remain unchanged.

### Layering and authority

- GPUI must consume only desktop product exchange state, not Pi/RPC structures.
- M220 remains the provider-normalization boundary.
- The immutable archive pair, restricted tools, and no-mutation authority remain unchanged.
- No transcript file persistence, session resume, token streaming, concurrent asks, model selector, provider settings, or credential management.

## Validation

Required gates:

- two successful asks retain two prompts paired with the correct turns in order;
- a recoverable failed ask does not append a completed exchange and its prompt is not silently lost in the panel;
- a fatal failed ask does not append a completed exchange and existing cleanup semantics stay green;
- native history renders question + answer + evidence for each completed exchange;
- panel source remains above process/Pi implementation details;
- existing M219–M228 protocol, probe, model-state, packaged-runtime, and desktop-session regressions remain green;
- Linux boundary/fmt/Clippy/workspace tests remain green;
- full macOS GPUI/desktop tests and `World Machine.app` build/validate/packaged smoke/archive/upload remain green.

## Non-goals

No persistent chat history, no resuming analyst sessions after app restart, no streaming UI, no concurrent turns, no protocol v2, no model/provider picker, no API-key storage, no new tools, and no mutation authority.
