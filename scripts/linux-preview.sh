#!/usr/bin/env bash
# Run the real World Machine desktop app on Linux and screenshot its windows.
#
# The desktop app is gated to macOS, which has meant that nobody working
# without a Mac could see it, and that every visual change shipped on the
# strength of "it compiles". Nothing in the gated code actually needs macOS
# to draw a window: GPUI renders on Linux too. This script copies the source
# tree, lifts the gate in the copy only, and runs the app under Xvfb against a
# demonstration library and the included World Packs, so a change can be
# looked at before it is pushed.
#
# It is a development aid, not a Linux port: fonts, window chrome, and the
# system appearance differ from a Mac, and nothing macOS-specific (keychain,
# Finder open events, menus) is exercised.
#
# Needs: xvfb-run, xdotool, x11-utils (xwininfo, xwd), imagemagick, and the
# GPUI Linux build dependencies (libxkbcommon-x11-dev, libvulkan-dev,
# mesa-vulkan-drivers for a software renderer).
#
# Usage:
#   [APPEARANCE=dark] scripts/linux-preview.sh [output-dir] [click-x click-y]...
#
# Every window the app has open after start-up (and after each optional click,
# given in pixels relative to Home's top-left corner, or to the newest window
# when the x is written @x) is written to output-dir as <window-id>.png.
# "scroll N" turns the mouse wheel N notches over the newest window. Opening
# the third World on Home, pressing What if…, and scrolling down is
#   scripts/linux-preview.sh shots 688 489 @1035 25 scroll 15
# and "hover @X Y" rests the pointer on the newest window to capture a hover.
# "frames N" right after a click captures N frames ~0.1s apart as
# frame-N.png, to see an animation play.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT_DIR="$(mkdir -p "${1:-$ROOT_DIR/target/linux-preview/shots}" && cd "${1:-$ROOT_DIR/target/linux-preview/shots}" && pwd)"
shift || true
WORK="$ROOT_DIR/target/linux-preview"
SRC="$WORK/src"
STATE="$WORK/state"

rm -rf "$SRC"
mkdir -p "$SRC"
tar -C "$ROOT_DIR" --exclude ./target --exclude ./.git -cf - . | tar -C "$SRC" -xf -

# Lift the macOS gate in the copy. analyst_readiness.rs is excluded because its
# gates choose between genuinely different per-OS constants.
grep -rl 'target_os = "macos"' "$SRC/apps/world-machine-desktop" \
    | grep -v analyst_readiness.rs \
    | xargs sed -i 's/target_os = "macos"/unix/g'
sed -i 's/features = \["font-kit"\]/features = ["font-kit", "x11"]/' \
    "$SRC/apps/world-machine-desktop/Cargo.toml"

export CARGO_TARGET_DIR="$WORK/target"
(
    cd "$SRC"
    cargo build -q -p world-machine-desktop \
        -p pocket-universe-pack -p micro-company-pack -p tiny-society-pack
)
BIN="$CARGO_TARGET_DIR/debug"

rm -rf "$STATE"
mkdir -p "$STATE/home" "$STATE/Worlds" "$STATE/packs"
(cd "$SRC" && cargo run -q -p pocket-universe --example demo_world -- "$STATE/Worlds")
(cd "$SRC" && cargo run -q -p tiny-society --example demo_world -- "$STATE/Worlds")
for pack in pocket-universe micro-company tiny-society; do
    "$BIN/$pack-pack" --write-bundle "$STATE/packs/$pack.worldpack"
done

cat > "$WORK/session.sh" <<'SESSION'
set -u
"$BIN/world-machine-desktop" > "$WORK/app.log" 2>&1 &
APP=$!
# First launch installs and probes the included Packs before Home settles.
sleep 20

home_window() {
    xwininfo -root -tree | awk '/"/ && $0 ~ /[0-9]{3,}x[0-9]{3,}\+/ {print $1; exit}'
}
origin() {
    xwininfo -id "$1" | awk '/Absolute upper-left X/ {x=$4} /Absolute upper-left Y/ {y=$4} END {print x, y}'
}

# X hands out window ids in increasing order, so the largest is the newest.
newest_window() {
    xwininfo -root -tree | awk '/"/ && $0 ~ /[0-9]{3,}x[0-9]{3,}\+/ {print $1}' \
        | while read -r id; do printf '%d %s\n' "$id" "$id"; done \
        | sort -n | tail -1 | cut -d' ' -f2
}

HOME_WINDOW=$(home_window)
set -- $CLICKS
while [ "$#" -ge 2 ]; do
    # "hover @X Y" rests the pointer on the newest window without clicking,
    # so a screenshot can show what the app previews under the pointer.
    if [ "$1" = hover ]; then
        read -r X Y < <(origin "$(newest_window)")
        xdotool mousemove $((X + ${2#@})) $((Y + $3))
        sleep 2
        HOLD_POINTER=1
        shift 3
        continue
    fi
    # "frames N" captures N frames of the newest window in quick succession,
    # to check that something animates rather than only where it ends up.
    if [ "$1" = frames ]; then
        FRAME_WINDOW=$(newest_window)
        for frame in $(seq "$2"); do
            xwd -id "$FRAME_WINDOW" -silent > "$WORK/frame.xwd"
            convert "$WORK/frame.xwd" -alpha off "$OUT_DIR/frame-$frame.png"
            sleep 0.08
        done
        shift 2
        continue
    fi
    # "scroll N" turns the wheel N notches down over the newest window.
    if [ "$1" = scroll ]; then
        read -r X Y < <(origin "$(newest_window)")
        xdotool mousemove $((X + 400)) $((Y + 400))
        for _ in $(seq "$2"); do xdotool click 5; sleep 0.05; done
        sleep 2
        shift 2
        continue
    fi
    # A leading @ clicks in the most recently opened window instead of Home.
    TARGET=$HOME_WINDOW
    case "$1" in
        @*) TARGET=$(newest_window); set -- "${1#@}" "${@:2}" ;;
    esac
    read -r X Y < <(origin "$TARGET")
    xdotool mousemove $((X + $1 - 2)) $((Y + $2 - 2))
    sleep 0.5
    xdotool mousemove $((X + $1)) $((Y + $2)) click 1
    # Right before "frames", capture straight away instead of waiting.
    if [ "${3:-}" = frames ]; then sleep 0.05; else sleep 10; fi
    shift 2
done

for WINDOW in $(xwininfo -root -tree | awk '/"/ && $0 ~ /[0-9]{3,}x[0-9]{3,}\+/ {print $1}'); do
    read -r X Y < <(origin "$WINDOW")
    # A GPUI window under Xvfb only paints once something happens to it, so
    # nudge the pointer until the capture has content.
    for attempt in 1 2 3 4 5 6; do
        # Keep a resting hover in place; otherwise wake the window up.
        if [ -z "${HOLD_POINTER:-}" ]; then
            xdotool mousemove $((X + 10 + attempt)) $((Y + 5))
        fi
        sleep 1.5
        xwd -id "$WINDOW" -silent > "$WORK/window.xwd"
        convert "$WORK/window.xwd" -alpha off "$OUT_DIR/$WINDOW.png"
        MEAN=$(convert "$OUT_DIR/$WINDOW.png" -format '%[fx:mean]' info:)
        awk "BEGIN { exit !($MEAN > 0.05) }" && break
    done
    echo "$OUT_DIR/$WINDOW.png"
done
kill "$APP"
SESSION

export BIN WORK OUT_DIR CLICKS="$*"
export HOME="$STATE/home"
export WORLD_MACHINE_LIBRARY_DIR="$STATE/Worlds"
export WORLD_MACHINE_INCLUDED_PACKS_DIR="$STATE/packs"
export WORLD_MACHINE_NO_UPDATE_CHECK=1
# APPEARANCE=dark checks the dark palette; Xvfb itself only ever reports light.
export WORLD_MACHINE_APPEARANCE="${APPEARANCE:-light}"
xvfb-run -a -s "-screen 0 1400x1000x24" bash "$WORK/session.sh"
