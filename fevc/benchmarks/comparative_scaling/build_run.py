#!/usr/bin/env python3
"""Create one immutable local staging directory for an SCC benchmark run."""

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
    run_kind: str,
    artifact_source_run_id: str | None,
    pilot_small_run_id: str | None,
    pilot_worst_run_id: str | None,
    replaces_run_id: str | None,
) -> dict[str, object]:
    repo = repo.resolve()
    require((repo / ".git").is_dir(), "repository root is invalid")
    require(not git(repo, "status", "--porcelain"), "repository is not clean")
    source_commit = git(repo, "rev-parse", "HEAD")
    require(len(source_commit) == 40, "invalid source commit")
    require(run_kind in {"preparation", "pilot-small", "pilot-worst", "production"},
            "invalid run kind")
    run_id_pattern = r"[A-Za-z0-9._-]+"
    require(re.fullmatch(run_id_pattern, output.name) is not None,
            "invalid run ID")
    if run_kind == "preparation":
        require(artifact_source_run_id is None,
                "canonical preparation cannot import artifacts")
    else:
        require(artifact_source_run_id is not None and
                re.fullmatch(run_id_pattern, artifact_source_run_id) is not None and
                artifact_source_run_id != output.name,
                "measurement run requires a distinct canonical artifact source")
    if run_kind == "production":
        require(bool(pilot_small_run_id) and bool(pilot_worst_run_id),
                "production requires both pilot run IDs")
        require(pilot_small_run_id != pilot_worst_run_id,
                "pilot run IDs must differ")
        require(replaces_run_id is None or
                (re.fullmatch(run_id_pattern, replaces_run_id) is not None and
                 replaces_run_id != output.name),
                "replacement production requires a distinct prior run ID")
    else:
        require(pilot_small_run_id is None and pilot_worst_run_id is None,
                "pilot prerequisites are production-only")
        require(replaces_run_id is None,
                "replacement lineage is production-only")
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
    subprocess.run(
        ("git", "archive", "--format=tar.gz", "--prefix=source/", "-o",
         str(archive), "HEAD"), cwd=repo, check=True,
    )
    bundle_sha = sha256(archive)
    (output / "input" / "source.tar.gz.sha256").write_text(
        f"{bundle_sha}  source.tar.gz\n", encoding="utf-8")
    with tempfile.TemporaryDirectory(prefix="fevc-comparative-source-") as temp:
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
        "schema": "FEVC-COMPARATIVE-SCALING-STAGED-RUN-V3",
        "status": "PASS",
        "run_id": output.name,
        "run_kind": run_kind,
        "artifact_source_run_id": artifact_source_run_id,
        "pilot_small_run_id": pilot_small_run_id,
        "pilot_worst_run_id": pilot_worst_run_id,
        "replaces_run_id": replaces_run_id,
        "source_commit": source_commit,
        "bundle_sha256": bundle_sha,
        "source_manifest_sha256": sha256(source_manifest_path),
        "task_manifest_sha256": sha256(task_manifest),
        "stata_spi_manifest_sha256": sha256(spi_manifest),
        "required_stata_processors": max(int(row["stata_processors"]) for row in rows),
        "maximum_mata_cores": max(int(row["mata_active_cores"]) for row in rows),
        "required_rust_threads": max(int(row["rust_threads"]) for row in rows),
        "required_matlab_workers": max(int(row["matlab_workers"]) for row in rows),
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
    parser.add_argument("--run-kind", required=True,
                        choices=("preparation", "pilot-small", "pilot-worst",
                                 "production"))
    parser.add_argument("--artifact-source-run-id")
    parser.add_argument("--pilot-small-run-id")
    parser.add_argument("--pilot-worst-run-id")
    parser.add_argument("--replaces-run-id")
    args = parser.parse_args()
    require(args.mem_per_core_gib > 0, "memory per core must be positive")
    require(0 < args.command_memory_gib <= args.mem_per_core_gib * 16,
            "command memory must fit the scheduler allocation")
    value = build(
        args.repo, args.output, args.stata_spi,
        mem_per_core_gib=args.mem_per_core_gib,
        command_memory_gib=args.command_memory_gib,
        run_kind=args.run_kind,
        artifact_source_run_id=args.artifact_source_run_id,
        pilot_small_run_id=args.pilot_small_run_id,
        pilot_worst_run_id=args.pilot_worst_run_id,
        replaces_run_id=args.replaces_run_id,
    )
    print("VCKSS_COMPARATIVE_SCALING_STAGING_PASS "
          f"{value['run_id']} {value['source_commit']} {value['bundle_sha256']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
