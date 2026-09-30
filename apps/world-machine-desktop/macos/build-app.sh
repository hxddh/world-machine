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
ICON_SOURCE="$SCRIPT_DIR/icon.png"
ICONSET_DIR="$TARGET_DIR/bundle/AppIcon.iconset"
RESOURCES_DIR="$APP_DIR/Contents/Resources"
BINARY_NAME="world-machine-desktop"
INCLUDED_PACK_DIR="$APP_DIR/Contents/Resources/World Packs"

if [[ -n "${WORLD_MACHINE_BUILD_COMMIT:-}" ]]; then
    BUILD_COMMIT="$WORLD_MACHINE_BUILD_COMMIT"
elif [[ -n "${GITHUB_SHA:-}" ]]; then
    BUILD_COMMIT="${GITHUB_SHA:0:12}"
else
    BUILD_COMMIT="$(git rev-parse --short=12 HEAD 2>/dev/null || printf 'unknown')"
fi
export WORLD_MACHINE_BUILD_COMMIT="$BUILD_COMMIT"

# WORLD_MACHINE_SIGNING_IDENTITY names a "Developer ID Application" identity
# in the keychain. Without it every executable is ad-hoc signed ("-"), which
# is what a local build and the unsigned pre-alpha channel use. With it,
# every Mach-O gets a hardened-runtime, timestamped Developer ID signature,
# which is what notarization requires; the Pack executables are signed before
# they are embedded in their .worldpack bundles.
SIGNING_IDENTITY="${WORLD_MACHINE_SIGNING_IDENTITY:--}"
sign_executable() {
    if [[ "$SIGNING_IDENTITY" == "-" ]]; then
        codesign --force --sign - "$1"
    else
        codesign --force --timestamp --options runtime --sign "$SIGNING_IDENTITY" "$1"
    fi
}

# Every World Pack the app ships, named the way it is named on the wire. One
# list drives the build, the signature, the bundle written into Resources, and
# the Info.plist check, so those four cannot disagree with each other; and
# scripts/check-included-packs.py ties this list to the Pack ids the app
# expects to find and to the versions the world crates actually build.
INCLUDED_PACK_NAMES=(
    pocket-universe
    tiny-society
)

PACKAGES=(
    -p world-machine-desktop
)
BINARIES=(
    world-machine-desktop
)
for pack_name in "${INCLUDED_PACK_NAMES[@]}"; do
    PACKAGES+=(-p "$pack_name-pack")
    BINARIES+=("$pack_name-pack")
done
case "$PROFILE" in
    release)
        PROFILE_FLAGS=(--release)
        PROFILE_DIR="release"
        ;;
    debug)
        PROFILE_FLAGS=()
        PROFILE_DIR="debug"
        ;;
    *)
        echo "unsupported WORLD_MACHINE_PROFILE: $PROFILE (expected release or debug)" >&2
        exit 2
        ;;
esac

# WORLD_MACHINE_UNIVERSAL=1 builds every bundled executable for both Apple
# Silicon and Intel and merges them with lipo, so one package serves both.
# Both targets must be installed (rustup target add x86_64-apple-darwin
# aarch64-apple-darwin). The default host-only build is unchanged.
UNIVERSAL_TARGETS=(aarch64-apple-darwin x86_64-apple-darwin)
if [[ "${WORLD_MACHINE_UNIVERSAL:-0}" == "1" ]]; then
    for target in "${UNIVERSAL_TARGETS[@]}"; do
        cargo build --locked "${PACKAGES[@]}" --target "$target" ${PROFILE_FLAGS[@]+"${PROFILE_FLAGS[@]}"}
    done
    BIN_DIR="$TARGET_DIR/universal/$PROFILE_DIR"
    rm -rf "$BIN_DIR"
    mkdir -p "$BIN_DIR"
    for binary in "${BINARIES[@]}"; do
        inputs=()
        for target in "${UNIVERSAL_TARGETS[@]}"; do
            inputs+=("$TARGET_DIR/$target/$PROFILE_DIR/$binary")
        done
        lipo -create -output "$BIN_DIR/$binary" "${inputs[@]}"
        echo "universal $binary: $(lipo -archs "$BIN_DIR/$binary")"
    done
else
    cargo build --locked "${PACKAGES[@]}" ${PROFILE_FLAGS[@]+"${PROFILE_FLAGS[@]}"}
    BIN_DIR="$TARGET_DIR/$PROFILE_DIR"
fi

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

BINARY_PATH="$BIN_DIR/$BINARY_NAME"
PACK_BINARIES=()
for pack_name in "${INCLUDED_PACK_NAMES[@]}"; do
    PACK_BINARIES+=("$BIN_DIR/$pack_name-pack")
done
for executable in \
    "$BINARY_PATH" \
    "${PACK_BINARIES[@]}"; do
    if [[ ! -x "$executable" ]]; then
        echo "built binary is missing or not executable: $executable" >&2
        exit 1
    fi
done

for pack_binary in "${PACK_BINARIES[@]}"; do
    sign_executable "$pack_binary"
done

rm -rf "$APP_DIR"
mkdir -p \
    "$APP_DIR/Contents/MacOS" \
    "$INCLUDED_PACK_DIR"
cp "$BINARY_PATH" "$APP_DIR/Contents/MacOS/$BINARY_NAME"
chmod +x "$APP_DIR/Contents/MacOS/$BINARY_NAME"
sed "s/@VERSION@/$VERSION/g" "$PLIST_TEMPLATE" > "$APP_DIR/Contents/Info.plist"

# The Dock, the Finder, the Cmd-Tab switcher and the About window all read
# the icon from Contents/Resources/AppIcon.icns; without it macOS draws the
# blank generic-application page and the app has no logo anywhere. sips and
# iconutil ship with macOS, so one 1024px source produces every size.
rm -rf "$ICONSET_DIR"
mkdir -p "$ICONSET_DIR"
if [[ ! -s "$ICON_SOURCE" ]]; then
    echo "app icon source is missing or empty: $ICON_SOURCE" >&2
    exit 1
fi
for icon_spec in \
    "16 icon_16x16" \
    "32 icon_16x16@2x" \
    "32 icon_32x32" \
    "64 icon_32x32@2x" \
    "128 icon_128x128" \
    "256 icon_128x128@2x" \
    "256 icon_256x256" \
    "512 icon_256x256@2x" \
    "512 icon_512x512" \
    "1024 icon_512x512@2x"; do
    icon_pixels="${icon_spec%% *}"
    icon_name="${icon_spec#* }"
    sips -z "$icon_pixels" "$icon_pixels" "$ICON_SOURCE" \
        --out "$ICONSET_DIR/$icon_name.png" > /dev/null
done
iconutil --convert icns "$ICONSET_DIR" --output "$RESOURCES_DIR/AppIcon.icns"
if [[ ! -s "$RESOURCES_DIR/AppIcon.icns" ]]; then
    echo "app icon was not produced: $RESOURCES_DIR/AppIcon.icns" >&2
    exit 1
fi

for pack_name in "${INCLUDED_PACK_NAMES[@]}"; do
    bundle="$INCLUDED_PACK_DIR/$pack_name.worldpack"
    "$BIN_DIR/$pack_name-pack" --write-bundle "$bundle"
    if [[ ! -s "$bundle" ]]; then
        echo "included World Pack is missing or empty: $bundle" >&2
        exit 1
    fi
    cargo run --locked -p world-pack-catalog --bin world-pack-check -- \
        --inspect-only "$bundle"
done

plutil -lint "$APP_DIR/Contents/Info.plist"

python3 - "$APP_DIR/Contents/Info.plist" "$INCLUDED_PACK_DIR" \
    "${INCLUDED_PACK_NAMES[@]}" <<'PY'
import plistlib
import sys
from pathlib import Path

plist_path = Path(sys.argv[1])
included_pack_dir = Path(sys.argv[2])
with plist_path.open("rb") as file:
    plist = plistlib.load(file)

world_type = "io.github.hxddh.world-machine.world"
code_type = "io.github.hxddh.world-machine.worldcode"
pack_type = "io.github.hxddh.world-machine.worldpack"
assert plist["CFBundleExecutable"] == "world-machine-desktop"
assert plist["CFBundleIdentifier"] == "io.github.hxddh.world-machine"
assert plist["CFBundlePackageType"] == "APPL"
assert plist["CFBundleIconFile"] == "AppIcon"
icon = plist_path.parent / "Resources" / "AppIcon.icns"
assert icon.is_file() and icon.stat().st_size > 0, icon

document_types = {
    item["LSItemContentTypes"][0]: item
    for item in plist["CFBundleDocumentTypes"]
}
assert set(document_types) == {world_type, code_type, pack_type}
assert document_types[world_type]["CFBundleTypeRole"] == "Editor"
assert document_types[code_type]["CFBundleTypeRole"] == "Viewer"
assert document_types[pack_type]["CFBundleTypeRole"] == "Viewer"
assert all(item["LSHandlerRank"] == "Owner" for item in document_types.values())

exported_types = {
    item["UTTypeIdentifier"]: item
    for item in plist["UTExportedTypeDeclarations"]
}
assert set(exported_types) == {world_type, code_type, pack_type}
# A World is gzip-compressed, not JSON.
world = exported_types[world_type]
assert "org.gnu.gnu-zip-archive" in world["UTTypeConformsTo"]
assert "public.json" not in world["UTTypeConformsTo"]
assert "public.content" in world["UTTypeConformsTo"]
assert world["UTTypeTagSpecification"]["public.filename-extension"] == ["world"]
# A World code is text, and Finder opens it with the app, as a visit.
code = exported_types[code_type]
assert "public.utf8-plain-text" in code["UTTypeConformsTo"]
assert code["UTTypeTagSpecification"]["public.filename-extension"] == ["worldcode"]
pack = exported_types[pack_type]
assert "public.data" in pack["UTTypeConformsTo"]
assert "public.content" in pack["UTTypeConformsTo"]
assert pack["UTTypeTagSpecification"]["public.filename-extension"] == ["worldpack"]

expected_packs = {f"{name}.worldpack" for name in sys.argv[3:]}
assert expected_packs, "build-app.sh passed no included Pack names"
actual_packs = {path.name for path in included_pack_dir.iterdir() if path.is_file()}
assert actual_packs == expected_packs, (actual_packs, expected_packs)
assert all((included_pack_dir / name).stat().st_size > 0 for name in expected_packs)
PY

sign_executable "$APP_DIR"
codesign --verify --strict --verbose=2 "$APP_DIR"
if [[ "$SIGNING_IDENTITY" != "-" ]]; then
    codesign --display --verbose=2 "$APP_DIR" 2>&1 | grep -E "Authority|Timestamp|flags" || true
fi
echo "signing: $([[ "$SIGNING_IDENTITY" == "-" ]] && echo ad-hoc || echo developer-id)"

echo "$APP_DIR"
