# Pi Read-only Analyst Bridge

M218 exposes the existing World Machine analyst tool host to Pi without granting Pi direct World, Projection, filesystem, shell, or mutation authority.

## Boundary

The runtime path is:

```text
Pi model/tool loop
  -> integrations/pi/world-machine-analyst.mjs
  -> world-agent-tool-stdio <left.world> <right.world>
  -> world-agent-tool-host
  -> world-agent-tools
  -> world-investigation-local
  -> world-investigation
  -> world-query
```

The left/right archive paths are process configuration. They are supplied when the analyst session starts and are never part of an LLM-visible tool schema.

The Pi extension reads the M214/M215 catalog at session start and dynamically registers only descriptors whose `read_only` field is `true`. Host tool names are normalized for provider tool-name constraints (`world.first-divergence` becomes `world_first_divergence`). Each Pi tool call preserves Pi's `toolCallId` as the host `call_id`, validates the correlated host response, and executes sequentially through one bound JSONL process.

This is separate from `world-pi-rpc`. `world-pi-rpc` remains the provider adapter for in-World `AgentRuntime` decisions. The analyst extension is an external read-only evidence surface and never becomes World mutation authority.

## Build and start

Build the local analyst host:

```bash
cargo build -p world-agent-tool-stdio
```

Start Pi in restricted RPC mode:

```bash
bash scripts/run-pi-analyst.sh left.world right.world --provider <provider> --model <model>
```

The launcher intentionally disables Pi built-in tools, automatic extension discovery, skills, prompt templates, themes, context files, and session persistence. It loads only `integrations/pi/world-machine-analyst.mjs` and replaces the default system prompt with a read-only analyst prompt.

Use `WORLD_MACHINE_ANALYST_PROGRAM` to point at an installed `world-agent-tool-stdio` binary. Use `PI_PROGRAM` to override the `pi` executable.

## Protocol behavior

The bridge keeps the M214 protocol unchanged:

- protocol: `world-machine-readonly-tools`
- version: `1`
- transport: one JSON document per UTF-8 line
- `list-tools` returns the canonical read-only catalog
- `invoke` echoes `call_id` and host tool name on both success and correlated tool failure
- protocol/version/correlation mismatches are bridge failures
- host tool failures are thrown back to Pi as tool failures rather than returned as successful text
- the bridge is single-flight and Pi tools are registered with `executionMode: "sequential"`

The extension parses stdout by byte-delimited LF rather than chunk/string splitting, so UTF-8 characters remain intact across Node stream chunk boundaries.

## Safety properties

The extension does not import filesystem or network modules and does not expose arbitrary command execution. The only child process it starts is the configured `world-agent-tool-stdio` executable with the two archive paths fixed at session startup.

The extension starts with no active Pi tools, rejects any catalog descriptor not marked read-only, then activates only the normalized World Machine analyst tool names. The restricted launcher independently disables Pi built-ins and automatic resource discovery, providing defense in depth.

`cargo test -p world-agent-tool-stdio` runs the Node transport tests through `scripts/check-pi-analyst.sh`, including catalog/invoke reuse, correlated remote errors, protocol/correlation rejection, abort cleanup, source-level authority guards, and launcher flag checks.
