#!/usr/bin/env python3
"""Collect 69 successful replacements plus three registered MATLAB censors."""

from __future__ import annotations

import argparse
import json
import re
import subprocess
from pathlib import Path
from typing import Any

try:
    from .censoring import (
        CENSORED_TASK_IDS,
        CENSOR_FIELDS,
        CENSOR_STATUS,
        SCHEMA as CENSOR_SCHEMA,
        qacct_fields,
        validate_censored_task,
    )
    from .common import load_json, read_manifest, require, sha256, write_tsv
    from .expand_task_ids import expand
    from .task_map import scheduler_id
except ImportError:
    from censoring import (  # type: ignore
        CENSORED_TASK_IDS,
        CENSOR_FIELDS,
        CENSOR_STATUS,
        SCHEMA as CENSOR_SCHEMA,
        qacct_fields,
        validate_censored_task,
    )
    from common import load_json, read_manifest, require, sha256, write_tsv  # type: ignore
    from expand_task_ids import expand  # type: ignore
    from task_map import scheduler_id  # type: ignore


SCHEMA = "FEVC-COMPARATIVE-SCALING-CENSORED-REPLACEMENT-COLLECTION-V1"


def collect_qacct(job_id: str, scheduler_task_id: int, output: Path) -> None:
    result = subprocess.run(
        ("qacct", "-j", job_id, "-t", str(scheduler_task_id)),
        check=True, text=True, stdout=subprocess.PIPE,
    )
    require(result.stdout.strip(), "qacct returned no task record")
    output.write_text(result.stdout, encoding="utf-8")


def collect(
    run_dir: Path,
    attempt_id: str,
    job_id: str,
    task_spec: str,
    output_dir: Path,
    processor_source_commit: str,
) -> dict[str, Any]:
    require(output_dir.is_dir() and not any(output_dir.iterdir()),
            "censored collection output must be an empty directory")
    identity = load_json(run_dir / "run_identity.json")
    require(identity.get("status") == "PASS" and
            identity.get("run_kind") == "production",
            "replacement run identity changed")
    require(re.fullmatch(r"[0-9a-f]{40}", processor_source_commit) is not None,
            "invalid postprocessor source commit")
    requested = expand(task_spec)
    authorizations = [load_json(path) for path in sorted(
        (run_dir / "receipts" / "replacement_authorizations").glob("*.json"))]
    affected = sorted({int(task_id) for value in authorizations
                       for task_id in value.get("task_ids", [])})
    require(len(authorizations) >= 1 and requested == affected and
            len(affected) == 72 and set(CENSORED_TASK_IDS) <= set(affected),
            "replacement authorization inventory changed")

    tasks = {int(row["task_id"]): row
             for row in read_manifest(run_dir / "input" / "tasks.tsv")}
    attempt = run_dir / "attempts" / attempt_id
    qacct_dir = attempt / "qacct"
    validation_dir = attempt / "validations"
    task_map = run_dir / "submissions" / f"{attempt_id}.task-map.tsv"
    require(qacct_dir.is_dir() and validation_dir.is_dir() and
            task_map.is_file() and not task_map.is_symlink(),
            "replacement attempt evidence directories changed")
    harness = run_dir / "source" / "fevc" / "benchmarks" / \
        "comparative_scaling"

    validated: list[int] = []
    censored: list[dict[str, Any]] = []
    qacct_hashes: dict[str, str] = {}
    validation_hashes: dict[str, str] = {}
    for task_id in affected:
        scheduler_task_id = scheduler_id(task_map, task_id)
        qacct_path = qacct_dir / f"{task_id}.txt"
        if not qacct_path.exists():
            collect_qacct(job_id, scheduler_task_id, qacct_path)
        fields = qacct_fields(qacct_path)
        require(fields["jobnumber"] == job_id and
                int(fields["taskid"]) == scheduler_task_id and
                fields["failed"] == "0",
                "replacement qacct identity changed")
        qacct_hashes[str(task_id)] = sha256(qacct_path)
        task = tasks[task_id]
        job_dir = attempt / "tasks" / task["experiment_id"]
        validation = validation_dir / f"{task_id}.json"
        if task_id in CENSORED_TASK_IDS:
            require(not validation.exists(),
                    "censored task unexpectedly has a successful validation")
            censored.append(validate_censored_task(
                task, job_dir, qacct_path, job_id=job_id,
                scheduler_task_id=scheduler_task_id))
            continue
        require(fields["exit_status"] == "0",
                "noncensored replacement task did not exit zero")
        if not validation.exists():
            subprocess.run((
                "python3", str(harness / "validate_task.py"),
                "--job-dir", str(job_dir), "--qacct", str(qacct_path),
                "--output", str(validation),
            ), check=True)
        payload = load_json(validation)
        require(payload.get("status") == "PASS" and
                int(payload["task"]["task_id"]) == task_id,
                "replacement validation changed")
        validated.append(task_id)
        validation_hashes[str(task_id)] = sha256(validation)

    require(len(validated) == 69 and
            [int(row["task_id"]) for row in censored] ==
            list(CENSORED_TASK_IDS) and
            all(row["status"] == CENSOR_STATUS for row in censored),
            "replacement success/censor partition changed")
    ledger = output_dir / "censored_matlab_3.tsv"
    write_tsv(ledger, CENSOR_FIELDS, censored)
    return {
        "schema": SCHEMA,
        "status": "PASS_WITH_REGISTERED_CENSORING",
        "run_id": identity["run_id"],
        "source_commit": identity["source_commit"],
        "bundle_sha256": identity["bundle_sha256"],
        "processor_source_commit": processor_source_commit,
        "attempt_id": attempt_id,
        "job_id": job_id,
        "task_map_sha256": sha256(task_map),
        "authorized_tasks": 72,
        "validated_tasks": 69,
        "validated_task_ids": validated,
        "censored_tasks": 3,
        "censored_task_ids": list(CENSORED_TASK_IDS),
        "censor_schema": CENSOR_SCHEMA,
        "censor_ledger_sha256": sha256(ledger),
        "qacct_sha256": qacct_hashes,
        "validation_sha256": validation_hashes,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--run-dir", type=Path, required=True)
    parser.add_argument("--attempt-id", required=True)
    parser.add_argument("--job-id", required=True)
    parser.add_argument("--task-ids", required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--processor-source-commit", required=True)
    args = parser.parse_args()
    require(re.fullmatch(r"[A-Za-z0-9._-]+", args.attempt_id) is not None,
            "invalid attempt ID")
    require(re.fullmatch(r"[0-9]+", args.job_id) is not None,
            "invalid job ID")
    value = collect(
        args.run_dir, args.attempt_id, args.job_id, args.task_ids,
        args.output_dir, args.processor_source_commit)
    receipt = args.output_dir / "censored_replacement.pass.json"
    require(not receipt.exists(), "censored collection receipt exists")
    receipt.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n",
                       encoding="utf-8")
    print("VCKSS_COMPARATIVE_SCALING_CENSORED_REPLACEMENT_PASS "
          "validated=69 censored=3")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
