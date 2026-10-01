#!/usr/bin/env python3
"""Cut the World Machine trailer from the app's own pictures.

The trailer is made of what the app draws, never of anything painted for
it: stills and frame sequences captured from the real app (the Linux
preview's `frames N`, or the CI screenshot runner), moved slowly (a gentle
push-in or pan across each still) and joined by short cross-fades, with a
few plain title cards. The cut list is `docs/press/trailer.json`; each shot
is one of

  {"card": ["Line one", "smaller second line"], "seconds": 3}
  {"still": ..., "title": ["Line one", "smaller second line"]}
  {"still": "docs/review/v23-dusk.png", "seconds": 5,
   "from": [x0, y0, x1, y1], "to": [x0, y0, x1, y1]}
  {"frames": "film/frame-*.png", "fps": 10, "fallback": {...a still...}}

A card's words fade in and out on plain paper, so a cross-fade only ever
mixes paper with a picture, never words with a busy scene. A shot's
`title` is laid on a quiet paper band across its lower third (or across
the sky, with `"title_at": "top"`), which fades
in after the shot has settled and out before it leaves.

`from` and `to` are crops of the source picture in its own pixels (the
window's title bar is left out that way); each is widened to 16:9 around
its centre. A `frames` path is looked for under `--shots` first, then the
repository; when no frame is found the shot's `fallback` plays instead, so
the trailer can always be cut, and gets better as captures arrive.

Needs Pillow and an ffmpeg with libx264: `ffmpeg` on the PATH, or
`pip install imageio-ffmpeg` (which carries one). Deterministic: the same
pictures and cut list give the same frames.

    python3 scripts/make-trailer.py [--shots DIR] [--audio FILE] [--out FILE]
"""

from __future__ import annotations

import argparse
import glob
import json
import re
import shutil
import subprocess
import sys
from pathlib import Path

try:
    from PIL import Image, ImageDraw, ImageFont
except ModuleNotFoundError:  # pragma: no cover - depends on the machine
    print("make-trailer: needs Pillow; run `python3 -m pip install pillow`", file=sys.stderr)
    raise SystemExit(1)

ROOT = Path(__file__).resolve().parent.parent
CUT = ROOT / "docs/press/trailer.json"
WIDTH, HEIGHT, FPS = 1920, 1080, 30
FADE = 0.6  # seconds of cross-fade between shots
PAPER = (246, 243, 236)  # the app's own paper colour, for cards
INK = (38, 38, 42)
INK_SOFT = (104, 100, 96)

FONT_CANDIDATES = [
    "/System/Library/Fonts/SFNS.ttf",
    "/System/Library/Fonts/Helvetica.ttc",
    "/Library/Fonts/Arial Unicode.ttf",
    "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
    "/usr/share/fonts/truetype/freefont/FreeSans.ttf",
]


def ffmpeg_exe() -> str:
    found = shutil.which("ffmpeg")
    if found:
        return found
    try:
        import imageio_ffmpeg  # type: ignore

        return imageio_ffmpeg.get_ffmpeg_exe()
    except ModuleNotFoundError:
        print(
            "make-trailer: needs ffmpeg; install it or run `python3 -m pip install imageio-ffmpeg`",
            file=sys.stderr,
        )
        raise SystemExit(1)


def font(size: int, override: str | None) -> ImageFont.ImageFont:
    for path in ([override] if override else []) + FONT_CANDIDATES:
        if path and Path(path).is_file():
            return ImageFont.truetype(path, size)
    return ImageFont.load_default(size)


def widen(box: list[float], image: Image.Image) -> tuple[float, float, float, float]:
    """`box` widened to 16:9 around its centre, and kept inside `image`."""
    x0, y0, x1, y1 = box
    cx, cy = (x0 + x1) / 2, (y0 + y1) / 2
    w, h = x1 - x0, y1 - y0
    if w / h < WIDTH / HEIGHT:
        w = h * WIDTH / HEIGHT
    else:
        h = w * HEIGHT / WIDTH
    scale = min(1.0, image.width / w, image.height / h)
    w, h = w * scale, h * scale
    cx = min(max(cx, w / 2), image.width - w / 2)
    cy = min(max(cy, h / 2), image.height - h / 2)
    return (cx - w / 2, cy - h / 2, cx + w / 2, cy + h / 2)


def ease(t: float) -> float:
    return t * t * (3 - 2 * t)


def still_frames(shot: dict, count: int) -> list[Image.Image]:
    image = Image.open(ROOT / shot["still"]).convert("RGB")
    whole = [0, 52, image.width, image.height]  # below the title bar
    start = widen(shot.get("from", whole), image)
    end = widen(shot.get("to", shot.get("from", whole)), image)
    frames = []
    for index in range(count):
        t = ease(index / max(count - 1, 1))
        box = tuple(a + (b - a) * t for a, b in zip(start, end))
        frames.append(image.resize((WIDTH, HEIGHT), Image.LANCZOS, box=box))
    return frames


def words(
    draw: ImageDraw.ImageDraw,
    lines: list[str],
    middle: float,
    font_path: str | None,
    big_size: int,
    small_size: int,
    ink: tuple[int, int, int],
    soft: tuple[int, int, int],
) -> None:
    """`lines` centred on `middle`: the first large, the rest smaller."""
    big, small = font(big_size, font_path), font(small_size, font_path)
    heights = [big_size + round(big_size * 0.3)] + [small_size + round(small_size * 0.45)] * (
        len(lines) - 1
    )
    y = middle - sum(heights) / 2
    for index, line in enumerate(lines):
        face = big if index == 0 else small
        width = draw.textlength(line, font=face)
        draw.text(((WIDTH - width) / 2, y), line, font=face, fill=ink if index == 0 else soft)
        y += heights[index]


def ramp(index: int, count: int, rise: float, fall: float) -> float:
    """0 to 1 over `rise` seconds from the start, back to 0 over `fall`
    seconds before the end, eased."""
    t = index / FPS
    left = (count - 1 - index) / FPS
    up = min(1.0, max(0.0, t / rise)) if rise else 1.0
    down = min(1.0, max(0.0, left / fall)) if fall else 1.0
    return ease(min(up, down))


def card_frames(shot: dict, count: int, font_path: str | None) -> list[Image.Image]:
    """Words fading in and out on paper; the paper itself stays still, so
    the cross-fades either side of a card mix only paper with a picture."""
    paper = Image.new("RGB", (WIDTH, HEIGHT), tuple(shot.get("paper", PAPER)))
    full = paper.copy()
    words(ImageDraw.Draw(full), shot["card"], HEIGHT / 2, font_path, 76, 40, INK, INK_SOFT)
    # The words wait for the cross-fade in and are gone before the one out.
    lead = round((FADE + 0.15) * FPS)
    span = count - lead - round((FADE + 0.1) * FPS)
    frames = []
    for index in range(count):
        inner = index - lead
        mix = ramp(inner, span, 0.6, 0.5) if 0 <= inner < span else 0.0
        frames.append(Image.blend(paper, full, mix) if mix < 1 else full)
    return frames


# Where a title band sits, as fractions of the frame's height: across the
# lower third by default, or across the sky ("title_at": "top") when the
# lower third is busy.
BANDS = {"bottom": (0.71, 0.87), "top": (0.08, 0.24)}
BAND_ALPHA = 0.90
BAND_FEATHER = 28  # px of soft edge above and below


def band_layer(lines: list[str], at: str, font_path: str | None) -> Image.Image:
    """A paper band with `lines` on it, as an RGBA layer: the paper of the
    app's own cards, nearly opaque, with soft top and bottom edges."""
    layer = Image.new("RGBA", (WIDTH, HEIGHT), (0, 0, 0, 0))
    top, bottom = (round(HEIGHT * edge) for edge in BANDS[at])
    alpha = Image.new("L", (1, HEIGHT), 0)
    for y in range(HEIGHT):
        if top <= y < bottom:
            edge = min(y - top, bottom - 1 - y)
            a = BAND_ALPHA * ease(min(1.0, (edge + 1) / BAND_FEATHER))
            alpha.putpixel((0, y), round(255 * a))
    band = Image.new("RGBA", (WIDTH, HEIGHT), PAPER + (0,))
    band.putalpha(alpha.resize((WIDTH, HEIGHT)))
    layer = Image.alpha_composite(layer, band)
    text = Image.new("RGBA", (WIDTH, HEIGHT), (0, 0, 0, 0))
    words(ImageDraw.Draw(text), lines, (top + bottom) / 2, font_path, 54, 32, INK, INK_SOFT)
    return Image.alpha_composite(layer, text)


def with_title(
    frames: list[Image.Image], lines: list[str], at: str, font_path: str | None
) -> list[Image.Image]:
    """`frames` with a title band that fades in once the shot has settled
    (after the cross-fade into it) and out before the cross-fade out."""
    layer = band_layer(lines, at, font_path)
    settle = round((FADE + 0.2) * FPS)
    count = len(frames)
    out = []
    for index, frame in enumerate(frames):
        inner = index - settle
        span = count - 2 * settle
        mix = ramp(inner, span, 0.5, 0.5) if 0 <= inner < span else 0.0
        if mix <= 0:
            out.append(frame)
            continue
        faded = layer.copy()
        faded.putalpha(faded.getchannel("A").point(lambda a: round(a * mix)))
        out.append(Image.alpha_composite(frame.convert("RGBA"), faded).convert("RGB"))
    return out


def natural(path: str) -> list:
    """frame-2 before frame-10."""
    return [int(part) if part.isdigit() else part for part in re.split(r"(\d+)", path)]


def sequence_frames(shot: dict, shots_dir: Path | None) -> list[Image.Image] | None:
    pattern = shot["frames"]
    paths: list[str] = []
    for base in [shots_dir, ROOT]:
        if base is not None:
            paths = sorted(glob.glob(str(base / pattern)), key=natural)
            if paths:
                break
    if not paths:
        return None
    source_fps = float(shot.get("fps", 10))
    pictures = [Image.open(path).convert("RGB") for path in paths]
    crop = shot.get("crop")
    out = []
    count = int(len(pictures) / source_fps * FPS)
    for index in range(count):
        at = index * source_fps / FPS
        left = min(int(at), len(pictures) - 1)
        right = min(left + 1, len(pictures) - 1)
        mix = at - int(at)
        frame = Image.blend(pictures[left], pictures[right], mix) if mix else pictures[left]
        whole = [0, 52, frame.width, frame.height]
        box = widen(crop or whole, frame)
        out.append(frame.resize((WIDTH, HEIGHT), Image.LANCZOS, box=box))
    return out


def shot_frames(shot: dict, shots_dir: Path | None, font_path: str | None) -> list[Image.Image]:
    title = shot.get("title")
    at = shot.get("title_at", "bottom")
    frames = None
    if "frames" in shot:
        frames = sequence_frames(shot, shots_dir)
        if not frames:
            print(f"make-trailer: no frames for {shot['frames']}; using its fallback", file=sys.stderr)
            shot = shot["fallback"]
    if frames is None:
        count = round(float(shot["seconds"]) * FPS)
        if "card" in shot:
            return card_frames(shot, count, font_path)
        frames = still_frames(shot, count)
    if title:
        frames = with_title(frames, title, at, font_path)
    return frames


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--cut", type=Path, default=CUT)
    parser.add_argument("--shots", type=Path, help="where captured frame sequences are")
    parser.add_argument("--audio", type=Path, help="a sound track to lay under the picture")
    parser.add_argument("--font", help="a .ttf/.ttc for the cards")
    parser.add_argument("--out", type=Path, default=ROOT / "target/trailer/world-machine-trailer.mp4")
    args = parser.parse_args()

    cut = json.loads(args.cut.read_text())
    args.out.parent.mkdir(parents=True, exist_ok=True)
    command = [
        ffmpeg_exe(), "-y", "-loglevel", "error",
        "-f", "rawvideo", "-pix_fmt", "rgb24", "-s", f"{WIDTH}x{HEIGHT}", "-r", str(FPS),
        "-i", "-",
    ]
    if args.audio:
        command += ["-i", str(args.audio), "-shortest", "-c:a", "aac", "-b:a", "160k"]
    command += [
        "-c:v", "libx264", "-preset", "slow", "-crf", "18", "-pix_fmt", "yuv420p",
        "-movflags", "+faststart", str(args.out),
    ]
    encoder = subprocess.Popen(command, stdin=subprocess.PIPE)
    assert encoder.stdin is not None

    fade = round(FADE * FPS)
    tail: list[Image.Image] = []
    written = 0
    for number, shot in enumerate(cut["shots"]):
        frames = shot_frames(shot, args.shots, args.font)
        # Cross-fade the last `fade` frames of the previous shot into the
        # first of this one; the first shot fades up from paper.
        if not tail:
            tail = [Image.new("RGB", (WIDTH, HEIGHT), PAPER)] * fade
        overlap = min(fade, len(tail), len(frames))
        for index in range(overlap):
            mix = (index + 1) / (overlap + 1)
            encoder.stdin.write(Image.blend(tail[-overlap + index], frames[index], mix).tobytes())
            written += 1
        body = frames[overlap:]
        keep = min(fade, len(body))
        for frame in body[: len(body) - keep]:
            encoder.stdin.write(frame.tobytes())
            written += 1
        tail = body[len(body) - keep :]
        print(f"shot {number + 1}: {len(frames) / FPS:.1f} s", file=sys.stderr)
    for index, frame in enumerate(tail):
        mix = (index + 1) / (len(tail) + 1)
        encoder.stdin.write(Image.blend(frame, Image.new("RGB", frame.size, PAPER), mix).tobytes())
        written += 1
    encoder.stdin.close()
    if encoder.wait() != 0:
        print("make-trailer: ffmpeg failed", file=sys.stderr)
        return 1
    print(f"{args.out} ({written / FPS:.1f} s, {WIDTH}x{HEIGHT} at {FPS} fps)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
