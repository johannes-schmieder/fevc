#!/usr/bin/env python3
"""Validate the matched cold/three-warm Optimization III local benchmark."""
from __future__ import annotations

import argparse
import csv
import json
import math
import re
import statistics
from pathlib import Path

RESULT_FIELDS = [f"r{row}{column}" for column in range(1, 5)
                 for row in range(1, 5)]
FIXED_INTEGER_FIELDS = (
    "processors", "probes", "seed", "iterations", "schur_actions",
    "schur_batches", "precond_apps", "precond_batches", "n_rows", "cells",
    "units", "strata", "workers", "firms",
)


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def read(path: Path, label: str) -> list[dict[str, str]]:
    require(path.is_file(), f"missing {label}: {path}")
    with path.open(newline="", encoding="utf-8") as handle:
        rows = list(csv.DictReader(handle))
    require(len(rows) == 4, f"{label} must contain four runs")
    require([row["run"] for row in rows] == ["1", "2", "3", "4"],
            f"{label} run order changed")
    require([row["temperature"] for row in rows] ==
            ["cold", "warm", "warm", "warm"],
            f"{label} temperature contract changed")
    return rows


def number(row: dict[str, str], field: str) -> float:
    value = float(row[field])
    require(math.isfinite(value), f"nonfinite {field}")
    return value


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--baseline", type=Path, required=True)
    parser.add_argument("--candidate", type=Path, required=True)
    parser.add_argument("--baseline-commit", required=True)
    parser.add_argument("--candidate-source", required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    require(re.fullmatch(r"[0-9a-f]{40}", args.baseline_commit) is not None,
            "baseline must be a Git commit")
    require(re.fullmatch(r"[0-9a-f]{40}(?:[0-9a-f]{24})?",
                         args.candidate_source) is not None,
            "candidate must be a commit or content hash")
    baseline = read(args.baseline, "baseline")
    candidate = read(args.candidate, "candidate")
    require({row["source_label"] for row in baseline} == {"baseline"} and
            {row["source_commit"] for row in baseline} ==
            {args.baseline_commit}, "baseline source binding changed")
    require({row["source_label"] for row in candidate} == {"candidate"} and
            {row["source_commit"] for row in candidate} ==
            {args.candidate_source}, "candidate source binding changed")
    for left, right in zip(baseline, candidate, strict=True):
        for field in FIXED_INTEGER_FIELDS:
            require(number(left, field) == number(right, field),
                    f"workload or solver work changed: {field}")
        require(left["route"] == right["route"] == "DIAGONAL" and
                left["engine"] == right["engine"] == "compressed",
                "route or engine changed")
        require(number(left, "max_residual") <= 1e-9 and
                number(right, "max_residual") <= 1e-9,
                "complete residual failed")
        require(number(left, "identity_residual") <= 1e-12 and
                number(right, "identity_residual") <= 1e-12,
                "target identity failed")
        scale = max(1.0, *(abs(number(left, f)) for f in RESULT_FIELDS),
                    *(abs(number(right, f)) for f in RESULT_FIELDS))
        difference = max(abs(number(left, f) - number(right, f))
                         for f in RESULT_FIELDS)
        require(difference / scale <= 2e-9, "exported estimates changed")
    baseline_warm = [number(row, "command_s") for row in baseline[1:]]
    candidate_warm = [number(row, "command_s") for row in candidate[1:]]
    base_median = statistics.median(baseline_warm)
    candidate_median = statistics.median(candidate_warm)
    improvement = 1 - candidate_median / base_median
    require(improvement >= 0.20, "candidate misses the 20% local threshold")
    output = {
        "validation_version": "KSS-NUMOPT-2-LOCAL-V1",
        "baseline_commit": args.baseline_commit,
        "candidate_source": args.candidate_source,
        "workload": {field: number(baseline[0], field)
                     for field in FIXED_INTEGER_FIELDS},
        "baseline_cold_seconds": number(baseline[0], "command_s"),
        "baseline_warm_seconds": baseline_warm,
        "baseline_warm_median_seconds": base_median,
        "candidate_cold_seconds": number(candidate[0], "command_s"),
        "candidate_warm_seconds": candidate_warm,
        "candidate_warm_median_seconds": candidate_median,
        "median_improvement_fraction": improvement,
        "maximum_result_relative_difference": max(
            abs(number(left, f) - number(right, f)) /
            max(1.0, abs(number(left, f)), abs(number(right, f)))
            for left, right in zip(baseline, candidate, strict=True)
            for f in RESULT_FIELDS
        ),
        "status": "PASS",
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(output, indent=2, sort_keys=True) + "\n",
                           encoding="utf-8")
    print("KSS_NUMOPT2_LOCAL_VALIDATION_PASS "
          f"improvement={100*improvement:.2f}%")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
