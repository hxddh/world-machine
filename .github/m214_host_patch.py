from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    if old not in text:
        raise SystemExit(f"missing marker for {label}")
    return text.replace(old, new, 1)


boundary_path = Path("scripts/check-boundaries.sh")
text = boundary_path.read_text()
text = replace_once(
    text,
    'AGENT_TOOLS="$ROOT/crates/world-agent-tools"\nHOST="$ROOT/crates/world-host"',
    'AGENT_TOOLS="$ROOT/crates/world-agent-tools"\nAGENT_TOOL_HOST="$ROOT/crates/world-agent-tool-host"\nHOST="$ROOT/crates/world-host"',
    "agent tool host directory",
)
text = replace_once(
    text,
    'agent_tools_forbidden=("world_projection" "world-projection" "ProjectionSnapshot" "world_core" "world-core" "../world-agent" "world_agent::" "gpui" "pi_agent" "openai" "anthropic")\nhost_forbidden=',
    'agent_tools_forbidden=("world_projection" "world-projection" "ProjectionSnapshot" "world_core" "world-core" "../world-agent" "world_agent::" "gpui" "pi_agent" "openai" "anthropic")\nagent_tool_host_forbidden=("world_projection" "world-projection" "ProjectionSnapshot" "world_core" "world-core" "../world-agent\\\"" "world_agent::" "AgentRuntime" "AgentObservation" "world-pi-rpc" "PiRpc" "pi_rpc" "gpui" "pi_agent" "openai" "anthropic" "reqwest" "hyper" "axum" "tokio" "websocket")\nhost_forbidden=',
    "agent tool host forbidden tokens",
)
agent_tools_block = '''if [[ -d "$AGENT_TOOLS" ]]; then
  for token in "${agent_tools_forbidden[@]}"; do
    if grep -Rni --exclude-dir=target -i -- "$token" "$AGENT_TOOLS" >/tmp/world-machine-agent-tools-boundary-check 2>/dev/null; then
      echo "Boundary violation: '$token' found in read-only world-agent-tools boundary:"
      cat /tmp/world-machine-agent-tools-boundary-check
      failed=1
    fi
  done
fi
'''
host_block = agent_tools_block + '''
if [[ -d "$AGENT_TOOL_HOST" ]]; then
  for token in "${agent_tool_host_forbidden[@]}"; do
    if grep -Rni --exclude-dir=target -i -- "$token" "$AGENT_TOOL_HOST" >/tmp/world-machine-agent-tool-host-boundary-check 2>/dev/null; then
      echo "Boundary violation: '$token' found in external read-only world-agent-tool-host:"
      cat /tmp/world-machine-agent-tool-host-boundary-check
      failed=1
    fi
  done
fi
'''
text = replace_once(text, agent_tools_block, host_block, "agent tool host check")
boundary_path.write_text(text)

Path("NEXT_TASK.md").write_text(r'''# Next Coding Task — M214 Read-Only Analyst Tool Host

Expose the M213 provider-neutral registry through a transport-neutral, correlated JSON host boundary for external analyst agents, while keeping the in-world `AgentRuntime` decision-only and perception-scoped.

## Current baseline

M209 owns progressive investigation, M210 adds the local CLI executor adapter, M211/M212 define typed and JSON read-only tools, and M213 adds deterministic multi-tool registration and dispatch. `world-pi-rpc` remains an in-world decision adapter and explicitly rejects tool execution events, so investigation tools must not be injected there.

## M214 — `world-agent-tool-host`

Add a separate host crate rather than expanding the core tool crate. Production dependencies are limited to `serde`, `serde_json`, and `world-agent-tools`.

Requests are strict provider-neutral JSON:

- `{"op":"list-tools"}`
- `{"op":"invoke","call_id":"...","tool":"...","input":{...}}`

Responses carry protocol `world-machine-readonly-tools`, version `1`, and one of:

- deterministic `catalog` from frozen M213 descriptors;
- correlated `result` echoing `call_id` and tool name;
- correlated `error` with stable kind `unknown-tool`, `invalid-input`, `investigation`, or `output-serialization`.

## Error boundary

M213 keeps typed `JsonToolDispatchError<E>` internally. M214 erases that typed error only at the external JSON host boundary, preserving a stable error kind plus diagnostic message. Malformed host requests are protocol failures from `handle_json` and never reach registry dispatch.

## Boundary rules

- Do not connect this host to in-world `world-pi-rpc` / `AgentRuntime`; that path remains decision-only and continues rejecting tool execution.
- `world-agent-tool-host` may depend on `world-agent-tools`, but not directly on Projection/Core truth, `world-agent`, GPUI, model providers, or network/server stacks.
- The host owns only a read-only registry supplied by its caller. It gains no archive, filesystem, network, model, or World mutation authority.
- No OpenAI, Anthropic, Pi, MCP, HTTP, or WebSocket SDK/protocol types enter the host contract.
- Tool invocation remains M214 host -> M213 registry -> M212 JSON tool -> M211 typed tool -> M209 investigation.

## Validation

- stable protocol/version and deterministic catalog;
- correlated call id/tool name on result and error;
- real first-divergence invocation preserves witness trace;
- unknown tool and invalid input map to distinct stable error kinds;
- unknown host request fields fail before dispatch;
- hard dependency/content guard for the host crate;
- focused fmt/tests/Clippy plus full workspace CI, external Pack conformance, and macOS validation when workspace/lockfile paths trigger it.

## Non-goals

No provider-specific adapter yet, no in-world tool use, no network server, no mutable tools, no server-side investigation cursor/session, and no evidence-query protocol v2.
''')
