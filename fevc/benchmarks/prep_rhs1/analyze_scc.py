#!/usr/bin/env python3
"""Summarize source-bound PREP-RHS-1 SCC baseline/candidate runs."""

from __future__ import annotations

import argparse
import csv
import hashlib
import json
import math
import re
import statistics
from collections import defaultdict
from pathlib import Path
from typing import Any

SCIENTIFIC_FIELDS = (
    "plugin_worker",
    "plugin_firm",
    "plugin_covariance",
    "plugin_total",
    "correction_worker",
    "correction_firm",
    "correction_covariance",
    "correction_total",
    "corrected_worker",
    "corrected_firm",
    "corrected_covariance",
    "corrected_total",
)
STRUCTURAL_FIELDS = (
    "input_rows",
    "N_stored",
    "N_retained",
    "N_physical",
    "worker_levels",
    "firm_levels",
    "deletion_units",
    "coefficient_cells",
    "target_strata",
    "requested_probes",
    "seed",
    "selected_batch",
    "engine_selected",
    "preconditioner_selected",
    "estimator_status",
    "sample_semantics_valid",
    "rhs_count",
)
TIMING_FIELDS = (
    "command_seconds",
    "life_work_seconds",
    "fit_seconds",
    "leverage_seconds",
    "target_seconds",
    "correction_seconds",
    "rng_seconds",
    "schur_seconds",
    "preconditioner_apply_seconds",
    "pcg_seconds",
)
RESOURCE_FIELDS = (
    "resource_peak_bytes",
    "resource_numerical_peak_bytes",
    "solver_schur_actions",
    "solver_precond_applications",
    "solver_max_residual",
)
REPETITION = re.compile(r"_rep[0-9]+$")
EVIDENCE_FILES = (
    "application.log",
    "generation.log",
    "generation_resources.txt",
    "input_receipt.tsv",
    "node_receipt.tsv",
    "process_resources.txt",
    "rhs.csv",
    "route_diagnostics.csv",
    "stage_memory.csv",
    "stata.pass",
    "summary.csv",
    "task.tsv",
    "validation.json",
    "wrapper.pass",
)


def _require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def _one_csv(path: Path) -> dict[str, str]:
    _require(path.is_file(), f"missing CSV: {path}")
    with path.open(newline="", encoding="utf-8") as handle:
        rows = list(csv.DictReader(handle))
    _require(len(rows) == 1, f"expected one row: {path}")
    return rows[0]


def _sha256(path: Path) -> str:
    _require(path.is_file(), f"missing evidence artifact: {path}")
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def _load_role(root: Path) -> dict[str, dict[str, Any]]:
    experiments = root / "experiments"
    _require(experiments.is_dir(), f"missing experiments directory: {experiments}")
    out: dict[str, dict[str, Any]] = {}
    for directory in sorted(experiments.iterdir()):
        if not directory.is_dir() or not (directory / "validation.json").is_file():
            continue
        summary = _one_csv(directory / "summary.csv")
        validation = json.loads((directory / "validation.json").read_text())
        _require(validation.get("status") == "PASS", f"unvalidated run: {directory}")
        _require(
            validation.get("experiment_id") == directory.name,
            f"experiment mismatch: {directory}",
        )
        _require(
            summary.get("source_commit") == validation.get("source_commit")
            and summary.get("bundle_sha256") == validation.get("bundle_sha256"),
            f"source binding mismatch: {directory}",
        )
        out[directory.name] = {
            "summary": summary,
            "validation": validation,
            "artifact_sha256": {
                name: _sha256(directory / name) for name in EVIDENCE_FILES
            },
        }
    return out


def _number(row: dict[str, str], field: str) -> float:
    value = float(row[field])
    _require(math.isfinite(value), f"nonfinite {field}")
    return value


def _median(rows: list[dict[str, str]], field: str) -> float:
    return statistics.median(_number(row, field) for row in rows)


def _percent(candidate: float, baseline: float) -> float | None:
    return None if baseline == 0 else 100 * (candidate / baseline - 1)


def _run_record(role: str, experiment: str, item: dict[str, Any]) -> dict[str, Any]:
    summary = item["summary"]
    validation = item["validation"]
    return {
        "role": role,
        "experiment": experiment,
        "scenario": REPETITION.sub("", experiment),
        "source_commit": validation["source_commit"],
        "bundle_sha256": validation["bundle_sha256"],
        "job_id": validation["job_id"],
        "hostname": validation["qacct"]["hostname"],
        "task_sha256": validation["task_sha256"],
        "input_sha256": validation["input_sha256"],
        "scientific": {field: summary[field] for field in SCIENTIFIC_FIELDS},
        "structural": {field: summary[field] for field in STRUCTURAL_FIELDS},
        "performance": {
            field: _number(summary, field)
            for field in TIMING_FIELDS + RESOURCE_FIELDS
        },
        "artifact_sha256": item["artifact_sha256"],
    }


def summarize(
    baseline_root: Path,
    candidate_root: Path,
    scientific_relative_gate: float,
) -> dict[str, Any]:
    baseline = _load_role(baseline_root)
    candidate = _load_role(candidate_root)
    paired_ids = sorted(set(baseline) & set(candidate))
    _require(paired_ids, "no paired validated experiments")

    structural_differences: list[dict[str, str]] = []
    scientific_differences: list[dict[str, Any]] = []
    maximum_scientific_relative_difference = 0.0
    same_host_pairs = 0
    groups: dict[str, list[str]] = defaultdict(list)
    for experiment in paired_ids:
        base = baseline[experiment]
        cand = candidate[experiment]
        base_summary = base["summary"]
        cand_summary = cand["summary"]
        if base["validation"]["qacct"]["hostname"] == cand["validation"]["qacct"]["hostname"]:
            same_host_pairs += 1
        for field in STRUCTURAL_FIELDS:
            if base_summary[field] != cand_summary[field]:
                structural_differences.append(
                    {
                        "experiment": experiment,
                        "field": field,
                        "baseline": base_summary[field],
                        "candidate": cand_summary[field],
                    }
                )
        for field in SCIENTIFIC_FIELDS:
            base_value = _number(base_summary, field)
            candidate_value = _number(cand_summary, field)
            difference = abs(candidate_value - base_value) / (1 + abs(base_value))
            maximum_scientific_relative_difference = max(
                maximum_scientific_relative_difference, difference
            )
            if base_summary[field] != cand_summary[field]:
                scientific_differences.append(
                    {
                        "experiment": experiment,
                        "field": field,
                        "baseline": base_summary[field],
                        "candidate": cand_summary[field],
                        "relative_difference": difference,
                    }
                )
        groups[REPETITION.sub("", experiment)].append(experiment)

    _require(not structural_differences, "paired structural outputs differ")
    _require(
        maximum_scientific_relative_difference <= scientific_relative_gate,
        "paired scientific outputs exceed the registered roundoff gate",
    )

    scenario_rows: list[dict[str, Any]] = []
    for scenario, experiments in sorted(groups.items()):
        base_rows = [baseline[item]["summary"] for item in experiments]
        candidate_rows = [candidate[item]["summary"] for item in experiments]
        row: dict[str, Any] = {
            "scenario": scenario,
            "repetitions": len(experiments),
            "same_host_pairs": sum(
                baseline[item]["validation"]["qacct"]["hostname"]
                == candidate[item]["validation"]["qacct"]["hostname"]
                for item in experiments
            ),
        }
        for field in TIMING_FIELDS + RESOURCE_FIELDS:
            base_median = _median(base_rows, field)
            candidate_median = _median(candidate_rows, field)
            row[f"baseline_{field}"] = base_median
            row[f"candidate_{field}"] = candidate_median
            row[f"change_percent_{field}"] = _percent(candidate_median, base_median)
        scenario_rows.append(row)

    baseline_commits = {item["validation"]["source_commit"] for item in baseline.values()}
    candidate_commits = {item["validation"]["source_commit"] for item in candidate.values()}
    _require(len(baseline_commits) == 1, "baseline source commits differ")
    _require(len(candidate_commits) == 1, "candidate source commits differ")
    return {
        "schema": "fevc-prep-rhs1-scc-summary-v1",
        "status": "PASS",
        "baseline_commit": next(iter(baseline_commits)),
        "candidate_commit": next(iter(candidate_commits)),
        "validated_baseline_runs": len(baseline),
        "validated_candidate_runs": len(candidate),
        "paired_runs": len(paired_ids),
        "same_host_pairs": same_host_pairs,
        "timing_comparison_status": (
            "HOST_CONFOUNDED" if same_host_pairs != len(paired_ids) else "HOST_MATCHED"
        ),
        "structural_comparison": "EXACT_PASS",
        "scientific_comparison": "ROUNDOFF_PASS",
        "scientific_relative_gate": scientific_relative_gate,
        "maximum_scientific_relative_difference": (maximum_scientific_relative_difference),
        "scientific_string_difference_count": len(scientific_differences),
        "scenarios": scenario_rows,
        "runs": [
            *(
                _run_record("baseline", experiment, baseline[experiment])
                for experiment in sorted(baseline)
            ),
            *(
                _run_record("candidate", experiment, candidate[experiment])
                for experiment in sorted(candidate)
            ),
        ],
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--baseline-dir", type=Path, required=True)
    parser.add_argument("--candidate-dir", type=Path, required=True)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--scientific-relative-gate", type=float, default=2e-13)
    args = parser.parse_args()
    _require(
        math.isfinite(args.scientific_relative_gate) and args.scientific_relative_gate >= 0,
        "invalid scientific roundoff gate",
    )
    result = summarize(args.baseline_dir, args.candidate_dir, args.scientific_relative_gate)
    payload = json.dumps(result, indent=2, sort_keys=True) + "\n"
    if args.output is not None:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(payload, encoding="utf-8")
    print(payload, end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
