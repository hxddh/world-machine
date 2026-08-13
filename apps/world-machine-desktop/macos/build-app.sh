#!/usr/bin/env bash
set -euo pipefail

if [[ "$(uname -s)" != "Darwin" ]]; then
    echo "build-app.sh must run on macOS" >&2
    exit 2
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../../.." && pwd)"
cd "$ROOT_DIR"

PROFILE="${WORLD_MACHINE_PROFILE:-release}"
TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT_DIR/target}"
APP_DIR="${WORLD_MACHINE_APP_DIR:-$TARGET_DIR/bundle/World Machine.app}"
PLIST_TEMPLATE="$SCRIPT_DIR/Info.plist.in"
BINARY_NAME="world-machine-desktop"
INCLUDED_PACK_DIR="$APP_DIR/Contents/Resources/World Packs"

case "$PROFILE" in
    release)
        cargo build \
            -p world-machine-desktop \
            -p pocket-universe-pack \
            -p micro-company-pack \
            --release
        PROFILE_DIR="release"
        ;;
    debug)
        cargo build \
            -p world-machine-desktop \
            -p pocket-universe-pack \
            -p micro-company-pack
        PROFILE_DIR="debug"
        ;;
    *)
        echo "unsupported WORLD_MACHINE_PROFILE: $PROFILE (expected release or debug)" >&2
        exit 2
        ;;
esac

VERSION="$(cargo metadata --no-deps --format-version 1 | python3 -c '
import json, sys
metadata = json.load(sys.stdin)
for package in metadata["packages"]:
    if package["name"] == "world-machine-desktop":
        print(package["version"])
        break
else:
    raise SystemExit("world-machine-desktop package not found")
')"

BINARY_PATH="$TARGET_DIR/$PROFILE_DIR/$BINARY_NAME"
POCKET_UNIVERSE_BINARY="$TARGET_DIR/$PROFILE_DIR/pocket-universe-pack"
MICRO_COMPANY_BINARY="$TARGET_DIR/$PROFILE_DIR/micro-company-pack"
for executable in "$BINARY_PATH" "$POCKET_UNIVERSE_BINARY" "$MICRO_COMPANY_BINARY"; do
    if [[ ! -x "$executable" ]]; then
        echo "built binary is missing or not executable: $executable" >&2
        exit 1
    fi
done

rm -rf "$APP_DIR"
mkdir -p "$APP_DIR/Contents/MacOS" "$INCLUDED_PACK_DIR"
cp "$BINARY_PATH" "$APP_DIR/Contents/MacOS/$BINARY_NAME"
chmod +x "$APP_DIR/Contents/MacOS/$BINARY_NAME"
sed "s/@VERSION@/$VERSION/g" "$PLIST_TEMPLATE" > "$APP_DIR/Contents/Info.plist"

"$POCKET_UNIVERSE_BINARY" \
    --write-bundle "$INCLUDED_PACK_DIR/pocket-universe.worldpack"
"$MICRO_COMPANY_BINARY" \
    --write-bundle "$INCLUDED_PACK_DIR/micro-company.worldpack"

for bundle in \
    "$INCLUDED_PACK_DIR/pocket-universe.worldpack" \
    "$INCLUDED_PACK_DIR/micro-company.worldpack"; do
    if [[ ! -s "$bundle" ]]; then
        echo "included World Pack is missing or empty: $bundle" >&2
        exit 1
    fi
    cargo run -p world-pack-catalog --bin world-pack-check -- \
        --inspect-only "$bundle"
done

plutil -lint "$APP_DIR/Contents/Info.plist"

python3 - "$APP_DIR/Contents/Info.plist" "$INCLUDED_PACK_DIR" <<'PY'
import plistlib
import sys
from pathlib import Path

plist_path = Path(sys.argv[1])
included_pack_dir = Path(sys.argv[2])
with plist_path.open("rb") as file:
    plist = plistlib.load(file)

expected_type = "io.github.hxddh.world-machine.world"
assert plist["CFBundleExecutable"] == "world-machine-desktop"
assert plist["CFBundleIdentifier"] == "io.github.hxddh.world-machine"
assert plist["CFBundlePackageType"] == "APPL"
assert plist["CFBundleDocumentTypes"][0]["LSItemContentTypes"] == [expected_type]
exported = plist["UTExportedTypeDeclarations"][0]
assert exported["UTTypeIdentifier"] == expected_type
assert "public.json" in exported["UTTypeConformsTo"]
assert "public.content" in exported["UTTypeConformsTo"]
assert exported["UTTypeTagSpecification"]["public.filename-extension"] == ["world"]

expected_packs = {
    "pocket-universe.worldpack",
    "micro-company.worldpack",
}
actual_packs = {path.name for path in included_pack_dir.iterdir() if path.is_file()}
assert actual_packs == expected_packs, (actual_packs, expected_packs)
assert all((included_pack_dir / name).stat().st_size > 0 for name in expected_packs)
PY

codesign --force --sign - "$APP_DIR"
codesign --verify --strict --verbose=2 "$APP_DIR"

echo "$APP_DIR"
