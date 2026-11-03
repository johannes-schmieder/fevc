#!/usr/bin/env python3
"""Validate the active FEVC MIT package boundary without rewriting files."""

from __future__ import annotations

import hashlib
from pathlib import Path
import subprocess


ROOT = Path(__file__).resolve().parents[2]
CANONICAL = ROOT / "LICENSES" / "MIT.txt"
REQUIRED_IDENTICAL = (ROOT / "LICENSE", ROOT / "fevc" / "LICENSE")
REQUIRED_TEXT = {
    ROOT / "CODE_LICENSE.md": (
        "MIT",
        "Ioannis Koutis",
        "source,\nnotices, and validation",
    ),
    ROOT / "THIRD_PARTY_NOTICES.md": (
        "rust/vendor/cmg/LICENSE",
        "No MATLAB source",
    ),
    ROOT / "fevc" / "THIRD_PARTY_NOTICES.txt": (
        "MIT",
        "rust/vendor/cmg/",
    ),
    ROOT / "rust" / "vendor" / "cmg" / "VENDOR.md": ("License: `MIT`", "Ioannis Koutis"),
    ROOT / "rust" / "vendor" / "cmg" / "UPSTREAM_MANIFEST.sha256": (),
    ROOT / "REUSE.toml": (
        'SPDX-License-Identifier = "MIT"',
        'precedence = "aggregate"',
    ),
}


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def cargo_manifests(root: Path) -> tuple[Path, ...]:
    """Audit source files, not ignored local dependency/experiment snapshots."""
    if not (root / ".git").exists():
        # Source distributions have no Git metadata and contain only payload.
        return tuple(sorted((root / "rust").glob("**/Cargo.toml")))
    completed = subprocess.run(
        ["git", "-C", str(root), "ls-files", "-z", "--cached", "--others",
         "--exclude-standard", "--", "rust"],
        check=True,
        stdout=subprocess.PIPE,
    )
    paths = {Path(item.decode("utf-8")) for item in completed.stdout.split(b"\0") if item}
    return tuple(sorted(
        root / path for path in paths
        if path.name == "Cargo.toml" and (root / path).is_file()
    ))


def main() -> int:
    errors: list[str] = []
    if not CANONICAL.is_file():
        errors.append(f"missing canonical license: {CANONICAL}")
    else:
        expected = digest(CANONICAL)
        for path in REQUIRED_IDENTICAL:
            if not path.is_file():
                errors.append(f"missing license copy: {path}")
            elif digest(path) != expected:
                errors.append(f"license copy differs from canonical text: {path}")

    for path, needles in REQUIRED_TEXT.items():
        if not path.is_file():
            errors.append(f"missing required notice/provenance file: {path}")
            continue
        text = path.read_text(encoding="utf-8", errors="strict")
        for needle in needles:
            if needle not in text:
                errors.append(f"{path}: missing required text {needle!r}")

    package = (ROOT / "fevc" / "fevc.pkg").read_text(encoding="utf-8")
    for entry in ("f LICENSE", "f THIRD_PARTY_NOTICES.txt"):
        if entry not in package.splitlines():
            errors.append(f"fevc.pkg: missing package entry {entry!r}")

    for path in cargo_manifests(ROOT):
        text = path.read_text(encoding="utf-8")
        has_license = (
            'license = "MIT"' in text
            or "license.workspace = true" in text
        )
        if "[package]" in text and not has_license:
            errors.append(f"{path}: package manifest lacks MIT")

    if errors:
        print("LICENSE_AUDIT=FAIL")
        for error in errors:
            print(error)
        return 1
    print("LICENSE_AUDIT=PASS")
    print(f"MIT_SHA256={digest(CANONICAL)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
