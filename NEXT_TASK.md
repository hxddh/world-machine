# Next Coding Task — M242 Explicitly Swap Pending Analyst Pair Sides

M214–M241 now provide the installed World Analyst path from immutable saved-World evidence through a restricted long-lived Pi analyst, provider-neutral turns, native runtime/model readiness, in-memory Question → Answer → Evidence history, explicit recovery/new-comparison/cancellation/retry/dismiss flows, retained-question evidence scoping, asynchronous catalog loading/refresh, typed reconciliation for saved-World and runtime drift, and an idle Setup surface where either side of the pending saved-World pair can be selected explicitly.

## M242 — add an explicit directional pair swap

M241 deliberately makes selecting the World already chosen on the opposite side a strict no-op. That avoids a surprising implicit swap, but it leaves one real usability and semantics gap: with exactly two saved Worlds, a valid `Left=A / Right=B` comparison cannot be reversed to `Left=B / Right=A` at all without a third temporary selection.

The analyst pair is directional, not just a set. `DesktopAnalystEvidenceScope` records left/right document IDs and archive fingerprints separately, the immutable session binds ordered left/right snapshots, and analyst questions or evidence may refer to that orientation. Reversing the pair therefore needs one explicit atomic product transition rather than two selector mutations or an implicit opposite-side click.

Add an explicit `Swap sides` action for the pending pair in idle Setup.

### Product behavior

- render a clear `Swap sides` action alongside the Left/Right Setup selectors when the pending pair is currently usable;
- invoking it atomically changes `Left=A / Right=B` to `Left=B / Right=A` with no intermediate same-World or missing-side state;
- the action is explicit only: M241 same-side/opposite-side selector clicks remain strict no-ops and never auto-swap;
- a successful swap invalidates retained `failed_question` and its M236 evidence scope exactly once because the ordered immutable comparison changed;
- preserve the composer draft, current refreshed catalog, runtime readiness, runtime path/settings state, and unrelated Setup state;
- do not trigger a catalog refresh, runtime recheck, session Start, retry, or model request merely because the pair was swapped;
- Start after a swap must bind immutable snapshots in the new left/right order and the resulting evidence scope must reflect that order;
- Active/Fatal sessions remain immutable and never expose the swap action.

### Eligibility and fail-closed behavior

Swap is available only when all of the following are true:

- phase is idle `Setup`;
- no catalog refresh, runtime check, settings/path operation, session startup, Ask, cancellation settlement, New Comparison close, or other existing busy work is in progress;
- there is no live analyst session;
- both selected IDs exist in the current refreshed `documents` catalog;
- the two IDs are distinct.

If either selected World is missing, the pair is sentinel/invalid, fewer than two saved Worlds remain, or the catalog has failed/been discarded, Swap stays disabled. In particular:

- a missing left remains explicitly replaceable through the M241 Left selector; Swap must not silently move or repair it;
- a missing right remains owned by the existing deterministic catalog fallback/reconciliation rules;
- first-open `right == left` sentinel state cannot be swapped;
- catalog refresh never performs an implicit swap.

### State-transition boundary

Prefer a small pure transition such as `swap_pending_pair_selection` beside M241's `update_pending_pair_selection`.

Required invariants:

- valid distinct `A/B` => exactly `B/A`;
- the two IDs change atomically as one product transition;
- retained failed-question text and evidence scope clear together on a real swap;
- invalid/same-ID input => strict no-op with retained intent preserved;
- composer, runtime, documents, settings, history, session/process/cancellation state are outside the helper and cannot be mutated by it;
- no pair-selection helper may call runtime/catalog/session code.

Do not implement Swap by calling the one-side M241 selector transition twice: the opposite-side no-op invariant is intentional, and a two-step mutation would introduce an invalid intermediate pair and could clear retained intent more than once.

### Concurrency and recovery rules

- reuse the existing Setup eligibility gates; do not create a parallel busy model;
- swapping does not increment catalog refresh generation and does not invalidate pair-independent runtime readiness;
- stale M237 catalog or M239 startup completions retain their existing generation/phase/session gates and may not overwrite a later user transition;
- M239 MissingWorld/LoadWorld startup drift and M240 Spawn drift recovery remain unchanged;
- Fatal recovery and New Comparison return to Setup with their existing selection/refresh semantics; they do not auto-swap;
- no automatic session Start/retry occurs after Swap.

### Implementation boundary

Keep this entirely in native desktop product state. Do not change analyst protocol v1, provider/model selection, Pi/filesystem authority, persistence, World truth, Pack behavior, runtime settings, `DesktopAnalystEvidenceScope` representation, or add a global Library/runtime watcher/cache.

The existing directional evidence scope is already sufficient; M242 only ensures the native pending-pair state can express the reverse orientation safely.

### Validation

Required gates:

- exactly two saved Worlds can be reversed from `A/B` to `B/A` through one explicit action;
- selector clicks on the opposite side remain M241 strict no-ops and never swap automatically;
- a successful swap clears failed-question text and evidence scope in lockstep while preserving composer and runtime readiness;
- invalid/sentinel/missing-side pair states cannot swap and preserve retained intent;
- Swap stays disabled outside idle usable Setup and during every existing busy/runtime/settings/catalog/session state;
- Start after Swap binds the new left/right order and M236 directional evidence-scope matching continues to fail closed for stale retained intent;
- catalog refresh right fallback and missing-left explicit-replacement behavior remain unchanged;
- M230–M241 recovery/new-comparison/cancel/retry/dismiss/evidence-scope/catalog/initial-open/startup-drift/runtime-drift/two-sided-selection regressions remain green;
- Linux boundary/fmt/Clippy/workspace/Pack conformance remain green;
- full macOS GPUI/desktop tests and `World Machine.app` build/validate/packaged smoke/archive/upload remain green.

## Non-goals

No implicit selector-driven swap, no automatic pair repair, no automatic left fallback, no global Library/runtime watcher or cache, no persistent chat history, no automatic startup retry/backoff, no resumable model turn, no reconnect, no streaming, no concurrent turns, no protocol v2, no provider/model picker, no API-key storage, no new analyst tools, and no mutation authority.
