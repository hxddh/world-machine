# Next Coding Task — M238 Load Initial Analyst Catalog Off the GPUI Path

M214–M237 now provide the installed World Analyst path from immutable saved-World evidence through a restricted long-lived Pi analyst, stable provider-neutral turns, native runtime/model readiness, an in-memory Question → Answer → Evidence transcript, explicit recovery/new-comparison/cancellation/retry/dismiss flows, retained-question evidence scoping, and background saved-World catalog refresh for fresh Setup transitions.

## M238 — make the first analyst catalog load asynchronous too

M237 moves `WorldLibrary::list()` off the GPUI update/render path for New Comparison, Fatal recovery, and manual Recheck, but the first click on `Analyze saved Worlds…` still calls `library.list()` synchronously inside `open_panel` before the analyst window is created.

That leaves the initial product entry point inconsistent with every later refresh. A large Library, slow filesystem, or expensive document parsing can still block the document click/update path before the user sees any analyst UI.

Make the first catalog load use the same background Setup pipeline as M237.

### Product behavior

- clicking `Analyze saved Worlds…` opens the analyst window immediately without synchronously enumerating or parsing the entire saved-World Library;
- the current saved document remains the fixed left anchor;
- the panel enters Setup and shows `Refreshing saved Worlds…` while the initial catalog load runs on the background executor;
- once the catalog arrives, reconcile the right-hand selection with the same deterministic same-pack-first policy used by M237, then run runtime readiness;
- if fewer than two saved Worlds exist, keep the window open in safe Setup with the existing actionable error rather than failing the click before a window appears;
- if the left anchor is no longer present by the time the background load completes, fail closed with the M237 anchor error;
- initial loading must not clear or invent composer/history/failed-question state;
- closing the window while the load is in flight must make completion harmless.

### Concurrency and state rules

- reuse the M237 catalog-refresh state machine instead of creating a second Library enumeration path;
- initial catalog completion must obey the same generation/phase/session stale-result gate;
- no runtime readiness check or Start action may become usable before the initial catalog is reconciled;
- no synchronous `WorldLibrary::list()` remains in the GPUI click/open path;
- reopening multiple analyst windows remains independent: each window owns its own catalog generation/state and no global cache is introduced.

### Implementation boundary

Keep this entirely in native desktop product state. Do not add a Library daemon/cache, watcher, analyst protocol field, provider/model request, Pi/filesystem authority, persistence record, World truth, or Pack-specific behavior.

Prefer making `AnalystPanelView::new` begin the existing M237 refresh flow with an initially invalid/unselected right-side state, rather than duplicating refresh logic in `open_panel`.

### Validation

Required gates:

- `open_panel` performs no synchronous `WorldLibrary::list()`;
- initial panel creation immediately enters background catalog refresh;
- initial catalog selects the deterministic default right World after load;
- fewer than two Worlds and missing-left cases stay safely visible/non-startable with actionable errors;
- closing/dropping the panel during initial refresh cannot publish stale state or start runtime/session work;
- New Comparison / Fatal recovery / manual Recheck continue to reuse the same refresh path;
- M230–M237 recovery/new-comparison/cancel/retry/dismiss/snapshot-scope/catalog regressions remain green;
- Linux boundary/fmt/Clippy/workspace/Pack conformance remain green;
- full macOS GPUI/desktop tests and `World Machine.app` build/validate/packaged smoke/archive/upload remain green.

## Non-goals

No global Library watcher/cache, no two-sided World selector yet, no automatic pair swapping, no persistent chat history, no automatic retry/backoff, no resumable model turn, no reconnect, no streaming, no concurrent turns, no protocol v2, no provider/model picker, no API-key storage, no new analyst tools, and no mutation authority.
