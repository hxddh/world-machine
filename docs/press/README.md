# Press kit

Everything a store page, a journalist or a festival submission needs, in one place. Each fact is checked against the code or a test; where it is still a decision, it says so.

- [store-page.md](store-page.md): short and long descriptions (English, Simplified Chinese, Japanese draft), the feature list and system requirements.
- [ai-disclosure.md](ai-disclosure.md): the AI disclosure, in Steam's content survey wording.
- [trailer.json](trailer.json): the trailer's cut list, which `scripts/make-trailer.py` turns into a 1080p video.

## Facts

| | |
|---|---|
| Title | World Machine |
| Developer | hxddh (independent) |
| Publisher | Self-published |
| Platform | macOS 14 or later, Apple silicon and Intel (one universal app) |
| Other platforms | None yet. Windows needs a GPUI Windows target and testing. |
| Release | Early release on GitHub since v0.2. Store date: *owner's decision* |
| Price | *Owner's decision.* Comparable titles sell at $6.99–$19.99 (docs/REVIEW_v0.23.md). |
| Demo | Free: the first hour of Tiny Society. When it ends, the World is kept for the full game. |
| Genre | A cozy life sim that keeps going while you are away: part idle game, part diorama, part story generator |
| Languages | English, Simplified Chinese; Japanese with v0.26 |
| Worlds included | Tiny Society (a harbour town) and Pocket Universe (a Mars colony, Maple Street in 1987, and Icebridge) |
| Time away | One day for every six hours away, up to a week each return |
| Story tested over | Three in-game years (1,080 days) in every place, by scripted players |
| Account, telemetry | None. Worlds are files on the player's Mac. |
| AI | Optional, off by default, with the player's own key or their Mac's model; see [ai-disclosure.md](ai-disclosure.md) |
| Built with | Rust and GPUI (the UI framework of the Zed editor). Drawings and sound are made by the game's own code. |
| Licence | Open source, Apache-2.0 |
| Source and downloads | https://github.com/hxddh/world-machine |
| Press contact | *To be filled in* |

## What makes it different (for a pitch)

1. **It lives without you.** Most cozy games wait for you. World Machine keeps going: close the window at breakfast and by the evening the harbour has lived two days of its own.
2. **It tells you what happened.** A return plays as a short film: the camera glides to each moment's people, and the words say what changed and why.
3. **Your answers come back.** A choice made on day 3 is still in the town's story in year three: in what stands, who stayed and what people say.
4. **Nothing is made up after the fact.** Every World keeps its history as recorded events and replays it exactly. The optional language model only words what the World has already decided, and every one of its lines is checked and recorded.

## Assets

| Asset | Where | State |
|---|---|---|
| Trailer (about 40 s, 1920×1080) | `python3 scripts/make-trailer.py` | A first cut from the review screenshots; the return film and dusk frames come from the Linux preview (see below) |
| Screenshots, 1100×900 | `docs/review/v24-*.png`, `docs/review/v25-*.png` and new ones from the preview | Linux preview, not a Mac; fonts and window chrome differ |
| App icon, 1024 px | `apps/world-machine-desktop/macos/icon.png` | Final for now |
| Key art | None | Waits for a commissioned artist (decision 6 in the v0.23 review) |

### Recommended screenshots

Store pages want 16:9. The preview window is 1100×900, so crop below the title bar (the top 52 px). `scripts/make-trailer.py` does the same for the trailer.

1. Dusk on the quay, with a line being said (`docs/review/v23-dusk.png`).
2. The whole town at dusk, zoomed out (`docs/review/v24-postcard.png`).
3. A chapter closing at night (`docs/review/v25-night.png`).
4. The return film in motion (to come from the preview's `frames`).
5. Talking to someone in your own words.
6. Designing a sail (`docs/review/v23-design.png`).
7. Icebridge with a question (`docs/review/v24-icebridge.png`).
8. Maple Street at dusk in the rain (`docs/review/v17-maple-street-dusk.png`).
9. The book or the almanac in the drawer (`docs/review/v24-drawer.png`).
10. The demo's ending card.

## Making the trailer

```bash
python3 -m pip install pillow imageio-ffmpeg   # or have ffmpeg on the PATH
python3 scripts/make-trailer.py --shots <dir with film/ and dusk/ frame folders>
```

It writes `target/trailer/world-machine-trailer.mp4`: 1920×1080, 30 fps, H.264, about 38 seconds, silent unless `--audio` names a track. The music can come from the game itself: `WORLD_MACHINE_SOUNDS=<dir> cargo test -p world-sound --release --test bars write_sounds -- --ignored` writes its sounds to files. The cut uses the app's own pictures only. Where a captured frame sequence is missing it falls back to a still, so it can always be cut and improves as captures arrive.
