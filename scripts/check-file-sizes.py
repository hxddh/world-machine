#!/usr/bin/env python3
"""No Rust file grows past its recorded size, and no new one passes 3,000 lines.

The review of v0.30 (docs/REVIEW_v0.30.md, P0) asks for the big files to be
split, and for nothing to grow while they are: `scripts/file-size-ratchet.txt`
records the line count of every tracked `.rs` file that was already over the
limit. A recorded file may shrink, never grow; any other file may grow up to
the limit and no further. When a recorded file shrinks, lower its number
(`--update` does that, and never raises one); when it is split below the
limit, its line may go.

Usage:
  scripts/check-file-sizes.py            check
  scripts/check-file-sizes.py --update   lower the recorded counts that went down
                                         and drop lines for files now under the
                                         limit or gone (never raises a count)
"""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
RATCHET = ROOT / "scripts/file-size-ratchet.txt"
LIMIT = 3000


def tracked_rust_files() -> list[str]:
    output = subprocess.run(
        ["git", "ls-files", "-z", "--", "*.rs"],
        cwd=ROOT,
        check=True,
        capture_output=True,
    ).stdout
    return sorted(name for name in output.decode().split("\0") if name)


def line_count(path: Path) -> int:
    with path.open("rb") as handle:
        return sum(1 for _ in handle)


def read_ratchet() -> tuple[list[str], dict[str, int]]:
    header: list[str] = []
    recorded: dict[str, int] = {}
    for number, line in enumerate(RATCHET.read_text().splitlines(), start=1):
        stripped = line.strip()
        if not stripped or stripped.startswith("#"):
            if not recorded:
                header.append(line)
            continue
        try:
            path, count = stripped.rsplit(maxsplit=1)
            recorded[path] = int(count)
        except ValueError:
            raise SystemExit(f"{RATCHET.relative_to(ROOT)}:{number}: expected `<path> <lines>`")
    return header, recorded


def main() -> int:
    update = "--update" in sys.argv[1:]
    header, recorded = read_ratchet()
    sizes = {}
    for name in tracked_rust_files():
        path = ROOT / name
        if path.is_file():
            sizes[name] = line_count(path)

    problems: list[str] = []
    notes: list[str] = []
    lowered = dict(recorded)
    for name, count in sizes.items():
        allowed = recorded.get(name)
        if allowed is None:
            if count > LIMIT:
                problems.append(
                    f"{name}: {count} lines, over the {LIMIT}-line limit for a file "
                    "not in the ratchet; split it"
                )
        elif count > allowed:
            problems.append(
                f"{name}: {count} lines, more than the {allowed} recorded; "
                "it may only shrink until it is split"
            )
        elif count < allowed:
            if count <= LIMIT:
                notes.append(f"{name}: down to {count} lines, under the limit; drop its line")
                lowered.pop(name)
            else:
                notes.append(f"{name}: down to {count} lines (from {allowed}); lower its number")
                lowered[name] = count
    for name in recorded:
        if name not in sizes:
            notes.append(f"{name}: recorded but no longer tracked; drop its line")
            lowered.pop(name, None)

    if update and lowered != recorded:
        body = "\n".join(f"{name} {count}" for name, count in sorted(lowered.items()))
        RATCHET.write_text("\n".join(header) + "\n" + body + "\n")
        print(f"check-file-sizes: lowered {RATCHET.relative_to(ROOT)}")
        notes = []

    for note in notes:
        print(f"check-file-sizes: {note} in {RATCHET.relative_to(ROOT)}")
    if problems:
        for problem in problems:
            print(f"check-file-sizes: {problem}", file=sys.stderr)
        return 1
    print(
        f"check-file-sizes: {len(sizes)} Rust files; none passes {LIMIT} lines "
        f"but the {len(lowered)} recorded, and none of those grew"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
