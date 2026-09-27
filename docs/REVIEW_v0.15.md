# v0.15.0 review: a deep simulation in a thin body

`v0.15.0` has a World that is genuinely alive underneath. People keep their own lives, remember what you said and did, and open up as a friendship grows. Questions come round in new words, and every return brings something to keep. This review asks a harder question than earlier ones: what would it take for every part of the product, not only its simulation, to stand beside the best products of its kind? The answer is that the leap now has to happen on the surface: in how it looks, moves, sounds and speaks, and in how it ships.

**Verdict:** the simulation is at or near the top of its class; almost everything a player sees, hears and touches is not. A player judges a cozy game in its first minute by its art, its motion and its sound. World Machine currently shows flat shapes, people about 40 points tall, 25 frames a second, no music, and every friend saying the same few secrets. Top products ship years of craft in exactly these places. The next releases should be a craft release, not a systems release.

## Scorecard

Each row is a dimension, the bar the best products set (sources in the research notes on the pull request), and where `v0.15.0` stands. **Gap** is an honest reading: close, far or missing.

| Dimension | What top products do | World Machine v0.15.0 | Gap |
| --- | --- | --- | --- |
| **Simulation and memory** | Stardew: schedules and heart events. Smallville: memory streams. RimWorld: a storyteller that paces events | People live every period, remember words and deeds, open doors at thresholds, a paced storyteller, deterministic replay | close; ahead in replay and cause |
| **Visual art** | A Short Hike, Dorfromantik, Spiritfarer: locked palette, diorama lighting, 3+ depth layers, characters readable by silhouette, expressions | A Pack palette, sky and weather from state, flat shapes, people about 40 pt with no expressions, one depth layer, no artist | far; needs an illustrator |
| **Animation and feel** | 60 fps (120 on ProMotion), eased motion, squash and stretch, 4+ idle behaviours per character, a reaction to every click within a frame | 25 fps (a 40 ms frame), an eased walk, a few poses, no idle behaviours, no click reaction | far |
| **Audio** | Animal Crossing: 24 hourly themes per weather, per-character babble. Unpacking: 14,000 foley files. Separate volume sliders | One ambient loop per landscape, a few ticks and bells, never heard on a Mac, no music, no voices | missing; needs a composer |
| **Characters and writing** | 150+ lines per core resident, a voice style sheet each, 5+ friendship scenes each. LLM voices bounded by what the character knows | About 1,800 authored sentences across both Packs and the Systems, mostly shared. Doors are the same four secrets and six keepsakes for everyone | far; needs a writer |
| **Building and expression** | Townscaper and Tiny Glade: place anywhere, never fail. Placement with systemic effects (Neko Atsume). Undo, photo mode | 8 things to make in Tiny Society, 24 in Pocket Universe, placed at a named place only. Someone comments, but a bench changes nothing | far |
| **Progression** | A collection book with silhouettes of what is missing, 60+ entries; an unlock per session for a month; goals a player can finish | Chapters, goals the careful player never finishes (pier, lamp), a keepsake drawer without silhouettes | far |
| **First minute** | A resident speaks in under 20 s, the first choice under 60 s, the first keepsake under 5 min; at most two lines of text at a time | Opens on the place; the first deed brings a reaction and then a question; nobody greets the player; nothing measured in seconds | close-ish |
| **Return loop** | Neko Atsume: visitors and gifts while away. Finch: gentle streaks. One opt-in notification a day, no punishment for absence | A retold return and a keepsake, no punishment; no notification (needs signing) | close in design; blocked on platform |
| **UX and accessibility** | 100–200% text scaling, Reduce Motion, keyboard, VoiceOver, 4.5:1 contrast, a second language | None of these; English interface only (Chinese understood when typed) | missing |
| **Platform** | Notarized, Sparkle updates, under 2% idle CPU, under 150 MB, cold start under 1.5 s, tested on a MacBook Air and a ProMotion Pro | Ad-hoc signed, banner-only updates, never run on a real Mac, no energy or memory numbers | missing; needs an Apple account and Macs |

## What code alone cannot do

A leap on every row needs people and accounts the repository cannot supply. Saying so now keeps the plan honest:

- **An illustrator or art director** for a style guide and character sheets. Code can draw bigger, readable, expressive people and a diorama grade, but not a signature look.
- **A composer**, or licensed stems, and a few field recordings. Code can build the adaptive music engine, per-character babble and the mixer, and fill them with generated placeholders.
- **A writer** for 150 lines and five scenes per core resident. Code can hold the style sheets and the tests; the voice itself is writing.
- **An Apple Developer account** (the five signing secrets) for notarization, notifications and Sparkle updates.
- **Real Macs and five playtesters** to time the first minute and to measure energy, memory and start-up.

## v0.16 and v0.17: a craft leap

Two releases, each with tested bars. v0.16 makes the World feel alive to the eye and ear; v0.17 gives its people their own voices and gives the player a place to make their own.

### v0.16: it looks, moves and sounds alive

1. **Motion with feel.**
   - The scene is drawn at the display's rate (60 fps, 120 on ProMotion) while in front, and drops to nothing when hidden. A frame's drawing work fits in 4 ms, checked by a benchmark.
   - No linear motion anywhere: every move eases, with anticipation and settle.
   - Every resident has at least four idle behaviours (look around, sit, stretch, a task at their work) and a walk with bob and sway.
   - A click on anyone or anything gets a visible reaction on the next frame: a wave, a hop, a placement bounce.
2. **People you can read.**
   - Residents drawn at least 64 pt tall, with faces that show how they feel (content, happy, sad, cross, thinking) from their state, and a silhouette of their own (hair, hat, coat, build).
   - A depth pass: at least three layers with gentle parallax as the camera moves, and a diorama grade (warm key light, cool shade) per time of day.
   - Tested: no two residents share a silhouette, and each state shows a different face.
3. **Sound that tells the time.**
   - An adaptive music engine: three layers per world (a base, a day layer, an evening layer) crossfaded by the hour and thinned by weather. Generated placeholders until a composer's stems replace them.
   - A babble voice per resident (Animalese-style, pitch and timbre their own) under every speech bubble.
   - Every UI sound in three or more variations, never the same twice running. Separate music, ambience, voice and interface volumes.
4. **The first minute.**
   - A resident walks up and greets the player within 20 seconds, the first choice comes within 60, and a first keepsake within 5 minutes of play. A scripted first session checks all three.
   - No card or line longer than two lines of text.

### v0.17: their own voices, your own place

5. **A voice for everyone.**
   - A style sheet per core resident (words they use, tics, what they avoid), and at least 150 lines each in Tiny Society's core cast, written to it.
   - Five friendship scenes per core resident that are theirs alone, replacing the shared secrets and keepsakes; Pocket Universe draws its people's from their traits.
   - World voice bounded by what each person knows, checked against a 200-prompt set that must produce no out-of-world answer. It stays a proposal, logged and never regenerated on replay.
6. **Build anywhere, and it matters.**
   - Place and move things anywhere on the ground, at least 20 in each Pack, with undo.
   - What is built changes lives: people sit on a bench, gather under a lamp in the evening, and bring gifts from a garden.
   - A photograph of the scene can be saved.
7. **A book of everything.**
   - A collection book of keepsakes, festival days, people met and things built, with silhouettes for what is still to find: at least 60 entries per Pack.
   - Goals a careful player can finish in a year.
8. **For everyone, and in Chinese.**
   - Text from 100% to 200% without clipping, Reduce Motion and Increase Contrast respected, every screen usable by keyboard, VoiceOver labels on every control.
   - Every string moved out of the code, and a full Simplified Chinese interface and Tiny Society.

### Alongside, outside the code

- **Ship-grade Mac:** with the five secrets set, notarize, send notifications (one a day at most, opt-in), and add Sparkle updates. Measure idle CPU, memory and cold start on a MacBook Air and a ProMotion Pro.
- **Art, music and writing:** an illustrator's style guide and character sheets for each Pack, a composer's stems for three worlds, and a writer for the core casts. The code in v0.16 and v0.17 is built to take them in without changes.
- **Playtests:** five unassisted first sessions, recorded, against the first-minute bar.

## How we'll know

- v0.16: a frame benchmark (4 ms), a lint over the scene code for linear motion, a test that every resident has four idle behaviours, distinct silhouettes and a face per state, sound data that differs by hour and weather and a babble voice per resident, and a scripted first session timed against 20 s, 60 s and 5 min.
- v0.17: at least 150 distinct lines per core resident, five scenes each that fire once, a 200-prompt World voice set with no out-of-world answer, 20 placeable things with undo and at least three systemic effects, a book of at least 60 entries, a pseudo-locale 30% longer that renders without clipping at 200% text, and every Chinese string present.
- Every earlier bar still holds, and replay stays deterministic.

Out of scope by decision: Worlds saved by earlier releases are not carried forward.
