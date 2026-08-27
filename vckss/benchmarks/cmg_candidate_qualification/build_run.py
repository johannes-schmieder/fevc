#!/usr/bin/env python3
"""Stage the immutable 72-task paired CMG-candidate qualification run."""

from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import tarfile
import tempfile
from pathlib import Path

from common import (
    CANDIDATE_BASE_COMMIT,
    COMPARISON_COMMIT,
    CORES,
    REPETITIONS,
    ROWS,
    RUN_SCHEMA,
    STRUCTURES,
    TASK_FIELDS,
    TASK_SCHEMA,
    read_manifest,
    require,
    sha256,
    write_tsv,
)


def git(repo: Path, *arguments: str) -> str:
    return subprocess.run(("git", *arguments), cwd=repo, check=True, text=True,
                          stdout=subprocess.PIPE, stderr=subprocess.PIPE).stdout.strip()


def manifest_for_archive(archive: Path) -> str:
    with tempfile.TemporaryDirectory(prefix="vckss-cmg-qualification-") as temp:
        root = Path(temp)
        with tarfile.open(archive, "r:gz") as handle:
            handle.extractall(root, filter="data")
        source = root / "source"
        lines = [f"{sha256(path)}  {path.relative_to(source).as_posix()}"
                 for path in sorted(source.rglob("*")) if path.is_file()]
    require(bool(lines), "source archive is empty")
    return "\n".join(lines) + "\n"


def build(repo: Path, output: Path, spi: Path, mem: int, command_mem: int) -> dict:
    repo = repo.resolve()
    require((repo / ".git").is_dir() and not git(repo, "status", "--porcelain"),
            "repository must be a clean Git checkout")
    candidate = git(repo, "rev-parse", "HEAD")
    require(len(candidate) == 40 and
            subprocess.run(("git", "merge-base", "--is-ancestor",
                            CANDIDATE_BASE_COMMIT, candidate), cwd=repo).returncode == 0,
            "candidate does not descend from the integrated d9 checkpoint")
    require(git(repo, "rev-parse", COMPARISON_COMMIT) == COMPARISON_COMMIT,
            "comparison checkpoint is unavailable")
    require(not output.exists() and spi.is_dir(), "invalid staging target or Stata SPI")
    require(mem > 0 and 0 < command_mem <= 16 * mem, "invalid memory policy")
    for name in ("stplugin.c", "stplugin.h"):
        require((spi / name).is_file() and not (spi / name).is_symlink(),
                f"invalid Stata SPI input: {name}")
    for name in ("input", "receipts", "submissions", "logs", "tasks",
                 "validations", "qacct", "collection"):
        (output / name).mkdir(parents=True, exist_ok=False)
    identities: dict[str, dict[str, str]] = {}
    for label, commit in (("candidate", candidate), ("comparison", COMPARISON_COMMIT)):
        archive = output / "input" / f"{label}.tar.gz"
        subprocess.run(("git", "archive", "--format=tar.gz", "--prefix=source/",
                        "-o", str(archive), commit), cwd=repo, check=True)
        manifest = output / "input" / f"{label}.files.sha256"
        manifest.write_text(manifest_for_archive(archive), encoding="utf-8")
        identities[label] = {"commit": commit, "bundle_sha256": sha256(archive),
                             "source_manifest_sha256": sha256(manifest)}
        (output / "input" / f"{label}.tar.gz.sha256").write_text(
            f"{identities[label]['bundle_sha256']}  {label}.tar.gz\n", encoding="utf-8")
    spi_out = output / "input" / "stata-spi"
    spi_out.mkdir()
    for name in ("stplugin.c", "stplugin.h"):
        shutil.copy2(spi / name, spi_out / name)
    spi_manifest = output / "input" / "stata-spi.sha256"
    spi_manifest.write_text("".join(
        f"{sha256(spi_out / name)}  stata-spi/{name}\n"
        for name in ("stplugin.c", "stplugin.h")), encoding="utf-8")

    tasks = []
    task_id = 0
    for structure, (connectivity, degree) in STRUCTURES.items():
        for cores in CORES:
            for replicate, seed, order in REPETITIONS:
                task_id += 1
                tasks.append({
                    "task_schema": TASK_SCHEMA, "task_id": task_id,
                    "experiment_id": f"cmgq_{structure}_n{ROWS}_c{cores}_r{replicate}",
                    "candidate_commit": candidate, "comparison_commit": COMPARISON_COMMIT,
                    "candidate_bundle_sha256": identities["candidate"]["bundle_sha256"],
                    "comparison_bundle_sha256": identities["comparison"]["bundle_sha256"],
                    "structure": structure, "connectivity": connectivity,
                    "cells_per_worker": degree, "rows": ROWS,
                    "workers": ROWS // degree, "firms": ROWS // degree // 40,
                    "active_cores": cores, "replicate": replicate, "seed": seed,
                    "execution_order": order, "probes": 200, "requested_slots": 16,
                    "mem_per_core_gib": mem, "command_memory_gib": command_mem,
                    "hard_wall_seconds": 43_200, "estimator_timeout_seconds": 18_000,
                })
    task_manifest = output / "input" / "tasks.tsv"
    write_tsv(task_manifest, TASK_FIELDS, tasks)
    read_manifest(task_manifest)
    identity = {
        "schema": RUN_SCHEMA, "status": "PASS", "run_id": output.name,
        "candidate_commit": candidate, "comparison_commit": COMPARISON_COMMIT,
        "candidate_bundle_sha256": identities["candidate"]["bundle_sha256"],
        "comparison_bundle_sha256": identities["comparison"]["bundle_sha256"],
        "candidate_source_manifest_sha256": identities["candidate"]["source_manifest_sha256"],
        "comparison_source_manifest_sha256": identities["comparison"]["source_manifest_sha256"],
        "task_manifest_sha256": sha256(task_manifest),
        "stata_spi_manifest_sha256": sha256(spi_manifest),
        "required_stata_processors": max(CORES),
        "mem_per_core_gib": mem, "command_memory_gib": command_mem,
        "tasks": 72, "candidate_calls": 72, "comparison_calls": 72,
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
    value = build(args.repo, args.output, args.stata_spi,
                  args.mem_per_core_gib, args.command_memory_gib)
    print("VCKSS_CMG_CANDIDATE_QUALIFICATION_STAGING_PASS "
          f"run={value['run_id']} candidate={value['candidate_commit']} "
          f"comparison={value['comparison_commit']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
