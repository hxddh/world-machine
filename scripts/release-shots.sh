#!/usr/bin/env bash
# The real-window screenshot harness: builds the desktop app in release,
# runs it under Xvfb at the World window's default size, plays the first
# ten minutes and the key moments, and saves what the window showed.
#
# The art director signs looks off on these pictures, never on test
# renders: what the player sees is the release app's own window, with the
# living stage's looks baked off the window's thread exactly as it ships.
# Every capture is also measured: every frame the app draws is logged, and
# the script fails if the loading wash a window opens under is not gone in
# time, or if any frame after it shows something unpainted.
#
# The desktop app's window builds on Linux with its `linux-window` feature
# (apps/world-machine-desktop/build.rs). It is built straight from the
# working tree into the workspace's own target directory: no copy of the
# source, so a release build from the tree never links what a copy built.
#
# Needs: Xvfb, xdotool, x11-utils (xwininfo), x11-apps (xwd), imagemagick, python3,
# and the GPUI Linux build dependencies (libxkbcommon-x11-dev,
# libvulkan-dev, mesa-vulkan-drivers for a software renderer).
#
# Usage:
#   scripts/release-shots.sh [out-dir] [scenario...]
#
# Scenarios (all of them when none is named). A World moves one of its
# days a sitting (three on its first), on the sitting ledger its file
# carries; each scenario starts on a library of its own, so a harbour the
# harness wrote opens on its first sitting, with its morning card:
#   first     a new World from an empty library: the first ten seconds,
#             second by second, the welcome (no morning on the harbour's
#             first day), the free wildflowers, then the first sitting's
#             three days played card by card to its Goodnight, and the
#             World asleep (Enter passes no more days);
#   find      a harbour on the day of a favour: the morning put away, Find,
#             and the camera's landing 0.3, 1.0 and 2.5 s after it began
#             to move;
#   return    a harbour played eleven days, then 72 hours away (its ledger
#             backdated): the new sitting opens on the morning card, then
#             the day, played to Goodnight after its one World day, and
#             asleep; opened again the same day (`return-again`) it is
#             asleep, with no morning and no day to pass. Time away moves
#             nothing, and there is no return film;
#   zoom      a year-three harbour, its morning put away, at every zoom
#             level, out and in, each step 0.3, 1.0 and 2.5 s after it
#             began;
#   year2     a second-year harbour as it opens, on its morning, and the
#             day begun;
#   places    each place at noon, dusk and night, its morning put away (the
#             harbour on day 20, and Ares, Maple Street and Icebridge as
#             Pocket Universe's start cards begin them).
#
# Environment: SKIP_BUILD=1 reuses the last build; BIN=<dir> runs the
# binaries in another release directory (its examples/ included); JOBS=N
# (default 4).
#
# Each scenario's frames are read from the app's frame log: the script
# reports when the loading wash was gone and every frame after that which
# showed something unpainted, and fails if there is any, or if the wash
# lasts more than 2 s (3 s for a new World). It also fails if any of the
# first 2 s of a window shows another World, or if a sitting is not what
# the game now does: a morning missing or shown twice, Goodnight after
# another number of days than the sitting's, a day passed while asleep
# (read from the window and the ledger). How long a stand-in look shows
# after a camera move is reported, not judged. If the display dies the run
# stops at once (exit 2) rather than hanging.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT_DIR="${1:-$ROOT_DIR/target/release-shots/shots}"
mkdir -p "$OUT_DIR"
OUT_DIR="$(cd "$OUT_DIR" && pwd)"
shift || true
SCENARIOS="${*:-first find return zoom year2 places}"
WORK="$ROOT_DIR/target/release-shots"
STATE="$WORK/state"
export CARGO_TARGET_DIR="$ROOT_DIR/target"
export CARGO_INCREMENTAL=0
BIN="${BIN:-$CARGO_TARGET_DIR/release}"

if [ -z "${SKIP_BUILD:-}" ]; then
    (
        set -e
        cd "$ROOT_DIR"
        cargo build --locked --release -j "${JOBS:-4}" -p world-machine-desktop \
            --features world-machine-desktop/linux-window \
            -p pocket-universe-pack -p tiny-society-pack
        cargo build --locked --release -j "${JOBS:-4}" -p tiny-society --example harness_world
        cargo build --locked --release -j "${JOBS:-4}" -p world-library --example pretend_away
        cargo build --locked --release -j "${JOBS:-4}" -p pocket-universe --example harness_place
    ) || { echo "release-shots: the release build failed" >&2; exit 1; }
fi

DISPLAY_NUMBER=":${SHOTS_DISPLAY:-91}"
export DISPLAY="$DISPLAY_NUMBER"
# A display already in use (another run, another program's windows) would
# put strangers' windows among the app's: stop rather than shoot them.
if [ -e "/tmp/.X11-unix/X${DISPLAY_NUMBER#:}" ] || [ -e "/tmp/.X${DISPLAY_NUMBER#:}-lock" ]; then
    echo "release-shots: the display $DISPLAY_NUMBER is in use; set SHOTS_DISPLAY to a free one" >&2
    exit 2
fi
Xvfb "$DISPLAY_NUMBER" -screen 0 1400x1000x24 >/dev/null 2>&1 &
XVFB=$!
APP=""
cleanup() {
    [ -n "$APP" ] && kill "$APP" 2>/dev/null || true
    kill "$XVFB" 2>/dev/null || true
}
trap cleanup EXIT
sleep 2
kill -0 "$XVFB" 2>/dev/null || { echo "release-shots: Xvfb did not start on $DISPLAY_NUMBER" >&2; exit 2; }

# A scenario that fails is reported, and the others still run.
set +e
# shellcheck source=scripts/release-shots-lib.sh
source "$ROOT_DIR/scripts/release-shots-lib.sh"

rm -f "$WORK/display-died"
for scenario in $SCENARIOS; do
    echo "== $scenario"
    "scenario_$scenario"
    need_display
done
# Non-zero when any scenario failed (the CI job fails with it).
report
exit $?
