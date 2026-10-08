# Store-page kit

Everything the store page needs, from the game's own pictures and words, for v0.28. The text leads with the handmade town; the demo and the game are whole without the voice. Every picture comes from the release build's real window or from the engine's own key-art renderer, never from a test render or a mock-up.

| | File | State |
|---|---|---|
| Key art, 3840×2160 | `keyart-harbour.jpg` (and `keyart-{ares,maple,icebridge}.jpg`) | Rendered by world-gpui's `key_art` example from the scene's own fixtures, on the v0.28 working tree of 8 October (the harbour at dusk in its second year; clear sky upper left for the title), saved as JPEG at quality 88. Signed off by the art director on 2026-10-08 for the store page. |
| Five screenshots, 1100×900 real window | `shot-1-day-one.png` … `shot-5-year-three.png` | From P's harness (`scripts/release-shots.sh`), chosen below. Signed off by the art director on 2026-10-08. |
| The 30-second capture plan | [capture-plan.md](capture-plan.md) | Written; recorded from the harness's scenarios. |
| Short description, en / zh / ja | [description.md](description.md) | Written; zh and ja need a native reader. |
| AI disclosure, en / zh / ja | [ai-disclosure.md](ai-disclosure.md) | Drafted for V3 to check; the survey text is [../ai-disclosure.md](../ai-disclosure.md). |

The longer description, the feature list and the system requirements are in [../store-page.md](../store-page.md); the facts sheet is [../README.md](../README.md).

## The five screenshots

Store pages want 16:9 at 1920×1080 or larger. Each shot is the 1100×900 window as the harness saved it; for the store, crop below the title bar (the top 52 px) to 1100×619 and scale to 1920×1080, as `scripts/make-trailer.py` does.

| # | File | Moment | Harness source | Why it is on the page |
|---|---|---|---|---|
| 1 | `shot-1-day-one.png` | Day 1 on the Harbour Front, morning: the pub, the lighthouse, four residents, Evan's first question | `first` (`N3-day1-first-screen`, after the art director's third round) | The handmade town at first sight, painted from the first frame. |
| 2 | `shot-2-return.png` | Back on day 24: the return film opens on the lamp you began, by the quay | `return` (the right half of `P-03b-return-beat-1`, the v0.28 window) | The town lived while you were away, and tells you. (The opening frame was the soft placeholder and was replaced by the art director.) The beat's line says "harbour" twice; it should be rewritten before a capture. |
| 3 | `shot-3-find.png` | Day 5: Mia asked you to invite Leo out; **Find** has landed on him, his card offering the favour as a reply | `find` (`P-04-find-landed`) | People ask things of you, and you can see who. |
| 4 | `shot-4-dusk.png` | Day 21 at dusk: lit windows, lamps along the quay, bunting from the fete | `places` (`P-07-harbour-19`) | The light follows the clock; the end of the demo's three weeks. |
| 5 | `shot-5-year-three.png` | Day 1,002, pulled back: the whole town your answers built | `zoom` (`P-05-year3-zoom-out4`) | Years of a town; every zoom level painted. |

All five are from P's v0.28 harness run (release build, the real 1100×900 window under Xvfb with a software renderer), copied from `scratchpad/v028/shots/`. Signed off by the art director on 2026-10-08; the zh and ja descriptions still need a native reader. **Under review:** the [v0.28 review](../../REVIEW_v0.28.md#what-the-art-director-found) found that `shot-2-return.png` is the sharp version of a frame players see blurred, and that the dusk shot reads grey; from v0.29 every store shot must match the frame the player sees at that moment.

## How the kit is made again

```bash
# The key art (a release build of world-gpui's example; writes A-keyart-<place>.png)
cargo run --locked --release -p world-gpui --example key_art -- target/keyart
# The real-window pictures (needs Xvfb; see the script's header)
scripts/release-shots.sh target/release-shots/shots
```

The `Release shots` workflow (`.github/workflows/release-shots.yml`) runs the harness for every release tag and keeps the pictures as the run's artifact, so each release has its own.
