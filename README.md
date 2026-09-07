# World Machine

Persistent worlds that remember, evolve, and branch. A World keeps living while you are away, tells you what changed when you return, and lets you fork it to try the other choice.

> **Pre-alpha for macOS.** Builds are ad-hoc signed and not notarized, so the first launch takes two extra steps. Everything stays on your Mac: no account, no telemetry.

## Try it in three steps

1. **Download** the latest zip from the [Releases page](https://github.com/hxddh/world-machine/releases) (universal: Apple Silicon and Intel, macOS 14 or newer). Homebrew users: `brew install --cask --no-quarantine hxddh/tap/world-machine`.
2. **Allow the first launch** by following [docs/INSTALL.md](docs/INSTALL.md); it takes under two minutes and explains why macOS asks.
3. **Start a World.** Home prepares Pocket Universe on first launch. Seed a place, let it live, come back later, and use **What if…** on any World card to compare two futures side by side.

<!-- screenshot: docs/screenshots/home.png (captured on a real Mac; the CI runner cannot rasterize text) -->

## What is inside

- **Pocket Universe** (start here): a tiny persistent world whose inhabitants act on their own. Three chapters unfold over visits: a relationship forms and you choose whether to steer it, a legacy takes shape, then pressure rises against the World's anchor and you hold or reach beyond it.
- **Micro Company**: a two-actor product company that can find traction or run out of cash.
- **Tiny Society**: the original reference world with economic circulation and institutional risk.
- Every World is a `.world` document in your library. Open, fork, compare, export, and import them; history is replayed from events, never re-run through an AI.

## Documentation

- [Install guide](docs/INSTALL.md), [known issues](docs/KNOWN_ISSUES.md), [privacy](docs/PRIVACY.md), [changelog](CHANGELOG.md)
- [Roadmap](docs/ROADMAP.md) and [pre-alpha release process](docs/PRE_ALPHA_RELEASES.md)
- [Architecture](ARCHITECTURE.md), [runtime overview](docs/RUNTIME.md), [World IR](docs/WORLD_IR_v0.1.md), [checking an external Pack](docs/PACK_CHECK.md), [Pi analyst](docs/PI_ANALYST.md)

## Build from source

```bash
cargo test --workspace
bash ./scripts/check-boundaries.sh
```

On macOS, run the desktop app or build the distributable bundle with its included World Packs:

```bash
cargo run -p world-machine-desktop
bash apps/world-machine-desktop/macos/build-app.sh
bash apps/world-machine-desktop/macos/package-release.sh
```

A source-tree `cargo run` does not scan for Packs. The packaged app discovers only the fixed `Contents/Resources/World Packs` allowlist and activates those on first launch; any other `.worldpack` requires a review of its executable identity and SHA-256 before it runs.

## Architecture rule

`world-core` owns semantic runtime primitives only. Domain concepts such as Person, Town, Bakery, Product, or Company live in World Packs, never in the kernel. UI state is not World state: renderers consume projections and may hold ephemeral selection and layout state only.

## License

Apache-2.0. See [LICENSE](LICENSE), [docs/LICENSING.md](docs/LICENSING.md), and [THIRD_PARTY_LICENSES.md](THIRD_PARTY_LICENSES.md).
