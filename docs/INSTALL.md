# Install World Machine on macOS

Requirements: a Mac with Apple Silicon or Intel running macOS 14 or newer. Nothing else is needed.

World Machine is pre-alpha and **not yet notarized by Apple**, so a copy downloaded with a browser is blocked on its first launch. The one-line installer avoids that entirely; the manual path needs one extra step.

## One line in Terminal (recommended)

Open Terminal (Spotlight: type "Terminal"), paste this line, press Return:

```bash
curl -fsSL https://raw.githubusercontent.com/hxddh/world-machine/main/scripts/install.sh | sh
```

It downloads the latest release, checks the checksum, puts World Machine in Applications, clears the first-launch block, and opens the app. About a minute. Run the same line again later to update; your Worlds are kept.

The script is [scripts/install.sh](../scripts/install.sh) if you want to read it first. It talks only to github.com and installs only the app.

## Or by hand

1. Download the `.zip` from the [Releases page](https://github.com/hxddh/world-machine/releases) and open it. You get a folder with `World Machine.app` and `Read Me First.txt`.
2. Drag `World Machine.app` into Applications.
3. Double-click it once. macOS says it could not verify the app; click **Done**. Open **System Settings → Privacy & Security**, scroll to the **Security** section, click **Open Anyway**, and confirm. From then on it opens normally.

   On macOS 14 you can instead Control-click the app and choose **Open**. On macOS 15 that shortcut no longer works.

   If a World fails to start after this, the Packs inside the app still carry the download flag. This one Terminal line clears it for the whole app:

   ```bash
   xattr -dr com.apple.quarantine "/Applications/World Machine.app"
   ```

## First run

The app opens straight into your first World: Pocket Universe, prepared during the few seconds after launch. Pick a place to seed and let it live. Home lists your Worlds; **What if…** on any of them tries the other choice side by side. Everything is saved under `~/Library/Application Support/World Machine`.

## Updating

Run the one-line installer again, or download the new release and replace the app in Applications (then repeat step 3 once). Your saved Worlds are not inside the app and are kept.

## Verifying a download (optional)

Every release publishes a `.zip.sha256` and a `release-manifest.json`. In Terminal, from the download folder:

```bash
shasum -a 256 -c World-Machine-*.zip.sha256
```

The output must end with `OK`. The manifest lists the exact commit, architectures, and `"notarized": false`, which is the current status, not an error. The one-line installer performs this check for you.

## Known limits of the pre-alpha build

- Not notarized. A browser download needs the one-time step 3 above.
- No automatic updates. Use **Help → Check for Updates…** or rerun the installer.
- The experimental World Analyst additionally needs Node and the Pi runtime; its menu entry says so when they are missing.

## Something went wrong?

In the app, choose **Help → Copy Diagnostics**, then **Help → Report a Problem…** and paste the diagnostics into the issue. The diagnostics text contains the build label, your macOS version, the paths the app uses, and the last lines of its log; nothing else. The log itself is at `~/Library/Logs/World Machine/world-machine.log` (**Help → Show Log in Finder**), and **World Machine → About World Machine…** shows the same build label.

If the app does not start at all, open an issue with the [bug report template](https://github.com/hxddh/world-machine/issues/new/choose) and include your macOS version and the release tag you downloaded.
