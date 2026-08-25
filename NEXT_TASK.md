# Next Coding Task — M248 Bound Analyst Evidence Previews

Status: implementation complete; presentation-boundary fix applied; exact validation restarted on `agent/m248-analyst-bounded-evidence-preview`.

M214–M247 now provide the installed World Analyst path from immutable saved-World evidence through a restricted long-lived Pi analyst, provider-neutral turns, native runtime/model readiness, in-memory Question → Answer → Evidence history, explicit recovery/new-comparison/cancellation/retry/dismiss flows, retained-question evidence scoping, asynchronous catalog loading/refresh, typed saved-World/runtime drift reconciliation, two-sided selection/Swap, local filtering, Pack/document identity, and stable selected/pair identity across Setup and Active/Fatal UI.

M247 closes the remaining Setup identity ambiguity and keeps maximum-length saved-World IDs reachable without changing selection semantics.

## M248 — bound tool input/output copies in the UI history projection

`world-analyst-client::AnalystToolCall` intentionally carries arbitrary `serde_json::Value` input/output and the turn protocol does not impose a UI-sized payload limit. Today `snapshot_history()` converts every tool input and output with `.to_string()` and stores those complete strings in `PanelTurn`; `render_turn()` then lays the complete copies out in the GPUI history.

That is a presentation-layer scaling bug. A legitimate evidence tool may return a large object/array, causing the desktop panel to allocate another full serialized copy and then build an unbounded text layout for it. The authoritative turn already remains in `DesktopAnalystSession`; the panel does not need another unbounded copy merely to render history.

### Product behavior

- keep Question and Analyst answer rendering unchanged in this slice;
- keep each evidence call's tool name, `ok` / `error` status and call ordering unchanged;
- replace the UI snapshot's unbounded tool `input` and `output` strings with bounded previews;
- use a fixed **4096-byte UTF-8 payload preview limit per input and per output**;
- a payload whose serialized JSON is at or below the limit renders exactly as today and is not marked truncated;
- a payload whose serialized JSON exceeds the limit renders only a valid UTF-8 prefix no larger than 4096 bytes and is explicitly marked as a truncated preview in the UI;
- do not append an ellipsis inside the JSON prefix or otherwise make the prefix look like valid complete JSON; truncation state belongs in adjacent UI metadata/labeling;
- input and output truncation are independent: one may be complete while the other is truncated;
- runtime errors remain unchanged and are not part of this payload-preview slice;
- do not add a full-payload expansion/copy action in M248. The goal is to bound the history projection, not move the same unbounded allocation behind a button.

### Allocation / implementation boundary

Keep the product slice in `apps/world-machine-desktop/src/analyst_panel.rs`. Do not change `AnalystTurn`, `AnalystToolCall`, `DesktopAnalystExchange`, `DesktopAnalystSession`, the turn protocol, Pi tools, provider/model logic or saved-World evidence.

Do **not** implement this as `value.to_string()` followed by `truncate()`: that still creates the complete large temporary JSON string before cutting it down, and a truncated `String` may retain oversized capacity.

Prefer a small local formatter that implements `std::fmt::Write` and accepts serialized `Display` fragments from `serde_json::Value` into a `String::with_capacity(4096)` buffer:

- copy at most the remaining byte budget;
- if a fragment crosses the remaining byte budget, back up to a valid UTF-8 character boundary before copying;
- once the limit is reached, discard later fragments while continuing to return `fmt::Result::Ok(())`, and remember `truncated = true`;
- return a freshly bounded `String` plus truncation metadata, so the UI projection does not retain a capacity proportional to the original payload;
- if the exact serialized payload ends exactly at 4096 bytes, it is complete and must not be marked truncated; truncation is true only when additional serialized bytes exist beyond the retained prefix.

A compact representation is sufficient, for example a local `PanelPayloadPreview { text: String, truncated: bool }`, with `PanelToolCall` holding one preview for input and one for output.

### State / correctness invariants

- `DesktopAnalystSession` continues to retain the complete authoritative `AnalystTurn` / `AnalystToolCall` values exactly as before;
- `snapshot_history()` remains a one-way UI projection and must not mutate the session, evidence scope, process, archives or turn values;
- call order, tool name, `is_error`, Question/Answer content and runtime-error content remain unchanged;
- no provider, model, Pi, protocol, timeout, cancellation, retry, Fatal, New comparison or evidence-scope behavior changes;
- no Library/catalog/filter/pair/Swap/Start/identity behavior changes;
- no persistence, Pack or World authority changes;
- do not introduce filesystem spill, cache files, database persistence or hidden secondary storage for large tool payloads.

### Validation

Required regressions:

- short ASCII input/output serialize exactly, remain byte-for-byte unchanged in the preview and report `truncated == false`;
- payload serialized to exactly 4096 bytes is preserved exactly and is **not** marked truncated;
- payload exceeding 4096 bytes produces `text.len() <= 4096`, reports `truncated == true`, and the stored preview is a fresh bounded allocation rather than a full source string subsequently truncated;
- a multi-byte UTF-8 payload that crosses the byte limit never produces invalid UTF-8 and never exceeds 4096 bytes;
- when only input exceeds the limit, only input is marked truncated; same for output;
- tool call ordering, tool/status fields and runtime errors survive `snapshot_history()` unchanged;
- the UI labels complete vs truncated input/output unambiguously without pretending a truncated JSON prefix is complete JSON;
- existing analyst session/history/retry/cancel/evidence-scope/catalog/identity regressions remain green;
- Linux boundary/Pi/fmt/Clippy/workspace/Pack gates remain green;
- full macOS Library/Packs/GPUI/desktop tests plus `World Machine.app` build/validate/packaged smoke/archive/upload remain green.

## Non-goals

No protocol payload-size limit, no tool-result truncation before the model sees it, no mutation of authoritative evidence, no answer truncation, no runtime-error truncation, no history virtualization/incremental projection yet, no full-payload viewer/export/copy action, no persistence of analyst history, no provider/model changes, no new Pi tools, no selector/catalog changes, and no Pack/World behavior changes.
