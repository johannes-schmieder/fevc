#!/usr/bin/env python3
"""Create one immutable local staging directory for an SCC benchmark run."""

from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import tarfile
import tempfile
from pathlib import Path

try:
    from .build_manifest import build_rows
    from .common import TASK_FIELDS, read_manifest, require, sha256, write_tsv
except ImportError:
    from build_manifest import build_rows  # type: ignore
    from common import TASK_FIELDS, read_manifest, require, sha256, write_tsv  # type: ignore


def git(repo: Path, *arguments: str) -> str:
    value = subprocess.run(
        ("git", *arguments), cwd=repo, check=True, text=True,
        stdout=subprocess.PIPE, stderr=subprocess.PIPE,
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


def build(
    repo: Path,
    output: Path,
    spi_dir: Path,
    *,
    mem_per_core_gib: int,
    command_memory_gib: int,
) -> dict[str, object]:
    repo = repo.resolve()
    require((repo / ".git").is_dir(), "repository root is invalid")
    require(not git(repo, "status", "--porcelain"), "repository is not clean")
    source_commit = git(repo, "rev-parse", "HEAD")
    require(len(source_commit) == 40, "invalid source commit")
    require(spi_dir.is_dir(), "Stata SPI directory is missing")
    for name in ("stplugin.c", "stplugin.h"):
        require((spi_dir / name).is_file() and not (spi_dir / name).is_symlink(),
                f"invalid Stata SPI input: {name}")
    require(not output.exists(), "staging target already exists")
    output.mkdir(parents=True)
    for name in ("input", "tasks", "validations", "qacct", "receipts",
                 "submissions", "logs", "collection"):
        (output / name).mkdir()
    archive = output / "input" / "source.tar.gz"
    subprocess.run(
        ("git", "archive", "--format=tar.gz", "--prefix=source/", "-o",
         str(archive), "HEAD"), cwd=repo, check=True,
    )
    bundle_sha = sha256(archive)
    (output / "input" / "source.tar.gz.sha256").write_text(
        f"{bundle_sha}  source.tar.gz\n", encoding="utf-8")
    with tempfile.TemporaryDirectory(prefix="vckss-comparative-source-") as temp:
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
    identity = {
        "schema": "VCKSS-COMPARATIVE-SCALING-STAGED-RUN-V1",
        "status": "PASS",
        "run_id": output.name,
        "source_commit": source_commit,
        "bundle_sha256": bundle_sha,
        "source_manifest_sha256": sha256(source_manifest_path),
        "task_manifest_sha256": sha256(task_manifest),
        "stata_spi_manifest_sha256": sha256(spi_manifest),
        "mem_per_core_gib": mem_per_core_gib,
        "command_memory_gib": command_memory_gib,
        "tasks": 300,
        "estimator_calls": 900,
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
    parser.add_argument("--command-memory-gib", type=int, default=112)
    args = parser.parse_args()
    require(args.mem_per_core_gib > 0, "memory per core must be positive")
    require(0 < args.command_memory_gib <= args.mem_per_core_gib * 16,
            "command memory must fit the scheduler allocation")
    value = build(
        args.repo, args.output, args.stata_spi,
        mem_per_core_gib=args.mem_per_core_gib,
        command_memory_gib=args.command_memory_gib,
    )
    print("VCKSS_COMPARATIVE_SCALING_STAGING_PASS "
          f"{value['run_id']} {value['source_commit']} {value['bundle_sha256']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
