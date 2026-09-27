# Known issues

Current as of `v0.14.0` in the [changelog](../CHANGELOG.md). Report anything else with **Help → Report a Problem…** in the app.

## Installation

- **Not notarized.** The first launch of a pre-alpha build needs the one-time "Open Anyway" step in [INSTALL.md](INSTALL.md). The pipeline is ready to sign and notarize; [RELEASE_SIGNING.md](RELEASE_SIGNING.md) lists the five secrets that switch it on.
- **No automatic download of updates.** Home shows a banner when a newer release exists (one request to GitHub per launch, see [PRIVACY.md](PRIVACY.md)); installing it is still download, drag onto Applications.
- **macOS 14 or newer only.** Older systems are not built or tested.

## Using Worlds

- **A Pack version change closes older Worlds.** A `.world` file records the exact World Pack version it was created with, and the app ships one version of each included Pack. When a Pack's rules change its version moves, and Worlds pinned to the older version report that they need a Pack this build does not have. Each release's section of the [changelog](../CHANGELOG.md) says which Packs moved; in `v0.14.0` Tiny Society moved to `0.8.0` and Pocket Universe to `0.25.0`, in `v0.13.0` Tiny Society moved to `0.7.0` and Pocket Universe to `0.24.0`, in `v0.12.0` Tiny Society moved to `0.6.0` and Pocket Universe to `0.23.0`, in `v0.11.0` Tiny Society moved to `0.5.0` and Pocket Universe to `0.22.0`, in `v0.10.0` Tiny Society moved to `0.4.0` and Pocket Universe to `0.21.0`, `v0.9.0` and `v0.8.0` moved none, and in `v0.7.0` Pocket Universe moved to `0.20.0`. Export a World you want to keep before updating; nothing is deleted, and the file still holds its whole history.

- **Renaming or removing a World that is open in a window.** Both write the World's file, so the open window is a step behind: after a rename choose **World → Reload** in that window, and after a removal close the window rather than saving from it, or the save writes the World back into the Library.
- **Removed Worlds are not deleted.** **Remove** moves the file into a `Removed` folder inside `~/Library/Application Support/World Machine/Worlds`. Emptying that folder is a Finder step; the app never deletes World files.
- **Compare Futures needs a choice.** **What if…** opens the comparison only when the World currently offers at least two choices; otherwise it opens the World with a note saying so.
- **Background time is bounded.** A World advances at most seven periods per return, however long you were away; the return briefing says how many.
- **Sound is unheard.** Settings → Sound plays a quiet loop made from a World's landscape, and small ticks and bells as cards turn and turns pass, through macOS's own player. They are tested as sound data, but have not been listened to on a Mac.
- **What people say is the Pack's own words, unless you talk to them with World voice on.** World voice narrates returns and, when it is on, answers for the people you talk to. The lines people say over their heads as they go about their day, and the three quick questions in their card, are always the Pack's own.
- **Without World voice, talking understands everyday phrasing, not everything.** People hear 28 kinds of thing in English and Chinese, and a 428-phrase corpus is heard without a miss. On new phrasing, measured on phrases written before any tuning, a quarter to nearly half were not understood and some were misheard; idioms ("bury the hatchet", "what makes you tick") are the usual misses. Anything not understood gets a hint. With World voice on, a model hears anything and picks from the same meanings.
- **World voice answers take a few seconds.** The window keeps moving and shows the person thinking; after 12 seconds the World answers in its own words. That deadline has only been tried in tests, not against a real model on a Mac.
- **People remember a month of what you said, and bring up one thing a day.** What they recall is the latest thing worth mentioning, the first time they see you that day.
- **Festivals happen while you are away.** Like a storm, a festival's day does not wait for you; the return tells how it went.
- **A long absence brings its own story.** While you are away wants wait for you and people live their own lives, but what cannot wait (a storm, market day, a chapter's climax) happens without you and can close a chapter. Its pacing has only been tried in tests, not over two weeks on a real Mac.
- **Pocket Universe gives one reason for every answer it cannot take:** "Not possible right now". Tiny Society names what is short ("Noah hasn't 60 to spare").
- **History tells a World's recent past.** History is told from the latest 1,200 events, about two months of a busy World, and shows its latest 40 moments. What people did each day is told as it happens, in what they say; History keeps today's and what changed between people. A detail panel lists someone's twelve latest changes, and the chain of causes and detail panel of an event are there for the latest 400 events. Older events are still in the file, and the chapter book tells them.
- **A year-old World's snapshot takes about 9 ms** in a release build, inside the 10 ms aimed for but only just, and a turn with its previews is inside its 50 ms. Neither has been timed on a real Mac.
- **The drawings are a first set, not an illustrator's.** Each Pack's buildings, people and festival fixtures are its own drawings in flat shapes, and Home's covers use them; things the player builds (benches, lamp posts, gardens) are still drawn with the app's own shapes. A commissioned artist's set should replace them before `1.0`.
- **Zooming uses the scroll wheel only.** There is no pinch or on-screen control yet; Escape pulls back.
- **Your hands allow two deeds a period,** and in Pocket Universe they cost nothing, since its places keep no money.
- **The calendar repeats by design.** Market day, supply drops and birthdays are left out of the tests that keep other questions to two in 30 periods and count how many follow from an answer.
- **Only the latest chapter's ending shows as a card,** and only if it closed within the last period; earlier ones are in the drawer's chapter book.
- **A living World redraws about 25 times a second while its window is in front.** Behind other windows it stops. Its effect on battery life has not been measured on a real Mac.
- **The sky follows your clock, not the World's.** Dawn, dusk and night are drawn from the Mac's local time, like a window onto the same sky; a World records nothing about them.
- **No notification while a World is closed.** A World keeps going without you and its window says when it next moves, but nothing tells you from outside the app. macOS delivers notifications reliably only to a signed app, so this waits on notarization.
- **Home offers two Packs to start.** *Future Archaeologist* and *Micro Company* are engine test Packs and are hidden from Home; Worlds already made with them still open, and launching with `WORLD_MACHINE_DEVELOPER=1` offers them again.
- **World Analyst is experimental** and needs Node and the Pi runtime on your PATH. The entry stays hidden otherwise. See [PI_ANALYST.md](PI_ANALYST.md).

## Verification gaps

- The pre-alpha has been exercised through automated tests and CI-built screenshots, not yet on a wide range of real Macs. Layout on small screens and with large accessibility text sizes is unverified.
- **Nothing that needs a keypress or a click is verified anywhere.** The CI screenshot runner can launch the app and open a World, but GitHub's macOS runners grant no Accessibility permission, so no keystroke it sends ever arrives — proven by pressing ⌘M, which the app binds, and watching the window not minimize. Full screen, every keyboard shortcut, and every button click are therefore unchecked until somebody runs the app on a real Mac.
- **Linux preview is not a Mac.** Since `v0.6.0`, `scripts/linux-preview.sh` runs the real app under Xvfb and every screenshot in [PRODUCT_REVIEW.md](PRODUCT_REVIEW.md) comes from it, clicks and keys included. Fonts, weights and window chrome differ from macOS, and the Pack install review has not been seen at all, because the preview cannot drive a file picker.
- The CI screenshot runner produced images with layout but no text through `v0.5.0`. That was not a limitation of the runner: the app itself rendered no text anywhere, on CI and on real Macs alike, and `v0.5.1` fixes it. Screenshots in the repository are still taken by hand on a real Mac.
