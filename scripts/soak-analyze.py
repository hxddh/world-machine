#!/usr/bin/env python3
"""Judges a soak run (scripts/soak.sh): memory, frames, no growth.

Reads, from the run's output folder:
- rss.tsv       seconds since start, the app's resident memory in kB, and
                (a third column, optional) the kB of it that is the
                renderer's graphics memory;
- app.log       the app's output, with its frame log every 2 s
                ("frame: N frames, mean X ms, longest Y ms; ...") and any
                panic;
- frames.log    a line for every frame the window drew (WORLD_GPUI_FRAME_LOG:
                ms, window, unpainted pictures, loading wash, ...);
- sittings.tsv  seconds since start, the World's sittings, and World days
                passed in the latest one.

It fails the run (exit 1) on:
- a panic in the app's output, or the app no longer running at the end;
- any frame of the World window with something unpainted (or under the
  loading wash) after its first painted frame;
- the app's own memory (RSS less the renderer's graphics memory) growing
  after warm-up faster than --rss-slope MB an hour (a least squares line
  through the samples after warm-up), or the whole RSS, graphics memory
  and all, ending more than --rss-growth MB above where warm-up left it.
  Graphics memory is left out of the slope only: under Xvfb the software
  renderer's video memory is resident in the process and is taken in
  32 MB blocks that are kept once taken (a high-water mark, at random
  moments), which on a real GPU is video memory, not the app's RAM; it
  is still in the growth bound, and is reported;
- the frame time's 95th percentile (of the 2 s means) in the last third of
  the run after warm-up more than --p95-ratio times, and --p95-slack ms
  over, the first third's; or any 2 s window whose mean passes
  --frame-ceiling ms;
- fewer sittings than --least-sittings-per-hour an hour: a driver that
  stopped moving the World is not soaking it;
- a run that stopped making progress before its end, alive but frozen: the
  World window's last frame more than --stale-minutes before the run's end,
  or no new sitting in the last --stall-minutes (or a quarter of the run,
  whichever is longer).

It writes summary.json and prints a summary.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

FRAME = re.compile(r"frame: (\d+) frames, mean ([\d.]+) ms, longest ([\d.]+) ms")


def percentile(values: list[float], fraction: float) -> float:
    if not values:
        return 0.0
    ordered = sorted(values)
    index = min(len(ordered) - 1, max(0, round(fraction * (len(ordered) - 1))))
    return ordered[index]


def slope(points: list[tuple[float, float]]) -> float:
    """Least squares slope of y over x."""
    if len(points) < 2:
        return 0.0
    n = len(points)
    mean_x = sum(x for x, _ in points) / n
    mean_y = sum(y for _, y in points) / n
    var = sum((x - mean_x) ** 2 for x, _ in points)
    if var == 0:
        return 0.0
    return sum((x - mean_x) * (y - mean_y) for x, y in points) / var


def read_rows(path: Path) -> list[list[str]]:
    if not path.exists():
        return []
    return [line.split() for line in path.read_text(errors="replace").splitlines() if line.strip()]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("out", type=Path)
    parser.add_argument("--minutes", type=float, required=True)
    parser.add_argument("--warmup-minutes", type=float, required=True)
    parser.add_argument("--alive", type=int, default=1, help="1 if the app was running at the end")
    parser.add_argument("--rss-slope", type=float, default=60.0, help="MB an hour after warm-up")
    parser.add_argument("--rss-growth", type=float, default=200.0, help="MB over the warm RSS at the end")
    parser.add_argument("--p95-ratio", type=float, default=1.5)
    parser.add_argument("--p95-slack", type=float, default=2.0, help="ms")
    parser.add_argument("--frame-ceiling", type=float, default=50.0, help="ms, any 2 s mean")
    parser.add_argument("--least-sittings-per-hour", type=float, default=30.0)
    parser.add_argument("--stale-minutes", type=float, default=2.0, help="last frame before the end")
    parser.add_argument("--stall-minutes", type=float, default=10.0, help="last new sitting before the end")
    args = parser.parse_args()
    out: Path = args.out
    warmup = args.warmup_minutes * 60
    problems: list[str] = []

    # Panics, and the app's life.
    log = (out / "app.log").read_text(errors="replace") if (out / "app.log").exists() else ""
    panics = [line for line in log.splitlines() if "panicked at" in line]
    if panics:
        problems.append(f"{len(panics)} panic(s): {panics[0].strip()}")
    if not args.alive:
        problems.append("the app was not running at the end of the run")

    # Memory.
    # Each sample: seconds, the whole RSS and the graphics memory in it (MB).
    samples = [
        (float(row[0]), float(row[1]) / 1024.0, float(row[2]) / 1024.0 if len(row) >= 3 else 0.0)
        for row in read_rows(out / "rss.tsv")
        if len(row) >= 2
    ]
    rss = [(t, mb) for t, mb, _ in samples]
    warm = [(t, mb) for t, mb in rss if t >= warmup]
    warm_samples = [sample for sample in samples if sample[0] >= warmup]
    own_slope = slope([(t / 3600.0, mb - gpu) for t, mb, gpu in warm_samples])
    rss_slope = slope([(t / 3600.0, mb) for t, mb in warm])
    gpu_start = warm_samples[0][2] if warm_samples else 0.0
    gpu_end = warm_samples[-1][2] if warm_samples else 0.0
    rss_start = warm[0][1] if warm else 0.0
    rss_end = warm[-1][1] if warm else 0.0
    rss_peak = max((mb for _, mb in rss), default=0.0)
    if len(warm) < 10:
        problems.append(f"only {len(warm)} RSS samples after warm-up")
    else:
        if own_slope > args.rss_slope:
            problems.append(
                f"the app's own memory grows {own_slope:.1f} MB/h after warm-up "
                f"(bound {args.rss_slope:.0f}; RSS with graphics memory {rss_slope:.1f})"
            )
        if rss_end - rss_start > args.rss_growth:
            problems.append(
                f"RSS ended {rss_end - rss_start:.0f} MB above the warm {rss_start:.0f} MB "
                f"(bound {args.rss_growth:.0f})"
            )

    # Frame time: the 2 s windows of the frame log, in order, timed by
    # where they fall among the frames drawn (the log has no clock of its
    # own; they come every 2 s from the first frame).
    windows = [(int(m.group(1)), float(m.group(2)), float(m.group(3))) for m in FRAME.finditer(log)]
    warm_windows = windows[int(warmup / 2) :] if len(windows) > warmup / 2 else []
    third = len(warm_windows) // 3
    first = [mean for _, mean, _ in warm_windows[:third]]
    last = [mean for _, mean, _ in warm_windows[-third:]] if third else []
    p95_first, p95_last = percentile(first, 0.95), percentile(last, 0.95)
    longest = max((longest for _, _, longest in windows), default=0.0)
    worst_mean = max((mean for _, mean, _ in warm_windows), default=0.0)
    if third < 5:
        problems.append(f"too few frame-time windows after warm-up ({len(warm_windows)})")
    else:
        if p95_last > p95_first * args.p95_ratio + args.p95_slack:
            problems.append(
                f"frame time p95 regressed from {p95_first:.2f} ms to {p95_last:.2f} ms "
                f"(bound x{args.p95_ratio} + {args.p95_slack} ms)"
            )
        if worst_mean > args.frame_ceiling:
            problems.append(f"a 2 s window averaged {worst_mean:.1f} ms a frame (ceiling {args.frame_ceiling:.0f})")

    # Unpainted frames, after the World window's first painted one.
    rows = [row for row in read_rows(out / "frames.log") if len(row) >= 4]
    counts: dict[str, int] = {}
    for row in rows:
        counts[row[1]] = counts.get(row[1], 0) + 1
    main_window = max(counts, key=counts.get) if counts else None
    own = [row for row in rows if row[1] == main_window]
    first_painted = next((int(r[0]) for r in own if int(r[2]) == 0 and float(r[3]) == 0), None)
    unpainted = [r for r in own if first_painted is not None and int(r[0]) > first_painted and (int(r[2]) > 0 or float(r[3]) > 0)]
    if first_painted is None:
        problems.append("the World window never painted")
    if unpainted:
        at = ", ".join(f"{int(r[0]) / 1000:.1f} s" for r in unpainted[:5])
        problems.append(f"{len(unpainted)} unpainted frame(s) after the first painted one (at {at})")

    # The World moved.
    sittings = [row for row in read_rows(out / "sittings.tsv") if len(row) >= 2 and row[1].isdigit()]
    reached = int(sittings[-1][1]) if sittings else 0
    hours = args.minutes / 60.0
    if reached < args.least_sittings_per_hour * hours:
        problems.append(
            f"only {reached} sittings in {args.minutes:.0f} min "
            f"(at least {args.least_sittings_per_hour:.0f} an hour)"
        )

    # Progress to the end: an app alive but frozen draws no more frames and
    # moves no more sittings, however steady its memory.
    run_s = args.minutes * 60.0
    last_frame_s = max((int(r[0]) / 1000.0 for r in own), default=0.0)
    if own and last_frame_s < run_s - args.stale_minutes * 60.0:
        problems.append(
            f"the World window's last frame was at {last_frame_s / 60:.1f} min of a "
            f"{args.minutes:.0f} min run (within {args.stale_minutes:.0f} min of its end wanted)"
        )
    last_rise_s = 0.0
    for before, after in zip(sittings, sittings[1:]):
        if int(after[1]) > int(before[1]):
            last_rise_s = float(after[0])
    stall_s = max(args.stall_minutes * 60.0, run_s / 4.0)
    if sittings and run_s > stall_s and last_rise_s < run_s - stall_s:
        problems.append(
            f"no new sitting after {last_rise_s / 60:.1f} min of a {args.minutes:.0f} min run "
            f"(one within the last {stall_s / 60:.0f} min wanted)"
        )

    summary = {
        "minutes": args.minutes,
        "warmup_minutes": args.warmup_minutes,
        "sittings": reached,
        "frames": len(own),
        "rss_samples": len(rss),
        "rss_warm_mb": round(rss_start, 1),
        "rss_end_mb": round(rss_end, 1),
        "rss_peak_mb": round(rss_peak, 1),
        "rss_slope_mb_per_hour": round(rss_slope, 2),
        "own_slope_mb_per_hour": round(own_slope, 2),
        "gpu_warm_mb": round(gpu_start, 1),
        "gpu_end_mb": round(gpu_end, 1),
        "frame_windows": len(windows),
        "frame_mean_p95_first_ms": round(p95_first, 2),
        "frame_mean_p95_last_ms": round(p95_last, 2),
        "frame_longest_ms": round(longest, 2),
        "unpainted_frames": len(unpainted),
        "last_frame_min": round(last_frame_s / 60.0, 2),
        "last_new_sitting_min": round(last_rise_s / 60.0, 2),
        "panics": len(panics),
        "problems": problems,
    }
    (out / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
    print("== soak")
    for key, value in summary.items():
        if key != "problems":
            print(f"  {key}: {value}")
    for problem in problems:
        print(f"  FAILED: {problem}")
    print("  ok" if not problems else "  the soak failed")
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
