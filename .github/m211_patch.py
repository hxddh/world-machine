from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    if old not in text:
        raise SystemExit(f"missing patch anchor: {label}")
    return text.replace(old, new, 1)


workspace = Path("Cargo.toml")
text = workspace.read_text()
text = replace_once(
    text,
    '  "crates/world-investigation",\n',
    '  "crates/world-investigation",\n  "crates/world-agent-tools",\n',
    "workspace member",
)
workspace.write_text(text)

boundaries = Path("scripts/check-boundaries.sh")
text = boundaries.read_text()
text = replace_once(
    text,
    'INVESTIGATION="$ROOT/crates/world-investigation"\n',
    'INVESTIGATION="$ROOT/crates/world-investigation"\nAGENT_TOOLS="$ROOT/crates/world-agent-tools"\n',
    "agent tools path",
)
text = replace_once(
    text,
    'investigation_forbidden=("world_projection" "world-projection" "ProjectionSnapshot" "world_core" "world-core" "world_agent" "world-agent" "gpui" "pi_agent" "openai" "anthropic")\n',
    'investigation_forbidden=("world_projection" "world-projection" "ProjectionSnapshot" "world_core" "world-core" "world_agent" "world-agent" "gpui" "pi_agent" "openai" "anthropic")\nagent_tools_forbidden=("world_projection" "world-projection" "ProjectionSnapshot" "world_core" "world-core" "../world-agent" "world_agent::" "gpui" "pi_agent" "openai" "anthropic")\n',
    "agent tools forbidden tokens",
)
text = replace_once(
    text,
    '''if [[ -d "$INVESTIGATION" ]]; then\n  for token in "${investigation_forbidden[@]}"; do\n    if grep -Rni --exclude-dir=target -i -- "$token" "$INVESTIGATION" >/tmp/world-machine-investigation-boundary-check 2>/dev/null; then\n      echo "Boundary violation: '$token' found in query-only world-investigation adapter:"\n      cat /tmp/world-machine-investigation-boundary-check\n      failed=1\n    fi\n  done\nfi\n\n''',
    '''if [[ -d "$INVESTIGATION" ]]; then\n  for token in "${investigation_forbidden[@]}"; do\n    if grep -Rni --exclude-dir=target -i -- "$token" "$INVESTIGATION" >/tmp/world-machine-investigation-boundary-check 2>/dev/null; then\n      echo "Boundary violation: '$token' found in query-only world-investigation adapter:"\n      cat /tmp/world-machine-investigation-boundary-check\n      failed=1\n    fi\n  done\nfi\n\nif [[ -d "$AGENT_TOOLS" ]]; then\n  for token in "${agent_tools_forbidden[@]}"; do\n    if grep -Rni --exclude-dir=target -i -- "$token" "$AGENT_TOOLS" >/tmp/world-machine-agent-tools-boundary-check 2>/dev/null; then\n      echo "Boundary violation: '$token' found in read-only world-agent-tools boundary:"\n      cat /tmp/world-machine-agent-tools-boundary-check\n      failed=1\n    fi\n  done\nfi\n\n''',
    "agent tools boundary scan",
)
boundaries.write_text(text)

Path("NEXT_TASK.md").write_text('''# Next Coding Task — M211 Read-Only Agent Tool Boundary

Expose the proven M209/M210 progressive investigation capability as a provider-neutral host-side agent tool without weakening the in-world `AgentRuntime` perception boundary.

## Current baseline

M209 provides `world-investigation`, a scheduler that depends only on public `world-query` DTOs. M210 proves a concrete local adapter can own `ProjectionSnapshot` while delegating all progressive search semantics through `ComparisonQueryExecutor`. The remaining gap is a reusable typed tool surface for external agent hosts.

## M211 — `world-agent-tools`

Add a new `world-agent-tools` crate with a read-only `world.first-divergence` tool.

The crate exposes:

- a stable `ReadOnlyToolDescriptor` with `read_only = true`;
- serializable `FirstDivergenceToolInput { root, direction, window_depth, max_depth }`;
- serializable `FirstDivergenceToolOutput` carrying the absolute M209 result;
- `FirstDivergenceTool<E>` parameterized only by the existing `ComparisonQueryExecutor` authority boundary;
- `invoke`, which maps the typed input to `investigate_first_divergence` and does not reimplement continuation replay, convergence, depth accounting, or trace composition.

## Boundary rules

- `world-agent-tools` production dependencies are limited to `serde`, `world-investigation`, and `world-query`.
- It must not depend on or name Projection/Core truth, `world-agent` / in-world `AgentRuntime`, GPUI, model-provider SDKs, or transport stacks.
- The tool executor is supplied by a host adapter. The tool itself has no archive, snapshot, filesystem, network, mutation, or model authority.
- This does not add tools to the in-world `AgentRuntime`; `AgentObservation` remains the only perception surface there.

## Compatibility

No change to evidence-query protocol v1, investigation envelope v1, CLI commands, Pack APIs, persistence formats, or AgentRuntime interfaces.

## Validation

- stable read-only descriptor and typed JSON input shape;
- progressive multi-window invocation proves reuse of M209 and preserves original-root witness traces;
- serializable output contains no executor or world-internal state;
- hard dependency/content boundary checks;
- `cargo fmt --all -- --check`;
- `cargo test -p world-agent-tools` and `cargo test -p world-investigation`;
- focused Clippy with warnings denied;
- full workspace CI, external Pack conformance, and macOS/GPUI validation when lockfile/workspace paths trigger it.

## Non-goals

No provider-specific SDK integration, no MCP/HTTP/WebSocket adapter, no in-world AgentRuntime query access, no `ProjectionSnapshot` exposure, no mutation tools, no server cursor/session, and no protocol v2.
''')
