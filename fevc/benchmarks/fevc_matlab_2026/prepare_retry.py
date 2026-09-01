#!/usr/bin/env python3
"""Authorize one exact-source retry for scheduler-failed production bundles."""

from __future__ import annotations

import argparse
import json
import re
from pathlib import Path

try:
    from .collect_generation import qacct_fields, task_range
    from .common import key_values, load_json, require
except ImportError:
    from collect_generation import qacct_fields, task_range  # type: ignore
    from common import key_values, load_json, require  # type: ignore


def prepare(run_dir: Path, requested: str, qacct_path: Path) -> dict[str, object]:
    identity = load_json(run_dir / "run_identity.json")
    require(identity.get("schema") == "FEVC-MATLAB-2026-CAMPAIGN-V2" and
            identity.get("source_mode") == "CLEAN_COMMIT",
            "retry requires a clean source-bound campaign")
    submission = key_values(run_dir / "submissions" / "campaign.tsv")
    production_job = submission.get("production_job_id", "")
    require(production_job.isdigit(), "production job identity changed")
    bundle_ids = task_range(requested)
    require(bundle_ids == list(range(bundle_ids[0], bundle_ids[-1] + 1)),
            "retry bundles must form one contiguous SGE range")
    bundle_range = (str(bundle_ids[0]) if len(bundle_ids) == 1 else
                    f"{bundle_ids[0]}-{bundle_ids[-1]}")

    records: dict[int, dict[str, str]] = {}
    chunks = [item for item in re.split(
        r"(?=^=+\nqname)", qacct_path.read_text(encoding="utf-8"),
        flags=re.MULTILINE) if item.strip()]
    for chunk in chunks:
        fields = qacct_fields(chunk)
        bundle_id = int(fields["taskid"])
        require(bundle_id not in records, "duplicate production qacct bundle")
        records[bundle_id] = fields
    for bundle_id in bundle_ids:
        require(bundle_id in records, "production accounting is incomplete")
        fields = records[bundle_id]
        require(fields["jobnumber"] == production_job,
                "production accounting job changed")
        require(fields["failed"] != "0",
                "retry is limited to scheduler or execution-host failures")

    return {
        "schema": "FEVC-MATLAB-2026-RETRY-PLAN-V1",
        "status": "PASS",
        "run_id": identity["run_id"],
        "source_commit": identity["source_commit"],
        "production_job_id": production_job,
        "retry_bundle_task_ids": bundle_ids,
        "retry_bundle_range": bundle_range,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--run-dir", type=Path, required=True)
    parser.add_argument("--bundle-ids", required=True)
    parser.add_argument("--qacct", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    require(not args.output.exists(), "retry plan already exists")
    value = prepare(args.run_dir, args.bundle_ids, args.qacct)
    args.output.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n",
                           encoding="utf-8")
    print(value["retry_bundle_range"])
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
