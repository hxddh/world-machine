#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
extension="$repo_root/integrations/pi/world-machine-analyst.mjs"
client="$repo_root/integrations/pi/world-machine-analyst-client.mjs"
tests="$repo_root/integrations/pi/tests/world-machine-analyst-client.test.mjs"
launcher="$repo_root/scripts/run-pi-analyst.sh"

node --check "$extension"
node --check "$client"
node --test "$tests"
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

for forbidden in \
  'node:fs' \
  'node:http' \
  'node:https' \
  'fetch(' \
  'world-projection' \
  'world-core' \
  'typebox'
do
  if grep -Fqi "$forbidden" "$extension" "$client"; then
    echo "Pi analyst extension contains forbidden authority/network/runtime token: $forbidden" >&2
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
