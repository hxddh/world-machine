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
| **GitHub `macos-14` runners** (CI, release, screenshots) | macos-14 | macos-15, macos-26 (= latest, Xcode 26.6), xcode-27 preview | `macos-14` is deprecated: brownouts on eight October days from **2026-10-05**, retired **2026-11-02** | **Move now** to macos-15 (build) and macos-26 (screenshots); checkout and upload-artifact to v7 |
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

## v0.19: a third year, lighter saves, and a current platform

1. **Keep CI alive (first, before 2026-10-05).**
   - Move `ci.yml` and `release-package.yml` to `macos-15`, and screenshots to `macos-15` and `macos-26`.
   - Bump `actions/checkout` and `upload-artifact`.
   - Pin Rust `1.98.1` so local and CI build the same compiler.
2. **Works that keep moving.**
   - A work in progress is always in the running: it counts as needed whenever no *work* is open, whatever other wants are.
   - Pocket Universe gets year turns of its own: someone grows up, a place is handed on, newcomers settle, and a festival is chosen.
   - Tested over three years in both Packs:
     - a work finishes at least every 60 days;
     - never a day without one under way;
     - new lines at least 55% in every 120 days of years two and three.
3. **Fewer, better letters; no quiet days.**
   - A letter at most every few days, and one that says something only a letter could (news from the year, something the writer remembers).
   - A quiet day gets something small and new instead (a first line from someone, a new corner of a place).
   - Tested: at most 2 letters a week, and no more than 4 quiet days in any 120.
4. **Saves that stay small.**
   - Keep a checkpoint of the World's state with the events since it, and fold older events into chapter summaries the book keeps. Replay stays exact from the checkpoint, and no decision is made again.
   - Snapshots read only what they show.
   - Tested at three years:
     - save under 3 MB;
     - snapshot under 15 ms;
     - a World code (checkpoint plus recent history) under 50,000 characters, reopening as the same World.
5. **A current GPUI.**
   - Move to a Zed stable-tag commit.
   - Use window visibility so the World and the strip draw nothing while covered or asleep, and `with_max_fps` for the strip's walk.
   - Pick up CJK line breaking and the crash fixes.
   - Tested: the frame budget benchmark, plus a hidden-window test that counts zero frames.
6. **World voice, cheaper and local.**
   - Sonnet 5 with thinking off, the model name in settings, a JSON schema for the three read-back lines, and refusal handling.
   - An opt-in on-device voice through `fm` on macOS 27, falling back to the World's own words where it is not available (mainland China today).
   - Parse pi's new error event and say plainly when pi has no key.
   - Tested with recorded responses, as the red-team set is today.
7. **Seen and heard properly.**
   - VoiceOver roles on the cards, drawer, book, strip and envelope, using the accessibility API GPUI already has.
   - Postcards and covers rendered in-app, with golden-image tests from the offscreen renderer, so the Screen Recording prompt goes away.
8. **Housekeeping:** `cargo update`, sha2 0.11. Document the pi licence rider and the plain-MIT alternative for a maintainer to decide.

**Not in this release:**
- **In-process audio (rodio).** It is worth doing, but it touches every sound and needs listening on a Mac.
- **Edition 2024.**
- **Notifications and widgets.** They still wait on signing.

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
