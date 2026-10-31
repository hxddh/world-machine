# Scenarios and helpers for scripts/release-shots.sh (sourced, not run).
#
# Every scenario starts the release app on a library of its own, drives it
# with xdotool, and saves what the window shows as <out>/<scenario>-<name>.png.
# The app writes a line for every frame it draws to the frame log
# (WORLD_GPUI_FRAME_LOG): milliseconds since its first frame, the window,
# whether what the camera saw was not yet painted (1 only under the loading
# wash, before a window's first look has baked), how much loading wash lay
# over it, whether every look had settled (0 while the nearest baked look
# stands in, scaled, for one still baking), how many residents and whole
# buildings stood in the window (-1 when folded), two columns the tile
# painter's rough painting used to fill (always 0 since the living stage),
# whether the camera was on its way, where it was bound, and which World
# it painted. `measure` reads that log after each scenario: how long the
# opening wash lasts, unpainted frames after it, how long a stand-in look
# shows after every camera move (reported, not judged: see KNOWN_ISSUES),
# and frames of another World in the first 2 s. Every camera move is also
# caught 0.3, 1.0 and 2.5 s after it began (`after_move`).
#
# A sitting (v0.30 S4) is read from the window and from the sitting
# ledger the World's file carries (`pretend_away --show`): the morning card and the
# Goodnight card are found by their one primary button ("Begin the day",
# "Goodnight") on the paper hanging from the top of the place, and the
# ledger says how many sittings there have been, how many World days this
# one passed and whether its morning was read.

STATE="${STATE:-$WORK/state}"
FAILED=0
SITTING_FAILED=""
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

# After something that moves the camera (Find, a zoom):
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

# Home: the newest window of Home's default size (760 by 760). It opens
# on the title screen, the town at dusk with Continue, New and Settings,
# walked with the arrows and chosen with Enter.
home_window() {
    local id
    for id in $(wins_all); do
        [ "$(size_of "$id")" = "760x760" ] && echo "$id"
    done | tail -1
}

wait_home() { # wait_home <seconds>
    local until=$((SECONDS + ${1:-30}))
    while [ $SECONDS -lt $until ]; do
        need_display
        [ -n "$(home_window)" ] && return 0
        sleep 0.1
    done
    return 1
}

# Chooses the title screen's first choice with Enter until a World window
# is there: Continue (the most recent World) when there is one, New on a
# first launch. Home may still be loading on a busy machine, so Enter is
# pressed again until the World window opens; Enter on Home is the same
# choice each time, and an open World only comes forward.
enter_from_title() { # enter_from_title [seconds per try]
    local w
    wait_home 30 || { echo "  no Home window"; return 1; }
    sleep 3
    for _ in 1 2 3 4; do
        w=$(home_window)
        [ -n "$w" ] || break
        xdotool windowfocus --sync "$w" 2>/dev/null || true
        xdotool key --window "$w" Return 2>/dev/null
        wait_world "${1:-15}" && break
    done
    wait_world 1
}

# Opens the most recent World from the title screen (Continue).
open_first() {
    enter_from_title 15
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

# ---- Sittings (v0.30 S4) -------------------------------------------------

# A sitting check that failed, for `measure` to fail the scenario with.
sitting_failed() { # sitting_failed <what>
    echo "  $SCENARIO: $1"
    SITTING_FAILED="${SITTING_FAILED:+$SITTING_FAILED; }$1"
}

# Clicks at a point of the World window (not the newest window: a tooltip
# or Home could be that).
click_world() { # click_world <x> <y> [seconds after]
    need_display
    local w; w=$(world_window)
    [ -n "$w" ] || { need_display; echo "  no World window to click"; return 1; }
    read -r X Y < <(origin "$w")
    xdotool mousemove $((X + $1 - 2)) $((Y + $2 - 2)); sleep 0.2
    xdotool mousemove $((X + $1)) $((Y + $2)) click 1
    sleep "${3:-1}"
}

# Where the sitting's card has its button, in the World window: the
# morning's "Begin the day" or Goodnight's "Goodnight", the one primary
# button on the paper hanging from the top of the place (520 px wide at
# most, centred in the 1100 px window, below the 52 px title bar). Found
# as the block of the accent colour (world-theme's ACCENT) that survives
# eroding by 24 x 6 px, so text drawn in or near it never counts; prints
# its centre, or nothing when no such card is up.
sitting_button() {
    need_display
    local w accent lamp box bw bh bx by
    w=$(world_window)
    [ -n "$w" ] || return 1
    # The light appearance's accent by day, and under the lamp a card is
    # read by at night (world-theme's `lamplit`); the dark appearance's.
    accent='#3a5796' lamp='#344b75'
    [ "${APPEARANCE:-light}" = dark ] && accent='#7f9ddc' lamp='#7f9ddc'
    timeout 10 xwd -id "$w" -silent > "$WORK/button.xwd" || return 1
    box=$(convert "$WORK/button.xwd" -alpha off -crop 520x460+290+100 +repage \
        -fuzz 1% -fill '#010203' -opaque "$accent" -opaque "$lamp" -fuzz 0 \
        -fill black +opaque '#010203' -fill white -opaque '#010203' \
        -morphology Erode Rectangle:24x6 -trim -format '%w %h %X %Y' info: 2>/dev/null)
    read -r bw bh bx by <<< "$box"
    [ -n "$by" ] && [ "${bx:-}" != "-1" ] && [ "${bw:-0}" -ge 20 ] || return 0
    echo "$((290 + ${bx#+} + bw / 2)) $((100 + ${by#+} + bh / 2))"
}

# One field of the World's sitting ledger, as its file carries it (host
# metadata beside its history: sittings, the local day the sitting began,
# the World days it passed, the morning read). Each scenario's library
# holds one World, so its one ledger.
ledger() { # ledger sittings|began|days|morning
    "$BIN/examples/pretend_away" --show "$STATE/Worlds" "$1" 2>/dev/null
}

# The morning a new sitting opens on: shot as <name>, then "Begin the
# day" clicked, and the day shot as <name>-begun. Fails the scenario if
# no morning shows (or it stays up), or the ledger does not have it read.
begin_day() { # begin_day [name]
    local name=${1:-morning} at
    at=$(sitting_button)
    shot "$name"
    if [ -z "$at" ]; then
        sitting_failed "no morning card"
        return 1
    fi
    # shellcheck disable=SC2086
    click_world $at 2
    shot "$name-begun"
    [ -z "$(sitting_button)" ] || sitting_failed "the morning stayed up after Begin the day"
    [ -n "$(ledger morning)" ] || sitting_failed "the ledger has no morning read"
}

# Begins the day if the place opened on a morning (a World whose first
# day it is opens on none).
begin_day_if_shown() { # begin_day_if_shown [name]
    [ -n "$(sitting_button)" ] && begin_day "$@"
    return 0
}

# Plays the sitting's days with Enter (each card's first answer, then the
# day's own card) until the Goodnight card comes up, shooting the window
# after each press as <prefix>-<n>; a morning that opens a later day of
# the sitting (the ledger short of `days`) is shot as <prefix>-<n>-morning
# and begun with Enter. Then says Goodnight and shoots the World asleep.
# Fails the scenario if Goodnight never comes in `presses`, a morning
# stays up, or the ledger has not passed `days` World days.
play_to_goodnight() { # play_to_goodnight <days> [presses] [prefix]
    local days=$1 presses=${2:-24} prefix=${3:-card} n=0 at=""
    while [ $n -lt "$presses" ]; do
        n=$((n + 1))
        key Return 3.5
        shot "$prefix-$n"
        at=$(sitting_button)
        [ -n "$at" ] || continue
        [ "$(ledger days)" -lt "$days" ] || break
        shot "$prefix-$n-morning"
        key Return 3.5
        if [ -n "$(sitting_button)" ]; then
            sitting_failed "the morning of day $(($(ledger days) + 1)) stayed up after Enter"
            return 1
        fi
        at=""
    done
    if [ -z "$at" ]; then
        sitting_failed "no Goodnight after $presses presses"
        return 1
    fi
    sleep 1
    shot goodnight
    [ "$(ledger days)" = "$days" ] ||
        sitting_failed "Goodnight after $(ledger days) World days, not $days"
    # shellcheck disable=SC2086
    click_world $at 3
    shot asleep
    [ -z "$(sitting_button)" ] || sitting_failed "the Goodnight card stayed up"
    # Asleep, the day's card is gone: Enter lets no more days pass.
    key Return 2
    [ "$(ledger days)" = "$days" ] || sitting_failed "a day passed while the World slept"
    sleep 2
    shot asleep-later
}

# Reads the frame log: the World window's loading wash must be gone within
# `bar` seconds of its first frame (the living stage shows the wash only
# until a window's first look has baked), the first painted frame must
# hold at least the residents and whole buildings asked for, and no frame
# after it may show anything unpainted. A sitting check that failed
# (`sitting_failed`) fails the scenario too.
measure() { # measure [bar seconds for the first painted frame (default 2)] [residents buildings]
    python3 - "$STATE/frames.log" "$SCENARIO" "${1:-2}" "${2:-0}" "${3:-0}" <<'PY'
import sys
path, scenario, bar = sys.argv[1], sys.argv[2], float(sys.argv[3])
least_people, least_buildings = int(sys.argv[4]), int(sys.argv[5])
rows = [line.split() for line in open(path)]
rows = [row for row in rows if len(row) >= 4]
# The World window's frames: the window that drew the most.
windows = {}
for row in rows:
    windows[row[1]] = windows.get(row[1], 0) + 1
main = max(windows, key=windows.get) if windows else None
own = [r for r in rows if r[1] == main]
if not own:
    print(f"  {scenario}: no frames logged"); sys.exit(1)
frames = [(int(r[0]), int(r[2]), float(r[3])) for r in own]
first_painted = next((t for t, gaps, wash in frames if gaps == 0 and wash == 0), None)
after = [(t, g, w) for t, g, w in frames if first_painted is not None and t > first_painted]
bad = [(t, g, w) for t, g, w in after if g > 0 or w > 0]
print(f"  {scenario}: {len(frames)} frames; the opening wash gone at "
      f"{'never' if first_painted is None else f'{first_painted / 1000:.2f} s'} (bar {bar:.1f} s); "
      f"{len(bad)} unpainted frames after it")
for t, g, w in bad[:10]:
    print(f"    unpainted at {t / 1000:.2f} s: {g} pictures, wash {w}")
ok = first_painted is not None and not bad and first_painted <= bar * 1000
if first_painted is not None:
    painted = next(r for r in own if int(r[0]) == first_painted)
    people = int(painted[5]) if len(painted) > 6 else -1
    buildings = int(painted[6]) if len(painted) > 6 else -1
    print(f"  {scenario}: the first painted frame has {people} residents and "
          f"{buildings} whole buildings in it (at least {least_people} and {least_buildings})")
    if people >= 0 and (people < least_people or buildings < least_buildings):
        ok = False

# After every camera move (Find, a zoom): how long a stand-in look (the
# nearest one baked, scaled) shows once the camera has settled, until
# every look in view has settled on its own. Reported, not judged: the
# living stage's stand-in runs are a known issue. Columns: 5 settled, 10
# whether the camera is on its way, 11 where it is bound (a new value is a
# move), 12 the kind of place, 13 the World's colours.
full = [r for r in own if len(r) >= 14]
moves = 0
worst = 0
if full:
    starts = [i for i in range(1, len(full)) if full[i][10] != full[i - 1][10]]
    for n, start in enumerate(starts):
        end = starts[n + 1] if n + 1 < len(starts) else len(full)
        seg = full[start:end]
        settle = next((k for k, r in enumerate(seg) if r[9] == "0"), None)
        if settle is None:
            continue
        moves += 1
        settled_at = int(seg[settle][0])
        sharp = next((int(r[0]) for r in seg[settle:] if r[4] == "1"), None)
        lasting = (int(seg[-1][0]) if sharp is None else sharp) - settled_at
        worst = max(worst, lasting)
    print(f"  {scenario}: {moves} camera moves; a stand-in look at most {worst} ms after "
          f"the camera settled (reported, not judged)")

# No frame of another World: in the first 2 s of every window, every frame
# is of the place (its kind and colours) the window settles on.
foreign = 0
for window in {r[1] for r in rows if len(r) >= 14}:
    mine = [r for r in rows if r[1] == window and len(r) >= 14]
    if not mine:
        continue
    began, place = int(mine[0][0]), (mine[-1][11], mine[-1][12])
    foreign += sum(1 for r in mine if int(r[0]) - began <= 2000 and (r[11], r[12]) != place)
print(f"  {scenario}: {foreign} frames of another World in the first 2 s")
if foreign:
    ok = False
sys.exit(0 if ok else 1)
PY
    local status=$?
    if [ $status -ne 0 ] || [ -n "$SITTING_FAILED" ]; then
        FAILED=1
        SUMMARY+=("$SCENARIO: FAILED${SITTING_FAILED:+ ($SITTING_FAILED)}")
    else
        SUMMARY+=("$SCENARIO: ok")
    fi
    SITTING_FAILED=""
}

report() {
    echo "== summary"
    printf '  %s\n' "${SUMMARY[@]}"
    echo "  pictures in $OUT_DIR"
    return $FAILED
}

# ---- Scenarios -----------------------------------------------------------

# A new World from an empty library: the title screen it opens on (New
# first, with no World to continue), New chosen with Enter, then the first
# seconds, the welcome, the free wildflowers, then the first sitting's
# six World days played card by card to its Goodnight (each later day
# opening on its morning, begun with Enter), and the World
# asleep. The harbour's first day opens on no morning.
scenario_first() {
    SCENARIO=first
    fresh_state
    start_app 12
    wait_home 60 || { echo "  no Home window"; FAILED=1; SUMMARY+=("first: FAILED (no Home window)"); return; }
    sleep 4
    shot title "$(home_window)"
    enter_from_title 60 || { echo "  no World window"; FAILED=1; SUMMARY+=("first: FAILED (no World window)"); return; }
    focus
    local started=$SECONDS
    for t in 0 1 2 3 4 5 6; do
        while [ $((SECONDS - started)) -lt "$t" ]; do sleep 0.05; done
        shot "t$t"
    done
    sleep 4
    shot welcome
    [ -z "$(sitting_button)" ] || sitting_failed "a morning on the harbour's first day"
    sleep 6
    shot wildflowers-offered
    key Escape 1.5
    shot day1
    # Enter takes each card's first answer, and then the day's own card.
    play_to_goodnight 6 80 day
    [ "$(ledger sittings)" = 1 ] || sitting_failed "sitting $(ledger sittings), not the first"
    stop_app
    # Painted within 3 s; the camera at rest (v0.30 S1c) opens on the
    # welcome with four residents round it and the focal building whole
    # (v0.28's three whole buildings are the overview's bar, one step out:
    # world-gpui's `the_first_screen_opens_on_the_focal_cluster`).
    measure 3 4 1
}

# Find: a harbour on a day with a favour; the camera's landing, frame by
# frame. It opens on its first sitting's morning, put away first. The
# favour chip sits in the toolbar left of the drawer button.
scenario_find() {
    SCENARIO=find
    harbour_library "${FIND_DAYS:-4}"
    start_app 12
    open_first
    sleep 6
    begin_day
    sleep 2
    shot before
    # The favour's Find chip, the last control but the menu in the bar.
    local from; from=$(camera_bound)
    click_world "${FIND_X:-990}" 84 0
    after_move landing "$from"
    [ "$(camera_bound)" != "$from" ] || sitting_failed "Find did not move the camera"
    sleep 1.5
    shot landed
    stop_app
    measure
}

# A new sitting after time away: a harbour played eleven days (its first
# sitting), then 72 hours away (`pretend_away` backdates the observer's
# ledger). It opens on the morning card, once; "Begin the day", then the
# sitting's one World day played to its Goodnight, and the World asleep.
# Time away moves nothing: no return film. Opened again the same day, it
# opens asleep, with no morning.
scenario_return() {
    SCENARIO=return
    harbour_library 11 72
    start_app 12
    open_first
    sleep 4
    begin_day morning
    sleep 2
    shot day
    play_to_goodnight 1 12 card
    [ "$(ledger sittings)" = 2 ] || sitting_failed "sitting $(ledger sittings), not the second"
    stop_app
    measure
    SCENARIO=return-again
    start_app 12
    open_first
    sleep 6
    shot reopened
    [ -z "$(sitting_button)" ] || sitting_failed "a card over the World opened again the same day"
    key Return 2
    [ "$(ledger days)" = 1 ] || sitting_failed "a day passed in a spent sitting"
    stop_app
    measure
}

# A year-three harbour at every zoom level, out and back in, once its
# morning is put away.
scenario_zoom() {
    SCENARIO=zoom
    harbour_library 1001
    start_app 12
    open_first
    sleep 6
    begin_day
    sleep 4
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
    stop_app
    measure
}

# A second-year harbour as it opens: on its morning, and the day after it.
scenario_year2() {
    SCENARIO=year2
    harbour_library 401
    start_app 12
    open_first
    sleep 6
    begin_day open
    sleep 6
    shot open-later
    stop_app
    measure
}

# Each place at noon, dusk and night: the harbour on day 20, and Pocket
# Universe's Ares, Maple Street and Icebridge as their start-screen cards
# begin them (one keeper each), each with its morning put away first (a
# place on its first day has none). Each first painted frame must show
# someone and a whole building (the harbour at night only a building: its
# fifteen are indoors, seen in their lit windows).
scenario_places() {
    for hour in 12 19 23; do
        SCENARIO="places-harbour-$hour"
        harbour_library 20
        start_app "$hour"
        open_first
        sleep 6
        begin_day morning
        sleep 4
        shot "harbour-$hour"
        stop_app
        # A harbour of fifteen is indoors at night, seen in its lit windows.
        if [ "$hour" = 12 ]; then measure 2 1 1; else measure 2 0 1; fi
        for place in ares maple icebridge; do
            SCENARIO="places-$place-$hour"
            # Only Pocket Universe included: the place is the one World
            # there is, and nothing gets a harbour's Pack ready beside it
            # while the title screen's Continue opens it.
            fresh_state pocket-universe
            "$BIN/examples/harness_place" "$STATE/Worlds" "$place" >/dev/null
            start_app "$hour"
            open_first
            sleep 6
            begin_day_if_shown morning
            sleep 2
            shot "$place-$hour"
            stop_app
            measure 2 1 1
        done
    done
    SCENARIO=places
}
