#!/usr/bin/env python3
"""Aggregate staged AKM projection evidence without imputing censored runs."""

from __future__ import annotations

import argparse
import csv
import json
import math
import statistics
from pathlib import Path
from typing import Any

from common import CORES, ROWS, UPSTREAM_COMMIT, read_manifest
from validate import csv_row, key_values


ROLES = ("vckss", "matlab")


def submitted_stages(run_dir: Path) -> list[str]:
    stages: list[str] = []
    for prefix in ("feasibility", "topup"):
        for rows in ROWS:
            stage = f"{prefix}-{rows}"
            if (run_dir / "submissions" / f"{stage}.txt").is_file():
                if not (run_dir / "receipts" / f"{stage}.complete").is_file():
                    raise ValueError(f"submitted stage is not finalized: {stage}")
                stages.append(stage)
    return stages


def base_cell(rows: int, cores: int, replicate: int, role: str, source: str) -> dict[str, Any]:
    return {
        "rows": rows,
        "workers": rows // 6,
        "firms": rows // 12,
        "cores": cores,
        "replicate": replicate,
        "role": role,
        "status": "NOT_RUN",
        "source_commit": source,
        "stage": "",
        "task_id": "",
        "task_sha256": "",
        "input_sha256": "",
        "order": "",
        "probes": "",
        "seed": "",
        "command_seconds": "",
        "whole_wall_seconds": "",
        "whole_peak_rss_kib": "",
        "stata_baseline_rss_kib": "",
        "incremental_rss_kib": "",
        "coefficient_max_unit_scaled_difference": "",
        "se_max_relative_difference": "",
        "covariance_diagonal_max_relative_difference": "",
        "se_z1": "",
        "se_z2": "",
        "projection_iterations": "",
        "route_hierarchy_levels": "",
        "route_terminal_vertices": "",
        "complete_residual": "",
        "residual_tolerance": "",
        "probe_throughput": "",
        "failure_code": "NOT_SUBMITTED",
    }


def load_captured_task(
    run_dir: Path,
    stage: str,
    task_id: int,
    task: dict[str, str],
    source: str,
) -> list[dict[str, Any]]:
    task_dir = run_dir / "tasks" / stage / f"task-{task_id}"
    validation = json.loads((task_dir / "validation.json").read_text(encoding="utf-8"))
    if validation.get("task_sha256") != task["task_sha256"]:
        raise ValueError(f"validation identity changed for {stage}/{task_id}")
    baseline = int((task_dir / "stata_baseline_rss_kib.txt").read_text().strip())
    values: list[dict[str, Any]] = []
    for public_role, directory_role in (("vckss", "rust"), ("matlab", "matlab")):
        status = key_values(task_dir / directory_role / "status.tsv")
        tree = json.loads((task_dir / directory_role / "process_tree.json").read_text())
        cell = base_cell(int(task["rows"]), int(task["cores"]), int(task["replicate"]), public_role, source)
        cell.update({
            "status": status["outcome"],
            "stage": stage,
            "task_id": task_id,
            "task_sha256": task["task_sha256"],
            "input_sha256": status["input_sha256"],
            "order": task["order"],
            "probes": int(task["probes"]),
            "seed": int(task["seed"]),
            "whole_wall_seconds": float(status["whole_wall_seconds"]),
            "whole_peak_rss_kib": int(tree["whole_peak_rss_kib"]),
            "stata_baseline_rss_kib": baseline if public_role == "vckss" else "",
            "incremental_rss_kib": int(tree["whole_peak_rss_kib"]) - baseline if public_role == "vckss" else "",
            "failure_code": "NONE" if status["outcome"] == "PASS" else (
                "ROLE_TIME_CAP" if status["outcome"] == "RIGHT_CENSORED" else "APPLICATION_FAILURE"
            ),
        })
        if validation.get("status") == "PASS":
            for field in (
                "coefficient_max_unit_scaled_difference",
                "se_max_relative_difference",
                "covariance_diagonal_max_relative_difference",
            ):
                cell[field] = validation[field]
        if status["outcome"] == "PASS" and public_role == "vckss":
            result = csv_row(task_dir / "rust" / "result.csv")
            cell.update({
                "command_seconds": float(result["command_seconds"]),
                "se_z1": math.sqrt(float(result["V_2_2"])),
                "se_z2": math.sqrt(float(result["V_3_3"])),
                "projection_iterations": int(float(result["projection_solver_iterations"])),
                "route_hierarchy_levels": int(float(result["route_hierarchy_levels"])),
                "route_terminal_vertices": int(float(result["route_terminal_vertices"])),
                "complete_residual": float(result["projection_complete_residual"]),
                "residual_tolerance": float(result["residual_tolerance"]),
                "probe_throughput": float(result["probe_throughput"]),
            })
        if status["outcome"] == "PASS" and public_role == "matlab":
            result = json.loads((task_dir / "matlab" / "result.json").read_text())
            cell.update({
                "command_seconds": float(result["command_seconds"]),
                "se_z1": float(result["se_z1"]),
                "se_z2": float(result["se_z2"]),
                "projection_iterations": int(result["fit_iterations"]),
                "complete_residual": float(result["complete_fit_residual"]),
                "residual_tolerance": 1e-10,
                "probe_throughput": float(result["probe_throughput"]),
            })
        values.append(cell)
    return values


def summaries(cells: list[dict[str, Any]]) -> list[dict[str, Any]]:
    values: list[dict[str, Any]] = []
    for rows in ROWS:
        for cores in CORES:
            selected = [cell for cell in cells if cell["rows"] == rows and cell["cores"] == cores]
            for role in ROLES:
                passed = [cell for cell in selected if cell["role"] == role and cell["status"] == "PASS"]
                value: dict[str, Any] = {
                    "rows": rows,
                    "cores": cores,
                    "role": role,
                    "successful_repetitions": len(passed),
                    "rankable": len(passed) == 3,
                    "median_command_seconds": None,
                    "median_whole_wall_seconds": None,
                    "median_peak_rss_kib": None,
                    "median_incremental_rss_kib": None,
                }
                if len(passed) == 3:
                    value.update({
                        "median_command_seconds": statistics.median(item["command_seconds"] for item in passed),
                        "median_whole_wall_seconds": statistics.median(item["whole_wall_seconds"] for item in passed),
                        "median_peak_rss_kib": statistics.median(item["whole_peak_rss_kib"] for item in passed),
                        "median_incremental_rss_kib": statistics.median(item["incremental_rss_kib"] for item in passed) if role == "vckss" else None,
                    })
                values.append(value)
            vckss = values[-2]
            matlab = values[-1]
            if vckss["rankable"] and matlab["rankable"]:
                values.append({
                    "rows": rows,
                    "cores": cores,
                    "role": "paired_ratio_matlab_over_vckss",
                    "successful_repetitions": 3,
                    "rankable": True,
                    "median_command_seconds": matlab["median_command_seconds"] / vckss["median_command_seconds"],
                    "median_whole_wall_seconds": matlab["median_whole_wall_seconds"] / vckss["median_whole_wall_seconds"],
                    "median_peak_rss_kib": None,
                    "median_incremental_rss_kib": None,
                })
    return values


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("run_dir", type=Path)
    parser.add_argument("summary_json", type=Path)
    parser.add_argument("cells_csv", type=Path)
    args = parser.parse_args()
    source = (args.run_dir / "source" / "SOURCE_COMMIT.txt").read_text().strip()
    stages = submitted_stages(args.run_dir)
    indexed: dict[tuple[int, int, int, str], dict[str, Any]] = {}
    for rows in ROWS:
        for cores in CORES:
            for replicate in (1, 2, 3):
                for role in ROLES:
                    indexed[(rows, cores, replicate, role)] = base_cell(rows, cores, replicate, role, source)
    for stage in stages:
        manifest = read_manifest(args.run_dir / "manifests" / f"{stage}.tsv")
        for task_id, task in enumerate(manifest, 1):
            for cell in load_captured_task(args.run_dir, stage, task_id, task, source):
                indexed[(cell["rows"], cell["cores"], cell["replicate"], cell["role"])] = cell
    cells = [indexed[key] for key in sorted(indexed)]
    with args.cells_csv.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=list(cells[0]), lineterminator="\n")
        writer.writeheader()
        writer.writerows(cells)
    status_counts = {status: sum(cell["status"] == status for cell in cells) for status in ("PASS", "FAIL", "RIGHT_CENSORED", "NOT_RUN")}
    result = {
        "schema": "VCKSS-PROJECTION-AKM-SUMMARY-V1",
        "status": "COMPLETE",
        "source_commit": source,
        "matlab_upstream_commit": UPSTREAM_COMMIT,
        "submitted_stages": stages,
        "registered_cell_count": 6,
        "registered_role_run_count": 36,
        "status_counts": status_counts,
        "one_repetition_is_indicative_only": True,
        "three_repetitions_required_for_medians": True,
        "right_censor_role_cap_seconds_7680000": 43_200,
        "summaries": summaries(cells),
    }
    args.summary_json.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps(result, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
