World Machine @TAG@ (pre-alpha)

Thank you for trying World Machine. This build is not yet notarized by
Apple, so macOS refuses to open it the first time. One minute fixes it,
once per download.

Easiest: open Terminal, paste this line, press Return. It installs the
app into Applications without the block and opens it:

  curl -fsSL https://raw.githubusercontent.com/hxddh/world-machine/main/scripts/install.sh | sh

By hand:

1. Drag "World Machine.app" into your Applications folder.

2. Double-click it. macOS says it "could not verify" the app. Click Done.

3. Open System Settings > Privacy & Security, scroll down to the
   Security section, and click "Open Anyway" next to World Machine.
   Confirm with your password or Touch ID.

4. Open World Machine again. It opens normally from now on.

Everything stays on your Mac: no account, no telemetry. If something
goes wrong, choose Help > Report a Problem in the app.

Full guide: https://github.com/hxddh/world-machine/blob/@TAG@/docs/INSTALL.md
