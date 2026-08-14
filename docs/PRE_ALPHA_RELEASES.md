# Pre-alpha macOS packages

World Machine is experimental pre-alpha software. The repository can build a repeatable macOS release package, but the current bundle is **ad-hoc signed and not notarized**.

That distinction is intentional. A package produced by this pipeline must not be presented as a normal production-signed macOS distribution.

## Package contents

A pre-alpha package contains three files:

- `World-Machine-<release>-macOS-<architecture>.zip` — the app bundle archive.
- the matching `.zip.sha256` — SHA-256 for the archive.
- `release-manifest.json` — machine-readable build identity and distribution status.

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

The archive is never labeled universal unless the built executable actually contains multiple architectures.

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

- **workflow dispatch** — builds a non-publishable `pre.0` package for release dry runs;
- **`v*-pre.*` tag push** — requires a publishable tag, reruns release-critical macOS tests, builds the app, creates the package, and uploads the validated package as a GitHub Actions artifact.

Creating the public GitHub Release entry and attaching these assets is intentionally a separate/manual step for now. The repository does not contain Apple Developer ID or notarization credentials, and the current automation must not imply that an ad-hoc-signed artifact is a normal notarized macOS release.

## Verification

From the directory containing the downloaded ZIP and checksum file:

```bash
shasum -a 256 -c World-Machine-*.zip.sha256
```

Also inspect `release-manifest.json` before using a pre-alpha build. In particular, confirm the expected tag, commit, architecture, SHA-256, and the current `notarized: false` status.
