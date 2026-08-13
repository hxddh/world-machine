# World Machine

> **Status:** Experimental / pre-alpha. The World IR and public APIs are intentionally unstable.

World Machine is an experimental semantic runtime for persistent, inspectable, branchable worlds.

The first product target is **Tiny Society**, but Tiny Society is deliberately not part of the kernel architecture. The runtime now also hosts **Pocket Universe** and the unrelated **Micro Company** Pack, so generality is tested by real Worlds rather than guessed framework abstractions.

## Current runtime

The repository currently implements:

`Entity / Relation -> Action -> Event -> State -> Scheduler / Behavior / Agent -> Replay -> Projection`

The renderer boundary is explicit:

`World -> ProjectionSnapshot -> GPUI`

- `world-projection` defines headless Collection / Timeline / Inspector / Semantic Canvas read models.
- World Packs produce their own projection data without introducing domain concepts into the renderer.
- `world-gpui` consumes only projection models; it does not own World truth and does not depend on `world-core` or a specific Pack.
- The generic GPUI renderer treats Briefing + Commands as the current focus: the next available continuation/choice appears before Canvas and Inspector, while `Explore the world` keeps the semantic state inspectable underneath.
- Projection layout uses three independent vertical scroll regions for Collection, Focus/Explore, and Timeline, so long worlds remain inspectable without moving the fixed World header or hiding current actions.
- Projection selection defaults to semantic Collection entities and preserves explicit user selection across snapshot updates when it remains valid; Timeline events become the fallback or an explicit investigation path rather than an automatic post-command focus.
- `world-machine-desktop` hosts durable `.world` documents, branching/lineage, external Pack installation, durable activation probing, and generic World creation.
- Tiny Society, Pocket Universe, and Micro Company exercise the same public Host/Pack boundaries.
- The macOS app bundle carries Pocket Universe and Micro Company as **included external Packs**. They are not built-ins and are not executed at startup; Home requires an explicit Review & Install action before the existing content-review, quarantine, durable-probe, and activation path runs.
- A fresh packaged Home presents **Pocket Universe** as the primary `Start here` experience (`Seed a place · Let it live · Branch what happens next`) while keeping Micro Company as a secondary World and moving Pack management behind the World/product hierarchy.
- Pocket Universe 0.10 turns its opening generations into a guided first story: observe the first cycle, notice the central relationship forming, then choose whether to steer it or leave the World alone. Larger interventions are presented as optional branches rather than required progress.
- Portable `.worldpack` files are registered as a native macOS file type. Double-clicking a Pack, using Open With, or opening it through the app routes the file into the same static review surface; the open event itself never installs or executes Pack code.
- After a newly installed Pack passes the durable probe and becomes active, Home offers an explicit `Create <World>` handoff. The probe still does not create a user World automatically; the CTA is ephemeral and only remains valid while that exact Pack version is enabled, active, content-valid, and registered.
- Pi remains an optional out-of-process `world-pi-rpc` AgentRuntime adapter.

## Run

```bash
cargo test --workspace
cargo run -p world-cli
bash ./scripts/check-boundaries.sh
```

On macOS, after the GPUI dependencies are available:

```bash
cargo run -p world-machine-desktop
```

Build the distributable app bundle, including the fixed official external `.worldpack` resources:

```bash
bash apps/world-machine-desktop/macos/build-app.sh
```

A source-tree `cargo run` does not invent or scan for included Packs. The packaged app discovers only the fixed `Contents/Resources/World Packs` allowlist (or an explicit development override), and installation still requires user review of the exact executable identity and SHA-256.

The packaged macOS app owns both `io.github.hxddh.world-machine.world` (`.world`) and `io.github.hxddh.world-machine.worldpack` (`.worldpack`). `.world` opens as a World document; `.worldpack` opens only as a Pack installation review.

## Check an external Pack

Statically inspect a `.worldpack` or developer manifest without running Pack code:

```bash
cargo run -p world-pack-catalog --bin world-pack-check -- \
  --inspect-only path/to/example.worldpack
```

Run the minimum durable external-Pack contract in an isolated temporary catalog:

```bash
cargo run -p world-pack-catalog --bin world-pack-check -- \
  path/to/example.worldpack
```

The default check verifies `Create -> Archive -> fresh-process Open` and removes its temporary managed copy afterward. See [docs/PACK_CHECK.md](docs/PACK_CHECK.md).

## Architecture rule

`world-core` owns semantic runtime primitives only. Domain concepts such as Person, Town, Bakery, Job, Evidence, FootballPlayer, Product, Customer, or Company must live in systems/world packs, never in the kernel.

UI state is not World state. Renderers consume projections and may hold ephemeral selection/layout state only.

## License

Apache-2.0. See [LICENSE](LICENSE), [docs/LICENSING.md](docs/LICENSING.md), and [THIRD_PARTY_LICENSES.md](THIRD_PARTY_LICENSES.md) for dependency-license boundaries.
