#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CORE="$ROOT/crates/world-core/src"
AGENT="$ROOT/crates/world-agent/src"
PI_RPC="$ROOT/crates/world-pi-rpc"
PACK_PROTOCOL="$ROOT/crates/world-pack-protocol"
PACK_BUNDLE="$ROOT/crates/world-pack-bundle"
PACK_PROCESS="$ROOT/crates/world-pack-process"
PACK_CATALOG="$ROOT/crates/world-pack-catalog"
PACK_SERVER="$ROOT/crates/world-pack-server"
PROJECTION="$ROOT/crates/world-projection"
HOST="$ROOT/crates/world-host"
LIBRARY="$ROOT/crates/world-library"
GPUI="$ROOT/crates/world-gpui"
DESKTOP="$ROOT/apps/world-machine-desktop"

core_forbidden=("TinySociety" "Tiny Society" "FutureArchaeologist" "Future Archaeologist" "Person" "Town" "Bakery" "Society" "gpui" "pi_agent" "FootballPlayer" "Evidence" "world_agent")
agent_forbidden=("pi_agent" "openai" "anthropic" "gpui")
pi_rpc_forbidden=("pi_agent_rust")
pack_bundle_forbidden=("TinySociety" "tiny_society" "tiny-society" "Tiny Society" "FutureArchaeologist" "future_archaeologist" "future-archaeologist" "Future Archaeologist" "gpui" "pi_agent" "openai" "anthropic")
pack_server_forbidden=("TinySociety" "tiny_society" "tiny-society" "Tiny Society" "FutureArchaeologist" "future_archaeologist" "future-archaeologist" "Future Archaeologist" "gpui" "pi_agent" "openai" "anthropic")
pack_catalog_forbidden=("TinySociety" "tiny_society" "tiny-society" "Tiny Society" "FutureArchaeologist" "future_archaeologist" "future-archaeologist" "Future Archaeologist" "gpui" "pi_agent" "openai" "anthropic")
pack_process_forbidden=("TinySociety" "tiny_society" "tiny-society" "Tiny Society" "FutureArchaeologist" "future_archaeologist" "future-archaeologist" "Future Archaeologist" "gpui" "pi_agent" "openai" "anthropic")
pack_protocol_forbidden=("TinySociety" "tiny_society" "tiny-society" "Tiny Society" "FutureArchaeologist" "future_archaeologist" "future-archaeologist" "Future Archaeologist" "gpui" "pi_agent" "openai" "anthropic")
projection_forbidden=("TinySociety" "Tiny Society" "FutureArchaeologist" "Future Archaeologist" "Bakery" "Society" "gpui" "pi_agent")
host_forbidden=("TinySociety" "tiny_society" "FutureArchaeologist" "future_archaeologist" "gpui" "pi_agent")
library_forbidden=("TinySociety" "tiny_society" "FutureArchaeologist" "future_archaeologist" "gpui" "pi_agent")
gpui_forbidden=("TinySociety" "tiny_society" "FutureArchaeologist" "future_archaeologist" "world_core" "pi_agent")
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

if [[ -d "$PACK_PROTOCOL" ]]; then
  for token in "${pack_protocol_forbidden[@]}"; do
    if grep -Rni --exclude-dir=target -i -- "$token" "$PACK_PROTOCOL" >/tmp/world-machine-pack-protocol-boundary-check 2>/dev/null; then
      echo "Boundary violation: '$token' found in generic world-pack-protocol:"
      cat /tmp/world-machine-pack-protocol-boundary-check
      failed=1
    fi
  done
fi

if [[ -d "$PACK_BUNDLE" ]]; then
  for token in "${pack_bundle_forbidden[@]}"; do
    if grep -Rni --exclude-dir=target -i -- "$token" "$PACK_BUNDLE" >/tmp/world-machine-pack-bundle-boundary-check 2>/dev/null; then
      echo "Boundary violation: '$token' found in generic world-pack-bundle:"
      cat /tmp/world-machine-pack-bundle-boundary-check
      failed=1
    fi
  done
fi

if [[ -d "$PACK_SERVER" ]]; then
  for token in "${pack_server_forbidden[@]}"; do
    if grep -Rni --exclude-dir=target -i -- "$token" "$PACK_SERVER" >/tmp/world-machine-pack-server-boundary-check 2>/dev/null; then
      echo "Boundary violation: '$token' found in generic world-pack-server:"
      cat /tmp/world-machine-pack-server-boundary-check
      failed=1
    fi
  done
fi

if [[ -d "$PACK_CATALOG" ]]; then
  for token in "${pack_catalog_forbidden[@]}"; do
    if grep -Rni --exclude-dir=target -i -- "$token" "$PACK_CATALOG" >/tmp/world-machine-pack-catalog-boundary-check 2>/dev/null; then
      echo "Boundary violation: '$token' found in generic world-pack-catalog:"
      cat /tmp/world-machine-pack-catalog-boundary-check
      failed=1
    fi
  done
fi

if [[ -d "$PACK_PROCESS" ]]; then
  for token in "${pack_process_forbidden[@]}"; do
    if grep -Rni --exclude-dir=target -i -- "$token" "$PACK_PROCESS" >/tmp/world-machine-pack-process-boundary-check 2>/dev/null; then
      echo "Boundary violation: '$token' found in generic world-pack-process:"
      cat /tmp/world-machine-pack-process-boundary-check
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

if [[ -d "$DESKTOP" ]]; then
  for token in "${desktop_forbidden[@]}"; do
    if grep -Rni --exclude-dir=target -i -- "$token" "$DESKTOP" >/tmp/world-machine-desktop-boundary-check 2>/dev/null; then
      echo "Boundary violation: '$token' found in unified World Machine desktop:"
      cat /tmp/world-machine-desktop-boundary-check
      failed=1
    fi
  done
fi

# Invariant: no `unsafe` code. Every library and binary root in the
# workspace forbids it, except the two packages that hold their few libc
# calls in one allowed place: world-pack-process (the unix pipe calls that
# bound a write, `deadline_stdin.rs`) and world-gpui (CPU-time calls in its
# tests only). Those two deny it instead, so a new `unsafe` anywhere else in
# them fails to compile.
unsafe_exceptions=(
  "crates/world-pack-process/src/lib.rs:#![deny(unsafe_code)]"
  "crates/world-gpui/src/lib.rs:#![cfg_attr(not(test), forbid(unsafe_code))]"
)
crate_roots=$(cargo metadata --no-deps --format-version 1 --offline \
  | python3 -c '
import json, sys
for package in json.load(sys.stdin)["packages"]:
    for target in package["targets"]:
        if set(target["kind"]) & {"lib", "rlib", "bin", "proc-macro", "cdylib"}:
            print(target["src_path"])
')
if [[ -z "$crate_roots" ]]; then
  echo "Could not list the workspace's crate roots (cargo metadata)"
  failed=1
fi
while IFS= read -r crate_root; do
  [[ -z "$crate_root" ]] && continue
  relative="${crate_root#"$ROOT"/}"
  wanted="#![forbid(unsafe_code)]"
  for exception in "${unsafe_exceptions[@]}"; do
    if [[ "${exception%%:*}" == "$relative" ]]; then
      wanted="${exception#*:}"
    fi
  done
  if ! grep -qF -- "$wanted" "$crate_root"; then
    echo "Invariant violation: $relative does not say $wanted"
    failed=1
  fi
done <<< "$crate_roots"

# Ratchet: Pack names in shared code. A shared crate (the kernel, the
# Asking a model links no World code (v0.28): world-voice and what it is
# built from never depend, however indirectly, on world-core, a System, the
# projection or the agent runtime, so the desktop app does not link them
# through it.
if command -v cargo >/dev/null 2>&1; then
  for crate in world-voice world-voice-prompt world-pi-transport world-run; do
    linked=$(cd "$ROOT" && cargo tree --offline -q -p "$crate" -e normal --prefix none 2>/dev/null \
      | grep -oE '^(world-core|world-projection|world-agent|conversation|lives|world-pi-rpc) ' | sort -u | tr '\n' ' ' || true)
    if [[ -n "$linked" ]]; then
      echo "Boundary violation: $crate links $linked(asking a model must link no World code)"
      failed=1
    fi
  done
fi

# renderer, the protocol, the Systems, the desktop app) should not know which
# Worlds exist. Some still do, and scripts/pack-names-ratchet.txt records how
# many lines of each crate's src/ name a Pack, its places or its ids. The
# count may go down, never up: a change that adds one fails here, and a change
# that removes some should lower the recorded number.
pack_names='harbou?r|lighthouse|icebridge|maple street|\bmars\b|\bares\b|pocket[-_ ]universe|tiny[-_ ]society|\bquay\b'
ratchet="$ROOT/scripts/pack-names-ratchet.txt"
while read -r crate allowed; do
  [[ -z "$crate" || "$crate" == \#* ]] && continue
  if [[ ! -d "$ROOT/$crate/src" ]]; then
    echo "Pack-name ratchet names $crate, which has no src/; remove its line from $ratchet"
    failed=1
    continue
  fi
  found=$( { grep -RniE --include='*.rs' -- "$pack_names" "$ROOT/$crate/src" || true; } | wc -l | tr -d ' ')
  if (( found > allowed )); then
    echo "Pack-name ratchet: $crate names a Pack on $found lines, more than the $allowed recorded in $ratchet."
    echo "Put Pack knowledge in the Pack (or its data), not in shared code."
    failed=1
  elif (( found < allowed )); then
    echo "Pack-name ratchet: $crate is down to $found lines (from $allowed); lower its number in $ratchet."
  fi
done < "$ratchet"
for crate_dir in "$ROOT"/crates/* "$ROOT"/systems/* "$ROOT/apps/world-machine-desktop"; do
  crate="${crate_dir#"$ROOT"/}"
  [[ -d "$crate_dir/src" ]] || continue
  if ! grep -qE "^$crate " "$ratchet"; then
    found=$( { grep -RniE --include='*.rs' -- "$pack_names" "$crate_dir/src" || true; } | wc -l | tr -d ' ')
    if (( found > 0 )); then
      echo "Pack-name ratchet: new shared crate $crate names a Pack on $found lines; it should name none."
      failed=1
    fi
  fi
done

if [[ "$failed" -ne 0 ]]; then
  exit 1
fi

echo "Architecture boundary check passed."
