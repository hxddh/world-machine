# Next Coding Task — M224 Packaged Analyst Runtime End-to-End Readiness

M214–M223 now form a complete first product path for read-only World analysis: evidence tools, restricted Pi analyst execution, a stable provider-neutral JSONL boundary, a strict Rust client, a desktop-owned session controller, and a native GPUI analyst panel.

M223 also bundles the analyst host, launcher, and Pi integration modules inside `World Machine.app`. The app build currently proves that those files exist, are non-empty, and produce a valid signed app. It does **not** yet prove that the packaged runtime can actually start from the installed app layout and complete analyst turns without relying on the source checkout.

## Current baseline

- M220 defines `world-machine-analyst-turns@1` and strips raw Pi/provider events from the product boundary.
- M221/M221.1 provide strict request validation, correlation, poisoning semantics, and process lifecycle handling.
- M222 binds one desktop session to two immutable raw archive snapshots and cleans them on close/fatal/drop.
- M223 adds the live `Analyze saved Worlds…` action, a separate native analyst window, fixed-pair selection, sequential background asks, answer/evidence/error rendering, and a native GPUI text input.
- M223 packages `world-agent-tool-stdio`, the M220 turn host/RPC modules, the restricted Pi extension/client, and `scripts/run-pi-analyst.sh` under `World Machine.app/Contents/Resources/Analyst Runtime`.
- The live `world_fork` action row also wires saved comparison and lineage actions; `saved_compare.rs` is therefore live through `world_fork.rs` rather than dormant.
- Node and Pi remain external runtime dependencies. Their absence or a launch failure is surfaced to the user instead of being hidden inside GPUI.

## M224 — packaged runtime end-to-end smoke

Add a deterministic macOS integration check that proves the **built app's packaged analyst runtime** can execute end to end without a source-tree runtime dependency.

The smoke path should:

1. build `World Machine.app` using the existing app bundle script;
2. address the analyst runtime exclusively through `Contents/Resources/Analyst Runtime`;
3. use deterministic local archive fixtures and the existing fake/test Pi strategy rather than real provider credentials or network calls;
4. start the packaged M220 turn host with the packaged `world-agent-tool-stdio` binary and packaged launcher/integration modules;
5. complete at least two sequential analyst turns in one long-lived session;
6. verify request/response correlation, canonical tool-call output, and final analyst text at the stable `world-machine-analyst-turns@1` boundary;
7. verify the second turn reuses the same analyst process/session rather than silently spawning a fresh one;
8. run from a working directory that is **not** the repository root so accidental source-checkout fallback is detected;
9. fail clearly if any packaged relative path, executable bit, launcher contract, or environment handoff is broken.

## Product/runtime boundary

Keep this milestone below the GPUI interaction layer. M224 is about proving that the app artifact contains a runnable analyst substrate, not adding more UI.

- Do not make GPUI own Node/Pi/process state.
- Do not add mutation tools or broaden analyst authority.
- Do not require real API keys, provider accounts, or external network access in CI.
- Do not weaken M220/M221 strict protocol validation to make the smoke test easier.
- Prefer exercising the same packaged files and launch contract that a real installed app uses instead of constructing a parallel test-only runtime layout.

## Runtime diagnostics

Where a packaging/runtime precondition can be checked deterministically before process launch, make the failure actionable. In particular, distinguish failures such as:

- packaged runtime file missing/incomplete;
- packaged analyst host missing or not executable;
- Node executable unavailable;
- Pi executable/launcher unavailable;
- analyst process started but failed protocol startup;
- turn protocol failure after startup.

Keep these as product-safe diagnostics; do not expose raw provider payloads or Pi event streams.

## Validation

Required gates:

- packaged-runtime smoke completes two sequential turns from outside the source checkout;
- the smoke uses packaged `world-agent-tool-stdio` and packaged JS/launcher files, not repository copies;
- no provider/network credentials are needed;
- malformed or missing packaged runtime pieces fail deterministically;
- existing M220/M221/M222 tests remain unchanged in authority semantics;
- standard Linux boundary/fmt/Clippy/workspace tests stay green;
- full macOS/GPUI tests stay green;
- `World Machine.app` still builds, validates, signs, archives, and uploads successfully.

## Non-goals

No token streaming UI, no concurrent asks, no provider settings redesign, no bundled Node/Pi distribution, no mutation tools, no HTTP/MCP/WebSocket server, and no protocol v2 in M224.
