#!/usr/bin/env python3
"""Validate one task across all five implementations and optional exact oracle."""

from __future__ import annotations

import argparse
import csv
import json
from pathlib import Path
from typing import Any

try:
    from .common import ROLES, TARGETS, atomic_json, exact_tolerance, normalization_factor
except ImportError:
    from common import ROLES, TARGETS, atomic_json, exact_tolerance, normalization_factor


def load_result(role: str, directory: Path) -> dict[str, Any]:
    path = directory / role / ("result.csv" if role == "fevc" else "result.json")
    if role == "fevc":
        with path.open(encoding="utf-8", newline="") as handle:
            rows = list(csv.DictReader(handle))
        if len(rows) != 1:
            raise ValueError("FEVC result row count changed")
        result: dict[str, Any] = rows[0]
    else:
        result = json.loads(path.read_text(encoding="utf-8"))
    return result


def validate(task_dir: Path) -> dict[str, Any]:
    task = json.loads((task_dir / "task.json").read_text(encoding="utf-8"))
    n, cores, algorithm = int(task["rows"]), int(task["cores"]), task["algorithm"]
    results: dict[str, Any] = {}
    for role in ROLES:
        status = json.loads((task_dir / role / "status.json").read_text(encoding="utf-8"))
        if status.get("status") != "PASS":
            raise ValueError(f"{role} wrapper status failed")
        result = load_result(role, task_dir)
        monitor = json.loads((task_dir / role / "process_tree.json").read_text(encoding="utf-8"))
        if result.get("status") != "PASS" or result.get("role") != role:
            raise ValueError(f"{role} result status failed")
        if int(float(result["rows"])) != n or int(float(result["cores"])) != cores:
            raise ValueError(f"{role} dimensions changed")
        if result["algorithm"] != algorithm or int(float(result["retained_rows"])) != n:
            raise ValueError(f"{role} algorithm or sample changed")
        if monitor.get("status") != "PASS" or int(monitor["phase_sample_count"]) < 1:
            raise ValueError(f"{role} phase memory evidence failed")
        expected_factor = normalization_factor(role, n)
        if abs(float(result["normalization_factor"]) - expected_factor) > 1e-15:
            raise ValueError(f"{role} normalization changed")
        targets = {name: float(result[f"normalized_{name}"]) for name in TARGETS}
        if abs(targets["total"]-targets["worker"]-targets["firm"]-2*targets["covariance"]) > 1e-10*(1+sum(map(abs,targets.values()))):
            raise ValueError(f"{role} target identity failed")
        results[role] = {"result":result,"monitor":monitor,"targets":targets}
    exact_checks: list[dict[str, Any]] = []
    if algorithm == "exact":
        oracle = json.loads((task_dir / "oracle.json").read_text(encoding="utf-8"))
        for role in ROLES:
            for target in TARGETS:
                expected = float(oracle["targets"][target])
                observed = results[role]["targets"][target]
                tolerance = exact_tolerance(expected)
                exact_checks.append({"role":role,"target":target,"observed":observed,
                                     "oracle":expected,"absolute_gap":abs(observed-expected),
                                     "tolerance":tolerance,"pass":abs(observed-expected)<=tolerance})
        if not all(item["pass"] for item in exact_checks):
            raise ValueError("exact consistency gate failed")
    return {"schema":"FEVC-FIVE-WAY-TASK-VALIDATION-V1","status":"PASS",
            "cell_id":task["cell_id"],"rows":n,"cores":cores,
            "repeat":int(task["repeat"]),"algorithm":algorithm,
            "roles":results,"exact_checks":exact_checks}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--task-dir", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    value = validate(args.task_dir); atomic_json(args.output,value)
    print(f"FEVC_FIVE_WAY_TASK_VALIDATION_PASS {value['cell_id']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
