# Install World Machine on macOS

Requirements: a Mac with Apple Silicon or Intel running macOS 14 or newer. Nothing else is needed, and nothing is typed.

1. Download the `.dmg` from the [Releases page](https://github.com/hxddh/world-machine/releases) and open it.
2. Drag **World Machine** onto the **Applications** shortcut in the window that appears.
3. Open World Machine from Applications. Your first World opens by itself.

## Until the app is notarized

World Machine is pre-alpha and not yet notarized by Apple, so the first launch of a pre-alpha build adds one step, once:

- macOS says it could not verify the app. Click **Done**.
- Open **System Settings → Privacy & Security**, scroll to the **Security** section, click **Open Anyway**, and confirm.
- Open World Machine again. From now on it opens normally.

On macOS 14 you can instead Control-click the app and choose **Open**. The `Read Me First.txt` next to the app in the download repeats these steps. Releases whose notes say "notarized by Apple" skip this section entirely.

## Updating

Download the new release and drag it onto Applications again, replacing the old copy. Your Worlds are kept: they live in `~/Library/Application Support/World Machine`, not inside the app. **Help → Check for Updates…** opens the Releases page.

## If something goes wrong

In the app, choose **Help → Copy Diagnostics**, then **Help → Report a Problem…** and paste the diagnostics into the issue. The text contains the build label, your macOS version, the paths the app uses, and the last lines of its log; nothing else. **Help → Show Log in Finder** opens the log, and **World Machine → About World Machine…** shows the build label.

If the app does not start at all, open an issue with the [bug report template](https://github.com/hxddh/world-machine/issues/new/choose) and include your macOS version and the release you downloaded.

## For the technically inclined

- Every release also publishes a `.zip`, a `.sha256` for each download, and `release-manifest.json` with the exact commit, architectures, and signing status. `shasum -a 256 -c <file>.sha256` verifies a download.
- `scripts/install.sh` installs or updates from the Terminal in one line and clears the first-launch block itself; it is optional and does nothing the steps above do not.
- The experimental World Analyst needs Node and the Pi runtime; its menu entry says so when they are missing.
