# Next Coding Task — M227 Installed Analyst Runtime Probe

M214–M226 now provide a complete installed-app path from read-only World evidence through a restricted long-lived Pi analyst, stable World Machine JSONL turn protocol, strict Rust client, immutable desktop snapshots, packaged runtime validation, installed readiness, and persistent Node/Pi executable paths for Finder-launched use.

## M227 — functional startup probe

M225/M226 currently prove that the configured runtime files exist and are executable. That is necessary but not sufficient: an arbitrary executable can satisfy X_OK, an incompatible Pi binary can reject the restricted RPC flags, and an installed runtime can still fail only after the user presses Start analysis.

Add the smallest no-model startup handshake that proves the exact configured chain can start before a desktop analyst session becomes Ready.

### Required path

`DesktopAnalystSession::start` must keep the existing M222 order:

1. capture the two immutable raw archive snapshots;
2. spawn the configured M220 turn host through Node;
3. on that **same long-lived process**, issue a startup `probe`;
4. M220 asks the restricted Pi RPC process for `get_state` and verifies the post-catalog extension readiness marker through `get_commands`, without sending a prompt;
5. only a correlated successful probe may expose `DesktopAnalystState::Ready`;
6. any probe failure closes the process and releases the snapshots.

Do not launch a second probe-only Pi process and do not make a provider/model request.

### Protocol

Keep `world-machine-analyst-turns@1`. Add an additive strict request/response pair:

- request: `{ "id": "...", "op": "probe", "timeout_ms": ... }`;
- success: `{ "protocol": "world-machine-analyst-turns", "version": 1, "type": "ready", "id": "..." }`.

The ready response carries no Pi state, model/provider details, tool names, archive paths, or raw runtime events. Existing `ask -> result/error` semantics remain unchanged.

### Authority and lifecycle

- probe is read-only and must not dispatch a model prompt;
- archive selection remains fixed at process startup;
- restricted launcher flags and M218 tool authority are unchanged;
- Pi/provider internals remain below M220/M221;
- probe shares the same single-flight runtime as later asks;
- probe timeout/protocol/transport contamination fails the startup closed;
- a command-level probe rejection also rejects startup and the desktop layer closes that process;
- GPUI continues to own no Node/Pi/process code and reuses its existing Starting/error/Recheck surface.

### Validation

Required gates:

- Pi RPC probe sends `get_state`, is correlated, and does not consume prompt request numbering;
- M220 emits only provider-neutral `ready` on successful probe;
- probe request shape is strict and cannot carry prompt/archive/tool fields;
- Rust client accepts `ready`, rejects mismatched result/correlation/protocol/version, and preserves poison semantics;
- `AnalystTurnProcess` tears down on fatal probe contamination;
- desktop startup invokes probe before Ready;
- probe failure shuts the process and cleans immutable snapshots;
- first real ask still reuses that same process;
- existing M219/M220 ask behavior stays green;
- M224 packaged runtime smoke stays green;
- Linux boundary/fmt/Clippy/workspace tests stay green;
- full macOS GPUI/desktop tests and World Machine.app build/validate/smoke/archive/upload stay green.

## Non-goals

No model call during readiness, no provider/API-key settings, no protocol v2, no token streaming UI, no second analyst process, no mutation tools, no concurrent asks, no broad Preferences framework, and no loosening of M222 immutable-snapshot or M223 cancellation boundaries.
