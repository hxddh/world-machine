# Next Coding Task — M247 Keep Stable Selected Identity Visible in Setup

Status: implementation complete; exact validation restarted after locking the three Setup identity-containment surfaces.

M214–M246 now provide the installed World Analyst path from immutable saved-World evidence through a restricted long-lived Pi analyst, provider-neutral turns, native runtime/model readiness, in-memory Question → Answer → Evidence history, explicit recovery/new-comparison/cancellation/retry/dismiss flows, retained-question evidence scoping, asynchronous catalog loading/refresh, typed saved-World/runtime drift reconciliation, explicit two-sided pair selection, atomic directional pair swapping, one shared local Setup filter, visible/searchable generic World Pack identity, stable saved-World document IDs on selector cards, and stable ordered pair identity throughout Active/Fatal analysis. M246 also keeps maximum-length exact IDs reachable by separating the Active identity from snapshot actions and allowing horizontal identity scrolling.

## M247 — keep the selected identity visible above each scrollable Setup list

The M245 selector cards make a saved World's durable `WorldDocumentId` legible when a semantic title would otherwise hide it. However each Left/Right saved-World list is capped at `260px` and vertically scrollable, while the fixed heading above that list still renders:

`Left · <semantic title>` / `Right · <semantic title>`

through the older `label_for()` path. A selected card can therefore scroll out of view, leaving the always-visible column heading ambiguous for two legitimate Worlds that share the same semantic title.

Close the Setup side of the same identity problem without changing selection semantics.

### Product behavior

- the fixed heading above each Left/Right selector list identifies the currently selected saved World using semantic title plus the exact stable document ID whenever the title would otherwise hide it;
- preserve M245's non-duplication rule: if the rendered title already equals the document ID because the title is missing/blank or exactly equal to the ID, show the ID only once;
- if the selected ID no longer has a matching in-memory `WorldDocumentSummary`, the fixed heading falls back to the exact selected `WorldDocumentId` rather than inventing a label or failing to render;
- keep the side label explicit and directional (`Left` / `Right`); no sorting, grouping, ranking, or automatic side changes;
- a maximum-length uninterrupted valid ID must remain reachable without widening or overlapping the two-column Setup layout: constrain the fixed identity heading to its column and allow horizontal scrolling when needed;
- containment must cover **every selector-card surface that can carry the stable ID**: when a semantic title exists, the secondary `ID <document-id>` line must be width-constrained/horizontally reachable; when the semantic title is missing/blank/equal to the ID and the exact ID becomes the primary card title, that primary title line must receive the same containment rather than overflowing the card/column;
- it is acceptable to apply the primary-title containment unconditionally so long semantic titles are also contained; do not truncate, abbreviate, normalize, hash, or mutate the rendered title/ID string;
- preserve M244 `Pack <id> · <version>` metadata, summary / World time / event count, selected/opposite styling, and M243 filter behavior;
- do not add Pack metadata to the fixed heading in this slice; M247 is only the durable saved-World identity closure.

### State / implementation boundary

Keep the slice entirely in `apps/world-machine-desktop/src/analyst_panel.rs` presentation.

Prefer extracting the single-document identity lookup already implicit inside M246's `pair_identity_header`, for example:

- `document_identity_label(id: &WorldDocumentId, documents: &[WorldDocumentSummary]) -> String`

It should reuse `document_title()` and M245's `document_id_label()` and fall back to the exact requested ID when the summary is absent. Then:

- M246 `pair_identity_header()` composes the Left/Right pair from that helper, preserving existing Active/Fatal output;
- each Setup selector heading uses the same helper for its selected side;
- the selector-card primary title line and optional secondary ID line each receive stable element IDs plus `min_w(0)` / horizontal overflow containment (or an equivalent exact-string-preserving containment) so both ID-rendering paths remain reachable;
- `document_id_label()`'s returned string and its semantic decision remain unchanged.

Required invariants:

- M247 never mutates `documents`, `left`, `right`, `session`, `history`, failed-question/evidence scope, runtime/readiness, catalog generation, settings, composer, error, cancellation, process, or filter state;
- M243 `document_matches_filter` / `document_visible_for_filter` remain unchanged;
- selected/opposite cards remain visible under non-matching filters exactly as before;
- M244 Pack display/filtering remains unchanged;
- M245 card identity semantics remain unchanged apart from overflow containment on both primary and secondary identity-bearing lines;
- M246 Active/Fatal pair identity output and responsive two-row layout remain unchanged;
- `default_right_for` / `refreshed_right_for` same-Pack-first fallback remains unchanged;
- M241 side selection, M242 explicit Swap, Start eligibility and complete authoritative catalog semantics remain unchanged;
- no Library query, refresh, recheck, persistence, protocol, provider, Pi, Pack, World, or model boundary changes are introduced.

### Validation

Required regressions:

- a selected summary `Maple Street` / `world-1` produces a fixed Setup identity containing both `Maple Street` and exact `ID world-1`;
- missing/blank/equal-to-ID semantic titles emit the ID only once;
- a missing summary falls back to the exact selected `WorldDocumentId`;
- two same-title summaries with different IDs remain distinguishable in both fixed Setup headings and the existing M246 Active/Fatal pair header;
- Left/Right directional order remains unchanged;
- an accepted 128-character uninterrupted ID is preserved exactly by the shared identity helper when rendered as a secondary `ID <document-id>` identity;
- an accepted 128-character uninterrupted ID is also preserved exactly when missing/blank title makes that ID the **primary card title**;
- fixed Setup heading, primary card title, and secondary card ID regions are width-constrained/horizontally reachable rather than truncating or forcing column expansion;
- M246 active identity tests, M245 card-ID tests, M244 Pack tests, M243 filter tests, same-Pack fallback, recency ordering, selected/opposite visibility, two-sided selection, Swap, Start eligibility, evidence-scope invalidation and catalog/runtime drift regressions remain green;
- Linux boundary/Pi/fmt/Clippy/workspace/Pack gates remain green;
- full macOS Library/Packs/GPUI/desktop tests plus `World Machine.app` build/validate/packaged smoke/archive/upload remain green.

## Non-goals

No title/ID editing, title uniqueness enforcement, copy-to-clipboard action, Pack metadata in fixed headings, lineage display/integration, selector sorting/grouping/faceting, new filtering semantics, automatic scrolling to the selected card, persisted selector/filter state, session/evidence identity changes, protocol/provider/model changes, persistence format changes, Pack behavior changes, or World mutation authority.
