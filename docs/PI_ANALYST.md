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
- returns final assistant text, tool calls, extension errors, and observed event types.

Prompt rejection is a command error and may leave the session reusable. Protocol contamination, timeout, abort, EOF, or unexpected process exit terminate the session so stale events cannot leak into a later turn.

## M220 stable analyst-turn protocol

Higher-level product code should not consume Pi events directly. M220 exposes one World Machine-owned JSONL protocol:

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
    "tool_calls":[],
    "extension_errors":[],
    "events":[]
  }
}
```

Pi command rejection is returned as a correlated non-fatal `error` envelope. Pi protocol/transport failures are returned as correlated fatal errors and the contaminated turn-host process terminates.

Per-turn request fields are strict. In particular, archive paths are not valid request fields.

## Safety properties

Production analyst integration modules import no filesystem or network API and expose no arbitrary command execution. The Pi extension starts with no active tools and activates only catalog-derived read-only World Machine tools. The restricted launcher independently disables Pi built-ins and automatic resource discovery.

M219 and M220 parse JSONL using LF byte framing rather than Node `readline`, preserving UTF-8 data across stream chunk boundaries.

`cargo test -p world-agent-tool-stdio` reaches `scripts/check-pi-analyst.sh`, which runs the Node transport/session/turn-host tests plus source-level authority and launcher checks. The M220 process regression uses a fake Pi executable but crosses the actual restricted launcher and proves two asks reuse one Pi child without model credentials.
