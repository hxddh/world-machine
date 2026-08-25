# Next Coding Task — M245 Surface Stable Saved-World Identity in Analyst Setup

Status: implementation complete; validation in progress on `agent/m245-analyst-document-identity`.

M214–M244 now provide the installed World Analyst path from immutable saved-World evidence through a restricted long-lived Pi analyst, provider-neutral turns, native runtime/model readiness, in-memory Question → Answer → Evidence history, explicit recovery/new-comparison/cancellation/retry/dismiss flows, retained-question evidence scoping, asynchronous catalog loading/refresh, typed saved-World/runtime drift reconciliation, explicit two-sided pair selection, atomic directional pair swapping, one shared local Setup filter, and visible/searchable generic World Pack identity.

## M245 — make the stable saved-World document ID legible

`WorldDocumentMetadata.display_title` is optional free-form metadata, not a unique Library identity. Multiple saved Worlds may therefore legally have the same semantic title. The analyst already uses `WorldDocumentId` as the durable Left/Right identity: selection helpers operate on it, the local filter can match it, and `DesktopAnalystEvidenceScope` binds retained evidence to the ordered left/right document IDs plus archive fingerprints.

Today `document_title()` intentionally prefers a semantic display title and only falls back to the document ID. As a result, whenever a World has a non-empty semantic title, its stable Library identity is hidden from the selector card. Two Worlds with the same title and Pack can therefore be difficult to distinguish precisely even though the analyst internally treats them as distinct evidence sources.

Surface the existing `WorldDocumentId` in the native selector presentation without changing selection or evidence semantics.

### Product behavior

- when a saved World has a non-empty semantic display title that differs from its document ID, show its exact stable document ID as secondary metadata on both Left and Right selector cards;
- use a compact generic label such as `ID <document-id>`; preserve the exact ID string and do not normalize, parse, abbreviate, hash, or derive another identity;
- when the semantic title is missing/blank and the primary card title already falls back to the document ID, do not add a duplicate secondary ID line;
- likewise, if the semantic display title is exactly the same string as the document ID, avoid rendering a redundant duplicate ID line;
- preserve M244's visible `Pack <id> · <version>` metadata and the existing summary / World time / event-count information;
- preserve the current Library recency order and M243 filtering behavior exactly; the filter already matches document IDs and M245 does not add another matcher or ranking path;
- selected/opposite cards remain visible under non-matching filters exactly as before;
- no document ID is copied into new analyst protocol/session state merely for display — use the existing `WorldDocumentSummary.id`.

### State / implementation boundary

Keep the slice entirely in `apps/world-machine-desktop/src/analyst_panel.rs` presentation over `WorldDocumentSummary`.

A small pure helper is preferred, for example:

- `document_id_label(document: &WorldDocumentSummary) -> Option<String>`

The helper should return a label only when the primary semantic title would otherwise hide the stable ID. It may compare the already-defined semantic `document_title(document)` with `document.id.to_string()` so the rule remains aligned with the actual rendered primary title.

Required invariants:

- M245 does not mutate `documents`, `left`, `right`, failed-question/evidence scope, runtime/readiness, catalog generation, settings, composer, errors, or session/process/cancellation state;
- M243 `document_matches_filter` / `document_visible_for_filter` behavior remains unchanged;
- M244 Pack label/filter behavior remains unchanged;
- `default_right_for` / `refreshed_right_for` same-Pack-first fallback remains unchanged;
- M241 side selection and M242 explicit swap semantics remain unchanged;
- Start and Swap eligibility continue to use the complete authoritative catalog;
- no sorting, grouping, ranking, Library query, persistence, protocol, provider, Pi, World, or Pack mutation boundary changes are introduced.

### Validation

Required regressions:

- a document with semantic title `Maple Street` and ID `world-1` exposes secondary label `ID world-1`;
- a document with missing semantic title uses `world-1` as its primary title and emits no duplicate secondary ID label;
- a blank/whitespace semantic title likewise emits no duplicate secondary ID label;
- a semantic title exactly equal to `world-1` emits no duplicate secondary ID label;
- the ID string is preserved exactly as accepted by `WorldDocumentId`;
- M243 ID filtering remains green without code changes to the matcher;
- M244 Pack ID/version display and filtering remain green;
- same-Pack-first fallback, recency ordering, selected/opposite visibility, two-sided selection, swap, Start eligibility, evidence-scope invalidation and catalog/runtime drift regressions remain green;
- Linux boundary/Pi/fmt/Clippy/workspace/Pack gates remain green;
- full macOS Library/Packs/GPUI/desktop tests plus `World Machine.app` build/validate/packaged smoke/archive/upload remain green.

## Non-goals

No renaming/editing document IDs, title uniqueness enforcement, automatic title generation, copy-to-clipboard action, tooltip/popover, sorting/grouping/faceting, new filtering semantics, Library search/indexing, persistent selector state, protocol/session identity changes, persistence format changes, provider/model changes, Pack behavior changes, or World mutation authority.
