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
- the nightly bars stay scheduled;
- every test name a workflow filters on or skips (`cargo test -- NAME`,
  `--skip NAME`, `--test NAME`) still names a test, so a renamed or deleted
  bar fails here instead of turning its step into one that runs nothing
  and passes (as `a_three_year_world_never_waits_for_painting` did);
- CI runs on pushes to main and on pull requests, not both for one PR;
- a release publishes nothing unless the nightly passed on its commit.
"""

from __future__ import annotations

import re
import shlex
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


def release_check_problems() -> list[str]:
    """The release shots run for every release: release-package.yml calls
    release-shots.yml as a job (a tag the release workflow creates starts no
    other workflow), and publishes nothing unless it passed."""
    path = WORKFLOWS / "release-package.yml"
    shots = WORKFLOWS / "release-shots.yml"
    if not path.exists() or not shots.exists():
        return []
    try:
        document = yaml.safe_load(path.read_text())
        called = yaml.safe_load(shots.read_text())
    except yaml.YAMLError:
        return []  # reported above
    relative = path.relative_to(ROOT)
    problems = []
    if "workflow_call" not in triggers(called or {}):
        problems.append(f"{shots.relative_to(ROOT)}: has no workflow_call trigger, so no release can run it")
    jobs = (document or {}).get("jobs") or {}
    callers = [
        name
        for name, job in jobs.items()
        if isinstance(job, dict) and job.get("uses") == "./.github/workflows/release-shots.yml"
    ]
    if not callers:
        problems.append(f"{relative}: no job calls ./.github/workflows/release-shots.yml")
        return problems
    publish = jobs.get("publish") or {}
    needs = publish.get("needs") or []
    needs = [needs] if isinstance(needs, str) else needs
    if not any(caller in needs for caller in callers):
        problems.append(f"{relative}: publish does not wait for the release shots ({callers[0]})")
    return problems


SOURCE_DIRS = ("crates", "systems", "worlds", "apps")
TEST_ATTRIBUTE = re.compile(r"^#\[(?:[\w:]+::)?test\b")
FN = re.compile(r"^(?:pub(?:\([\w:]+\))?\s+)?(?:async\s+)?fn\s+(\w+)")
# libtest options that take the next word as their value.
LIBTEST_VALUED = {"--skip", "--test-threads", "--format", "--color", "--logfile", "-Z", "--shuffle-seed"}


def test_names() -> set[str]:
    """The name of every `#[test]` (or `#[gpui::test]`) function."""
    names = set()
    for directory in SOURCE_DIRS:
        for path in (ROOT / directory).rglob("*.rs"):
            if "target" in path.relative_to(ROOT).parts:
                continue
            pending = False
            for line in path.read_text(errors="replace").splitlines():
                line = line.strip()
                if TEST_ATTRIBUTE.match(line):
                    pending = True
                    continue
                if not pending:
                    continue
                if line.startswith(("#[", "//")) or not line:
                    continue
                match = FN.match(line)
                if match:
                    names.add(match.group(1))
                pending = False
    return names


def test_targets() -> set[str]:
    """Every integration test target: `tests/<name>.rs` and `tests/<name>/main.rs`."""
    targets = set()
    for directory in SOURCE_DIRS:
        for path in (ROOT / directory).rglob("tests/*.rs"):
            targets.add(path.stem)
        for path in (ROOT / directory).rglob("tests/*/main.rs"):
            targets.add(path.parent.name)
    return targets


def cargo_test_filters(command: str, env: dict) -> list[tuple[str, str]]:
    """(kind, name) for each test filter, skip and `--test` target in a
    `cargo test` command, with `$VAR`s from the workflow's env expanded."""
    if not re.search(r"\bcargo\s+test\b", command):
        return []
    expanded = re.sub(r"\$\{?(\w+)\}?", lambda m: str(env.get(m.group(1), m.group(0))), command)
    try:
        words = shlex.split(expanded)
    except ValueError:
        return []
    found = []
    if "--" in words:
        split = words.index("--")
        cargo, libtest = words[:split], words[split + 1 :]
    else:
        cargo, libtest = words, []
    for index, word in enumerate(cargo):
        if word == "--test" and index + 1 < len(cargo):
            found.append(("--test", cargo[index + 1]))
        elif word.startswith("--test="):
            found.append(("--test", word.split("=", 1)[1]))
    skip_value, previous = False, ""
    for word in libtest:
        if skip_value:
            skip_value = False
            if previous == "--skip":
                found.append(("--skip", word))
            continue
        if word in LIBTEST_VALUED:
            skip_value, previous = True, word
            continue
        if word.startswith("--skip="):
            found.append(("--skip", word.split("=", 1)[1]))
        elif not word.startswith("-"):
            found.append(("filter", word))
    return found


def unknown_test_names() -> list[str]:
    """Each filter, skip or `--test` target in a workflow that names nothing."""
    names = test_names()
    targets = test_targets()
    problems = []
    for path in sorted(WORKFLOWS.glob("*.yml")):
        try:
            document = yaml.safe_load(path.read_text())
        except yaml.YAMLError:
            continue
        if not isinstance(document, dict):
            continue
        top_env = document.get("env") or {}
        for job_name, job in (document.get("jobs") or {}).items():
            env = {**top_env, **((job or {}).get("env") or {})}
            for step in (job or {}).get("steps") or []:
                script = step.get("run") if isinstance(step, dict) else None
                if not isinstance(script, str):
                    continue
                step_env = {**env, **(step.get("env") or {})}
                joined = re.sub(r"\\\n\s*", " ", script)
                for command in joined.splitlines():
                    for kind, name in cargo_test_filters(command.strip(), step_env):
                        if "$" in name:
                            continue  # a value only the runner knows
                        if kind == "--test":
                            if name not in targets:
                                problems.append(
                                    f"{path.relative_to(ROOT)}: job {job_name}: "
                                    f"--test {name} names no tests/{name}.rs"
                                )
                        elif not any(name.rsplit("::", 1)[-1] in test for test in names):
                            problems.append(
                                f"{path.relative_to(ROOT)}: job {job_name}: {kind} {name} "
                                "matches no test; a renamed or deleted test leaves this "
                                "step running nothing"
                            )
    return problems


def shard_problems() -> list[str]:
    """Every test runs somewhere. CI's test shards are a list of packages
    and one shard of the whole workspace less some: each package that shard
    leaves out (but the desktop app, tested on macOS and Windows) is tested
    by another shard. And whatever CI skips as too slow (`SLOW`) the
    nightly does not skip."""
    path = WORKFLOWS / "ci.yml"
    nightly = WORKFLOWS / "nightly.yml"
    if not path.exists():
        return []
    try:
        document = yaml.safe_load(path.read_text()) or {}
        night = yaml.safe_load(nightly.read_text()) or {} if nightly.exists() else {}
    except yaml.YAMLError:
        return []
    relative = path.relative_to(ROOT)
    problems = []
    test = (document.get("jobs") or {}).get("test") or {}
    shards = (((test.get("strategy") or {}).get("matrix") or {}).get("shard")) or []
    if not shards:
        return [f"{relative}: the test job has no shards"]
    tested, excluded = set(), set()
    for shard in shards:
        words = str(shard.get("packages", "")).split()
        for flag, value in zip(words, words[1:]):
            if flag == "-p":
                tested.add(value)
            elif flag == "--exclude":
                excluded.add(value)
    for package in sorted(excluded - tested - {"world-machine-desktop"}):
        problems.append(f"{relative}: no test shard tests {package}, which the workspace shard leaves out")
    slow = shlex.split(str((document.get("env") or {}).get("SLOW", "")))
    nightly_skips = " ".join(str(value) for value in (night.get("env") or {}).values())
    for flag, name in zip(slow, slow[1:]):
        if flag == "--skip" and name.rsplit("::", 1)[-1] in nightly_skips:
            problems.append(
                f"{relative}: {name} is skipped as slow here and skipped by the nightly too, "
                "so it runs nowhere"
            )
    return problems


def trigger_problems() -> list[str]:
    """CI runs once per change: on pushes to main, and on pull requests.
    Pushes to every branch as well start each PR's checks twice."""
    path = WORKFLOWS / "ci.yml"
    if not path.exists():
        return []
    try:
        document = yaml.safe_load(path.read_text()) or {}
    except yaml.YAMLError:
        return []
    on = triggers(document)
    problems = []
    push = on.get("push") if "push" in on else None
    if "push" in on and (not isinstance(push, dict) or push.get("branches") != ["main"]):
        problems.append(
            f"{path.relative_to(ROOT)}: push runs on more than main, so every PR runs twice; "
            "use `push: branches: [main]` and `pull_request`"
        )
    if "pull_request" not in on:
        problems.append(f"{path.relative_to(ROOT)}: has no pull_request trigger")
    if not (document.get("concurrency") or {}).get("cancel-in-progress"):
        problems.append(f"{path.relative_to(ROOT)}: does not cancel superseded runs")
    return problems


def nightly_gate_problems() -> list[str]:
    """release-package.yml publishes nothing unless its gate on the nightly
    passed, and the nightly reports a red scheduled run."""
    path = WORKFLOWS / "release-package.yml"
    nightly = WORKFLOWS / "nightly.yml"
    if not path.exists() or not nightly.exists():
        return []
    try:
        document = yaml.safe_load(path.read_text()) or {}
        night = yaml.safe_load(nightly.read_text()) or {}
    except yaml.YAMLError:
        return []
    problems = []
    jobs = document.get("jobs") or {}
    gate = jobs.get("nightly-gate")
    if not isinstance(gate, dict):
        problems.append(f"{path.relative_to(ROOT)}: no nightly-gate job checks the nightly")
    else:
        for name in ("macos-release", "publish"):
            needs = (jobs.get(name) or {}).get("needs") or []
            needs = [needs] if isinstance(needs, str) else needs
            if "nightly-gate" not in needs:
                problems.append(f"{path.relative_to(ROOT)}: {name} does not wait for nightly-gate")
    report = (night.get("jobs") or {}).get("report-red-night")
    if not isinstance(report, dict) or (report.get("permissions") or {}).get("issues") != "write":
        problems.append(
            f"{nightly.relative_to(ROOT)}: no report-red-night job (with issues: write) "
            "opens an issue when a scheduled night is red"
        )
    return problems


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

    problems.extend(release_check_problems())
    problems.extend(unknown_test_names())
    problems.extend(shard_problems())
    problems.extend(trigger_problems())
    problems.extend(nightly_gate_problems())

    if problems:
        for problem in problems:
            print(problem, file=sys.stderr)
        return 1

    print(
        f"check-workflows: {len(files)} workflow file(s) parse and can run; "
        "actions are pinned, permissions scoped, cargo --locked, every test "
        "filter names a test, and a release waits for a green nightly"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
