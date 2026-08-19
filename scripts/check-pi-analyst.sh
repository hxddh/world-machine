#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
extension="$repo_root/integrations/pi/world-machine-analyst.mjs"
client="$repo_root/integrations/pi/world-machine-analyst-client.mjs"
rpc="$repo_root/integrations/pi/world-machine-analyst-rpc.mjs"
turn_host="$repo_root/integrations/pi/world-machine-analyst-turn-host.mjs"
tests_dir="$repo_root/integrations/pi/tests"
launcher="$repo_root/scripts/run-pi-analyst.sh"

node --check "$extension"
node --check "$client"
node --check "$rpc"
node --check "$turn_host"
node --test "$tests_dir"/*.test.mjs
bash -n "$launcher"

for required in \
  'pi.setActiveTools([])' \
  'pi.registerTool({' \
  'executionMode: "sequential"' \
  'descriptor.read_only !== true' \
  'parameters: descriptor.input_schema'
do
  if ! grep -Fq "$required" "$extension"; then
    echo "Pi analyst extension is missing required boundary: $required" >&2
    exit 1
  fi
done

for required in \
  'agent_settled' \
  'Pi analyst RPC session is single-flight' \
  'toolCallId' \
  'RESTRICTED_LAUNCHER' \
  'scripts/run-pi-analyst.sh'
do
  if ! grep -Fq "$required" "$rpc"; then
    echo "Pi analyst RPC session is missing required boundary: $required" >&2
    exit 1
  fi
done

if grep -Fq 'launcher =' "$rpc"; then
  echo "Pi analyst spawnRestricted must not accept a caller-supplied launcher" >&2
  exit 1
fi

for required in \
  'world-machine-analyst-turns' \
  'PiAnalystRpcSession.spawnRestricted' \
  'unknown analyst turn request field' \
  'tool_calls'
do
  if ! grep -Fq "$required" "$turn_host"; then
    echo "Analyst turn host is missing required boundary: $required" >&2
    exit 1
  fi
done

for forbidden in \
  'node:fs' \
  'node:http' \
  'node:https' \
  'fetch(' \
  'world-projection' \
  'world-core' \
  'WORLD_ACTION' \
  'agentruntime' \
  'typebox'
do
  if grep -Fqi "$forbidden" "$extension" "$client" "$rpc" "$turn_host"; then
    echo "Pi analyst integration contains forbidden authority/network/runtime token: $forbidden" >&2
    exit 1
  fi
done

for required_flag in \
  '--mode rpc' \
  '--no-builtin-tools' \
  '--no-extensions' \
  '--no-skills' \
  '--no-prompt-templates' \
  '--no-themes' \
  '--no-context-files'
do
  if ! grep -Fq -- "$required_flag" "$launcher"; then
    echo "Pi analyst launcher is missing restricted flag: $required_flag" >&2
    exit 1
  fi
done
