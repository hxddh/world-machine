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

## Behavior (next)

A source of Actions in response to World observations/events.

Planned implementations:

- RuleBehavior
- NativeBehavior
- AgentBehavior

LLM/agent intelligence is therefore one Behavior backend, not a privileged kernel primitive.

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

Replay applies historical Events directly and does not re-run scheduler decisions. Pending scheduler work is copied by `World::replay` in v0; historical fork reconstruction currently starts with an empty scheduler because schedule creation is not yet event-sourced. Snapshot/branch hardening will make runtime-service state explicit.

### Replay

Replay applies recorded Events to the baseline state. It never invokes the original decision maker.

### Branch

The v0 branch operation forks from an Event prefix. Future work will add stable branch identifiers, snapshots and copy-on-write storage.

### Capability / Perception

Not implemented yet. Future Behavior execution receives a filtered observation, never global World state by default.
