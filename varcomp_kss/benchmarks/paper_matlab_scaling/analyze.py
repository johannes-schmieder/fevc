#!/usr/bin/env python3
"""Summarize the complete validated paper Stata--MATLAB scaling matrix."""

from __future__ import annotations

import argparse
import csv
import json
import statistics
from collections import defaultdict
from pathlib import Path
from typing import Any

try:
    from .common import (
        ORDERS,
        SEEDS,
        STRUCTURES,
        finite,
        load_json,
        read_manifest,
        require,
    )
except ImportError:  # Direct script execution on the SCC.
    from common import (
        ORDERS,
        SEEDS,
        STRUCTURES,
        finite,
        load_json,
        read_manifest,
        require,
    )


def distribution(values: list[float]) -> dict[str, float]:
    require(len(values) == 6, "each cell distribution must contain six jobs")
    ordered = sorted(values)
    quartiles = statistics.quantiles(ordered, n=4, method="inclusive")
    return {
        "minimum": ordered[0],
        "q25": quartiles[0],
        "median": statistics.median(ordered),
        "q75": quartiles[2],
        "maximum": ordered[-1],
    }


def analyze(
    evidence_root: Path, manifest_path: Path,
) -> tuple[list[dict[str, Any]], list[dict[str, Any]], dict[str, Any]]:
    tasks = read_manifest(manifest_path)
    expected = {row["experiment_id"]: row for row in tasks}
    validations: dict[str, dict[str, Any]] = {}
    for path in evidence_root.glob("*/validation.json"):
        value = load_json(path)
        experiment = value.get("experiment_id")
        require(experiment in expected, f"unexpected experiment: {experiment}")
        require(experiment not in validations, f"duplicate evidence: {experiment}")
        require(value.get("schema") == "PAPER-MATLAB-SCALING-VALIDATION-V1",
                f"invalid validation schema: {experiment}")
        require(value.get("status") in {
            "PASS_NUMERICAL_AND_TIMING",
            "PASS_TIMING_MATLAB_NUMERICAL_REJECTED",
        }, f"invalid validation status: {experiment}")
        validations[str(experiment)] = value
    require(set(validations) == set(expected), "comparison evidence is incomplete")

    by_cell: dict[tuple[str, int], list[dict[str, Any]]] = defaultdict(list)
    jobs: list[dict[str, Any]] = []
    for experiment, task in expected.items():
        value = validations[experiment]
        for field in ("structure", "connectivity", "order"):
            require(value[field] == task[field],
                    f"task mismatch: {experiment} {field}")
        for field in ("cells_per_worker", "rows", "workers", "firms", "probes", "seed"):
            require(value[field] == int(task[field]),
                    f"task mismatch: {experiment} {field}")
        by_cell[(task["structure"], int(task["rows"]))].append(value)
        jobs.append({
            "experiment_id": experiment,
            "structure": value["structure"],
            "connectivity": value["connectivity"],
            "cells_per_worker": value["cells_per_worker"],
            "rows": value["rows"],
            "workers": value["workers"],
            "firms": value["firms"],
            "probes": value["probes"],
            "seed": value["seed"],
            "order": value["order"],
            "status": value["status"],
            "source_commit": value["source_commit"],
            "bundle_sha256": value["bundle_sha256"],
            "task_sha256": value["task_sha256"],
            "input_sha256": value["input_sha256"],
            "hostname": value["hostname"],
            "job_id": value["job_id"],
            "stata_command_seconds": value["stata_command_seconds"],
            "matlab_command_seconds": value["matlab_command_seconds"],
            "stata_over_matlab_command_ratio":
                value["stata_over_matlab_command_ratio"],
            "matlab_numerical_result_accepted":
                value["matlab_numerical_result_accepted"],
            "qacct_wall_seconds": value["qacct_wall_seconds"],
            "qacct_maxvmem_bytes": value["qacct_maxvmem_bytes"],
            "stata_gnu_max_rss_bytes": value["stata_gnu_max_rss_bytes"],
            "matlab_process_tree_peak_rss_bytes":
                value["matlab_process_tree_peak_rss_bytes"],
        })

    cells: list[dict[str, Any]] = []
    for (structure, row_count), values in sorted(by_cell.items()):
        require(len(values) == 6, f"cell {structure} N{row_count} lacks six jobs")
        require({value["seed"] for value in values} == set(SEEDS),
                f"cell {structure} N{row_count} lacks registered seeds")
        require({value["order"] for value in values} == set(ORDERS),
                f"cell {structure} N{row_count} lacks both orders")
        for seed in SEEDS:
            require({value["order"] for value in values if value["seed"] == seed}
                    == set(ORDERS), f"cell {structure} N{row_count} seed {seed} "
                    "lacks both orders")
        hashes = {value["input_sha256"] for value in values}
        require(len(hashes) == 1,
                f"cell {structure} N{row_count} used different input bytes")
        stata = distribution([
            finite(value["stata_command_seconds"], "Stata command")
            for value in values
        ])
        matlab = distribution([
            finite(value["matlab_command_seconds"], "MATLAB command")
            for value in values
        ])
        ratio = distribution([
            finite(value["stata_over_matlab_command_ratio"], "time ratio")
            for value in values
        ])
        degree = int(values[0]["cells_per_worker"])
        cell = {
            "structure": structure,
            "connectivity": values[0]["connectivity"],
            "cells_per_worker": degree,
            "rows": row_count,
            "workers": row_count // degree,
            "firms": row_count // degree // 40,
            "probes": 200,
            "jobs": 6,
            "input_sha256": next(iter(hashes)),
            "matlab_numerically_accepted_jobs": sum(
                bool(value["matlab_numerical_result_accepted"])
                for value in values
            ),
            "matlab_numerically_rejected_jobs": sum(
                not bool(value["matlab_numerical_result_accepted"])
                for value in values
            ),
            "stata_command_seconds": stata,
            "matlab_command_seconds": matlab,
            "stata_over_matlab_command_ratio": ratio,
        }
        cells.append(cell)

    require(len(cells) == 20 and {cell["structure"] for cell in cells}
            == set(STRUCTURES), "registered cell matrix changed")
    rejections = sorted(
        value["experiment_id"] for value in validations.values()
        if not value["matlab_numerical_result_accepted"]
    )
    summary = {
        "schema": "PAPER-MATLAB-SCALING-MATRIX-V1",
        "status": ("PASS_WITH_MATLAB_NUMERICAL_REJECTIONS" if rejections else
                   "PASS_NUMERICAL_AND_TIMING"),
        "comparison_contract": "descriptive_p200_command_time_v1",
        "job_count": len(jobs),
        "cell_count": len(cells),
        "source_commits": sorted({job["source_commit"] for job in jobs}),
        "bundle_sha256s": sorted({job["bundle_sha256"] for job in jobs}),
        "structures": list(STRUCTURES),
        "row_grid": sorted({cell["rows"] for cell in cells}),
        "seeds": list(SEEDS),
        "orders": list(ORDERS),
        "matlab_numerical_rejections": rejections,
        "corrected_estimate_equality_gate": "NONE_DESCRIPTIVE_ONLY",
        "cells": cells,
    }
    return sorted(jobs, key=lambda row: row["experiment_id"]), cells, summary


def write_csv(path: Path, rows: list[dict[str, Any]]) -> None:
    require(rows, f"no rows for {path.name}")
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=list(rows[0]), lineterminator="\n")
        writer.writeheader()
        writer.writerows(rows)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--evidence-root", type=Path, required=True)
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    args = parser.parse_args()
    require(args.output_dir.is_dir(), "output directory is missing")
    paths = [args.output_dir / name for name in
             ("jobs.csv", "cells.csv", "summary.json")]
    require(not any(path.exists() for path in paths),
            "analysis target already exists")
    jobs, cells, summary = analyze(args.evidence_root, args.manifest)
    write_csv(paths[0], jobs)
    flat_cells = [{
        "structure": cell["structure"],
        "connectivity": cell["connectivity"],
        "cells_per_worker": cell["cells_per_worker"],
        "rows": cell["rows"],
        "workers": cell["workers"],
        "firms": cell["firms"],
        "probes": cell["probes"],
        "jobs": cell["jobs"],
        "input_sha256": cell["input_sha256"],
        "matlab_numerically_accepted_jobs":
            cell["matlab_numerically_accepted_jobs"],
        "matlab_numerically_rejected_jobs":
            cell["matlab_numerically_rejected_jobs"],
        **{
            f"{language}_{stat}_seconds": cell[f"{language}_command_seconds"][stat]
            for language in ("stata", "matlab")
            for stat in ("minimum", "q25", "median", "q75", "maximum")
        },
        **{
            f"stata_over_matlab_{stat}_ratio":
                cell["stata_over_matlab_command_ratio"][stat]
            for stat in ("minimum", "q25", "median", "q75", "maximum")
        },
    } for cell in cells]
    write_csv(paths[1], flat_cells)
    paths[2].write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n",
                        encoding="utf-8")
    print(f"PAPER_MATLAB_SCALING_MATRIX_PASS jobs={summary['job_count']} "
          f"status={summary['status']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
