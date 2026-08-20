# Next Coding Task — M226 Persistent Analyst Runtime Paths

M214–M225 now provide a complete first installed-app path for read-only World analysis: read-only evidence tools, restricted Pi execution, a stable provider-neutral turn protocol, a strict Rust client, immutable desktop-owned analyst sessions, a native GPUI analyst panel, packaged-runtime E2E validation, and preflight readiness that explains whether the installed runtime can actually start before a World session is created.

## Current baseline

- M220 defines `world-machine-analyst-turns@1` and strips raw Pi/provider events from the product boundary.
- M221/M221.1 provide strict request/correlation validation and fail-closed process semantics.
- M222 binds one analyst session to two immutable raw archive snapshots and owns cleanup/cancellation below GPUI.
- M223 adds the live native `Analyze saved Worlds…` surface and packages the analyst host, launcher, extension/client, and `world-agent-tool-stdio` inside `World Machine.app`.
- M224 proves that packaged substrate end to end from outside the source checkout, including two evidence-backed turns through the packaged Rust tool host on one long-lived Pi process.
- M225 adds a desktop-owned readiness model and native preflight surface. It resolves/validates the packaged runtime, launcher dependencies, Node, Pi, and the analyst tool host before `Start analysis` is enabled. Runtime discovery and executable probing stay below GPUI.
- Node and Pi intentionally remain external dependencies. Today the practical user-facing overrides are still environment variables (`WORLD_MACHINE_NODE_PROGRAM` and `PI_PROGRAM`), which is awkward for a macOS app launched from Finder.

## M226 — persistent Node/Pi runtime paths

Add the smallest durable user setting needed to turn M225 diagnostics into a usable Finder-installed workflow: persist optional **Node** and **Pi** executable paths for World Analyst.

Do not build a general settings framework in this milestone. The persisted schema should contain only:

- optional absolute Node executable path;
- optional absolute Pi executable path;
- an explicit settings format version.

The packaged analyst runtime root and packaged `world-agent-tool-stdio` remain implementation/development concerns, not ordinary user settings.

## Storage and precedence

Use the existing macOS World Machine application-support root rather than inventing another location:

`~/Library/Application Support/World Machine/`

Store a small versioned analyst settings file beside the existing `Worlds/` and `Packs/` areas.

Resolution precedence must be explicit and testable:

1. environment override (`WORLD_MACHINE_NODE_PROGRAM` / `PI_PROGRAM`);
2. persisted user path;
3. existing bare-name default (`node` / `pi`) resolved by M225 readiness.

An environment-controlled value must not be silently replaced by a persisted value.

## Product settings API

Filesystem ownership belongs below GPUI. Add a small desktop product API that can:

- load settings;
- save settings atomically;
- clear one or both persisted paths;
- distinguish absent settings from malformed/unsupported settings;
- validate that persisted values are absolute paths before accepting them;
- feed the effective Node/Pi selections into the existing M225 `DesktopAnalystConfig`/readiness path.

Keep the format strict and versioned. Unknown versions, unknown required structure, or malformed JSON must produce a stable product-safe error and must **not** be silently overwritten during load/recheck.

Use write-to-temp + rename (or equivalent atomic replacement) so interruption cannot leave a partially written settings file.

## Native analyst surface

Extend the M225 setup/readiness area only as much as needed for the two paths.

Required behavior:

- when Node or Pi is unavailable, offer a native way to choose/configure the corresponding executable without requiring Terminal environment setup;
- prefer native file picking over another free-form path text field;
- show the currently effective source for each path: environment, persisted setting, or PATH/default;
- if an environment override controls a field, explain that it is environment-controlled instead of pretending a persisted edit will win;
- allow clearing a persisted Node or Pi path back to PATH/default behavior;
- after save/clear, run M225 readiness again without reopening the World document;
- all file I/O remains off the GPUI update/render path;
- no analyst session is started merely by editing settings.

## Layering and authority

- GPUI View types must not read/write the settings file directly.
- GPUI must not mutate process environment variables.
- Reuse `DesktopAnalystConfig` and M225 readiness; do not create a second session-start model.
- Keep M220/M221 protocol, read-only authority, M222 immutable snapshots, and M223 cancellation semantics unchanged.
- Provider/model/thinking values are not persisted by this milestone.

## Validation

Required gates:

- no-file defaults work cleanly;
- save/load round-trip for Node/Pi paths;
- persisted paths must be absolute;
- malformed JSON and unsupported version produce stable errors without data loss;
- environment override > persisted path > bare default precedence is covered directly;
- clear/reset restores default/PATH behavior;
- atomic replacement is exercised without leaving a partial target file;
- settings changes feed M225 readiness without starting an analyst turn;
- panel source remains above filesystem/process implementation details;
- M224 packaged-runtime smoke remains green;
- standard Linux boundary/fmt/Clippy/workspace tests remain green;
- full macOS/GPUI tests remain green;
- `World Machine.app` still builds, validates, signs, passes the packaged analyst smoke, archives, and uploads successfully.

## Non-goals

No credential manager, no API-key storage, no provider/model/thinking settings redesign, no bundled Node/Pi distribution, no shell-profile sourcing, no token streaming UI, no concurrent asks, no mutation tools, no analyst protocol v2, and no broad application Preferences rewrite.
