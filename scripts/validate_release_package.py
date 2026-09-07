#!/usr/bin/env python3
from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from pathlib import Path


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Validate a World Machine release package.")
    parser.add_argument("package_dir", type=Path)
    parser.add_argument(
        "--publishing",
        action="store_true",
        help="Require a publishable tag: a stable v<version> or pre.1 or newer.",
    )
    return parser.parse_args()


def validate(package_dir: Path, *, publishing: bool) -> dict:
    manifest_path = package_dir / "release-manifest.json"
    manifest = json.loads(manifest_path.read_text())

    required = {
        "schema_version",
        "tag",
        "app_version",
        "bundle_identifier",
        "commit",
        "architectures",
        "signing",
        "notarized",
        "artifact",
        "sha256",
        "included_packs",
    }
    missing = sorted(required.difference(manifest))
    if missing:
        raise ValueError(f"release manifest is missing fields: {', '.join(missing)}")
    if manifest["schema_version"] != 1:
        raise ValueError(f"unsupported release manifest schema: {manifest['schema_version']}")
    signing = manifest["signing"]
    notarized = manifest["notarized"]
    if signing == "ad-hoc":
        if notarized is not False:
            raise ValueError("an ad-hoc signed package cannot be notarized")
    elif signing == "developer-id":
        if notarized is not True:
            raise ValueError("a Developer ID package must be notarized before it is published")
    else:
        raise ValueError(f"unknown signing mode {signing!r}; expected ad-hoc or developer-id")

    version = str(manifest["app_version"])
    tag = str(manifest["tag"])
    match = re.fullmatch(rf"v{re.escape(version)}(?:-pre\.(\d+))?", tag)
    if not match:
        raise ValueError(
            f"tag {tag!r} must match app version {version!r} as v{version} or v{version}-pre.N"
        )
    if publishing and match.group(1) is not None and int(match.group(1)) < 1:
        raise ValueError("published pre-release numbers start at 1; pre.0 is CI-only")

    artifact = package_dir / str(manifest["artifact"])
    checksum = package_dir / f"{artifact.name}.sha256"
    for path in (artifact, checksum, manifest_path):
        if not path.is_file() or path.stat().st_size == 0:
            raise ValueError(f"release asset is missing or empty: {path}")

    actual_sha = hashlib.sha256(artifact.read_bytes()).hexdigest()
    expected_sha = str(manifest["sha256"])
    if actual_sha != expected_sha:
        raise ValueError(
            f"artifact SHA-256 mismatch: manifest={expected_sha} actual={actual_sha}"
        )
    if checksum.read_text().strip() != f"{expected_sha}  {artifact.name}":
        raise ValueError(f"checksum file does not match manifest: {checksum}")

    if "dmg" in manifest:
        dmg = package_dir / str(manifest["dmg"])
        dmg_checksum = package_dir / f"{dmg.name}.sha256"
        for path in (dmg, dmg_checksum):
            if not path.is_file() or path.stat().st_size == 0:
                raise ValueError(f"release asset is missing or empty: {path}")
        expected_dmg_sha = str(manifest["dmg_sha256"])
        actual_dmg_sha = hashlib.sha256(dmg.read_bytes()).hexdigest()
        if actual_dmg_sha != expected_dmg_sha:
            raise ValueError(
                f"dmg SHA-256 mismatch: manifest={expected_dmg_sha} actual={actual_dmg_sha}"
            )
        if dmg_checksum.read_text().strip() != f"{expected_dmg_sha}  {dmg.name}":
            raise ValueError(f"dmg checksum file does not match manifest: {dmg_checksum}")

    architectures = manifest["architectures"]
    if not isinstance(architectures, list) or not architectures:
        raise ValueError("release manifest must contain at least one architecture")
    packs = manifest["included_packs"]
    if not isinstance(packs, list) or not packs:
        raise ValueError("release manifest must contain included World Packs")

    return manifest


def main() -> int:
    args = parse_args()
    manifest = validate(args.package_dir, publishing=args.publishing)
    print(
        json.dumps(
            {
                "tag": manifest["tag"],
                "app_version": manifest["app_version"],
                "architectures": manifest["architectures"],
                "artifact": manifest["artifact"],
                "sha256": manifest["sha256"],
                "signing": manifest["signing"],
                "notarized": manifest["notarized"],
                "included_packs": manifest["included_packs"],
            },
            indent=2,
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:
        print(f"validate_release_package.py: {error}", file=sys.stderr)
        raise SystemExit(1)
