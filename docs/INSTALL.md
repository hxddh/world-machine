# Install World Machine on macOS

World Machine ships as a **pre-alpha, ad-hoc signed, not notarized** macOS app. That means macOS will stop the first launch with a message that it "could not verify" the app. This is expected for an unnotarized build, not a sign that the download is damaged. The steps below take under two minutes.

Requirements: an Apple Silicon or Intel Mac running macOS 14 (Sonoma) or macOS 15 (Sequoia). Check the release manifest for the architecture of the build you downloaded.

## 1. Download and verify

From the [Releases page](https://github.com/hxddh/world-machine/releases), download the `.zip` and its `.zip.sha256` for the version you want. Then, in Terminal, from the download folder:

```bash
shasum -a 256 -c World-Machine-*.zip.sha256
```

The output must end with `OK`. If it does not, delete the file and download again.

Also open `release-manifest.json` on the release page and confirm the tag, commit, and architecture match what you expect. Every manifest for this release line says `"notarized": false`; that is the current status, not an error.

## 2. Unpack and move to Applications

Double-click the `.zip`. Drag `World Machine.app` into `/Applications`.

Do this before the first launch. macOS runs an unnotarized app that is still in Downloads from a temporary read-only location, which slows the first start and can confuse file dialogs.

## 3. Allow the first launch

Pick one of the two options. Option A is the reliable path and also covers the World Packs bundled inside the app. Option B uses only System Settings.

### Option A: Terminal (recommended)

```bash
xattr -dr com.apple.quarantine "/Applications/World Machine.app"
```

Then open the app normally. This removes the download quarantine flag from the app and everything inside it, including the bundled World Packs and the analyst tool host, which the app launches as separate processes.

### Option B: System Settings

The exact steps depend on your macOS version.

**macOS 15 Sequoia**

1. Double-click `World Machine.app`. A dialog says the app was not opened because Apple could not verify it. Click **Done**.
2. Open **System Settings → Privacy & Security** and scroll down to the **Security** section.
3. You will see a line saying World Machine was blocked to protect your Mac. Click **Open Anyway**.
4. Authenticate, then click **Open Anyway** again in the confirmation dialog.

The Control-click → Open shortcut that older macOS versions offered no longer bypasses Gatekeeper on macOS 15, so do not rely on it.

**macOS 14 Sonoma**

1. In Finder, Control-click `World Machine.app` and choose **Open**.
2. In the dialog, click **Open**.

If a World fails to start after Option B, run the Option A command once. The bundled Packs are separate executables and may still carry the quarantine flag.

## 4. First run

The app opens on Home. Choose **Start here** to seed a Pocket Universe World. Your Worlds are saved as `.world` files in the World Machine library under `~/Library/Application Support`.

## Updating

Download the new release, verify it the same way, and replace the app in `/Applications`. Repeat step 3 for the new copy: the quarantine flag is set on every download. Your saved Worlds are not inside the app bundle and are kept.

## Known limits of the pre-alpha build

- Not notarized. Every fresh download needs step 3.
- No automatic updates. Watch the Releases page.
- The World Analyst panel is experimental and needs Node and the Pi runtime installed separately; it is not required to use Worlds.

## Something went wrong?

Open an issue with the [bug report template](https://github.com/hxddh/world-machine/issues/new/choose). Include the build label shown in the app (it looks like `Pre-alpha 0.1.0 · build abc123def456 · aarch64`) and your macOS version.
