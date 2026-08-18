# Next Coding Task — M214 Read-Only Analyst Tool Host

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
- focused fmt/tests/Clippy plus full workspace CI, external Pack conformance, and macOS validation because workspace membership changes.

## Non-goals

No provider-specific adapter yet, no in-world tool use, no network server, no mutable tools, no server-side investigation cursor/session, and no evidence-query protocol v2.
