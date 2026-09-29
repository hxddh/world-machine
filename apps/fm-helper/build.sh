#!/usr/bin/env bash
# Builds fm-helper, the on-device model helper, when this Mac can: `swift`
# is installed and the macOS SDK has the FoundationModels framework (macOS
# 26 and later). Anywhere else it says why and stops without failing, so a
# release is never held up by it. The helper is signed ad hoc; no Developer
# ID is needed. It lands in target/fm-helper/fm-helper; the app finds it
# through WORLD_MACHINE_FM_HELPER (crates/world-voice/src/helper.rs).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUT="$ROOT/target/fm-helper"

if ! command -v swift >/dev/null 2>&1; then
    echo "fm-helper: no swift here; skipped."
    exit 0
fi
SDK="$(xcrun --sdk macosx --show-sdk-path 2>/dev/null || true)"
if [[ -z "$SDK" || ! -d "$SDK/System/Library/Frameworks/FoundationModels.framework" ]]; then
    echo "fm-helper: this SDK has no FoundationModels; skipped."
    exit 0
fi

swift build -c release --package-path "$ROOT/apps/fm-helper" --scratch-path "$OUT/build"
mkdir -p "$OUT"
cp "$(swift build -c release --package-path "$ROOT/apps/fm-helper" --scratch-path "$OUT/build" --show-bin-path)/fm-helper" "$OUT/fm-helper"
codesign --force --sign - "$OUT/fm-helper"
echo "fm-helper: built $OUT/fm-helper (ad hoc)."
