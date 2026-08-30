#!/usr/bin/env python3
"""Aggregate the nine accepted paired projection-scaling tasks."""

from __future__ import annotations

import argparse
import csv
import json
import math
import statistics
from pathlib import Path


def csv_row(path: Path) -> dict[str, str]:
    with path.open(newline="", encoding="utf-8") as handle:
        rows = list(csv.DictReader(handle))
    if len(rows) != 1:
        raise ValueError(f"expected one row in {path}")
    return rows[0]


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("run_dir", type=Path)
    parser.add_argument("summary_json", type=Path)
    parser.add_argument("cells_csv", type=Path)
    args = parser.parse_args()

    cells = []
    source_commit = None
    for task_id in range(1, 10):
        task = args.run_dir / "tasks" / f"task-{task_id}"
        validation = json.loads((task / "validation.json").read_text(encoding="utf-8"))
        if validation.get("status") != "PASS":
            raise ValueError(f"task {task_id} validation failed")
        rust = csv_row(task / "rust" / "result.csv")
        matlab = json.loads((task / "matlab" / "result.json").read_text(encoding="utf-8"))
        current_source = rust["source_commit"]
        source_commit = source_commit or current_source
        if current_source != source_commit or matlab["source_commit"] != source_commit:
            raise ValueError("source commit changed across cells")
        rows = int(rust["rows"])
        replicate = 1 + (task_id - 1) % 3
        baseline = validation["stata_baseline_rss_kib"]
        for role, result in (("vckss", rust), ("matlab", matlab)):
            role_name = "rust" if role == "vckss" else "matlab"
            tree = json.loads((task / role_name / "process_tree.json").read_text())
            whole_wall = float((task / role_name / "whole_wall_seconds.txt").read_text())
            peak = int(tree["whole_peak_rss_kib"])
            covariance = {
                f"covariance_{left}_{right}": (
                    float(result[f"V_{left}_{right}"])
                    if role == "vckss"
                    else ""
                )
                for left in range(1, 4)
                for right in range(1, 4)
            }
            if role == "vckss":
                se_z1 = math.sqrt(float(result["V_2_2"]))
                se_z2 = math.sqrt(float(result["V_3_3"]))
            else:
                covariance["covariance_2_2"] = float(result["V_z1_z1"])
                covariance["covariance_3_3"] = float(result["V_z2_z2"])
                se_z1 = float(result["se_z1"])
                se_z2 = float(result["se_z2"])
            cells.append({
                "task_id": task_id,
                "rows": rows,
                "replicate": replicate,
                "role": role,
                "order": (task / "order.txt").read_text().strip(),
                "probes": int(rust["probes"]),
                "command_seconds": float(result["command_seconds"]),
                "whole_wall_seconds": whole_wall,
                "whole_peak_rss_kib": peak,
                "stata_baseline_rss_kib": baseline if role == "vckss" else "",
                "incremental_rss_kib": peak - baseline if role == "vckss" else "",
                "coefficient_max_relative_difference": validation["coefficient_max_relative_difference"],
                "se_max_relative_difference": validation["se_max_relative_difference"],
                "covariance_diagonal_max_relative_difference": validation["covariance_diagonal_max_relative_difference"],
                "se_z1": se_z1,
                "se_z2": se_z2,
                **covariance,
            })

    summaries = []
    for rows in (6000, 24000, 96000):
        for role in ("vckss", "matlab"):
            selected = [cell for cell in cells if cell["rows"] == rows and cell["role"] == role]
            if len(selected) != 3:
                raise ValueError("registered repetition count changed")
            summaries.append({
                "rows": rows,
                "role": role,
                "median_command_seconds": statistics.median(cell["command_seconds"] for cell in selected),
                "median_whole_wall_seconds": statistics.median(cell["whole_wall_seconds"] for cell in selected),
                "median_whole_peak_rss_kib": statistics.median(cell["whole_peak_rss_kib"] for cell in selected),
                "median_incremental_rss_kib": (
                    statistics.median(cell["incremental_rss_kib"] for cell in selected)
                    if role == "vckss" else None
                ),
                "max_coefficient_relative_difference": max(cell["coefficient_max_relative_difference"] for cell in selected),
                "max_se_relative_difference": max(cell["se_max_relative_difference"] for cell in selected),
                "max_covariance_diagonal_relative_difference": max(cell["covariance_diagonal_max_relative_difference"] for cell in selected),
                "covariance_z1_seed_range": max(cell["covariance_2_2"] for cell in selected) - min(cell["covariance_2_2"] for cell in selected),
                "covariance_z2_seed_range": max(cell["covariance_3_3"] for cell in selected) - min(cell["covariance_3_3"] for cell in selected),
            })

    def summary(rows: int, role: str) -> dict:
        return next(item for item in summaries if item["rows"] == rows and item["role"] == role)

    rss_growth = (
        summary(96000, "vckss")["median_incremental_rss_kib"]
        / summary(24000, "vckss")["median_incremental_rss_kib"]
    )
    if rss_growth > 5.5:
        raise ValueError(f"incremental VCkss RSS growth gate failed: {rss_growth}")
    for rows in (6000, 24000, 96000):
        summaries.append({
            "rows": rows,
            "role": "paired_ratio_matlab_over_vckss",
            "median_command_seconds": summary(rows, "matlab")["median_command_seconds"] / summary(rows, "vckss")["median_command_seconds"],
            "median_whole_wall_seconds": summary(rows, "matlab")["median_whole_wall_seconds"] / summary(rows, "vckss")["median_whole_wall_seconds"],
            "median_whole_peak_rss_kib": None,
            "median_incremental_rss_kib": None,
            "max_coefficient_relative_difference": max(summary(rows, role)["max_coefficient_relative_difference"] for role in ("vckss", "matlab")),
            "max_se_relative_difference": max(summary(rows, role)["max_se_relative_difference"] for role in ("vckss", "matlab")),
            "max_covariance_diagonal_relative_difference": max(summary(rows, role)["max_covariance_diagonal_relative_difference"] for role in ("vckss", "matlab")),
            "covariance_z1_seed_range": None,
            "covariance_z2_seed_range": None,
        })

    fields = list(cells[0])
    with args.cells_csv.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=fields, lineterminator="\n")
        writer.writeheader()
        writer.writerows(cells)
    result = {
        "schema": "VCKSS-PROJECTION-SCALING-SUMMARY-V1",
        "status": "PASS",
        "source_commit": source_commit,
        "matlab_upstream_commit": "8b957ffeb10b8465a3584fceb0265cccc48379e1",
        "task_count": 9,
        "cell_count": 6,
        "repetitions_per_cell": 3,
        "vckss_incremental_rss_growth_24000_to_96000": rss_growth,
        "summaries": summaries,
    }
    args.summary_json.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps(result, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
