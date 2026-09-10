#!/usr/bin/env python3
"""Remove only disposable, ignored development artifacts.

The default is a dry run.  Pass ``--apply`` to remove the printed paths.
Tracked files and source-bound evidence trees are never eligible.
"""

from __future__ import annotations

import argparse
import fnmatch
import json
import os
import shutil
import subprocess
from dataclasses import asdict, dataclass
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
PROTECTED_PREFIXES = (
    ".git/",
    ".venv/",
    ".local/",
    "output/",
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
)
PROTECTED_EXACT = {".git", ".venv", ".local", "output"}
PROTECTED_PARTS = {"evidence"}
DISPOSABLE_DIRS = {
    ".ci/stata/run",
    ".hypothesis",
    ".pytest_cache",
    ".ruff_cache",
}
BUILD_CACHE_DIRS = {"rust/fuzz/target", "rust/stata_backend/target", "rust/target"}
DISPOSABLE_DIR_NAMES = {
    ".ipynb_checkpoints",
    ".mypy_cache",
    ".nox",
    ".tox",
    "build",
    "dist",
}
DISPOSABLE_DIR_GLOBS = ("*.egg-info",)
DISPOSABLE_FILE_GLOBS = (
    ".coverage*",
    "coverage.xml",
    "*.asv",
    "*.aux",
    "*.bbl",
    "*.bcf",
    "*.blg",
    "*.fdb_latexmk",
    "*.fls",
    "*.lof",
    "*.log",
    "*.lot",
    "*.out",
    "*.run.xml",
    "*.smcl",
    "*.synctex.gz",
    "*.toc",
    "*.xdv",
)


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
        # Installed plugins may be the exact binaries named by a qualification
        # receipt. Ignored does not mean disposable.
        or (len(parts) == 2 and parts[0] == "fevc" and parts[1].endswith(".plugin"))
        or any(part in PROTECTED_PARTS for part in parts)
        or any(
            normalized == prefix.rstrip("/") or normalized.startswith(prefix)
            for prefix in PROTECTED_PREFIXES
        )
    )


def _candidate_paths(*, build_caches: bool = False) -> set[Path]:
    candidates: set[Path] = set()
    # Pruning avoids traversing local evidence, toolchains and build caches.
    # These can be much larger than the source tree.
    for directory, dirs, files in os.walk(REPO_ROOT, followlinks=False):
        parent = Path(directory)
        for name in dirs[:]:
            path = parent / name
            relative = _relative(path)
            if (path.is_symlink() or is_protected(relative)
                    or (relative in BUILD_CACHE_DIRS and not build_caches)):
                dirs.remove(name)
                continue
            if (relative in DISPOSABLE_DIRS or relative in BUILD_CACHE_DIRS
                    or name == "__pycache__" or name in DISPOSABLE_DIR_NAMES
                    or any(fnmatch.fnmatchcase(name, pattern)
                           for pattern in DISPOSABLE_DIR_GLOBS)):
                candidates.add(path)
        for name in files:
            if any(fnmatch.fnmatchcase(name, pattern)
                   for pattern in (".DS_Store", *DISPOSABLE_FILE_GLOBS)):
                candidates.add(parent / name)
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


def collect(*, build_caches: bool = False) -> tuple[Path, ...]:
    tracked = _git_lines("ls-files")
    retained = tracked | _git_lines("ls-files", "--others", "--exclude-standard")
    selected: list[Path] = []
    for path in sorted(_candidate_paths(build_caches=build_caches)):
        relative = _relative(path)
        if is_protected(relative):
            continue
        if not build_caches and any(relative == name or relative.startswith(name + "/")
                                    for name in BUILD_CACHE_DIRS):
            continue
        # Do not recurse through a directory symlink into another workspace.
        if path.is_symlink() or any(parent.is_symlink() for parent in path.parents
                                    if parent != REPO_ROOT):
            continue
        # A disposable-looking ancestor must not swallow protected evidence.
        if path.is_dir() and any(is_protected(_relative(child))
                                 for child in path.rglob("*")):
            continue
        if relative in retained or any(
            retained_path.startswith(f"{relative.rstrip('/')}/")
            for retained_path in retained
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


def clean(*, apply: bool, build_caches: bool = False) -> CleanupReport:
    selected = collect(build_caches=build_caches)
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
    parser.add_argument("--build-caches", action="store_true",
                        help="also remove ignored Rust build caches (requires rebuilding)")
    args = parser.parse_args()
    report = clean(apply=args.apply, build_caches=args.build_caches)
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
