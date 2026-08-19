# Next Coding Task — M221 Rust Analyst-Turn Client

M214–M220 now provide a complete external read-only analyst path: stable evidence tools, a fixed archive-bound host, a restricted Pi extension, a long-lived Pi analyst session, and the provider-neutral World Machine `world-machine-analyst-turns@1` process protocol.

The next missing layer is a Rust client that lets the native product consume analyst turns without knowing anything about Pi RPC, Node event semantics, or provider-normalized tool names.

## Current baseline

- M214–M218 expose read-only World evidence to a restricted Pi tool loop.
- M219 owns Pi prompt acknowledgement, event accumulation, `agent_settled` completion, tool telemetry, and session failure semantics.
- M220 normalizes completed turns into a strict JSONL protocol with the archive pair/model configuration bound only at process startup.
- M220 strips raw Pi event names/details and exposes canonical World Machine tool calls as `{call_id, tool, input, output, is_error}` plus normalized `runtime_errors`.
- `world-pi-rpc` remains decision-only for in-World `AgentRuntime` and is not part of the external analyst product path.

## M221 — Rust analyst-turn client

Add a small provider-neutral Rust boundary for `world-machine-analyst-turns@1` that:

- defines owned typed request/result/error DTOs matching M220 exactly;
- validates protocol name/version and caller request-id correlation;
- preserves final analyst text, canonical tool-call telemetry, and normalized runtime errors;
- distinguishes correlated non-fatal `command` errors from fatal `protocol`, `transport`, and `internal` errors using the M220 `fatal` flag;
- offers a sequential `BufRead + Write` client for deterministic unit tests;
- offers a child-process session that starts the M220 Node turn host with a fixed left/right archive pair and optional provider/model/thinking process configuration;
- keeps one child alive across sequential asks and closes it deterministically;
- treats EOF, invalid JSON, protocol/version mismatch, wrong request id, and fatal remote errors as a poisoned session.

A product caller should be able to ask a question in Rust and receive one completed World Machine analyst turn without importing Pi concepts.

## Boundary rules

M221 must not depend on `world-agent`, `world-core`, `world-projection`, GPUI, provider SDKs, or mutation APIs. It must not parse Pi events, register tools, execute World queries directly, or accept archive paths in per-turn requests. Those authorities stay below the M220 process boundary.

Prefer a dedicated small crate if that keeps the ownership boundary clearer than placing the client in an existing tool-host crate. If a new crate is added, update the generated lockfile normally and let the full workspace/macOS gate validate the membership change.

## Validation

- DTO round-trip and strict protocol/version/correlation tests;
- in-memory sequential client tests for result and each error class;
- assertions that raw Pi event/provider fields cannot deserialize into or appear in the public M221 DTOs;
- real subprocess test using the M220 fake-Pi path, proving two Rust asks reuse one turn-host/Pi session;
- child EOF/non-zero-exit/shutdown cleanup tests;
- architecture guard preventing Projection/Core/AgentRuntime/provider coupling;
- existing M214–M220 tests remain unchanged and green;
- full workspace Clippy/tests, external Pack conformance, and macOS/GPUI gate if workspace membership changes.

## Non-goals

No GPUI/UI integration yet, no concurrent/multiplexed asks, no provider abstraction, no protocol v2, no HTTP/MCP/WebSocket server, and no mutation tools.
