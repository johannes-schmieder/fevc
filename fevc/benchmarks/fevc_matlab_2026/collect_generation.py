#!/usr/bin/env python3
"""Collect terminal bundle accounting and validate every completed main cell."""

from __future__ import annotations

import argparse
import json
import re
import subprocess
from pathlib import Path
from typing import Any

try:
    from .build_bundles import read_bundles
    from .common import key_values, load_json, read_manifest, require, sha256
except ImportError:
    from build_bundles import read_bundles  # type: ignore
    from common import key_values, load_json, read_manifest, require, sha256  # type: ignore


SCHEMA = "FEVC-MATLAB-2026-GENERATION-INVENTORY-V1"


def task_range(value: str) -> list[int]:
    if re.fullmatch(r"[0-9]+", value):
        return [int(value)]
    match = re.fullmatch(r"([0-9]+)-([0-9]+)", value)
    require(match is not None, "unsupported submission task range")
    assert match is not None
    first, last = map(int, match.groups())
    require(1 <= first <= last <= 24, "bundle task range changed")
    return list(range(first, last + 1))


def qacct_fields(text: str) -> dict[str, str]:
    values: dict[str, str] = {}
    for line in text.splitlines():
        fields = line.strip().split(None, 1)
        if len(fields) == 2:
            values[fields[0]] = fields[1]
    require({"jobnumber", "taskid", "project", "slots", "failed", "exit_status"}
            <= values.keys(), "incomplete qacct record")
    return values


def collect_qacct(job_id: str, expected: list[int], output: Path) -> dict[int, Path]:
    result = subprocess.run(("qacct", "-j", job_id), check=True, text=True,
                            stdout=subprocess.PIPE)
    chunks = [item for item in re.split(
        r"(?=^=+\nqname)", result.stdout, flags=re.MULTILINE) if item.strip()]
    require(len(chunks) == len(expected), "terminal qacct bundle count changed")
    paths: dict[int, Path] = {}
    for chunk in chunks:
        fields = qacct_fields(chunk)
        bundle_id = int(fields["taskid"])
        require(bundle_id in expected and bundle_id not in paths,
                "unexpected or duplicate qacct bundle task")
        path = output / f"bundle-{bundle_id:03d}.txt"
        path.write_text(chunk, encoding="utf-8")
        paths[bundle_id] = path
    require(sorted(paths) == expected, "qacct bundle inventory changed")
    return paths


def inventory(run_dir: Path, attempt_id: str, job_id: str) -> dict[str, Any]:
    identity = load_json(run_dir / "run_identity.json")
    require(identity.get("schema") == "FEVC-MATLAB-2026-STAGED-RUN-V1" and
            identity.get("status") == "PASS", "run identity changed")
    submission = key_values(run_dir / "submissions" / f"{attempt_id}.tsv")
    require(submission.get("schema") == "FEVC-MATLAB-2026-SUBMISSION-V1" and
            submission.get("job_id") == job_id and
            submission.get("attempt_id") == attempt_id,
            "submission identity changed")
    expected_bundles = task_range(submission["task_range"])
    cell_filter = submission["cell_filter"]
    require(cell_filter == "NONE" or cell_filter.isdigit(), "invalid cell filter")

    cells = {int(row["task_id"]): row
             for row in read_manifest(run_dir / "input" / "tasks.tsv")}
    bundles = {int(row["bundle_task_id"]): row
               for row in read_bundles(run_dir / "input" / "bundles.tsv")}
    attempt = run_dir / "attempts" / attempt_id
    qacct_dir = attempt / "qacct"
    validation_dir = attempt / "validations"
    qacct_paths = collect_qacct(job_id, expected_bundles, qacct_dir)
    harness = run_dir / "source" / "fevc" / "benchmarks" / "fevc_matlab_2026"

    validated: list[int] = []
    bundle_hashes: dict[str, str] = {}
    validation_hashes: dict[str, str] = {}
    for bundle_id in expected_bundles:
        fields = qacct_fields(qacct_paths[bundle_id].read_text(encoding="utf-8"))
        require(fields["jobnumber"] == job_id and fields["project"] == "welfgr" and
                fields["slots"] == "28" and fields["failed"] == "0" and
                fields["exit_status"] == "0", "bundle scheduler gate failed")
        bundle_dir = attempt / "bundles" / f"bundle-{bundle_id:03d}"
        receipt = key_values(bundle_dir / "receipt.tsv")
        require(receipt.get("schema") == "FEVC-MATLAB-2026-BUNDLE-RECEIPT-V1" and
                receipt.get("status") == "PASS" and
                int(receipt.get("bundle_task_id", -1)) == bundle_id and
                receipt.get("job_id") == job_id and
                (bundle_dir / "application.pass").is_file(),
                "bundle application gate failed")
        registered_ids = [int(value) for value in
                          bundles[bundle_id]["cell_task_ids"].split(",")]
        selected_ids = registered_ids if cell_filter == "NONE" else [int(cell_filter)]
        require(set(selected_ids) <= set(registered_ids) and
                int(receipt.get("completed_cells", -1)) == len(selected_ids),
                "bundle completed-cell inventory changed")
        bundle_hashes[str(bundle_id)] = sha256(bundle_dir / "receipt.tsv")
        for cell_id in selected_ids:
            validation = validation_dir / f"{cell_id}.json"
            job_dir = attempt / "tasks" / cells[cell_id]["experiment_id"]
            subprocess.run((
                "python3", str(harness / "validate_task.py"),
                "--job-dir", str(job_dir), "--qacct", str(qacct_paths[bundle_id]),
                "--output", str(validation),
            ), check=True)
            payload = load_json(validation)
            require(payload.get("status") == "PASS" and
                    int(payload["task"]["task_id"]) == cell_id,
                    "cell validation changed")
            validated.append(cell_id)
            validation_hashes[str(cell_id)] = sha256(validation)

    return {
        "schema": SCHEMA,
        "status": "PASS",
        "terminal": True,
        "run_id": identity["run_id"],
        "run_kind": identity["run_kind"],
        "attempt_id": attempt_id,
        "job_id": job_id,
        "expected_bundle_ids": expected_bundles,
        "qacct_records": len(expected_bundles),
        "validated_cell_ids": validated,
        "validated_cells": len(validated),
        "cell_filter": cell_filter,
        "bundle_receipt_sha256": bundle_hashes,
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
    require(re.fullmatch(r"[0-9]+", args.job_id) is not None, "invalid job ID")
    require(not args.output.exists(), "generation inventory target exists")
    value = inventory(args.run_dir, args.attempt_id, args.job_id)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n",
                           encoding="utf-8")
    print("FEVC_MATLAB_2026_GENERATION_PASS "
          f"bundles={value['qacct_records']} cells={value['validated_cells']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
