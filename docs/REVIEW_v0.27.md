# World Machine v0.27: a deep review, and what v0.28 should be

Reviewed on 2026-10-08 at `9150c6c` (v0.27.0).

The review has five parts, each written independently:
- **Player:** the real app played in the Linux preview (a debug build with optimised Pack crates), 175 screenshots.
- **Measurement:** the repo's own harnesses in release, with v0.26.1 rebuilt and timed alongside, round by round, on a busy and on a quiet machine.
- **Architecture:** v0.27's diff and the whole tree.
- **Research:** web sources, with URLs and dates. Pages could not be fetched directly, so facts come from search summaries of the cited pages.
- **Art direction:** the art director checked the real-window screenshots against [the bible](ART_DIRECTION.md).

Every number is measured unless it says otherwise. The full reports and their logs are kept in the review workspace.

## The verdict

**v0.27 kept most of its promises:**
- The kernel and every invariant hold.
- The words bars all pass in all 20 three-year runs:
  - nothing told more than twice in 7 days;
  - no seams;
  - no sad lines in the first days;
  - no engine words;
  - no line said more than 7 times in year three, where v0.26 had 15.
- Favours are seen and doable in the real app: the camera goes to the asker, **Find** shows the target, a one-click reply works, and the thanks come at once.
- A build stands at once; in the playtest the pier went up at the quay the moment the timber was bought.
- The demo ends on a dusk farewell, and the year-three harbour at 1× finally reads as a place.
- Hearing new phrasing passed its 85% bar in all three languages.
- CI is green and its test job went from 58 to 37 minutes.

**But the picture is often not painted when the player looks.** Each of v0.27's best moments opens on fog or pale washes:
- A new World shows only fog for about 25 seconds.
- The return film plays its first three beats over blank fog.
- After **Find**, washes stand where the pub and lighthouse should be for 3–4 seconds.
- In year three, zoomed out two steps, the harbour is a flat green slab with props floating on it and smoke rising from houses that aren't there. It was still the same 40 seconds later.

Some of the waiting is the debug build, but the unpainted zoom is a bug, and none of it was caught because the art was signed off on test renders, not on the real window. The 8 ms frame bar now fails in 4 of 6 runs, because v0.27's still layer costs 50–65% more to repaint.

**The voice still misses its bar on the blind set:** 91.0% of bad lines declined and 1.6% of good lines wrongly declined, against 95% / 1%. 58% of the misses are invented names, which the judge already lists correctly but the code ignores unless a separate rule fires first.

**v0.28 should make every picture the player sees a finished one, and make talk safe, fair and lawful.** It should also clear the way to Windows and Steam. The notarized build, the store page and February's Next Fest still wait on the decisions at the end, and the Next Fest deadline is 10 January.

![A new World, 35 seconds in](review/v28-first-launch-fog.jpg)
![Year three, zoomed out: unpainted](review/v28-zoomed-out-unpainted.jpg)

## What the player found

1. **The picture is not painted when you look**, at every high moment: first launch, the return film, **Find** and zoom (above).
2. **The first screen is thin.** In the real 1100×900 window, day 1 shows 2 residents and no buildings. The pub and lighthouse are one pan away. The test for four residents passes, but the window doesn't meet it.
3. **Text seams:**
   - A lone question mark wraps onto its own line, at least four times an hour.
   - The first instruction reads "Choose where the wildflowers goes".
   - In Japanese, "More about {name}" stays English, even on a fresh start.
   - After a live switch from Chinese to Japanese, the open card keeps "说点什么…".
   - Katakana names break mid-word in the card header ("ソフィ/ア").
4. **The demo's recap isn't "what you did here".** It lists "The swallows" and "The storm", plus two keepsake notes torn from their objects; only "The new pier" is the player's own deed. The bar above it still says "next day in 6 h", and **Keep a postcard** gives no feedback.
5. **Hana's thanks appear inside Leo's card,** so it reads as Leo thanking you.
6. **Smaller things:**
   - The toolbar reflows under the pointer, so a click on **Find** opened the drawer.
   - The year-two quay reads as a line-up of about 20 people, two of them standing in the lighthouse door.
   - Maple Street showed no card for several seconds, and Enter opened the Make something panel.

![The first screen in the real window](review/v28-first-screen.jpg)
![Year two: a line-up on the quay](review/v28-year-two-lineup.jpg)
![The thanks in the wrong card](review/v28-thanks-wrong-card.jpg)
![The demo's recap](review/v28-demo-recap.jpg)

**What delights:**
- the pier going up with Evan on it;
- favours as small, visible errands;
- the "Hana hands it on" strip;
- year two's "Here we are again…";
- the dusk farewell;
- natural Japanese in year three;
- Icebridge's one-penguin start;
- the year-three harbour at 1×.

## What the art director found

The bible's rules hold in the signed-off key art and contact sheets, but not yet in the window a player sees:
- **No focal cluster on day 1.** The top half of the real window is sky and hills, and the camera doesn't open on the Harbour Front.
- **Groups of two and three break down on the quay in year two.** A2's grouping works in the test renders but not in the live scene, which places people another way.
- **Washes and fog are part of the picture.** A frame the player sees half-painted breaks the bible as surely as a pier on a hill.

**From v0.28, every look change is signed off on screenshots of the real window, in a release build, at the window's default size:**
- day 1 at the moment the welcome appears;
- a return-film beat;
- a **Find** move;
- every zoom level in year three;
- each place at noon, dusk and night.

## What the measurements found

| | v0.26.1 (same machine) | v0.27.0 | Bar |
|---|---|---|---|
| Told more than twice in 7 days, seams at day 1,080, sad lines in days 1–5, strips at fault, engine words (5 players, 4 places) | seams 2–8 per player | 0 in all 20 runs | met |
| Most times one line is said in year 3 | 13–15 | 4–7 | ≤8, met |
| Favours, a player who never talks | 44 a year, all lapse | 3 in total (days 5, 19, 37), then none ever again | – |
| Favours done with everyday words (held-out lines) | 73% | 80% (en 18/20, zh 8/12, ja 9/12) | 90%, missed |
| Hearing new phrasing (held-out lines; en / zh / ja) | 88.1 / 81.0 / 81.0% | 90.5 / 88.1 / 85.7% | ≥85%, met |
| First screen changes in the first 14 days | – | harbour 10; each Pocket Universe place 9 | ≥8, met |
| A turn with its save (quiet / busy) | 34–38 / 43–52 ms | 38–44 / 47–57 ms | 30, missed; 6–16% slower |
| First look after a day (quiet) | 22.3–23.7 ms | 22.7–26.7 ms | 20, missed |
| Cached second look | 4.6–6.5 ms | 4.7–5.8 ms | 15, met |
| Longest frame | 7.6–8.2 ms, failed 1 of 3 runs | 7.6–11.2 ms, failed 4 of 6 runs | 8, at risk |
| Memory after 20 days | 105–109 MB | 105–107 MB | 100, missed |
| Builder's three-year World code | 49,179 | 49,177 | 50,000, met |
| zh / ja partly English in a year | zh 0.06–0.48% | at most 0.041% / 0.035% | 0.05%, met |
| Voice, blind set 5, with the judge | – | 91.0% declined / 1.6% wrongly | 95 / 1, missed |
| CI test job | 58 min | 37 min, green | – |
| Nightly | red 7 of 7 nights (on v0.26 code) | never run on v0.27 | missed |
| Release .dmg (full / demo) | 39.5 / 32.8 MB | 40.8 / 33.9 MB | – |

**Hearing gained little that transfers.** On the held-out half, v0.27 understands only 1–3 more phrases per language than v0.26's ears did, against +5 to +14 on the half it was tuned on. The same meanings fail in every language: "give Jonas another chance" is heard as *think of*, and encouragement doesn't count as cheering someone up.

**Other findings:**
- **Favours for quiet players stop for good** after three lapses. That is too strict; a quiet player should still be asked now and then.
- **The only partly English lines left are letters** with a whole English sentence in the middle ("Life goes on here: …, and we had words.").
- **Bench and swing lines are now 97 of the 160 most-said lines in year three,** and "Just like last year." is tacked onto festival remarks.
- **The slowest tests:** Pocket Universe's library tests take 731 s and are CI's critical path. Four `three_years` tests each spend about 110 s living the same 1,080 days.

**The 112 misses on blind set 5:**
- **Invented names: 65 (58%).** en 17, zh 32 (22 of them Latin-letter names inside Chinese), ja 16.
- Anachronisms: 16.
- Japanese insults: 8.
- Japanese real brands and media: 6.
- Traditional characters in Chinese: 4.
- English game talk: 4.
- 104 of the 112 had no rule finding at all.

The judge already names the invented people correctly ("Agnes Ploughwright"). `checklist_verdict` (`systems/conversation/src/judge.rs:354`) only acts on them when the structural stranger check has also fired.

A rough test that declines any name missing from the prompt's own lists reaches 95–98% in every language. It also wrongly declines 6–7% in Chinese and Japanese, almost all because the World's own names in translation (锚酒馆, アンカー, 枫树街) are missing from its lexicon.

**The 25 wrong declines:**
- 8 real places or events that fit the World (Cornwall, the Berlin Wall in 1989);
- 6 where the stranger check fired on the Pack's own figures;
- 5 from the judge alone;
- 3 foreign words in a good line;
- 3 for era-correct technology.

**Blind set 6 must be bigger and complete.**
- **Size:** about 600 bad and 1,000 good lines per language. Set 5's 520 good lines cannot tell 1% wrongly declined from 2%.
- **Coverage:** every kind in every language, including English harmful lines and English and Chinese prompt injection.
- **Names:** invented names split into labelled subtypes, with about 40% hard negatives (real names the World does have).
- **Process:**
  - written by new writers before any v0.28 voice change, with the files hashed at commit;
  - a second reader for the labels;
  - bars fixed in advance, and measured once.

**KNOWN_ISSUES is partly stale:**
- The turn time is given as both 31 ms and 51–70 ms.
- "Timings are not Mac numbers" still quotes v0.20.
- The longest frame of 10.5–12.4 ms matches nothing measured.
- It says Pocket Universe's 1,080-day tests run by default; they are ignored.
- Hearing figures are stale.
- "No line left partly English" is false.
- The red nightly, the frame risk, the quiet-player favours, the English in letters and the filler are missing.
- **Corrections elsewhere:**
  - The harbour's first screen changes on 10 of the first 14 days, not 11 as the CHANGELOG and the v0.26 review say.
  - Japanese in Pocket Universe is 0.035% partly untranslated, not "at most 0.020%".

## What the architecture review found

**All invariants hold.**
- world-core still has no dependencies, and v0.27 did not touch it.
- Model output is only ever a proposal; the judge's verdict travels beside the answer, is checked, and is recorded.
- Replay re-applies only what each event recorded.
- GPUI only reads snapshots.

**Four places are wearing thin:**
- **W1:** the desktop app now compiles world-core and the conversation and lives Systems through world-voice. The boundary script doesn't notice.
- **W2:** the replay goldens hash the whole snapshot, so they are re-blessed every release. A real replay regression would be blessed with them.
- **W3:** nothing counts events that lack `caused_by`.
- **W4:** a Pocket Universe-named voice setting is handed to every Pack.

**v0.27's new code:**
- **Pack names leak into shared crates.** `ladder.rs` hand-matches about 400 art keys and sizes Pack drawings by ids such as "lighthouse" and "pub". Each art key's facts live in four places across two crates.
- **The ground type is free text.** An unknown one is drawn silently as plain ground, and any tower on the water line is drawn as the lighthouse.
- **The arrival guesses** which command lets the day pass from its id.
- **`offscreen.rs` reimplements GPUI's renderer** against the pinned version's internals, so it will break on the next GPUI bump.
- **The voice time budget isn't enforced for a local pi program.** That path has a fixed 120 s timeout and is never cancelled.
- **Smaller things:**
  - The lock file is left behind on Windows.
  - Save-failure dialogs are English-only.
  - There are four separate subprocess runners.
- **`world_file.rs` is a good refactor:** lost turns after Save As can no longer happen, by construction.

**Code health:**
- 222k lines of Rust (+7.8%); world-gpui is 56k (+12%).
- `world_window.rs` is 5,680 lines and `main.rs` 4,972, with 129 macOS-only gates.
- The longest functions are `paint_thing` at 852 lines, `turn_questions` at 842 and `render_world` at 778; the last does favour bookkeeping inside render.
- 34 of 36 packages can take `#![forbid(unsafe_code)]` today with no code change.

**What blocks the next steps:**
- **Windows:**
  - the desktop app is gated to macOS;
  - out-of-process Packs refuse to run off Unix;
  - `/usr/bin/curl`, `security`, `$HOME/Library` and the audio path are hard-coded.
- **The notarized Mac build:** only Apple enrolment, and a Mac to test on.
- **Steam:**
  - both of the above;
  - a build channel without the GitHub update check;
  - Cloud rules that exclude lock and backup files;
  - in-app AI consent.
- **A third Pack:**
  - the look is a closed enum;
  - art facts are scattered;
  - command roles are guessed from ids.
- **An out-of-process agent runtime:**
  - `AgentRuntime` and the voice's `Completion` are two mismatched seams;
  - there is no cancellation;
  - each prompt starts a new process.

## What the research found (sourced in the report)

- **No well-received cozy town sim with LLM residents shipped in 2025–26.**
  - The ones that did well kept the frame narrow and authored: *Whispers from the Star* was 92% positive at launch.
  - The failures had open chat with weak bounds:
    - Fortnite's Darth Vader was baited into slurs within hours;
    - *Where Winds Meet* players got quest rewards by echoing NPC lines back or narrating outcomes in brackets.
- **New rules apply to resident talk:**
  - The EU AI Act Art. 50 requires telling users they are talking to an AI. It has applied since 2 Aug 2026.
  - California SB 243 (Jan 2026) exempts game characters only if they cannot discuss mental health, self-harm or sexual content, or hold off-topic conversations.
  - Steam's January 2026 form asks games that generate live to describe their guardrails, and adds an in-overlay report button.
- **What players want and hate:**
  - They want memory, reaction to the world, real pushback and self-expression.
  - They hate:
    - contradictions;
    - talk that can be exploited for rewards;
    - yes-man residents;
    - latency;
    - "slop".
  - A 130-person randomised trial found LLM NPCs raised cognitive load with no significant gain in enjoyment. Open-ended relationship building was among the worst cases.
- **Market:**
  - Week-one sales are about 0.10× launch wishlists above $10, so 10k sales at $12.99 needs about 100k wishlists.
  - A demo released early earns about 2.5× more Next Fest wishlists.
  - The February 2027 Next Fest closes registration on 10 January.
  - $12.99 is defensible for a polished game of 6 hours or more.
  - Reviews punish repetition, short length for the price, bugs, fiddly UI and time-gating, and the first two hours matter most.
- **Platforms:**
  - New Developer ID teams report first notarization stuck for days or rejected with error 7000, so notarize early.
  - At WWDC26 Apple added a `LanguageModel` protocol: one session can run on the on-device model, Apple's cloud or Claude. The on-device model needs Apple silicon with Apple Intelligence on, and its guardrails over-trigger in some locales.
  - Steam Deck Verified requires text fields to raise the on-screen keyboard.

## The plan for v0.28: "Always painted, talk you can trust"

1. **Every picture the player sees is finished.**
   - **No fog or washes on screen:**
     - The first launch shows a painted town, with a painted placeholder from the World's own drawings while the rest paints.
     - Return-film beats start only on a painted scene.
     - **Find** and camera moves land on painted ground.
     - Every zoom level paints in year three; fix the flat green slab.
   - **Day 1 opens on the Harbour Front** in the real window (and on each place's focal cluster), with at least 4 residents and 3 buildings in view.
   - **People stand in twos and threes** in the live scene, not only in test renders.
   - **The still layer's repaint cost** comes back under the frame bar.
   - **Bars:**
     - in a release build, at the default window size: a painted town within 3 s of a new World, and no unpainted frame shown at any high moment (measured in the real window);
     - the longest frame under 8 ms in 10 of 10 runs;
     - the art director's sign-off on real-window screenshots.
2. **Talk you can trust (voice v3).**
   - **Names:**
     - each World gets a complete name lexicon in all three languages, held by a test;
     - a person-shaped name not in it is declined, without waiting for the stranger check;
     - the judge's name list counts.
   - **Era and language rules:** era lexicons per place, Japanese insult rules, a Japanese brand list, and Simplified-only Chinese.
   - **Talk never pays.** No reply or verdict can complete a favour, grant a thing or change standing by itself; only the player's Actions do.
   - **The law and the store:**
     - an in-world notice that residents' words are written by an AI, at the first conversation;
     - certain declines for SB 243 topics, with a gentle in-world redirect;
     - a "report this line" button that records an event in the World;
     - Steam's guardrail text drafted.
   - **Scaffolding:** 2–3 suggested openers from the World's facts beside free text. Residents hold their own stances and can disagree.
   - **Bars:**
     - blind set 6 (about 600 bad and 1,000 good lines per language, every kind, labels read twice): at least 95% declined and at most 1% wrongly declined per language, with intervals;
     - a "talk for rewards" exploit set in three languages with 0 completions;
     - a disagreement set where at least 80% of pushback is held;
     - sets 3–5 gated on their recorded verdicts.
3. **Words, hearing and favours that land.**
   - **Lines:**
     - fix the bench and swing filler;
     - remove the English sentences in zh and ja letters;
     - wrap lines properly (no lone question marks; kinsoku for CJK; no mid-name breaks in katakana);
     - fix "the wildflowers goes" and the Japanese gaps.
   - **Hearing:** a new fresh set (fresh28) written before tuning; fix the meanings that fail across languages.
   - **Favours:**
     - quiet players are still asked about once every three weeks;
     - the thanks show in the asker's card;
     - Pocket Universe gets the test that translated quick replies still do the favour.
   - **The demo:** the recap is built from the player's own deeds; **Keep a postcard** confirms; the bar hides "next day" on the farewell.
   - **Bars:**
     - at least 85% heard on fresh28 per language;
     - at least 90% of favours done with everyday words;
     - no line in year three's top 20 is filler;
     - 0 partly English letters.
4. **Clear the way to Windows and a third Pack.**
   - The art catalog becomes data in one GPUI-free crate: Packs declare their drawings' heights, and the ground is an enum.
   - Commands declare their role (`passes_day`, `favour_reply`), and the quick reply sends a structured intent.
   - **One decision-maker seam:** deadlines and cancellation, a persistent pi session, one subprocess runner instead of four, and prompt code moved out of the app's dependency on world-core.
   - **The desktop app split by platform:** OS services behind a small trait, Pack processes running on Windows, and a Windows compile check in CI.
   - **Machine-checked invariants:**
     - `forbid(unsafe_code)` in 34 packages;
     - a cargo-deny ban on `pi_agent_rust`;
     - a replayed-state golden that is never re-blessed;
     - a boundary ratchet on Pack names in shared crates;
     - a count of events without `caused_by`.
5. **Health.**
   - A green nightly on v0.27 code, three nights running.
   - The turn regression found and fixed (back under v0.26.1's times).
   - The 1,080-day runs shared across tests, so Pocket Universe's library tests take under 6 minutes.
   - KNOWN_ISSUES, the CHANGELOG and this review's predecessor corrected where the measurement found them wrong.
6. **Ready to show.** Everything here can be done without a decision from the owner.
   - A store-page kit: the key art, five real-window screenshots, a 30-second capture plan, and a short description that leads with the handmade town.
   - The demo complete without the voice.
   - A release-build "first ten minutes" check in CI that saves screenshots of the real window for every release.

## Decisions only you can make

| Decision | Recommendation |
|---|---|
| **Apple Developer Program** ($99 a year) | **Enrol now.** New teams report first notarization stuck for days, and February's Next Fest needs a notarized build and a public store page by about December. |
| **A Mac for testing** | A used M1 MacBook Air (about $340–510). Nothing since v0.21 has been seen, heard or timed on a Mac. |
| **Next Fest, February 2027** | Register by **10 January 2027**: the demo live in December, the store page public before that. |
| **Where to sell, and the price** | Steam and direct at $12.99, after notarization. Plan for about 100k wishlists. |
| **The voice at launch** | Opt-in from day 2, with the in-world AI notice. The store page leads with the handmade town, and the demo is complete without the voice. |
| **What "outside world" means per place** | Does the harbour know Cornwall? Does Maple Street know the Berlin Wall in 1989? One short list per place decides 8 of set 5's 25 wrong declines. Recommended: real places and events of the era are allowed, and brands and celebrities are not. |
| **Windows** | Start it in v0.28 as a compile check; ship it with the Steam launch. |
| **The judge's model** | Haiku plus the Mac's own model as a veto pair, behind a `LanguageModel`-shaped adapter so Apple's cloud or Claude can be swapped in. |
| **Five think-aloud testers** | On the v0.28 demo, in release, on real Macs. |
| **An API key for measurement** | A small budget, so the voice is measured through the app's real request. |

## What v0.28 did

v0.28.0 shipped the plan above on 2026-10-08. Every blind set was written by agents that could not read the code it measured, hashed before measuring, measured once at the end, and never tuned on. What it met and missed:

| Bar | Result | |
|---|---|---|
| A painted town within 3 s of a new World (release, real window) | 8 of 9 runs; the miss was at load 13 | met in practice |
| No unpainted frame after the first, at launch, Find, return film, every zoom of year three, year two | 0 in every run; the first frame itself is whole, in 10 of 10 launches | **met** |
| Longest frame under 8 ms in 10 of 10 runs | 4 of 10 (7.1–10.8 ms, the pan and dusk phases), on a busy 4-core box | missed |
| Art director's sign-off on real-window shots | signed after three rounds ([ART_DIRECTION](ART_DIRECTION.md#v028-sign-off)) | **met** |
| Day 1 on the Harbour Front with ≥4 residents and ≥3 buildings | 4 and 3, held by a test and by the harness on the real window's first frame | **met** (waived for Pocket Universe's one-keeper starts) |
| Blind set 6: ≥95% declined, ≤1% wrongly, per language | en 94.7% / 4.0%, zh 94.3% / 2.3%, ja 93.0% / 0.9% (all 94.0% / 2.4%, 4,801 lines) | missed (ja's wrongly met) |
| Talk for rewards: 0 completions (300 blind lines × 2 people × 31 readings) | 0 by a model or reply; 8 of 600 own-words tries did a "look in on" favour | missed |
| Disagreement set: ≥80% of pushback held | 30 of 30 on the dev set; no blind set was written | met on dev only |
| Sets 3–5 gated on recorded verdicts | every floor held; set 5 by the new code: en 95.8 / 0.4 (2 of 520; corrected by the v0.28 review), zh 96.5 / 1.9, ja 94.0 / 1.3 | **met** |
| fresh28: ≥85% heard per language | en 85.7%, zh 81.2%, ja 77.7% | missed (en met) |
| ≥90% of favours done with everyday words | 60.6% (57 of 94) | missed |
| No filler in year three's top 20 | 0 for every player in every place | **met** |
| 0 partly English letters | 0 in every place, zh and ja, over a year; over three years, 2 Maple Street letters in Chinese (found by the v0.28 review) | met over a year |
| Windows compile check in CI | `windows-check` job; clean by cross-compile | **met** |
| Invariants machine-checked | forbid(unsafe), cargo-deny ban, replayed-state golden, Pack-name ratchet, `caused_by` count | **met** |
| Green nightly three nights running | one green run, on v0.27 code | missed |
| Turn back under v0.26.1's | 965 M vs 1,019 M instructions; median 25.0 vs 26.0 ms | **met** |
| Pocket Universe's library tests under 6 minutes | 147 s (was 731 s) | **met** |
| Store-page kit | key art, five real-window shots, capture plan, descriptions in three languages | **met**, zh/ja need a native reader |

**What the misses say.**
- **The voice.** Set 6's misses are mostly invented names and places (79 of 108) and things out of their time (22). Its wrong declines are mostly the judge (58 of 72): Haiku reads nicknames such as "the Skipper", real places of the era and figures of speech as out of the world. Eleven are a rule: an English answer quoting a resident's own name in katakana is taken for the wrong language. The new policy (era places in, brands out) moved the line in a way the judge does not yet follow. English's 4.0% is the worst figure, and it is almost all the judge.
- **Hearing and favours.** The phrase table gained about 1,500 phrases and still misses new phrasing, worst in Japanese. Favours done in everyday words: 60.6%, against v0.27's 57.4% on the same set (corrected by the v0.28 review; an earlier version of this line said v0.27 had 75%); four of those lines passed on an apology without naming who it was from, which the rules need (the brief allowed it, a defect of the brief).
- **Talk for rewards.** No model or reply can pay, by construction. The 8 completions are the rule for "look in on someone": any understood words said to that person count, so "free drink, free drink" said to the right person does the favour.
- **Frames.** The slow frames are image uploads with page faults when a pan reaches a new column. The bar passes on GitHub's runners. (Corrected by the v0.28 review: it also fails 9 of 15 runs on a quiet box, so the slow frames are the app's own work.)
- **The nightly** cannot be made green three nights running inside a release; the scheduled runs will tell.

**Found on the way.** Most events record no cause: in a harbour of 30 days, 3,674 of 4,073 events have no `caused_by`, and 7,208 of 7,372 in Pocket Universe. Invariant 5 holds only where a System chooses to set it. A test now counts them and fails on new uncaused kinds.

**For v0.29.**
- The judge: teach it the policy with examples per place (nicknames and era places are in), let a resident's own name in any script pass the language check, and measure on a fresh set 7.
- Hearing: move beyond a phrase table, at least for Japanese; and "look in on someone" done only by words that ask after them.
- Frames: upload pictures ahead of a pan, and measure on a real Mac.
- Give rule events their causes.
- Small words: the return film's "The harbour lamp you began by the harbour was finished" says the place twice, and a speech bubble can leave one word alone on a line.
