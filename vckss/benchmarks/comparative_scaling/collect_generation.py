#!/usr/bin/env python3
"""Collect terminal accounting and validate every successful array task."""

from __future__ import annotations

import argparse
import csv
import json
import re
import subprocess
from pathlib import Path
from typing import Any

try:
    from .common import TASK_FIELDS, load_json, require, sha256
except ImportError:
    from common import TASK_FIELDS, load_json, require, sha256  # type: ignore


SCHEMA = "VCKSS-COMPARATIVE-SCALING-GENERATION-INVENTORY-V1"


def frozen_manifest(run_dir: Path,
                    identity: dict[str, Any]) -> list[dict[str, str]]:
    """Read a byte-bound predecessor manifest under its historical schema."""
    path = run_dir / "input" / "tasks.tsv"
    require(path.is_file() and sha256(path) == identity.get("task_manifest_sha256"),
            "frozen task-manifest identity changed")
    with path.open(encoding="utf-8", newline="") as handle:
        reader = csv.DictReader(handle, delimiter="\t")
        require(tuple(reader.fieldnames or ()) == TASK_FIELDS,
                "frozen task-manifest fields changed")
        tasks = [dict(row) for row in reader]
    require(len(tasks) == 300 and
            [int(task["task_id"]) for task in tasks] == list(range(1, 301)),
            "frozen task inventory changed")
    return tasks


def qacct_fields(path: Path) -> dict[str, str]:
    values: dict[str, str] = {}
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        fields = line.strip().split(None, 1)
        if len(fields) == 2:
            values[fields[0]] = fields[1]
    require({"jobnumber", "taskid", "failed", "exit_status"} <= values.keys(),
            f"incomplete qacct: {path}")
    return values


def collect_qacct(job_id: str, output: Path) -> None:
    result = subprocess.run(
        ("qacct", "-j", job_id), check=True, text=True,
        stdout=subprocess.PIPE,
    )
    chunks = [item for item in re.split(
        r"(?=^=+\nqname)", result.stdout, flags=re.MULTILINE) if item.strip()]
    require(len(chunks) == 300, "terminal qacct must contain 300 task records")
    seen: set[int] = set()
    for chunk in chunks:
        match = re.search(r"^taskid\s+(\d+)", chunk, re.MULTILINE)
        require(match is not None, "qacct task ID is missing")
        task_id = int(match.group(1))
        require(task_id not in seen, "duplicate qacct task ID")
        seen.add(task_id)
        (output / f"{task_id}.txt").write_text(chunk, encoding="utf-8")
    require(seen == set(range(1, 301)), "qacct task inventory changed")


def failure_class(task: dict[str, str], job_dir: Path) -> str:
    if task["structure"] == "weak_d3" and int(task["rows"]) == 7_680:
        replicate = int(task["replicate"])
        signatures = {
            1: (job_dir / "mata" / "application.txt", "assertion is false"),
            2: (job_dir / "rust" / "application.txt", "worker articulations"),
            3: (job_dir / "matlab" / "failure.json", "RetainedMatches"),
        }
        path, signature = signatures[replicate]
        require(path.is_file() and signature in path.read_text(
            encoding="utf-8", errors="replace"),
            "weak-small failure signature changed")
        return "SCIENTIFIC_SAMPLE_CONTRACT"
    if int(task["active_cores"]) == 1:
        monitor = job_dir / "matlab" / "process_tree.monitor.txt"
        source = monitor.read_text(encoding="utf-8", errors="replace")
        require("TypeError: 'int' object is not iterable" in source,
                "one-core instrumentation failure signature changed")
        return "INSTRUMENTATION_ONE_WORKER_JSON_SCALAR"
    raise ValueError(f"unclassified failed task {task['task_id']}")


def inventory(run_dir: Path, attempt_id: str, job_id: str) -> dict[str, Any]:
    identity = load_json(run_dir / "run_identity.json")
    require(identity.get("run_kind") == "production" and
            identity.get("status") == "PASS", "production identity changed")
    tasks = frozen_manifest(run_dir, identity)
    attempt = run_dir / "attempts" / attempt_id
    qacct_dir = attempt / "qacct"
    validation_dir = attempt / "validations"
    require(qacct_dir.is_dir() and validation_dir.is_dir(),
            "attempt directories are missing")
    if len(list(qacct_dir.glob("[0-9]*.txt"))) != 300:
        collect_qacct(job_id, qacct_dir)

    harness = run_dir / "source" / "vckss" / "benchmarks" / \
        "comparative_scaling"
    successful: list[int] = []
    failures: dict[str, str] = {}
    qacct_hashes: dict[str, str] = {}
    validation_hashes: dict[str, str] = {}
    for task in tasks:
        task_id = int(task["task_id"])
        qacct_path = qacct_dir / f"{task_id}.txt"
        fields = qacct_fields(qacct_path)
        require(fields["jobnumber"] == job_id and
                int(fields["taskid"]) == task_id and
                fields["failed"] == "0", "scheduler accounting changed")
        qacct_hashes[str(task_id)] = sha256(qacct_path)
        validation = validation_dir / f"{task_id}.json"
        job_dir = attempt / "tasks" / task["experiment_id"]
        if fields["exit_status"] == "0":
            if not validation.exists():
                subprocess.run((
                    "python3", str(harness / "validate_task.py"),
                    "--job-dir", str(job_dir), "--qacct", str(qacct_path),
                    "--output", str(validation),
                ), check=True)
            value = load_json(validation)
            require(value.get("status") == "PASS" and
                    int(value["task"]["task_id"]) == task_id,
                    "successful validation changed")
            successful.append(task_id)
            validation_hashes[str(task_id)] = sha256(validation)
        else:
            require(fields["exit_status"] == "1" and not validation.exists(),
                    "failed task accounting changed")
            failures[str(task_id)] = failure_class(task, job_dir)

    classes = sorted(set(failures.values()))
    return {
        "schema": SCHEMA,
        "status": "PASS",
        "terminal": True,
        "run_id": identity["run_id"],
        "source_commit": identity["source_commit"],
        "bundle_sha256": identity["bundle_sha256"],
        "attempt_id": attempt_id,
        "job_id": job_id,
        "qacct_records": 300,
        "scheduler_failed_records": 0,
        "exit_status_zero_records": len(successful),
        "exit_status_nonzero_records": len(failures),
        "validated_task_ids": successful,
        "failed_task_ids": [int(value) for value in failures],
        "failure_classifications": failures,
        "failure_class_counts": {
            label: sum(value == label for value in failures.values())
            for label in classes
        },
        "qacct_sha256": qacct_hashes,
        "validation_sha256": validation_hashes,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--run-dir", type=Path, required=True)
    parser.add_argument("--attempt-id", required=True)
    parser.add_argument("--job-id", required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    require(re.fullmatch(r"[A-Za-z0-9._-]+", args.attempt_id) is not None,
            "invalid attempt ID")
    require(re.fullmatch(r"[0-9]+", args.job_id) is not None,
            "invalid job ID")
    require(not args.output.exists(), "generation inventory target exists")
    value = inventory(args.run_dir, args.attempt_id, args.job_id)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n",
                           encoding="utf-8")
    print("VCKSS_COMPARATIVE_SCALING_GENERATION_INVENTORY_PASS "
          f"validated={value['exit_status_zero_records']} "
          f"failed={value['exit_status_nonzero_records']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
