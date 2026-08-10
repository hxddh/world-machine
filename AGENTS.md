# Coding Agent Constitution

You are implementing **World Machine**. The first user-facing product is Tiny Society, but Tiny Society is not the architecture.

## Invariants

1. `world-core` must not depend on Tiny Society concepts, GPUI, `pi_agent_rust`, or any specific world/system package.
2. LLM output is never authoritative world state.
3. Agents/users/rules propose Actions; they do not directly mutate authoritative world state.
4. Meaningful changes flow through validated Action -> Event -> State transitions.
5. Events preserve causal provenance via `caused_by`.
6. Historical replay must not require re-running an LLM or other decision maker.
7. GPUI state is UI state, not World truth.
8. `pi_agent_rust` will be an interchangeable `AgentRuntime` adapter, not a kernel dependency. Prefer an optional out-of-process RPC adapter; do not let restricted/non-standard runtime licenses flow into `world-core` or the provider-neutral agent protocol.
9. Prefer vertical slices and deterministic tests over speculative abstraction.
10. Generality must emerge from multiple real worlds, not from guessed framework requirements.

## Architecture guardrail

Before finishing a kernel change, run:

```bash
./scripts/check-boundaries.sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

If a product requirement appears to require a domain-specific kernel concept, place it in a System or World Pack instead.
