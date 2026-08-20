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
ANALYST_RUNTIME_DIR="$APP_DIR/Contents/Resources/Analyst Runtime"
ANALYST_BIN_DIR="$ANALYST_RUNTIME_DIR/bin"
ANALYST_INTEGRATION_DIR="$ANALYST_RUNTIME_DIR/integrations/pi"
ANALYST_SCRIPT_DIR="$ANALYST_RUNTIME_DIR/scripts"

if [[ -n "${WORLD_MACHINE_BUILD_COMMIT:-}" ]]; then
    BUILD_COMMIT="$WORLD_MACHINE_BUILD_COMMIT"
elif [[ -n "${GITHUB_SHA:-}" ]]; then
    BUILD_COMMIT="${GITHUB_SHA:0:12}"
else
    BUILD_COMMIT="$(git rev-parse --short=12 HEAD 2>/dev/null || printf 'unknown')"
fi
export WORLD_MACHINE_BUILD_COMMIT="$BUILD_COMMIT"

case "$PROFILE" in
    release)
        cargo build \
            -p world-machine-desktop \
            -p world-agent-tool-stdio \
            -p pocket-universe-pack \
            -p micro-company-pack \
            --release
        PROFILE_DIR="release"
        ;;
    debug)
        cargo build \
            -p world-machine-desktop \
            -p world-agent-tool-stdio \
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
ANALYST_HOST_BINARY="$TARGET_DIR/$PROFILE_DIR/world-agent-tool-stdio"
POCKET_UNIVERSE_BINARY="$TARGET_DIR/$PROFILE_DIR/pocket-universe-pack"
MICRO_COMPANY_BINARY="$TARGET_DIR/$PROFILE_DIR/micro-company-pack"
for executable in \
    "$BINARY_PATH" \
    "$ANALYST_HOST_BINARY" \
    "$POCKET_UNIVERSE_BINARY" \
    "$MICRO_COMPANY_BINARY"; do
    if [[ ! -x "$executable" ]]; then
        echo "built binary is missing or not executable: $executable" >&2
        exit 1
    fi
done

rm -rf "$APP_DIR"
mkdir -p \
    "$APP_DIR/Contents/MacOS" \
    "$INCLUDED_PACK_DIR" \
    "$ANALYST_BIN_DIR" \
    "$ANALYST_INTEGRATION_DIR" \
    "$ANALYST_SCRIPT_DIR"
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

for module in \
    world-machine-analyst-turn-host.mjs \
    world-machine-analyst-rpc.mjs \
    world-machine-analyst.mjs \
    world-machine-analyst-client.mjs; do
    cp "$ROOT_DIR/integrations/pi/$module" "$ANALYST_INTEGRATION_DIR/$module"
done
cp "$ROOT_DIR/scripts/run-pi-analyst.sh" "$ANALYST_SCRIPT_DIR/run-pi-analyst.sh"
cp "$ANALYST_HOST_BINARY" "$ANALYST_BIN_DIR/world-agent-tool-stdio"
chmod +x \
    "$ANALYST_SCRIPT_DIR/run-pi-analyst.sh" \
    "$ANALYST_BIN_DIR/world-agent-tool-stdio"

for runtime_file in \
    "$ANALYST_INTEGRATION_DIR/world-machine-analyst-turn-host.mjs" \
    "$ANALYST_INTEGRATION_DIR/world-machine-analyst-rpc.mjs" \
    "$ANALYST_INTEGRATION_DIR/world-machine-analyst.mjs" \
    "$ANALYST_INTEGRATION_DIR/world-machine-analyst-client.mjs" \
    "$ANALYST_SCRIPT_DIR/run-pi-analyst.sh" \
    "$ANALYST_BIN_DIR/world-agent-tool-stdio"; do
    if [[ ! -s "$runtime_file" ]]; then
        echo "bundled analyst runtime file is missing or empty: $runtime_file" >&2
        exit 1
    fi
done
if [[ ! -x "$ANALYST_SCRIPT_DIR/run-pi-analyst.sh" ]]; then
    echo "bundled analyst launcher is not executable" >&2
    exit 1
fi
if [[ ! -x "$ANALYST_BIN_DIR/world-agent-tool-stdio" ]]; then
    echo "bundled analyst host is not executable" >&2
    exit 1
fi

plutil -lint "$APP_DIR/Contents/Info.plist"

python3 - "$APP_DIR/Contents/Info.plist" "$INCLUDED_PACK_DIR" "$ANALYST_RUNTIME_DIR" <<'PY'
import plistlib
import sys
from pathlib import Path

plist_path = Path(sys.argv[1])
included_pack_dir = Path(sys.argv[2])
analyst_runtime_dir = Path(sys.argv[3])
with plist_path.open("rb") as file:
    plist = plistlib.load(file)

world_type = "io.github.hxddh.world-machine.world"
pack_type = "io.github.hxddh.world-machine.worldpack"
assert plist["CFBundleExecutable"] == "world-machine-desktop"
assert plist["CFBundleIdentifier"] == "io.github.hxddh.world-machine"
assert plist["CFBundlePackageType"] == "APPL"

document_types = {
    item["LSItemContentTypes"][0]: item
    for item in plist["CFBundleDocumentTypes"]
}
assert set(document_types) == {world_type, pack_type}
assert document_types[world_type]["CFBundleTypeRole"] == "Editor"
assert document_types[pack_type]["CFBundleTypeRole"] == "Viewer"
assert document_types[world_type]["LSHandlerRank"] == "Owner"
assert document_types[pack_type]["LSHandlerRank"] == "Owner"

exported_types = {
    item["UTTypeIdentifier"]: item
    for item in plist["UTExportedTypeDeclarations"]
}
assert set(exported_types) == {world_type, pack_type}
world = exported_types[world_type]
assert "public.json" in world["UTTypeConformsTo"]
assert "public.content" in world["UTTypeConformsTo"]
assert world["UTTypeTagSpecification"]["public.filename-extension"] == ["world"]
pack = exported_types[pack_type]
assert "public.data" in pack["UTTypeConformsTo"]
assert "public.content" in pack["UTTypeConformsTo"]
assert pack["UTTypeTagSpecification"]["public.filename-extension"] == ["worldpack"]

expected_packs = {
    "pocket-universe.worldpack",
    "micro-company.worldpack",
}
actual_packs = {path.name for path in included_pack_dir.iterdir() if path.is_file()}
assert actual_packs == expected_packs, (actual_packs, expected_packs)
assert all((included_pack_dir / name).stat().st_size > 0 for name in expected_packs)

expected_runtime_files = {
    "integrations/pi/world-machine-analyst-turn-host.mjs",
    "integrations/pi/world-machine-analyst-rpc.mjs",
    "integrations/pi/world-machine-analyst.mjs",
    "integrations/pi/world-machine-analyst-client.mjs",
    "scripts/run-pi-analyst.sh",
    "bin/world-agent-tool-stdio",
}
for relative in expected_runtime_files:
    path = analyst_runtime_dir / relative
    assert path.is_file() and path.stat().st_size > 0, path
PY

codesign --force --sign - "$ANALYST_BIN_DIR/world-agent-tool-stdio"
codesign --force --sign - "$APP_DIR"
codesign --verify --strict --verbose=2 "$APP_DIR"

echo "$APP_DIR"
