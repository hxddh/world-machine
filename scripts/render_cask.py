#!/usr/bin/env python3
"""Render a Homebrew cask for a published pre-alpha package.

Reads the ``release-manifest.json`` that ``package-release.sh`` writes and
prints a cask whose ``version``, ``sha256``, and download URL match that exact
release. The release workflow attaches the result to the GitHub Release; the
``hxddh/homebrew-tap`` repository copies it into ``Casks/world-machine.rb``.

The cask refuses a manifest that is notarized or not ad-hoc signed, for the
same reason the release notes do: this pipeline only describes the unsigned
pre-alpha channel, and the caveats text below must stay true.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

REPOSITORY = "hxddh/world-machine"
TAP = "hxddh/tap"


def render_cask(manifest: dict) -> str:
    if manifest.get("signing") != "ad-hoc":
        raise ValueError(f"expected ad-hoc signing, found {manifest.get('signing')!r}")
    if manifest.get("notarized") is not False:
        raise ValueError("expected notarized == false for the pre-alpha channel")

    tag = manifest["tag"]
    if not tag.startswith("v"):
        raise ValueError(f"tag must start with 'v': {tag!r}")
    version = tag[1:]
    artifact = manifest["artifact"]
    if version not in artifact:
        raise ValueError(f"artifact {artifact!r} does not embed version {version!r}")
    artifact_template = artifact.replace(version, "#{version}")
    sha256 = manifest["sha256"]
    if len(sha256) != 64:
        raise ValueError("sha256 must be 64 hex characters")

    return f'''cask "world-machine" do
  version "{version}"
  sha256 "{sha256}"

  url "https://github.com/{REPOSITORY}/releases/download/v#{{version}}/{artifact_template}"
  name "World Machine"
  desc "Persistent worlds that remember, evolve, and branch"
  homepage "https://github.com/{REPOSITORY}"

  depends_on macos: ">= :sonoma"

  app "World Machine.app"

  caveats <<~EOS
    World Machine {version} is a pre-alpha build: ad-hoc signed, not notarized by Apple.
    macOS blocks the first launch unless the quarantine flag is skipped at install time:

      brew install --cask --no-quarantine {TAP}/world-machine

    If it was installed without that flag, follow the first-launch steps in
    https://github.com/{REPOSITORY}/blob/{tag}/docs/INSTALL.md
  EOS

  zap trash: [
    "~/Library/Application Support/World Machine",
    "~/Library/Logs/World Machine",
  ]
end
'''


def main(argv: list[str]) -> int:
    if len(argv) != 2:
        print("usage: render_cask.py <release-manifest.json>", file=sys.stderr)
        return 2
    manifest = json.loads(Path(argv[1]).read_text())
    sys.stdout.write(render_cask(manifest))
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
