#!/usr/bin/env python3
"""Validate and summarize collected same-host FE-BUF-1 SCC pairs."""

from __future__ import annotations

import argparse
import csv
import json
import statistics
from pathlib import Path

BASELINE = "7f5a38b49cccd5bf0396f62fedd542881d68bad1"
CANDIDATE = "1cb441f20be0d747483cd8f81746af4187192d0f"
SIZES = (256, 1024, 4096, 8192, 15625)
ORDERS = ("ab", "ba")
TIMINGS = ("command_s", "work_s", "schur_s", "pcg_s", "rng_s")
EXACT = (
    "processors", "probes", "seed", "iterations", "schur_actions",
    "schur_batches", "precond_apps", "precond_batches", "n_rows", "cells",
    "units", "strata", "workers", "firms", "identity_residual", "r11",
    "r21", "r31", "r41", "r12", "r22", "r32", "r42", "r13", "r23",
    "r33", "r43", "r14", "r24", "r34", "r44", "route", "engine",
    "max_residual",
)


def read_rows(
    path: Path, role: str, commit: str, expected_runs: int
) -> list[dict[str, str]]:
    with path.open(newline="", encoding="utf-8") as handle:
        rows = list(csv.DictReader(handle))
    if len(rows) != expected_runs:
        raise RuntimeError(f"{path}: expected {expected_runs} runs")
    for index, row in enumerate(rows, 1):
        if row["source_label"] != role or row["source_commit"] != commit:
            raise RuntimeError(f"{path}: source binding failed")
        if int(row["run"]) != index or float(row["result_mreldif"]) != 0:
            raise RuntimeError(f"{path}: run order or repeatability failed")
    return rows


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--large-root", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    pairs: list[dict[str, object]] = []
    for firms in SIZES:
        for order in ORDERS:
            evidence_root = (
                args.large_root
                if firms == 15625 and args.large_root is not None
                else args.root
            )
            root = evidence_root / f"F{firms}-P256-{order}"
            expected_runs = 1 if firms == 15625 else 3
            marker = root / "pair.pass"
            if not marker.is_file():
                raise RuntimeError(f"missing pair marker: {marker}")
            left = read_rows(
                root / "baseline.csv", "baseline", BASELINE, expected_runs
            )
            right = read_rows(
                root / "candidate.csv", "candidate", CANDIDATE, expected_runs
            )
            for run, (baseline, candidate) in enumerate(zip(left, right, strict=True), 1):
                for field in EXACT:
                    if baseline[field] != candidate[field]:
                        raise RuntimeError(
                            f"F{firms} {order} run {run}: mismatch {field}"
                        )
                baseline_columns = int(float(baseline["fe_legacy_columns"]))
                candidate_columns = int(float(candidate["fe_buffered_columns"])) + int(
                    float(candidate["fe_legacy_columns"])
                )
                if baseline_columns != candidate_columns:
                    raise RuntimeError(f"F{firms} {order}: Schur accounting failed")
            baseline_timed = left[1:] if expected_runs == 3 else left
            candidate_timed = right[1:] if expected_runs == 3 else right
            base_times = {
                field: statistics.median(float(row[field]) for row in baseline_timed)
                for field in TIMINGS
            }
            cand_times = {
                field: statistics.median(float(row[field]) for row in candidate_timed)
                for field in TIMINGS
            }
            changes = {
                field: 100 * (cand_times[field] / base_times[field] - 1)
                for field in TIMINGS
            }
            pairs.append(
                {
                    "firms": firms,
                    "rows": int(left[0]["n_rows"]),
                    "order": order,
                    "repetitions": expected_runs,
                    "temperature": "warm" if expected_runs == 3 else "cold_single",
                    "baseline_warm_median_seconds": base_times,
                    "candidate_warm_median_seconds": cand_times,
                    "candidate_change_percent": changes,
                    "buffered_columns": int(float(right[-1]["fe_buffered_columns"])),
                    "fallback_columns": int(float(right[-1]["fe_legacy_columns"])),
                    "modeled_workspace_bytes": int(float(right[-1]["fe_workspace_bytes"])),
                    "modeled_cell_bytes_avoided": int(float(right[-1]["fe_avoided_bytes"])),
                    "status": "PASS",
                }
            )
    by_size = {}
    for firms in SIZES:
        selected = [pair for pair in pairs if pair["firms"] == firms]
        by_size[str(firms)] = {
            field: statistics.median(
                pair["candidate_change_percent"][field] for pair in selected
            )
            for field in TIMINGS
        }
    summary = {
        "schema": "vckss-fe-buf1-scc-v1",
        "status": "PASS",
        "baseline": BASELINE,
        "candidate": CANDIDATE,
        "evidence_roots": {
            "standard": str(args.root),
            "f15625": str(args.large_root or args.root),
        },
        "pairs": pairs,
        "median_change_percent_by_size": by_size,
    }
    args.output.write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n")
    print("FE_BUF1_SCC_ANALYSIS_PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
