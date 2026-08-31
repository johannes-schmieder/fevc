#!/usr/bin/env python3
"""Authorize exact missing or scheduler-failed task IDs for one retry attempt."""

from __future__ import annotations

import argparse
import json
import subprocess
from pathlib import Path
from typing import Any

try:
    from .common import key_values, load_json, read_single_task, require
    from .expand_task_ids import expand
except ImportError:
    from common import key_values, load_json, read_single_task, require  # type: ignore
    from expand_task_ids import expand  # type: ignore


RETRY_SCHEMA = "FEVC-COMPARATIVE-SCALING-RETRY-AUTHORIZATION-V1"


def raw_qacct(path: Path) -> dict[str, str]:
    values: dict[str, str] = {}
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        fields = line.strip().split(None, 1)
        if len(fields) == 2:
            values[fields[0]] = fields[1]
    require({"jobnumber", "taskid", "failed", "exit_status"} <= values.keys(),
            f"incomplete prior qacct: {path}")
    return values


def submitted_jobs(run_dir: Path, task_ids: list[int]) -> dict[int, set[str]]:
    output = {task_id: set() for task_id in task_ids}
    for path in sorted((run_dir / "submissions").glob("*.tsv")):
        value = key_values(path)
        if value.get("mode") not in {"production", "retry"}:
            continue
        covered = set(expand(value["task_range"]))
        for task_id in task_ids:
            if task_id in covered:
                output[task_id].add(value["job_id"])
    require(all(output.values()), "retry includes a task never submitted in this run")
    return output


def require_jobs_inactive(jobs: set[str]) -> None:
    for job_id in sorted(jobs):
        status = subprocess.run(
            ("qstat", "-j", job_id), stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL, check=False,
        )
        require(status.returncode != 0,
                f"prior array job {job_id} is still active")


def classify(run_dir: Path, task_id: int) -> str:
    validations = sorted((run_dir / "attempts").glob(
        f"*/validations/{task_id}.json"))
    if validations:
        require(all(load_json(path).get("status") != "PASS" for path in validations),
                f"task {task_id} already has a successful validation")
        raise ValueError(
            f"task {task_id} has validation evidence; use a new generation")

    task_outputs: list[Path] = []
    for path in sorted((run_dir / "attempts").glob("*/tasks/*/task.tsv")):
        if int(read_single_task(path)["task_id"]) == task_id:
            task_outputs.append(path.parent)
    require(not any((path / "wrapper.pass").exists() for path in task_outputs),
            f"task {task_id} completed its wrapper; use a new generation")

    qacct_paths = sorted((run_dir / "attempts").glob(f"*/qacct/{task_id}.txt"))
    if qacct_paths:
        records = [raw_qacct(path) for path in qacct_paths]
        require(not any(value["failed"] == "0" for value in records),
                f"task {task_id} has non-infrastructure accounting evidence")
        return "SCHEDULER_FAILED"
    require(not task_outputs,
            f"task {task_id} has output but incomplete accounting")
    return "MISSING_TASK_AND_ACCOUNTING"


def authorize(run_dir: Path, task_spec: str, *, check_qstat: bool = True) -> dict[str, Any]:
    task_ids = expand(task_spec)
    jobs_by_task = submitted_jobs(run_dir, task_ids)
    if check_qstat:
        require_jobs_inactive(set().union(*jobs_by_task.values()))
    classifications = {str(task_id): classify(run_dir, task_id)
                       for task_id in task_ids}
    identity = load_json(run_dir / "run_identity.json")
    require(identity.get("run_kind") == "production" and
            identity.get("status") == "PASS", "retry run identity changed")
    return {
        "schema": RETRY_SCHEMA,
        "status": "PASS",
        "run_id": identity["run_id"],
        "source_commit": identity["source_commit"],
        "bundle_sha256": identity["bundle_sha256"],
        "source_manifest_sha256": identity["source_manifest_sha256"],
        "task_manifest_sha256": identity["task_manifest_sha256"],
        "task_spec": task_spec,
        "task_ids": task_ids,
        "prior_job_ids": {str(key): sorted(value)
                          for key, value in jobs_by_task.items()},
        "classifications": classifications,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--run-dir", type=Path, required=True)
    parser.add_argument("--task-ids", required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    require(not args.output.exists(), "retry authorization target already exists")
    value = authorize(args.run_dir, args.task_ids)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n",
                           encoding="utf-8")
    print("VCKSS_COMPARATIVE_SCALING_RETRY_AUTHORIZED "
          f"tasks={args.task_ids}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
