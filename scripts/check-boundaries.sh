#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CORE="$ROOT/crates/world-core/src"

forbidden=("TinySociety" "Tiny Society" "Person" "Town" "Bakery" "Society" "gpui" "pi_agent" "FootballPlayer" "Evidence")

failed=0
for token in "${forbidden[@]}"; do
  if grep -Rni --exclude-dir=target -- "$token" "$CORE" >/tmp/world-machine-boundary-check 2>/dev/null; then
    echo "Boundary violation: '$token' found in world-core:"
    cat /tmp/world-machine-boundary-check
    failed=1
  fi
done

if [[ "$failed" -ne 0 ]]; then
  exit 1
fi

echo "Architecture boundary check passed."
