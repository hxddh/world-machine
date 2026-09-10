#!/usr/bin/env python3
"""Draws the World Machine app icon and writes it as a PNG.

One stroke. It begins at the centre, thin and pale, and spirals outward,
thickening and warming as it goes: a World accumulating its own history,
the oldest of it small and far back, the present broad and warm at the
open end. Nothing else is on the tile.

The first icon this app shipped was a node with two lines to two smaller
nodes, which is the share glyph every platform already uses, and it said
nothing this app does. A spiral is not a glyph anyone else is using and
it is the one shape that means "this kept going while you were away".

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
# every side, and the corner is a squircle rather than a circular arc — a
# plain rounded rectangle reads as slightly wrong next to every neighbour
# in the Dock.
INSET = 92.0
SQUIRCLE_EXPONENT = 5.0

GROUND_INNER = (0x2B, 0x27, 0x40)
GROUND_OUTER = (0x10, 0x0F, 0x19)
OLDEST = (0xF3, 0xEE, 0xE1)
NEWEST = (0xE0, 0xA0, 0x54)

TURNS = 1.30
RADIUS_START, RADIUS_END = 54.0, 338.0
WIDTH_START, WIDTH_END = 18.0, 68.0

CENTRE = SIZE / 2.0
TWO_PI = math.pi * 2.0
SWEEP = TURNS * TWO_PI


def coverage(distance: float) -> float:
    """Antialiased coverage for a signed distance, one pixel wide."""
    return min(1.0, max(0.0, 0.5 - distance))


def mix(a, b, t: float):
    return tuple(a[i] + (b[i] - a[i]) * t for i in range(3))


def smooth(t: float) -> float:
    t = min(1.0, max(0.0, t))
    return t * t * (3.0 - 2.0 * t)


def squircle_distance(x: float, y: float) -> float:
    """Signed distance to the icon's rounded square, negative inside."""
    half = (SIZE - 2 * INSET) / 2.0
    u = abs(x - CENTRE) / half
    v = abs(y - CENTRE) / half
    generalised = (u**SQUIRCLE_EXPONENT + v**SQUIRCLE_EXPONENT) ** (
        1.0 / SQUIRCLE_EXPONENT
    )
    return (generalised - 1.0) * half


def radius_at(fraction: float) -> float:
    return RADIUS_START + (RADIUS_END - RADIUS_START) * fraction


def width_at(fraction: float) -> float:
    return WIDTH_START + (WIDTH_END - WIDTH_START) * fraction


def point_at(fraction: float, offset_x: float, offset_y: float):
    angle = fraction * SWEEP
    radius = radius_at(fraction)
    return (
        CENTRE + offset_x + radius * math.cos(angle),
        CENTRE + offset_y + radius * math.sin(angle),
    )


def centring_offset():
    """Shift the stroke so its own bounding box sits centred on the tile.

    A spiral opens to one side, so drawing it about the tile's centre
    leaves it visibly heavier on that side.
    """
    low_x = low_y = float("inf")
    high_x = high_y = float("-inf")
    for step in range(241):
        fraction = step / 240
        x, y = point_at(fraction, 0.0, 0.0)
        reach = width_at(fraction) / 2.0
        low_x, high_x = min(low_x, x - reach), max(high_x, x + reach)
        low_y, high_y = min(low_y, y - reach), max(high_y, y + reach)
    return CENTRE - (low_x + high_x) / 2.0, CENTRE - (low_y + high_y) / 2.0


OFFSET_X, OFFSET_Y = centring_offset()


def stroke_at(x: float, y: float):
    """Signed distance to the spiral, and how far along it that point is.

    The spiral is one-to-one in angle, so the nearest point is found by
    reading the pixel's own angle and trying each turn. The endpoints are
    measured as discs, which is what gives the stroke its round caps.
    """
    dx = x - (CENTRE + OFFSET_X)
    dy = y - (CENTRE + OFFSET_Y)
    radius = math.hypot(dx, dy)
    angle = math.atan2(dy, dx)
    if angle < 0.0:
        angle += TWO_PI

    best = None
    best_fraction = 0.0
    for turn in range(-1, 3):
        fraction = (angle + turn * TWO_PI) / SWEEP
        if not 0.0 <= fraction <= 1.0:
            continue
        distance = abs(radius - radius_at(fraction)) - width_at(fraction) / 2.0
        if best is None or distance < best:
            best, best_fraction = distance, fraction

    for fraction in (0.0, 1.0):
        cap_x, cap_y = point_at(fraction, OFFSET_X, OFFSET_Y)
        distance = math.hypot(x - cap_x, y - cap_y) - width_at(fraction) / 2.0
        if best is None or distance < best:
            best, best_fraction = distance, fraction

    return best, best_fraction


def render() -> bytearray:
    rows = bytearray()
    for y in range(SIZE):
        sample_y = y + 0.5
        rows.append(0)  # PNG filter type 0 for this scanline
        for x in range(SIZE):
            sample_x = x + 0.5
            alpha = coverage(squircle_distance(sample_x, sample_y))
            if alpha <= 0.0:
                rows.extend((0, 0, 0, 0))
                continue

            from_centre = math.hypot(sample_x - CENTRE, sample_y - CENTRE)
            pixel = mix(
                GROUND_INNER,
                GROUND_OUTER,
                min(1.0, (from_centre / (SIZE * 0.58)) ** 1.25),
            )

            distance, fraction = stroke_at(sample_x, sample_y)
            covered = coverage(distance)
            if covered > 0.0:
                pixel = mix(pixel, mix(OLDEST, NEWEST, smooth(fraction)), covered)

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
