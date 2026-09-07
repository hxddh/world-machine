World Machine @TAG@ (pre-alpha)

Thank you for trying World Machine. This build is not yet notarized by
Apple, so macOS will refuse to open it the first time. This takes one
minute to fix and only happens once per download.

1. Drag "World Machine.app" into your Applications folder.

2. Double-click it. macOS says it "could not verify" the app. Click Done.

3. Open System Settings > Privacy & Security, scroll down to the
   Security section, and click "Open Anyway" next to World Machine.
   Confirm with your password or Touch ID.

4. Open World Machine again. It opens normally from now on.

If you prefer the Terminal, this one command does the same:

  xattr -dr com.apple.quarantine "/Applications/World Machine.app"

Everything stays on your Mac: no account, no telemetry. If something
goes wrong, choose Help > Report a Problem in the app.

Full guide: https://github.com/hxddh/world-machine/blob/@TAG@/docs/INSTALL.md
