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
MANIFEST_PATH="$OUTPUT_DIR/release-manifest.json"

rm -rf "$OUTPUT_DIR"
mkdir -p "$OUTPUT_DIR"

ditto -c -k --keepParent "$APP_DIR" "$ZIP_PATH"
SHA256="$(shasum -a 256 "$ZIP_PATH" | awk '{print $1}')"
printf '%s  %s\n' "$SHA256" "$ZIP_NAME" > "$CHECKSUM_PATH"

python3 - \
    "$MANIFEST_PATH" \
    "$APP_DIR" \
    "$ZIP_NAME" \
    "$SHA256" \
    "$TAG" \
    "$VERSION" \
    "$BUNDLE_ID" \
    "$COMMIT" \
    "$ARCHS" <<'PY'
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
    "signing": "ad-hoc",
    "notarized": False,
    "artifact": artifact,
    "sha256": sha256,
    "included_packs": included_packs,
}
Path(manifest_path).write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n")
PY

python3 "$ROOT_DIR/scripts/publish_github_prerelease.py" --dry-run "$OUTPUT_DIR"

echo "$ZIP_PATH"
echo "$CHECKSUM_PATH"
echo "$MANIFEST_PATH"
