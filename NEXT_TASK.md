# Next Coding Task — M4 Clock / Scheduler

Implement the smallest deterministic logical Clock/Scheduler vertical slice.

Requirements:

1. Introduce no domain concepts into `world-core`.
2. A caller can schedule an `ActionRequest` for a logical timestamp.
3. Advancing the World runs all due scheduled actions in deterministic order.
4. Equal-time ordering must be stable and tested.
5. Failed scheduled actions must not corrupt queue/state.
6. Executed Actions still follow Action -> Event -> State; Scheduler may not mutate state directly.
7. Replay must continue to require no scheduler decision re-execution for historical Events.
8. Do not add async, Tokio, GPUI, Pi, serde, SQLite, ECS or LLM dependencies.

Deliver:

- implementation
- deterministic ordering tests
- failure atomicity test
- update `docs/WORLD_IR_v0.1.md`
- update `docs/ROADMAP.md`
- run architecture boundary check and Rust tests
