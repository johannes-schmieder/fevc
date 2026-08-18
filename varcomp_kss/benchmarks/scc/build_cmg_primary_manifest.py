#!/usr/bin/env python3
"""Build the deterministic CMG-MATA-1 matched SCC primary task manifest."""
from __future__ import annotations

import argparse
import csv
import hashlib
import re
from pathlib import Path

HEX40 = re.compile(r"[0-9a-f]{40}")
HEX64 = re.compile(r"[0-9a-f]{64}")
SCALES = ((1024, 3), (256, 3), (64, 2), (32, 1), (16, 1))
FIELDS = (
    "task_version", "source_commit", "bundle_sha256", "experiment_id",
    "repetition", "firms", "workers", "cells_per_worker",
    "rows_per_cell", "connectivity", "probes", "seed", "batch",
    "stata_hard_wall_seconds", "stata_slots", "stata_mem_per_core_gib",
    "stata_processors", "matlab_hard_wall_seconds",
    "matlab_mem_per_core_gib", "label",
)


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--source-commit", required=True)
    parser.add_argument("--bundle", required=True)
    args = parser.parse_args()
    require(HEX40.fullmatch(args.source_commit) is not None,
            "invalid source commit")
    require(HEX64.fullmatch(args.bundle) is not None, "invalid bundle hash")
    require(not args.output.exists(), "manifest target already exists")
    require(args.output.parent.is_dir() and not args.output.parent.is_symlink(),
            "manifest parent must be a real directory")

    rows: list[dict[str, str | int]] = []
    for firms, repetitions in SCALES:
        for degree in range(2, 8):
            for rows_per_cell in (1, 8):
                for repetition in range(1, repetitions + 1):
                    experiment = (
                        f"cmg1_f{firms}_d{degree}_r{rows_per_cell}_"
                        f"p20_rep{repetition}"
                    )
                    rows.append({
                        "task_version": "CMG-MATA-1-PRIMARY-TASK-V1",
                        "source_commit": args.source_commit,
                        "bundle_sha256": args.bundle,
                        "experiment_id": experiment,
                        "repetition": repetition,
                        "firms": firms,
                        "workers": 40 * firms,
                        "cells_per_worker": degree,
                        "rows_per_cell": rows_per_cell,
                        "connectivity": "strong",
                        "probes": 20,
                        "seed": 8675309,
                        "batch": "auto",
                        "stata_hard_wall_seconds": 1800,
                        "stata_slots": 14,
                        "stata_mem_per_core_gib": 4,
                        "stata_processors": 4,
                        "matlab_hard_wall_seconds": 7200,
                        "matlab_mem_per_core_gib": 32,
                        "label": "cmg-mata1-primary",
                    })
    require(len(rows) == 120, "primary manifest must contain 120 matched rows")
    experiments = [str(row["experiment_id"]) for row in rows]
    require(len(set(experiments)) == len(experiments),
            "duplicate primary experiment")

    temporary = args.output.with_name(args.output.name + ".tmp")
    require(not temporary.exists(), "temporary manifest target already exists")
    with temporary.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=FIELDS, delimiter="\t",
                                lineterminator="\n")
        writer.writeheader()
        writer.writerows(rows)
    temporary.replace(args.output)
    digest = hashlib.sha256(args.output.read_bytes()).hexdigest()
    print(f"CMG_PRIMARY_MANIFEST_PASS rows={len(rows)} sha256={digest}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
