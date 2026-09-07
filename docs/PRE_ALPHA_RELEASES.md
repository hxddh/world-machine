# Pre-alpha macOS packages

World Machine is experimental pre-alpha software. The repository can build a repeatable macOS release package, but the current bundle is **ad-hoc signed and not notarized**.

That distinction is intentional. A package produced by this pipeline must not be presented as a normal production-signed macOS distribution.

## Package contents

A package contains five files:

- `World-Machine-<release>-macOS-<architecture>.dmg` — the download for people: the app and an Applications shortcut.
- `World-Machine-<release>-macOS-<architecture>.zip` — the app bundle archive, for scripts and checksums.
- a `.sha256` for each of the two.
- `release-manifest.json` — machine-readable build identity and distribution status.

The manifest's `signing` is `ad-hoc` or `developer-id` and `notarized` is `false` or `true`; the validator accepts exactly those two combinations. With the secrets in [RELEASE_SIGNING.md](RELEASE_SIGNING.md) the same workflow produces the signed, notarized package.

The archive unpacks to a `World Machine <release>` folder holding `World Machine.app` and, for unnotarized builds only, `Read Me First.txt`, the first-launch note rendered from `apps/world-machine-desktop/macos/READ_ME_FIRST.txt`.

The manifest records:

- app version;
- release tag;
- exact Git commit;
- bundle identifier;
- binary architecture(s);
- included external World Packs;
- archive SHA-256;
- `signing: ad-hoc`;
- `notarized: false`.

The archive is never labeled universal unless the built executable actually contains multiple architectures. The release workflow builds with `WORLD_MACHINE_UNIVERSAL=1`, which compiles every bundled executable for `aarch64-apple-darwin` and `x86_64-apple-darwin` and merges them with `lipo`; a local `build-app.sh` without that variable stays a host-only build.

## Tag contract

Publishable pre-alpha tags use:

```text
v<app-version>-pre.<N>
```

where `N >= 1`.

For the current `0.1.0` app this means, for example:

```text
v0.1.0-pre.1
```

The package validator rejects a tag whose version does not exactly match `world-machine-desktop`.

`pre.0` is reserved for CI/dry-run packages and must not be published as a release.

## Build locally on macOS

```bash
bash apps/world-machine-desktop/macos/build-app.sh
bash apps/world-machine-desktop/macos/package-release.sh
```

The package step verifies the app signature, derives the real executable architecture, creates the archive and checksum, writes the manifest, and validates the resulting package.

For a release-tag-equivalent local build:

```bash
WORLD_MACHINE_RELEASE_TAG=v0.1.0-pre.1 \
  bash apps/world-machine-desktop/macos/package-release.sh
python3 scripts/validate_release_package.py \
  --publishing target/release-package
```

## GitHub Actions

`.github/workflows/release-package.yml` supports two modes:

- **workflow dispatch** without `release_tag` — builds a non-publishable `pre.0` package for release dry runs;
- **workflow dispatch** with `release_tag` set to a publishable `v<app-version>-pre.N` — validates the tag against the app version, builds and packages, then creates that tag at the dispatched commit and publishes the pre-release. Use this when the tag cannot be pushed from a local checkout;
- **`v*-pre.*` tag push** — requires a publishable tag, reruns release-critical macOS tests, builds the app, creates the package, uploads the validated package as a GitHub Actions artifact, and then publishes a GitHub **pre-release** for the tag with the three package files attached.

The pre-release notes are rendered by `scripts/render_release_notes.py` from `release-manifest.json`. They lead with the not-notarized status and link `docs/INSTALL.md` at the release tag, so the first thing a downloader reads is how to get past Gatekeeper. The repository does not contain Apple Developer ID or notarization credentials, and the release entry must never present an ad-hoc-signed artifact as a normal notarized macOS release.

## Publishing a pre-alpha

```bash
git tag v0.1.0-pre.1
git push origin v0.1.0-pre.1
```

Or, from the Actions tab, run **Pre-alpha Package** on `main` with `release_tag` set to `v0.1.0-pre.1`; the workflow creates the tag itself.

The workflow refuses a tag whose version does not match `world-machine-desktop`. On a tag push, `gh release create --verify-tag` refuses to publish if the tag is missing from the repository; on a dispatch, the tag is created at the dispatched commit. A release that already exists for the tag makes the publish step fail rather than overwrite it; delete the release manually before re-running if that is intended.

## Verification

From the directory containing the downloaded ZIP and checksum file:

```bash
shasum -a 256 -c World-Machine-*.zip.sha256
```

Also inspect `release-manifest.json` before using a pre-alpha build. In particular, confirm the expected tag, commit, architecture, SHA-256, and the current `notarized: false` status.

First-launch steps for the unnotarized app on macOS 14 and 15 are in [INSTALL.md](INSTALL.md).
