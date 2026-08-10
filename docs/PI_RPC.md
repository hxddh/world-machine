# Pi RPC adapter contract

`world-pi-rpc` is the first concrete `AgentRuntime` adapter. It deliberately uses an **external** Pi process rather than linking `pi_agent_rust` into World Machine.

The implementation was checked against `Dicklesworthstone/pi_agent_rust` at upstream commit `44ddf80ff1fccbeb08501c1e8eaa69f2b5dd5d92` (August 2026).

## Transport

Pi documents headless RPC as JSON Lines over stdin/stdout:

```text
pi --mode rpc
```

World Machine currently launches one fresh process per decision and sends exactly one `prompt` request. Stdin is then closed. Pi's current implementation drains an in-flight turn before exiting after stdin EOF.

## Decision-only process

The adapter starts Pi with:

```text
--mode rpc
--no-tools
--no-extensions
--no-skills
--no-prompt-templates
--no-themes
--no-session
--hide-cwd-in-prompt
```

The intent is to use Pi only as a reasoning/choice backend. It must not gain ambient filesystem, shell, extension, or session side effects from the World Machine decision path.

The adapter also fails closed if the RPC stream contains a tool-execution or extension-UI event despite those flags.

## World action protocol

Plain Pi RPC does not currently expose a proven dynamic World Action registration command. M8 therefore does not pretend that World Actions are native Pi tools.

Instead, the adapter sends only the already-filtered `AgentObservation` plus concrete `AvailableAction` names/descriptions. Pi must return exactly:

```text
WORLD_ACTION:<action-name>
```

No prose or Markdown is accepted. The returned name must match an offered action, and the provider-neutral `AgentExecutor` validates the choice again before recording the decision and executing the actual World Action.

## Output compatibility

Pi documentation and current examples expose two text-stream shapes across revisions. The adapter accepts both:

- `message_update.assistantMessageEvent.type = text_delta` with `delta`
- direct `type = text_delta` with `delta` (and the older `data.text` fallback)

A successful `response` for the `prompt` command is required before the decision text is accepted.

## Known v0 limits

- one process per decision; no warm/persistent Pi session yet;
- no native World Action tool injection yet;
- no Pi binary is downloaded, linked, or redistributed by `world-pi-rpc`;
- model/provider configuration is inherited from the user's external Pi installation;
- the process has a 120-second default decision timeout.

M8.1 may add a persistent transport and native action/tool registration only if those changes preserve the same provider-neutral, capability, replay, and licensing boundaries.
