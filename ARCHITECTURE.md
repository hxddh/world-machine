# World Machine Architecture

## North star

A `.world` is a persistent software object with state, history, rules, actions, intelligence, time, projections, capabilities, and branches.

Tiny Society is the first reference product. Pocket Universe is the long-term architecture constraint, not the current feature target.

## Thin waist

The semantic kernel is intentionally small:

- Entity
- Relation
- Event
- Action
- Behavior (later milestone)
- Projection (later milestone)

Runtime services:

- Clock / Scheduler
- Snapshot / Replay / Branch
- Capability / Perception

## Dependency direction

```text
world-core
   |\
   | +--> world-gpui        (future)
   |
   +----> world-agent       (future)
             |
             +--> world-pi  (future)

world-core --> reusable systems --> world packs
```

Forbidden:

- `world-core -> GPUI`
- `world-core -> pi_agent_rust`
- `world-core -> Tiny Society domain`

## State transition rule

External/domain code must not directly mutate authoritative world state.

```text
ActionRequest
    -> validate
    -> produce EventDraft
    -> materialize Event
    -> apply typed StateChange operations
    -> append immutable Event log
```

Replay applies recorded events and never re-runs the original decision maker.

## Why events carry state changes

A semantic Event records both human-meaningful provenance and the exact generic state changes required for deterministic replay. This keeps reducers independent of domain concepts while preserving causal history.

This is an intentionally minimal v0 design. If multiple real worlds later demonstrate a better reducer model, evolve it then rather than guessing now.

## Licensing boundary

`world-core` and the provider-neutral agent protocol must not depend on agent implementations whose licenses add field-of-use, party, or deployment restrictions. Restricted or non-standard runtimes belong behind optional adapters, preferably out-of-process.

GPUI is a renderer dependency, not a kernel dependency. `pi_agent_rust` is an agent runtime adapter, not a World IR dependency. See `docs/LICENSING.md`.

## Systems and World Packs

Reusable domain semantics live above the kernel:

```text
world-core
   |
   +--> systems/*
           |
           +--> worlds/*
```

A System may define domain Actions, Behaviors, schemas, and helpers while depending only on generic World primitives. A World Pack composes Systems, seeds entities/relations, schedules work, and defines world-specific rules and projections.

The first concrete example is `systems/society-basic` + `worlds/tiny-society`. The kernel must remain unchanged when a World introduces concepts such as residents, jobs, weather, boats, orders, or relationships.
