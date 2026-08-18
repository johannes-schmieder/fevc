#!/usr/bin/env python3
"""Validate one source-bound KSS-NUMOPT-2 synthetic SCC result."""
from __future__ import annotations

import argparse
import csv
import hashlib
import json
import math
import re
from pathlib import Path


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def sha256(path: Path) -> str:
    require(path.is_file(), f"missing file: {path}")
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def key_values(path: Path) -> dict[str, str]:
    require(path.is_file(), f"missing key-value receipt: {path}")
    with path.open(newline="", encoding="utf-8") as handle:
        reader = csv.reader(handle, delimiter="\t")
        require(next(reader, None) == ["key", "value"], "invalid TSV header")
        rows = list(reader)
    require(all(len(row) == 2 for row in rows), "invalid TSV row")
    values = {row[0]: row[1] for row in rows}
    require(len(values) == len(rows), "duplicate TSV key")
    return values


def one_csv(path: Path) -> dict[str, str]:
    require(path.is_file(), f"missing CSV: {path}")
    with path.open(newline="", encoding="utf-8") as handle:
        rows = list(csv.DictReader(handle))
    require(len(rows) == 1, f"expected one CSV row: {path}")
    return rows[0]


def num(row: dict[str, str], field: str) -> float:
    require(field in row, f"missing {field}")
    value = float(row[field])
    require(math.isfinite(value), f"nonfinite {field}")
    return value


def integer(row: dict[str, str], field: str) -> int:
    value = num(row, field)
    require(value.is_integer(), f"noninteger {field}")
    return int(value)


def qacct(path: Path) -> dict[str, str]:
    require(path.is_file(), f"missing qacct: {path}")
    values: dict[str, str] = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        fields = line.strip().split(None, 1)
        if len(fields) == 2:
            values[fields[0]] = fields[1]
    required = {"jobnumber", "taskid", "project", "granted_pe", "slots",
                "failed", "exit_status", "ru_wallclock", "cpu", "maxvmem",
                "hostname"}
    require(required <= values.keys(), "incomplete qacct")
    return values


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--run-dir", type=Path, required=True)
    parser.add_argument("--experiment", required=True)
    parser.add_argument("--expected-commit", required=True)
    parser.add_argument("--expected-bundle", required=True)
    args = parser.parse_args()
    require(re.fullmatch(r"[0-9a-f]{40}", args.expected_commit) is not None,
            "invalid expected commit")
    require(re.fullmatch(r"[0-9a-f]{64}", args.expected_bundle) is not None,
            "invalid expected bundle")
    require(re.fullmatch(r"[A-Za-z0-9._-]+", args.experiment) is not None,
            "invalid experiment")

    run = args.run_dir
    output = run / "experiments" / args.experiment
    task = key_values(output / "task.tsv")
    require(sha256(output / "task.tsv") ==
            (output / "task.sha256").read_text().strip(), "task hash mismatch")
    require(task.get("task_version") == "KSS-NUMOPT-2-TASK-V1",
            "task version changed")
    require(task.get("source_commit") == args.expected_commit and
            task.get("bundle_sha256") == args.expected_bundle,
            "task source binding changed")
    require((run / "source_commit.txt").read_text().strip() == args.expected_commit,
            "run commit mismatch")
    require((run / "bundle.sha256").read_text().strip() == args.expected_bundle,
            "run bundle mismatch")

    job_id = (run / "submissions" / f"{args.experiment}.job_id").read_text().strip()
    require(job_id.isdigit(), "invalid job ID")
    accounting = qacct(run / "qacct" / f"{args.experiment}.txt")
    require(accounting["jobnumber"] == job_id and
            accounting["taskid"] == "undefined", "qacct job mismatch")
    require(accounting["project"] == "welfgr" and
            accounting["granted_pe"] == "omp", "scheduler binding changed")
    require(int(accounting["slots"]) == int(task["slots"]), "slot mismatch")
    require(accounting["failed"] == "0" and accounting["exit_status"] == "0",
            "scheduler or wrapper failure")

    require((output / "wrapper.pass").is_file(), "missing wrapper marker")
    require((output / "stata.pass").is_file(), "missing application marker")
    require(not (output / "wrapper.fail").exists(), "wrapper failure marker exists")
    receipt = key_values(output / "input_receipt.tsv")
    node = key_values(output / "node_receipt.tsv")
    input_sha = (output / "input.sha256").read_text().strip()
    require(re.fullmatch(r"[0-9a-f]{64}", input_sha) is not None,
            "invalid input hash")
    require(receipt.get("receipt_version") == "KSS-NUMOPT-2-INPUT-V1" and
            node.get("receipt_version") == "KSS-NUMOPT-2-NODE-V1",
            "receipt version changed")
    require(node.get("input_sha256") == input_sha and
            node.get("source_commit") == args.expected_commit and
            node.get("bundle_sha256") == args.expected_bundle,
            "node source binding changed")

    summary = one_csv(output / "summary.csv")
    workers = int(task["workers"]); firms = int(task["firms"])
    density = int(task["cells_per_worker"]); rpc = int(task["rows_per_cell"])
    cells = workers * density; rows = cells * rpc
    require(integer(summary, "input_rows") == rows and
            integer(summary, "N_stored") == rows and
            integer(summary, "N_retained") == rows, "row dimensions changed")
    require(integer(summary, "worker_levels") == workers and
            integer(summary, "firm_levels") == firms, "FE dimensions changed")
    require(integer(summary, "coefficient_cells") == cells and
            integer(summary, "deletion_units") == cells and
            integer(summary, "target_strata") == cells, "atom dimensions changed")
    require(summary["source_commit"] == args.expected_commit and
            summary["bundle_sha256"] == args.expected_bundle and
            summary["input_sha256"] == input_sha, "summary binding changed")
    require(summary["engine_selected"] == "compressed" and
            summary["estimator_status"] ==
            "KSS_SCALE_EXPERIMENTAL_POINT_ESTIMATES", "estimate not accepted")
    require(integer(summary, "actual_stata_processors") ==
            int(task["stata_processors"]), "Stata processor mismatch")
    probes = int(task["probes"])
    require(integer(summary, "rhs_count") == 3 * probes + 1,
            "RHS count changed")
    require(num(summary, "solver_max_residual") <=
            num(summary, "residual_acceptance_tolerance") and
            num(summary, "rhs_max_residual") <=
            num(summary, "residual_acceptance_tolerance"),
            "complete residual failed")
    for prefix in ("plugin", "correction", "corrected"):
        identity = (num(summary, f"{prefix}_worker") +
                    num(summary, f"{prefix}_firm") +
                    2 * num(summary, f"{prefix}_covariance"))
        require(abs(identity - num(summary, f"{prefix}_total")) <=
                2e-9 * (1 + abs(identity)), f"{prefix} identity failed")
    for required in ("generation_resources.txt", "process_resources.txt",
                     "rhs.csv", "stage_memory.csv", "application.log",
                     "generation.log"):
        require((output / required).is_file(), f"missing {required}")

    result = {
        "validation_version": "KSS-NUMOPT-2-VALIDATION-V1",
        "experiment_id": args.experiment,
        "job_id": job_id,
        "source_commit": args.expected_commit,
        "bundle_sha256": args.expected_bundle,
        "task_sha256": sha256(output / "task.tsv"),
        "input_sha256": input_sha,
        "workers": workers,
        "firms": firms,
        "cells": cells,
        "rows": rows,
        "probes": probes,
        "command_seconds": num(summary, "command_seconds"),
        "work_seconds": num(summary, "life_work_seconds"),
        "peak_model_bytes": num(summary, "resource_peak_bytes"),
        "solver_iterations": num(summary, "solver_iterations"),
        "solver_actions": num(summary, "solver_schur_actions"),
        "max_residual": num(summary, "solver_max_residual"),
        "qacct": accounting,
        "status": "PASS",
    }
    destination = output / "validation.json"
    destination.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n",
                           encoding="utf-8")
    print(f"KSS_NUMOPT2_VALIDATION_PASS {args.experiment} {job_id}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
