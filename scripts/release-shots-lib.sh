# Scenarios and helpers for scripts/release-shots.sh (sourced, not run).
#
# Every scenario starts the release app on a library of its own, drives it
# with xdotool, and saves what the window shows as <out>/<scenario>-<name>.png.
# The app writes a line for every frame it draws to the frame log
# (WORLD_GPUI_FRAME_LOG): milliseconds since its first frame, the window,
# how many pictures of what the camera saw were not painted, how much
# loading wash lay over it, whether every layer had settled, and how many
# residents and whole buildings stood in the window (-1 when folded).
# `measure` reads that log after each scenario.

STATE="${STATE:-$WORK/state}"
FAILED=0
SUMMARY=()

wins_all() { xwininfo -root -tree | awk '/[0-9][0-9][0-9]+x[0-9][0-9][0-9]+\+/ {print $1}'; }
newest() { wins_all | while read -r id; do printf '%d %s\n' "$id" "$id"; done | sort -n | tail -1 | cut -d' ' -f2; }
origin() { xwininfo -id "$1" | awk '/Absolute upper-left X/ {x=$4} /Absolute upper-left Y/ {y=$4} END {print x, y}'; }
size_of() { xwininfo -id "$1" | awk '/Width:/ {w=$2} /Height:/ {h=$2} END {print w"x"h}'; }

# A window manager is what makes a window active; under bare Xvfb the app
# would think it was in the background and draw only when something
# happens. Giving it the input focus is enough.
focus() { local w; w=$(newest); [ -n "$w" ] && xdotool windowfocus --sync "$w" 2>/dev/null || true; }

# The World window: the newest window of the World's default size.
world_window() {
    local id
    for id in $(wins_all); do
        [ "$(size_of "$id")" = "1100x900" ] && echo "$id"
    done | tail -1
}

wait_world() { # wait_world <seconds>
    local until=$((SECONDS + ${1:-30}))
    while [ $SECONDS -lt $until ]; do
        [ -n "$(world_window)" ] && return 0
        sleep 0.1
    done
    return 1
}

fresh_state() {
    rm -rf "${STATE:?}"
    mkdir -p "$STATE/home" "$STATE/Worlds" "$STATE/packs"
    for pack in pocket-universe tiny-society; do
        "$BIN/$pack-pack" --write-bundle "$STATE/packs/$pack.worldpack" >/dev/null
    done
}

start_app() { # start_app [hour]
    : > "$STATE/frames.log"
    HOME="$STATE/home" WORLD_MACHINE_LIBRARY_DIR="$STATE/Worlds" \
    WORLD_MACHINE_INCLUDED_PACKS_DIR="$STATE/packs" WORLD_MACHINE_NO_UPDATE_CHECK=1 \
    WORLD_MACHINE_APPEARANCE="${APPEARANCE:-light}" WORLD_MACHINE_HOUR="${1:-12}" \
    WORLD_GPUI_FRAME_LOG="$STATE/frames.log" \
        "$BIN/world-machine-desktop" > "$STATE/app.log" 2>&1 &
    APP=$!
}

stop_app() {
    [ -n "$APP" ] && kill "$APP" 2>/dev/null || true
    wait "$APP" 2>/dev/null || true
    APP=""
    sleep 0.5
}

shot() { # shot <name> [window]
    local w=${2:-$(world_window)}
    [ -n "$w" ] || w=$(newest)
    xwd -id "$w" -silent > "$WORK/shot.xwd" && convert "$WORK/shot.xwd" -alpha off "$OUT_DIR/$SCENARIO-$1.png"
    echo "  $SCENARIO-$1.png"
}

click() { # click <x> <y> [seconds after] (relative to the newest window)
    local w; w=$(newest)
    read -r X Y < <(origin "$w")
    xdotool mousemove $((X + $1 - 2)) $((Y + $2 - 2)); sleep 0.2
    xdotool mousemove $((X + $1)) $((Y + $2)) click 1
    sleep "${3:-1}"
}

key() { # key <chord> [seconds after]
    local w; w=$(world_window); [ -n "$w" ] || w=$(newest)
    xdotool key --window "$w" "$1" 2>/dev/null
    sleep "${2:-1}"
}

# Opens the first World on Home (Home is 760 by 760; its first card's
# Open button sits at 327, 325).
open_first() {
    # Home may still be loading on a busy machine: click again until the
    # World window is there.
    sleep 3
    for _ in 1 2 3 4; do
        click 327 325 0.2
        wait_world 15 && break
    done
    focus
}

# A library holding one harbour lived `days` days by a warm player.
harbour_library() { # harbour_library <days> [hours away]
    fresh_state
    "$BIN/examples/harness_world" "$STATE/Worlds" "harbour-$1" "$1" >/dev/null
    if [ -n "${2:-}" ]; then
        "$BIN/examples/pretend_away" "$STATE/Worlds" "$2" >/dev/null
    fi
}

# Reads the frame log: the World window's very first frame must already be
# the place in its own drawings (nothing unpainted, no wash), with at least
# the residents and whole buildings asked for in it; then when the scene
# was first painted, and every frame after that which showed something
# unpainted.
measure() { # measure <bar seconds for the first painted frame, or -> [residents buildings]
    python3 - "$STATE/frames.log" "$SCENARIO" "${1:--}" "${2:-0}" "${3:-0}" <<'PY'
import sys
path, scenario, bar = sys.argv[1], sys.argv[2], sys.argv[3]
least_people, least_buildings = int(sys.argv[4]), int(sys.argv[5])
rows = [line.split() for line in open(path)]
rows = [row for row in rows if len(row) >= 4]
# The World window's frames: the window that drew the most.
windows = {}
for row in rows:
    windows[row[1]] = windows.get(row[1], 0) + 1
main = max(windows, key=windows.get) if windows else None
frames = [(int(r[0]), int(r[2]), float(r[3])) for r in rows if r[1] == main]
if not frames:
    print(f"  {scenario}: no frames logged"); sys.exit(1)
first_painted = next((t for t, gaps, wash in frames if gaps == 0 and wash == 0), None)
after = [(t, g, w) for t, g, w in frames if first_painted is not None and t > first_painted]
bad = [(t, g, w) for t, g, w in after if g > 0 or w > 0]
print(f"  {scenario}: {len(frames)} frames; first painted at "
      f"{'never' if first_painted is None else f'{first_painted / 1000:.2f} s'}; "
      f"{len(bad)} unpainted frames after it")
for t, g, w in bad[:10]:
    print(f"    unpainted at {t / 1000:.2f} s: {g} pictures, wash {w}")
ok = first_painted is not None and not bad
first = next(r for r in rows if r[1] == main)
people = int(first[5]) if len(first) > 6 else -1
buildings = int(first[6]) if len(first) > 6 else -1
whole = int(first[2]) == 0 and float(first[3]) == 0
print(f"  {scenario}: first frame {'whole' if whole else 'UNPAINTED'} "
      f"({first[2]} pictures unpainted, wash {first[3]}), "
      f"{people} residents and {buildings} whole buildings in it")
if not whole or (people >= 0 and (people < least_people or buildings < least_buildings)):
    ok = False
if bar != "-" and (first_painted is None or first_painted > float(bar) * 1000):
    ok = False
sys.exit(0 if ok else 1)
PY
    local status=$?
    if [ $status -ne 0 ]; then FAILED=1; SUMMARY+=("$SCENARIO: FAILED"); else SUMMARY+=("$SCENARIO: ok"); fi
}

report() {
    echo "== summary"
    printf '  %s\n' "${SUMMARY[@]}"
    echo "  pictures in $OUT_DIR"
    return $FAILED
}

# ---- Scenarios -----------------------------------------------------------

# A new World from an empty library: the first seconds, the welcome, the
# free wildflowers, the first card, and days passing.
scenario_first() {
    SCENARIO=first
    fresh_state
    start_app 12
    wait_world 60 || { echo "  no World window"; FAILED=1; return; }
    focus
    local started=$SECONDS
    for t in 0 1 2 3 4 5 6; do
        while [ $((SECONDS - started)) -lt "$t" ]; do sleep 0.05; done
        shot "t$t"
    done
    sleep 4
    shot welcome
    sleep 6
    shot wildflowers-offered
    key Escape 1.5
    shot day1
    # The first card, then a few days: Enter takes the card's first answer.
    for day in 1 2 3 4 5; do
        key Return 3
        shot "day$day-answered"
        key Return 4
        shot "day$((day + 1))"
    done
    stop_app
    measure 3 4 3
}

# Find: a harbour on a day with a favour; the camera's landing, frame by
# frame. The favour chip sits in the toolbar left of the drawer button.
scenario_find() {
    SCENARIO=find
    harbour_library "${FIND_DAYS:-4}"
    start_app 12
    open_first
    sleep 8
    shot before
    # The favour's Find chip, the last control but the menu in the bar.
    click "${FIND_X:-990}" 84 0
    for i in 0 1 2 3 4 5; do sleep 0.4; shot "landing-$i"; done
    sleep 3
    shot landed
    stop_app
    measure -
}

# The return film after 72 hours away, beat by beat.
scenario_return() {
    SCENARIO=return
    harbour_library 11 72
    start_app 12
    open_first
    for beat in 0 1 2 3 4 5 6 7; do
        sleep 2.6
        shot "beat-$beat"
    done
    stop_app
    measure -
}

# A year-three harbour at every zoom level, out and back in.
scenario_zoom() {
    SCENARIO=zoom
    harbour_library 1001
    start_app 12
    open_first
    sleep 10
    shot 1x
    for step in 1 2 3 4; do
        key minus 0.3
        shot "out$step-now"
        sleep 4
        shot "out$step"
    done
    for step in 1 2 3 4 5 6; do
        key equal 0.3
        sleep 3
        shot "in$step"
    done
    key Escape 3
    shot back
    stop_app
    measure -
}

# A second-year harbour as it opens.
scenario_year2() {
    SCENARIO=year2
    harbour_library 401
    start_app 12
    open_first
    sleep 10
    shot open
    sleep 6
    shot open-later
    stop_app
    measure -
}

# Each place at noon, dusk and night: the harbour on day 20, and Pocket
# Universe's Ares, Maple Street and Icebridge as their start-screen cards
# begin them (one keeper each). Each first frame must show someone and a
# whole building (the harbour at night only a building: its fifteen are
# indoors, seen in their lit windows).
scenario_places() {
    for hour in 12 19 23; do
        SCENARIO="places-harbour-$hour"
        harbour_library 20
        start_app "$hour"
        open_first
        sleep 10
        shot "harbour-$hour"
        stop_app
        # A harbour of fifteen is indoors at night, seen in its lit windows.
        if [ "$hour" = 12 ]; then measure - 1 1; else measure - 0 1; fi
        for place in ares maple icebridge; do
            SCENARIO="places-$place-$hour"
            fresh_state
            "$BIN/examples/harness_place" "$STATE/Worlds" "$place" >/dev/null
            start_app "$hour"
            open_first
            sleep 8
            shot "$place-$hour"
            stop_app
            measure - 1 1
        done
    done
    SCENARIO=places
}
