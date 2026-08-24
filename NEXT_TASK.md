# Next Coding Task — M243 Filter Saved Worlds in Analyst Setup

Status: implementation in progress on `agent/m243-analyst-saved-world-filter`.

M214–M242 now provide the installed World Analyst path from immutable saved-World evidence through a restricted long-lived Pi analyst, provider-neutral turns, native runtime/model readiness, in-memory Question → Answer → Evidence history, explicit recovery/new-comparison/cancellation/retry/dismiss flows, retained-question evidence scoping, asynchronous catalog loading/refresh, typed saved-World/runtime drift reconciliation, explicit two-sided pair selection, and an atomic `Swap sides` action for directional comparisons.

## M243 — locally filter the saved-World selectors

M241 makes both sides independently selectable and M242 makes the ordered pair reversible, but Setup still renders the entire refreshed saved-World catalog in both scrollable columns. That is acceptable for a handful of Worlds and increasingly inefficient as a Library grows: users must visually scan/scroll two duplicated full lists to find a candidate.

Add one lightweight native Setup filter over the already-refreshed in-memory catalog. This is a view/navigation improvement only; it must not become another Library query path or alter pair/session semantics.

### Product behavior

- render one `Filter saved Worlds…` text field in Setup above the Left/Right selectors;
- matching is local, immediate, case-insensitive substring matching over user-visible saved-World identity: semantic title, document ID, and non-empty display summary;
- trim leading/trailing whitespace from the filter; an empty/whitespace-only filter shows the normal complete catalog;
- apply the same filter to both Left and Right columns so the user searches one catalog rather than maintaining divergent side filters;
- always keep each column's current selected World and the opposite-side World visible even when they do not match the filter, so orientation and the M241 no-implicit-swap state remain obvious;
- matching cards retain their existing selected/opposite/eligible styling and click semantics;
- if there are no additional matches, show a small local `No other saved Worlds match this filter` message rather than treating it as a catalog failure;
- clearing/editing the filter must never change Left/Right IDs, failed-question/evidence scope, runtime readiness, catalog generation, settings, composer draft, errors, or session state.

### Lifecycle and eligibility

- the filter is a Setup-only navigation control; Active/Fatal surfaces remain unchanged;
- filter edits are local presentation state and do not acquire or change the analyst busy/runtime/settings/catalog/session gates;
- New Comparison/Fatal recovery keep their existing catalog-refresh behavior; the current filter may be preserved across the transition if the same analyst window remains open, but it must not affect reconciliation;
- closing/reopening the analyst window starts with an empty filter;
- no automatic Start, selection, swap, refresh, retry, or runtime check is triggered by filter edits.

### State / implementation boundary

Keep filtering entirely above `WorldLibrary` and analyst session/runtime layers.

Prefer a small pure matcher such as `document_matches_filter(document, query)` and a rendering helper that decides whether a card is visible. The authoritative `documents` vector remains the complete latest catalog; never replace it with a filtered subset.

Required invariants:

- filtering changes presentation only;
- `documents`, `left`, `right`, `runtime`, `failed_question`, `failed_question_scope`, `last_error`, session/process/cancellation state and catalog generation are not mutated by filter edits;
- selected/opposite IDs remain visible when present in `documents`, even when the query does not match them;
- M241 same/opposite selector no-op rules and M242 explicit-swap-only rule remain unchanged;
- Start continues to validate against the complete current catalog, not the filtered view;
- catalog refresh reconciliation continues to operate on the complete returned list and cannot be influenced by filter text.

### Validation

Required gates:

- title, ID and display-summary matching are case-insensitive and whitespace-normalized;
- empty filter exposes the full catalog;
- selected and opposite cards remain visible under a non-matching filter;
- a non-matching filter cannot make an invalid/missing selected ID appear valid;
- selecting a visible matching candidate follows M241 exactly and clears retained failed-question/evidence scope only on a real pair change;
- `Swap sides` remains available according to M242 pair/catalog/busy eligibility and is not gated by whether the two selected cards match the filter;
- Start eligibility and immutable ordered session binding are unaffected by filter text;
- catalog refresh, missing-left recovery, deterministic right fallback, runtime drift recovery, stale completion gates and all M230–M242 regressions remain green;
- Linux boundary/fmt/Clippy/workspace/Pack gates remain green;
- full macOS GPUI/desktop tests plus `World Machine.app` build/validate/packaged smoke/archive/upload remain green.

## Non-goals

No server-side/Library search API, no indexing, fuzzy/semantic search, ranking, pagination, sorting redesign, per-side divergent filters, persistent filter state, global Library watcher/cache, protocol/provider/model changes, new analyst tools, persistence changes, or World mutation authority.
