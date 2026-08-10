# Implementation Roadmap

Pocket Universe is the architecture north star. Tiny Society is the first product and reference World.

## M0 — Repository and architecture guardrails

Status: implemented and passing GitHub CI.

- Rust workspace
- `world-core`
- `world-cli`
- architecture constitution
- CI definition
- boundary checker

## M1 — Minimal World data model

Status: implemented and passing GitHub CI.

- Entity
- Relation
- Value
- WorldState

## M2 — Action -> Event -> State

Status: implemented and passing GitHub CI.

- ActionRegistry
- validated Action evaluation
- Event + causal references
- generic StateChange operations
- atomic event application
- failed actions/events do not partially mutate state

## M3 — Deterministic history

Status: initial implementation and passing GitHub CI.

- event log
- replay from baseline
- clock preserved by replay
- prefix fork

Next hardening:

- explicit snapshot type
- stable branch identifiers
- event/cause validation
- snapshot + suffix replay

## M4 — Clock / Scheduler

Status: implemented and passing GitHub CI.

- logical world time accessor
- scheduled `ActionRequest`s
- deterministic `(time, insertion order)` scheduling
- failed scheduled actions remain queued and do not partially mutate state
- replay does not re-run historical scheduler decisions

Known v0 limitation: historical `fork_after` does not reconstruct scheduler queue state because scheduling itself is not event-sourced yet. This belongs with snapshot/branch hardening rather than domain logic.

No LLM and no UI.

## M5 — Behavior Runtime

Status: implemented and passing GitHub CI.

- Behavior trait and registry
- deterministic RuleBehavior / NativeBehavior
- event subscriptions
- FIFO event -> behavior -> action -> event loop
- registration/action ordering guarantees
- causal trigger propagation
- hard action budget for loop protection
- replay remains event-only and never re-runs Behaviors

## M6 — Deterministic Tiny Society vertical slice

Status: implemented and passing GitHub CI.

No LLM.

Target:

- 8–10 residents
- 4 locations
- routine schedules
- work
- simple resource/money flow
- relationship edges
- one forced causal chain that can be inspected and forked

Success criterion: the simulation is coherent without an LLM.

## M7 — AgentRuntime boundary

Status: implemented and passing GitHub CI.

- provider-neutral AgentRuntime trait
- MockAgentRuntime
- PerceptionPolicy
- AgentDecisionEvent
- historical decisions replay without model calls

## M8 — pi_agent_rust adapter

Status: implemented locally as an out-of-process decision adapter, pending GitHub CI.

- `world-pi-rpc` crate; no `pi_agent_rust` crate dependency
- one-shot external `pi --mode rpc` transport
- `--no-tools --no-extensions --no-session` decision-only process boundary
- strict `WORLD_ACTION:<name>` output protocol
- filtered observation + offered action prompt
- fail closed on unknown actions, tool attempts, extension UI, malformed RPC, or failed prompt

M8.1 later: persistent RPC sessions and native World Action tool registration once the Pi RPC/extension surface is proven for dynamic tool injection.

## M9 — GPUI projection shell

- `world-gpui` crate
- Collection
- Inspector
- Timeline
- minimal Semantic Canvas

No Tiny-Society-specific GPUI state in the renderer.

## M10 — Tiny Society product loop

- `While you were away`
- causal `Why?`
- `Fork here`
- resident inspection
- history

## M11 — Architecture canary

Create a tiny detective World without changing `world-core`.

If the kernel needs `if world == society` or detective-specific concepts, fix the abstraction before proceeding.
