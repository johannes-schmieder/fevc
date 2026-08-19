#!/usr/bin/env python3
"""Validate and summarize interleaved same-host FE-BUF-1 CZ18 pairs."""

from __future__ import annotations

import argparse
import csv
import json
import statistics
from pathlib import Path

BASELINE = "7f5a38b49cccd5bf0396f62fedd542881d68bad1"
CANDIDATE = "1cb441f20be0d747483cd8f81746af4187192d0f"
INPUT_SHA = "1748ca2a6a46f248e05c0329407e7e7708ec7628c1ffce5f0e06ee264bdf0575"
ORDERS = ("ab", "ba")
TIMINGS = (
    "command_s",
    "selection_s",
    "work_s",
    "schur_s",
    "pcg_s",
    "leverage_s",
    "target_s",
    "correction_s",
    "rng_s",
)
EXACT = (
    "processors",
    "probes",
    "seed",
    "input_sha256",
    "selected_batch",
    "route",
    "engine",
    "estimator_status",
    "iterations",
    "schur_actions",
    "schur_batches",
    "precond_apps",
    "precond_batches",
    "max_residual",
    "acceptance",
    "n_rows",
    "n_retained",
    "cells",
    "units",
    "strata",
    "workers",
    "firms",
    "identity_residual",
    "sample_count",
    "life_sample_restored",
    "data_restored",
    "rng_restored",
    "sort_rng_restored",
    "r11",
    "r21",
    "r31",
    "r41",
    "r12",
    "r22",
    "r32",
    "r42",
    "r13",
    "r23",
    "r33",
    "r43",
    "r14",
    "r24",
    "r34",
    "r44",
)


def read_row(path: Path, role: str, commit: str, order: str, round_: int) -> dict[str, str]:
    with path.open(newline="", encoding="utf-8") as handle:
        rows = list(csv.DictReader(handle))
    if len(rows) != 1:
        raise RuntimeError(f"{path}: expected one result row")
    row = rows[0]
    if (
        row["source_label"] != role
        or row["source_commit"] != commit
        or row["pair_order"] != order
        or int(row["run"]) != round_
        or row["input_sha256"] != INPUT_SHA
    ):
        raise RuntimeError(f"{path}: source, input, order, or round binding failed")
    return row


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    pairs: list[dict[str, object]] = []
    for order in ORDERS:
        root = args.root / f"CZ18-P20-{order}"
        if not (root / "pair.pass").is_file():
            raise RuntimeError(f"missing pair marker: {root / 'pair.pass'}")
        for round_ in range(1, 4):
            baseline = read_row(
                root / f"round{round_}-baseline/summary.csv",
                "baseline",
                BASELINE,
                order,
                round_,
            )
            candidate = read_row(
                root / f"round{round_}-candidate/summary.csv",
                "candidate",
                CANDIDATE,
                order,
                round_,
            )
            for field in EXACT:
                if baseline[field] != candidate[field]:
                    raise RuntimeError(
                        f"CZ18 {order} round {round_}: mismatch {field}: "
                        f"{baseline[field]!r} != {candidate[field]!r}"
                    )
            baseline_columns = int(float(baseline["fe_legacy_columns"]))
            candidate_columns = int(float(candidate["fe_buffered_columns"])) + int(
                float(candidate["fe_legacy_columns"])
            )
            if baseline_columns != candidate_columns:
                raise RuntimeError(f"CZ18 {order} round {round_}: Schur accounting failed")
            base_times = {field: float(baseline[field]) for field in TIMINGS}
            cand_times = {field: float(candidate[field]) for field in TIMINGS}
            pairs.append(
                {
                    "order": order,
                    "round": round_,
                    "baseline_seconds": base_times,
                    "candidate_seconds": cand_times,
                    "candidate_change_percent": {
                        field: 100 * (cand_times[field] / base_times[field] - 1)
                        for field in TIMINGS
                    },
                    "buffered_columns": int(float(candidate["fe_buffered_columns"])),
                    "fallback_columns": int(float(candidate["fe_legacy_columns"])),
                    "fallback_batches": int(float(candidate["fe_fallback_batches"])),
                    "modeled_workspace_bytes": int(float(candidate["fe_workspace_bytes"])),
                    "modeled_cell_bytes_avoided": int(float(candidate["fe_avoided_bytes"])),
                    "status": "PASS",
                }
            )
    summary = {
        "schema": "varcomp-kss-fe-buf1-cz18-scc-v1",
        "status": "PASS",
        "baseline": BASELINE,
        "candidate": CANDIDATE,
        "input_sha256": INPUT_SHA,
        "pairs": pairs,
        "median_change_percent": {
            field: statistics.median(
                pair["candidate_change_percent"][field] for pair in pairs
            )
            for field in TIMINGS
        },
        "warm_median_change_percent": {
            field: statistics.median(
                pair["candidate_change_percent"][field]
                for pair in pairs
                if pair["round"] >= 2
            )
            for field in TIMINGS
        },
    }
    args.output.write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n")
    print("FE_BUF1_CZ18_ANALYSIS_PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
