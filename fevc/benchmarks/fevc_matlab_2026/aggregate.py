#!/usr/bin/env python3
"""Aggregate accepted main cells into paper-facing FEVC--MATLAB 2026 tables."""

from __future__ import annotations

import argparse
import csv
import json
import statistics
from collections import defaultdict
from collections.abc import Iterable
from pathlib import Path
from typing import Any

try:
    from .common import ESTIMATORS, RESULT_SCHEMA, TARGETS, load_json, require, sha256
except ImportError:
    from common import (  # type: ignore
        ESTIMATORS,
        RESULT_SCHEMA,
        TARGETS,
        load_json,
        require,
        sha256,
    )


COLLECTION_SCHEMA = "FEVC-MATLAB-2026-MAIN-COLLECTION-V1"


def write_tsv(path: Path, rows: list[dict[str, Any]]) -> None:
    require(rows, f"cannot write empty table: {path.name}")
    with path.open("w", encoding="utf-8", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=tuple(rows[0]), delimiter="\t",
                                lineterminator="\n")
        writer.writeheader()
        writer.writerows(rows)


def quartiles(values: Iterable[float]) -> dict[str, float]:
    items = sorted(float(value) for value in values)
    require(len(items) == 6, "main summary requires six repetitions")
    q1, _, q3 = statistics.quantiles(items, n=4, method="inclusive")
    return {
        "median": statistics.median(items), "q1": q1, "q3": q3,
        "min": min(items), "max": max(items),
    }


def call_row(payload: dict[str, Any], role: str) -> dict[str, Any]:
    task = payload["task"]
    value = payload["roles"][role]
    order = task["execution_order"].split(",")
    row = {
        "task_id": task["task_id"], "experiment_id": task["experiment_id"],
        "source_commit": task["source_commit"], "structure": task["structure"],
        "connectivity": task["connectivity"],
        "cells_per_worker": task["cells_per_worker"], "rows": task["rows"],
        "workers": task["workers"], "firms": task["firms"],
        "active_cores": task["active_cores"], "stata_processors": task["stata_processors"],
        "replicate": task["replicate"], "seed": task["seed"],
        "execution_order": task["execution_order"],
        "execution_position": order.index(role) + 1, "role": role,
        "rankable": int(payload["rankable"]),
        "scientific_status": value["scientific_status"],
        "hostname": payload["node"]["hostname"],
        "cpu_model": payload["node"]["cpu_model"],
        "command_seconds": value["command_seconds"],
        "process_wall_seconds": value["process_wall_seconds"],
        "process_cpu_seconds": value["process_cpu_seconds"],
        "import_seconds": value["import_seconds"],
        "pool_startup_seconds": value.get("pool_startup_seconds", ""),
        "pool_teardown_seconds": value.get("pool_teardown_seconds", ""),
        "solver_iterations": value["solver_iterations"],
        "max_complete_residual": value["max_complete_residual"],
        "empty_rss_bytes": value["empty_rss_bytes"],
        "data_rss_bytes": value["data_rss_bytes"],
        "data_footprint_bytes": value["data_footprint_bytes"],
        "estimator_increment_bytes": value["estimator_increment_bytes"],
        "estimator_peak_rss_bytes": value["estimator_peak_rss_bytes"],
        "whole_process_peak_rss_bytes": value["whole_process_peak_rss_bytes"],
        "memory_amplification": value["memory_amplification"],
        "empty_pss_bytes": value["empty_pss_bytes"],
        "data_pss_bytes": value["data_pss_bytes"],
        "estimator_start_pss_bytes": value["estimator_start_pss_bytes"],
        "estimator_end_pss_bytes": value["estimator_end_pss_bytes"],
        "qacct_maxvmem_bytes": payload["qacct"]["maxvmem_bytes"],
        "input_sha256": payload["input_sha256"],
    }
    row.update({f"corrected_{name}": value["targets"][name] for name in TARGETS})
    return row


def aggregate(
    run_dir: Path,
    attempt_id: str,
    output_dir: Path,
    validation_dir: Path | None = None,
) -> dict[str, Any]:
    identity = load_json(run_dir / "run_identity.json")
    inventory_path = run_dir / "receipts" / f"{attempt_id}.generation.json"
    inventory = load_json(inventory_path)
    require(identity.get("schema") == "FEVC-MATLAB-2026-CAMPAIGN-V2" and
            identity.get("source_mode") == "CLEAN_COMMIT" and
            inventory.get("status") == "PASS" and
            inventory.get("stage") == "production" and
            inventory.get("validated_cells") == 240,
            "production inventory is incomplete")
    validation_dir = validation_dir or run_dir / "attempts" / attempt_id / "validations"
    paths = sorted(validation_dir.glob("*.json"), key=lambda path: int(path.stem))
    require(len(paths) == 240, "production must contain 240 cell validations")
    payloads = [load_json(path) for path in paths]
    require(all(value.get("schema") == RESULT_SCHEMA and value.get("status") == "PASS"
                for value in payloads), "cell validation schema changed")

    calls = [call_row(payload, role) for payload in payloads for role in ESTIMATORS]
    require(len(calls) == 480, "call inventory changed")
    paired: list[dict[str, Any]] = []
    for payload in payloads:
        task = payload["task"]
        rust = payload["roles"]["rust"]
        matlab = payload["roles"]["matlab"]
        paired.append({
            "task_id": task["task_id"], "structure": task["structure"],
            "rows": task["rows"], "workers": task["workers"], "firms": task["firms"],
            "active_cores": task["active_cores"], "replicate": task["replicate"],
            "seed": task["seed"], "execution_order": task["execution_order"],
            "hostname": payload["node"]["hostname"],
            "rankable": int(payload["rankable"]),
            "rust_seconds": rust["command_seconds"],
            "matlab_seconds": matlab["command_seconds"],
            "rust_to_matlab_time_ratio": (
                rust["command_seconds"] / matlab["command_seconds"]),
            "rust_peak_rss_bytes": rust["estimator_peak_rss_bytes"],
            "matlab_peak_rss_bytes": matlab["estimator_peak_rss_bytes"],
            "rust_to_matlab_peak_memory_ratio": (
                rust["estimator_peak_rss_bytes"] / matlab["estimator_peak_rss_bytes"]),
            "rust_data_footprint_bytes": rust["data_footprint_bytes"],
            "matlab_data_footprint_bytes": matlab["data_footprint_bytes"],
            "rust_estimator_increment_bytes": rust["estimator_increment_bytes"],
            "matlab_estimator_increment_bytes": matlab["estimator_increment_bytes"],
            "rust_iterations": rust["solver_iterations"],
            "matlab_iterations": matlab["solver_iterations"],
            **{f"scaled_gap_{name}": payload["scaled_target_gaps"][name]
               for name in TARGETS},
        })

    grouped: dict[tuple[str, int, int], list[dict[str, Any]]] = defaultdict(list)
    for row in paired:
        grouped[(row["structure"], row["rows"], row["active_cores"])].append(row)
    summaries: list[dict[str, Any]] = []
    for (structure, rows, cores), values in sorted(grouped.items()):
        require(len(values) == 6, "paired cell repetition count changed")
        summary = {"structure": structure, "rows": rows, "active_cores": cores,
                   "repetitions": 6, "rankable_repetitions": sum(v["rankable"] for v in values),
                   "distinct_hosts": len({v["hostname"] for v in values})}
        for field in (
            "rust_seconds", "matlab_seconds", "rust_to_matlab_time_ratio",
            "rust_peak_rss_bytes", "matlab_peak_rss_bytes",
            "rust_to_matlab_peak_memory_ratio", "rust_data_footprint_bytes",
            "matlab_data_footprint_bytes", "rust_estimator_increment_bytes",
            "matlab_estimator_increment_bytes", "rust_iterations", "matlab_iterations",
        ):
            summary.update({f"{field}_{name}": value
                            for name, value in quartiles(v[field] for v in values).items()})
        summaries.append(summary)
    require(len(summaries) == 40, "main paired-summary cell count changed")

    output_dir.mkdir(parents=True)
    calls_path = output_dir / "calls.tsv"
    paired_path = output_dir / "paired_repetitions.tsv"
    summary_path = output_dir / "paired_summary.tsv"
    write_tsv(calls_path, calls)
    write_tsv(paired_path, paired)
    write_tsv(summary_path, summaries)
    receipt = {
        "schema": COLLECTION_SCHEMA, "status": "PASS",
        "run_id": identity["run_id"], "attempt_id": attempt_id,
        "source_commit": identity["source_commit"],
        "collector_source_commit": inventory["collector_source_commit"],
        "validated_cells": 240, "accepted_calls": 480,
        "rankable_calls": sum(row["rankable"] for row in calls),
        "calls_sha256": sha256(calls_path),
        "paired_repetitions_sha256": sha256(paired_path),
        "paired_summary_sha256": sha256(summary_path),
        "generation_inventory_sha256": sha256(inventory_path),
    }
    (output_dir / "collection.json").write_text(
        json.dumps(receipt, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return receipt


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--run-dir", type=Path, required=True)
    parser.add_argument("--attempt-id", default="production")
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--validation-dir", type=Path)
    args = parser.parse_args()
    require(not args.output_dir.exists(), "collection target exists")
    value = aggregate(
        args.run_dir, args.attempt_id, args.output_dir, args.validation_dir,
    )
    print("FEVC_MATLAB_2026_COLLECTION_PASS "
          f"cells={value['validated_cells']} calls={value['accepted_calls']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
