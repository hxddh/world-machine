# World IR v0.1

This document describes the current semantic contract, not a frozen file format.

## Entity

A thing that exists in the World.

```text
Entity {
  id
  kind
  components: Map<String, Value>
}
```

The kernel does not know what a `Person`, `Player`, `Document`, `Building`, or `Evidence` is.

## Relation

A typed first-class edge between entities.

```text
Relation {
  id
  kind
  from
  to
  properties: Map<String, Value>
}
```

Relations may be asymmetric and stateful.

## Action

The only runtime-facing request to intentionally change the World.

```text
ActionRequest {
  actor?
  action
  args
  caused_by[]
}
```

An Action implementation validates a request against the current authoritative state and produces an `EventDraft`.

## Event

An immutable record of a meaningful transition.

```text
Event {
  id
  kind
  world_time
  actor?
  targets[]
  caused_by[]
  payload
  changes[]
}
```

`caused_by` forms the initial causal graph used by future `Why?` projections.

## StateChange

Generic exact mutations carried by an Event for deterministic replay.

Current v0 operations:

- CreateEntity
- RemoveEntity
- SetComponent
- RemoveComponent
- CreateRelation
- RemoveRelation
- SetRelationProperty
- RemoveRelationProperty

The current implementation applies all changes atomically: either the complete Event is valid or no state mutation is committed.

## Behavior

A deterministic source of Actions in response to committed World Events.

Current implementations:

- `RuleBehavior`
- `NativeBehavior`

A future `AgentBehavior` will use the same contract through a provider-neutral AgentRuntime. LLM/agent intelligence is therefore one Behavior backend, not a privileged kernel primitive.

Behavior execution semantics are deliberately explicit:

1. committed Events enter a FIFO reaction queue;
2. matching Behaviors run in registration order;
3. each Behavior's proposed Actions run in returned order;
4. proposed Actions re-enter the normal `Action -> Event -> State` path;
5. generated Events are appended to the FIFO reaction queue;
6. a hard action budget terminates unbounded reaction chains deterministically.

Every Action produced by a Behavior carries the triggering Event as a causal reference. Replay applies the resulting recorded Events directly and does not re-run Behaviors.

## Projection (next)

A query + rendering description that turns World state/history into a surface.

Initial surface targets:

- Collection
- Inspector
- Timeline
- Canvas

GPUI will implement the renderer but will not own World truth.

## Runtime services

### Clock / Scheduler

Logical time is independent of wall-clock time. The v0 kernel uses an integer logical timestamp; future adapters may map it to real-time, simulated, or turn-based clocks.

A scheduled action is runtime work, not an authoritative state mutation:

```text
ScheduledAction {
  id
  world_time
  ActionRequest
}
```

The Scheduler orders work by `(world_time, insertion_sequence)`, so equal-time actions execute deterministically. Advancing a World executes every due scheduled action through the normal `Action -> Event -> State` path.

A failed scheduled action remains queued. Its attempted time advance is rolled back to the last successfully committed logical time, while earlier successful Events remain committed.

Replay applies historical Events directly and does not re-run scheduler decisions. Pending scheduler work is copied by `World::replay` in v0; historical fork reconstruction currently starts with an empty scheduler because schedule creation is not event-sourced yet. Snapshot/branch hardening will make runtime-service state explicit.

### Replay

Replay applies recorded Events to the baseline state. It never invokes the original decision maker.

### Branch

The v0 branch operation forks from an Event prefix. Future work will add stable branch identifiers, snapshots and copy-on-write storage.

### Capability / Perception

Not implemented yet. Future Behavior execution receives a filtered observation, never global World state by default.

## Reference composition: Tiny Society

The first reference World validates that domain semantics can live entirely above the kernel:

```text
world-core
  -> society-basic system
  -> tiny-society world
```

`world-core` still knows nothing about residents, employment, storms, boats, orders, or dismissal. The World expresses those concepts through generic Entities, Relations, Actions, Events, Behaviors, and scheduled Actions.

The first vertical slice deliberately records the causal chain:

```text
storm_started
  -> boat_damaged
  -> income_lost
  -> loan_requested
  -> temporary_work_assigned
  -> shift_missed
  -> order_lost
  -> worker_dismissed
```

This chain is a reference test for future `Why?` and branch projections rather than a hard-coded kernel story primitive.
