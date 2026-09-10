#!/usr/bin/env python3
"""Draws the World Machine app icon and writes it as a PNG.

The icon is a lineage: one world, and the two worlds branched from it.
That is the single idea the app is built around, and it is the one shape
that still reads at 32 pixels in the Dock.

Everything is drawn analytically from signed distances, so the same
source produces the same file on any machine with a stdlib Python. Run

    python3 scripts/render_app_icon.py apps/world-machine-desktop/macos/icon.png

to regenerate it; scripts/check-app-icon.py checks the committed PNG
still matches what this script draws.
"""

from __future__ import annotations

import math
import struct
import sys
import zlib
from pathlib import Path

SIZE = 1024

# macOS draws app icons inside a rounded square that leaves a margin on
# every side, so a bare 1024x1024 square would sit visibly larger than
# every neighbour in the Dock.
INSET = 100.0
RADIUS = 185.0

GROUND_TOP = (0x2B, 0x38, 0x66)
GROUND_BOTTOM = (0x14, 0x1A, 0x33)
PARENT = (0xF7, 0xF7, 0xF3)
CHILD = (0xD9, 0x8C, 0x5F)
LINK = (0x6E, 0x7A, 0xA8)

PARENT_CENTER = (352.0, 512.0)
PARENT_RADIUS = 118.0
CHILDREN = ((712.0, 316.0), (712.0, 708.0))
CHILD_RADIUS = 74.0
LINK_WIDTH = 30.0


def rounded_square_distance(x: float, y: float) -> float:
    """Signed distance to the rounded square, negative inside."""
    half = (SIZE - 2 * INSET) / 2.0
    center = SIZE / 2.0
    dx = abs(x - center) - (half - RADIUS)
    dy = abs(y - center) - (half - RADIUS)
    outside = math.hypot(max(dx, 0.0), max(dy, 0.0))
    inside = min(max(dx, dy), 0.0)
    return outside + inside - RADIUS


def circle_distance(x: float, y: float, center, radius: float) -> float:
    return math.hypot(x - center[0], y - center[1]) - radius


def segment_distance(x: float, y: float, a, b, width: float) -> float:
    ax, ay = a
    bx, by = b
    vx, vy = bx - ax, by - ay
    length_squared = vx * vx + vy * vy
    t = 0.0 if length_squared == 0 else ((x - ax) * vx + (y - ay) * vy) / length_squared
    t = min(1.0, max(0.0, t))
    return math.hypot(x - (ax + vx * t), y - (ay + vy * t)) - width / 2.0


def coverage(distance: float) -> float:
    """Antialiased coverage for a signed distance, one pixel wide."""
    return min(1.0, max(0.0, 0.5 - distance))


def over(base, colour, alpha: float):
    return tuple(base[i] + (colour[i] - base[i]) * alpha for i in range(3))


def render() -> bytearray:
    rows = bytearray()
    for y in range(SIZE):
        sample_y = y + 0.5
        ground_mix = sample_y / SIZE
        ground = tuple(
            GROUND_TOP[i] + (GROUND_BOTTOM[i] - GROUND_TOP[i]) * ground_mix
            for i in range(3)
        )
        rows.append(0)  # PNG filter type 0 for this scanline
        for x in range(SIZE):
            sample_x = x + 0.5
            alpha = coverage(rounded_square_distance(sample_x, sample_y))
            if alpha <= 0.0:
                rows.extend((0, 0, 0, 0))
                continue

            pixel = ground
            for child in CHILDREN:
                pixel = over(
                    pixel,
                    LINK,
                    coverage(
                        segment_distance(
                            sample_x, sample_y, PARENT_CENTER, child, LINK_WIDTH
                        )
                    ),
                )
            for child in CHILDREN:
                pixel = over(
                    pixel,
                    CHILD,
                    coverage(circle_distance(sample_x, sample_y, child, CHILD_RADIUS)),
                )
            pixel = over(
                pixel,
                PARENT,
                coverage(
                    circle_distance(sample_x, sample_y, PARENT_CENTER, PARENT_RADIUS)
                ),
            )

            rows.extend(
                (
                    int(pixel[0] + 0.5),
                    int(pixel[1] + 0.5),
                    int(pixel[2] + 0.5),
                    int(alpha * 255 + 0.5),
                )
            )
    return rows


def chunk(kind: bytes, payload: bytes) -> bytes:
    return (
        struct.pack(">I", len(payload))
        + kind
        + payload
        + struct.pack(">I", zlib.crc32(kind + payload) & 0xFFFFFFFF)
    )


def png(rows: bytearray) -> bytes:
    header = struct.pack(">IIBBBBB", SIZE, SIZE, 8, 6, 0, 0, 0)
    return (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", header)
        + chunk(b"IDAT", zlib.compress(bytes(rows), 9))
        + chunk(b"IEND", b"")
    )


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: render_app_icon.py <output.png>", file=sys.stderr)
        return 2
    Path(sys.argv[1]).write_bytes(png(render()))
    return 0


if __name__ == "__main__":
    sys.exit(main())
