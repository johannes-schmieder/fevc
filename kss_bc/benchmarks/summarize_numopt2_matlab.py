#!/usr/bin/env python3
"""Summarize validated KSS/MATLAB runs on identical synthetic task shapes.

The maintained MATLAB code does not implement KSS's custom target weights,
RNG schedule, or solver tolerance.  This tool therefore compares elapsed time
and observed resource use only.  Corrected estimates are retained as
descriptive receipts and never enter an equality gate.
"""
from __future__ import annotations

import argparse
import csv
import hashlib
import json
import math
import re
from pathlib import Path
from typing import Any

GIB = 1024**3
HARD_MEMORY_BYTES = 128 * GIB
ADMISSION_HEADROOM = 0.20
REFERENCE_EXPERIMENT = "strong_f64_d2"


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def finite(value: Any, label: str) -> float:
    result = float(value)
    require(math.isfinite(result), f"nonfinite {label}")
    return result


def load_json(path: Path) -> dict[str, Any]:
    require(path.is_file(), f"missing JSON: {path}")
    value = json.loads(path.read_text(encoding="utf-8"))
    require(isinstance(value, dict), f"invalid JSON object: {path}")
    return value


def key_values(path: Path) -> dict[str, str]:
    require(path.is_file(), f"missing receipt: {path}")
    with path.open(newline="", encoding="utf-8") as handle:
        reader = csv.reader(handle, delimiter="\t")
        require(next(reader, None) == ["key", "value"], f"invalid header: {path}")
        rows = list(reader)
    require(all(len(row) == 2 for row in rows), f"invalid row: {path}")
    result = {row[0]: row[1] for row in rows}
    require(len(result) == len(rows), f"duplicate key: {path}")
    return result


def one_csv(path: Path) -> dict[str, str]:
    require(path.is_file(), f"missing CSV: {path}")
    with path.open(newline="", encoding="utf-8") as handle:
        rows = list(csv.DictReader(handle))
    require(len(rows) == 1, f"expected one CSV row: {path}")
    return rows[0]


def qacct(path: Path, *, require_success: bool = True) -> dict[str, str]:
    require(path.is_file(), f"missing qacct: {path}")
    result: dict[str, str] = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        fields = line.strip().split(None, 1)
        if len(fields) == 2:
            result[fields[0]] = fields[1]
    if require_success:
        require(result.get("failed") == result.get("exit_status") == "0",
                f"failed qacct: {path}")
    return result


def parse_memory(value: str) -> int:
    units = {"": 1, "K": 1024, "M": 1024**2, "G": 1024**3, "T": 1024**4}
    text = value.strip().upper()
    suffix = text[-1] if text and text[-1] in units and not text[-1].isdigit() else ""
    number = text[:-1] if suffix else text
    result = float(number) * units[suffix]
    require(math.isfinite(result) and result >= 0, f"invalid memory: {value}")
    return math.ceil(result)


def matlab_pcg_status(path: Path) -> dict[str, Any]:
    """Parse the maintained command's single reported PCG termination."""
    require(path.is_file(), f"missing MATLAB application log: {path}")
    source = path.read_text(encoding="utf-8")
    converged = re.search(
        r"pcg converged at iteration ([0-9]+) to a solution with relative "
        r"residual ([0-9.eE+-]+)\.",
        source,
    )
    stopped = re.search(
        r"pcg stopped at iteration ([0-9]+) without converging.*?"
        r"The iterate returned \(number ([0-9]+)\) has relative residual "
        r"([0-9.eE+-]+)\.",
        source,
        re.DOTALL,
    )
    require((converged is None) != (stopped is None),
            f"ambiguous MATLAB PCG status: {path}")
    if converged is not None:
        return {
            "converged": True,
            "termination_iteration": int(converged.group(1)),
            "returned_iteration": int(converged.group(1)),
            "relative_residual": finite(converged.group(2), "MATLAB PCG residual"),
        }
    assert stopped is not None
    return {
        "converged": False,
        "termination_iteration": int(stopped.group(1)),
        "returned_iteration": int(stopped.group(2)),
        "relative_residual": finite(stopped.group(3), "MATLAB PCG residual"),
    }


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def comparison_row(root: Path, experiment: str) -> dict[str, Any]:
    matlab_dir = root / "matlab" / experiment
    kss_dir = root / "synthetic" / experiment
    if not (kss_dir / "validation.json").is_file():
        kss_dir = root / "synthetic_censored" / experiment
    matlab_validation = load_json(matlab_dir / "validation.json")
    kss_validation = load_json(kss_dir / "validation.json")
    require(matlab_validation.get("status") == "PASS", "MATLAB validation failed")
    kss_status = kss_validation.get("status")
    require(kss_status in {
        "PASS", "CENSORED_APPLICATION_TIMEOUT",
        "STOPPED_AFTER_DECISION_BOUND",
    }, "KSS validation failed")
    matlab_task = key_values(matlab_dir / "task.tsv")
    kss_task = key_values(kss_dir / "task.tsv")
    for field in (
        "workers", "firms", "cells_per_worker", "rows_per_cell",
        "connectivity", "probes", "seed",
    ):
        require(matlab_task[field] == kss_task[field],
                f"task mismatch for {experiment}: {field}")
    aggregate = load_json(matlab_dir / "aggregate.json")
    summary = one_csv(kss_dir / "summary.csv") if kss_status == "PASS" else None
    matlab_qacct = qacct(matlab_dir / "qacct.txt")
    kss_qacct = qacct(
        kss_dir / "qacct.txt", require_success=kss_status == "PASS"
    )
    tree = load_json(matlab_dir / "process_tree_rss.json")
    pcg = matlab_pcg_status(matlab_dir / "application.txt")
    require(aggregate["corrected_estimate_equality_gate"] ==
            "NONE_DESCRIPTIVE_ONLY", "unsupported equality gate")
    require(aggregate["target_weight_semantics_comparable"] is False,
            "unsupported target-weight comparison")
    kss_command = (
        finite(summary["command_seconds"], "KSS command")
        if summary is not None else None
    )
    kss_command_lower_bound = (
        kss_command if kss_command is not None else finite(
            kss_validation["command_lower_bound_seconds"],
            "KSS censored command lower bound",
        )
    )
    matlab_command = finite(aggregate["command_seconds"], "MATLAB command")
    require(
        kss_command_lower_bound > 0 and matlab_command > 0,
        "nonpositive command time",
    )
    corrected_fields = (
        "corrected_worker", "corrected_firm", "corrected_covariance",
        "corrected_total",
    )
    kss_corrected = (
        {field: finite(summary[field], f"KSS {field}") for field in corrected_fields}
        if summary is not None else {field: None for field in corrected_fields}
    )
    matlab_corrected = {
        field: finite(aggregate[field], f"MATLAB {field}")
        for field in corrected_fields
    }
    return {
        "experiment": experiment,
        "workers": int(kss_task["workers"]),
        "firms": int(kss_task["firms"]),
        "cells_per_worker": int(kss_task["cells_per_worker"]),
        "rows_per_cell": int(kss_task["rows_per_cell"]),
        "connectivity": kss_task["connectivity"],
        "probes": int(kss_task["probes"]),
        "seed": int(kss_task["seed"]),
        "frequency_semantics_comparable": bool(
            aggregate["frequency_semantics_comparable"]
        ),
        "target_weight_semantics_comparable": False,
        "rng_draws_comparable": False,
        "solver_tolerance_comparable": False,
        "corrected_estimate_equality_gate": "NONE_DESCRIPTIVE_ONLY",
        "comparison_source_commit": matlab_validation[
            "comparison_source_commit"
        ],
        "comparison_bundle_sha256": matlab_validation[
            "comparison_bundle_sha256"
        ],
        "kss_source_commit": matlab_validation["kss_source_commit"],
        "kss_bundle_sha256": matlab_validation["kss_bundle_sha256"],
        "kss_job_id": kss_validation["job_id"],
        "matlab_job_id": matlab_validation["job_id"],
        "kss_evidence_status": kss_status,
        "kss_command_seconds": kss_command,
        "kss_command_lower_bound_seconds": kss_command_lower_bound,
        "matlab_command_seconds": matlab_command,
        "matlab_over_kss_command_ratio": (
            matlab_command / kss_command if kss_command is not None else None
        ),
        "matlab_over_kss_command_ratio_upper_bound": (
            matlab_command / kss_command_lower_bound
        ),
        "kss_qacct_wall_seconds": finite(kss_qacct["ru_wallclock"], "KSS wall"),
        "matlab_qacct_wall_seconds": finite(
            matlab_qacct["ru_wallclock"], "MATLAB wall"
        ),
        "kss_qacct_maxvmem_bytes": parse_memory(kss_qacct["maxvmem"]),
        "matlab_qacct_maxvmem_bytes": parse_memory(matlab_qacct["maxvmem"]),
        "matlab_process_tree_peak_rss_bytes": int(
            finite(tree["peak_rss_kib"], "MATLAB process-tree RSS") * 1024
        ),
        "matlab_pcg_converged": pcg["converged"],
        "matlab_numerical_result_accepted": pcg["converged"],
        "matlab_pcg_termination_iteration": pcg["termination_iteration"],
        "matlab_pcg_returned_iteration": pcg["returned_iteration"],
        "matlab_pcg_relative_residual": pcg["relative_residual"],
        "kss_corrected_worker_descriptive": kss_corrected["corrected_worker"],
        "matlab_corrected_worker_descriptive": matlab_corrected[
            "corrected_worker"
        ],
        "corrected_worker_abs_gap_descriptive": None if summary is None else abs(
            matlab_corrected["corrected_worker"] -
            kss_corrected["corrected_worker"]
        ),
        "kss_corrected_firm_descriptive": kss_corrected["corrected_firm"],
        "matlab_corrected_firm_descriptive": matlab_corrected["corrected_firm"],
        "corrected_firm_abs_gap_descriptive": None if summary is None else abs(
            matlab_corrected["corrected_firm"] -
            kss_corrected["corrected_firm"]
        ),
        "kss_corrected_covariance_descriptive": kss_corrected[
            "corrected_covariance"
        ],
        "matlab_corrected_covariance_descriptive": matlab_corrected[
            "corrected_covariance"
        ],
        "corrected_covariance_abs_gap_descriptive": None if summary is None else abs(
            matlab_corrected["corrected_covariance"] -
            kss_corrected["corrected_covariance"]
        ),
        "kss_corrected_total_descriptive": kss_corrected["corrected_total"],
        "matlab_corrected_total_descriptive": matlab_corrected[
            "corrected_total"
        ],
        "corrected_total_abs_gap_descriptive": None if summary is None else abs(
            matlab_corrected["corrected_total"] -
            kss_corrected["corrected_total"]
        ),
    }


def write_csv(path: Path, rows: list[dict[str, Any]]) -> None:
    require(rows, "no comparison rows")
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(
            handle, fieldnames=list(rows[0]), lineterminator="\n"
        )
        writer.writeheader()
        writer.writerows(rows)


def admission_rows(root: Path, reference: dict[str, Any]) -> list[dict[str, Any]]:
    """Project process-tree RSS by the largest registered dimension ratio."""
    result: list[dict[str, Any]] = []
    evidence_dirs = [root / "synthetic", root / "synthetic_censored"]
    reference_cells = (
        int(reference["workers"]) * int(reference["cells_per_worker"])
    )
    reference_rows = reference_cells * int(reference["rows_per_cell"])
    reference_peak = int(reference["matlab_process_tree_peak_rss_bytes"])
    directories = sorted(
        path
        for evidence_dir in evidence_dirs
        if evidence_dir.is_dir()
        for path in evidence_dir.iterdir()
        if path.is_dir()
    )
    for directory in directories:
        validation_path = directory / "validation.json"
        if not validation_path.is_file():
            continue
        require(
            load_json(validation_path).get("status") in {
                "PASS", "CENSORED_APPLICATION_TIMEOUT",
                "STOPPED_AFTER_DECISION_BOUND",
            },
            f"unvalidated KSS task: {directory}",
        )
        task = key_values(directory / "task.tsv")
        workers = int(task["workers"])
        firms = int(task["firms"])
        cells = workers * int(task["cells_per_worker"])
        rows = cells * int(task["rows_per_cell"])
        scale = max(
            workers / int(reference["workers"]),
            firms / int(reference["firms"]),
            cells / reference_cells,
            rows / reference_rows,
        )
        point = reference_peak * scale
        admitted_peak = point * (1 + ADMISSION_HEADROOM)
        compared = (
            root / "matlab" / directory.name / "validation.json"
        ).is_file()
        result.append({
            "experiment": directory.name,
            "workers": workers,
            "firms": firms,
            "cells_per_worker": int(task["cells_per_worker"]),
            "rows_per_cell": int(task["rows_per_cell"]),
            "connectivity": task["connectivity"],
            "raw_rows": rows,
            "dimension_scale_from_reference": scale,
            "process_tree_rss_point_gib": point / GIB,
            "process_tree_rss_with_20pct_headroom_gib": admitted_peak / GIB,
            "admitted_under_128_gib": admitted_peak <= HARD_MEMORY_BYTES,
            "validated_matlab_comparison": compared,
        })
    require(result, "no validated KSS tasks for MATLAB admission")
    missing = [
        row["experiment"] for row in result
        if row["admitted_under_128_gib"] and
        not row["validated_matlab_comparison"]
    ]
    require(not missing, "missing admitted MATLAB comparisons: " + ", ".join(missing))
    return result


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--evidence-root", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    args = parser.parse_args()
    matlab_root = args.evidence_root / "matlab"
    require(matlab_root.is_dir(), f"missing MATLAB evidence: {matlab_root}")
    experiments = sorted(
        directory.name for directory in matlab_root.iterdir()
        if directory.is_dir() and (directory / "validation.json").is_file()
    )
    require(experiments, "no validated MATLAB comparisons")
    rows = [comparison_row(args.evidence_root, name) for name in experiments]
    reference_rows = [
        row for row in rows if row["experiment"] == REFERENCE_EXPERIMENT
    ]
    require(len(reference_rows) == 1, "missing unique MATLAB admission reference")
    admission = admission_rows(args.evidence_root, reference_rows[0])
    args.output_dir.mkdir(parents=True, exist_ok=True)
    csv_path = args.output_dir / "matlab_comparison.csv"
    write_csv(csv_path, rows)
    admission_path = args.output_dir / "matlab_admission.csv"
    write_csv(admission_path, admission)
    payload = {
        "schema": "KSS-NUMOPT-2-MATLAB-COMPARISON-V1",
        "status": "PASS",
        "comparison_count": len(rows),
        "experiments": experiments,
        "comparison_source_commits": sorted({
            row["comparison_source_commit"] for row in rows
        }),
        "comparison_bundle_sha256s": sorted({
            row["comparison_bundle_sha256"] for row in rows
        }),
        "kss_source_commits": sorted({row["kss_source_commit"] for row in rows}),
        "kss_bundle_sha256s": sorted({row["kss_bundle_sha256"] for row in rows}),
        "matlab_pcg_converged_experiments": [
            row["experiment"] for row in rows if row["matlab_pcg_converged"]
        ],
        "matlab_pcg_nonconverged_experiments": [
            row["experiment"] for row in rows if not row["matlab_pcg_converged"]
        ],
        "matlab_numerical_result_accepted_experiments": [
            row["experiment"]
            for row in rows if row["matlab_numerical_result_accepted"]
        ],
        "matlab_numerical_result_not_accepted_experiments": [
            row["experiment"]
            for row in rows if not row["matlab_numerical_result_accepted"]
        ],
        "kss_completed_experiments": [
            row["experiment"] for row in rows
            if row["kss_evidence_status"] == "PASS"
        ],
        "kss_censored_experiments": [
            row["experiment"] for row in rows
            if row["kss_evidence_status"] != "PASS"
        ],
        "admission_reference": REFERENCE_EXPERIMENT,
        "admission_rule": (
            "reference process-tree RSS times the largest workers, firms, "
            "cells, or raw-rows ratio, plus 20 percent headroom, at most 128 GiB"
        ),
        "admitted_experiments": [
            row["experiment"] for row in admission
            if row["admitted_under_128_gib"]
        ],
        "not_admitted_experiments": [
            row["experiment"] for row in admission
            if not row["admitted_under_128_gib"]
        ],
        "interpretation": (
            "Runtime and resources only; corrected estimates are descriptive "
            "because target weights, RNG draws, and tolerances differ. A "
            "MATLAB corrected result is not numerically accepted when its "
            "reported PCG did not converge."
        ),
        "comparison_csv_sha256": sha256(csv_path),
        "admission_csv_sha256": sha256(admission_path),
    }
    (args.output_dir / "matlab_comparison.json").write_text(
        json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    print(f"KSS_NUMOPT2_MATLAB_SUMMARY_PASS {len(rows)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
