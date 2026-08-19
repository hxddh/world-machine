# Next Coding Task — M223 Native Analyst Panel

M214–M222 now provide the complete read-only analyst substrate below the UI: archive evidence tools, a restricted Pi analyst loop, a provider-neutral process protocol, a strict Rust client, and a desktop-owned session controller.

M222 deliberately stops before GPUI. The next milestone is the first visible product surface that lets a user select two persisted Worlds, ask questions about their differences, and read evidence-backed answers without exposing Pi/Node/process concepts.

## Current baseline

- M220 defines `world-machine-analyst-turns@1` and strips provider/Pi internals.
- M221/M221.1 expose a strict fail-closed Rust client and hardened process lifecycle.
- M222 adds `world_machine_desktop::analyst_session::DesktopAnalystSession`.
- M222 loads the two `WorldLibrary` documents once, extracts raw `WorldArchive` JSON into private immutable temporary snapshots, binds the analyst process to that fixed pair, and cleans snapshots on close/fatal/drop.
- M222 exposes only product-facing state, completed `AnalystTurn` history, `ask`, and `close`; GPUI does not own child/stdin/stdout/Pi state.
- The live document chrome currently wires `world_fork` and `strategy_compare`. `saved_compare.rs` exists in the source tree but is dormant and must not be treated as an already-live entry point.

## M223 — native analyst panel

Add a real GPUI analyst surface to `world-machine-desktop` that:

- adds an explicit live document action such as `Analyze saved Worlds…` rather than relying on dormant `saved_compare.rs`;
- uses the current persisted Library World as the default left side and lets the user choose a different persisted World as the right side;
- starts exactly one `DesktopAnalystSession` after the pair is confirmed;
- presents a question input, completed answer history, canonical evidence/tool-call summaries, recoverable errors, and fatal/closed state using only the M222 public API;
- runs blocking analyst work off the GPUI render/update path and returns results to the View through normal GPUI async/background mechanisms;
- disables or serializes input while one ask is in flight so the M221/M222 single-flight contract remains visible in product behavior;
- closes the M222 session when the analyst window is closed so process and archive snapshots are cleaned promptly;
- keeps provider/model/executable configuration outside rendering code, with a clear unavailable/configuration state when the analyst runtime cannot start.

## Product shape

Prefer a focused separate analyst window/panel over embedding chat into every World document. The first useful workflow should be:

`current saved World -> choose comparison World -> ask why/how they differ -> inspect answer + evidence calls -> ask follow-up`

Do not require the user to understand archives, tools, Pi, request IDs, or provider-normalized names.

## Boundary rules

- GPUI code may depend on the public M222 desktop analyst API, but must not import `world-analyst-client` directly.
- Views must not own `Child`, stdin/stdout handles, Pi RPC/event names, archive snapshot paths, or evidence-query executors.
- No World mutation tools; the analyst remains read-only.
- No arbitrary per-question archive switching. Changing the pair means starting a new M222 session.
- Do not block the UI thread while waiting for an analyst answer.

## Validation

- pair-selection behavior and same-World prevention;
- one in-flight ask at a time;
- answer-history rendering from retained `AnalystTurn` values;
- canonical evidence/tool-call rendering without raw Pi/provider fields;
- recoverable error allows another ask in the same session;
- fatal state disables further asks and presents a stable failure state;
- closing the analyst window closes the controller exactly once;
- source guard proving GPUI does not import/process Pi/Node lifecycle details;
- full macOS/GPUI tests plus World Machine.app build/validation.

## Non-goals

No streaming-token UI yet, no concurrent asks, no mutation tools, no HTTP/MCP/WebSocket server, no protocol v2, and no general provider settings redesign.
