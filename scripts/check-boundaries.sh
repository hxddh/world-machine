#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CORE="$ROOT/crates/world-core/src"
AGENT="$ROOT/crates/world-agent/src"

core_forbidden=("TinySociety" "Tiny Society" "Person" "Town" "Bakery" "Society" "gpui" "pi_agent" "FootballPlayer" "Evidence" "world_agent")
agent_forbidden=("pi_agent" "openai" "anthropic" "gpui")

failed=0
for token in "${core_forbidden[@]}"; do
  if grep -Rni --exclude-dir=target -- "$token" "$CORE" >/tmp/world-machine-boundary-check 2>/dev/null; then
    echo "Boundary violation: '$token' found in world-core:"
    cat /tmp/world-machine-boundary-check
    failed=1
  fi
done

if [[ -d "$AGENT" ]]; then
  for token in "${agent_forbidden[@]}"; do
    if grep -Rni --exclude-dir=target -i -- "$token" "$AGENT" >/tmp/world-machine-agent-boundary-check 2>/dev/null; then
      echo "Boundary violation: '$token' found in provider-neutral world-agent:"
      cat /tmp/world-machine-agent-boundary-check
      failed=1
    fi
  done
fi

if [[ "$failed" -ne 0 ]]; then
  exit 1
fi

echo "Architecture boundary check passed."
