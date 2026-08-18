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
from pathlib import Path
from typing import Any


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


def qacct(path: Path) -> dict[str, str]:
    require(path.is_file(), f"missing qacct: {path}")
    result: dict[str, str] = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        fields = line.strip().split(None, 1)
        if len(fields) == 2:
            result[fields[0]] = fields[1]
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


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def comparison_row(root: Path, experiment: str) -> dict[str, Any]:
    matlab_dir = root / "matlab" / experiment
    kss_dir = root / "synthetic" / experiment
    matlab_validation = load_json(matlab_dir / "validation.json")
    kss_validation = load_json(kss_dir / "validation.json")
    require(matlab_validation.get("status") == "PASS", "MATLAB validation failed")
    require(kss_validation.get("status") == "PASS", "KSS validation failed")
    matlab_task = key_values(matlab_dir / "task.tsv")
    kss_task = key_values(kss_dir / "task.tsv")
    for field in (
        "workers", "firms", "cells_per_worker", "rows_per_cell",
        "connectivity", "probes", "seed",
    ):
        require(matlab_task[field] == kss_task[field],
                f"task mismatch for {experiment}: {field}")
    aggregate = load_json(matlab_dir / "aggregate.json")
    summary = one_csv(kss_dir / "summary.csv")
    matlab_qacct = qacct(matlab_dir / "qacct.txt")
    kss_qacct = qacct(kss_dir / "qacct.txt")
    tree = load_json(matlab_dir / "process_tree_rss.json")
    require(aggregate["corrected_estimate_equality_gate"] ==
            "NONE_DESCRIPTIVE_ONLY", "unsupported equality gate")
    require(aggregate["target_weight_semantics_comparable"] is False,
            "unsupported target-weight comparison")
    kss_command = finite(summary["command_seconds"], "KSS command")
    matlab_command = finite(aggregate["command_seconds"], "MATLAB command")
    require(kss_command > 0 and matlab_command > 0, "nonpositive command time")
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
        "kss_job_id": kss_validation["job_id"],
        "matlab_job_id": matlab_validation["job_id"],
        "kss_command_seconds": kss_command,
        "matlab_command_seconds": matlab_command,
        "matlab_over_kss_command_ratio": matlab_command / kss_command,
        "kss_qacct_wall_seconds": finite(kss_qacct["ru_wallclock"], "KSS wall"),
        "matlab_qacct_wall_seconds": finite(
            matlab_qacct["ru_wallclock"], "MATLAB wall"
        ),
        "kss_qacct_maxvmem_bytes": parse_memory(kss_qacct["maxvmem"]),
        "matlab_qacct_maxvmem_bytes": parse_memory(matlab_qacct["maxvmem"]),
        "matlab_process_tree_peak_rss_bytes": int(
            finite(tree["peak_rss_kib"], "MATLAB process-tree RSS") * 1024
        ),
        "matlab_corrected_worker": finite(
            aggregate["corrected_worker"], "MATLAB worker target"
        ),
        "matlab_corrected_firm": finite(
            aggregate["corrected_firm"], "MATLAB firm target"
        ),
        "matlab_corrected_covariance": finite(
            aggregate["corrected_covariance"], "MATLAB covariance target"
        ),
        "matlab_corrected_total": finite(
            aggregate["corrected_total"], "MATLAB total target"
        ),
    }


def write_csv(path: Path, rows: list[dict[str, Any]]) -> None:
    require(rows, "no comparison rows")
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=list(rows[0]))
        writer.writeheader()
        writer.writerows(rows)


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
    args.output_dir.mkdir(parents=True, exist_ok=True)
    csv_path = args.output_dir / "matlab_comparison.csv"
    write_csv(csv_path, rows)
    payload = {
        "schema": "KSS-NUMOPT-2-MATLAB-COMPARISON-V1",
        "status": "PASS",
        "comparison_count": len(rows),
        "experiments": experiments,
        "interpretation": (
            "Runtime and resources only; corrected estimates are descriptive "
            "because target weights, RNG draws, and tolerances differ."
        ),
        "comparison_csv_sha256": sha256(csv_path),
    }
    (args.output_dir / "matlab_comparison.json").write_text(
        json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    print(f"KSS_NUMOPT2_MATLAB_SUMMARY_PASS {len(rows)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
