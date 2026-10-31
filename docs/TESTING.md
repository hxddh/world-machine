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
unpainted frames, about 100 to 300 ms after zooming out and back in (the
camera bound to a new zoom whose look had not baked, with no stand-in and no
wash): a real finding of the living stage, which this bar keeps visible
until it is fixed (`docs/KNOWN_ISSUES.md`).

## The release shots (`.github/workflows/release-shots.yml`)

`scripts/release-shots.sh` plays the first ten minutes and the key moments
in the real window, for every release (and the `first` scenario for every
pull request that touches the window). See the script's header.
