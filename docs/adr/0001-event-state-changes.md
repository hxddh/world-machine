# ADR 0001: Events carry generic state changes

Status: Accepted for v0.1

## Context

World Machine needs deterministic replay without making `world-core` understand domain concepts. We also want meaningful semantic events with causal provenance.

## Decision

An Event contains:

- semantic metadata (`kind`, actor, targets, payload, causes), and
- a list of generic `StateChange` operations.

Executing an Action creates an Event. Applying that Event applies its state changes. Replay applies the same Events again to a snapshot/baseline state.

## Consequences

Pros:

- deterministic replay is trivial and domain-independent;
- event provenance and state mutation remain coupled;
- no domain reducers are required in `world-core`.

Trade-offs:

- events may be larger;
- future systems may prefer typed reducers or derived state.

We will revisit only when at least two real worlds demonstrate concrete pressure to change this model.
