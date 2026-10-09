#!/usr/bin/env bash
# Measures what the app costs while nobody can see its World: frames drawn
# and CPU used with the World window in front, covered by another window,
# and unmapped (minimised), under Xvfb.
#
#   scripts/measure-background.sh [bin-dir] [seconds]
#
# bin-dir holds a Linux build of the app with its window
# (`cargo build --release -p world-machine-desktop --features linux-window`)
# and the two Pack binaries; it defaults to target/release. Frames are
# counted from the diorama's own frame log (WORLD_GPUI_FRAME_LOG), CPU from
# /proc as a share of one core, for the app alone and with its Pack
# processes. The bar (docs/REVIEW_v0.28.md, plan item 5): covered or
# minimised, no frames drawn and under 2% of one core. Exits 1 when it is
# missed. "Covered" moves the input focus to the covering window, as a
# window manager does; covered with the focus kept is measured but not held
# to the bar (see below). Needs Xvfb, xdotool, xwininfo and xlogo (x11-apps).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN="$(cd "${1:-$ROOT/target/release}" && pwd)"
SECONDS_EACH="${2:-30}"
WORK="$(mktemp -d)"
STATE="$WORK/state"
APP=""
XVFB=""
COVER=""

cleanup() {
    [ -n "$COVER" ] && kill "$COVER" 2>/dev/null || true
    [ -n "$APP" ] && kill "$APP" 2>/dev/null || true
    [ -n "$XVFB" ] && kill "$XVFB" 2>/dev/null || true
    rm -rf "$WORK"
}
trap cleanup EXIT

# A display of its own, whichever number is free (another harness, such as
# the release screenshots, may be using one at the same time).
Xvfb -displayfd 3 -screen 0 1400x1000x24 -nolisten tcp 3>"$WORK/display" >/dev/null 2>&1 &
XVFB=$!
for _ in $(seq 100); do [ -s "$WORK/display" ] && break; sleep 0.1; done
export DISPLAY=":$(tr -d '[:space:]' < "$WORK/display")"
for _ in $(seq 50); do xwininfo -root >/dev/null 2>&1 && break; sleep 0.1; done

mkdir -p "$STATE/home" "$STATE/Worlds" "$STATE/packs"
for pack in pocket-universe tiny-society; do
    "$BIN/$pack-pack" --write-bundle "$STATE/packs/$pack.worldpack" >/dev/null
done
: > "$STATE/frames.log"
HOME="$STATE/home" WORLD_MACHINE_LIBRARY_DIR="$STATE/Worlds" \
WORLD_MACHINE_INCLUDED_PACKS_DIR="$STATE/packs" WORLD_MACHINE_NO_UPDATE_CHECK=1 \
WORLD_GPUI_FRAME_LOG="$STATE/frames.log" \
    "$BIN/world-machine-desktop" > "$STATE/app.log" 2>&1 &
APP=$!

size_of() { xwininfo -id "$1" 2>/dev/null | awk '/Width:/ {w=$2} /Height:/ {h=$2} END {print w"x"h}'; }
world_window() {
    xwininfo -root -tree | awk '/[0-9][0-9][0-9]+x[0-9][0-9][0-9]+\+/ {print $1}' |
        while read -r id; do [ "$(size_of "$id")" = "1100x900" ] && echo "$id"; done | tail -1 || true
}
WORLD=""
for _ in $(seq 600); do
    WORLD="$(world_window)"
    [ -n "$WORLD" ] && break
    sleep 0.1
done
if [ -z "$WORLD" ]; then
    echo "measure-background: no World window opened" >&2
    tail -20 "$STATE/app.log" >&2
    exit 2
fi
# The first World's window can be made anew once Home has opened (each is a
# window of its own): wait until the World's window has stayed the same for
# ten seconds, then put it in front.
stable=0
while [ "$stable" -lt 10 ]; do
    sleep 1
    now="$(world_window)"
    if [ -n "$now" ] && [ "$now" = "$WORLD" ]; then
        stable=$((stable + 1))
    else
        WORLD="$now"
        stable=0
    fi
done
xdotool windowfocus --sync "$WORLD" 2>/dev/null || true
sleep 5

# Clock ticks of a process and of all its descendants.
ticks_of() { awk '{print $14 + $15}' "/proc/$1/stat" 2>/dev/null || echo 0; }
descendants() {
    local pid=$1 child
    for child in $(cat /proc/"$pid"/task/*/children 2>/dev/null); do
        echo "$child"
        descendants "$child"
    done
}
tree_ticks() {
    local total
    total=$(ticks_of "$APP")
    for pid in $(descendants "$APP"); do total=$((total + $(ticks_of "$pid"))); done
    echo "$total"
}
HZ=$(getconf CLK_TCK)
FAILED=0
measure() { # measure <label> <must-be-quiet>
    local frames0 app0 tree0 frames1 app1 tree1 start end
    frames0=$(wc -l < "$STATE/frames.log")
    app0=$(ticks_of "$APP"); tree0=$(tree_ticks); start=$(date +%s%N)
    sleep "$SECONDS_EACH"
    frames1=$(wc -l < "$STATE/frames.log")
    if [ -n "${VERBOSE:-}" ]; then
        xwininfo -root -tree | grep -E '[0-9]{3,}x[0-9]{3,}\+' | sed 's/^/    window /'
        tail -n +"$((frames0 + 1))" "$STATE/frames.log" | awk '{n[$2]++} END {for (w in n) print "    frames of", w, n[w]}'
    fi
    app1=$(ticks_of "$APP"); tree1=$(tree_ticks); end=$(date +%s%N)
    python3 - "$1" "$2" "$((frames1 - frames0))" "$((app1 - app0))" "$((tree1 - tree0))" \
        "$HZ" "$(((end - start) / 1000000))" <<'PY' || FAILED=1
import sys
label, quiet, frames, app, tree, hz, ms = sys.argv[1:]
seconds = int(ms) / 1000
app_share = 100 * int(app) / int(hz) / seconds
tree_share = 100 * int(tree) / int(hz) / seconds
print(f"{label:<28} frames {int(frames):>5} ({int(frames) / seconds:5.1f}/s)  "
      f"app {app_share:5.2f}% of a core  app+Packs {tree_share:5.2f}%")
if quiet == "yes" and (int(frames) > 0 or app_share >= 2.0):
    print(f"  missed the bar: no frames and under 2% of one core")
    raise SystemExit(1)
PY
}

echo "World window $WORLD, ${SECONDS_EACH}s each"
measure "in front (focused)" no
xlogo -geometry 1400x1000+0+0 >/dev/null 2>&1 &
COVER=$!
sleep 2
COVER_WINDOW="$(xwininfo -root -tree | awk '/"xlogo"/ {print $1; exit}')"
# What a window manager does when another window is raised over the World:
# the input focus goes with it.
[ -n "$COVER_WINDOW" ] && xdotool windowfocus --sync "$COVER_WINDOW" 2>/dev/null || true
sleep 2
measure "covered (focus moved)" yes
# Covered while the World keeps the input focus, which only happens without
# a window manager. Not a bar: GPUI's window is 32-bit (ARGB), so the X
# server composites it itself and never reports it obscured, and GPUI can
# only go by focus there; macOS reports occlusion itself.
xdotool windowfocus --sync "$WORLD" 2>/dev/null || true
sleep 2
measure "covered (World kept focus)" no
kill "$COVER" 2>/dev/null || true
wait "$COVER" 2>/dev/null || true
COVER=""
sleep 2
# The World's window may have been made anew meanwhile (a new day's film):
# every window of a World's size is minimised.
for window in $(xwininfo -root -tree | awk '/[0-9][0-9][0-9]+x[0-9][0-9][0-9]+\+/ {print $1}'); do
    [ "$(size_of "$window")" = "1100x900" ] && xdotool windowunmap "$window" 2>/dev/null || true
done
sleep 3
measure "minimised (unmapped)" yes
exit "$FAILED"
