# Next Coding Task — M7 AgentRuntime Boundary

Introduce provider-neutral agent intelligence without coupling World Machine to Pi, OpenAI, Anthropic, or any other runtime.

Requirements:

1. Add a `world-agent` crate outside `world-core`.
2. Define a small provider-neutral `AgentRuntime` interface for a decision over a filtered World observation and a set of world-valid Actions.
3. Define `AgentObservation`, `AvailableAction`, and `AgentDecision` data structures without model/provider-specific fields.
4. Add a `PerceptionPolicy` abstraction. The default path must never expose global World state implicitly.
5. Implement a deterministic `MockAgentRuntime` for tests; do not integrate Pi yet.
6. Agent decisions that affect the World must be represented as recorded semantic Events or otherwise carry sufficient provenance so replay never calls the AgentRuntime again.
7. Agent output may only select/propose an Action; it may not mutate `WorldState` directly or invent consequences.
8. Keep `world-core` independent of `world-agent`; integrate through an adapter/orchestration layer if needed.
9. No GPUI, Pi SDK, model SDK, async runtime, networking, SQLite, or vector database yet.
10. Add tests for perception filtering, deterministic mock decisions, invalid-action rejection, and replay without runtime calls.

Deliver:

- `crates/world-agent`
- provider-neutral AgentRuntime protocol/types
- MockAgentRuntime
- PerceptionPolicy
- one Tiny Society decision point exercised through the mock runtime without adding agent concepts to `world-core`
- architecture/license boundary tests updated as needed
- docs/roadmap updated
- GitHub CI green
