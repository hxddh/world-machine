# Changelog

Releases live on the [Releases page](https://github.com/hxddh/world-machine/releases). `0.2.0` is the first release intended to be usable without reading the repository; the `v0.1.0-pre.N` tags below were its pre-releases.

## Unreleased

**Worlds created before this release will not open.** Tiny Society moves to `0.3.0` because it now records things it used to leave unrecorded, and a `.world` file records the Pack version it was created with. Export anything you want to keep before updating; Pocket Universe and Micro Company Worlds are unaffected.

- **Coming back to a World stops reading like a dashboard.** A return briefing mixed two different things in one list: what happened to somebody, and how much of the routine ran. Measured over fourteen returns to a Tiny Society World, eight of them led with "Harbor Bakery had customers · 40 purchases · 400 revenue" — a counter, presented exactly like news. The two are now marked apart where the World produces them, so a counter can never again be handed to you as the answer to "what happened while I was away?".
- **Tiny Society 0.3.0: running out of money is something you are told about.** Jonas leaves the opening story unemployed with savings to burn, and burning them took twenty periods during which the World recorded nothing at all — his cash fell from 85 towards nothing and every briefing in between said only that the bakery had customers. The World now records the crossings: when he starts spending savings he cannot replace, when he can no longer cover a day, and when he is covering his own days again. Each is recorded once per spell, so none of them becomes another daily counter.
- **A day nobody could pay for no longer passes in silence.** When Jonas could not cover his living costs the simulation simply skipped him — no event, no trace, and a World that appeared to freeze with him at 5 cash forever. That day is now recorded like anything else that happens.
- **The same thing happening every day is told once.** Once Sea Finch is back in the water Jonas sells a catch daily, and a briefing would print three identical lines about it. The most recent stands for the rest; how many there were is what the counters are for.

## v0.5.2 (2026-09-10)

- **A new app icon.** The first one was a node joined to two smaller nodes, which is the share glyph every platform already uses and said nothing about what this app does. It is now a single stroke that spirals outward from the centre, thin and pale where a World's history begins and broad and warm at the present — the one shape that means "this kept going while you were away".
- **Full screen.** The Window menu had Minimize and Zoom but no **Enter Full Screen**, and ⌃⌘F — which works in every other Mac app — did nothing. AppKit only adds that item for apps whose menus come from a nib, and this app builds its menus in code, so it had to be added by hand.

## v0.5.1 (2026-09-10)

**Every release before this one shipped with no text on screen.** World Machine drew its windows, its cards, its buttons and its borders, and every word inside them was invisible — the app was unusable, and had been since the first release. `gpui_platform`, the crate the app opens its window through, ships with an empty default feature set; without `font-kit` it quietly installs a text system that measures text but draws nothing, logs no error, and never crashes. Enabling that feature is the whole fix. A check now fails the build if any desktop app in this repository asks for a window without also asking to be able to render text.

- **The app has an icon.** The bundle never carried one, so macOS drew the blank generic-application page in the Dock, the Finder and the Cmd-Tab switcher. It is now built from a single source image at every size macOS asks for.
- **Home no longer opens with an error on it.** `v0.5.0` started shipping a World as a bundled Pack that the app also carries compiled into itself. The Host refuses to register one Pack twice, so the install failed, the Pack never became reachable, and Home reported it on every launch. An installed Pack now wins over the copy inside the app.
- **The Create button on Home stays on screen.** The first-run card laid its text out at full width and pushed its own button past the window edge at the size Home opens at.
- The About window no longer says everything stays on your Mac without qualification — the same correction the privacy page took in `v0.5.0`, in the one place a user actually reads it.

## v0.5.0 (2026-09-10)

**Every World you already have still opens.** No World Pack changed the rules its Worlds run under, so nothing saved by `v0.4.0` — or by anything older that still opens — is closed by this release. That is the first time a content release has been able to say so, and it is what the additive-content discipline is for.

- **Your Worlds can write in their own words.** Until now a World told you what happened using sentences written into the app, and there were only so many of them: over twenty week-long returns, the lines describing trouble repeated 82% of the time because each one had exactly three phrasings. Give a World a voice and what it tells you is written about your World instead — the same measurement drops those lines to no repetition at all, and two Worlds grown from the same seed stop sharing their prose.
- **World Machine has a Settings window.** ⌘, opens it, and it is where a World's voice lives: on or off, whether it reaches a model through a program already on your Mac or through an API key, and the key itself. It says in plain words what each choice means. ⌘, used to open About, which is not what that key does anywhere else on macOS; About is still in the menu.
- **A key belongs to you, not to the app.** It is kept in your login keychain rather than in any file World Machine writes, so a backup of your Worlds folder never carries one, and you can see, inspect, or delete it yourself in Keychain Access under "World Machine · World voice".
- **Nothing is worse for having a voice.** A model that is unreachable, slow, refuses, or answers with something unusable leaves a World reading exactly as it does with no voice at all, line by line. A voice can only put an already-recorded fact into words: it cannot decide what happens in a World, so a bad answer costs you a clumsy sentence and never a broken World. Replaying a World never asks a model anything.
- **Tiny Society ships in the app.** It has been advertised in the README since before `v0.2.0` and its second consequence chain shipped in `v0.3.0`, but the packaged app only ever carried Pocket Universe and Micro Company, so nobody who downloaded World Machine could open it. It is now one of the included Worlds on Home, alongside the other two.
- **What leaves your Mac, and what does not.** An API key is the only setting in the app that sends anything anywhere: one request each time you return to a World, carrying what that World has already recorded and nothing else — not your other Worlds, not your library, not your file names, not the log. One request per return, never one per period. A program you choose is between you and that program. Both are off until you switch them on, and the [privacy page](docs/PRIVACY.md) now says all of this rather than claiming everything stays on your Mac without qualification.
- The Packs the app ships now have to agree about who they are. The Pack's own id and version, what the app expects to find, and what the build actually writes into the app are three lists in three places that cannot see each other; a drift between them makes a bundled World refuse to install, and only on a real Mac. A check now fails the build instead.

## v0.4.0 (2026-09-10)

**Worlds created before this release will not open.** Pocket Universe moves to `0.18` because the era engine changes the rules its Worlds run under, and a `.world` file records the Pack version it was created with. Export anything you want to keep before updating; Tiny Society and Micro Company Worlds are unaffected.

- Pocket Universe 0.18: **a World's story no longer ends.** The four chapters were a ladder — measured, a World completed all of them by its thirteenth period and then offered one button that changed nothing for as long as you kept it. They are now the stages of one era, and eras loop: when a succession settles, the successor becomes the keeper of a new era and the World faces a different trouble. No era faces the trouble the one before it just survived, and how an era ended decides what the next one inherits — a legacy handed on unchanged carries over, one let go has to form again.
- Each seed now has a second trouble to face: Ares can lose its air to a dust season as well as its water to the reclaimer, Maple Street can lose the night bus as well as the lease, and Icebridge can empty its fish vault as well as crack a span.
- A third trouble per seed, which is what lets a World's history start to matter: with only two, the never-repeat rule already decided everything. Ares can now also lose contact with the relay, Maple Street can lose its crowd to a highway mall, and Icebridge can lose its ice to an early thaw.
- **How you have been answering changes what comes next.** Two Worlds at the same era facing the same trouble diverge if one has been handing its legacy on and the other rewriting it. A World that has answered the same way several eras running is told so: "3 eras running have handed their habits on unchanged; nobody keeping them now chose them."
- **A World left alone keeps living.** Measured before this: a World seeded and then never answered walked to its eighteenth period and stopped there forever, holding buttons nobody was going to press; one never given its opening choices never got started at all. Every choice now has a deadline as well as a default, so leaving is itself an answer — the World reaches the default on its own, one choice per period. Coming back, the briefing leads with **Decided without you** and says what it settled and why. What it decided is exactly as durable as what you decide; a trouble left unanswered still costs the World its anchor rather than being quietly held for you, and a lost anchor is never rebuilt without you.
- **A week away is now a week of World time.** Background catch-up was capped at seven periods — a day and three quarters — from when a World held a fixed amount of story and running through it unattended was the risk. Eras removed that ceiling, so the cap now says the simpler thing: a World lives through at most a week of its own time while you are away. A day away moves it four periods; a week away moves it a week; a month away still moves it a week, because a return should be readable rather than exhaustive.
- **A long absence reads as the eras it crossed**, not as a list of periods. Coming back to a World that turned over while you were gone now opens with *"2 eras turned — you left during era 1; this is era 3"* and what the current era began on, above everything that happened inside it.
- An era opens on a quiet stretch before its trouble arrives, and says so instead of falling back to first-visit language. A World past its first era says which era it is and what it inherited.

## v0.3.0 (2026-09-08)

**Worlds created before this release will not open.** Pocket Universe moves to `0.17` and Tiny Society to `0.2` because both change the rules their Worlds run under, and a `.world` file records the exact Pack version it was created with. This release ships only the new versions, so a Pocket Universe or Tiny Society World saved by `v0.2.2` or earlier reports that it needs a Pack version this build does not have. Export anything you want to keep before updating; Micro Company Worlds are unaffected.

- Pocket Universe 0.17: chapter four. Once the pressure has resolved, two generations later someone who did not live through the World's beginning steps forward. This chapter has no deadline — the successor waits as long as you leave them waiting, but every generation of waiting deepens habits of their own, so the briefing moves from "New hands" through "Own habits" to "Already theirs". Entrust the legacy unchanged or release it to be rewritten; the answer is durable, and releasing resets the legacy's reinforcement cycles the way recovering a lost anchor does. The same choice reads differently depending on how chapter three ended.
- Tiny Society: a second consequence chain. Reopening the bakery lean leaves Mara working the counter alone. Only the demand that came back — Jonas buying bread again, which is the end of the first chain — is counted, and once the counter has carried that trade and the till holds a wage in reserve, Mara takes Mia on and the job the closure cost the island comes back. A World whose harbour never recovered keeps a one-person bakery for good.
- My Worlds: once you have six or more Worlds, the list gains **Find a World** and an **Order** switch — most recently played, or A to Z. Typing matches a World's name, its World Pack's title, or its file id, and the heading says how many of your Worlds are showing.
- Windows open where you left them. Home and World windows remember their size and position between launches; a window saved on a display you no longer have opens centred instead of off-screen.

## v0.2.2 (2026-09-08)

- My Worlds: **Rename** on any World card gives it the name you type, and clearing the name lists it under its World Pack's title again. Branches no longer all read "Pocket Universe" with only a file id to tell them apart.
- My Worlds: **Remove** takes a World off Home after a second click on the card. Its file moves to a `Removed` folder inside your Worlds folder, so a removal by mistake is a drag back in the Finder.
- A World window is titled by the World's name, and its header shows the name with the file id beside it. A World named on Home is called by that name in its own window, in its save and reload lines, and in the file name Save As suggests.
- One damaged or foreign file in the Worlds folder no longer hides every other World. The Worlds that can be read are listed as usual, and Home names the files it could not read.

## v0.2.1 (2026-09-08)

- Home shows a banner when a newer stable release exists (one request to the GitHub Releases API per launch; `WORLD_MACHINE_NO_UPDATE_CHECK=1` turns it off). Download opens the release page.
- Help → Report a Problem… opens the issue template with the build label and macOS version already filled in.
- Dark mode: every window follows the macOS appearance. The light palette is adapted automatically (backgrounds dark, cards slightly raised, text and accents light) and windows re-render when the system switches.
- Standard Mac behaviour: Cmd-W closes a window, Cmd-M minimizes, Cmd-H hides, a Window menu, Hide Others / Show All in the app menu, and clicking the Dock icon after the last window is closed brings Home back.

## v0.2.0 (2026-09-07)

The "usable" release: install by drag and drop, a World that opens by itself, two buttons to live in it, and a way to report problems. Not yet notarized: the first launch asks once until the Developer ID pipeline in docs/RELEASE_SIGNING.md is switched on.

- Diagnostics: a local log under `~/Library/Logs/World Machine`, an About window with the build label and signing status, and Help menu items to copy diagnostics, show the log, open the install guide, and report a problem. Cmd-Q quits.
- Home: **What if…** on every World card opens the World and runs the comparison immediately (its first two choices, 20 periods); "Change choices…" in the result window opens the full setup.
- Fewer controls: Home's header buttons move to a File menu (Import World, Install World Pack, Refresh); installed-Pack management is folded behind one line; the World window keeps only Branch and What if… as buttons, with Save As, Reload, lineage, comparisons, and the Analyst in a World menu.
- Layout: Home header, World cards, document chrome, and the projection center pane no longer lay out wider than their window.
- Copy: the first-launch path speaks of Worlds being prepared and ready instead of Packs, probes, and registries.
- Tiny Society: every briefing opens with a "Harbor today" state line, so a fresh visit and a return share the same now / changed / can-do shape.
- Install: releases now ship a DMG with an Applications shortcut; no Terminal in the install path. The pipeline signs with a Developer ID, notarizes, and staples as soon as the five secrets in docs/RELEASE_SIGNING.md exist. Help → Check for Updates… opens the Releases page. A one-line Terminal installer remains as an optional extra.
- First launch on a fresh install opens straight into the first Pocket Universe World instead of stopping at a Create card.
- Release: the zip unpacks to a folder with the app and a `Read Me First.txt` explaining the one-time "Open Anyway" step.
- Docs: user-first README, this changelog, known issues, privacy note.

## v0.1.0-pre.5 (2026-09-07)

- Pocket Universe 0.16: chapter three. After a legacy has reinforced itself, a seed-specific pressure rises against the World's anchor, warns, peaks, and can durably cost the anchor; the observer holds or reaches, and a lost anchor can be recovered.

## v0.1.0-pre.4 (2026-09-07)

- Universal build: one zip runs on Apple Silicon and Intel Macs, including the bundled Packs and the analyst host.

## v0.1.0-pre.3 (2026-09-07)

- Included Packs activate on first launch without a review dialog; the review flow remains for user-supplied `.worldpack` files.

## v0.1.0-pre.2 (2026-09-07)

- The experimental World Analyst entry is hidden unless Node and Pi are installed, so a fresh install needs nothing beyond the app.

## v0.1.0-pre.1 (2026-09-07)

- First automated pre-release: a tag or a workflow dispatch builds the app, validates the package, and publishes the zip, SHA-256, manifest, and release notes. Install guide for macOS 14 and 15.
