# Changelog

World Machine is pre-alpha. Versions below are the pre-release tags on the [Releases page](https://github.com/hxddh/world-machine/releases); `0.2.0` is the first release intended to be usable without reading the repository.

## Unreleased

- Diagnostics: a local log under `~/Library/Logs/World Machine`, an About window with the build label and signing status, and Help menu items to copy diagnostics, show the log, open the install guide, and report a problem. Cmd-Q quits.
- Home: **What if…** on every World card opens the World and runs the comparison immediately (its first two choices, 20 periods); "Change choices…" in the result window opens the full setup.
- Fewer controls: Home's header buttons move to a File menu (Import World, Install World Pack, Refresh); installed-Pack management is folded behind one line; the World window keeps only Branch and What if… as buttons, with Save As, Reload, lineage, comparisons, and the Analyst in a World menu.
- Layout: Home header, World cards, document chrome, and the projection center pane no longer lay out wider than their window.
- Copy: the first-launch path speaks of Worlds being prepared and ready instead of Packs, probes, and registries.
- Tiny Society: every briefing opens with a "Harbor today" state line, so a fresh visit and a return share the same now / changed / can-do shape.
- Install: a one-line installer (`curl … install.sh | sh`) downloads, verifies, installs, clears the first-launch block, and opens the app; rerun to update. Help → Check for Updates… opens the Releases page.
- First launch on a fresh install opens straight into the first Pocket Universe World instead of stopping at a Create card.
- Release: the zip unpacks to a folder with the app and a `Read Me First.txt` explaining the one-time "Open Anyway" step.
- Docs: user-first README, this changelog, known issues, privacy note.

## v0.1.0-pre.5 (2026-09-07)

- Pocket Universe 0.16: chapter three. After a legacy has reinforced itself, a seed-specific pressure rises against the World's anchor, warns, peaks, and can durably cost the anchor; the observer holds or reaches, and a lost anchor can be recovered.

## v0.1.0-pre.4 (2026-09-07)

- Universal build: one zip runs on Apple Silicon and Intel Macs, including the bundled Packs and the analyst host.

## v0.1.0-pre.3 (2026-09-07)

- Included Packs activate on first launch without a review dialog; the review flow remains for user-supplied `.worldpack` files.

## v0.1.0-pre.2 (2026-09-07)

- The experimental World Analyst entry is hidden unless Node and Pi are installed, so a fresh install needs nothing beyond the app.

## v0.1.0-pre.1 (2026-09-07)

- First automated pre-release: a tag or a workflow dispatch builds the app, validates the package, and publishes the zip, SHA-256, manifest, and release notes. Install guide for macOS 14 and 15.
