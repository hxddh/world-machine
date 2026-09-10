#!/usr/bin/env python3
"""The Packs the app ships must agree about who they are.

Three places name the included World Packs and they cannot see each other:

  * `worlds/<pack>/src/persistence.rs` holds the Pack id and version that end
    up in the bundle's manifest;
  * `apps/world-machine-desktop/src/included_packs.rs` declares what the app
    expects to find, and `review_pack_path` refuses to install a bundle whose
    manifest disagrees;
  * `apps/world-machine-desktop/macos/build-app.sh` decides which bundles are
    written into the app at all.

A drift between the first two makes a shipped Pack uninstallable, and only on a
real Mac at install time. A drift between the second and third makes a Pack
either invisible or a dangling entry. This check keeps them honest.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
INCLUDED_PACKS = ROOT / "apps/world-machine-desktop/src/included_packs.rs"
BUILD_APP = ROOT / "apps/world-machine-desktop/macos/build-app.sh"

# Which world crate is the source of truth for each Pack id. The constants sit
# in different files per crate, so the whole crate is searched.
PACK_SOURCES = {
    "world-machine.pocket-universe": ROOT / "worlds/pocket-universe/src",
    "world-machine.micro-company": ROOT / "worlds/micro-company/src",
    "world-machine.tiny-society": ROOT / "worlds/tiny-society/src",
}

SPEC = re.compile(
    r"IncludedPackSpec\s*\{(?P<body>.*?)\}", re.DOTALL
)
FIELD = re.compile(r'(?P<key>\w+)\s*:\s*"(?P<value>[^"]*)"')


def declared_specs() -> list[dict[str, str]]:
    source = INCLUDED_PACKS.read_text()
    start = source.index("const INCLUDED_PACKS:")
    end = source.index("\n];", start)
    return [
        {m.group("key"): m.group("value") for m in FIELD.finditer(spec.group("body"))}
        for spec in SPEC.finditer(source[start:end])
    ]


def pack_identity(crate: Path) -> tuple[str, str]:
    for source_file in sorted(crate.rglob("*.rs")):
        source = source_file.read_text()
        pack_id = re.search(r'PACK_ID: &str = "([^"]+)"', source)
        version = re.search(r'PACK_VERSION: &str = "([^"]+)"', source)
        if pack_id and version:
            return pack_id.group(1), version.group(1)
    raise SystemExit(f"could not read a Pack id and version anywhere under {crate}")


def bundles_written_by_build() -> set[str]:
    """The bundle file names build-app.sh writes into the app."""
    source = BUILD_APP.read_text()
    block = re.search(r"INCLUDED_PACK_NAMES=\((?P<body>[^)]*)\)", source)
    if block is None:
        raise SystemExit(f"could not find INCLUDED_PACK_NAMES in {BUILD_APP}")
    return {
        f"{name}.worldpack"
        for name in block.group("body").split()
        if name and not name.startswith("#")
    }


def main() -> int:
    problems: list[str] = []
    specs = declared_specs()
    if not specs:
        problems.append(f"no IncludedPackSpec entries found in {INCLUDED_PACKS}")

    for spec in specs:
        pack_id = spec.get("id", "")
        source = PACK_SOURCES.get(pack_id)
        if source is None:
            problems.append(
                f"{pack_id} is bundled by the app but this check does not know "
                f"which world crate owns it; add it to PACK_SOURCES"
            )
            continue
        actual_id, actual_version = pack_identity(source)
        if (actual_id, actual_version) != (pack_id, spec.get("version", "")):
            problems.append(
                f"{INCLUDED_PACKS.name} declares {pack_id} @ {spec.get('version')}, "
                f"but {source.relative_to(ROOT)} builds {actual_id} @ {actual_version}. "
                f"The app would refuse to install its own bundled Pack."
            )

    declared_files = {spec.get("file_name", "") for spec in specs}
    built_files = bundles_written_by_build()
    for missing in sorted(declared_files - built_files):
        problems.append(
            f"{INCLUDED_PACKS.name} expects {missing}, but build-app.sh never writes it"
        )
    for extra in sorted(built_files - declared_files):
        problems.append(
            f"build-app.sh writes {extra}, but {INCLUDED_PACKS.name} never lists it, "
            f"so the app would ignore it"
        )

    if problems:
        for problem in problems:
            print(f"included Pack check failed: {problem}", file=sys.stderr)
        return 1

    print(f"Included Pack check passed ({len(specs)} Packs).")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
