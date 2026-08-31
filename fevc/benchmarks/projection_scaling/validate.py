#!/usr/bin/env python3
"""Validate one focused projection pair or the 6,000-row exact gate."""

from __future__ import annotations

import argparse
import csv
import json
import math
from pathlib import Path


def csv_row(path: Path) -> dict[str, str]:
    with path.open(newline="", encoding="utf-8") as handle:
        rows = list(csv.DictReader(handle))
    if len(rows) != 1:
        raise ValueError(f"expected one row in {path}, found {len(rows)}")
    return rows[0]


def unit_scaled(left: float, right: float) -> float:
    return abs(left - right) / max(1.0, abs(left), abs(right))


def rel(left: float, right: float) -> float:
    return abs(left - right) / max(1e-300, abs(left), abs(right))


def matrix_rel(left: dict[str, str], right: dict[str, str]) -> float:
    differences = []
    scale = 1e-300
    for i in range(1, 4):
        for j in range(1, 4):
            lvalue = float(left[f"V_{i}_{j}"])
            rvalue = float(right[f"V_{i}_{j}"])
            differences.append(abs(lvalue - rvalue))
            scale = max(scale, abs(lvalue), abs(rvalue))
    return max(differences) / scale


def validate_process(role_dir: Path) -> dict:
    tree = json.loads((role_dir / "process_tree.json").read_text(encoding="utf-8"))
    if tree.get("status") != "PASS" or tree.get("whole_peak_rss_kib", 0) <= 0:
        raise ValueError(f"invalid process tree in {role_dir}")
    if not (role_dir / "resources.txt").is_file():
        raise ValueError(f"missing resource receipt in {role_dir}")
    return tree


def validate_pair(pair_dir: Path) -> dict:
    rust = csv_row(pair_dir / "rust" / "result.csv")
    matlab = json.loads((pair_dir / "matlab" / "result.json").read_text(encoding="utf-8"))
    if rust["status"] != "PASS" or matlab.get("status") != "PASS":
        raise ValueError("application status did not pass")
    rows = int(rust["rows"])
    probes = int(rust["probes"])
    if rows != int(matlab["rows"]) or probes != int(matlab["probes"]):
        raise ValueError("paired dimensions or probe counts changed")
    if rust["source_commit"] != matlab["source_commit"]:
        raise ValueError("paired source identity changed")
    coefficient_relative = max(
        unit_scaled(float(rust[f"b_{term}"]), float(matlab[f"b_{term}"]))
        for term in ("z1", "z2")
    )
    se_relative = max(
        rel(math.sqrt(float(rust[f"V_{index}_{index}"])), float(matlab[f"se_{term}"]))
        for index, term in ((2, "z1"), (3, "z2"))
    )
    covariance_diagonal_relative = max(
        rel(float(rust[f"V_{index}_{index}"]), float(matlab[f"V_{term}_{term}"]))
        for index, term in ((2, "z1"), (3, "z2"))
    )
    if coefficient_relative > 1e-8:
        raise ValueError(f"coefficient gate failed: {coefficient_relative}")
    if se_relative > 0.01:
        raise ValueError(f"standard-error gate failed: {se_relative}")
    if covariance_diagonal_relative > 0.01:
        raise ValueError(f"covariance-diagonal gate failed: {covariance_diagonal_relative}")
    rust_tree = validate_process(pair_dir / "rust")
    matlab_tree = validate_process(pair_dir / "matlab")
    baseline = int((pair_dir / "stata_baseline_rss_kib.txt").read_text().strip())
    if baseline <= 0 or rust_tree["whole_peak_rss_kib"] <= baseline:
        raise ValueError("empty-Stata RSS baseline did not reconcile")
    return {
        "schema": "FEVC-PROJECTION-SCALING-PAIR-VALIDATION-V1",
        "status": "PASS",
        "rows": rows,
        "probes": probes,
        "source_commit": rust["source_commit"],
        "coefficient_max_relative_difference": coefficient_relative,
        "se_max_relative_difference": se_relative,
        "covariance_diagonal_max_relative_difference": covariance_diagonal_relative,
        "rust_whole_peak_rss_kib": rust_tree["whole_peak_rss_kib"],
        "matlab_whole_peak_rss_kib": matlab_tree["whole_peak_rss_kib"],
        "stata_baseline_rss_kib": baseline,
        "rust_incremental_rss_kib": rust_tree["whole_peak_rss_kib"] - baseline,
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--pair", type=Path)
    group.add_argument("--gate", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()

    pair_dir = args.pair or args.gate
    result = validate_pair(pair_dir)
    if args.gate:
        exact = csv_row(args.gate / "exact" / "result.csv")
        rust = csv_row(args.gate / "rust" / "result.csv")
        coefficient_relative = max(
            unit_scaled(float(exact[f"b_{term}"]), float(rust[f"b_{term}"]))
            for term in ("cons", "z1", "z2")
        )
        covariance_relative = matrix_rel(exact, rust)
        matlab = json.loads((args.gate / "matlab" / "result.json").read_text())
        matlab_exact_se_relative = max(
            rel(math.sqrt(float(exact[f"V_{index}_{index}"])), float(matlab[f"se_{term}"]))
            for index, term in ((2, "z1"), (3, "z2"))
        )
        if coefficient_relative > 1e-8:
            raise ValueError(f"exact coefficient gate failed: {coefficient_relative}")
        if covariance_relative > 0.01:
            raise ValueError(f"exact covariance gate failed: {covariance_relative}")
        if matlab_exact_se_relative > 0.01:
            raise ValueError(f"MATLAB exact-SE gate failed: {matlab_exact_se_relative}")
        result.update({
            "schema": "FEVC-PROJECTION-SCALING-GATE-VALIDATION-V1",
            "exact_rust_coefficient_max_relative_difference": coefficient_relative,
            "exact_rust_covariance_matrix_relative_difference": covariance_relative,
            "exact_matlab_se_max_relative_difference": matlab_exact_se_relative,
        })

    args.output.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps(result, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
