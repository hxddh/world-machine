# Next Coding Task — M215 Local Stdio Analyst Process

Turn the M214 transport-neutral analyst host into a real long-lived local process without introducing provider SDKs or network authority, and establish one reusable concrete boundary for local archive comparison.

## Current baseline

M209 owns progressive investigation semantics. M210 exposes them through a CLI-local comparison executor. M211–M213 define read-only tools, JSON dispatch, and deterministic registry semantics. M214 adds a strict external analyst host while keeping `world-pi-rpc` decision-only.

## M215 — reusable local executor + JSON-lines stdio

Add `world-investigation-local` as the explicit authority-bearing companion to query-only `world-investigation`:

- own the left/right `ProjectionSnapshot` values;
- load two `.world` archives through built-in Pack restoration;
- implement M209 `ComparisonQueryExecutor` by delegating to existing `world-query` semantics;
- preserve typed side-specific read/parse/open errors;
- contain no Agent/provider/network/UI dependencies.

Add `world-agent-tool-stdio <left.world> <right.world>`:

- bind the archive pair once at process startup, so tool calls cannot choose arbitrary filesystem paths;
- register `world.first-divergence` in the M213 registry;
- read one M214 JSON request per non-empty stdin line;
- write and flush exactly one M214 JSON response line per valid host request;
- keep correlated tool-level errors in-band and continue serving later requests;
- treat malformed JSON or malformed host request envelopes as process-level failures.

After the shared local executor and stdio process are green, refactor M210 `world-cli evidence-investigate-compare` to reuse `world-investigation-local` instead of retaining its private snapshot executor.

## Authority boundary

The concrete local archive/Projection authority lives only in `world-investigation-local`. The stdio process depends on that adapter and the M214/M213 layers but does not directly depend on Projection/Core, in-world `world-agent`/`world-pi-rpc`, provider SDKs, or network/server stacks.

Invocation remains:

`stdio framing -> M214 host -> M213 registry -> M212 JSON tool -> M211 typed tool -> M209 investigation -> world-investigation-local -> world-query`

## Validation

- local adapter opens real built-in archives and preserves left/right failure attribution;
- one stdio session can list tools then invoke a real first divergence;
- unknown tool returns correlated error and does not terminate the session;
- malformed JSON is a non-zero process-level failure;
- crate-level authority guards for local executor and stdio leaf;
- M210 CLI regression remains green after the follow-up refactor;
- fmt, boundaries, focused tests/Clippy, full workspace CI, external Pack conformance, and macOS/GPUI/.app validation because workspace/lockfile changes.

## Non-goals

No OpenAI/Anthropic/Pi adapter yet, no MCP/HTTP/WebSocket server, no mutable tools, no in-world AgentRuntime tool injection, no arbitrary archive paths in tool input, and no evidence-query protocol v2.
