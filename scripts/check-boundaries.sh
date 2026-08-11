#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CORE="$ROOT/crates/world-core/src"
AGENT="$ROOT/crates/world-agent/src"
PI_RPC="$ROOT/crates/world-pi-rpc"
PROJECTION="$ROOT/crates/world-projection"
HOST="$ROOT/crates/world-host"
LIBRARY="$ROOT/crates/world-library"
GPUI="$ROOT/crates/world-gpui"
STRATEGY_GPUI="$ROOT/crates/world-strategy-gpui"
DESKTOP="$ROOT/apps/world-machine-desktop"

core_forbidden=("TinySociety" "Tiny Society" "FutureArchaeologist" "Future Archaeologist" "Person" "Town" "Bakery" "Society" "gpui" "pi_agent" "FootballPlayer" "Evidence" "world_agent")
agent_forbidden=("pi_agent" "openai" "anthropic" "gpui")
pi_rpc_forbidden=("pi_agent_rust")
projection_forbidden=("TinySociety" "Tiny Society" "FutureArchaeologist" "Future Archaeologist" "Bakery" "Society" "gpui" "pi_agent")
host_forbidden=("TinySociety" "tiny_society" "FutureArchaeologist" "future_archaeologist" "gpui" "pi_agent")
library_forbidden=("TinySociety" "tiny_society" "FutureArchaeologist" "future_archaeologist" "gpui" "pi_agent")
gpui_forbidden=("TinySociety" "tiny_society" "FutureArchaeologist" "future_archaeologist" "world_core" "pi_agent")
strategy_gpui_forbidden=("TinySociety" "tiny_society" "FutureArchaeologist" "future_archaeologist" "Bakery" "Mara" "Jonas" "world_core" "pi_agent")
desktop_forbidden=("TinySociety" "tiny_society" "FutureArchaeologist" "future_archaeologist")

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

if [[ -d "$PI_RPC" ]]; then
  for token in "${pi_rpc_forbidden[@]}"; do
    if grep -Rni --exclude-dir=target -i -- "$token" "$PI_RPC" >/tmp/world-machine-pi-boundary-check 2>/dev/null; then
      echo "Boundary violation: '$token' found in out-of-process world-pi-rpc adapter:"
      cat /tmp/world-machine-pi-boundary-check
      failed=1
    fi
  done
fi

if [[ -d "$PROJECTION" ]]; then
  for token in "${projection_forbidden[@]}"; do
    if grep -Rni --exclude-dir=target -i -- "$token" "$PROJECTION" >/tmp/world-machine-projection-boundary-check 2>/dev/null; then
      echo "Boundary violation: '$token' found in generic world-projection:"
      cat /tmp/world-machine-projection-boundary-check
      failed=1
    fi
  done
fi

if [[ -d "$HOST" ]]; then
  for token in "${host_forbidden[@]}"; do
    if grep -Rni --exclude-dir=target -i -- "$token" "$HOST" >/tmp/world-machine-host-boundary-check 2>/dev/null; then
      echo "Boundary violation: '$token' found in generic world-host:"
      cat /tmp/world-machine-host-boundary-check
      failed=1
    fi
  done
fi

if [[ -d "$LIBRARY" ]]; then
  for token in "${library_forbidden[@]}"; do
    if grep -Rni --exclude-dir=target -i -- "$token" "$LIBRARY" >/tmp/world-machine-library-boundary-check 2>/dev/null; then
      echo "Boundary violation: '$token' found in generic world-library:"
      cat /tmp/world-machine-library-boundary-check
      failed=1
    fi
  done
fi

if [[ -d "$GPUI" ]]; then
  for token in "${gpui_forbidden[@]}"; do
    if grep -Rni --exclude-dir=target -i -- "$token" "$GPUI" >/tmp/world-machine-gpui-boundary-check 2>/dev/null; then
      echo "Boundary violation: '$token' found in world-gpui renderer:"
      cat /tmp/world-machine-gpui-boundary-check
      failed=1
    fi
  done
fi

if [[ -d "$STRATEGY_GPUI" ]]; then
  for token in "${strategy_gpui_forbidden[@]}"; do
    if grep -Rni --exclude-dir=target -i -- "$token" "$STRATEGY_GPUI" >/tmp/world-machine-strategy-gpui-boundary-check 2>/dev/null; then
      echo "Boundary violation: '$token' found in generic world-strategy-gpui renderer:"
      cat /tmp/world-machine-strategy-gpui-boundary-check
      failed=1
    fi
  done
fi

if [[ -d "$DESKTOP" ]]; then
  for token in "${desktop_forbidden[@]}"; do
    if grep -Rni --exclude-dir=target -i -- "$token" "$DESKTOP" >/tmp/world-machine-desktop-boundary-check 2>/dev/null; then
      echo "Boundary violation: '$token' found in unified World Machine desktop:"
      cat /tmp/world-machine-desktop-boundary-check
      failed=1
    fi
  done
fi

if [[ "$failed" -ne 0 ]]; then
  exit 1
fi

echo "Architecture boundary check passed."
