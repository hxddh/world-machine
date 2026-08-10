# World Machine

> **Status:** Experimental / pre-alpha. The World IR and public APIs are intentionally unstable.

World Machine is an experimental semantic runtime for persistent, inspectable, branchable worlds.

The first product target is **Tiny Society**, but Tiny Society is deliberately not part of the kernel architecture. The kernel is intended to host very different worlds: detective cases, football simulations, scientific playgrounds, personal-data worlds, and future worlds that are not known today.

## Current milestone

The repository currently implements the headless kernel slice:

`Entity / Relation -> Action -> Event -> State -> Scheduler -> Replay`

No GPUI or agent-runtime implementation dependency is allowed in `world-core`. Pi integrations must remain optional adapters; GPUI belongs only in the renderer layer.

## Run

```bash
cargo test --workspace
cargo run -p world-cli
./scripts/check-boundaries.sh
```

## Architecture rule

`world-core` owns semantic runtime primitives only. Domain concepts such as Person, Town, Bakery, Job, Evidence, or FootballPlayer must live in systems/world packs, never in the kernel.

## License

Apache-2.0. See [LICENSE](LICENSE) and [docs/LICENSING.md](docs/LICENSING.md) for the dependency-license boundary.
