# Next Coding Task — M246 Keep Stable Pair Identity Visible During Analysis

Status: implementation complete; P2 responsive-header fix applied; validation in progress on `agent/m246-analyst-active-pair-identity`.

M214–M245 now provide the installed World Analyst path from immutable saved-World evidence through a restricted long-lived Pi analyst, provider-neutral turns, native runtime/model readiness, in-memory Question → Answer → Evidence history, explicit recovery/new-comparison/cancellation/retry/dismiss flows, retained-question evidence scoping, asynchronous catalog loading/refresh, typed saved-World/runtime drift reconciliation, explicit two-sided pair selection, atomic directional pair swapping, one shared local Setup filter, visible/searchable generic World Pack identity, and stable saved-World document IDs on Setup selector cards when semantic titles would otherwise hide them.

## M246 — preserve that stable identity after Start

M245 closes an important Setup ambiguity: `display_title` is free-form and non-unique, while `WorldDocumentId` is the durable identity used by pair selection and `DesktopAnalystEvidenceScope`. However the Active/Fatal analyst surface still renders the fixed snapshot pair as:

`<left semantic title> ↔ <right semantic title>`

through `label_for()`, which prefers `document_title()` and therefore hides the document ID again whenever a semantic title exists. Two legitimate saved Worlds with the same title can be distinguishable during Setup and then become ambiguous immediately after Start, even though the running analyst remains bound to two distinct ordered document IDs and archive fingerprints.

Carry the M245 identity rule into the read-only Active/Fatal pair header without changing the fixed evidence pair or session behavior.

### Product behavior

- the Active analyst header identifies both sides using semantic title as the primary human-readable label plus the exact stable document ID when the title would otherwise hide it;
- use the same non-duplication rule as M245: if a side's rendered primary title already equals its document ID because the title is missing/blank or exactly equal to the ID, do not repeat the ID;
- a suitable compact form is `Maple Street · ID world-1 ↔ Maple Street · ID world-2`; exact visual punctuation may follow the current native style, but Left/Right order must remain explicit and unchanged;
- if a side no longer has a matching in-memory `WorldDocumentSummary`, fall back to its exact `WorldDocumentId` rather than inventing a title or failing to render;
- Fatal state uses the same `render_active()` surface and must retain the same pair identity while recovery is offered;
- preserve the existing `Read-only · fixed snapshot pair` status and `New comparison` behavior;
- long exact IDs must not overlap or be clipped by the status/actions: the identity occupies its own full-width horizontally scrollable row, with snapshot status/actions on a separate row;
- do not add Pack metadata to the Active header in this slice; M246 is specifically the stable evidence-source identity closure from M245, not a redesign of the active header;
- do not change Setup selector headings/cards, filtering, Library order, or pair selection.

### State / implementation boundary

Keep the slice entirely in `apps/world-machine-desktop/src/analyst_panel.rs` presentation.

Prefer a small pure helper over changing `label_for()` globally, because `label_for()` is also used by Setup selector headings and M246 should not silently broaden their formatting. For example:

- `document_identity_label(document: &WorldDocumentSummary) -> String`, reusing `document_title()` and M245's `document_id_label()`; and/or
- `pair_identity_label(id: &WorldDocumentId, documents: &[WorldDocumentSummary]) -> String`, falling back to the exact ID when the summary is absent.

Then use that helper only in `render_active()` for the Left/Right fixed snapshot pair.

Required invariants:

- Left/Right ordering remains directional and exactly matches the running `DesktopAnalystSession` evidence scope;
- M246 never changes `left`, `right`, `documents`, `session`, `history`, failed-question/evidence scope, runtime/readiness, catalog generation, settings, composer, error, cancellation, or process state;
- no catalog refresh, runtime recheck, session restart, model request, retry, or close operation is caused by rendering identity labels;
- M245 Setup `document_id_label` behavior remains unchanged;
- M244 Pack display/filtering and M243 filtering remain unchanged;
- M241 side selection and M242 explicit swap semantics remain unchanged;
- `New comparison` and fatal recovery state transitions remain unchanged;
- no protocol/provider/Pi/persistence/World/Pack mutation or new identity authority is introduced.

### Validation

Required regressions:

- two summaries with the same semantic title and IDs `world-1` / `world-2` produce distinct active identity labels containing the exact respective IDs;
- a summary whose primary title already falls back to its ID emits only the ID once;
- a summary whose semantic title exactly equals its ID emits only the ID once;
- a missing summary falls back to the exact requested `WorldDocumentId`;
- Left/Right ordering is preserved by the rendered pair label;
- the identity region is full-width and horizontally scrollable independently of the snapshot status/actions so an uninterrupted maximum-length ID remains reachable without overlap/clipping;
- the same identity helper is usable in Active and Fatal rendering without state mutation;
- M245 Setup ID-label tests, M244 Pack tests, M243 filter tests, same-Pack fallback, recency ordering, selected/opposite visibility, two-sided selection, swap, Start eligibility, evidence-scope invalidation and catalog/runtime drift regressions remain green;
- Linux boundary/Pi/fmt/Clippy/workspace/Pack gates remain green;
- full macOS Library/Packs/GPUI/desktop tests plus `World Machine.app` build/validate/packaged smoke/archive/upload remain green.

## Non-goals

No Active-header Pack display, lineage display, editable titles/IDs, copy-to-clipboard action, selector redesign, sorting/grouping/faceting, filter changes, persisted UI state, session/evidence identity changes, analyst protocol changes, provider/model changes, persistence format changes, Pack behavior changes, or World mutation authority.
