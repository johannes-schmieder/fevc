#!/usr/bin/env python3
"""Aggregate all terminal paired projection-scaling tasks.

Successful runs retain the original acceptance checks. Terminal failed runs
are reported rather than mistaken for missing evidence: every registered role
receives a PASS, FAIL, or NOT_RUN row and the summary status is FAIL.
"""

from __future__ import annotations

import argparse
import csv
import json
import math
import re
import statistics
from pathlib import Path
from typing import Any


SOURCE_RE = re.compile(r"^[0-9a-f]{40}$")
ROWS = (6000, 24000, 96000)
ROLES = ("fevc", "matlab")
UPSTREAM_COMMIT = "8b957ffeb10b8465a3584fceb0265cccc48379e1"


def csv_row(path: Path) -> dict[str, str]:
    with path.open(newline="", encoding="utf-8") as handle:
        rows = list(csv.DictReader(handle))
    if len(rows) != 1:
        raise ValueError(f"expected one row in {path}")
    return rows[0]


def expected_layout(task_id: int) -> tuple[int, int, int, str]:
    rows = ROWS[(task_id - 1) // 3]
    replicate = 1 + (task_id - 1) % 3
    probes = math.ceil(math.log2(rows) / 0.01)
    order = (
        "rust_matlab"
        if (((task_id - 1) // 3) + replicate) % 2 == 0
        else "matlab_rust"
    )
    return rows, replicate, probes, order


def optional_float(path: Path) -> float | None:
    if not path.is_file():
        return None
    return float(path.read_text(encoding="utf-8").strip())


def optional_tree(path: Path) -> dict[str, Any] | None:
    if not path.is_file():
        return None
    return json.loads(path.read_text(encoding="utf-8"))


def parse_qacct(path: Path) -> dict[int, dict[str, str]]:
    if not path.is_file():
        raise ValueError("missing complete array qacct receipt")
    records: dict[int, dict[str, str]] = {}
    current: int | None = None
    for raw in path.read_text(encoding="utf-8").splitlines():
        line = raw.strip()
        if line.startswith("TASK="):
            current = int(line.split("=", 1)[1])
            records[current] = {}
            continue
        if current is None or not line:
            continue
        key, _, value = line.partition(" ")
        if key in {
            "jobnumber",
            "taskid",
            "failed",
            "exit_status",
            "hostname",
            "qname",
            "ru_wallclock",
            "ru_maxrss",
            "maxvmem",
        }:
            records[current][key] = value.strip()
    if set(records) != set(range(1, 10)):
        raise ValueError(f"incomplete array qacct tasks: {sorted(records)}")
    for task_id, record in records.items():
        if "jobnumber" not in record:
            raise ValueError(f"missing qacct job number for task {task_id}")
        if int(record.get("taskid", -1)) != task_id:
            raise ValueError(f"qacct task identity failed for task {task_id}")
        if record.get("failed") != "0":
            raise ValueError(f"scheduler failure for task {task_id}")
        if "exit_status" not in record:
            raise ValueError(f"missing qacct exit status for task {task_id}")
    return records


def base_cell(
    task_id: int,
    role: str,
    status: str,
    source_commit: str,
    baseline: int | None,
) -> dict[str, Any]:
    rows, replicate, probes, order = expected_layout(task_id)
    seed = 20260830 + replicate
    return {
        "task_id": task_id,
        "rows": rows,
        "replicate": replicate,
        "role": role,
        "status": status,
        "order": order,
        "probes": probes,
        "seed": seed,
        "source_commit": source_commit,
        "command_seconds": "",
        "whole_wall_seconds": "",
        "whole_peak_rss_kib": "",
        "stata_baseline_rss_kib": (
            baseline if role == "fevc" and baseline is not None else ""
        ),
        "incremental_rss_kib": "",
        "coefficient_max_relative_difference": "",
        "se_max_relative_difference": "",
        "covariance_diagonal_max_relative_difference": "",
        "se_z1": "",
        "se_z2": "",
        "failure_code": "",
        "failure_message": "",
        "solver_iterations": "",
        "solver_max_iterations": "",
        "reported_residual": "",
        "residual_kind": "",
        **{
            f"covariance_{left}_{right}": ""
            for left in range(1, 4)
            for right in range(1, 4)
        },
    }


def pass_cells(
    task_id: int, task: Path, source_commit: str
) -> list[dict[str, Any]]:
    rows, _, probes, order = expected_layout(task_id)
    validation = json.loads(
        (task / "validation.json").read_text(encoding="utf-8")
    )
    if validation.get("status") != "PASS":
        raise ValueError(f"task {task_id} validation failed")
    if int(validation["rows"]) != rows or int(validation["probes"]) != probes:
        raise ValueError(f"task {task_id} validation dimensions changed")
    if validation["source_commit"] != source_commit:
        raise ValueError(f"task {task_id} validation source changed")
    actual_order = (task / "order.txt").read_text(encoding="utf-8").strip()
    if actual_order != order or not (task / "pair.pass").is_file():
        raise ValueError(f"task {task_id} order or pair receipt changed")

    rust = csv_row(task / "rust" / "result.csv")
    matlab = json.loads(
        (task / "matlab" / "result.json").read_text(encoding="utf-8")
    )
    if rust["status"] != "PASS" or matlab.get("status") != "PASS":
        raise ValueError(f"task {task_id} application status changed")
    if (
        rust["source_commit"] != source_commit
        or matlab["source_commit"] != source_commit
    ):
        raise ValueError(f"task {task_id} result source changed")
    if int(rust["rows"]) != rows or int(matlab["rows"]) != rows:
        raise ValueError(f"task {task_id} result rows changed")
    if int(rust["probes"]) != probes or int(matlab["probes"]) != probes:
        raise ValueError(f"task {task_id} result probes changed")

    baseline = int(
        (task / "stata_baseline_rss_kib.txt").read_text(encoding="utf-8").strip()
    )
    cells: list[dict[str, Any]] = []
    for role, result in (("fevc", rust), ("matlab", matlab)):
        role_name = "rust" if role == "fevc" else "matlab"
        tree = json.loads(
            (task / role_name / "process_tree.json").read_text(encoding="utf-8")
        )
        if tree.get("status") != "PASS" or int(
            tree.get("whole_peak_rss_kib", 0)
        ) <= 0:
            raise ValueError(f"task {task_id} {role} process tree failed")
        peak = int(tree["whole_peak_rss_kib"])
        cell = base_cell(task_id, role, "PASS", source_commit, baseline)
        cell.update(
            {
                "command_seconds": float(result["command_seconds"]),
                "whole_wall_seconds": float(
                    (task / role_name / "whole_wall_seconds.txt").read_text()
                ),
                "whole_peak_rss_kib": peak,
                "incremental_rss_kib": peak - baseline if role == "fevc" else "",
                "coefficient_max_relative_difference": validation[
                    "coefficient_max_relative_difference"
                ],
                "se_max_relative_difference": validation[
                    "se_max_relative_difference"
                ],
                "covariance_diagonal_max_relative_difference": validation[
                    "covariance_diagonal_max_relative_difference"
                ],
            }
        )
        if role == "fevc":
            cell["se_z1"] = math.sqrt(float(result["V_2_2"]))
            cell["se_z2"] = math.sqrt(float(result["V_3_3"]))
            for left in range(1, 4):
                for right in range(1, 4):
                    cell[f"covariance_{left}_{right}"] = float(
                        result[f"V_{left}_{right}"]
                    )
        else:
            cell["se_z1"] = float(result["se_z1"])
            cell["se_z2"] = float(result["se_z2"])
            cell["covariance_2_2"] = float(result["V_z1_z1"])
            cell["covariance_3_3"] = float(result["V_z2_z2"])
        cells.append(cell)
    return cells


def failure_detail(role: str, application: str) -> dict[str, Any]:
    if role == "fevc":
        match = re.search(
            r"did not converge in ([0-9]+) iterations; "
            r"final reduced residual was "
            r"([0-9]+(?:\.[0-9]+)?(?:[eE][+-]?[0-9]+)?)",
            application,
        )
        if match:
            return {
                "failure_code": "PCG_MAXITER",
                "failure_message": "VCkss diagonal model PCG did not converge.",
                "solver_iterations": int(match.group(1)),
                "solver_max_iterations": int(match.group(1)),
                "reported_residual": float(match.group(2)),
                "residual_kind": "reduced_model",
            }
    else:
        matches = re.findall(
            r"iterate returned \(number ([0-9]+)\) has relative residual "
            r"([0-9]+(?:\.[0-9]+)?(?:[eE][+-]?[0-9]+)?)",
            application,
        )
        if "Grounded fit did not converge" in application and matches:
            iteration, residual = matches[-1]
            return {
                "failure_code": "MATLAB_FIT_PCG_MAXITER",
                "failure_message": (
                    "Maintained-MATLAB grounded fit PCG did not pass the "
                    "registered tolerance."
                ),
                "solver_iterations": int(iteration),
                "solver_max_iterations": 1000,
                "reported_residual": float(residual),
                "residual_kind": "grounded_normal_equation",
            }
    return {
        "failure_code": "APPLICATION_FAILURE",
        "failure_message": f"{role} application failed before a validated result.",
        "solver_iterations": "",
        "solver_max_iterations": "",
        "reported_residual": "",
        "residual_kind": "",
    }


def failed_cells(
    task_id: int, task: Path, source_commit: str
) -> list[dict[str, Any]]:
    _, _, _, order = expected_layout(task_id)
    first_role_name, second_role_name = order.split("_")
    role_map = {"rust": "fevc", "matlab": "matlab"}
    first_role = role_map[first_role_name]
    second_role = role_map[second_role_name]
    baseline_path = task / "stata_baseline_rss_kib.txt"
    baseline = (
        int(baseline_path.read_text(encoding="utf-8").strip())
        if baseline_path.is_file()
        else None
    )
    first_dir = task / first_role_name
    application_path = first_dir / "application.txt"
    if not application_path.is_file():
        raise ValueError(f"task {task_id} lacks the first-role failure log")
    application = application_path.read_text(encoding="utf-8", errors="replace")
    failed = base_cell(task_id, first_role, "FAIL", source_commit, baseline)
    failed.update(failure_detail(first_role, application))
    whole_wall = optional_float(first_dir / "whole_wall_seconds.txt")
    tree = optional_tree(first_dir / "process_tree.json")
    if whole_wall is not None:
        failed["whole_wall_seconds"] = whole_wall
    if tree is not None and int(tree.get("whole_peak_rss_kib", 0)) > 0:
        peak = int(tree["whole_peak_rss_kib"])
        failed["whole_peak_rss_kib"] = peak
        if first_role == "fevc" and baseline is not None:
            failed["incremental_rss_kib"] = peak - baseline
    not_run = base_cell(task_id, second_role, "NOT_RUN", source_commit, baseline)
    not_run.update(
        {
            "failure_code": "BLOCKED_BY_FIRST_ROLE_FAILURE",
            "failure_message": (
                f"Not run because {first_role} failed first in the registered "
                "sequential order."
            ),
        }
    )
    by_role = {first_role: failed, second_role: not_run}
    return [by_role[role] for role in ROLES]


def successful_summaries(
    cells: list[dict[str, Any]],
) -> list[dict[str, Any]]:
    summaries: list[dict[str, Any]] = []
    for rows in ROWS:
        for role in ROLES:
            selected = [
                cell
                for cell in cells
                if cell["rows"] == rows
                and cell["role"] == role
                and cell["status"] == "PASS"
            ]
            if len(selected) != 3:
                continue
            summaries.append(
                {
                    "rows": rows,
                    "role": role,
                    "median_command_seconds": statistics.median(
                        cell["command_seconds"] for cell in selected
                    ),
                    "median_whole_wall_seconds": statistics.median(
                        cell["whole_wall_seconds"] for cell in selected
                    ),
                    "median_whole_peak_rss_kib": statistics.median(
                        cell["whole_peak_rss_kib"] for cell in selected
                    ),
                    "median_incremental_rss_kib": (
                        statistics.median(
                            cell["incremental_rss_kib"] for cell in selected
                        )
                        if role == "fevc"
                        else None
                    ),
                    "max_coefficient_relative_difference": max(
                        cell["coefficient_max_relative_difference"]
                        for cell in selected
                    ),
                    "max_se_relative_difference": max(
                        cell["se_max_relative_difference"] for cell in selected
                    ),
                    "max_covariance_diagonal_relative_difference": max(
                        cell["covariance_diagonal_max_relative_difference"]
                        for cell in selected
                    ),
                    "covariance_z1_seed_range": max(
                        cell["covariance_2_2"] for cell in selected
                    )
                    - min(cell["covariance_2_2"] for cell in selected),
                    "covariance_z2_seed_range": max(
                        cell["covariance_3_3"] for cell in selected
                    )
                    - min(cell["covariance_3_3"] for cell in selected),
                }
            )

    def find(rows: int, role: str) -> dict[str, Any] | None:
        return next(
            (
                item
                for item in summaries
                if item["rows"] == rows and item["role"] == role
            ),
            None,
        )

    for rows in ROWS:
        fevc = find(rows, "fevc")
        matlab = find(rows, "matlab")
        if fevc is None or matlab is None:
            continue
        summaries.append(
            {
                "rows": rows,
                "role": "paired_ratio_matlab_over_vckss",
                "median_command_seconds": matlab["median_command_seconds"]
                / fevc["median_command_seconds"],
                "median_whole_wall_seconds": matlab["median_whole_wall_seconds"]
                / fevc["median_whole_wall_seconds"],
                "median_whole_peak_rss_kib": None,
                "median_incremental_rss_kib": None,
                "max_coefficient_relative_difference": max(
                    fevc["max_coefficient_relative_difference"],
                    matlab["max_coefficient_relative_difference"],
                ),
                "max_se_relative_difference": max(
                    fevc["max_se_relative_difference"],
                    matlab["max_se_relative_difference"],
                ),
                "max_covariance_diagonal_relative_difference": max(
                    fevc["max_covariance_diagonal_relative_difference"],
                    matlab["max_covariance_diagonal_relative_difference"],
                ),
                "covariance_z1_seed_range": None,
                "covariance_z2_seed_range": None,
            }
        )
    return summaries


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("run_dir", type=Path)
    parser.add_argument("summary_json", type=Path)
    parser.add_argument("cells_csv", type=Path)
    args = parser.parse_args()

    source_commit = (
        (args.run_dir / "source" / "SOURCE_COMMIT.txt")
        .read_text(encoding="utf-8")
        .strip()
    )
    if not SOURCE_RE.fullmatch(source_commit):
        raise ValueError("invalid source commit receipt")
    qacct = parse_qacct(args.run_dir / "receipts" / "array-qacct.txt")
    job_numbers = {record["jobnumber"] for record in qacct.values()}
    if len(job_numbers) != 1:
        raise ValueError(f"qacct job identity changed: {sorted(job_numbers)}")
    scheduler_job_id = int(job_numbers.pop())

    cells: list[dict[str, Any]] = []
    passed_tasks: list[int] = []
    failed_tasks: list[int] = []
    for task_id in range(1, 10):
        task = args.run_dir / "tasks" / f"task-{task_id}"
        if (task / "validation.json").is_file():
            cells.extend(pass_cells(task_id, task, source_commit))
            passed_tasks.append(task_id)
            if qacct[task_id]["exit_status"] != "0":
                raise ValueError(
                    f"task {task_id} passed application gates but qacct did not"
                )
        else:
            cells.extend(failed_cells(task_id, task, source_commit))
            failed_tasks.append(task_id)
            if qacct[task_id]["exit_status"] == "0":
                raise ValueError(
                    f"task {task_id} failed application gates but qacct exited zero"
                )

    summaries = successful_summaries(cells)
    rankable_rows = [
        rows
        for rows in ROWS
        if all(
            sum(
                cell["rows"] == rows
                and cell["role"] == role
                and cell["status"] == "PASS"
                for cell in cells
            )
            == 3
            for role in ROLES
        )
    ]
    v24 = next(
        (
            item
            for item in summaries
            if item["rows"] == 24000 and item["role"] == "fevc"
        ),
        None,
    )
    v96 = next(
        (
            item
            for item in summaries
            if item["rows"] == 96000 and item["role"] == "fevc"
        ),
        None,
    )
    rss_growth = None
    rss_growth_status = "NOT_EVALUATED"
    if v24 is not None and v96 is not None:
        rss_growth = (
            v96["median_incremental_rss_kib"]
            / v24["median_incremental_rss_kib"]
        )
        rss_growth_status = "PASS" if rss_growth <= 5.5 else "FAIL"

    status = "PASS" if not failed_tasks and rss_growth_status == "PASS" else "FAIL"
    failures = [
        {
            key: cell[key]
            for key in (
                "task_id",
                "rows",
                "replicate",
                "role",
                "status",
                "order",
                "failure_code",
                "failure_message",
                "solver_iterations",
                "solver_max_iterations",
                "reported_residual",
                "residual_kind",
                "whole_wall_seconds",
                "whole_peak_rss_kib",
                "incremental_rss_kib",
            )
        }
        for cell in cells
        if cell["status"] != "PASS"
    ]

    fields = list(cells[0])
    with args.cells_csv.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=fields, lineterminator="\n")
        writer.writeheader()
        writer.writerows(cells)
    result = {
        "schema": "FEVC-PROJECTION-SCALING-SUMMARY-V2",
        "status": status,
        "source_commit": source_commit,
        "matlab_upstream_commit": UPSTREAM_COMMIT,
        "scheduler_job_id": scheduler_job_id,
        "scheduler_accounting_complete": True,
        "scheduler_failed_zero_task_count": 9,
        "scheduler_zero_exit_tasks": passed_tasks,
        "scheduler_nonzero_exit_tasks": failed_tasks,
        "task_count": 9,
        "passed_task_count": len(passed_tasks),
        "failed_task_count": len(failed_tasks),
        "registered_role_run_count": 18,
        "passed_role_run_count": sum(cell["status"] == "PASS" for cell in cells),
        "failed_role_run_count": sum(cell["status"] == "FAIL" for cell in cells),
        "not_run_role_count": sum(cell["status"] == "NOT_RUN" for cell in cells),
        "registered_cell_count": 6,
        "rankable_cell_count": 2 * len(rankable_rows),
        "rankable_rows": rankable_rows,
        "repetitions_per_cell": 3,
        "vckss_incremental_rss_growth_24000_to_96000": rss_growth,
        "vckss_incremental_rss_growth_gate": rss_growth_status,
        "se_covariance_96000_gate": (
            "NOT_EVALUATED" if 96000 not in rankable_rows else "PASS"
        ),
        "speed_96000_gate": (
            "NOT_EVALUATED" if 96000 not in rankable_rows else "DESCRIPTIVE"
        ),
        "failures": failures,
        "summaries": summaries,
    }
    args.summary_json.write_text(
        json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    print(json.dumps(result, indent=2, sort_keys=True))
    if status != "PASS":
        raise SystemExit(2)


if __name__ == "__main__":
    main()
