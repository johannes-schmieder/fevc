#!/usr/bin/env python3
"""Build the non-submitting PREP-BND-1 MATLAB comparison task manifest."""

from __future__ import annotations

import argparse
import csv
import hashlib
from pathlib import Path

from common import HEX40, HEX64, TASK_FIELDS, TASK_SCHEMA, require

FIRMS_AND_PROBES = ((64, 20), (256, 20), (1024, 20), (256, 200), (1024, 200))
SEEDS = (104729, 8675309, 20260819)
ORDERS = ("stata_matlab", "matlab_stata")


def build_rows(source_commit: str, bundle_sha256: str) -> list[dict[str, object]]:
    require(HEX40.fullmatch(source_commit) is not None, "invalid source commit")
    require(HEX64.fullmatch(bundle_sha256) is not None, "invalid bundle hash")
    result: list[dict[str, object]] = []
    for firms, probes in FIRMS_AND_PROBES:
        workers = 40 * firms
        for seed in SEEDS:
            for order in ORDERS:
                short_order = "sm" if order == "stata_matlab" else "ms"
                result.append({
                    "task_schema": TASK_SCHEMA,
                    "source_commit": source_commit,
                    "bundle_sha256": bundle_sha256,
                    "experiment_id": f"prep_bnd1_f{firms}_p{probes}_s{seed}_{short_order}",
                    "firms": firms,
                    "workers": workers,
                    "cells_per_worker": 3,
                    "rows": workers * 3,
                    "probes": probes,
                    "seed": seed,
                    "order": order,
                    "requested_slots": 4,
                    "stata_processors": 4,
                    "matlab_pool_workers": 4,
                    "mem_per_core_gib": 14,
                    "hard_wall_seconds": 7200,
                    "sample_contract": "same_literal_rows_v1",
                    "target_contract": "uniform_stored_rows_v1",
                    "comparison_contract": "descriptive_jla_own_pcg_gate_v1",
                })
    require(len(result) == 30, "comparison manifest must have 30 tasks")
    require(len({row["experiment_id"] for row in result}) == len(result),
            "duplicate experiment id")
    return result


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
        require(args.task_dir.is_dir(), "task directory is missing")
        require(not any(args.task_dir.iterdir()), "task directory is not empty")
        for row in rows:
            task_path = args.task_dir / f"{row['experiment_id']}.tsv"
            with task_path.open("w", newline="", encoding="utf-8") as handle:
                writer = csv.DictWriter(handle, fieldnames=TASK_FIELDS,
                                        delimiter="\t", lineterminator="\n")
                writer.writeheader()
                writer.writerow(row)
    print(f"PREP_BND1_MATLAB_MANIFEST_PASS rows={len(rows)} "
          f"sha256={hashlib.sha256(args.output.read_bytes()).hexdigest()}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
