# Next Coding Task — M225 Installed Analyst Runtime Readiness

M214–M224 now form a complete first product path for read-only World analysis: evidence tools, restricted Pi analyst execution, a stable provider-neutral JSONL boundary, a strict Rust client, a desktop-owned session controller, a native GPUI analyst panel, and a macOS packaged-runtime smoke that proves the built app can run two real evidence-backed turns from outside the source checkout.

## Current baseline

- M220 defines `world-machine-analyst-turns@1` and strips raw Pi/provider events from the product boundary.
- M221/M221.1 provide strict correlation, request validation, poisoning semantics, and process lifecycle handling.
- M222 binds one desktop session to two immutable raw archive snapshots and owns cleanup/cancellation below GPUI.
- M223 adds the live `Analyze saved Worlds…` surface and packages the analyst host, launcher, extension/client, and `world-agent-tool-stdio` under `World Machine.app/Contents/Resources/Analyst Runtime`.
- M224 proves that packaged substrate end to end: from a non-repository working directory, the packaged turn host launches the packaged restricted analyst path, the packaged extension performs a real read-only evidence call through the packaged Rust tool host, and two sequential turns reuse one long-lived Pi process.
- Node and Pi intentionally remain external runtime dependencies. The desktop currently resolves Node as `WORLD_MACHINE_NODE_PROGRAM` or bare `node`, resolves Pi from `PI_PROGRAM` or the launcher default, and generally discovers a missing executable only when session startup fails.

## M225 — installed runtime readiness

Turn the now-proven packaged substrate into an installed-app experience that can explain whether the analyst runtime is usable **before** starting a World session.

Add a desktop-owned readiness layer with a small stable result model. It should distinguish at least:

1. packaged analyst runtime missing/incomplete;
2. packaged `world-agent-tool-stdio` missing or not executable;
3. Node program unresolved/not executable;
4. Pi program unresolved/not executable;
5. runtime ready to start analyst sessions.

The readiness result must be product-safe: do not expose raw Pi events, provider payloads, shell output, or unstable process internals.

## Ownership and layering

The executable/runtime probe belongs below GPUI interaction code.

- GPUI may request readiness asynchronously and render the stable result.
- GPUI View types must not search `PATH`, invoke `Command`, own child processes, or know Pi protocol details.
- Preserve `DesktopAnalystConfig` as the concrete session-start configuration; readiness should validate/resolve that configuration rather than create a second execution model.
- Prefer a small desktop product API that can be injected/tested independently from the View.
- Keep M220/M221 protocol and authority semantics unchanged.

## Executable resolution

Honor explicit overrides first:

- `WORLD_MACHINE_NODE_PROGRAM`
- `PI_PROGRAM`
- `WORLD_MACHINE_ANALYST_PROGRAM`
- `WORLD_MACHINE_ANALYST_RUNTIME_ROOT`

For bare program names, resolve against the process environment deterministically and verify executable eligibility before declaring readiness. The implementation must behave predictably under a Finder-like limited `PATH`; tests should not assume an interactive shell startup file has run.

Do not silently execute arbitrary shell startup files and do not bundle Node or Pi in this milestone.

## Native analyst surface

The analyst setup window should expose a concise readiness state before `Start analyst` can launch a session.

Required behavior:

- readiness work runs off the GPUI update/render path;
- ready state allows the existing M222 session start flow unchanged;
- missing Node/Pi/runtime state is visible and actionable rather than surfacing only as a later generic `Spawn` failure;
- retry/recheck is possible without reopening the World document;
- no provider credentials or provider settings UI is added in M225.

## Validation

Required gates:

- deterministic tests for explicit executable paths, bare-name `PATH` resolution, non-executable/missing programs, and Finder-like limited `PATH`;
- readiness never mutates a World or starts an analyst turn;
- panel source remains above raw process/Pi layers;
- existing M222/M223 cancellation and immutable-snapshot behavior remains unchanged;
- M224 packaged-runtime smoke remains green;
- standard Linux boundary/fmt/Clippy/workspace tests stay green;
- full macOS/GPUI tests stay green;
- `World Machine.app` still builds, validates, signs, passes the packaged analyst smoke, archives, and uploads successfully.

## Non-goals

No bundled Node/Pi distribution, no credential manager, no provider/model settings redesign, no token streaming UI, no concurrent asks, no mutation tools, no HTTP/MCP/WebSocket server, and no analyst protocol v2.
