#!/usr/bin/env bash
# Builds World Machine for Windows and packages it: a zip of the app folder
# (always) and an Inno Setup installer (when ISCC.exe is there), each with a
# .sha256 beside it, in target/windows-package. Run on Windows from Git Bash
# (the release workflow's `windows-release` job does).
#
# Unsigned unless WORLD_MACHINE_WINDOWS_SIGN is set to a command that signs
# the file named as its last argument in place, for example
#   signtool sign /fd SHA256 /tr http://timestamp.acs.microsoft.com /td SHA256 /dlib ... /dmdf ...
# (Azure Artifact Signing) or signtool with a standard certificate; see
# docs/RELEASE_SIGNING.md, "Windows". The app is signed before it is zipped
# and put in the installer, and the installer after it is made.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
HERE="$ROOT/apps/world-machine-desktop/windows"
OUT="$ROOT/target/windows-package"
STAGE="$OUT/stage"
FEATURES="${WORLD_MACHINE_WINDOWS_FEATURES:-}"

cd "$ROOT"
VERSION="$(cargo metadata --locked --no-deps --format-version 1 | python -c '
import json, sys
for package in json.load(sys.stdin)["packages"]:
    if package["name"] == "world-machine-desktop":
        print(package["version"])
        break
')"
LABEL="${WORLD_MACHINE_RELEASE_TAG:-v$VERSION}"

sign() {
    if [[ -n "${WORLD_MACHINE_WINDOWS_SIGN:-}" ]]; then
        echo "signing $1"
        # shellcheck disable=SC2086
        $WORLD_MACHINE_WINDOWS_SIGN "$1"
    fi
}

cargo build --locked --release -p world-machine-desktop ${FEATURES:+--features "$FEATURES"}
cargo build --locked --release -p tiny-society-pack -p pocket-universe-pack

rm -rf "$OUT"
mkdir -p "$STAGE/World Packs"
cp target/release/world-machine-desktop.exe "$STAGE/World Machine.exe"
sign "$STAGE/World Machine.exe"
for pack in tiny-society pocket-universe; do
    "target/release/$pack-pack.exe" --write-bundle "$STAGE/World Packs/$pack.worldpack"
done
python "$HERE/make-icon.py" apps/world-machine-desktop/macos/icon.png "$STAGE/World Machine.ico"
sed "s/@VERSION@/$LABEL/" "$HERE/READ_ME_FIRST.txt" | sed 's/$/\r/' > "$STAGE/READ ME FIRST.txt"
sed 's/$/\r/' LICENSE > "$STAGE/LICENSE.txt"

# The zip: the app folder as the installer lays it out.
ZIP="World-Machine-$VERSION-Windows-x64.zip"
(cd "$STAGE" && 7z a -tzip -bso0 -bsp0 "$OUT/$ZIP" .)

# The installer, where Inno Setup is installed.
ISCC="${ISCC:-}"
if [[ -z "$ISCC" ]]; then
    for candidate in "/c/Program Files (x86)/Inno Setup 6/ISCC.exe" "/c/Program Files/Inno Setup 6/ISCC.exe"; do
        [[ -x "$candidate" ]] && ISCC="$candidate" && break
    done
fi
if [[ -n "$ISCC" ]]; then
    "$ISCC" /Qp "/DAppVersion=$VERSION" "/DStageDir=$(cygpath -w "$STAGE")" \
        "/DOutputDir=$(cygpath -w "$OUT")" "$(cygpath -w "$HERE/world-machine.iss")"
    sign "$OUT/World-Machine-$VERSION-Windows-x64-Setup.exe"
else
    echo "::warning title=No Windows installer::Inno Setup (ISCC.exe) is not installed; only the zip was made."
fi

cd "$OUT"
for file in *.zip *.exe; do
    [[ -f "$file" ]] || continue
    sha256sum "$file" | sed 's/ \*/  /' > "$file.sha256"
done
signing="unsigned"
[[ -n "${WORLD_MACHINE_WINDOWS_SIGN:-}" ]] && signing="signed"
python - "$VERSION" "$LABEL" "$signing" <<'PY'
import json, os, sys
version, label, signing = sys.argv[1:]
files = sorted(name for name in os.listdir(".") if name.endswith((".zip", ".exe")))
json.dump(
    {"app_version": version, "tag": label, "platform": "windows-x64",
     "signing": signing, "files": files,
     "included_packs": ["tiny-society.worldpack", "pocket-universe.worldpack"]},
    open("release-manifest-windows.json", "w"), indent=2)
PY
ls -l "$OUT"
