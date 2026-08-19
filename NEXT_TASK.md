# Next Coding Task — M222 Native Desktop Analyst Session Controller

M214–M221 now provide a complete external read-only analyst stack from World evidence through a restricted Pi tool loop to a provider-neutral Rust client. The native product can finally consume one completed analyst turn without knowing Pi RPC, Node event names, or provider-normalized tool shapes.

The next missing layer is not UI. It is a small desktop-owned session controller that binds a fixed pair of saved Library Worlds to `world-analyst-client` while keeping process lifecycle and configuration out of GPUI views.

## Current baseline

- M214–M218 expose read-only evidence tools through an archive-bound restricted Pi extension.
- M219 owns Pi prompt acknowledgement, event accumulation, `agent_settled` completion, and process failure semantics.
- M220 normalizes the completed turn into `world-machine-analyst-turns@1` and strips Pi/provider internals.
- M221 exposes that protocol to Rust as typed DTOs plus a fail-closed long-lived child-process client.
- `WorldLibrary` already exposes `path(&WorldDocumentId) -> PathBuf`, so desktop code does not need another archive-resolver crate.
- The live desktop document actions currently wire `world_fork` and `strategy_compare`. A `saved_compare.rs` source file exists, but it is not currently wired from `main.rs`; M222 must not depend on that dormant UI path as if it were already product surface.

## M222 — native desktop analyst session controller

Add a non-View controller in `world-machine-desktop` that:

- accepts a fixed saved-World pair (`left`, `right`) plus the existing `WorldLibrary`;
- resolves both archive paths exactly once with `WorldLibrary::path` and refuses identical/missing documents before starting an analyst process;
- owns one `world_analyst_client::AnalystTurnProcess` for the lifetime of the analyst session;
- exposes a small product state model such as idle / answer / recoverable-error / fatal-error / closed without exposing Pi concepts;
- submits sequential questions through M221 and retains completed `AnalystTurn` values for later UI rendering;
- keeps correlated non-fatal command rejection reusable and closes/poisons the controller after fatal M221 failures;
- owns deterministic shutdown/drop behavior so future comparison windows cannot orphan Node/Pi children;
- keeps provider/model/thinking and executable/script resolution in explicit desktop configuration rather than hard-coding them inside GPUI rendering code.

The controller should be testable without GPUI by injecting an analyst-process factory or similarly narrow abstraction. Its production implementation may use `AnalystTurnProcess`; tests should prove path binding, state transitions, reuse after non-fatal rejection, fail-closed fatal behavior, and cleanup.

## Product integration point

M222 should remain independent of a particular View. It establishes the desktop ownership boundary first: `WorldLibrary + left/right WorldDocumentId -> analyst session controller`.

M223 can then choose the actual product entry point. Likely options include wiring a saved-World comparison surface or attaching analysis to persisted outcomes from the existing strategy comparison flow. That decision should be made against the live UI, not by reviving the dormant `saved_compare.rs` file implicitly.

## Boundary rules

M222 may depend on `world-analyst-client` and `world-library`. It must not depend on `world-pi-rpc`, parse Pi events, register tools, execute evidence queries directly, or mutate either World. GPUI Views must not own raw `Child`, stdin/stdout handles, Node paths, or Pi lifecycle state.

Archive selection remains fixed for the controller lifetime. A question cannot choose another archive pair or add tools.

## Validation

- controller unit tests independent of GPUI;
- saved-World path binding tests using a temporary `WorldLibrary`;
- sequential ask and retained-turn state tests;
- non-fatal command rejection followed by successful reuse;
- fatal client error closes/poisons the controller and prevents another ask;
- drop/shutdown cleanup tests with no orphaned child;
- architecture/source guard preventing Pi/provider/process details from leaking into GPUI View types;
- existing M214–M221 tests unchanged and green;
- full workspace and macOS/GPUI gates.

## Non-goals

No visible chat/analyst panel yet, no streaming tokens, no concurrent asks, no arbitrary file picker for analyst archives, no mutation tools, no protocol v2, and no provider abstraction layer.
