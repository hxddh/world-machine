# Licensing architecture

World Machine's original code is licensed under Apache-2.0.

The license of the World Machine kernel is intentionally independent from the licenses of optional agent runtimes and rendering/runtime integrations.

## Dependency policy

- `world-core` and `world-agent` must remain free of dependencies that add field-of-use, party, or deployment restrictions beyond the project's Apache-2.0 terms.
- `world-projection` is an Apache-2.0 headless read-model layer and must remain independent from GPUI, Tiny Society, Pi, and model providers.
- GPUI is Apache-2.0 and is isolated in the optional `world-gpui` renderer crate. Because `gpui_platform` is currently consumed from the Zed workspace rather than crates.io, GPUI and `gpui_platform` are pinned to the same Zed Git revision to avoid mixed-source type/version drift.
- The current GPUI pin is Zed revision `4e8057d74db3570b3bd419ff296eb84c35b3a5a3`. Updating it requires re-running the macOS renderer compile job and re-checking upstream crate licenses.
- `pi_agent_rust` currently uses an MIT-derived license with an additional OpenAI/Anthropic rider. It must not become a dependency of `world-core`, `world-agent`, or define the World IR.
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

The renderer boundary is projection-driven:

```text
world-core / world packs
        |
world-projection
        |
   world-gpui
```

Agent implementations and renderers are replaceable adapters. World truth, World IR, replay, branching, and projection semantics must never depend on one model provider, one agent runtime, or one UI framework.

## Dependency review

Before adding or updating a third-party crate:

1. check the crate's declared license rather than assuming the parent repository license applies;
2. check transitive dependencies for incompatible or non-standard restrictions;
3. keep restricted runtimes behind optional adapter boundaries;
4. pin coupled Git-workspace crates to the same upstream revision;
5. update `THIRD_PARTY_LICENSES.md` when a dependency becomes part of a distributed artifact.

Automated dependency-license checking should be added as the distributed dependency surface grows. The semantic kernel remains deliberately small and third-party-light.
