#!/usr/bin/env python3
"""Require complete successful SCC accounting before consuming RC array output."""

from __future__ import annotations

import argparse
import json
import math
import re
import subprocess
import time
from pathlib import Path


def validate(payload: str, job: int, expected: set[str]) -> list[dict[str, str]]:
    records = []
    for block in re.split(r"(?m)^=+\s*$", payload):
        if not block.strip():
            continue
        record = {}
        for line in block.strip().splitlines():
            fields = line.split(maxsplit=1)
            if len(fields) != 2 or fields[0] in record:
                raise ValueError("malformed or duplicate accounting field")
            record[fields[0]] = fields[1].strip()
        required = {"jobnumber", "taskid", "owner", "project", "slots", "failed",
                    "exit_status", "ru_wallclock", "cpu", "maxvmem", "hostname", "qname"}
        if not required <= record.keys():
            raise ValueError("incomplete accounting record")
        if (record["jobnumber"] != str(job) or record["owner"] != "johannes"
                or record["project"] != "welfgr" or record["slots"] != "1"):
            raise ValueError("accounting identity or allocation mismatch")
        if record["failed"].split()[0] != "0" or record["exit_status"] != "0":
            raise ValueError(f"failed scheduler record: job {job} task {record['taskid']}")
        if any(not math.isfinite(float(record[key])) or float(record[key]) < 0
               for key in ("ru_wallclock", "cpu")):
            raise ValueError("invalid accounting resource use")
        if not re.fullmatch(r"[0-9]+(?:\.[0-9]+)?[KMGT]?", record["maxvmem"]):
            raise ValueError("invalid maximum virtual memory")
        records.append(record)
    keys = [record["taskid"] for record in records]
    if len(set(keys)) != len(keys) or set(keys) != expected:
        raise ValueError("missing, duplicate, or unexpected scheduler tasks")
    return records


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("run_dir", type=Path)
    parser.add_argument("--stages", nargs="+", choices=("build", "tasks", "aggregate"),
                        default=["build", "tasks"])
    parser.add_argument("--timeout", type=float, default=180)
    args = parser.parse_args()
    manifest = json.loads((args.run_dir / "input/campaign-manifest.json").read_text())
    if not 0 <= args.timeout <= 300:
        parser.error("timeout must be between zero and 300 seconds")
    summary = []
    for stage in args.stages:
        job = int((args.run_dir / f"submissions/{stage}.job_id").read_text())
        expected = ({str(task["task_id"]) for task in manifest["tasks"]}
                    if stage == "tasks" else {"undefined"})
        deadline = time.monotonic() + args.timeout
        while True:
            result = subprocess.run(["qacct", "-j", str(job)], capture_output=True, text=True)
            try:
                if result.returncode:
                    raise ValueError(result.stderr.strip() or "accounting not yet available")
                records = validate(result.stdout, job, expected)
                break
            except ValueError as error:
                if time.monotonic() >= deadline or "failed scheduler record" in str(error):
                    raise SystemExit(f"RC scheduler gate failed: {stage}: {error}") from error
                time.sleep(5)
        path = args.run_dir / f"qacct/{stage}.txt"
        if path.exists() and path.read_text() != result.stdout:
            raise SystemExit(f"refusing to replace different accounting: {path}")
        if not path.exists():
            path.write_text(result.stdout)
        summary.append({"stage": stage, "job": job, "records": len(records)})
    print(json.dumps({"status": "PASS", "stages": summary}, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
