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
    signing = manifest.get("signing")
    notarized = manifest.get("notarized")
    if signing == "ad-hoc" and notarized is False:
        gatekeeper = "blocked"
    elif signing == "developer-id" and notarized is True:
        gatekeeper = "clear"
    else:
        raise ValueError(
            "release notes describe either an ad-hoc unnotarized build or a "
            "Developer ID notarized build"
        )

    install_url = f"https://github.com/{repository}/blob/{tag}/docs/INSTALL.md"
    architectures = ", ".join(manifest["architectures"])
    packs = "\n".join(f"- `{pack}`" for pack in manifest["included_packs"])
    artifact = manifest["artifact"]
    dmg = manifest.get("dmg")
    download = f"`{dmg}`" if dmg else f"`{artifact}`"

    if gatekeeper == "clear":
        status_line = (
            "Signed with a Developer ID and notarized by Apple. Download, open, drag "
            "the app to Applications, and it runs."
        )
        install = f"""## Install

1. Download {download} below and open it.
2. Drag **World Machine** onto the **Applications** shortcut.
3. Open World Machine from Applications. Your first World opens by itself.
"""
        signing_cell = "Developer ID, notarized"
    else:
        status_line = (
            "> **Pre-alpha, not yet notarized by Apple.** macOS asks once before the "
            "first launch; the steps below take a minute and happen only once."
        )
        install = f"""## Install

1. Download {download} below and open it.
2. Drag **World Machine** onto the **Applications** shortcut.
3. Open World Machine from Applications. macOS says it could not verify the app; click **Done**, then open **System Settings → Privacy & Security**, scroll to **Security**, click **Open Anyway**, and confirm. From then on it opens normally, and your first World opens by itself.

The [install guide]({install_url}) has the same steps with more detail.
"""
        signing_cell = "ad-hoc, not notarized"

    return f"""# World Machine {tag}

{status_line}

{install}
Experimental pre-alpha software: everything stays on your Mac, and the World format may still change between releases.

## Build

| Field | Value |
| --- | --- |
| App version | {manifest["app_version"]} |
| Commit | `{manifest["commit"]}` |
| Architecture | {architectures} |
| Signing | {signing_cell} |
| SHA-256 (zip) | `{manifest["sha256"]}` |

Included World Packs:

{packs}

## Verify the download (optional)

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
