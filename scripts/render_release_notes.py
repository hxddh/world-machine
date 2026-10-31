#!/usr/bin/env python3
"""Render GitHub release notes from a validated release-manifest.json."""
from __future__ import annotations

import argparse
import json
import re
from pathlib import Path


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Render release notes from a release manifest.")
    parser.add_argument("manifest", type=Path)
    parser.add_argument(
        "--repository",
        required=True,
        help="GitHub owner/name used to link the install guide at the release tag.",
    )
    parser.add_argument(
        "--demo-manifest",
        type=Path,
        help="The demo's manifest, when the release carries the demo beside the full app.",
    )
    parser.add_argument(
        "--changelog",
        type=Path,
        help=(
            "CHANGELOG.md: its entry for the manifest's app version becomes the "
            "notes' \"What's in\" section. A release with no entry is refused."
        ),
    )
    return parser.parse_args()


def changelog_entry(changelog: str, version: str, repository: str, tag: str) -> str:
    """The body of `## v<version> (<date>)` in the changelog, its relative
    links pointing at the files at the release tag."""
    heading = re.compile(rf"^## v{re.escape(version)} \([^)]*\)[ \t]*$", re.MULTILINE)
    found = heading.search(changelog)
    if found is None:
        raise SystemExit(f"CHANGELOG.md has no entry for v{version}")
    rest = changelog[found.end():]
    following = re.search(r"^## ", rest, re.MULTILINE)
    body = (rest[: following.start()] if following else rest).strip()
    if not body:
        raise SystemExit(f"CHANGELOG.md's entry for v{version} is empty")

    def absolute(link: re.Match[str]) -> str:
        target = link.group(2)
        if re.match(r"^(?:[a-z]+:|#|/)", target):
            return link.group(0)
        return f"{link.group(1)}(https://github.com/{repository}/blob/{tag}/{target})"

    return re.sub(r"(\[[^\]]*\])\(([^)\s]+)\)", absolute, body)


def stable_line(version: str, repository: str, tag: str) -> str:
    """What the notes promise about saves: before 1.0 nothing; from 1.0 on,
    the save contract (docs/SAVES.md)."""
    major = int(version.split(".", 1)[0])
    if major < 1:
        return (
            "The World file format may still change between releases; see the "
            f"[changelog](https://github.com/{repository}/blob/{tag}/CHANGELOG.md)."
        )
    return (
        "Saves are versioned from 1.0: every later release opens this one's Worlds, "
        "and a World saved by a newer release is refused, never damaged. Worlds from "
        "the v0.x previews do not open; see "
        f"[saves](https://github.com/{repository}/blob/{tag}/docs/SAVES.md)."
    )


def demo_section(demo: dict | None) -> str:
    """A short note on the free demo published beside the full app."""
    if not demo:
        return ""
    download = demo.get("dmg") or demo["artifact"]
    return f"""
## The free demo

`{download}` is **World Machine Demo**: Tiny Society's first session and the next morning, about half an hour, signed the same way as the full app. When it ends, the harbour is kept in your Library, and the full app opens it and carries on from there. SHA-256 (zip): `{demo["sha256"]}`.
"""


def render(
    manifest: dict,
    repository: str,
    demo: dict | None = None,
    changelog: str | None = None,
) -> str:
    tag = str(manifest["tag"])
    version = str(manifest["app_version"])
    whats_in = ""
    if changelog is not None:
        whats_in = (
            f"\n## What's in {version}\n\n"
            f"{changelog_entry(changelog, version, repository, tag)}\n"
        )
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

    prerelease = "-pre." in tag
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
3. Open World Machine from Applications. It opens on its title screen; choose **New World** to begin.
"""
        signing_cell = "Developer ID, notarized"
    else:
        status_line = (
            "> **Not yet notarized by Apple.** macOS asks once before the first "
            "launch; the steps below take a minute and happen only once."
        )
        install = f"""## Install

1. Download {download} below and open it.
2. Drag **World Machine** onto the **Applications** shortcut.
3. Open World Machine from Applications. macOS says it could not verify the app; click **Done**, then open **System Settings → Privacy & Security**, scroll to **Security**, click **Open Anyway**, and confirm. From then on it opens normally, on its title screen; choose **New World** to begin.

The [install guide]({install_url}) has the same steps with more detail.
"""
        signing_cell = "ad-hoc, not notarized"

    return f"""# World Machine {tag}

{status_line}

{install}
{"Pre-release build. " if prerelease else ""}No account, no telemetry; your Worlds are files on your Mac and nothing is uploaded. Two optional features can reach a model if you switch them on. {stable_line(version, repository, tag)}
{whats_in}
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
{demo_section(demo)}"""


def main() -> None:
    args = parse_args()
    manifest = json.loads(args.manifest.read_text())
    demo = json.loads(args.demo_manifest.read_text()) if args.demo_manifest else None
    changelog = args.changelog.read_text() if args.changelog else None
    print(render(manifest, args.repository, demo, changelog), end="")


if __name__ == "__main__":
    main()
