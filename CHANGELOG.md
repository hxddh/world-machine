# Changelog

Releases live on the [Releases page](https://github.com/hxddh/world-machine/releases). `0.2.0` is the first release intended to be usable without reading the repository; the `v0.1.0-pre.N` tags below were its pre-releases.

## Unreleased

**Worlds created before this release will not open.** Pocket Universe moves to `0.18` because the era engine changes the rules its Worlds run under, and a `.world` file records the Pack version it was created with. Export anything you want to keep before updating; Tiny Society and Micro Company Worlds are unaffected.

- Pocket Universe 0.18: **a World's story no longer ends.** The four chapters were a ladder — measured, a World completed all of them by its thirteenth period and then offered one button that changed nothing for as long as you kept it. They are now the stages of one era, and eras loop: when a succession settles, the successor becomes the keeper of a new era and the World faces a different trouble. No era faces the trouble the one before it just survived, and how an era ended decides what the next one inherits — a legacy handed on unchanged carries over, one let go has to form again.
- Each seed now has a second trouble to face: Ares can lose its air to a dust season as well as its water to the reclaimer, Maple Street can lose the night bus as well as the lease, and Icebridge can empty its fish vault as well as crack a span.
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
