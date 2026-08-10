# Licensing architecture

World Machine's original code is licensed under Apache-2.0.

The license of the World Machine kernel is intentionally independent from the licenses of optional agent runtimes and rendering/runtime integrations.

## Dependency policy

- `world-core` and the future `world-agent` protocol layer must remain free of dependencies that add field-of-use, party, or deployment restrictions beyond the project's Apache-2.0 terms.
- GPUI is currently an Apache-2.0 crate and may be used by the future `world-gpui` adapter subject to normal third-party notice obligations.
- `pi_agent_rust` currently uses an MIT-derived license with an additional OpenAI/Anthropic rider. It must not become a dependency of `world-core` or define the World IR.
- The current Pi integration is an optional out-of-process RPC adapter (`world-pi-rpc`). It launches an externally installed Pi binary and does not link or redistribute `pi_agent_rust`. An in-process integration, if ever added, must remain optional and carry the upstream license/rider notices required by that dependency.
- A packaged application must include notices for every dependency actually distributed in that package. The Apache-2.0 license of World Machine does not replace third-party license terms.

## Architecture consequence

The agent boundary is provider-neutral:

```text
world-core
    |
world-agent / World Agent Protocol
    |
    +-- world-pi-rpc -> external pi runtime
    +-- local runtime
    +-- other runtime adapters
```

Agent implementations are replaceable adapters. World truth, World IR, replay, branching, and projection semantics must never depend on one model provider or one agent runtime.

## Dependency review

Before adding or updating a third-party crate:

1. check the crate's declared license rather than assuming the parent repository license applies;
2. check transitive dependencies for incompatible or non-standard restrictions;
3. keep restricted runtimes behind optional adapter boundaries;
4. update `THIRD_PARTY_LICENSES.md` when a dependency becomes part of a distributed artifact.

Automated dependency-license checking should be added when the repository begins carrying third-party runtime dependencies. Until then, the kernel intentionally has no third-party Rust dependencies.
