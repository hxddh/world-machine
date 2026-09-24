# Product review — v0.5.2 plus the v0.6 Pack content (2026-09-24)

**Verdict:** the engine is real, but what a user sees isn't a product yet. Every release so far has been judged by measuring the code. Nobody looked at the screen, and the screen shows it.

This review reads the product the way a user meets it: the app window, not the tests. Every screenshot below comes from the real desktop app running under `scripts/linux-preview.sh`, which this change adds. It is the first time the app has been looked at on a machine without a Mac.

## What was wrong

### 1. Nobody could see the app, so nobody did

- The desktop app is gated to macOS, and the CI screenshot job can't draw text. `scripts/check-desktop-fonts.py` records that v0.5.0 and every earlier release shipped windows with **no text in them at all**, and nothing caught it for five releases.
- `docs/screenshots/` doesn't exist. The README describes a window it has never shown.
- Consequence: layout bugs no test can see. The World map ("Explore the world") had collapsed to a one-pixel line and never showed in any release.

### 2. The World window was a debug inspector

![World window before](review/world-before.png)

- **Three equal columns**: a list headed "World Contents", a centre grid of identical boxes, and a 300 px event log. Nothing says which one to read first.
- **The log is the kernel's raw log**: `t=110`, `Agent Decision Recorded · Tomas Vale · Event #71`, one line per internal step.
- **Jargon on every choice**: *"Choice signal: one full cycle resolves under current rules: world growth, both actor turns, relationship update, then period consequences."* *"Verified by this Event: World direction = Rooted at generation 6. Later growth and legacy formation read this durable posture."*
- **The decision was said three times**: once as a briefing card listing every option and its mechanics, again as a button panel, and again in "Choice evidence".
- **One weight for everything**: 156 `text_sm`, 196 `text_xs`, and no font weights anywhere. Every heading had the same weight as the body text.
- **The title twice**: the World's name appeared in two stacked title bars.
- **Kernel words in the details panel**: *"Recorded events whose StateChanges directly changed this entity"*, *"Relation incarnations … Removed relations remain inspectable as recorded tombstones"*.

### 3. There was no design system

- 519 colour literals, 126 of them different colours, written directly into the view code.
- Dark mode came from a formula (`world_theme::darken`) that inverts lightness, not from a palette anyone chose.
- **Zero** hover states in the whole app. Every clickable thing was a `div` with a border, identical to things that aren't clickable.

### 4. Home

![Home before](review/home-before.png)

- Each World card had three stacked buttons of different widths, plus developer data: `World time 110 · 75 events` and the file id `ares-pocket-colony`.
- The screenshot fixture saved Worlds without names, so every screenshot listed both Worlds as "Pocket Universe".
- After the bundled Packs activated, Home showed **"Pocket Universe is ready — Start your first World · no World created yet"** to someone who already had two Pocket Universe Worlds.
- Status messages were inserted above the page, so a Pack finishing start-up pushed the card under the pointer down. Opening "Maple Street · 1987" announced *"Opened Pocket Universe"*.

### Root cause

The team's feedback loop was code measurement: "lines stopped repeating", "83% → 0% repeat". That is necessary, but it cannot tell you a window is empty, a panel collapsed, or a sentence is unreadable. `NEXT_TASK.md` already says a real-device check and a usability test "outrank everything". Neither has happened since v0.2.1. The rest of this document is what that loop would have caught.

## What this change does

![World window after](review/world-after.png)

**A design system** (`world_theme::tokens`, `world_gpui::ui`)
- Named colour roles (window, surface, sidebar, text ×3, accent, success/warning/danger), each with a light and dark value chosen by hand.
- Tests check contrast for both appearances: body text ≥ 7:1, secondary text ≥ 4.5:1 (WCAG AA), labels ≥ 3:1.
- One type scale (page title / heading / row title / body / detail / label / caption) with real font weights.
- `button` (primary, secondary) and `list_row`, all with hover states.

**The World window reads like a page, not a log**
- One reading column, at most 700 px wide. From top to bottom:
  - where and when you are;
  - the headline;
  - **the story**, told in order as a thread of beats;
  - **Your turn**, the one panel on the page with an accent, where each choice is a row that answers the pointer;
  - "Where things stand", for the standing facts;
  - "Look closer", with the map (fixed, it now renders), the details, *Why it happened*, and *What it led to*.
- A sidebar with **People and places** and a **History** grouped by moment ("Time 110"), not stamped with `t=` and `Event #` on every line.
- Below 920 px the sidebar folds under the page instead of squeezing it.
- The desktop window has one title bar with **Branch** and a primary **What if…**.
- The details panel uses plain words: *Connected to*, *What changed it*, *Between*, *What this changed*. Raw payload and state-change tables of ids are hidden here; they stay available to the CLI and the Analyst.

**Copy**
- Choices describe themselves in the World's own words. "Choice signal" mechanics are gone from the screen, and every consequence stays traceable in *Why it happened*.
- "Choice evidence · X — Verified by this Event: …" now reads **"You chose · Rooted — This World now leans rooted. What grows next, and what it leaves behind, follows that direction."**
- Trust and tension are no longer stated twice in one card. *"Resolved into a durable partnership"* now reads *"They have settled into a lasting partnership."*
- "Event #N" is gone from timeline lines and detail subtitles.

**Home**

![Home after](review/home-after.png)

- Each card has the World's name, a one-line summary, what kind of World it is and how far it has lived, one primary **Open**, and quiet text actions (What if… · Rename · Export… · Remove).
- **Start a new World** is a grid of cards with a clear "Start a World →", and the recommended World first.
- The false "no World created yet" card is gone.
- Status floats over the bottom of the window instead of moving the page, and it names the World you opened.

**Seeing the app without a Mac**
- `scripts/linux-preview.sh` copies the tree, lifts the macOS gate in the copy only, and runs the real app under Xvfb with the bundled Packs and a demonstration library. It can click, and it captures every window. It doesn't replace checking on a Mac, but it closes the "nobody looked" gap for everyday changes.
- `cargo run -p pocket-universe --example playthrough` prints fourteen visits as text, so the copy can be read end to end without the app.

## What is still not good enough, in order

1. **Check on a real Mac.** San Francisco has real weights. The Linux fallback font here does not, so headings look lighter in these screenshots than they will on a Mac. Capture `docs/screenshots/` in light and dark.
2. **The other windows are untouched**: What if…, lineage, Settings, the Analyst panel, Pack review. They still use literal colours and `adapt()`. Move them onto `ui` and `tokens`, then delete `adapt()`.
3. **Recorded prose still has engine words.** *"Legacy cycle 8 is now a durable adaptive pattern."* is written into each `legacy_reinforced` event (`worlds/pocket-universe/src/legacy.rs`). Changing it changes what the Pack records, so it needs a Pack-version decision, or the narrator seam, rather than a copy edit.
4. **The history shows every internal step.** Two "Agent Decision Recorded" lines per visit tell the reader nothing. The projection should mark bookkeeping events so History can fold them, the way `BriefingItemKind::Status` already separates news from counters.
5. **"Your turn · …" is still also a card under "Where things stand".** The *why now* sentence belongs at the top of the decision panel. That needs the projection to say which briefing item explains the open choice.
6. **The passive choice is listed first.** "Watch the pressure build" leads, and it is the one the World would take anyway. The decision panel should lead with the real choices.
7. **The usability test** in `NEXT_TASK.md`. None of the above replaces five strangers and five minutes each.
