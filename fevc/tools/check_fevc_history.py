#!/usr/bin/env python3
"""Verify archived VCKSS evidence and the active changelog history."""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path, PurePosixPath

REPO_ROOT = Path(__file__).resolve().parents[2]
BASELINE_COMMIT = "a5805145b93961a98068de3452ff793013847942"
BASELINE_TREE = "7a3753d0a8550b3de7e8dee32e5671dc35b6d05d"
VCKSS_ARCHIVE_COMMIT = "fccf47a915e6a9d6ddd1af89bac770364b7fe561"
VCKSS_ARCHIVE_TREE = "aea812f447b29d4e2940b81608f03102990f523c"


def is_historical(relative: str) -> bool:
    path = PurePosixPath(relative)
    parts = path.parts
    name = path.name
    if relative.startswith(
        (
            ".ci/stata/results/",
            "docs/history/",
            "docs/migration/",
            "reviews/",
            "rust/progress/",
            "rust/qualification/evidence/",
        )
    ):
        return True
    if not relative.startswith("vckss/"):
        return False
    if relative.startswith(
        (
            "vckss/qualification/",
            "vckss/tests/equivalence/",
        )
    ):
        return True
    if relative in {
        "vckss/cmg/docs/UPSTREAM_SOURCE_MANIFEST.yaml",
        "vckss/tools/run_rename_equivalence.py",
        "vckss/tools/run_vckss_rename_equivalence.py",
    }:
        return True
    if "evidence" in parts or "reports" in parts:
        return True
    if relative.startswith("vckss/docs/"):
        return "_2026-" in name or name in {
            "MATLAB_KSS_VERSION_COMPARISON.md",
            "RUST_MATA_PARITY.md",
            "development_acceptance_v1.json",
            "rust_mata_parity.json",
        }
    if not relative.startswith("vckss/benchmarks/"):
        return False
    if "report" in parts or name.endswith(".pdf") or name == "SHA256SUMS":
        return True
    if name.endswith(".md") and name != "README.md":
        return name[:1].isupper() or "_2026-" in name
    if name.endswith(".json"):
        return any(
            token in name
            for token in (
                "2026-",
                "checkpoint",
                "decision",
                "failure",
                "headline",
                "profile",
                "receipt",
            )
        )
    return False


def baseline_tree() -> dict[str, str]:
    tree = subprocess.run(
        ["git", "-C", str(REPO_ROOT), "rev-parse", f"{BASELINE_COMMIT}^{{tree}}"],
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()
    if tree != BASELINE_TREE:
        raise ValueError(f"baseline tree changed: {tree} != {BASELINE_TREE}")
    completed = subprocess.run(
        ["git", "-C", str(REPO_ROOT), "ls-tree", "-r", "-z", BASELINE_COMMIT],
        check=True,
        stdout=subprocess.PIPE,
    )
    selected: dict[str, str] = {}
    for record in completed.stdout.split(b"\0"):
        if not record:
            continue
        metadata, raw_path = record.split(b"\t", 1)
        mode, kind, blob = metadata.decode("ascii").split()
        relative = raw_path.decode("utf-8")
        if mode == "100644" or mode == "100755":
            if kind == "blob" and is_historical(relative):
                selected[relative] = blob
    return selected


def archived_vckss_tree() -> str:
    return subprocess.run(
        [
            "git",
            "-C",
            str(REPO_ROOT),
            "rev-parse",
            f"{VCKSS_ARCHIVE_COMMIT}:vckss",
        ],
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()


def baseline_changelog_tail() -> bytes:
    completed = subprocess.run(
        ["git", "-C", str(REPO_ROOT), "show", f"{BASELINE_COMMIT}:vckss/CHANGELOG.md"],
        check=True,
        stdout=subprocess.PIPE,
    )
    prefix = b"# Changelog\n\n"
    if not completed.stdout.startswith(prefix):
        raise ValueError("baseline changelog has an unexpected heading")
    return completed.stdout[len(prefix) :]


def audit() -> tuple[int, list[str]]:
    expected = baseline_tree()
    errors: list[str] = []
    archive_tree = archived_vckss_tree()
    if archive_tree != VCKSS_ARCHIVE_TREE:
        errors.append(
            "archived VCKSS tree changed: "
            f"{archive_tree} != {VCKSS_ARCHIVE_TREE}"
        )
    live_tree = REPO_ROOT / "vckss"
    if live_tree.exists():
        errors.append("obsolete working-tree directory remains: vckss/")
    changelog = REPO_ROOT / "fevc/CHANGELOG.md"
    if not changelog.is_file():
        errors.append("active changelog is absent: fevc/CHANGELOG.md")
    elif not changelog.read_bytes().endswith(baseline_changelog_tail()):
        errors.append("pre-rename changelog body changed")
    return len(expected), errors


def main() -> int:
    try:
        count, errors = audit()
    except (OSError, ValueError, subprocess.CalledProcessError, UnicodeDecodeError) as exc:
        print(f"FEVC history audit could not run: {exc}", file=sys.stderr)
        return 2
    if errors:
        print("FEVC HISTORY AUDIT FAILED", file=sys.stderr)
        for error in errors:
            print(f"- {error}", file=sys.stderr)
        return 1
    print(
        "FEVC HISTORY AUDIT PASS: "
        f"{count} baseline files archived and changelog tail verified"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
