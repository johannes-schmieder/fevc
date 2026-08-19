#!/usr/bin/env python3
"""Summarize a complete, validated PREP-BND-1 MATLAB comparison matrix."""

from __future__ import annotations

import argparse
import csv
import json
import statistics
from collections import defaultdict
from pathlib import Path
from typing import Any

from common import TARGETS, TASK_FIELDS, finite, load_json, require


def median_absolute_deviation(values: list[float]) -> float:
    center = statistics.median(values)
    return statistics.median(abs(value - center) for value in values)


def distribution(values: list[float]) -> dict[str, float]:
    require(values, "empty distribution")
    return {
        "minimum": min(values),
        "median": statistics.median(values),
        "maximum": max(values),
        "median_absolute_deviation": median_absolute_deviation(values),
    }


def read_manifest(path: Path) -> list[dict[str, str]]:
    require(path.is_file(), "missing task manifest")
    with path.open(newline="", encoding="utf-8") as handle:
        reader = csv.DictReader(handle, delimiter="\t")
        require(tuple(reader.fieldnames or ()) == TASK_FIELDS,
                "manifest fields changed")
        rows = list(reader)
    require(len(rows) == 30, "manifest must contain 30 tasks")
    require(len({row["experiment_id"] for row in rows}) == len(rows),
            "duplicate manifest experiment")
    return rows


def analyze(evidence_root: Path, manifest: Path) -> tuple[list[dict[str, Any]], dict[str, Any]]:
    tasks = read_manifest(manifest)
    expected = {row["experiment_id"]: row for row in tasks}
    validations: dict[str, dict[str, Any]] = {}
    for path in evidence_root.glob("*/validation.json"):
        value = load_json(path)
        experiment = value.get("experiment_id")
        require(experiment in expected, f"unexpected experiment: {experiment}")
        require(experiment not in validations, f"duplicate evidence: {experiment}")
        require(value.get("status") in {
            "PASS_NUMERICAL_AND_TIMING",
            "PASS_TIMING_MATLAB_NUMERICAL_REJECTED",
        }, f"invalid validation status: {experiment}")
        validations[experiment] = value
    require(set(validations) == set(expected), "comparison evidence is incomplete")

    by_cell: dict[tuple[int, int], list[dict[str, Any]]] = defaultdict(list)
    for experiment, task in expected.items():
        value = validations[experiment]
        for field in ("firms", "workers", "rows", "probes", "seed", "order"):
            expected_value: Any = task[field]
            actual: Any = value[field]
            if field != "order":
                expected_value = int(expected_value)
            require(actual == expected_value, f"task mismatch: {experiment} {field}")
        by_cell[(int(task["firms"]), int(task["probes"]))].append(value)

    summaries: list[dict[str, Any]] = []
    for (firms, probes), rows in sorted(by_cell.items()):
        require(len(rows) == 6, f"cell F{firms} P{probes} must have six jobs")
        require({row["order"] for row in rows} == {"stata_matlab", "matlab_stata"},
                "source-order reversal is missing")
        require(len({row["seed"] for row in rows}) == 3,
                "three deterministic seeds are required")
        for seed in {row["seed"] for row in rows}:
            require({row["order"] for row in rows if row["seed"] == seed} ==
                    {"stata_matlab", "matlab_stata"},
                    f"seed {seed} lacks both orders")
        input_hashes = {row["input_sha256"] for row in rows}
        require(len(input_hashes) == 1, "same cell used different input bytes")
        accepted = [row for row in rows if row["matlab_numerical_result_accepted"]]
        rejected = [row for row in rows if not row["matlab_numerical_result_accepted"]]
        summary: dict[str, Any] = {
            "firms": firms,
            "workers": 40 * firms,
            "rows": 120 * firms,
            "probes": probes,
            "jobs": len(rows),
            "seeds": sorted({row["seed"] for row in rows}),
            "input_sha256": next(iter(input_hashes)),
            "matlab_numerically_accepted_jobs": len(accepted),
            "matlab_numerically_rejected_jobs": len(rejected),
            "stata_command_seconds": distribution([
                finite(row["stata_command_seconds"], "Stata command") for row in rows
            ]),
            "matlab_command_seconds": distribution([
                finite(row["matlab_command_seconds"], "MATLAB command") for row in rows
            ]),
            "stata_over_matlab_command_ratio": distribution([
                finite(row["stata_over_matlab_command_ratio"], "time ratio")
                for row in rows
            ]),
            "qacct_wall_seconds": distribution([
                finite(row["qacct_wall_seconds"], "qacct wall") for row in rows
            ]),
        }
        if accepted:
            summary["accepted_scaled_target_gap_descriptive"] = distribution([
                finite(row["scaled_target_gap_descriptive"], "scaled gap")
                for row in accepted
            ])
            summary["accepted_component_gap_max_descriptive"] = {
                target: max(finite(
                    row["absolute_target_gaps_descriptive"][target], f"{target} gap"
                ) for row in accepted)
                for target in TARGETS
            }
            summary["accepted_seed_dispersion"] = {
                language: {
                    target: max(finite(row[f"{language}_targets"][target], target)
                                for row in accepted) -
                            min(finite(row[f"{language}_targets"][target], target)
                                for row in accepted)
                    for target in TARGETS
                }
                for language in ("stata", "matlab")
            }
        summaries.append(summary)

    # Input bytes depend only on firms, not probes, seed, or source order.
    hashes_by_firm: dict[int, set[str]] = defaultdict(set)
    for row in validations.values():
        hashes_by_firm[int(row["firms"])].add(str(row["input_sha256"]))
    require(all(len(values) == 1 for values in hashes_by_firm.values()),
            "probe or seed changed fixed input bytes")
    rejected_experiments = sorted(
        row["experiment_id"] for row in validations.values()
        if not row["matlab_numerical_result_accepted"]
    )
    aggregate = {
        "schema": "PREP-BND-1-MATLAB-MATRIX-V1",
        "status": ("PASS_WITH_MATLAB_NUMERICAL_REJECTIONS" if rejected_experiments
                   else "PASS_NUMERICAL_AND_TIMING"),
        "comparison_contract": "descriptive_jla_own_pcg_gate_v1",
        "job_count": len(validations),
        "cell_count": len(summaries),
        "source_commits": sorted({row["source_commit"] for row in validations.values()}),
        "bundle_sha256s": sorted({row["bundle_sha256"] for row in validations.values()}),
        "matlab_numerical_rejections": rejected_experiments,
        "corrected_estimate_equality_gate": "NONE_DESCRIPTIVE_ONLY",
        "cells": summaries,
    }
    return summaries, aggregate


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--evidence-root", type=Path, required=True)
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    args = parser.parse_args()
    require(args.output_dir.is_dir(), "output directory is missing")
    csv_path = args.output_dir / "summary.csv"
    json_path = args.output_dir / "summary.json"
    require(not csv_path.exists() and not json_path.exists(),
            "summary target already exists")
    summaries, aggregate = analyze(args.evidence_root, args.manifest)
    flat = [{
        "firms": row["firms"],
        "workers": row["workers"],
        "rows": row["rows"],
        "probes": row["probes"],
        "jobs": row["jobs"],
        "matlab_numerically_accepted_jobs": row["matlab_numerically_accepted_jobs"],
        "matlab_numerically_rejected_jobs": row["matlab_numerically_rejected_jobs"],
        "stata_command_median_seconds": row["stata_command_seconds"]["median"],
        "matlab_command_median_seconds": row["matlab_command_seconds"]["median"],
        "stata_over_matlab_median_ratio":
            row["stata_over_matlab_command_ratio"]["median"],
        "accepted_scaled_gap_median":
            row.get("accepted_scaled_target_gap_descriptive", {}).get("median"),
        "accepted_scaled_gap_maximum":
            row.get("accepted_scaled_target_gap_descriptive", {}).get("maximum"),
    } for row in summaries]
    with csv_path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=list(flat[0]),
                                lineterminator="\n")
        writer.writeheader()
        writer.writerows(flat)
    json_path.write_text(json.dumps(aggregate, indent=2, sort_keys=True) + "\n",
                         encoding="utf-8")
    print(f"PREP_BND1_MATLAB_MATRIX_PASS jobs={aggregate['job_count']} "
          f"status={aggregate['status']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
