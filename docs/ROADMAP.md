# Implementation Roadmap

Pocket Universe is the architecture north star. Tiny Society is the first product and reference World.

World Machine is no longer only a kernel experiment. The repository now has a durable macOS product shell, multiple Worlds, background progression, causal inspection, and a Tiny Society whose long-running choices produce materially different economic futures.

This roadmap separates the stable runtime foundation from the current product frontier.

## Phase I — Semantic World Runtime

### M0 — Repository and architecture guardrails

Status: implemented and enforced by GitHub CI.

- Rust workspace
- architecture constitution (`AGENTS.md` / architecture docs)
- boundary checker
- formatting, Clippy, semantic tests, and macOS product CI
- release `World Machine.app` artifact build

### M1 — Minimal World data model

Status: implemented.

- Entity
- Relation
- Value
- WorldState

### M2 — Action -> Event -> State

Status: implemented.

- ActionRegistry
- validated Action evaluation
- immutable Event history with causal references
- generic StateChange operations
- atomic event application
- failed Actions/Events do not partially mutate state

### M3 — Deterministic history / replay / fork

Status: implemented for the current product surface, with further branch hardening still possible.

- event log
- replay from baseline
- logical clock preserved by replay
- historical prefix fork
- durable archives reconstruct the same visible history

Known design frontier:

- richer explicit branch identity / lineage metadata
- more efficient snapshot + suffix replay for very large Worlds
- scheduler-history reconstruction beyond the current archive model

These are optimization/hardening tasks, not blockers for the current product loop.

### M4 — Clock / Scheduler

Status: implemented.

- logical World time
- scheduled `ActionRequest`s
- deterministic `(time, insertion order)` execution
- failed scheduled Actions do not corrupt state
- replay does not re-run historical scheduler decisions

Wall-clock time remains outside World truth.

### M5 — Behavior Runtime

Status: implemented.

- RuleBehavior / NativeBehavior
- event subscriptions
- event -> behavior -> action -> event loop
- causal trigger propagation
- deterministic ordering
- loop/action budget protection
- replay remains Event-only and does not re-run Behaviors

### M6 — Deterministic Tiny Society vertical slice

Status: implemented and substantially expanded beyond the original slice.

The original validation goal was that a coherent social World must work without an LLM. Tiny Society now proves that with residents, places, jobs, money, relationships, scheduled living, causal crises, recovery, and long-running economic feedback.

### M7 — AgentRuntime boundary

Status: implemented.

- provider-neutral AgentRuntime
- MockAgentRuntime
- scoped perception
- AgentDecisionEvent
- historical decisions replay without model calls
- Agent output remains a proposed World Action, never authoritative state mutation

### M8 — pi_agent_rust adapter

Status: implemented in the workspace as the out-of-process `world-pi-rpc` boundary and exercised by semantic CI.

- no `pi_agent_rust` dependency in `world-core`
- external decision transport
- strict offered-Action protocol
- filtered observation
- fail-closed handling for invalid decisions/tool attempts/protocol failures

Future optimization, not a prerequisite for product work:

- persistent RPC sessions
- richer dynamic World Action tool registration if/when the external Pi runtime surface makes that useful and stable

## Phase II — Generic Product Surface

### M9 — Projection + GPUI shell

Status: implemented and used by the product.

Generic projection surfaces include:

- Collection
- Inspector
- Timeline
- Semantic Canvas
- causal Why projection
- ProjectionCommand
- Pack-neutral ProjectionIntent

`world-gpui` renders generic Projection state. Tiny Society does not own a parallel application UI model.

### M10 — World Host + multiple Worlds

Status: implemented.

- WorldRegistry / WorldRegistration
- Pack descriptors and version checks
- integrity-checked archive opening
- generic WorldSession lifecycle
- Tiny Society
- Future Archaeologist
- built-in World registration

The presence of materially different Worlds is the current architecture canary: adding a World must not introduce Pack-specific concepts into `world-core` or generic GPUI rendering.

### M11 — Durable `.world` documents and Library

Status: implemented.

- WorldArchive persistence
- integrity validation
- World Library
- durable sessions
- optimistic revision/conflict checks
- candidate -> persist -> commit transaction shape
- Save As / external document targets
- no-op persistence detection

A persistence failure or stale document conflict does not advance the live World.

### M12 — macOS document product

Status: implemented and continuously validated on macOS CI.

- `World Machine.app`
- Home / World document windows
- native document identity/title behavior
- generic GPUI product shell
- Tiny Society and Future Archaeologist desktop regressions
- release `.app` archive artifact

## Phase III — Living Worlds

### Durable background living

Status: implemented.

- Pack-neutral `WorldSession::advance_background(periods)`
- Tiny Society maps a background period to deterministic living progression
- durable background candidate transaction
- static Worlds remain true persistence no-ops when their archive does not change
- background progression never reads wall-clock time from World/runtime truth

### Observer clock

Status: implemented outside `.world` truth.

- device-local observer metadata
- bounded wall-clock -> background-period policy
- claim / rollback semantics
- catch-up before document presentation
- failed durable catch-up leaves the live World untouched

### Return experience

Status: implemented.

- transient visit cursor
- `While you were away`
- return briefing survives the current visit but is not persisted as World truth
- ordinary World interaction clears the transient return mode
- causal events remain inspectable through Timeline / Inspector / Why

## Phase IV — Tiny Society as a Causal Living World

This phase is the current product proof that World Machine can support a persistent society rather than a scripted story.

### Economic circulation

Status: implemented.

- work transfers real workplace cash to residents
- resident purchases transfer real personal cash into Harbor Bakery
- Bakery revenue occurs before payroll in the deterministic living day
- no synthetic income is created merely to keep the simulation alive

### Long-run institutional risk

Status: implemented.

- payroll reserve exhaustion becomes durable history
- Pub and School can lose the ability to fund future payroll
- income disruption propagates to households
- residents spend savings before changing consumption
- `bread_budget_cut` protects emergency savings rather than deleting wealth
- reduced household demand can later produce Bakery payroll crisis and closure

### Consequences and recovery

Status: implemented.

- retaining Jonas has a real long-run payroll cost
- payroll shortfall causes Bakery closure through a persisted causal edge
- Mara can reopen with personal savings
- reopening does not erase prior employment/history consequences
- restored fishing can repair Jonas's income path
- support/reciprocity can complete and be repaid
- restored fishing income can spill back into local Bakery demand

### Adaptive recovery

Status: implemented through M38 and validated against the current local-economy model.

From the same durable Bakery closure:

- traditional salaried reopen: higher investment, restores Mara's fixed Bakery wage, and can fail again under weak demand;
- lean owner-run reopen: lower investment, changes Mara to owner-operator, removes the fixed Bakery payroll burden, and survives the same tested horizon.

This is an important product threshold: a fork now changes the World’s operating structure, not just one Event.

## Phase V — Strategy as a First-Class World Primitive

### M42 — Branch Strategy Comparison

Status: next.

Goal: make divergent World histories understandable side by side without Pack-specific comparison code.

#### M42A — Headless generic comparison

Compare two `ProjectionSnapshot`s/history views deterministically using stable semantic identifiers:

- World time
- visible Entity/Inspector state differences
- left-only/right-only Events
- command differences
- added/removed/changed visible entities

No AgentRuntime, wall clock, filesystem mutation, GPUI, or Pack-specific semantics in the comparison engine.

#### M42B — Host strategy harness

From the same checked archive:

- open two independent sessions;
- apply independent ProjectionIntent strategies;
- advance both by the same explicit background periods;
- compare the outcomes;
- failure on one branch must not mutate the other or the source archive.

Tiny Society acceptance case:

- source: same long-run Bakery closure;
- left: traditional reopen;
- right: lean owner-run reopen;
- advance both 20 periods;
- generic comparison must expose closed-vs-open Bakery outcome, Mara state difference, cash/state changes, and divergent Event history.

#### M42C — Strategy comparison product surface

Only after M42A/B semantic CI is green:

- side-by-side outcome summary
- changed state before raw history
- ordinary Inspector / Why navigation on either side
- no Tiny Society-specific GPUI View
- no duplicated World truth in UI state

## After M42

The next decisions should be driven by whether the comparison primitive remains generic across a second materially different World.

Likely directions:

1. apply strategy comparison to Future Archaeologist or a small new canary World;
2. strengthen branch identity/lineage only where the product needs it;
3. expose Builder/World Pack composition after the runtime can create, live in, fork, and compare multiple Worlds cleanly;
4. return to richer AgentRuntime integration only where cognition adds value that deterministic World systems cannot provide.

The project should resist adding infrastructure merely because it is architecturally interesting. New runtime primitives should be justified by a product behavior that at least two different Worlds can use.
