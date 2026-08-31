#!/usr/bin/env python3
"""Validate the active VCkss GPL package boundary without rewriting files."""

from __future__ import annotations

import hashlib
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
CANONICAL = ROOT / "LICENSES" / "GPL-3.0-only.txt"
REQUIRED_IDENTICAL = (ROOT / "LICENSE", ROOT / "fevc" / "LICENSE")
REQUIRED_TEXT = {
    ROOT / "CODE_LICENSE.md": (
        "GPL-3.0-only",
        "completed on 2026-08-29",
        "No public release or",
    ),
    ROOT / "THIRD_PARTY_NOTICES.md": (
        "rust/vendor/cmg/LICENSE",
        "No MATLAB source",
    ),
    ROOT / "fevc" / "THIRD_PARTY_NOTICES.txt": (
        "GPL-3.0-only",
        "rust/vendor/cmg/",
    ),
    ROOT / "rust" / "vendor" / "cmg" / "VENDOR.md": ("GPL",),
    ROOT / "rust" / "vendor" / "cmg" / "UPSTREAM_MANIFEST.sha256": (),
    ROOT / "REUSE.toml": (
        'SPDX-License-Identifier = "GPL-3.0-only"',
        'precedence = "aggregate"',
    ),
}


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


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

    cargo_files = tuple((ROOT / "rust").glob("**/Cargo.toml"))
    for path in cargo_files:
        if "vendor/cmg" in path.as_posix():
            continue
        text = path.read_text(encoding="utf-8")
        has_license = (
            'license = "GPL-3.0-only"' in text
            or "license.workspace = true" in text
        )
        if "[package]" in text and not has_license:
            errors.append(f"{path}: package manifest lacks GPL-3.0-only")

    if errors:
        print("LICENSE_AUDIT=FAIL")
        for error in errors:
            print(error)
        return 1
    print("LICENSE_AUDIT=PASS")
    print(f"GPL_SHA256={digest(CANONICAL)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
