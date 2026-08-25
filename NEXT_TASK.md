# Next Coding Task — M244 Surface World Pack Identity in Analyst Setup

Status: implementation complete; validation in progress on `agent/m244-analyst-pack-identity`.

M214–M243 now provide the installed World Analyst path from immutable saved-World evidence through a restricted long-lived Pi analyst, provider-neutral turns, native runtime/model readiness, in-memory Question → Answer → Evidence history, explicit recovery/new-comparison/cancellation/retry/dismiss flows, retained-question evidence scoping, asynchronous catalog loading/refresh, typed saved-World/runtime drift reconciliation, explicit two-sided pair selection, atomic directional pair swapping, and one shared local Setup filter over the already-refreshed saved-World catalog.

## M244 — make Pack identity visible and searchable

The saved-World Library already has a deterministic browsing order: `WorldLibrary::list()` returns the most recently persisted documents first and uses stable document ID only as the tie-breaker. Do not add a second sorting policy in the analyst.

The remaining Setup information gap is Pack identity. `WorldDocumentSummary` already carries a generic `WorldPackRef`, and the analyst's existing initial/right fallback intentionally prefers another World from the same Pack. The current selector cards, however, show title/summary/time/event count but hide the Pack entirely. With multiple World Packs in one Library, users therefore cannot see the criterion behind a same-Pack default/fallback and M243 cannot find a World by Pack identity.

Surface that existing generic identity in the native Setup cards and include it in the local filter.

### Product behavior

- every saved-World card in both Left and Right Setup selectors shows the document's generic Pack identity as secondary metadata;
- display both `WorldPackRef.id` and `WorldPackRef.version` without Pack-specific naming or assumptions about version syntax;
- keep semantic title as the primary identity and preserve the existing summary, World time, and event-count information;
- extend M243's one shared local filter to match Pack ID and Pack version case-insensitively in addition to title, document ID, and display summary;
- an empty/whitespace-only filter still exposes the complete current catalog;
- selected/opposite cards remain visible under non-matching filters exactly as in M243;
- the existing same-Pack-first initial/right fallback remains unchanged; M244 only makes the already-existing Pack fact legible/searchable;
- preserve `WorldLibrary::list()` order exactly. Filtering may hide cards, but M244 must not sort, regroup, or rank the authoritative list or the filtered result;
- no Pack identity is copied into analyst protocol/session state merely for display.

### State / implementation boundary

Keep the slice entirely in native desktop presentation over `WorldDocumentSummary`.

A small pure helper such as `pack_label(&WorldPackRef) -> String` is acceptable if it keeps card rendering and tests explicit. The version string is opaque product data: do not automatically prefix it with `v`, parse it as semver, or special-case known Packs.

Extend the existing M243 `document_matches_filter` helper rather than adding a second filtering path. The authoritative `documents` vector remains unchanged and complete.

Required invariants:

- Pack display/filtering never mutates `documents`, `left`, `right`, runtime/readiness, failed-question/evidence scope, catalog generation, settings, composer, errors, or session/process/cancellation state;
- `default_right_for` / `refreshed_right_for` same-Pack-first semantics remain byte-for-byte or behaviorally unchanged;
- M241 side selection, M242 explicit swap, and M243 selected/opposite visibility semantics remain unchanged;
- Start and Swap eligibility continue to use the complete current catalog and remain independent of filter text;
- catalog refresh/reconciliation remains the sole owner of current summaries and selection fallback;
- no Pack-specific UI branches or Pack-specific comparison semantics are introduced.

### Validation

Required gates:

- generic card metadata exposes both Pack ID and Pack version;
- Pack ID matching is case-insensitive;
- Pack version matching is case-insensitive/string-based and makes no semver assumptions;
- unrelated Pack queries do not match unless another existing title/ID/summary field matches;
- existing title, document-ID, summary, whitespace, selected/opposite, and missing-ID M243 filter regressions stay green;
- existing same-Pack-first right-selection tests remain green and no sorting/reordering is added;
- M230–M243 recovery/new-comparison/cancel/retry/dismiss/evidence-scope/catalog/initial-load/startup-drift/runtime-drift/two-sided-selection/swap/filter regressions remain green;
- Linux boundary/Pi/fmt/Clippy/workspace/Pack gates remain green;
- full macOS Library/Packs/GPUI/desktop tests plus `World Machine.app` build/validate/packaged smoke/archive/upload remain green.

## Non-goals

No sorting change, Pack grouping/faceting, per-Pack picker, fuzzy/semantic ranking, server-side/Library search API, indexing, pagination, persistent filter state, global Library watcher/cache, protocol/provider/model changes, Pack loading/mutation, persistence changes, or World mutation authority.
