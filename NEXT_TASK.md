# Next Coding Task — M42 Branch Strategy Comparison

Build the first generic comparison layer for divergent World histories.

Tiny Society can now produce materially different futures from the same durable state: after a long-run Harbor Bakery closure, a traditional salaried reopen fails again while an owner-run lean reopen survives. The next step is not another Tiny Society event. It is to make World Machine able to compare those futures as a first-class product capability.

## Product goal

A user should be able to take one World state, create two independent strategies, advance both by the same amount of World time, and understand **what became different and why**.

The comparison must be generic. Tiny Society is the acceptance World, not the abstraction.

## Architecture boundary

Do not add Tiny Society concepts to `world-core`, `world-host`, `world-projection`, or GPUI.

The comparison layer may depend on generic `WorldSession`, `WorldRegistry`, `WorldArchive`, `ProjectionSnapshot`, `SelectionId`, inspector rows, timeline items, and commands. It must not know about Bakery, Mara, jobs, payroll, fishing, or any other Pack-specific semantic name.

Do not modify authoritative World truth merely to support comparison. A comparison is derived from two already-valid World histories.

## M42A — Headless branch comparison

Implement the reusable comparison model before any side-by-side UI.

Requirements:

1. Add a small generic comparison module/crate at the Host/Projection boundary. Prefer a new focused crate if that keeps `world-host` and `world-projection` narrow.
2. Accept two independent World snapshots/history views produced by normal Host sessions. Do not special-case a Pack ID.
3. Produce a deterministic comparison result that can represent at least:
   - left/right titles and World times;
   - visible Entity state differences by stable `SelectionId`;
   - Inspector row differences for matching entities;
   - timeline Events present only on the left or right;
   - commands available only on one side;
   - a concise list of changed/added/removed visible entities when applicable.
4. Equality/diffing must use stable semantic identifiers, not screen position or rendered strings alone.
5. Preserve causal history references. The comparison may summarize Events, but it must not manufacture Events or causes.
6. Comparison must be pure and replay-safe: the same pair of snapshots always produces the same result and never invokes an AgentRuntime, Behavior, wall clock, filesystem mutation, or GPUI.
7. Add unit tests using synthetic generic snapshots so the abstraction is proven independently of Tiny Society.

## M42B — Host strategy harness

Add a generic helper that proves two strategies can be evaluated from the same source archive without sharing mutable state.

Requirements:

1. Open two independent sessions from the same checked `WorldArchive` through `WorldRegistry`.
2. Apply a caller-supplied `ProjectionIntent` sequence independently to each session.
3. Advance both sessions by the same explicit number of background periods when requested.
4. Return the two resulting `ProjectionSnapshot`s plus their generic comparison.
5. A failure on one strategy must not mutate the other session or the source archive.
6. Do not read wall-clock time. M42 receives explicit background periods only.

## Tiny Society acceptance scenario

Use the existing long-run Bakery recovery fork as the product canary:

1. Produce or restore the same durable Tiny Society snapshot in which Harbor Bakery is closed after the long-run demand contraction.
2. Left strategy: invoke `tiny-society.reopen-bakery`.
3. Right strategy: invoke `tiny-society.reopen-bakery-lean`.
4. Advance both by 20 background periods.
5. The generic comparison must make the divergent outcome observable without Pack-specific comparison code:
   - traditional branch ends with Bakery closed;
   - lean branch ends with Bakery open;
   - Mara's visible job/state differs;
   - the two histories contain different recovery/closure Events;
   - relevant visible cash/state differences are represented.
6. Archive/reopen either branch before comparison in at least one regression so the result is proven against durable history, not only in-memory state.

## M42C — Product surface, only after headless semantics are green

Once M42A/B are stable, expose comparison through the generic product shell:

- side-by-side strategy summary;
- changed entities/state first, raw timeline second;
- selection on either side should still use ordinary Inspector/Why surfaces;
- no Tiny Society-specific GPUI View;
- no duplicated World truth in UI state.

Do not start M42C until Linux semantic CI for M42A/B is green.

## Validation

Before merge:

- `bash ./scripts/check-boundaries.sh`
- `cargo fmt --all -- --check`
- semantic Clippy/workspace tests
- generic comparison unit tests
- Tiny Society strategy acceptance regression
- macOS `world-library` / `world-gpui` / desktop regressions
- release `World Machine.app` artifact build remains green

## Why this is next

World Machine already has persistence, replay, causal history, background living, branching, generic projections, and Worlds whose choices now produce materially different long-run outcomes. The missing product primitive is the ability to **see two possible Worlds together**.

M42 turns `Fork` from a debugging/history operation into a user-facing strategy instrument. It is also a direct test of the Pocket Universe thesis: if comparison is genuinely generic, the same primitive should later work for a detective investigation, company simulation, football world, personal-data world, or a future generated World without changing the comparison engine.
