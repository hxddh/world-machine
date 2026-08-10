# Next Coding Task — M5 Behavior Runtime

Implement the smallest deterministic Behavior Runtime vertical slice.

Requirements:

1. Introduce no domain concepts into `world-core`.
2. Define a small Behavior abstraction that can observe selected Event kinds and propose `ActionRequest`s.
3. Implement deterministic `RuleBehavior` and `NativeBehavior` paths first. No LLM or Pi integration.
4. Behavior output must always re-enter the World through `ActionRegistry`; Behaviors may not mutate WorldState directly.
5. Define deterministic ordering when multiple Behaviors react to the same Event.
6. Add loop/recursion protection so an Event -> Behavior -> Action -> Event chain cannot run unbounded.
7. Recorded Events remain sufficient for replay; replay must not re-run Behaviors.
8. Keep the API compatible with a future `AgentBehavior` implemented by a provider-neutral AgentRuntime.
9. Do not add async, Tokio, GPUI, Pi, serde, SQLite, ECS, or model dependencies yet.

Deliver:

- Behavior registry/runtime implementation
- deterministic ordering tests
- loop-budget test
- no-direct-mutation architecture test
- update `docs/WORLD_IR_v0.1.md`
- update `docs/ROADMAP.md`
- run architecture boundary check and Rust tests
