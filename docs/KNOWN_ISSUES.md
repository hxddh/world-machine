# Known issues

Current as of `v0.3.0` in the [changelog](../CHANGELOG.md). Report anything else with **Help → Report a Problem…** in the app.

## Installation

- **Not notarized.** The first launch of a pre-alpha build needs the one-time "Open Anyway" step in [INSTALL.md](INSTALL.md). The pipeline is ready to sign and notarize; [RELEASE_SIGNING.md](RELEASE_SIGNING.md) lists the five secrets that switch it on.
- **No automatic download of updates.** Home shows a banner when a newer release exists (one request to GitHub per launch, see [PRIVACY.md](PRIVACY.md)); installing it is still download, drag onto Applications.
- **macOS 14 or newer only.** Older systems are not built or tested.

## Using Worlds

- **A Pack version change closes older Worlds.** A `.world` file records the exact World Pack version it was created with, and the app ships one version of each included Pack. When a Pack's rules change its version moves, and Worlds pinned to the older version report that they need a Pack this build does not have. The `Unreleased` section of the [changelog](../CHANGELOG.md) says which Packs moved in this release. Export a World you want to keep before updating; nothing is deleted, and the file still holds its whole history.

- **Renaming or removing a World that is open in a window.** Both write the World's file, so the open window is a step behind: after a rename choose **World → Reload** in that window, and after a removal close the window rather than saving from it, or the save writes the World back into the Library.
- **Removed Worlds are not deleted.** **Remove** moves the file into a `Removed` folder inside `~/Library/Application Support/World Machine/Worlds`. Emptying that folder is a Finder step; the app never deletes World files.
- **Compare Futures needs a choice.** **What if…** opens the comparison only when the World currently offers at least two choices; otherwise it opens the World with a note saying so.
- **Background time is bounded.** A World advances at most seven periods per return, however long you were away; the return briefing says how many.
- **World Analyst is experimental** and needs Node and the Pi runtime on your PATH. The entry stays hidden otherwise. See [PI_ANALYST.md](PI_ANALYST.md).

## Verification gaps

- The pre-alpha has been exercised through automated tests and CI-built screenshots, not yet on a wide range of real Macs. Layout on small screens and with large accessibility text sizes is unverified.
- The CI screenshot runner renders layout but no text (a GPUI glyph-atlas limitation inside Apple Virtualization), so the screenshots in the repository are taken by hand on a real Mac.
