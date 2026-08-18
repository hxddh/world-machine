#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 2 ]]; then
  echo "usage: $0 <left.world> <right.world> [pi provider/model args...]" >&2
  exit 2
fi

left_archive=$1
right_archive=$2
shift 2

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
analyst_program=${WORLD_MACHINE_ANALYST_PROGRAM:-"$repo_root/target/debug/world-agent-tool-stdio"}
pi_program=${PI_PROGRAM:-pi}
extension="$repo_root/integrations/pi/world-machine-analyst.mjs"

if [[ ! -x "$analyst_program" ]]; then
  echo "World Machine analyst executable not found: $analyst_program" >&2
  echo "Build it with: cargo build -p world-agent-tool-stdio" >&2
  echo "Or set WORLD_MACHINE_ANALYST_PROGRAM to an installed executable." >&2
  exit 2
fi

export WORLD_MACHINE_ANALYST_PROGRAM="$analyst_program"
export WORLD_MACHINE_LEFT_ARCHIVE="$left_archive"
export WORLD_MACHINE_RIGHT_ARCHIVE="$right_archive"

exec "$pi_program" \
  --mode rpc \
  --no-session \
  --no-builtin-tools \
  --no-extensions \
  --extension "$extension" \
  --no-skills \
  --no-prompt-templates \
  --no-themes \
  --no-context-files \
  --system-prompt "You are a read-only World Machine analyst. Use only the World Machine analyst tools registered for this session. Treat their output as evidence. You cannot mutate the World or select different archive paths." \
  "$@"
