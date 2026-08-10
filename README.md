# World Machine

> **Status:** Experimental / pre-alpha. The World IR and public APIs are intentionally unstable.

World Machine is an experimental semantic runtime for persistent, inspectable, branchable worlds.

The first product target is **Tiny Society**, but Tiny Society is deliberately not part of the kernel architecture. The kernel is intended to host very different worlds: detective cases, football simulations, scientific playgrounds, personal-data worlds, and future worlds that are not known today.

## Current milestone

The repository currently implements:

`Entity / Relation -> Action -> Event -> State -> Scheduler / Behavior / Agent -> Replay -> Projection`

The first renderer boundary is now explicit:

`World -> ProjectionSnapshot -> GPUI`

- `world-projection` defines headless Collection / Timeline / Inspector / Semantic Canvas read models.
- Tiny Society produces its own projection data without introducing Society concepts into the renderer.
- `world-gpui` consumes only projection models; it does not own World truth and does not depend on `world-core` or Tiny Society.
- `tiny-society-desktop` is the first macOS GPUI application shell.
- Pi remains an optional out-of-process `world-pi-rpc` adapter.

## Run

```bash
cargo test --workspace
cargo run -p world-cli
cargo run -p tiny-society
bash ./scripts/check-boundaries.sh
```

On macOS, after the GPUI dependencies are available:

```bash
cargo run -p tiny-society-desktop
```

## Architecture rule

`world-core` owns semantic runtime primitives only. Domain concepts such as Person, Town, Bakery, Job, Evidence, or FootballPlayer must live in systems/world packs, never in the kernel.

UI state is not World state. Renderers consume projections and may hold ephemeral selection/layout state only.

## License

Apache-2.0. See [LICENSE](LICENSE), [docs/LICENSING.md](docs/LICENSING.md), and [THIRD_PARTY_LICENSES.md](THIRD_PARTY_LICENSES.md) for dependency-license boundaries.
