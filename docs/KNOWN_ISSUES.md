# Known issues

Current as of the `Unreleased` section of the [changelog](../CHANGELOG.md). Report anything else with **Help → Report a Problem…** in the app.

## Installation

- **Not notarized.** A browser download needs the one-time "Open Anyway" step in [INSTALL.md](INSTALL.md); the one-line installer avoids it. This stays true until a Developer ID certificate exists.
- **No automatic updates.** Help → Check for Updates… opens the Releases page; rerunning the installer updates in place.
- **macOS 14 or newer only.** Older systems are not built or tested.

## Using Worlds

- **Compare Futures needs a choice.** **What if…** opens the comparison only when the World currently offers at least two choices; otherwise it opens the World with a note saying so.
- **Background time is bounded.** A World advances at most seven periods per return, however long you were away; the return briefing says how many.
- **World Analyst is experimental** and needs Node and the Pi runtime on your PATH. The entry stays hidden otherwise. See [PI_ANALYST.md](PI_ANALYST.md).

## Verification gaps

- The pre-alpha has been exercised through automated tests and CI-built screenshots, not yet on a wide range of real Macs. Layout on small screens and with large accessibility text sizes is unverified.
- The CI screenshot runner renders layout but no text (a GPUI glyph-atlas limitation inside Apple Virtualization), so the screenshots in the repository are taken by hand on a real Mac.
