#!/usr/bin/env bash
# The soak test (1.0 P7): the release app, in its real window under Xvfb,
# driven for a long while through a World's many sittings, watched for
# growth. It fails on a panic or the app dying, on any unpainted frame once
# the window has painted, on resident memory (RSS) growing faster than its
# bound after warm-up, on the frame time's 95th percentile regressing from
# the first part of the run to the last, and on a driver that stopped
# moving the World (no new sitting for too long).
#
# How it drives (scripts/release-shots-lib.sh's helpers): a new Tiny
# Society World from an empty library, with Settings' "Play freely" on, so
# Goodnight's Enter is "Sleep now" and every sitting's end begins the next
# at once. Enter answers each card (its first answer: works go up in
# scaffolding, deeds are done), lets each day pass and begins each morning;
# now and then it clicks where people stand and says a few words to whoever
# it found (talk), opens and puts away the hands (building), zooms out and
# back in, and opens and closes the drawer.
#
# What it measures, sampled through the run:
# - RSS of the app, and the renderer's graphics memory within it, from
#   /proc every SAMPLE seconds (rss.tsv);
# - the scene's frame time on the window's thread, from the app's frame log
#   (WORLD_MACHINE_FRAME_LOG: every 2 s, frames, mean and longest);
# - every frame the window draws (WORLD_GPUI_FRAME_LOG: unpainted pictures
#   and loading wash per frame);
# - the World's sitting ledger, from its file (sittings.tsv).
# scripts/soak-analyze.py reads them and judges; its bounds are in
# docs/TESTING.md.
#
# Usage:
#   scripts/soak.sh [out-dir]
#
# Environment: SOAK_MINUTES (default 120; 5 is a quick local run),
# SOAK_WARMUP_MINUTES (default a sixth of the run, 2 to 15), SAMPLE
# (seconds between RSS samples, default 5), SKIP_BUILD=1 reuses the last
# build, BIN=<dir> runs another release directory, JOBS=N (default 4),
# SOAK_DISPLAY (default 93), and the analyzer's bounds (see
# scripts/soak-analyze.py). Needs what scripts/release-shots.sh needs.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT_DIR="${1:-$ROOT_DIR/target/soak/out}"
mkdir -p "$OUT_DIR"
OUT_DIR="$(cd "$OUT_DIR" && pwd)"
WORK="$ROOT_DIR/target/soak"
STATE="$WORK/state"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT_DIR/target}"
export CARGO_INCREMENTAL=0
BIN="${BIN:-$CARGO_TARGET_DIR/release}"
MINUTES="${SOAK_MINUTES:-120}"
WARMUP="${SOAK_WARMUP_MINUTES:-$(( MINUTES / 6 > 15 ? 15 : (MINUTES / 6 > 2 ? MINUTES / 6 : 2) ))}"
SAMPLE="${SAMPLE:-5}"

if [ -z "${SKIP_BUILD:-}" ]; then
    (
        set -e
        cd "$ROOT_DIR"
        cargo build --locked --release -j "${JOBS:-4}" -p world-machine-desktop \
            --features world-machine-desktop/linux-window \
            -p pocket-universe-pack -p tiny-society-pack
        cargo build --locked --release -j "${JOBS:-4}" -p world-library --example pretend_away
    ) || { echo "soak: the release build failed" >&2; exit 1; }
fi

DISPLAY_NUMBER=":${SOAK_DISPLAY:-93}"
export DISPLAY="$DISPLAY_NUMBER"
if [ -e "/tmp/.X11-unix/X${DISPLAY_NUMBER#:}" ] || [ -e "/tmp/.X${DISPLAY_NUMBER#:}-lock" ]; then
    echo "soak: the display $DISPLAY_NUMBER is in use; set SOAK_DISPLAY to a free one" >&2
    exit 2
fi
Xvfb "$DISPLAY_NUMBER" -screen 0 1400x1000x24 >/dev/null 2>&1 &
XVFB=$!
APP=""
SAMPLER=""
cleanup() {
    [ -n "$SAMPLER" ] && kill "$SAMPLER" 2>/dev/null || true
    [ -n "$APP" ] && kill "$APP" 2>/dev/null || true
    kill "$XVFB" 2>/dev/null || true
}
trap cleanup EXIT
sleep 2
kill -0 "$XVFB" 2>/dev/null || { echo "soak: Xvfb did not start on $DISPLAY_NUMBER" >&2; exit 2; }

set +e
# shellcheck source=scripts/release-shots-lib.sh
source "$ROOT_DIR/scripts/release-shots-lib.sh"
rm -f "$WORK/display-died"
SCENARIO=soak

# A new World, with "Play freely" chosen (Settings' file under the app's
# HOME, as the app writes it).
fresh_state tiny-society
SUPPORT="$STATE/home/Library/Application Support/World Machine"
mkdir -p "$SUPPORT"
printf '{\n  "version": 1,\n  "play_freely": true\n}\n' > "$SUPPORT/Analyst Settings.json"

export WORLD_MACHINE_FRAME_LOG=1
start_app 12
echo "soak: the app is $APP; $MINUTES minutes, warm-up $WARMUP, a sample every $SAMPLE s"
STARTED=$(date +%s)
DEADLINE=$((STARTED + MINUTES * 60))

# RSS, every SAMPLE seconds, until the app is gone: seconds since the
# start, kB, and the kB of it that is the renderer's graphics memory (the
# resident part of the GPU driver's memfd and /dev/dri mappings: under
# Xvfb the software renderer's video memory counts as the process's own).
: > "$OUT_DIR/rss.tsv"
(
    while kill -0 "$APP" 2>/dev/null; do
        rss=$(awk '/^VmRSS:/ {print $2}' "/proc/$APP/status" 2>/dev/null)
        gpu=$(awk '/^[0-9a-f]+-[0-9a-f]+ / { graphics = ($6 ~ /^\/memfd:/ || $6 ~ /^\/dev\/dri\//) }
                   /^Rss:/ && graphics { kb += $2 }
                   END { print kb + 0 }' "/proc/$APP/smaps" 2>/dev/null)
        [ -n "$rss" ] && printf '%s\t%s\t%s\n' "$(($(date +%s) - STARTED))" "$rss" "${gpu:-0}" >> "$OUT_DIR/rss.tsv"
        sleep "$SAMPLE"
    done
) &
SAMPLER=$!

enter_from_title 60 || { echo "soak: no World window"; exit 1; }
focus
sleep 8
shot opened

# Where people may stand on the place, in the World window (1100 x 900:
# the title bar above, the cards below): clicked in turn, one a round.
SPOTS=(300:560 420:600 550:580 680:600 800:560 480:520 620:540 360:640 740:640)
: > "$OUT_DIR/sittings.tsv"
round=0
last_sittings=0
last_progress=$(date +%s)
while [ "$(date +%s)" -lt "$DEADLINE" ]; do
    need_display
    kill -0 "$APP" 2>/dev/null || { echo "soak: the app died"; break; }
    round=$((round + 1))
    focus
    # Cards, days, Goodnight (Sleep now) and mornings: Enter, at a
    # player's pace.
    for _ in 1 2 3 4 5 6; do
        key Return 2.5
    done
    case $((round % 6)) in
        1) # Talk: click where someone may stand; Enter goes to their
           # words if it found someone; say a few; put it away.
           spot=${SPOTS[$((round / 6 % ${#SPOTS[@]}))]}
           click_world "${spot%:*}" "${spot#*:}" 1.5
           key Return 1
           xdotool type --delay 80 "how are you" 2>/dev/null
           key Return 3
           key Escape 1
           key Escape 1 ;;
        2) # Building: the hands, opened and put away.
           key h 2
           key Escape 1 ;;
        3) # The camera: out, and back in.
           key minus 2
           key minus 2
           key equal 2
           key equal 2 ;;
        4) # The drawer, opened and closed.
           key ctrl+i 2
           key ctrl+i 1 ;;
        *) ;;
    esac
    sittings=$(ledger sittings)
    now=$(date +%s)
    printf '%s\t%s\t%s\n' "$((now - STARTED))" "${sittings:-?}" "$(ledger days)" >> "$OUT_DIR/sittings.tsv"
    if [ -n "$sittings" ] && [ "$sittings" != "$last_sittings" ]; then
        last_sittings=$sittings
        last_progress=$now
    elif [ $((now - last_progress)) -gt 120 ]; then
        # Something is open that Enter does not answer: put it away.
        key Escape 1
        key Escape 1
    fi
    if [ $((round % 60)) = 0 ]; then
        shot "round-$round"
    fi
done
shot last
kill -0 "$APP" 2>/dev/null
ALIVE=$?
stop_app
kill "$SAMPLER" 2>/dev/null
SAMPLER=""
cp "$STATE/frames.log" "$OUT_DIR/frames.log"
cp "$STATE/app.log" "$OUT_DIR/app.log"

python3 "$ROOT_DIR/scripts/soak-analyze.py" "$OUT_DIR" \
    --minutes "$MINUTES" --warmup-minutes "$WARMUP" --alive "$((1 - ALIVE))"
