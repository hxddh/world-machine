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
#   scripts/linux-preview.sh [output-dir] [click-x click-y]...
#
# Every window the app has open after start-up (and after each optional click,
# given in pixels relative to Home's top-left corner) is written to
# output-dir as <window-id>.png. Clicking Open on the first World card is
#   scripts/linux-preview.sh shots 688 190
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

HOME_WINDOW=$(home_window)
set -- $CLICKS
while [ "$#" -ge 2 ]; do
    read -r X Y < <(origin "$HOME_WINDOW")
    xdotool mousemove $((X + $1 - 2)) $((Y + $2 - 2))
    sleep 0.5
    xdotool mousemove $((X + $1)) $((Y + $2)) click 1
    sleep 10
    shift 2
done

for WINDOW in $(xwininfo -root -tree | awk '/"/ && $0 ~ /[0-9]{3,}x[0-9]{3,}\+/ {print $1}'); do
    read -r X Y < <(origin "$WINDOW")
    # A GPUI window under Xvfb only paints once something happens to it, so
    # nudge the pointer until the capture has content.
    for attempt in 1 2 3 4 5 6; do
        xdotool mousemove $((X + 10 + attempt)) $((Y + 5))
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
xvfb-run -a -s "-screen 0 1400x1000x24" bash "$WORK/session.sh"
