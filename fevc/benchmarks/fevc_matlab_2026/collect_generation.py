#!/usr/bin/env python3
"""Collect terminal bundle accounting and validate every completed main cell."""

from __future__ import annotations

import argparse
import json
import os
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
    result: list[int] = []
    for part in value.split(","):
        if re.fullmatch(r"[0-9]+", part):
            result.append(int(part))
            continue
        match = re.fullmatch(r"([0-9]+)-([0-9]+)", part)
        require(match is not None, "unsupported submission task range")
        assert match is not None
        first, last = map(int, match.groups())
        require(first <= last, "bundle task range changed")
        result.extend(range(first, last + 1))
    require(result and len(result) == len(set(result)) and
            all(1 <= item <= 24 for item in result),
            "bundle task range changed")
    return result


def cell_list(value: str) -> list[int]:
    result = [int(part) for part in value.split(",") if part.isdigit()]
    require(result and len(result) == len(value.split(",")) and
            len(result) == len(set(result)) and
            all(1 <= item <= 240 for item in result),
            "cell filter changed")
    return result


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
    paths: dict[int, Path] = {}
    for chunk in chunks:
        fields = qacct_fields(chunk)
        bundle_id = int(fields["taskid"])
        if bundle_id not in expected:
            continue
        require(bundle_id not in paths, "duplicate qacct bundle task")
        path = output / f"bundle-{bundle_id:03d}.txt"
        path.write_text(chunk, encoding="utf-8")
        paths[bundle_id] = path
    require(sorted(paths) == expected, "qacct bundle inventory changed")
    return paths


def inventory(
    run_dir: Path,
    attempt_id: str,
    job_id: str,
    bundle_range: str,
    cell_filter: str,
    skip_bundles: str,
) -> dict[str, Any]:
    identity = load_json(run_dir / "run_identity.json")
    require(identity.get("schema") == "FEVC-MATLAB-2026-CAMPAIGN-V2" and
            identity.get("status") == "PASS", "run identity changed")
    expected_bundles = task_range(bundle_range)
    skipped = set() if skip_bundles == "NONE" else set(task_range(skip_bundles))
    require(skipped <= set(expected_bundles), "skipped bundle is outside range")
    expected_bundles = [item for item in expected_bundles if item not in skipped]
    require(expected_bundles, "no bundles remain after skip")
    filtered_cells = [] if cell_filter == "NONE" else cell_list(cell_filter)

    cells = {int(row["task_id"]): row
             for row in read_manifest(run_dir / "input" / "tasks.tsv")}
    bundles = {int(row["bundle_task_id"]): row
               for row in read_bundles(run_dir / "input" / "bundles.tsv")}
    attempt = run_dir / "attempts" / attempt_id
    qacct_dir = attempt / "qacct"
    validation_dir = attempt / "validations"
    qacct_paths = collect_qacct(job_id, expected_bundles, qacct_dir)
    harness = Path(__file__).resolve().parent
    collector_source_commit = os.environ.get("FEVC_COLLECTION_SOURCE_COMMIT", "")
    if not collector_source_commit:
        collector_source_commit = subprocess.run(
            ("git", "-C", str(harness), "rev-parse", "HEAD"),
            check=True, text=True, stdout=subprocess.PIPE,
        ).stdout.strip()
    require(re.fullmatch(r"[0-9a-f]{40}", collector_source_commit) is not None,
            "collector source identity changed")

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
                receipt.get("cell_filter") == cell_filter and
                (bundle_dir / "application.pass").is_file(),
                "bundle application gate failed")
        registered_ids = [int(value) for value in
                          bundles[bundle_id]["cell_task_ids"].split(",")]
        selected_ids = registered_ids if cell_filter == "NONE" else filtered_cells
        if attempt_id == "pilot":
            require(expected_bundles == [24] and selected_ids == [5, 235],
                    "pilot boundary cells changed")
        else:
            require(set(selected_ids) <= set(registered_ids),
                    "filtered cell is outside bundle")
        require(int(receipt.get("completed_cells", -1)) == len(selected_ids),
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
        "collector_source_commit": collector_source_commit,
        "stage": attempt_id,
        "attempt_id": attempt_id,
        "job_id": job_id,
        "expected_bundle_ids": expected_bundles,
        "skipped_bundle_ids": sorted(skipped),
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
    parser.add_argument("--bundle-range", required=True)
    parser.add_argument("--cell-filter", default="NONE")
    parser.add_argument("--skip-bundles", default="NONE")
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    require(re.fullmatch(r"[A-Za-z0-9._-]+", args.attempt_id) is not None,
            "invalid attempt ID")
    require(re.fullmatch(r"[0-9]+", args.job_id) is not None, "invalid job ID")
    require(not args.output.exists(), "generation inventory target exists")
    value = inventory(
        args.run_dir, args.attempt_id, args.job_id,
        args.bundle_range, args.cell_filter, args.skip_bundles,
    )
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n",
                           encoding="utf-8")
    print("FEVC_MATLAB_2026_GENERATION_PASS "
          f"bundles={value['qacct_records']} cells={value['validated_cells']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
