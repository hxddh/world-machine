# Next Coding Task — M6 Deterministic Tiny Society Vertical Slice

Build the first real World on top of the generic kernel without adding Tiny Society concepts to `world-core`.

Requirements:

1. Add reusable Systems / World Pack structure outside `world-core`.
2. Seed 8–10 residents and roughly 4 locations using generic Entity / Relation data.
3. Implement deterministic routine schedules, work, simple money/resource flow, and relationship edges using Actions + Behaviors + Scheduler.
4. Implement one intentionally testable causal chain:

   storm -> boat damage -> income loss -> loan request -> temporary work -> missed shift -> order loss -> dismissal

5. Every meaningful transition must be a semantic Event with causal provenance.
6. No LLM, Pi, GPUI, SQLite, serde, ECS, async, or networking yet.
7. The simulation must be deterministic from a fixed seed/baseline.
8. Add an integration test proving the full causal chain, deterministic replay, and a fork before dismissal.
9. `world-core` must remain unchanged unless a concrete generic runtime defect is discovered. If a kernel change is required, document why it is not Society-specific.

Deliver:

- first `systems/` crates/modules as justified by the vertical slice
- first `worlds/tiny-society` reference World
- CLI/demo output of the causal history
- deterministic integration tests
- update architecture/IR/roadmap docs
- GitHub CI green
