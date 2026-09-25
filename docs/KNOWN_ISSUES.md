# Known issues

Current as of `v0.9.0` in the [changelog](../CHANGELOG.md). Report anything else with **Help → Report a Problem…** in the app.

## Installation

- **Not notarized.** The first launch of a pre-alpha build needs the one-time "Open Anyway" step in [INSTALL.md](INSTALL.md). The pipeline is ready to sign and notarize; [RELEASE_SIGNING.md](RELEASE_SIGNING.md) lists the five secrets that switch it on.
- **No automatic download of updates.** Home shows a banner when a newer release exists (one request to GitHub per launch, see [PRIVACY.md](PRIVACY.md)); installing it is still download, drag onto Applications.
- **macOS 14 or newer only.** Older systems are not built or tested.

## Using Worlds

- **A Pack version change closes older Worlds.** A `.world` file records the exact World Pack version it was created with, and the app ships one version of each included Pack. When a Pack's rules change its version moves, and Worlds pinned to the older version report that they need a Pack this build does not have. Each release's section of the [changelog](../CHANGELOG.md) says which Packs moved; `v0.9.0` and `v0.8.0` moved none, and in `v0.7.0` Pocket Universe moved to `0.20.0`. Export a World you want to keep before updating; nothing is deleted, and the file still holds its whole history.

- **Renaming or removing a World that is open in a window.** Both write the World's file, so the open window is a step behind: after a rename choose **World → Reload** in that window, and after a removal close the window rather than saving from it, or the save writes the World back into the Library.
- **Removed Worlds are not deleted.** **Remove** moves the file into a `Removed` folder inside `~/Library/Application Support/World Machine/Worlds`. Emptying that folder is a Finder step; the app never deletes World files.
- **Compare Futures needs a choice.** **What if…** opens the comparison only when the World currently offers at least two choices; otherwise it opens the World with a note saying so.
- **Background time is bounded.** A World advances at most seven periods per return, however long you were away; the return briefing says how many.
- **Sound is unheard.** Settings → Sound plays a quiet loop made from a World's landscape, and small ticks and bells as cards turn and turns pass, through macOS's own player. They are tested as sound data, but have not been listened to on a Mac.
- **What people say is the Pack's own words.** World voice narrates returns, but does not reword the lines people say in speech bubbles or their answers when asked.
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
