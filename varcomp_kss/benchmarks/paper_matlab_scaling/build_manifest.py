#!/usr/bin/env python3
"""Build the frozen 120-task paper Stata--MATLAB scaling manifest."""

from __future__ import annotations

import argparse
import csv
import hashlib
from pathlib import Path

try:
    from .common import (
        HEX40,
        HEX64,
        ORDERS,
        ROW_GRID,
        SEEDS,
        STRUCTURES,
        TASK_FIELDS,
        TASK_SCHEMA,
        require,
    )
except ImportError:  # Direct script execution on the SCC.
    from common import (
        HEX40,
        HEX64,
        ORDERS,
        ROW_GRID,
        SEEDS,
        STRUCTURES,
        TASK_FIELDS,
        TASK_SCHEMA,
        require,
    )


def build_rows(source_commit: str, bundle_sha256: str) -> list[dict[str, object]]:
    require(HEX40.fullmatch(source_commit) is not None, "invalid source commit")
    require(HEX64.fullmatch(bundle_sha256) is not None, "invalid bundle hash")
    rows: list[dict[str, object]] = []
    for structure, (connectivity, degree) in STRUCTURES.items():
        for row_count in ROW_GRID:
            workers = row_count // degree
            firms = workers // 40
            require(row_count == workers * degree and workers == 40 * firms,
                    "registered row grid is not divisible")
            for seed in SEEDS:
                for order in ORDERS:
                    short_order = "sm" if order == "stata_matlab" else "ms"
                    rows.append({
                        "task_schema": TASK_SCHEMA,
                        "source_commit": source_commit,
                        "bundle_sha256": bundle_sha256,
                        "experiment_id": (
                            f"paper_{structure}_n{row_count}_s{seed}_{short_order}"
                        ),
                        "structure": structure,
                        "connectivity": connectivity,
                        "cells_per_worker": degree,
                        "rows": row_count,
                        "workers": workers,
                        "firms": firms,
                        "probes": 200,
                        "seed": seed,
                        "order": order,
                        "requested_slots": 4,
                        "stata_processors": 4,
                        "matlab_pool_workers": 4,
                        "mem_per_core_gib": 14,
                        "hard_wall_seconds": 28800,
                        "sample_contract": "same_literal_match_rows_v1",
                        "target_contract": "uniform_stored_rows_v1",
                        "comparison_contract": "descriptive_p200_command_time_v1",
                    })
    require(len(rows) == 120, "paper matrix must have 120 tasks")
    require(len({row["experiment_id"] for row in rows}) == 120,
            "duplicate experiment id")
    return rows


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--task-dir", type=Path)
    parser.add_argument("--source-commit", required=True)
    parser.add_argument("--bundle", required=True)
    args = parser.parse_args()
    require(not args.output.exists(), "manifest target already exists")
    require(args.output.parent.is_dir(), "manifest parent is missing")
    rows = build_rows(args.source_commit, args.bundle)
    with args.output.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=TASK_FIELDS, delimiter="\t",
                                lineterminator="\n")
        writer.writeheader()
        writer.writerows(rows)
    if args.task_dir is not None:
        require(args.task_dir.is_dir() and not any(args.task_dir.iterdir()),
                "task directory must be empty")
        for row in rows:
            task = args.task_dir / f"{row['experiment_id']}.tsv"
            with task.open("w", newline="", encoding="utf-8") as handle:
                writer = csv.DictWriter(handle, fieldnames=TASK_FIELDS,
                                        delimiter="\t", lineterminator="\n")
                writer.writeheader()
                writer.writerow(row)
    digest = hashlib.sha256(args.output.read_bytes()).hexdigest()
    print(f"PAPER_MATLAB_SCALING_MANIFEST_PASS rows=120 sha256={digest}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
