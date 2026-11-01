# Testing beyond the unit tests

Most of what the app promises is held by `cargo test --workspace` and by
CI (`.github/workflows/ci.yml`). This page is for the runs that are too
long or too heavy for every pull request: what they drive, what they
measure, and the bounds they fail on. `scripts/check-workflows.py` keeps the
workflows honest about them.

## The nightly (`.github/workflows/nightly.yml`)

Every night at 03:17 UTC, and by hand (`gh workflow run nightly.yml --ref
main`). No release is published from a commit whose nightly is not green
(`release-package.yml`, `nightly-gate`), and a red scheduled night opens an
issue (`report-red-night`).

- **long-bars**: every `#[ignore]`d bar and the rest of the workspace, in
  release.
- **timing-bars**: the frame, snapshot and turn budgets, in release.
- **soak**: two hours in the real window (below).

## The soak (1.0 P7: memory, frames, no growth)

`scripts/soak.sh` builds the desktop app in release (`--features
linux-window`), starts it under Xvfb exactly as the release shots do
(`scripts/release-shots-lib.sh`), and drives a new Tiny Society World for
`SOAK_MINUTES` (120 in the nightly) with Settings' "Play freely" on:

- **Enter**, at a player's pace (every 2.5 s): each card's first answer
  (works go up in scaffolding), each day passed, each Goodnight answered
  with "Sleep now", each morning begun. So the World goes through many
  sittings, a World day each.
- Every sixth round, in turn: **talk** (a click where someone may stand on
  the place, Enter to their words, "how are you" said, put away), the
  **hands** (building) opened and put away, the **camera** zoomed out and
  back in, and the **drawer** opened and closed.
- If no new sitting has begun for two minutes, Escape twice, to put away
  whatever Enter does not answer.

While it plays it samples the app's resident memory (`/proc/<pid>/status`,
every 5 s), the World window's frame logs (`WORLD_GPUI_FRAME_LOG`: every
frame, with its unpainted pictures and loading wash; `WORLD_MACHINE_FRAME_LOG`:
every 2 s, the scene's mean and longest frame time on the window's thread),
and the World's sitting ledger from its file. `scripts/soak-analyze.py`
judges the run and writes `summary.json`; the nightly keeps the whole folder
(samples, logs, a picture every 60 rounds) as the `soak` artifact.

The first sixth of the run (2 to 15 minutes) is **warm-up**: the app
loads its looks, fonts and sounds, and memory settles. The run fails on:

| What | Bound (soak-analyze.py flag) |
| --- | --- |
| A panic in the app's output, or the app not running at the end | none allowed |
| A frame of the World window unpainted, or under the loading wash, after its first painted frame | none allowed |
| RSS growth after warm-up, least-squares slope | 60 MB an hour (`--rss-slope`) |
| RSS at the end above RSS when warm-up ended | 200 MB (`--rss-growth`) |
| Frame time p95 (of the 2 s means), last third after warm-up against the first third | at most 1.5 x + 2 ms (`--p95-ratio`, `--p95-slack`) |
| Any 2 s window's mean frame time | 50 ms (`--frame-ceiling`) |
| Sittings reached | at least 30 an hour (`--least-sittings-per-hour`): a stuck driver is not a soak |
| Progress to the end: the World window's last frame, and the last new sitting | within 2 min of the end (`--stale-minutes`); within the last 10 min or quarter of the run, whichever is longer (`--stall-minutes`): an app alive but frozen is not a soak |

The World itself grows as it lives (each day's events are its history), so
memory is allowed a gentle slope; a leak in the window, the looks or the
sound grows much faster than one World day every half minute can explain.
The renderer is a software one under Xvfb, so the frame bounds compare the
run with itself rather than with a real screen's budget.

Run it locally (the same tools as `scripts/release-shots.sh`):

```bash
SOAK_MINUTES=5 scripts/soak.sh target/soak/out
```

`SKIP_BUILD=1` reuses the last release build; `BIN=<dir>` runs another
release directory; `SOAK_DISPLAY` picks a free X display (default 93).

The first local run of the short mode (5 minutes, 2 of warm-up, Linux
under Xvfb with the software renderer) reached 11 sittings (day 16); RSS
was 542 MB when warm-up ended, 486 MB at the end (peak 571 MB, slope
negative); the frame time's p95 of the 2 s means went from 0.67 ms to
2.11 ms (bound 3.0 ms), longest frame 81 ms; no panic. It **failed** on 8
unpainted frames, about 100 to 300 ms after zooming out and back in: after
a day passed, the old whole-place look was let go once its successor had
baked but before it reached the display. Since 1.0 P5 a whole look is kept
until its successor is ready, and a six-minute run finds none. A short run
still fails the RSS slope, on `main` before P5 too: two minutes of warm-up
are too few to judge memory, which is what the two hours are for.

The first two-hour run (the nightly on `79083de`, 490 sittings) failed on
memory alone: RSS 589 MB when warm-up ended, 1,044 MB at the end, 253 MB
an hour. Nothing the app holds was growing: counting every picture the
window paints showed them bounded (under 200 live, about 25 MB), and the
growth was all in anonymous memory outside glibc's main heap, that is in
malloc's per-thread arenas. glibc maps a block of at least its mmap
threshold for itself and unmaps it on free, but every such block freed
raises the threshold to its size (up to 32 MiB); once the first look was
let go, the window's pictures and looks, painted on the painter's and the
bakers' threads, came out of a dozen arenas that kept their freed pixels.
With `MALLOC_MMAP_THRESHOLD_=131072` a 20-minute run held flat (slope
31 MB an hour against 219), and `MALLOC_ARENA_MAX=2` nearly so (64). The
app now fixes the threshold at glibc's default first thing in `main`
(`world_machine_desktop::allocator`, the one `unsafe` in the package;
`tests/allocator.rs` fails without it, keeping 24 MB of freed pictures).
Side by side, one hour each on the same machine (10 minutes of warm-up,
219 sittings): before, 618 MB warm, 934 MB at the end, 420 MB an hour
(anonymous memory 245 to 506 MB), failed; after, 433 MB warm, 499 MB at
the end, 44 MB an hour (anonymous 65 to 97 MB), passed. A local run of
20 or 30 minutes still judges the slope on too little: the fixed app
settles near 470 MB within about fifteen minutes, but its short-run slope
(130 to 250 MB an hour) is the settling, not a leak.

The nightly on the fix (`373d2dd`, 490 sittings) held memory far better
(422 MB warm, 528 MB at the end, peak 576 MB, growth 105 MB within its
200) but still failed the slope: 71.8 MB an hour against 60. Five local
one-hour runs of the same code (10 minutes of warm-up, about 217 sittings
each), with the app's mappings sampled every 20 s, put the rest in three
places, from warm-up to the end:

- **The GPU's memory, in 32 MB steps** (`/memfd:allocation`, the
  software renderer's video memory, which is the process's own RAM under
  Xvfb): +0, +32, +32, +32 and +64 MB. A build of GPUI that reported its
  atlas and wgpu's allocator (local only, not committed) showed why: the
  allocator takes video memory in 32 MB blocks and gives one back only
  when nothing is left in it. The window's pictures live in 1024 x 1024
  atlas textures (4 MB each), usually 20 to 30 of them; at a moment with
  more pictures handed over at once (39 textures, 156 MB, 44 minutes into
  one run) it took two more blocks, and kept all eight (228 MB reserved)
  when the atlas fell back to 22 textures (109 MB in use). The step is the
  highest the atlas has been, not a leak; it comes when it comes, so one
  hour is a coin toss (53 to 129 MB an hour across the five runs).
- **malloc's arenas** (anonymous memory): +4 to +17 MB. With
  `MALLOC_ARENA_MAX=2` as well, three runs held anonymous memory about
  30 MB lower throughout, but grew it just as much (+6 to +14 MB), and their
  slopes (54, 33, 107 MB an hour) moved with the GPU's steps like the
  others. Calling `malloc_trim` near the end of one of them gave back
  13 MB of 495: little is free and kept.
- **The main heap**: +5 to +6 MB, about 30 KB a World day. The World's own
  history is there. Lived headless as the app lives it (its file opened as
  a durable session, a day at a time, the Pack in the same process), a
  World holds 74 KB more a day (33 MB at day 50, 67 MB at day 500); in the
  app most of that is in the Pack's own process (17 MB to 41 MB over an
  hour), which the soak does not measure.

Nothing the app holds was found growing: the painted pictures stay bounded
(150 to 300 in the atlas), and the frame logs the soak turns on are
written line by line to their files, never kept.

## The release shots (`.github/workflows/release-shots.yml`)

`scripts/release-shots.sh` plays the first ten minutes and the key moments
in the real window, for every release (and the `first` scenario for every
pull request that touches the window). See the script's header.
