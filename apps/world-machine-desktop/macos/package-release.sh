#!/usr/bin/env bash
set -euo pipefail

if [[ "$(uname -s)" != "Darwin" ]]; then
    echo "package-release.sh must run on macOS" >&2
    exit 2
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../../.." && pwd)"
cd "$ROOT_DIR"

TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT_DIR/target}"
APP_DIR="${WORLD_MACHINE_APP_DIR:-$TARGET_DIR/bundle/World Machine.app}"
OUTPUT_DIR="${WORLD_MACHINE_RELEASE_DIR:-$TARGET_DIR/release-package}"
BINARY="$APP_DIR/Contents/MacOS/world-machine-desktop"
PLIST="$APP_DIR/Contents/Info.plist"

if [[ ! -d "$APP_DIR" || ! -x "$BINARY" || ! -f "$PLIST" ]]; then
    echo "World Machine.app is missing; run build-app.sh first" >&2
    exit 1
fi

codesign --verify --strict --verbose=2 "$APP_DIR"

VERSION="$(plutil -extract CFBundleShortVersionString raw -o - "$PLIST")"
BUNDLE_ID="$(plutil -extract CFBundleIdentifier raw -o - "$PLIST")"
ARCHS="$(lipo -archs "$BINARY")"
ARCH_LABEL="${ARCHS// /-}"
TAG="${WORLD_MACHINE_RELEASE_TAG:-v${VERSION}-pre.0}"
COMMIT="${GITHUB_SHA:-$(git rev-parse HEAD)}"
RELEASE_LABEL="${TAG#v}"
ZIP_NAME="World-Machine-${RELEASE_LABEL}-macOS-${ARCH_LABEL}.zip"
ZIP_PATH="$OUTPUT_DIR/$ZIP_NAME"
CHECKSUM_PATH="$ZIP_PATH.sha256"
DMG_NAME="World-Machine-${RELEASE_LABEL}-macOS-${ARCH_LABEL}.dmg"
DMG_PATH="$OUTPUT_DIR/$DMG_NAME"
MANIFEST_PATH="$OUTPUT_DIR/release-manifest.json"

# Signing mode is whatever build-app.sh produced: an ad-hoc signature has no
# authority chain, a Developer ID signature does.
if codesign --display --verbose=2 "$APP_DIR" 2>&1 | grep -q "Authority=Developer ID Application"; then
    SIGNING="developer-id"
else
    SIGNING="ad-hoc"
fi
NOTARIZED="false"

rm -rf "$OUTPUT_DIR"
mkdir -p "$OUTPUT_DIR"

# Notarization needs a Developer ID signature and a notarytool keychain
# profile (WORLD_MACHINE_NOTARY_PROFILE, created with
# `xcrun notarytool store-credentials`). The app is submitted, the ticket is
# stapled to it, and the DMG below is notarized and stapled as well, so a
# download opens with no Gatekeeper dialog at all.
notarize() {
    xcrun notarytool submit "$1" --keychain-profile "$WORLD_MACHINE_NOTARY_PROFILE" --wait
}
if [[ "${WORLD_MACHINE_NOTARIZE:-0}" == "1" ]]; then
    if [[ "$SIGNING" != "developer-id" ]]; then
        echo "WORLD_MACHINE_NOTARIZE=1 needs a Developer ID signed app" >&2
        exit 1
    fi
    : "${WORLD_MACHINE_NOTARY_PROFILE:?WORLD_MACHINE_NOTARY_PROFILE names the notarytool keychain profile}"
    SUBMISSION="$OUTPUT_DIR/notarize-submission.zip"
    ditto -c -k --keepParent "$APP_DIR" "$SUBMISSION"
    notarize "$SUBMISSION"
    rm -f "$SUBMISSION"
    xcrun stapler staple "$APP_DIR"
    xcrun stapler validate "$APP_DIR"
    spctl --assess --type execute --verbose=2 "$APP_DIR"
    NOTARIZED="true"
fi

# The zip unpacks to one folder holding the app and a plain-text first-launch
# note, so a user who never sees the Release page still learns why macOS
# blocks the first open and what to do about it.
STAGE_ROOT="$OUTPUT_DIR/stage"
STAGE_DIR="$STAGE_ROOT/World Machine $RELEASE_LABEL"
mkdir -p "$STAGE_DIR"
ditto "$APP_DIR" "$STAGE_DIR/World Machine.app"
if [[ "$NOTARIZED" != "true" ]]; then
    sed "s|@TAG@|$TAG|g" "$SCRIPT_DIR/READ_ME_FIRST.txt" > "$STAGE_DIR/Read Me First.txt"
fi
ditto -c -k --keepParent "$STAGE_DIR" "$ZIP_PATH"

# The DMG is the download for people: open it, drag the app onto the
# Applications shortcut, done. The zip stays for scripts and checksums.
DMG_STAGE="$STAGE_ROOT/dmg"
mkdir -p "$DMG_STAGE"
ditto "$APP_DIR" "$DMG_STAGE/World Machine.app"
ln -s /Applications "$DMG_STAGE/Applications"
if [[ "$NOTARIZED" != "true" ]]; then
    cp "$STAGE_DIR/Read Me First.txt" "$DMG_STAGE/Read Me First.txt"
fi
hdiutil create -volname "World Machine" -srcfolder "$DMG_STAGE" -ov -format UDZO -quiet "$DMG_PATH"
if [[ "$SIGNING" == "developer-id" ]]; then
    codesign --force --timestamp --sign "$WORLD_MACHINE_SIGNING_IDENTITY" "$DMG_PATH"
fi
if [[ "$NOTARIZED" == "true" ]]; then
    notarize "$DMG_PATH"
    xcrun stapler staple "$DMG_PATH"
fi
rm -rf "$STAGE_ROOT"

SHA256="$(shasum -a 256 "$ZIP_PATH" | awk '{print $1}')"
printf '%s  %s\n' "$SHA256" "$ZIP_NAME" > "$CHECKSUM_PATH"
DMG_SHA256="$(shasum -a 256 "$DMG_PATH" | awk '{print $1}')"
printf '%s  %s\n' "$DMG_SHA256" "$DMG_NAME" > "$DMG_PATH.sha256"

python3 - \
    "$MANIFEST_PATH" \
    "$APP_DIR" \
    "$ZIP_NAME" \
    "$SHA256" \
    "$TAG" \
    "$VERSION" \
    "$BUNDLE_ID" \
    "$COMMIT" \
    "$ARCHS" \
    "$SIGNING" \
    "$NOTARIZED" \
    "$DMG_NAME" \
    "$DMG_SHA256" <<'PY'
import json
import sys
from pathlib import Path

(
    manifest_path,
    app_dir,
    artifact,
    sha256,
    tag,
    version,
    bundle_id,
    commit,
    architectures,
    signing,
    notarized,
    dmg,
    dmg_sha256,
) = sys.argv[1:]

pack_dir = Path(app_dir) / "Contents" / "Resources" / "World Packs"
included_packs = sorted(path.name for path in pack_dir.glob("*.worldpack"))
if not included_packs:
    raise SystemExit("release app does not contain any included World Packs")

manifest = {
    "schema_version": 1,
    "tag": tag,
    "app_version": version,
    "bundle_identifier": bundle_id,
    "commit": commit,
    "architectures": architectures.split(),
    "signing": signing,
    "notarized": notarized == "true",
    "artifact": artifact,
    "sha256": sha256,
    "dmg": dmg,
    "dmg_sha256": dmg_sha256,
    "included_packs": included_packs,
}
Path(manifest_path).write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n")
PY

python3 "$ROOT_DIR/scripts/validate_release_package.py" "$OUTPUT_DIR"

echo "$ZIP_PATH"
echo "$CHECKSUM_PATH"
echo "$DMG_PATH"
echo "$MANIFEST_PATH"
