#!/usr/bin/env python3
"""Validate the commit-bound local Optimization III batch-width benchmark."""
from __future__ import annotations

import argparse
import csv
import json
import math
import statistics
from pathlib import Path


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def number(row: dict[str, str], field: str) -> float:
    value = float(row[field])
    require(math.isfinite(value), f"nonfinite {field}")
    return value


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--input", type=Path, required=True)
    parser.add_argument("--expected-commit", required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    require(len(args.expected_commit) == 40, "invalid commit")
    with args.input.open(newline="", encoding="utf-8") as handle:
        rows = list(csv.DictReader(handle))
    require([int(number(row, "batch")) for row in rows] == [1, 2, 4, 8, 16],
            "batch sequence changed")
    for row in rows:
        require(row["source_commit"] == args.expected_commit, "commit changed")
        require(number(row, "processors") == 4 and number(row, "probes") == 200,
                "workload changed")
        require(number(row, "schur_actions") > 0 and
                number(row, "schur_batches") > 0, "solver work missing")
        require(number(row, "max_residual") <= 1e-10, "residual failed")
        require(number(row, "result_mreldif") <= 2e-9, "result changed")
        require(number(row, "identity_residual") <= 1e-12, "identity failed")
    reference = next(row for row in rows if int(number(row, "batch")) == 16)
    reference_work = number(reference, "work_s")
    factors = {
        str(int(number(row, "batch"))): number(row, "work_s") / reference_work
        for row in rows
    }
    payload = {
        "validation_version": "KSS-NUMOPT-2-BATCH-V1",
        "status": "PASS",
        "source_commit": args.expected_commit,
        "batch_work_seconds": {
            str(int(number(row, "batch"))): number(row, "work_s") for row in rows
        },
        "work_factor_relative_to_batch_16": factors,
        "median_command_seconds": statistics.median(
            number(row, "command_s") for row in rows
        ),
    }
    args.output.write_text(
        json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    print("KSS_NUMOPT2_BATCH_VALIDATION_PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
