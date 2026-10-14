# World Machine v0.29: a step back, and the leap v0.30 should make

Reviewed on 2026-10-14 at `412d8cf` (v0.29.0, plus the nightly fix of #346).

The owner asked for a qualitative leap, not another round of fixes. So this review steps back further than the last six. It has three parts, each written independently:
- **Play and art:** v0.29.0 built in release and played in its real window under Xvfb by a critic who is also an art director. They covered:
  - a new World from day 1 to day 25, and its return after 72 hours;
  - a 420-day harbour;
  - each Pocket Universe place.

  About 150 screenshots, plus the text of the 420-day World read from its event log. No model key was available.
- **Architecture:** the whole tree and its git history from v0.23 to v0.29, the CI logs and the Actions history. Nothing was built.
- **Research:** web sources with URLs, dated 2026-10-14. As in the last two reviews, pages could not be opened through the network proxy. Every fact comes from search summaries, and the sales and player counts are third-party estimates. Read them again before money is spent on them.

The full reports and screenshots are kept in the review workspace.

## The verdict

**Twenty-nine releases have polished how frames look. None has changed what playing feels like.**

The craft is real:
- an event-sourced history that replays exactly, with a recorded cause for 90% of what happens;
- chapter cards with the best writing in the game;
- moment strips;
- the Ares cards ("there are no bees on Mars. Help me pollinate them with a paintbrush?");
- a dusk in year two under bunting.

But nobody would tell a friend about it yet. The critic expects most new players to stop between minute 15 and 25, for three reasons.

**1. The town's life is told, not shown.** The 420-day World holds 11,124 events, 6,156 of them residents living their lives. Friendships form, children are born, the bakery closes, three great storms pass. On screen the player sees two or three people standing still:
- No one walks, carries, works or sits.
- A storm is a caption ("Sky's turning black out west").
- The outing you arranged with Leo is agreed in words and then never happens.
- The return film is nine captions over standing figures: "Leo rested on the swing" does not show Leo on the swing.
- Chapter 1's card ("Jonas and Mia became firm friends. Evan and Emma started walking out together… Mara hasn't forgotten being let down") describes a month the player never saw.

![The return film: captions over people standing still](review/v30-r-beats.jpg)
![Chapter 1 ends: the best writing in the game, about things you never saw](review/v30-h1-chapter.jpg)

**2. The player has no role and thin verbs.** You are an unexplained benefactor with a purse ("Money in town 3,835"). The loop is: read a card, pick one of two or three options, press Enter.
- "Everyone helps" is offered almost every time.
- Choosing "Buy the timber" makes a whole pier appear at once, with people already on its deck. There is no waiting and no making.
- Building is dropping one of 21 props on one strip of quay, and the swing snapped away from where it was clicked.
- Nothing you place is used on screen: the record says residents "rested on the bench", but no one is ever seen sitting.

![Buy the timber: the whole pier appears at once](review/v30-h1-pier.jpg)

**3. Time gives no reason to come back.**
- The bar says "next day in 6 h", yet Enter passes days without limit: the critic lived 25 days in 9 minutes and had seen the whole loop.
- Away time runs at four town days a real day, so a return after three days is twelve days of other people's news.
- Nothing you planted needs you tomorrow.

**And the residents do not become people:**
- Without a key, talk is a 28-intent parser. It fails on the most natural question: asked "Is the pier hard to build?", the carpenter said "I'm not sure what you mean. Ask me how I am…". On Mars, "Do you ever miss Earth?" got the same reply.
- All 78 letters in the 420-day World fill one template: opener, then two names, then "became firm friends" or "fell out", then sign-off.
- "Sold odds and ends outside Anchor Pub" is told 30–50 times per resident a year, and only 2,556 of 7,817 told lines are distinct.

![Asked about the pier, the carpenter does not understand](review/v30-h1-evan.jpg)

**The look is competent, tasteful and still.** Night windows, Ares's habitats, the year-two dusk and the strips would sit comfortably in the Steam cozy mid-tier. Animal Crossing, Stardew, Spiritfarer and A Short Hike sell constant small motion; this frame is wallpaper:
- People are about 50 px tall in a side-on strip four and a half screens wide.
- 40–50% of each frame is inert sky and hills.
- Zoomed out, a year-two town is houses scattered on a beige field, with no streets and nothing joining one building to the next.
- A year-two noon shows 2 people; 15 are "in work".

![A year-two town at noon: two people on screen](review/v30-y2-noon.jpg)

**The engineering shows where the effort went.** Since v0.23 the tree grew from 175.6k to 246.5k lines of Rust, about 37% of it tests:
- **The painter:** the CPU-painted diorama and its painter are about 40k lines. They took 27% of all changed lines, and their golden tests take 58% of CI test time (907 of 1,569 s).
- **v0.28 and v0.29** went mostly on one class of bug, the rough stand-in showing where the player looks. That bug exists only because the scene is painted on the CPU in tiles.
- **Golden churn:** the 64 continuation digests were re-blessed six times in v0.29 alone, and 10 of its 33 commits touched goldens.
- **The split:** roughly 55–65% of v0.28 and v0.29 went to making existing things look and read right, plus golden, test and translation upkeep, and 20–25% to new things a player can do.

**Meanwhile the architecture's best abilities are barely used in play:**
- exact replay of recorded events;
- forking a World before any event (`fork_after` and `ForkBeforeEvent` work in both Packs, with no way to reach them);
- a year simulated headless in about 10 s;
- a recorded cause for nine events in ten.

**The research agrees with the critic.** The comparable hits grew on three things this game is thin on:
- a reason to have it in front of you every day (Neko Atsume, Rusty's Retirement, Bongo Cat);
- something unmistakably yours to make and show (Animal Crossing: New Horizons, Tiny Glade, Townscaper);
- stories players retell (Dwarf Fortress, RimWorld, The Sims, Wildermyth).

Open chat as the headline is weak: AI-native games on Steam mostly sell in the thousands, and games that disclose AI get about 53% fewer reviews. Language models earn their place when they react to what the player expresses, inside rules the model cannot break, with the output recorded rather than regenerated. That is exactly what this kernel does.

**All three reviews, written independently, point the same way:** make the life the simulation already produces visible, as a town that moves and a story told in scenes. Then build the daily return, sharing and the desktop companion on top of it.

## What the player found, in full

### The first hour, minute by minute

- **0:00.** A black frame, then the harbour on day 1. Leo: "Oh, a new face! I'm Leo. Welcome to the harbour." Nothing says who you are or what you are for.
- **0:10.** "Make something" opens unasked, offering free wildflowers.
- **0:20.** Clicking the roped plot puts a 70 px planter on the quay in front of it, not in it.
- **0:40.** The pier question fails (above). Every canned line carries a Report link.
- **1:30.** The pier appears whole (above).
- **2:00.** The hands catalogue has 21 props, and the swing snaps to the quay strip, overlapping the plot.
- **2:30–4:20, days 2–6.**
  - Three or four cards a day, and tooltips one after another.
  - One good causal echo: the oven was patched, and later Mara says "The patched oven's given up mid-batch".
  - Day 5's Spring Clean strip is the first real delight.
- **5:00.** The book says "13 of 78", and every keepsake is the same brown parcel. The one letter: "I wanted to tell you something. Not much news, only this: I ran errands for Jonas. Come by soon."
- **5:30.** A favour. Asked to watch the sunset on the pier, Leo answers "I'd love that. Mia put you up to this, didn't they?" Charming, and then nothing happens on screen.
- **6:00–8:30, days 7–13.**
  - Chapter 1 ends (above).
  - Day 13's "swallows" strip reuses Spring Clean's composition and its "around the swing you made".
- **Return after 72 hours.**
  - Home shows a lilac placeholder and "12 days have passed".
  - The film is captions over standees.
  - It ends on the day-1 pier card again: "Here we are again. Last time round…".

### Pocket Universe

- **Ares** is one keeper on a plain. The card writing is the best in the product, but the card's speaker, Tomas, is never on screen.
- **Soft lock:** in Ares and Maple Street, while the free bench offer is open, 9–15 presses of Enter advance nothing, and nothing says why.
- **Maple Street:** "Want to go to the arcade with me?" works, because place keywords are matched.

### The other problems, ranked by how much they block love

1. **The life is told, not shown.**
2. **No role, and thin verbs.**
3. **Time works against coming back.**
4. **Residents do not become people:** the parser, the template letters, the repeats, and templated pronouns ("Could I learn from them?" about Jonas).
5. **Expression and friction:**
   - placement ignores the click;
   - a gift parcel on the quay cannot be clicked;
   - the postcard toast hides behind the card;
   - Report appears on canned lines;
   - sharing is a 38,105-character code.

## What the architecture review found

**The layers are healthy:**
- **Kernel:** `world-core` is 3.2k lines, has no dependencies, and keeps every invariant.
- **Systems:** 32k lines.
- **Packs:** 61k lines, run out of process.
- **Voice:** 14k lines, bounded, and proved never to pay.

**The cost centres:**

| Cost | Size | What it costs |
|---|---|---|
| CPU tile painter and diorama | ~40k lines in `world-gpui` (63.6k total, doubled since v0.23) | 27% of changed lines, 58% of CI test time, most of two releases; keeps the town still |
| Goldens and digests | 64 continuation digests, 33 PNG goldens, 32 `.world` fixtures | re-blessed on every intended change, so they almost never catch a real bug; save compatibility is guarded separately, by event replay |
| Content | English inside Rust, plus zh and ja `.tsv` compiled into the app | one new line touches three languages, slot tests and digests; a new place needs an app-side enum and a 1.4–2.4k-line `works/*.rs` |
| Giant files | `world_window.rs` 7,122 lines (+23% in v0.29); `conversation/lib.rs` 5,948; `lives/lib.rs` 5,424; desktop `main.rs` 4,688 | agent collisions, and "found on the way" regressions every release |
| CI | PR test job 32.5 min; nightly red on 13 of the last 15 nights before #346 | slow feedback, alarms ignored |
| The only eyes | Xvfb with software Vulkan, 5 fps on 2.9 cores | no release since v0.21 seen on a real Mac, none ever played on Windows; four art rounds judged frames no player will see |
| Voice gating | ~25k lines plus ~130 corpus files | off by default, unmeasured with a real key, carries the disclosure and legal cost |

**What it is great at, unused in play:**
1. **Replay.** Saves from v0.20 onward replay event for event. Play uses this only for the return film and History.
2. **Forking.** It works in both Packs. `world-compare` (1.6k lines) is still in the tree, and "What If…" is still in the zh and ja catalogs, but no UI reaches any of it: it was cut in v0.18 as an analyst's tool.
3. **Headless speed.** A turn is 4 ms for a day passing, 23 ms with its look at three years, so a year runs in about 10 s.
4. **Causes.** Fewer than 10% of events now lack a cause.
5. **Out-of-process Packs, guests and World codes.** The start of a social layer.
6. **The desktop strip and near-zero background cost** (about 1% of a core when covered).

## What the research found (sourced in the report)

- **Animal Crossing: New Leaf → New Horizons** leapt on expression and sharing, not content:
  - terraforming, added because players were resetting their islands to get the layout they wanted;
  - Dream Addresses, offline visits that cannot change the island.

  Its failure mode was chore burnout and droughts between updates.
- **The Sims 1 → 2** leapt on ageing, memories and wants together. World Machine has ageing; memory of *the player* and visible wants are the lesson.
- **Tiny Glade** (two people, Rust) sold 616k in under a month on pure expression and screenshots, with a median playtime of about 1 hour. Townscaper calls itself a toy.
- **Rusty's Retirement** (an idle farm in a strip at the bottom of the screen) sold 500k+. **Bongo Cat** holds about 170k concurrent players and added friends on your desktop.
  - **Desktop Mate** fell from about 23k to 1.6k concurrent players in three months: a companion with no living world is a novelty.
- **Stories players retell** are the long tail:
  - Dwarf Fortress's Boatmurdered and Legends mode;
  - Wildermyth's comic panels and legacy heroes;
  - Spiritfarer's farewells.
- **Stanford's Generative Agents** were compelling because news spread between agents (a party invitation, dates, turning up on time), not because of chat. Infinite Craft records each generated result once, for everyone; that is this kernel's principle. Where Winds Meet's AI NPCs were tricked into paying out rewards, which is exactly what this game's "never pays" bound prevents.
- **AI disclosure applies to us regardless.** docs/press/ai-disclosure.md says the shipped lines, scenes, letters and translations were written with a language model's help. Steam treats that as pre-generated AI content, so the label applies whether or not the voice ships.
- **Steam is 94% Windows.** June 2027 is the realistic Next Fest (February's registration closes on 10 January 2027), and Wholesome Direct lines up with it.

## Candidate leaps

| | Leap | The player's promise | Why it's a leap | Size | Verdict |
|---|---|---|---|---|---|
| **A** | **The town you watch live** | Your town really lives on screen, and every story is a scene you can replay and share. | Turns the biggest hidden asset (a causal, replayable history of ~6k lived events a year) into the experience; fixes the painter's root cost at the same time | Large: about two cycles | **Recommended for v0.30** |
| B | The harbour beside you | A tiny town lives along your screen's edge all day; step in for five minutes. | The strongest 2024–2026 commercial evidence for tiny desktop teams | Medium, but needs A: CPU painting is too costly to run all day | v0.31, on top of A |
| C | The other road | From any choice, see what would have happened, and step into it. | Nothing else in the genre can do it; uses forking, replay and causes | Medium | v0.31–v0.32 |
| D | Build it with your hands | Shape lanes, gardens and a square, and the residents take them over. | Expression, the ACNH and Tiny Glade lesson | Large renderer and layout rework | Later, and only once A makes residents use what you place |
| E | Someone to care about | A few residents become yours over weeks: written arcs, letters about you, farewells. | Attachment, the one thing every peer has | Medium-large writing, in three languages | Folded into A, at a small scale |
| F | Residents with minds (AI as the core) | Talk to anyone about anything. | — | — | **Not recommended:** the guard is below its bars, the disclosure and legal costs are measured, and the evidence says it sells poorly as a headline |

## The plan for v0.30: "The town you watch live"

**One sentence:** the life the simulation already makes is shown on screen. Residents walk, work, carry and gather. Festivals, storms, outings, arrivals, births and builds play as short scenes. The return film is a cut of those scenes, replayed from the record with no model. And one day a day gives you a reason to come back.

### S0. The spike: go or no-go (first week, before anything else)

Bake the existing `tiny-skia` brush code's buildings, props, works and people into sprite sheets: 2–3 zoom levels and a few lighting variants, at load or build time. GPUI then draws them each frame as images.

**Go if all three hold:**
1. About 300 sprites move at 120 Hz with each frame planned in under 8 ms, **on a real Mac**.
2. One baked harbour frame earns the art director's yes against the bible.
3. Glow and light read well as layered images, since GPUI has no custom shaders.

If it fails, A falls back to keeping the painter and adding moving people and scenes as live layers over the still town. That is slower and less alive, but still the leap in substance.

### S1. The living stage

- Sprites composited on the GPU replace the tile painter. The rough stand-in, the tiles, the paint-ahead queues and "painted per region" accounting are deleted, probably 8–12k lines.
- A closer stage camera, with people at 120–160 px instead of about 50.
- One continuous ground that joins buildings with paths and lots.
- The interface hidden at rest, with numbers moved into the book.
- One hero palette per place with real value contrast, and dusk lit rather than tinted.

**Bars:**
- No rough painting ever, because there are no tiles.
- 120 Hz on a real Mac.
- The art director's free-play sign-off, on a real device.

### S2. Residents who move

- Six to eight animated verbs: walk, carry, sit, work, wave, gather or dance.
- Driven by what the record already says each resident is doing: at the stall, in the pub, on the quay, on the bench you placed.
- Things you place are used visibly: a bench gets sitters, a lamp gathers people at dusk, a stall gets a keeper.

**Bar:** at least 6 residents visibly doing something on any first screen of a lived town, by day.

### S3. Every story is a scene

Each type below plays as a staged scene of 5–10 seconds, with the camera, people and props moving. Together they cover about 80% of the events that reach the player:
- festival
- storm
- outing (the favour you arranged)
- arrival
- birth
- quarrel and making up
- a build going up by hand

The return film becomes a cut of these scenes, replayed from the recorded events. Invariant 6 makes that free of any model. Chapter cards follow the scenes they describe instead of replacing them.

**Bars:**
- Every return-film beat shows its event happening.
- A stranger watching a 20-second clip can say what happened (4 of 5 testers).
- Arranged outings always play.

### S4. One day a day

- The town's day follows the real day, or follows each sitting, by the owner's choice (decision 2).
- Enter no longer passes days without limit.
- Away time no longer compresses into twelve days of news.
- A small daily ritual:
  - a morning letter written to you, about your shared past rather than a template;
  - one choice with weight;
  - one thing to tend overnight (a garden, or a build you finish by hand);
  - one find for the book.
- Never punitive: missing a day costs nothing (the Neko Atsume lesson).

**Bars:**
- Every day has something new that is yours.
- No day's ritual repeats a line.
- A five-day think-aloud test (decision 3).

### S5. Residents become people (small, focused)

- **Letters:** about you and what you did together, replacing the one template.
- **Pronouns** fixed.
- **Hearing grounded in place and occupation:** the carpenter knows the pier, the keeper knows Earth.
- **Repeats:** none of the "lived" lines told more than 5 times a year per resident.
- **Heart arcs:** one written arc per core resident, spanning several cards and favours. Two residents per place, in three languages.

### S6. Share a story

- Export a scene or the return film as a short clip (GIF or MP4), captioned in the reader's language.
- A short World link or file replaces the 38,105-character code.

**Bar:** a clip plays and reads correctly with no app.

### S7. Friction first (first days, in parallel with S0)

- the Pocket Universe free-offer soft lock;
- placement that ignores the click;
- the unclickable parcel;
- the postcard toast hidden behind the card;
- Report on canned lines;
- the black first frame.

### S8. Pay for it: what to stop and delete

The stop list:
1. Replace the 64 continuation digests with event-replay checks, which is what save compatibility needs. Keep one replayed-state digest per Pack, re-blessed only at release.
2. Move golden render tests out of PR CI and into the nightly in release mode, after giving `world-gpui` an optimisation level in debug and measuring. The PR run should be under 15 minutes.
3. Stop releasing daily with 490-line reviews and full KNOWN_ISSUES rewrites. Release when a slice is playable, with a short changelog.
4. No art rounds on Xvfb frames. Sign-off happens on a real Mac or PC (decision 3).
5. Freeze the voice: no new red-team sets, judge work or lexicon growth until there is a key and a ship-or-cut decision (decision 5).
6. Retire wall-clock asserts in debug tests. Run the nightly on the release branch too.
7. Split `world_window.rs`, desktop `main.rs` and `conversation/lib.rs` before S1 starts (about one agent-week of mechanical work).
8. Delete what nothing uses. Keep `world-compare` and the fork intent for leap C.

### The order

1. S7 and S8's splits and CI cuts, alongside the S0 spike.
2. The go/no-go.
3. S1, then S2 and S3 together.
4. S4 and S5.
5. S6 last, because it needs scenes worth sharing.

S2 and S3 are the leap itself. If time runs short, S6 and the S5 arcs move to v0.31; S1–S4 do not.

### What would prove the bet wrong

Five think-aloud players watch a lived town for ten minutes and cannot say what anyone is doing, or a stranger shown a 20-second scene cannot say what happened. Run that test on the S0 spike's frames, with even one scripted scene, before committing past it.

## Decisions only you can make

| # | Decision | Options | Recommendation |
|---|---|---|---|
| 1 | The leap | A "the town you watch live" / B the desktop companion first / C the other road / D building | **A**, with B as v0.31 on top of it |
| 2 | Time | one town day per real day / one day per sitting / keep free Enter and add the ritual | **One day per sitting**, capped, with a non-punitive real-day ritual; the softest version of the return hook |
| 3 | A real Mac and a real Windows PC | buy or borrow now / keep judging on Xvfb | **Now.** The S0 go/no-go and every art sign-off depend on it; the last nine releases were never seen on a real screen |
| 4 | AI disclosure | keep the label (the shipped text was written with a model's help) / have a person rewrite the shipped text | Keep the label for v0.30, and decide before the Steam page goes up; the label applies whether or not the voice ships |
| 5 | The voice | freeze / cut from the default build / ship it with a key | **Freeze** until there is a key to measure it with |
| 6 | Cadence | daily releases / a release per playable slice (about weekly) | **Per slice.** v0.30 is about two cycles of work; pre-releases can go out for testers |

## Status of v0.29's work since release

- **Nightly bars** were red every night after v0.29 merged. #346 fixed them: the blind-set test is skipped, a memento and the work it marks count as one telling, and four Pocket Universe bugs from v0.29 are fixed. It ran green on its branch, and the first green run on `main` is pending.
- **The release workflow's publish step** failed after publishing v0.29.0, because a Windows file was named as a plain path in a `nullglob` array. #345 fixed it.

## The owner's answers (2026-10-14)

The owner accepted every recommendation:
1. **The leap:** A, "the town you watch live", with the desktop companion (B) as v0.31 on top of it.
2. **Time:** one day per sitting, capped, with a non-punitive real-day ritual.
3. **Real devices:** a real Mac and a real Windows PC go in the loop. The S0 go/no-go and every art sign-off are judged on them. Until a device is to hand, the spike is also measured on CI's macOS runner (real Metal, a virtual GPU) as an interim reading, and the owner runs the spike's benchmark build on their own Mac.
4. **AI disclosure:** keep the label for v0.30, and decide on a human rewrite before the Steam page goes up.
5. **The voice:** frozen: no new red-team sets, judge work or lexicon growth until there is a key and a ship-or-cut decision.
6. **Cadence:** a release per playable slice, with pre-releases for testers.
