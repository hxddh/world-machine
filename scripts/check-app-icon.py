#!/usr/bin/env python3
"""The bundled app icon has a source, and the bundle actually installs it.

Three things have to agree or the app ships with the blank generic-
application icon again: the committed PNG must be what
scripts/render_app_icon.py draws, the Info.plist template must name
AppIcon, and build-app.sh must write Resources/AppIcon.icns.

Pixels are compared rather than file bytes, so a different zlib does not
fail the check.
"""

from __future__ import annotations

import importlib.util
import struct
import sys
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ICON = ROOT / "apps/world-machine-desktop/macos/icon.png"
PLIST = ROOT / "apps/world-machine-desktop/macos/Info.plist.in"
BUILD = ROOT / "apps/world-machine-desktop/macos/build-app.sh"
RENDERER = ROOT / "scripts/render_app_icon.py"


def load_renderer():
    sys.dont_write_bytecode = True
    spec = importlib.util.spec_from_file_location("render_app_icon", RENDERER)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def png_pixels(data: bytes) -> tuple[int, int, bytes]:
    assert data[:8] == b"\x89PNG\r\n\x1a\n", "not a PNG"
    width = height = None
    compressed = b""
    offset = 8
    while offset < len(data):
        (length,) = struct.unpack(">I", data[offset : offset + 4])
        kind = data[offset + 4 : offset + 8]
        payload = data[offset + 8 : offset + 8 + length]
        if kind == b"IHDR":
            width, height, depth, colour = struct.unpack(">IIBB", payload[:10])
            assert (depth, colour) == (8, 6), (depth, colour)
        elif kind == b"IDAT":
            compressed += payload
        offset += 12 + length
    return width, height, zlib.decompress(compressed)


def main() -> int:
    problems: list[str] = []

    if not ICON.is_file() or ICON.stat().st_size == 0:
        print(f"{ICON.relative_to(ROOT)} is missing or empty", file=sys.stderr)
        return 1

    renderer = load_renderer()
    width, height, stored = png_pixels(ICON.read_bytes())
    if (width, height) != (renderer.SIZE, renderer.SIZE):
        problems.append(
            f"{ICON.relative_to(ROOT)} is {width}x{height}, "
            f"expected {renderer.SIZE}x{renderer.SIZE}"
        )
    elif bytes(renderer.render()) != stored:
        problems.append(
            f"{ICON.relative_to(ROOT)} does not match what "
            "scripts/render_app_icon.py draws; regenerate it with\n"
            "    python3 scripts/render_app_icon.py "
            f"{ICON.relative_to(ROOT)}"
        )

    plist = PLIST.read_text()
    for key in ("CFBundleIconFile", "CFBundleIconName"):
        if f"<key>{key}</key>" not in plist:
            problems.append(f"{PLIST.relative_to(ROOT)} declares no {key}")
    if "<string>AppIcon</string>" not in plist:
        problems.append(f"{PLIST.relative_to(ROOT)} does not name the AppIcon icon")

    # Comments explain the icon at length, so only the commands count.
    build = "\n".join(
        line
        for line in BUILD.read_text().splitlines()
        if not line.lstrip().startswith("#")
    )
    for needle in ("icon.png", "iconutil", "AppIcon.icns"):
        if needle not in build:
            problems.append(
                f"{BUILD.relative_to(ROOT)} never mentions {needle}, so the "
                "bundle would carry no icon"
            )

    if problems:
        for problem in problems:
            print(problem, file=sys.stderr)
        return 1

    print("check-app-icon: the icon source, the plist and the bundle agree")
    return 0


if __name__ == "__main__":
    sys.exit(main())
