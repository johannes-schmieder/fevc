#!/usr/bin/env python3
"""Require complete SGE accounting and classify one captured array stage."""

from __future__ import annotations

import argparse
import json
from pathlib import Path

from common import read_manifest


def parse_qacct(path: Path, expected_tasks: int, job_id: int) -> dict[int, dict[str, str]]:
    records: dict[int, dict[str, str]] = {}
    current: int | None = None
    for raw in path.read_text(encoding="utf-8").splitlines():
        line = raw.strip()
        if line.startswith("TASK="):
            current = int(line.split("=", 1)[1])
            records[current] = {}
            continue
        if current is None or not line:
            continue
        key, _, value = line.partition(" ")
        if key in {"jobnumber", "taskid", "failed", "exit_status", "hostname", "qname", "ru_wallclock", "ru_maxrss", "maxvmem"}:
            records[current][key] = value.strip()
    expected = set(range(1, expected_tasks + 1))
    if set(records) != expected:
        raise ValueError(f"incomplete qacct tasks: {sorted(records)}")
    for task_id, record in records.items():
        if int(record.get("jobnumber", -1)) != job_id:
            raise ValueError(f"job identity changed for task {task_id}")
        if int(record.get("taskid", -1)) != task_id:
            raise ValueError(f"task identity changed for task {task_id}")
        if record.get("failed") != "0" or record.get("exit_status") != "0":
            raise ValueError(f"scheduler/application failure for task {task_id}")
    return records


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("run_dir", type=Path)
    parser.add_argument("stage")
    parser.add_argument("job_id", type=int)
    args = parser.parse_args()
    manifest = args.run_dir / "manifests" / f"{args.stage}.tsv"
    tasks = read_manifest(manifest)
    qacct_path = args.run_dir / "receipts" / f"{args.stage}-qacct.txt"
    parse_qacct(qacct_path, len(tasks), args.job_id)
    statuses: list[str] = []
    for task_id, task in enumerate(tasks, 1):
        task_dir = args.run_dir / "tasks" / args.stage / f"task-{task_id}"
        if not (task_dir / "task.captured").is_file():
            raise ValueError(f"task {task_id} capture marker is missing")
        value = json.loads((task_dir / "validation.json").read_text(encoding="utf-8"))
        if value.get("task_sha256") != task["task_sha256"]:
            raise ValueError(f"task {task_id} validation identity changed")
        statuses.append(value.get("status", "MISSING"))
    result = {
        "schema": "VCKSS-PROJECTION-AKM-STAGE-V1",
        "status": "PASS" if set(statuses) == {"PASS"} else "COMPLETE_NONPASS",
        "stage": args.stage,
        "job_id": args.job_id,
        "task_count": len(tasks),
        "task_statuses": statuses,
        "scheduler_accounting_complete": True,
    }
    output = args.run_dir / "receipts" / f"{args.stage}.json"
    output.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    marker = args.run_dir / "receipts" / f"{args.stage}.complete"
    marker.write_text(f"VCKSS PROJECTION AKM STAGE COMPLETE {args.stage} {args.job_id}\n", encoding="utf-8")
    if result["status"] == "PASS":
        (args.run_dir / "receipts" / f"{args.stage}.pass").write_text(
            f"VCKSS PROJECTION AKM STAGE PASS {args.stage} {args.job_id}\n",
            encoding="utf-8",
        )
    print(json.dumps(result, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
