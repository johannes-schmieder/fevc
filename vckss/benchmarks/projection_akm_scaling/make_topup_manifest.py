#!/usr/bin/env python3
"""Admit replications two and three only for eligible passing cells."""

from __future__ import annotations

import argparse
import json
from pathlib import Path

from common import CORES, ROWS, topup_tasks, write_manifest


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("run_dir", type=Path)
    parser.add_argument("rows", type=int, choices=ROWS)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    for rows in ROWS:
        if not (args.run_dir / "receipts" / f"feasibility-{rows}.pass").is_file():
            raise ValueError("all feasibility stages must pass before top-up selection")
    eligible: list[tuple[int, int]] = []
    for task_id, cores in enumerate(CORES, 1):
        path = args.run_dir / "tasks" / f"feasibility-{args.rows}" / f"task-{task_id}" / "validation.json"
        value = json.loads(path.read_text(encoding="utf-8"))
        if value.get("status") != "PASS" or int(value["cores"]) != cores:
            raise ValueError("passing feasibility identity changed")
        if float(value["pair_whole_wall_seconds"]) <= 36_000:
            eligible.append((args.rows, cores))
    values = topup_tasks(eligible)
    if values:
        write_manifest(args.output, values)
    result = {
        "schema": "VCKSS-PROJECTION-AKM-TOPUP-SELECTION-V1",
        "status": "PASS" if values else "NO_ELIGIBLE_CELLS",
        "rows": args.rows,
        "eligible_cores": [cores for _, cores in eligible],
        "task_count": len(values),
        "pair_wall_eligibility_seconds": 36_000,
    }
    args.output.with_suffix(".json").write_text(
        json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    print(json.dumps(result, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
