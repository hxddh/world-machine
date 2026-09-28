# v0.18.0 review: the second year holds; the third year and the platform are next

`v0.18.0` gave the World a second year. This review plays both Packs for two to three years and checks every dependency, SDK and service the product relies on for newer versions worth using.

**Verdict:** `v0.18.0` does what it promised for the second year. Past it, three things give way:

- Tiny Society's ladder of works **stops climbing**. One rung waits for 300 days while the goal still shows as in progress.
- **Saves grow about 10 MB a year**, which makes a year-old World code about a megabyte of text.
- What people say **gets staler every season**. Pocket Universe, which has no year turns, is back to 45% new lines by its second year.

On the platform side:

- **CI stops working next week:** GitHub starts switching off the `macos-14` runners on 2026-10-05.
- **GPUI is seven weeks and 132 relevant commits behind**, with an API that would stop hidden windows drawing.
- **World voice uses a model now listed as legacy.** A cheaper, faster one fits the job better.
- **macOS 27 ships a free on-device model with a command-line tool** the app can call without any Swift.

## Measured

The scripted warm player from the last review answered the first question each day, made something new every third day and made a suggestion every tenth. It played Tiny Society for 1,080 days, and Maple Street and Ares (Pocket Universe) for 720 days each. All runs were release builds in this container.

| Tiny Society, at day | 120 | 365 | 480 | 720 | 1080 |
| --- | --- | --- | --- | --- | --- |
| Book found | 70 of 91 | 106 of 114 | 108 of 116 | 109 of 116 | 110 of 117 |
| Works finished (goals) | 6 of 7 | 26 of 27 | 27 of 28 | 27 of 28 | 28 of 29 |
| Keepsakes / letters | 17 / 89 | 63 / 288 | 83 / 383 | 123 / 583 | 183 / 883 |
| Events in the World | 3,178 | 9,471 | 12,259 | 17,674 | 25,330 |
| Save file (JSON) | 2.9 MB | 9.8 MB | 13.1 MB | 19.9 MB | 29.9 MB |
| Snapshot (ms) | 37 | 29 | 35 | 34 | 49 |

| New lines, per 120 days | 1st | 2nd | 3rd | 4th | 5th | 6th | 7th | 8th | 9th |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Tiny Society | 93% | 83% | 72% | 70% | 65% | 62% | 56% | 54% | 51% |
| Maple Street | 90% | 73% | 60% | 53% | 45% | 43% | | | |
| Ares | 91% | 76% | 67% | 56% | 54% | 45% | | | |

- **Held:**
  - at most 3 keepsakes in any week, in every run;
  - no question asked more than 4 times in any year;
  - the book reaches 93% within a year in Tiny Society and about 90% by year two in Pocket Universe;
  - Pocket Universe's ladder keeps climbing (26 works done by day 720);
  - 72 to 108 suggestions were held without error.
- **The ladder stalls in Tiny Society.**
  - From about day 400, "Paint the swings by the school" never comes up again. Only two works finish in the next 680 days.
  - The cause is how storylets choose: a work is a *want*. After it has come up once, it only comes up again when no other want is open, and in Tiny Society's deck another want nearly always is.
  - The `v0.18.0` test checks that a goal is *in progress*, not that it *moves*, so it passed.
- **Quiet days are back.** On 13 to 21 days in every 120 (11 to 19 in Pocket Universe) nothing new is found, kept, written or ended.
  - Letters are no longer quiet: 883 in three years, nearly one a day, so the letter box floods and letters stop meaning much.
- **Saves grow with every event.**
  - Tiny Society records about 9,500 events a year. The save is about 10 MB of JSON a year, and each change rewrites the whole file.
  - A World code was about 100 characters per event in the `v0.18.0` tests, so a year-old World's code would be roughly a million characters. That is an estimate from that ratio, not a measurement.
  - Snapshots take 25–50 ms at one to three years, against the `v0.13` bar of 10 ms. Timings in this container are noisy, but they have drifted up.
- **Staleness.**
  - The lines heard most in the third year are still about what the player made ("The light down by Harbor makes you want to stay.", 13 times in 120 days).
  - Pocket Universe has none of Tiny Society's year turns, so its second year runs at 45%.

## Dependencies, SDKs and services

The workspace has 9 direct crates from crates.io or git and 701 packages in its graph. Every duplicate version in the graph comes from inside GPUI. The full research, with a source for every claim, is summarised here.

| Component | Ours | Latest | What it offers | Do |
| --- | --- | --- | --- | --- |
| **GitHub `macos-14` runners** (CI, release, screenshots) | macos-14 (**moved in this PR**) | macos-15, macos-26 (= latest, Xcode 26.6), xcode-27 preview | `macos-14` is deprecated: brownouts on eight October days from **2026-10-05**, retired **2026-11-02** | **Move now** to macos-15 (build) and macos-26 (screenshots); checkout and upload-artifact to v7 |
| **GPUI** (git pin) | `4e8057d`, 2026-08-10 | Zed main `5becf8b`, 2026-09-28: 707 commits, 132 in GPUI. Stable tag v1.21.0 | New since our pin:<br>• `observe_window_visibility`, so nothing draws while covered, minimised or asleep<br>• `Animation::with_max_fps` and spring animations<br>• headless rendering on Linux (wgpu)<br>• runtime font install (a bundled CJK font)<br>• CJK line breaking<br>• two crash fixes<br>crates.io still has only 0.2.2 (2025) | Bump to a Zed **stable-tag** commit in v0.19 |
| GPUI, already in our pin | — | — | Accessibility roles (`.role()`; we use only `.aria_label`, 11 times). `render_to_image` offscreen, behind `test-support` | Use now: VoiceOver roles and golden images |
| **Rust** | 1.97.1 local; CI's `@stable` already resolves to 1.98.1 | 1.98.1 (2026-09-03) | Zed main needs 1.98.1. 1.98.1 fixes a miscompile. Local and CI disagree today | Pin `1.98.1` in `rust-toolchain.toml` with the GPUI bump. Edition 2024 later |
| flate2 | 1.1.9 | 1.1.10 | Patch | `cargo update` (89 compatible patch or minor bumps) |
| sha2 | 0.10.9 | 0.11.0 | Edition 2024, new `digest`; 3 call sites | v0.19 |
| base64 | 0.22.1 | 0.23.1 | SIMD engines; GPUI's usvg still uses 0.22, so bumping would add a duplicate | Skip until usvg moves |
| chrono, serde, serde_json, url | current | — | — | — |
| **World voice model** (Claude API) | `claude-opus-5` (now *legacy*), effort low, `max_tokens` 1024 | Sonnet 5 ($2/$10 per MTok, thinking can be off, not retired before 2027-06-30); Opus 5.5 ($4/$20); Fable 5.1 | A one-line in-character answer suits **Sonnet 5**, thinking off, about 200 tokens: roughly $0.003 a line. Other useful API features:<br>• structured outputs (a JSON schema for the three read-back lines)<br>• `stop_reason: refusal`<br>• streaming<br>• the Batch API at half price (for pre-writing away-time letters) | Switch the model, make it configurable, add a schema and refusal handling |
| **macOS 27 "Golden Gate"** (2026-09-14) | app targets macOS 14+ | Foundation Models: a rebuilt on-device model with image input, a 32K-context Private Cloud Compute model with no API key, and an **`fm` command-line tool** (`fm respond --schema`) | An opt-in, free, offline World voice with guided JSON output, called like `afplay`. **Not launched in mainland China** (press reports, August 2026); needs Apple silicon and a one-time licence acceptance | Opt-in `fm` voice in v0.19, falling back to the World's own words |
| Audio | `/usr/bin/afplay`, one process per sound | rodio 0.22.2, cpal 0.18.2 | In-process mixing, loops, fades and ducking | v0.20 |
| Photos and postcards | `/usr/sbin/screencapture` (asks for Screen Recording) | GPUI offscreen render (above) | Postcards without a permission prompt | Golden tests now; in-app after the GPUI bump |
| Notifications, widgets, menu bar | none | `objc2-user-notifications` 0.3.2; WidgetKit needs a Swift extension | "A letter came" | Still blocked on signing |
| **pi** (optional local runtime) | protocol as of 2026-08 | pi_agent_rust v0.6.1 (2026-09-24) | A terminal `error` event (`auth.missing_api_key`); the message shapes we read are unchanged | Parse the error event |
| pi licence | — | — | pi_agent_rust is MIT **plus a rider granting no rights to Anthropic, OpenAI or anyone acting for them**, with "use" including running and testing. The TypeScript pi (badlogic/pi-mono, plain MIT) has the same `--mode rpc` | A maintainer should decide. Document the plain-MIT pi as an alternative, after a compatibility test |
| Offline Chinese models | — | llama.cpp b11229, MLX 0.32.2, Qwen3.5 0.8B–9B (secondary sources) | A voice for where Apple Intelligence is not offered, run as a subprocess like pi | Watch |

## The upgrade, done ahead of v0.19

Every dependency is now on its latest stable release, in the same pull request as this review:

- **GPUI** is pinned to Zed's **v1.21.0** stable tag (`33c9585`). The crate is still not published on crates.io, so a pin is the only way. It needed no code changes.
- **Rust** is pinned to **1.98.1** in `rust-toolchain.toml`, and CI now installs exactly that version instead of whatever `stable` is.
- **Crates:** `cargo update` took every compatible bump.
  - sha2 moved to 0.11. It no longer formats a digest as hex, so the three Pack crates now write the hex themselves.
  - base64 moved to 0.23. GPUI's SVG library still uses 0.22, so the tree now has both.
  - Three transitive crates stay behind because a dependency pins them: cocoa 0.26.0, generic-array 0.14.7 and unicode-properties 0.1.3.
- **CI** moved off `macos-14` to `macos-15`. Screenshots now run on `macos-15` and `macos-26`. `checkout` and `upload-artifact` are at v7 and `paths-filter` at v4.
- **World voice** now defaults to `claude-sonnet-5`:
  - Thinking is off for models that allow it (Sonnet 5, Opus 5), and effort is low for those that do not.
  - `WORLD_MACHINE_VOICE_MODEL` picks another model.
  - A refusal counts as no answer, so the World's own words stand.
- **Pocket Universe's narrator** asks for its lines as JSON held to a schema (`output_config.format`), so nothing is picked out of prose. Its older "LINE n:" reply is still read.
- **pi's new terminal `error` event is read.** A missing key is named as `auth.missing_api_key` instead of "no successful response".
- **GPUI capabilities put to use** (tested on Linux through GPUI's test window unless noted):
  - *Window visibility* (`observe_window_visibility`, `is_visible`): the strip and the World window draw nothing and keep no clock while nobody can see them, and draw once the moment they are shown. Tested with simulated visibility changes; real macOS occlusion is compiled only.
  - *Springs* (`with_spring`, `SpringConfig`): question, asking and gift cards spring in; the strip's letter drops with one bounce, using the spring maths inside our own 12 fps budget. Reduce Motion (GPUI's and ours) starts them at rest. Tested by element bounds.
  - *Accessibility roles* (`.role()`): buttons, answers (with position in set and "Not now" reasons), the drawer's lists, the hands' verb tabs, the strip, Home cards and Settings switches and chips. Found on the way: elements with a label but no role were never exposed at all. Element roles are tested; the live tree and VoiceOver are not (GPUI's test window has no accessibility tree), and Home/Settings are compiled only.
  - *Headless rendering* (`HeadlessAppContext::with_platform`): four golden pictures (diorama by day and at dusk, strip, postcard) drawn from GPUI's real `Scene` by a small CPU rasteriser in the tests, since GPUI's only headless renderer is Metal. They catch changes in what is drawn, not Metal's pixels, and carry no text. Real-pixel goldens on a macOS runner are a v0.19 item.
  - *Not used:* `Animation::with_max_fps` keeps waking a hidden element, which would undo the visibility gating; the strip's own scheduler already caps at 12 fps. GPUI v1.21.0 still lets CJK closing punctuation start a line, so our speech paging now applies that rule itself (tested); text GPUI wraps itself is unchanged.

## v0.19: concrete content

Each item names what changes, where, and the test that holds it.

1. **Works that keep moving.**
   - `storylets` gains a lane for goals. A storylet that advances an unfinished goal counts as *needed* whenever no storylet for a goal is open, whatever wants are open. Deck order decides which goal comes first.
   - The Tiny Society ladder is unchanged otherwise.
   - Tested by the 480-day player, plus a 1,080-day run: a work finishes at least every 60 days in both Packs, and no rung waits longer than 45 days to be asked.
2. **Pocket Universe years.** Each place gets its own year turns, as Tiny Society has, in a `years.rs` of its own:
   - **Ares:** Nia's apprentice takes over the greenhouse, and the relay crew rotates (one newcomer settles).
   - **Maple Street:** the arcade changes hands, and Ray's kid starts at the high school.
   - **Icebridge:** a chick fledges, and the lantern-keeper passes the lantern on.
   - Each year adds a festival chosen by the last.
   - Tested: the second year's new lines at least 60%, as in Tiny Society.
3. **A third year.**
   - Tiny Society's year turns go on:
     - year 4: the school gets a second teacher;
     - year 5: Leo retires to the quay and his boat passes to Mia;
     - later years: newcomers marry, move out and move in.
   - The ladder gains a fourth round ("light it up": lamps and bunting on each work).
   - Tested: new lines at least 55% in every 120 days up to day 1,080.
4. **Fewer letters, no quiet days.**
   - Letters at most two a week, each carrying news from the year or a memory of the writer's own.
   - A quiet day instead gets a small first: someone's first line about something they have never mentioned, or a corner of a place not yet seen.
   - Tested: at most 2 letters in any 7 days, and at most 4 quiet days in any 120, over 1,080 days.
5. **Saves a tenth the size.**
   - What fills a year of Tiny Society: 9,622 events and 9.9 MB of JSON.
     - Day-to-day "lived" events are 49% of events and 70% of the bytes, about 1.4 KB each, holding about ten changed values.
     - Written with tags, each changed value costs about 110 bytes.
     - Serialising a year takes 34 ms, and happens on every save.
   - Steps, each tested:
     - (a) A compact value and change encoding, read alongside the old one. Target at least 3 times smaller.
     - (b) The World file is deflated on disk (`flate2`, already a dependency). Target a year under 1 MB.
     - (c) A checkpoint of state every season, with the events since. Replay starts from the checkpoint and stays exact, and the full history stays in the file for the book and History.
     - (d) A World code carries the latest checkpoint and the last season's events. Target under 50,000 characters at three years.
   - Tested:
     - a three-year World saves under 3 MB;
     - it reopens event for event;
     - a code reopens as the same World;
     - its snapshot takes under 15 ms.
6. **Snapshots under 15 ms at three years.**
   - Profile the projection at day 1,080 and remove every full scan of the history from snapshot. The book, keepsakes, letters and the "met" set are currently rebuilt from every event on every snapshot.
   - Keep running counts in the System's notes instead.
   - Tested by the existing snapshot benchmark, extended to three years.
7. **World voice everywhere.**
   - An opt-in on-device voice through macOS 27's `fm respond --schema`. It is shown only when `/usr/bin/fm` is present and working, and falls back to the World's own words (so mainland China gets no model voice).
   - A Settings field for the Claude model name.
   - Tested with recorded `fm` output, as the red-team set is today.
8. **In-process sound (rodio).** It needs listening on a Mac, so it waits for one.

**Not in v0.19:**
- edition 2024;
- notifications and widgets, which still wait on signing;
- a bundled Chinese font, which macOS does not need (PingFang ships with it).

## Progress in v0.19.0

Seven of the eight items shipped; rodio waits for a Mac, as planned.

| Item | Target | v0.19.0 |
|---|---|---|
| 1. Works keep moving | a work every ≤60 days, a rung asked within 45 | Tiny Society 41 and 3 days; Pocket Universe 53 and 27 |
| 2. Pocket Universe years | second year ≥60% new lines | 70–75% |
| 3. A third year | ≥55% new in every 120 days to day 1,080 | lowest 58% |
| 4. Letters and quiet days | ≤2 letters a week, ≤4 quiet days in 120 | 2; Tiny Society 0, Pocket Universe ≤1 |
| 5. Saves | three years under 3 MB, code under 50,000 characters | 2.09 MB; 48,020 characters (`wm2:`) |
| 6. Snapshots | under 15 ms at three years | about 11 ms median, release, first snapshot after each day |
| 7. World voice everywhere | `fm` voice and a model field | both; `fm` tested against recorded replies only |

Still open: the "let the day pass" preview is now the largest part of a snapshot; resting and gathering lines are the most repeated in the third year; a code's visit tells only the latest season in History and the book.

## Sources

- **CI runners:**
  - [runner-images #13518: macos-14 deprecation and brownouts](https://github.com/actions/runner-images/issues/13518)
  - [runner-images #14404](https://github.com/actions/runner-images/issues/14404)
  - [runner-images README](https://github.com/actions/runner-images/blob/main/README.md)
- **GPUI:**
  - `git ls-remote https://github.com/zed-industries/zed`, and `git log 4e8057d..5becf8b -- crates/gpui*` (PRs #64107 visibility, #62579 `with_max_fps`, #62778 springs, #64718 Linux headless rendering, #63498 fonts, #62743 CJK line breaking, #64623 and #64672 crash fixes)
  - [gpui on crates.io](https://crates.io/crates/gpui)
  - [Zed's rust-toolchain.toml](https://raw.githubusercontent.com/zed-industries/zed/5becf8b5910fd538ccc5489e8edd3fde32917fad/rust-toolchain.toml)
- **Rust:** [RELEASES.md](https://raw.githubusercontent.com/rust-lang/rust/main/RELEASES.md).
- **Crates:**
  - [base64 release notes](https://github.com/marshallpierce/rust-base64/blob/master/RELEASE-NOTES.md)
  - [sha2 changelog](https://github.com/RustCrypto/hashes/blob/master/sha2/CHANGELOG.md)
  - the crates.io API
- **Apple:**
  - [WWDC26 macOS guide](https://developer.apple.com/wwdc26/guides/macos/)
  - [WWDC26 session 241: Foundation Models](https://developer.apple.com/videos/play/wwdc2026/241/)
  - [WWDC26 session 334: the `fm` tool](https://developer.apple.com/videos/play/wwdc2026/334/)
- **Apple Intelligence in China (press):**
  - [TechCrunch, 2026-07-15](https://techcrunch.com/2026/07/15/apple-intelligence-approved-for-launch-in-china-with-alibabas-qwen-ai/)
  - [TechNode, 2026-08-10](https://technode.com/2026/08/10/apple-says-mainland-china-has-not-launched-qwen-integration-after-mac-guide-disappears/)
- **Claude:** [models overview](https://platform.claude.com/docs/en/about-claude/models/overview).
- **pi:**
  - [pi_agent_rust changelog](https://github.com/Dicklesworthstone/pi_agent_rust/blob/main/CHANGELOG.md)
  - [pi_agent_rust RPC doc](https://github.com/Dicklesworthstone/pi_agent_rust/blob/main/docs/rpc.md)
  - [pi-mono RPC doc](https://github.com/badlogic/pi-mono/blob/main/packages/coding-agent/docs/rpc.md)
