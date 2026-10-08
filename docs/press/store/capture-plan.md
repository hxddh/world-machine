# The 30-second capture

What to record for the store page's first video and the "first 30 seconds" of a trailer: six beats of the real window, in a release build at the World window's default size (1100×900), with nothing drawn over the scene but a line being said. The voice is off: everything here is the game's own.

Every beat is a moment that `scripts/release-shots.sh` already plays and checks as painted, so a capture is the harness run with the screen recorded. Record only frames the harness reports as painted; a beat with an unpainted frame is recorded again, never cut around.

## The beats

| Time | Beat | Harness scenario | What must be in frame |
|---|---|---|---|
| 0:00–0:04 | A new harbour opens on the Harbour Front in the morning. | `first` (the first ten seconds) | The quay and its cluster, at least 4 people and 3 buildings, painted from the first frame shown. |
| 0:04–0:09 | Your hands: the free wildflowers placed, then the first card answered. | `first` (day 1) | The flowers going down where you put them; the card's choice; scaffolding rising on its own site. |
| 0:09–0:15 | Back after three days away: the return film glides from moment to moment. | `return` | Two or three beats of the film, each on a painted scene, with the line that tells it. |
| 0:15–0:20 | Someone asks a favour; **Find** takes the camera to the person it is about. | `find` | The asker saying it aloud, the camera's landing on painted ground, the glow on the person. |
| 0:20–0:25 | Dusk on the quay, then night: the lamps along the water, and windows that say who is home. | `places` (Tiny Society at dusk and night) | Lamps lit with their pools and broken reflections; lit windows. |
| 0:25–0:30 | Pull back over a year-three harbour, every zoom level painted, to the whole town. | `zoom` | A continuous pull from street level to the whole town; then the title card. |

The title card is the game's own paper card, as in [../trailer.json](../trailer.json): "World Machine", and under it "A little harbour town that goes on living while you are away".

## How to record it

- **On Linux (Xvfb), as the harness runs.** Record the harness's display with `ffmpeg -f x11grab -framerate 30 -video_size 1100x900 -i :<display>+<x>,<y>` on the World window's position (the harness prints it), one file per scenario, and cut the beats from those files. The harness's own frame log gives the time of each painted frame, so each beat starts on a frame the log calls painted.
- **On a Mac (preferred for the store).** The same scenarios by hand, recorded with `screencapture -v -R <x>,<y>,1100,900`, in the light appearance. Fonts and window chrome differ from Linux; the store's video should come from a Mac.
- **Frame.** The store wants 16:9: crop the 1100×900 window to 1100×619 below the title bar (the top 52 px), keeping the cards along the bottom out of frame where a beat allows, and scale to 1920×1080.
- **Sound.** The game's own sound for the scene (the sea, wind, the quay); `WORLD_MACHINE_SOUNDS=<dir> cargo test -p world-sound --release --test bars write_sounds -- --ignored` writes it to files when a recording cannot capture it.
- **Languages.** Record English first. The Chinese and Japanese cuts are the same beats with the app's language switched in Settings › Display; check every line on screen is in the language (no English sentence inside a letter).

## Checks before it is used

- Every frame painted: the harness's report for each scenario says no unpainted frame was shown.
- The art director has signed off the stills of each beat in [README.md](README.md).
- No invented claim: nothing on screen or on the cards that [description.md](description.md) does not hold.
