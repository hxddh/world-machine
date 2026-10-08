#!/usr/bin/env python3
"""Every workflow file parses, and still declares the triggers it needs.

A workflow whose YAML is malformed does not fail — it stops existing.
GitHub reports it as "does not have 'workflow_dispatch' trigger" and
otherwise says nothing, so a broken screenshot or release workflow reads
exactly like one nobody dispatched. That is how the app shipped four
releases with no text on screen: the one job that could have shown it
was assumed to be useless rather than checked.

It also holds the supply-chain rules of the v0.25 release review:

- every action is pinned by a full commit SHA, with the version it stands
  for in a comment (`uses: owner/repo@<40 hex> # v1.2.3`), so a moved tag
  cannot change what runs next to the signing certificate;
- every workflow declares `permissions:` at the top, and grants nothing
  there beyond read: a job that needs to write says so itself;
- every cargo build, test, run or lint is `--locked`, so CI builds exactly
  the dependencies in Cargo.lock;
- the nightly bars stay scheduled.
"""

from __future__ import annotations

import re
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
DISPATCHABLE = {"screenshots.yml", "release-package.yml", "nightly.yml", "release-shots.yml"}

# Workflows that are useless unless they run on their own.
SCHEDULED = {"nightly.yml"}

USES = re.compile(r"^\s*(?:-\s+)?uses:\s*(?P<ref>[^\s#]+)\s*(?:#\s*(?P<comment>\S.*))?$")
PINNED = re.compile(r"^[\w.-]+/[\w./-]+@[0-9a-f]{40}$")
CARGO = re.compile(r"\bcargo\s+(?:\+\S+\s+)?(?:build|test|run|clippy|check|bench|doc|install)\b")


def unpinned_actions(text: str) -> list[str]:
    """Every `uses:` that is not a full commit SHA with its version noted."""
    problems = []
    for number, line in enumerate(text.splitlines(), start=1):
        match = USES.match(line)
        if not match:
            continue
        ref = match.group("ref").strip("'\"")
        if ref.startswith("./") or ref.startswith("docker://"):
            continue
        if not PINNED.match(ref):
            problems.append(f"line {number}: {ref} is not pinned by a full commit SHA")
        elif not match.group("comment"):
            problems.append(
                f"line {number}: {ref} has no `# <version>` comment saying what the SHA is"
            )
    return problems


def grants_write(permissions) -> bool:
    if isinstance(permissions, str):
        return permissions == "write-all" or permissions.endswith("write")
    if isinstance(permissions, dict):
        return any(value == "write" for value in permissions.values())
    return False


def run_commands(document: dict):
    """Each shell command in every `run:` step, continuation lines joined."""
    for job_name, job in (document.get("jobs") or {}).items():
        for step in (job or {}).get("steps") or []:
            script = step.get("run") if isinstance(step, dict) else None
            if not isinstance(script, str):
                continue
            joined = re.sub(r"\\\n\s*", " ", script)
            for command in joined.splitlines():
                command = command.strip()
                if command and not command.startswith("#"):
                    yield job_name, command


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
        elif name in SCHEDULED and "schedule" not in triggers(document):
            problems.append(f"{path.relative_to(ROOT)}: has no schedule, so it never runs by itself")
        if not document.get("jobs"):
            problems.append(f"{path.relative_to(ROOT)}: declares no jobs")

        relative = path.relative_to(ROOT)
        for problem in unpinned_actions(path.read_text()):
            problems.append(f"{relative}: {problem}")
        if "permissions" not in document:
            problems.append(
                f"{relative}: declares no top-level permissions, so it gets the "
                "repository's default token; declare `permissions:` (read or none)"
            )
        elif grants_write(document["permissions"]):
            problems.append(
                f"{relative}: grants write at the top level; grant it in the job that needs it"
            )
        for job_name, command in run_commands(document):
            if CARGO.search(command) and not re.search(r"--(locked|frozen)\b", command):
                problems.append(f"{relative}: job {job_name}: not --locked: {command}")

    if problems:
        for problem in problems:
            print(problem, file=sys.stderr)
        return 1

    print(
        f"check-workflows: {len(files)} workflow file(s) parse and can run; "
        "actions are pinned, permissions scoped and cargo --locked"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
