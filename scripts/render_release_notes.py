#!/usr/bin/env python3
"""Render GitHub release notes from a validated release-manifest.json."""
from __future__ import annotations

import argparse
import json
from pathlib import Path


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Render pre-alpha release notes.")
    parser.add_argument("manifest", type=Path)
    parser.add_argument(
        "--repository",
        required=True,
        help="GitHub owner/name used to link the install guide at the release tag.",
    )
    return parser.parse_args()


def render(manifest: dict, repository: str) -> str:
    tag = str(manifest["tag"])
    if manifest.get("notarized") is not False or manifest.get("signing") != "ad-hoc":
        raise ValueError("release notes template only describes ad-hoc, unnotarized builds")

    install_url = f"https://github.com/{repository}/blob/{tag}/docs/INSTALL.md"
    architectures = ", ".join(manifest["architectures"])
    packs = "\n".join(f"- `{pack}`" for pack in manifest["included_packs"])
    artifact = manifest["artifact"]

    return f"""# World Machine {tag}

> **Not notarized.** This pre-alpha build is ad-hoc signed, so a browser download is blocked on its first launch. The one-line installer avoids that; the [install guide]({install_url}) has the manual path.

## Install

Open Terminal, paste, press Return:

```bash
curl -fsSL https://raw.githubusercontent.com/hxddh/world-machine/main/scripts/install.sh | sh
```

Or download the zip below, drag the app to Applications, and allow it once in System Settings → Privacy & Security. Rerun the same line to update later.

Experimental pre-alpha software. The World IR and public APIs are unstable.

## Build

| Field | Value |
| --- | --- |
| App version | {manifest["app_version"]} |
| Commit | `{manifest["commit"]}` |
| Architecture | {architectures} |
| Signing | ad-hoc, not notarized |
| SHA-256 | `{manifest["sha256"]}` |

Included World Packs:

{packs}

## Verify the download (optional)

The installer checks this for you. By hand:

```bash
shasum -a 256 -c {artifact}.sha256
```

Then open `release-manifest.json` and confirm the tag, commit, and checksum above.
"""


def main() -> None:
    args = parse_args()
    manifest = json.loads(args.manifest.read_text())
    print(render(manifest, args.repository), end="")


if __name__ == "__main__":
    main()
