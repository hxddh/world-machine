# Next Coding Task — M241 Select Either Saved World in Analyst Setup

M214–M240 now provide the installed World Analyst path from immutable saved-World evidence through a restricted long-lived Pi analyst, provider-neutral turns, native runtime/model readiness, in-memory Question → Answer → Evidence history, explicit recovery/new-comparison/cancellation/retry/dismiss flows, retained-question evidence scoping, asynchronous catalog loading/refresh, and typed reconciliation for both saved-World drift and runtime Spawn drift at the Setup → Start boundary.

## M241 — make fresh analyst comparison selection two-sided

The remaining Setup limitation is artificial: the World from which the analyst window was opened is permanently fixed as `left`, while the user may choose only `right`. The lower session/evidence model already accepts any two distinct saved `WorldDocumentId`s, and M235/M236 already define how retained failed intent must be invalidated when the immutable comparison changes.

Allow the user to choose either side of the pending saved-World pair while keeping all existing snapshot, recovery, refresh, and fail-closed guarantees.

### Product behavior

- the saved World that opened the analyst window remains the **initial** left selection; the existing deterministic same-pack-first policy still chooses the initial right selection after the first asynchronous catalog load;
- in idle Setup, expose explicit Left and Right saved-World selectors/cards so either side can be changed before Start;
- a selector must never accept the World already selected on the opposite side; selecting the current value or the opposite-side value is a strict no-op;
- do **not** automatically swap sides when the user selects the opposite-side World;
- any real change to either left or right invalidates retained `failed_question` and its M236 evidence scope exactly once, while preserving composer draft, runtime readiness, settings, and unrelated Setup state;
- Start remains available only for two distinct IDs that both exist in the current refreshed catalog and while the existing runtime readiness is Ready;
- after Start, the immutable archive snapshots/evidence scope bind exactly the selected left/right pair; Active/Fatal sessions never expose pair mutation.

### Catalog refresh behavior

Generalize the M237–M239 pair reconciliation without introducing a second catalog path.

- if both selected IDs still exist after refresh, preserve both selections and refreshed metadata;
- if the selected right disappears while left still exists, preserve the existing deterministic `default_right_for(left, documents)` fallback and invalidate retained failed intent when the pair changes;
- if the selected left disappears but the selected right still exists, keep Setup fail-closed but keep the **Left** selector usable so the user can explicitly choose another saved World; do not silently replace or swap the missing left selection;
- if either side is missing, Start stays disabled until the user explicitly restores a valid distinct pair;
- if catalog refresh itself fails, keep the M237 behavior that discards stale `documents` so no stale selector remains actionable;
- fewer than two saved Worlds remains an actionable, non-startable Setup state;
- New Comparison, Fatal recovery, manual Recheck, and first-open loading continue to reuse the same per-window catalog refresh pipeline.

### State-transition boundary

Prefer replacing right-only `update_pending_right` with a small pure pair-selection transition that receives the opposite/current IDs plus retained failed-question text/scope and changes only the requested side.

Required invariants:

- same-side current selection => no-op, retained intent preserved;
- opposite-side selection => no-op, retained intent preserved;
- real left change => only left changes, retained failed intent/scope cleared;
- real right change => only right changes, retained failed intent/scope cleared;
- no runtime/catalog refresh is triggered merely by local pair selection;
- M236 fresh-session evidence-scope equality remains the authority for same-ID archive-content changes.

### Concurrency and recovery rules

- Left/Right selectors are disabled during catalog refresh, runtime/path settings work, startup, Active/Fatal sessions, and any other existing busy state;
- runtime readiness remains pair-independent and may be preserved across an idle local pair change;
- stale catalog/startup completions retain the M237/M239 generation/phase/session gates and may not overwrite a later explicit selection;
- M239 MissingWorld/LoadWorld startup drift and M240 Spawn drift recovery remain unchanged;
- no automatic session Start/retry occurs after a pair change or reconciliation.

### Implementation boundary

Keep this entirely in native desktop product state. Do not change analyst protocol v1, provider/model selection, Pi/filesystem authority, persistence, World truth, Pack behavior, runtime settings, or add a global Library watcher/cache.

### Validation

Required gates:

- first-open behavior still anchors left to the opening saved World and deterministically chooses right;
- idle Setup can explicitly change left or right while preserving distinctness;
- same-value/opposite-side selections are strict no-ops;
- a true change on either side clears retained failed-question text and evidence scope in lockstep;
- right disappearance retains deterministic fallback; left disappearance remains safe/non-startable but allows explicit left replacement;
- pair controls stay disabled outside idle usable Setup;
- Start binds exactly the selected pair and no Active/Fatal pair mutation is possible;
- M230–M240 recovery/new-comparison/cancel/retry/dismiss/evidence-scope/catalog/initial-open/startup-drift/runtime-drift regressions remain green;
- Linux boundary/fmt/Clippy/workspace/Pack conformance remain green;
- full macOS GPUI/desktop tests and `World Machine.app` build/validate/packaged smoke/archive/upload remain green.

## Non-goals

No automatic pair swapping, no automatic left fallback, no global Library/runtime watcher or cache, no persistent chat history, no automatic startup retry/backoff, no resumable model turn, no reconnect, no streaming, no concurrent turns, no protocol v2, no provider/model picker, no API-key storage, no new analyst tools, and no mutation authority.
