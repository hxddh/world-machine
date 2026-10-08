#!/usr/bin/env bash
# The real-window screenshot harness: builds the desktop app in release,
# runs it under Xvfb at the World window's default size, plays the first
# ten minutes and the key moments, and saves what the window showed.
#
# The art director signs looks off on these pictures, never on test
# renders: what the player sees is the release app's own window, with the
# painting off the window's thread exactly as it ships. Every capture is
# also measured: every frame the app draws is logged, and a frame with an
# unpainted still layer (a flat stand-in where the ground or the buildings
# should be, or the loading wash) is reported; the script fails if any key
# moment shows one.
#
# The desktop app's window builds on Linux with its `linux-window` feature
# (apps/world-machine-desktop/build.rs). It is built straight from the
# working tree into the workspace's own target directory: no copy of the
# source, so a release build from the tree never links what a copy built.
#
# Needs: Xvfb, xdotool, x11-utils (xwininfo, xwd), imagemagick, python3,
# and the GPUI Linux build dependencies (libxkbcommon-x11-dev,
# libvulkan-dev, mesa-vulkan-drivers for a software renderer).
#
# Usage:
#   scripts/release-shots.sh [out-dir] [scenario...]
#
# Scenarios (all of them when none is named):
#   first     a new World from an empty library: the first ten seconds,
#             second by second, then day 1 when the welcome is up, the free
#             wildflowers placed, the first card answered and a few days
#             passed;
#   find      a harbour on the day of a favour: Find, and the camera's
#             landing frame by frame;
#   return    a harbour played eleven days, then 72 hours away: the return
#             film, beat by beat;
#   zoom      a year-three harbour at every zoom level, in and out;
#   year2     a second-year harbour as it opens;
#   places    each place at noon, dusk and night (the harbour on day 20,
#             and Ares, Maple Street and Icebridge as Pocket Universe's
#             start cards begin them).
#
# Environment: SKIP_BUILD=1 reuses the last build; BIN=<dir> runs the
# binaries in another release directory (its examples/ included); JOBS=N
# (default 4).
#
# Each scenario's frames are read from the app's frame log: the script
# reports when the scene was first painted and every frame after that which
# showed something unpainted, and fails if there is any (or, for a new
# World, if the town is not painted within 3 seconds).
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
        cargo build --locked --release -j "${JOBS:-4}" -p world-observer --example pretend_away
        cargo build --locked --release -j "${JOBS:-4}" -p pocket-universe --example harness_place
    ) || { echo "release-shots: the release build failed" >&2; exit 1; }
fi

DISPLAY_NUMBER=":${SHOTS_DISPLAY:-91}"
export DISPLAY="$DISPLAY_NUMBER"
Xvfb "$DISPLAY_NUMBER" -screen 0 1400x1000x24 >/dev/null 2>&1 &
XVFB=$!
APP=""
cleanup() {
    [ -n "$APP" ] && kill "$APP" 2>/dev/null || true
    kill "$XVFB" 2>/dev/null || true
}
trap cleanup EXIT
sleep 2

# A scenario that fails is reported, and the others still run.
set +e
# shellcheck source=scripts/release-shots-lib.sh
source "$ROOT_DIR/scripts/release-shots-lib.sh"

for scenario in $SCENARIOS; do
    echo "== $scenario"
    "scenario_$scenario"
done
# Non-zero when any scenario failed (the CI job fails with it).
report
exit $?
