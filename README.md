# World Machine

Persistent worlds that remember and evolve. A World grows a day at a time: each day you sit down with it, a morning brings a letter, a choice, something looked after and a find, and you let the day pass when you are ready.

> **World Machine 1.0 for macOS.** No account, no telemetry; your Worlds are files on your Mac and nothing is uploaded. Two optional features can reach a model if you switch them on — see [privacy](docs/PRIVACY.md). Builds are not yet notarized, so the first launch asks once; see [docs/RELEASE_SIGNING.md](docs/RELEASE_SIGNING.md) for what makes that go away.

## Try it in three steps

1. **Download** the `.dmg` from the [Releases page](https://github.com/hxddh/world-machine/releases), open it, and drag World Machine onto Applications. macOS 14 or newer, Apple Silicon or Intel. Until the app is notarized, the first launch asks once; [docs/INSTALL.md](docs/INSTALL.md) shows the one step.
2. **Choose New World** on the title screen. Your harbour's first session is six days, then Goodnight.
3. **Come back tomorrow.** Each sitting brings one new day and its morning. Nothing moves while you are away, and with **Play freely** on in Settings the next day comes as soon as you want it.

<!-- screenshot: docs/screenshots/home.png (captured on a real Mac; the CI runner cannot rasterize text) -->

## What is inside

- **Pocket Universe** (start here): a tiny persistent world whose inhabitants act on their own. A relationship forms and you choose whether to steer it, a legacy takes shape, trouble rises against the World's anchor, and a successor steps forward — and then the next era begins, facing a different trouble, shaped by how you answered the last one. It grows a day at a time, and each morning says what changed.
- **Tiny Society**: a harbour town where money circulates between neighbours. One durable choice about a fishing boat decides whether local spending recovers, whether the bakery reopens, and whether the job the closure cost comes back.
- Every World is a `.world` document in your library. Open, rename, export, and import them; history is replayed from events, never re-run through an AI.

## Documentation

- [Install guide](docs/INSTALL.md), [known issues](docs/KNOWN_ISSUES.md), [privacy](docs/PRIVACY.md), [changelog](CHANGELOG.md)
- [Roadmap](docs/ROADMAP.md), [product review](docs/PRODUCT_REVIEW.md), and [pre-alpha release process](docs/PRE_ALPHA_RELEASES.md)
- [Architecture](ARCHITECTURE.md), [runtime overview](docs/RUNTIME.md), [World IR](docs/WORLD_IR_v0.1.md), [checking an external Pack](docs/PACK_CHECK.md)

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
