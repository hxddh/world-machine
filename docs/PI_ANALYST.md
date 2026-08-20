# Pi Read-only Analyst Bridge

M218–M220 provide a complete external read-only analyst path without granting Pi direct World, Projection, filesystem, shell, or mutation authority.

## Runtime layers

```text
Product / future Rust client
  -> world-machine-analyst-turns@1
  -> integrations/pi/world-machine-analyst-turn-host.mjs   (M220)
  -> integrations/pi/world-machine-analyst-rpc.mjs         (M219)
  -> restricted Pi RPC process + analyst extension          (M218)
  -> world-agent-tool-stdio <left.world> <right.world>      (M215)
  -> world-agent-tool-host
  -> world-agent-tools
  -> world-investigation-local
  -> world-investigation
  -> world-query
```

The archive pair, provider, model, and thinking level are process configuration. Per-turn requests cannot replace archive paths or acquire new tools.

This path is deliberately separate from `world-pi-rpc`. `world-pi-rpc` remains the provider adapter for in-World `AgentRuntime` decisions. The analyst stack is external, read-only evidence analysis and never becomes World mutation authority.

## Restricted Pi layer

Build the local read-only tool host:

```bash
cargo build -p world-agent-tool-stdio
```

The restricted launcher is:

```bash
bash scripts/run-pi-analyst.sh left.world right.world --provider <provider> --model <model>
```

It disables Pi built-in tools, automatic extension discovery, skills, prompt templates, themes, context files, and session persistence. It loads only `integrations/pi/world-machine-analyst.mjs` and supplies a read-only analyst system prompt.

The extension reads the canonical M214/M215 tool catalog at session start, rejects descriptors not marked `read_only`, and dynamically registers only the returned tools. Host names are normalized for provider constraints (`world.first-divergence` becomes `world_first_divergence`). Pi's `toolCallId` is preserved as the host `call_id`.

## M219 long-lived analyst session

`PiAnalystRpcSession` keeps one restricted Pi process alive across sequential prompts. It is intentionally single-flight because Pi's event stream is session-ordered rather than request-multiplexed.

A turn:

- sends a correlated prompt request id;
- validates the immediate `prompt` acknowledgement;
- accepts the restricted analyst tool events;
- keeps tool-call telemetry correlated by `toolCallId`;
- waits for `agent_settled`, not the first `agent_end`;
- returns final assistant text plus internal tool/runtime telemetry.

Prompt rejection is a command error and may leave the session reusable. Protocol contamination, timeout, abort, EOF, or unexpected process exit terminate the session so stale events cannot leak into a later turn.

## M220 stable analyst-turn protocol

Higher-level product code must not consume Pi events or provider-normalized tool names. M220 is the normalization boundary and exposes one World Machine-owned JSONL protocol:

- protocol: `world-machine-analyst-turns`
- version: `1`
- one UTF-8 JSON request/response per line
- process startup binds the archive pair and model configuration

Request:

```json
{"id":"ask-1","op":"ask","prompt":"Where do these histories first diverge?","timeout_ms":120000}
```

Successful response:

```json
{
  "protocol":"world-machine-analyst-turns",
  "version":1,
  "type":"result",
  "id":"ask-1",
  "turn":{
    "request_id":"world-analyst-1",
    "text":"...",
    "tool_calls":[
      {
        "call_id":"tool-1",
        "tool":"world.first-divergence",
        "input":{},
        "output":{},
        "is_error":false
      }
    ],
    "runtime_errors":[]
  }
}
```

M220 deliberately removes raw Pi event names, provider-safe tool names, provider result wrappers, and raw Pi error details. A tool call uses the canonical World Machine tool name and canonical tool output whenever the M218 result metadata supplies them.

Correlated error response:

```json
{
  "protocol":"world-machine-analyst-turns",
  "version":1,
  "type":"error",
  "id":"ask-1",
  "error":{
    "kind":"command",
    "fatal":false,
    "message":"..."
  }
}
```

`command` rejection is non-fatal and later asks may reuse the session. `protocol`, `transport`, and `internal` errors are fatal; the contaminated turn-host process emits the correlated error and terminates.

Per-turn request fields are strict. In particular, archive paths are not valid request fields.

M227 adds an additive protocol-v1 startup probe used by the desktop before exposing a session as Ready. The probe sends no model prompt: M220 asks the already-started restricted Pi RPC process for `get_state`, verifies the post-catalog extension readiness marker through `get_commands`, and returns only `{type:"ready", id}`. Raw Pi state, provider/model details, and tool names are not exposed. The same long-lived Pi process is then reused by later `ask` requests.

## Safety properties

Production analyst integration modules import no filesystem or network API and expose no arbitrary command execution. The Pi extension starts with no active tools and activates only catalog-derived read-only World Machine tools. The restricted launcher independently disables Pi built-ins and automatic resource discovery.

M219 and M220 parse JSONL using LF byte framing rather than Node `readline`, preserving UTF-8 data across stream chunk boundaries.

`cargo test -p world-agent-tool-stdio` reaches `scripts/check-pi-analyst.sh`, which runs the Node transport/session/turn-host tests plus source-level authority and launcher checks. The M220 process regression uses a fake Pi executable but crosses the actual restricted launcher, proves two asks reuse one Pi child, and verifies provider-only event/result details do not escape the M220 protocol.
