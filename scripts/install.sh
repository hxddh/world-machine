#!/bin/sh
# World Machine one-line installer for macOS.
#
#   curl -fsSL https://raw.githubusercontent.com/hxddh/world-machine/main/scripts/install.sh | sh
#
# Downloads the latest pre-release, checks its SHA-256 against the published
# checksum, puts "World Machine.app" in /Applications (or ~/Applications when
# /Applications is not writable), clears the quarantine flag so macOS does not
# block the first launch, and opens the app. Re-running it updates in place.
# Nothing else is installed and nothing is sent anywhere: the only network
# access is to github.com to fetch the release.
#
# Pass a tag as the first argument to install that release instead of the
# latest one, e.g. `sh install.sh v0.1.0-pre.7`.
set -eu

REPO="hxddh/world-machine"
APP_NAME="World Machine.app"
WANTED_TAG="${1:-}"

say() { printf '%s\n' "$*"; }
fail() { printf 'install: %s\n' "$*" >&2; exit 1; }

[ "$(uname -s)" = "Darwin" ] || fail "World Machine runs on macOS; this installer does nothing elsewhere."
for tool in curl shasum ditto xattr open; do
    command -v "$tool" >/dev/null 2>&1 || fail "$tool is missing; it ships with macOS, so something is unusual about this Mac."
done

api="https://api.github.com/repos/$REPO/releases"
if [ -n "$WANTED_TAG" ]; then
    api="$api/tags/$WANTED_TAG"
else
    api="$api?per_page=10"
fi
say "Looking up the latest World Machine release…"
listing="$(curl -fsSL -H 'Accept: application/vnd.github+json' "$api")" \
    || fail "could not reach github.com to find the release"

# The first zip in the listing belongs to the newest release; the listing is
# newest-first and every release carries exactly one zip.
zip_url="$(printf '%s' "$listing" | tr -d '\n' | grep -o '"browser_download_url": *"[^"]*World-Machine-[^"]*-macOS-[^"]*\.zip"' | head -n 1 | sed 's/.*"\(https[^"]*\)"/\1/')"
[ -n "$zip_url" ] || fail "no macOS package found in the release listing"
zip_name="${zip_url##*/}"
tag="$(printf '%s' "$zip_url" | sed 's|.*/download/\([^/]*\)/.*|\1|')"

workdir="$(mktemp -d "${TMPDIR:-/tmp}/world-machine-install.XXXXXX")"
trap 'rm -rf "$workdir"' EXIT

say "Downloading $tag…"
curl -fL --progress-bar -o "$workdir/$zip_name" "$zip_url"
curl -fsSL -o "$workdir/$zip_name.sha256" "$zip_url.sha256" || fail "could not download the checksum for $zip_name"

say "Checking the download…"
(cd "$workdir" && shasum -a 256 -c "$zip_name.sha256" >/dev/null) || fail "checksum mismatch; the download is damaged or altered, nothing was installed"

say "Unpacking…"
ditto -x -k "$workdir/$zip_name" "$workdir/unpacked"
app_path="$(find "$workdir/unpacked" -maxdepth 2 -name "$APP_NAME" -type d | head -n 1)"
[ -n "$app_path" ] || fail "the package did not contain $APP_NAME"

target_dir="/Applications"
[ -w "$target_dir" ] || target_dir="$HOME/Applications"
mkdir -p "$target_dir"
target="$target_dir/$APP_NAME"
if [ -d "$target" ]; then
    say "Replacing the copy in $target_dir…"
    osascript -e "tell application \"World Machine\" to quit" >/dev/null 2>&1 || true
    rm -rf "$target"
fi
ditto "$app_path" "$target"

# The zip was downloaded by curl, not a browser, so it normally carries no
# quarantine flag; clear it anyway so a Finder-downloaded copy behaves the same.
xattr -dr com.apple.quarantine "$target" 2>/dev/null || true

say "Installed $tag to $target"
say "Your Worlds live in ~/Library/Application Support/World Machine; the app keeps everything on this Mac."
open -a "$target"
