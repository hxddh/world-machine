#!/usr/bin/env python3
"""Every desktop app must ask `gpui_platform` for the `font-kit` feature.

`gpui_platform`'s default feature set is empty. Without `font-kit`,
`MacPlatform::new` silently installs `gpui::NoopTextSystem` instead of
`MacTextSystem`: layout still runs (with a fixed 0.6em advance per
character, so windows are laid out too wide), every glyph rasterizes to
zero bounds, and the window paints its boxes and borders with no text in
them at all. The app looks broken but never crashes and never logs an
error, so nothing but this check catches it.

v0.5.0 and every release before it shipped that way.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
NEEDLE = "gpui_platform"
FEATURE = "font-kit"


def main() -> int:
    problems: list[str] = []
    checked = 0
    for manifest in sorted(ROOT.glob("apps/*/Cargo.toml")):
        for number, line in enumerate(
            manifest.read_text().splitlines(), start=1
        ):
            stripped = line.strip()
            if stripped.startswith("#"):
                continue
            if not re.match(rf"^{NEEDLE}\s*=", stripped):
                continue
            checked += 1
            features = re.search(r"features\s*=\s*\[([^\]]*)\]", stripped)
            listed = (
                [item.strip().strip('"') for item in features.group(1).split(",")]
                if features
                else []
            )
            if FEATURE not in listed:
                problems.append(
                    f"{manifest.relative_to(ROOT)}:{number}: "
                    f"{NEEDLE} without features = [\"{FEATURE}\"] — "
                    "this app would render no text at all"
                )

    if checked == 0:
        print(
            f"check-desktop-fonts: found no {NEEDLE} dependency to check; "
            "the check has drifted from the manifests",
            file=sys.stderr,
        )
        return 1

    if problems:
        for problem in problems:
            print(problem, file=sys.stderr)
        return 1

    print(f"check-desktop-fonts: {checked} desktop app(s) request {FEATURE}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
