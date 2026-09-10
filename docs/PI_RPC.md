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

## Two choices, not one

A Pack that can reach a local model has two separate uses for it, and they are
configured separately because they cost very different amounts.

| Setting | Values | What it does | What it costs |
| --- | --- | --- | --- |
| `WORLD_MACHINE_POCKET_UNIVERSE_MIND` | `deterministic` (default), `pi` | Decides what the World's inhabitants do | Two requests per period — a week-long catch-up is fifty-six |
| `WORLD_MACHINE_POCKET_UNIVERSE_VOICE` | `none` (default), `pi` | Says what happened, in this World's own words | One request per return, however long the observer was away |
| `WORLD_MACHINE_PI_PROGRAM` | path (default `pi`) | The program both use | — |
| `WORLD_MACHINE_ANTHROPIC_API_KEY` | key | Required by `…_VOICE=api` | — |

`…_VOICE` takes `none` (default), `pi` (a local program), or `api` (a key). The
`api` value without a key is an error rather than a quiet fall back to silence:
somebody asked for a voice and would otherwise never learn why their Worlds
stayed the same.

The `api` voice is the only path on which a World's contents leave the machine.
It sends one request per return to `api.anthropic.com` carrying the facts that
return is about to show and nothing else, asks `claude-opus-5` at `low` effort
with a `max_tokens` ceiling of 2048 — a narration the World would refuse above
one paragraph a line has nothing to gain from a larger one — and gives up after
30 seconds rather than hanging a return. The key is written to `curl`'s
configuration on standard input and never becomes a process argument, because
arguments are readable by every process on the machine; the request body goes to
a file created `0600` and removed when the narration is done.

The app sets `…_VOICE` and `…_PI_PROGRAM` on the Pack processes it launches
when somebody turns **World voice** on in the Analyst settings and has chosen a
Pi program. A host may only add settings named `WORLD_MACHINE_…` to a Pack's
environment; anything else is refused, so a host configures a Pack rather than
reshaping what it inherits.

The mind only ever picks one of two offered actions, so the combination worth
having is **`MIND=deterministic` with `VOICE=pi`**: the simulation stays
instant and the prose is the World's own. Both default to off, so a Pack that
is given nothing behaves exactly as it always has.

## Narration

The voice is asked once per return for every line the return digest is about
to show, numbered, and its answer is matched back by number rather than
trusted to arrive in order. The same locked-down process is used — no tools,
no extensions, no session — and the World's contents go in inside
`<world_data>` as data, never as instructions, with newlines stripped so a
World's own text cannot forge a fact.

Nothing the model returns is trusted. A line that is missing, mis-numbered,
duplicated, empty, over-long, or carrying control characters leaves that entry
on the World's built-in copy, and a model that cannot be reached at all leaves
every entry on it. A World is never worse for having asked.

What comes back is recorded as a `world_narrated` Event, so it is durable,
replay-exact, and never regenerated: replaying a World never runs a model.
