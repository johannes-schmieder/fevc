#!/usr/bin/env python3
"""Create one source-bound local staging directory for an SCC campaign."""

from __future__ import annotations

import argparse
import json
import re
import shutil
import subprocess
import tarfile
import tempfile
from pathlib import Path

try:
    from .build_bundles import BUNDLE_FIELDS, read_bundles
    from .build_bundles import build_rows as build_bundle_rows
    from .build_manifest import build_rows
    from .common import (
        SMOKE_ESTIMATOR_TIMEOUT_SECONDS,
        SMOKE_HARD_WALL_SECONDS,
        SMOKE_REQUESTED_SLOTS,
        SMOKE_TASK_SCHEMA,
        TASK_FIELDS,
        read_manifest,
        read_single_task,
        require,
        sha256,
        write_tsv,
    )
except ImportError:
    from build_bundles import BUNDLE_FIELDS, read_bundles  # type: ignore
    from build_bundles import build_rows as build_bundle_rows  # type: ignore
    from build_manifest import build_rows  # type: ignore
    from common import (  # type: ignore
        SMOKE_ESTIMATOR_TIMEOUT_SECONDS,
        SMOKE_HARD_WALL_SECONDS,
        SMOKE_REQUESTED_SLOTS,
        SMOKE_TASK_SCHEMA,
        TASK_FIELDS,
        read_manifest,
        read_single_task,
        require,
        sha256,
        write_tsv,
    )


def git(repo: Path, *arguments: str) -> str:
    value = subprocess.run(
        ("git", *arguments), cwd=repo, check=True, text=True,
        capture_output=True,
    )
    return value.stdout.strip()


def source_manifest(source: Path) -> str:
    lines: list[str] = []
    for path in sorted(source.rglob("*")):
        if path.is_symlink():
            raise ValueError(f"source archive contains a symlink: {path}")
        if path.is_file():
            lines.append(f"{sha256(path)}  {path.relative_to(source).as_posix()}")
    require(lines, "source archive is empty")
    return "\n".join(lines) + "\n"


def development_archive(repo: Path, archive: Path) -> None:
    """Archive current tracked and untracked candidate bytes for a diagnostic smoke."""
    with tempfile.TemporaryDirectory(prefix="fevc-matlab-2026-development-") as temp:
        source = Path(temp) / "source"
        source.mkdir()
        paths = git(repo, "ls-files", "--cached", "--others", "--exclude-standard")
        for relative_text in paths.splitlines():
            relative = Path(relative_text)
            candidate = repo / relative
            if not candidate.exists():
                continue
            require(candidate.is_file() and not candidate.is_symlink(),
                    f"invalid development source path: {relative_text}")
            target = source / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(candidate, target)
        with tarfile.open(archive, "w:gz") as handle:
            handle.add(source, arcname="source")


def build(
    repo: Path,
    output: Path,
    spi_dir: Path,
    *,
    mem_per_core_gib: int,
    command_memory_gib: int,
    development: bool,
) -> dict[str, object]:
    repo = repo.resolve()
    require((repo / ".git").is_dir(), "repository root is invalid")
    dirty = bool(git(repo, "status", "--porcelain"))
    require(development or not dirty,
            "publication campaign requires a clean repository")
    source_commit = git(repo, "rev-parse", "HEAD")
    require(len(source_commit) == 40, "invalid source commit")
    run_id_pattern = r"[A-Za-z0-9._-]+"
    require(re.fullmatch(run_id_pattern, output.name) is not None,
            "invalid run ID")
    require(spi_dir.is_dir(), "Stata SPI directory is missing")
    for name in ("stplugin.c", "stplugin.h"):
        require((spi_dir / name).is_file() and not (spi_dir / name).is_symlink(),
                f"invalid Stata SPI input: {name}")
    require(not output.exists(), "staging target already exists")
    output.mkdir(parents=True)
    for name in ("input", "attempts", "receipts",
                 "submissions", "logs", "collection"):
        (output / name).mkdir()
    archive = output / "input" / "source.tar.gz"
    if development:
        development_archive(repo, archive)
    else:
        subprocess.run(
            ("git", "archive", "--format=tar.gz", "--prefix=source/", "-o",
             str(archive), "HEAD"), cwd=repo, check=True,
        )
    bundle_sha = sha256(archive)
    (output / "input" / "source.tar.gz.sha256").write_text(
        f"{bundle_sha}  source.tar.gz\n", encoding="utf-8")
    with tempfile.TemporaryDirectory(prefix="fevc-matlab-2026-source-") as temp:
        temporary = Path(temp)
        with tarfile.open(archive, "r:gz") as handle:
            handle.extractall(temporary, filter="data")
        source = temporary / "source"
        (source / "SOURCE_COMMIT.txt").write_text(source_commit + "\n",
                                                   encoding="utf-8")
        manifest_text = source_manifest(source)
    source_manifest_path = output / "input" / "source.files.sha256"
    source_manifest_path.write_text(manifest_text, encoding="utf-8")

    spi_output = output / "input" / "stata-spi"
    spi_output.mkdir()
    for name in ("stplugin.c", "stplugin.h"):
        shutil.copy2(spi_dir / name, spi_output / name)
    spi_manifest = output / "input" / "stata-spi.sha256"
    spi_manifest.write_text(
        "".join(f"{sha256(spi_output / name)}  stata-spi/{name}\n"
                for name in ("stplugin.c", "stplugin.h")), encoding="utf-8")

    task_manifest = output / "input" / "tasks.tsv"
    rows = build_rows(
        source_commit,
        bundle_sha,
        mem_per_core_gib=mem_per_core_gib,
        command_memory_gib=command_memory_gib,
    )
    write_tsv(task_manifest, TASK_FIELDS, rows)
    read_manifest(task_manifest)
    smoke_manifest = output / "input" / "smoke.tsv"
    smoke = dict(rows[0])
    smoke.update({
        "task_schema": SMOKE_TASK_SCHEMA,
        "experiment_id": "smoke_strong_d2_n7680_c4_r1",
        "active_cores": SMOKE_REQUESTED_SLOTS,
        "stata_processors": SMOKE_REQUESTED_SLOTS,
        "rust_threads": SMOKE_REQUESTED_SLOTS,
        "matlab_workers": SMOKE_REQUESTED_SLOTS,
        "requested_slots": SMOKE_REQUESTED_SLOTS,
        "command_memory_gib": min(command_memory_gib,
                                  mem_per_core_gib * SMOKE_REQUESTED_SLOTS),
        "hard_wall_seconds": SMOKE_HARD_WALL_SECONDS,
        "estimator_timeout_seconds": SMOKE_ESTIMATOR_TIMEOUT_SECONDS,
    })
    write_tsv(smoke_manifest, TASK_FIELDS, (smoke,))
    read_single_task(smoke_manifest)
    bundle_manifest = output / "input" / "bundles.tsv"
    write_tsv(bundle_manifest, BUNDLE_FIELDS, build_bundle_rows(read_manifest(task_manifest)))
    read_bundles(bundle_manifest)
    identity = {
        "schema": "FEVC-MATLAB-2026-CAMPAIGN-V2",
        "status": "PASS",
        "run_id": output.name,
        "source_mode": "DEVELOPMENT_SNAPSHOT" if development else "CLEAN_COMMIT",
        "source_dirty": dirty,
        "source_commit": source_commit,
        "bundle_sha256": bundle_sha,
        "source_manifest_sha256": sha256(source_manifest_path),
        "task_manifest_sha256": sha256(task_manifest),
        "smoke_task_manifest_sha256": sha256(smoke_manifest),
        "bundle_manifest_sha256": sha256(bundle_manifest),
        "stata_spi_manifest_sha256": sha256(spi_manifest),
        "required_stata_processors": max(int(row["stata_processors"]) for row in rows),
        "required_rust_threads": max(int(row["rust_threads"]) for row in rows),
        "required_matlab_workers": max(int(row["matlab_workers"]) for row in rows),
        "smoke_slots": SMOKE_REQUESTED_SLOTS,
        "mem_per_core_gib": mem_per_core_gib,
        "command_memory_gib": command_memory_gib,
        "scheduler_bundles": 24,
        "cells": 240,
        "estimator_calls": 480,
    }
    (output / "run_identity.json").write_text(
        json.dumps(identity, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return identity


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--stata-spi", type=Path, required=True)
    parser.add_argument("--mem-per-core-gib", type=int, default=8)
    parser.add_argument("--command-memory-gib", type=int, default=192)
    parser.add_argument("--development", action="store_true",
                        help="archive current candidate bytes for a smoke-only run")
    args = parser.parse_args()
    require(args.mem_per_core_gib > 0, "memory per core must be positive")
    require(0 < args.command_memory_gib <= args.mem_per_core_gib * 28,
            "command memory must fit the scheduler allocation")
    value = build(
        args.repo, args.output, args.stata_spi,
        mem_per_core_gib=args.mem_per_core_gib,
        command_memory_gib=args.command_memory_gib,
        development=args.development,
    )
    print("FEVC_MATLAB_2026_STAGING_PASS "
          f"{value['run_id']} {value['source_commit']} {value['bundle_sha256']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
