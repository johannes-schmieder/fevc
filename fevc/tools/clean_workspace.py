#!/usr/bin/env python3
"""Remove only disposable, ignored development artifacts.

The default is a dry run.  Pass ``--apply`` to remove the printed paths.
Tracked files and source-bound evidence trees are never eligible.
"""

from __future__ import annotations

import argparse
import json
import shutil
import subprocess
from dataclasses import asdict, dataclass
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
PROTECTED_PREFIXES = (
    ".git/",
    ".venv/",
    ".ci/stata/results/",
    "docs/history/",
    "reviews/",
    "rust/progress/",
    "rust/qualification/",
    "qualification/",
    "fevc/benchmarks/reports/",
    "fevc/cmg/benchmarks/reports/",
    "fevc/docs/",
    "fevc/qualification/",
    "vckss/qualification/",
)
PROTECTED_EXACT = {".git", ".venv"}
PROTECTED_PARTS = {"evidence"}
DISPOSABLE_DIRS = {
    ".ci/stata/run",
    ".hypothesis",
    ".pytest_cache",
    ".ruff_cache",
    "rust/fuzz/target",
    "rust/stata_backend/target",
    "rust/target",
}


@dataclass(frozen=True)
class CleanupReport:
    applied: bool
    paths: tuple[str, ...]
    files: int
    bytes: int


def _git_lines(*args: str) -> set[str]:
    result = subprocess.run(
        ["git", "-C", str(REPO_ROOT), *args, "-z"],
        check=True,
        stdout=subprocess.PIPE,
    )
    return {
        item.decode("utf-8")
        for item in result.stdout.split(b"\0")
        if item
    }


def _relative(path: Path) -> str:
    return path.relative_to(REPO_ROOT).as_posix()


def is_protected(relative: str) -> bool:
    normalized = relative.rstrip("/")
    parts = Path(normalized).parts
    return (
        normalized in PROTECTED_EXACT
        or any(part in PROTECTED_PARTS for part in parts)
        or any(
            normalized == prefix.rstrip("/") or normalized.startswith(prefix)
            for prefix in PROTECTED_PREFIXES
        )
    )


def _candidate_paths() -> set[Path]:
    candidates = {
        REPO_ROOT / relative
        for relative in DISPOSABLE_DIRS
        if (REPO_ROOT / relative).exists()
    }
    candidates.update(
        path for path in REPO_ROOT.rglob("__pycache__") if path.is_dir()
    )
    candidates.update(
        path for path in REPO_ROOT.rglob(".DS_Store") if path.is_file()
    )
    candidates.update(path for path in REPO_ROOT.rglob("*.log") if path.is_file())
    candidates.update(
        path for path in (REPO_ROOT / "fevc").glob("*.plugin") if path.is_file()
    )
    return candidates


def _file_totals(path: Path) -> tuple[int, int]:
    if path.is_file() or path.is_symlink():
        return 1, path.lstat().st_size
    files = 0
    size = 0
    for child in path.rglob("*"):
        if child.is_file() or child.is_symlink():
            files += 1
            size += child.lstat().st_size
    return files, size


def collect() -> tuple[Path, ...]:
    tracked = _git_lines("ls-files")
    selected: list[Path] = []
    for path in sorted(_candidate_paths()):
        relative = _relative(path)
        if is_protected(relative):
            continue
        if relative in tracked or any(
            tracked_path.startswith(f"{relative.rstrip('/')}/")
            for tracked_path in tracked
        ):
            continue
        ignored = subprocess.run(
            ["git", "-C", str(REPO_ROOT), "check-ignore", "-q", "--", relative],
            check=False,
        ).returncode == 0
        if not ignored:
            continue
        if any(parent in selected for parent in path.parents):
            continue
        selected.append(path)
    return tuple(selected)


def clean(*, apply: bool) -> CleanupReport:
    selected = collect()
    file_count = 0
    byte_count = 0
    for path in selected:
        files, size = _file_totals(path)
        file_count += files
        byte_count += size
    if apply:
        for path in selected:
            if path.is_dir() and not path.is_symlink():
                shutil.rmtree(path)
            else:
                path.unlink(missing_ok=True)
    return CleanupReport(
        applied=apply,
        paths=tuple(_relative(path) for path in selected),
        files=file_count,
        bytes=byte_count,
    )


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--dry-run", action="store_true", help="print only (default)")
    mode.add_argument("--apply", action="store_true", help="remove eligible artifacts")
    parser.add_argument("--json", action="store_true", help="emit machine-readable output")
    args = parser.parse_args()
    report = clean(apply=args.apply)
    if args.json:
        print(json.dumps(asdict(report), indent=2, sort_keys=True))
    else:
        action = "removed" if report.applied else "would remove"
        for relative in report.paths:
            print(relative)
        print(f"{action} {report.files} file(s), {report.bytes} byte(s)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
