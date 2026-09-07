# Making World Machine a normal macOS app

A normal macOS app is one you download, open, and use. On macOS that is only possible for apps signed with an Apple **Developer ID** and **notarized** by Apple. Without both, Gatekeeper interrupts the first launch, and no packaging trick removes that. The release pipeline is already built for the signed path; it turns on the moment five repository secrets exist.

## What to do, once

1. Enrol in the [Apple Developer Program](https://developer.apple.com/programs/enroll/) (USD 99 per year). Enrolment for an individual takes a day or two.
2. In Xcode → Settings → Accounts → Manage Certificates, create a **Developer ID Application** certificate. Export it from Keychain Access as a `.p12` with a password.
3. Create an [app-specific password](https://support.apple.com/102654) for the Apple ID.
4. Note the ten-character Team ID from the [membership page](https://developer.apple.com/account#MembershipDetailsCard).
5. Add these repository secrets under Settings → Secrets and variables → Actions:

| Secret | Value |
| --- | --- |
| `APPLE_CERTIFICATE_P12` | `base64 -i DeveloperID.p12 \| pbcopy` |
| `APPLE_CERTIFICATE_PASSWORD` | the `.p12` password |
| `APPLE_ID` | the Apple ID email |
| `APPLE_TEAM_ID` | the Team ID |
| `APPLE_APP_PASSWORD` | the app-specific password |

The next release dispatch signs every executable with the hardened runtime, submits the app and the DMG to Apple's notary service, staples the tickets, and publishes. The manifest reads `"signing": "developer-id", "notarized": true`, the release notes drop the warning, and the `Read Me First.txt` is no longer included. Without the secrets the pipeline builds the ad-hoc pre-alpha package exactly as before.

## What the pipeline does with them

- `build-app.sh` signs the analyst host, both Pack executables (before they are embedded in their `.worldpack` bundles), and the app with `--options runtime --timestamp` and the Developer ID identity from `WORLD_MACHINE_SIGNING_IDENTITY`.
- `package-release.sh` detects the Developer ID signature, submits the app with `notarytool`, staples it, builds the DMG, signs and notarizes the DMG too, and records the result in `release-manifest.json`.
- `validate_release_package.py` refuses a Developer ID package that is not notarized, so a half-configured pipeline fails instead of publishing a confusing build.
- The secrets never leave the runner: the certificate is imported into a temporary keychain that is discarded with the job.

## Local signing

To sign locally with a certificate in the login keychain:

```bash
WORLD_MACHINE_SIGNING_IDENTITY="Developer ID Application: Your Name (TEAMID)" \
  bash apps/world-machine-desktop/macos/build-app.sh
xcrun notarytool store-credentials world-machine --apple-id … --team-id … --password …
WORLD_MACHINE_NOTARIZE=1 WORLD_MACHINE_NOTARY_PROFILE=world-machine \
  bash apps/world-machine-desktop/macos/package-release.sh
```
