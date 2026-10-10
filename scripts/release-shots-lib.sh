# Scenarios and helpers for scripts/release-shots.sh (sourced, not run).
#
# Every scenario starts the release app on a library of its own, drives it
# with xdotool, and saves what the window shows as <out>/<scenario>-<name>.png.
# The app writes a line for every frame it draws to the frame log
# (WORLD_GPUI_FRAME_LOG): milliseconds since its first frame, the window,
# how many pictures of what the camera saw were not painted, how much
# loading wash lay over it, whether every layer had settled, how many
# residents and whole buildings stood in the window (-1 when folded), how
# much of the rough painting showed anywhere and on the moment's subject,
# whether the camera was on its way, where it was bound, and which World it
# painted. `measure` reads that log after each scenario: unpainted frames,
# the rough painting after every camera move (at most 250 ms on the subject
# and 600 ms anywhere once the camera settles), and frames of another World
# in the first 2 s. Every camera move is also caught 0.3, 1.0 and 2.5 s
# after it began (`after_move`).

STATE="${STATE:-$WORK/state}"
FAILED=0
SUMMARY=()

# The display must be alive for anything here: when Xvfb dies, every X
# call fails (and `xwininfo -id ""` would wait for a click for ever), so the
# whole run stops at once with a clear message instead of hanging.
display_alive() {
    { [ -z "${XVFB:-}" ] || kill -0 "$XVFB" 2>/dev/null; } &&
        timeout 5 xwininfo -root >/dev/null 2>&1
}
# Called anywhere, also inside $(...): a subshell cannot stop the run, so
# the death is written down, and every helper (and the scenario loop)
# stops the run on it from the main shell.
need_display() {
    if [ -e "$WORK/display-died" ] || ! display_alive; then
        echo "release-shots: the display $DISPLAY died; stopping" >&2
        : > "$WORK/display-died"
        if [ "$BASHPID" = "$$" ]; then
            SUMMARY+=("${SCENARIO:-?}: FAILED (the display died)")
            FAILED=1
            report
            exit 2
        fi
        exit 2
    fi
}
wins_all() { need_display; timeout 5 xwininfo -root -tree | awk '/[0-9][0-9][0-9]+x[0-9][0-9][0-9]+\+/ {print $1}'; }
newest() { wins_all | while read -r id; do printf '%d %s\n' "$id" "$id"; done | sort -n | tail -1 | cut -d' ' -f2; }
origin() { [ -n "$1" ] || { need_display; echo "0 0"; return 1; }; timeout 5 xwininfo -id "$1" | awk '/Absolute upper-left X/ {x=$4} /Absolute upper-left Y/ {y=$4} END {print x, y}'; }
size_of() { [ -n "$1" ] || return 1; timeout 5 xwininfo -id "$1" | awk '/Width:/ {w=$2} /Height:/ {h=$2} END {print w"x"h}'; }

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
        need_display
        [ -n "$(world_window)" ] && return 0
        sleep 0.1
    done
    return 1
}

fresh_state() { # fresh_state [pack...] (both included Packs when none is named)
    rm -rf "${STATE:?}"
    mkdir -p "$STATE/home" "$STATE/Worlds" "$STATE/packs"
    for pack in ${*:-pocket-universe tiny-society}; do
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
    need_display
    local w=${2:-$(world_window)}
    [ -n "$w" ] || w=$(newest)
    [ -n "$w" ] || { need_display; echo "  no window for $SCENARIO-$1"; return 1; }
    timeout 10 xwd -id "$w" -silent > "$WORK/shot.xwd" && convert "$WORK/shot.xwd" -alpha off "$OUT_DIR/$SCENARIO-$1.png"
    echo "  $SCENARIO-$1.png"
}

# Where the World window's camera is bound, from the frame log's last line
# for the window that drew the most (column 11; empty before any frame).
camera_bound() {
    awk 'NF >= 11 {n[$2]++; last[$2] = $11} END {m = ""; for (w in n) if (m == "" || n[w] > n[m]) m = w; if (m != "") print last[m]}' "$STATE/frames.log" 2>/dev/null
}

# After something that moves the camera (Find, a return beat, Esc, a zoom):
# waits up to `timeout` seconds for the camera to be bound somewhere new,
# then saves what the window shows 0.3, 1.0 and 2.5 seconds after it began
# to move, as <name>-0.3s, <name>-1.0s and <name>-2.5s: the frames a player
# sees while the camera arrives, not only settled ones.
after_move() { # after_move <name> [from-bound] [timeout]
    local from=${2:-$(camera_bound)} until=$((SECONDS + ${3:-8})) began
    while [ $SECONDS -lt $until ] && [ "$(camera_bound)" = "$from" ]; do sleep 0.03; done
    began=$(date +%s%N)
    for at in 0.3 1.0 2.5; do
        local due=$((began + ${at%.*} * 1000000000 + ${at#*.} * 100000000))
        while [ "$(date +%s%N)" -lt "$due" ]; do sleep 0.02; done
        shot "$1-${at}s"
    done
}

click() { # click <x> <y> [seconds after] (relative to the newest window)
    need_display
    local w; w=$(newest)
    [ -n "$w" ] || { need_display; echo "  no window to click"; return 1; }
    read -r X Y < <(origin "$w")
    xdotool mousemove $((X + $1 - 2)) $((Y + $2 - 2)); sleep 0.2
    xdotool mousemove $((X + $1)) $((Y + $2)) click 1
    sleep "${3:-1}"
}

key() { # key <chord> [seconds after]
    need_display
    local w; w=$(world_window); [ -n "$w" ] || w=$(newest)
    [ -n "$w" ] || { need_display; echo "  no window for a key"; return 1; }
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

# What the player sees after every camera move (Find, a return beat, Esc, a
# zoom): how long the rough painting stays on the moment's subject, and
# anywhere, once the camera has settled. Columns: 8 rough anywhere and 9 on
# the subject (square pixels), 10 whether the camera is on its way, 11
# where it is bound (a new value is a move), 12 the kind of place, 13 the
# World's colours.
full = [r for r in rows if r[1] == main and len(r) >= 14]
SUBJECT_BAR, ANY_BAR = 250, 600
worst_subject = worst_any = 0
moves = 0
unsettled = 0
if full:
    starts = [0] + [i for i in range(1, len(full)) if full[i][10] != full[i - 1][10]]
    for n, start in enumerate(starts):
        end = starts[n + 1] if n + 1 < len(starts) else len(full)
        seg = full[start:end]
        settle = next((k for k, r in enumerate(seg) if r[9] == "0"), None)
        if settle is None:
            unsettled += 1
            continue
        settled_at = int(seg[settle][0])
        # Only a move the next one leaves time to judge (or the last).
        if end < len(full) and int(full[end][0]) - settled_at < 700:
            continue
        def lasting(col):
            clean_from = None
            for r in seg[settle:]:
                if float(r[col]) > 0:
                    clean_from = None
                elif clean_from is None:
                    clean_from = int(r[0])
            if clean_from is None:
                return int(seg[-1][0]) - settled_at
            return max(0, clean_from - settled_at)
        if start == 0:
            # The place opening: from its first frame, not from a settle.
            settled_at, settle = int(seg[0][0]), 0
        on_subject, anywhere = lasting(8), lasting(7)
        if start == 0:
            print(f"  {scenario}: opening: the rough painting on the subject for {on_subject} ms, "
                  f"anywhere for {anywhere} ms")
            worst_subject, worst_any = max(worst_subject, on_subject), max(worst_any, anywhere)
            continue
        moves += 1
        worst_subject = max(worst_subject, on_subject)
        worst_any = max(worst_any, anywhere)
        if on_subject > SUBJECT_BAR or anywhere > ANY_BAR:
            print(f"    move at {int(seg[0][0]) / 1000:.2f} s (settled {settled_at / 1000:.2f} s): "
                  f"rough on the subject {on_subject} ms, anywhere {anywhere} ms")
    print(f"  {scenario}: {moves} camera moves judged; the rough painting on the subject at most "
          f"{worst_subject} ms after the camera settled (bar {SUBJECT_BAR}), anywhere at most "
          f"{worst_any} ms (bar {ANY_BAR})")
    if worst_subject > SUBJECT_BAR or worst_any > ANY_BAR:
        ok = False

# No frame of another World: in the first 2 s of every window, every frame
# is of the place (its kind and colours) the window settles on.
foreign = 0
for window in {r[1] for r in rows if len(r) >= 14}:
    own = [r for r in rows if r[1] == window and len(r) >= 14]
    if not own:
        continue
    began, place = int(own[0][0]), (own[-1][11], own[-1][12])
    foreign += sum(1 for r in own if int(r[0]) - began <= 2000 and (r[11], r[12]) != place)
print(f"  {scenario}: {foreign} frames of another World in the first 2 s")
if foreign:
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
    local from; from=$(camera_bound)
    click "${FIND_X:-990}" 84 0
    after_move landing "$from"
    sleep 1.5
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
    # Each beat moves the camera to what it is about.
    sleep 3
    shot beat-0
    for beat in 1 2 3 4 5 6 7; do
        after_move "beat-$beat" "" 12
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
    local from
    for step in 1 2 3 4; do
        from=$(camera_bound)
        key minus 0
        after_move "out$step" "$from" 3
        sleep 1.5
        shot "out$step"
    done
    for step in 1 2 3 4 5 6; do
        from=$(camera_bound)
        key equal 0
        after_move "in$step" "$from" 3
    done
    from=$(camera_bound)
    key Escape 0
    after_move back "$from" 3
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
            # Only Pocket Universe included: with Tiny Society there too and
            # no World of it, Home offers to create one in a banner that
            # arrives late and moves the first World's Open button, so the
            # click could create a harbour instead of opening the place.
            fresh_state pocket-universe
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
