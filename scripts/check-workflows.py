#!/usr/bin/env python3
"""Every workflow file parses, and still declares the triggers it needs.

A workflow whose YAML is malformed does not fail — it stops existing.
GitHub reports it as "does not have 'workflow_dispatch' trigger" and
otherwise says nothing, so a broken screenshot or release workflow reads
exactly like one nobody dispatched. That is how the app shipped four
releases with no text on screen: the one job that could have shown it
was assumed to be useless rather than checked.
"""

from __future__ import annotations

import sys
from pathlib import Path

try:
    import yaml
except ModuleNotFoundError:  # pragma: no cover - depends on the runner image
    print(
        "check-workflows: PyYAML is not installed; run "
        "`python3 -m pip install pyyaml`",
        file=sys.stderr,
    )
    raise SystemExit(1)

ROOT = Path(__file__).resolve().parent.parent
WORKFLOWS = ROOT / ".github/workflows"

# Workflows that are useless unless they can be started by hand.
DISPATCHABLE = {"screenshots.yml", "release-package.yml"}


def triggers(document: dict) -> dict:
    # PyYAML reads a bare `on:` key as the boolean True.
    section = document.get("on", document.get(True))
    if isinstance(section, str):
        return {section: None}
    if isinstance(section, list):
        return {name: None for name in section}
    return section or {}


def main() -> int:
    files = sorted(WORKFLOWS.glob("*.yml")) + sorted(WORKFLOWS.glob("*.yaml"))
    if not files:
        print("check-workflows: no workflow files found", file=sys.stderr)
        return 1

    problems: list[str] = []
    for path in files:
        name = path.name
        try:
            document = yaml.safe_load(path.read_text())
        except yaml.YAMLError as error:
            problems.append(f"{path.relative_to(ROOT)}: does not parse: {error}")
            continue
        if not isinstance(document, dict):
            problems.append(f"{path.relative_to(ROOT)}: is not a mapping")
            continue
        if not triggers(document):
            problems.append(
                f"{path.relative_to(ROOT)}: declares no triggers, so it can never run"
            )
        elif name in DISPATCHABLE and "workflow_dispatch" not in triggers(document):
            problems.append(
                f"{path.relative_to(ROOT)}: has no workflow_dispatch trigger, "
                "so it cannot be started by hand"
            )
        if not document.get("jobs"):
            problems.append(f"{path.relative_to(ROOT)}: declares no jobs")

    if problems:
        for problem in problems:
            print(problem, file=sys.stderr)
        return 1

    print(f"check-workflows: {len(files)} workflow file(s) parse and can run")
    return 0


if __name__ == "__main__":
    sys.exit(main())
