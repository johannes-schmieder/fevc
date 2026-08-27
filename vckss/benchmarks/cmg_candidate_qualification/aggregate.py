#!/usr/bin/env python3
"""Apply every registered promotion gate to 72 paired SCC validations."""

from __future__ import annotations

import argparse
import json
import statistics
from pathlib import Path

from common import (
    ACCEPTANCE_SCHEMA,
    CORES,
    RESULT_SCHEMA,
    STRUCTURES,
    geometric_mean,
    load_json,
    require,
    sha256,
    write_tsv,
)

TASK_FIELDS = (
    "task_id", "experiment_id", "structure", "active_cores", "replicate",
    "execution_order", "hostname", "cpu_model", "connected_vector_only",
    "candidate_command_seconds", "comparison_command_seconds", "time_ratio",
    "candidate_phase_rss_bytes", "comparison_phase_rss_bytes", "rss_ratio",
    "candidate_planned_batches", "comparison_planned_batches",
    "candidate_serial_batches", "comparison_serial_batches",
)
CELL_FIELDS = (
    "structure", "active_cores", "paired_tasks", "median_time_ratio",
    "geometric_mean_time_ratio", "geometric_mean_rss_ratio",
    "connected_vector_only", "cell_gate", "vector_improvement_gate",
)


def summarize(payloads: list[dict]) -> tuple[list[dict], list[dict], dict]:
    require(len(payloads) == 72 and
            [int(item["task"]["task_id"]) for item in payloads] == list(range(1, 73)),
            "qualification requires exact tasks 1..72")
    require(all(item.get("schema") == RESULT_SCHEMA and item.get("status") == "PASS" and
                item.get("statistical_gate", {}).get("status") == "PASS" and
                item.get("qacct", {}).get("failed") == "0" and
                item.get("qacct", {}).get("exit_status") == "0"
                for item in payloads), "application, scientific, wrapper, or qacct gate failed")
    bindings = {(item["task"]["candidate_commit"], item["task"]["comparison_commit"],
                 item["task"]["candidate_bundle_sha256"], item["task"]["comparison_bundle_sha256"],
                 item["node"]["candidate_binary_manifest_sha256"],
                 item["node"]["comparison_binary_manifest_sha256"])
                for item in payloads}
    require(len(bindings) == 1, "source or binary identity differs across tasks")
    task_rows = []
    for item in payloads:
        task = item["task"]
        candidate = item["roles"]["candidate"]
        comparison = item["roles"]["comparison"]
        time_ratio = candidate["command_seconds"] / comparison["command_seconds"]
        rss_ratio = (candidate["estimator_phase_peak_rss_bytes"] /
                     comparison["estimator_phase_peak_rss_bytes"])
        task_rows.append({
            "task_id": task["task_id"], "experiment_id": task["experiment_id"],
            "structure": task["structure"], "active_cores": task["active_cores"],
            "replicate": task["replicate"], "execution_order": task["execution_order"],
            "hostname": item["node"]["hostname"], "cpu_model": item["node"]["cpu_model"],
            "connected_vector_only": int(item["connected_vector_only"]),
            "candidate_command_seconds": candidate["command_seconds"],
            "comparison_command_seconds": comparison["command_seconds"],
            "time_ratio": time_ratio,
            "candidate_phase_rss_bytes": candidate["estimator_phase_peak_rss_bytes"],
            "comparison_phase_rss_bytes": comparison["estimator_phase_peak_rss_bytes"],
            "rss_ratio": rss_ratio,
            "candidate_planned_batches": candidate["cmg_planned_batches"],
            "comparison_planned_batches": comparison["cmg_planned_batches"],
            "candidate_serial_batches": candidate["cmg_serial_batches"],
            "comparison_serial_batches": comparison["cmg_serial_batches"],
        })
    cells = []
    for structure in STRUCTURES:
        for cores in CORES:
            selected = [row for row in task_rows if row["structure"] == structure and
                        int(row["active_cores"]) == cores]
            require(len(selected) == 6, "cell repetition count changed")
            median_ratio = statistics.median(row["time_ratio"] for row in selected)
            vector_only = cores > 1 and all(row["connected_vector_only"] == 1
                                            for row in selected)
            cells.append({
                "structure": structure, "active_cores": cores, "paired_tasks": 6,
                "median_time_ratio": median_ratio,
                "geometric_mean_time_ratio": geometric_mean(row["time_ratio"] for row in selected),
                "geometric_mean_rss_ratio": geometric_mean(row["rss_ratio"] for row in selected),
                "connected_vector_only": int(vector_only),
                "cell_gate": "PASS" if median_ratio <= 1.05 else "FAIL",
                "vector_improvement_gate": ("PASS" if vector_only and median_ratio < 1.0 else
                                             "NOT_APPLICABLE" if not vector_only else "FAIL"),
            })
    parallel = [row["time_ratio"] for row in task_rows if int(row["active_cores"]) in (8, 16)]
    one_core = [row for row in task_rows if int(row["active_cores"]) == 1]
    parallel_gm = geometric_mean(parallel)
    one_core_time = geometric_mean(row["time_ratio"] for row in one_core)
    one_core_rss = geometric_mean(row["rss_ratio"] for row in one_core)
    vector_cells = {cores: [row for row in cells if int(row["active_cores"]) == cores and
                            row["connected_vector_only"] == 1]
                    for cores in (8, 16)}
    require(parallel_gm <= 1.00, "parallel geometric-mean time gate failed")
    require(all(row["cell_gate"] == "PASS" for row in cells),
            "graph/core median regression gate failed")
    require(all(vector_cells[cores] and all(row["vector_improvement_gate"] == "PASS"
                                            for row in vector_cells[cores])
                for cores in (8, 16)), "connected vector-only improvement gate failed")
    require(one_core_time <= 1.03, "one-core command-time gate failed")
    require(one_core_rss <= 1.05, "one-core peak-RSS gate failed")
    candidate_commit, comparison_commit, candidate_bundle, comparison_bundle, candidate_binary, comparison_binary = bindings.pop()
    receipt = {
        "schema": ACCEPTANCE_SCHEMA, "status": "PASS",
        "candidate_commit": candidate_commit, "comparison_commit": comparison_commit,
        "candidate_bundle_sha256": candidate_bundle,
        "comparison_bundle_sha256": comparison_bundle,
        "candidate_binary_manifest_sha256": candidate_binary,
        "comparison_binary_manifest_sha256": comparison_binary,
        "validated_tasks": 72, "accepted_candidate_calls": 72,
        "accepted_comparison_calls": 72,
        "parallel_geometric_mean_command_time_ratio": parallel_gm,
        "maximum_graph_core_median_time_ratio": max(row["median_time_ratio"] for row in cells),
        "one_core_geometric_mean_command_time_ratio": one_core_time,
        "one_core_geometric_mean_phase_rss_ratio": one_core_rss,
        "connected_vector_only_cells": {str(core): len(vector_cells[core]) for core in (8, 16)},
        "connected_vector_only_maximum_median_ratio": {
            str(core): max(row["median_time_ratio"] for row in vector_cells[core])
            for core in (8, 16)},
    }
    return task_rows, cells, receipt


def aggregate(run_dir: Path, output_dir: Path) -> dict:
    paths = sorted((run_dir / "validations").glob("*.json"), key=lambda path: int(path.stem))
    payloads = [load_json(path) for path in paths]
    identity = load_json(run_dir / "run_identity.json")
    preparation = load_json(run_dir / "receipts" / "preparation" / "qacct.pass.json")
    require(identity.get("status") == preparation.get("status") == "PASS" and
            identity.get("candidate_commit") == preparation.get("candidate_commit") and
            identity.get("comparison_commit") == preparation.get("comparison_commit") and
            identity.get("required_stata_processors") ==
            preparation.get("required_stata_processors") == 16 and
            int(preparation.get("licensed_stata_processors", 0)) >= 16,
            "preparation/run identity changed")
    require(all(item["task"]["candidate_commit"] == identity["candidate_commit"] and
                item["task"]["comparison_commit"] == identity["comparison_commit"] and
                item["task"]["candidate_bundle_sha256"] == identity["candidate_bundle_sha256"] and
                item["task"]["comparison_bundle_sha256"] == identity["comparison_bundle_sha256"]
                for item in payloads), "validation source identity changed")
    for structure in STRUCTURES:
        hashes = {item["input_sha256"] for item in payloads
                  if item["task"]["structure"] == structure}
        require(len(hashes) == 1, f"{structure} input is not deterministic across tasks")
    tasks, cells, receipt = summarize(payloads)
    tasks_path = output_dir / "paired_tasks_72.tsv"
    cells_path = output_dir / "graph_core_cells_12.tsv"
    require(not tasks_path.exists() and not cells_path.exists(), "collection target exists")
    write_tsv(tasks_path, TASK_FIELDS, tasks)
    write_tsv(cells_path, CELL_FIELDS, cells)
    receipt["artifact_sha256"] = {tasks_path.name: sha256(tasks_path),
                                  cells_path.name: sha256(cells_path)}
    receipt.update({
        "candidate_source_manifest_sha256":
            identity["candidate_source_manifest_sha256"],
        "comparison_source_manifest_sha256":
            identity["comparison_source_manifest_sha256"],
        "task_manifest_sha256": identity["task_manifest_sha256"],
        "stata_spi_manifest_sha256": identity["stata_spi_manifest_sha256"],
        "required_stata_processors": identity["required_stata_processors"],
        "licensed_stata_processors": preparation["licensed_stata_processors"],
        "stata_processor_capability_sha256":
            preparation["stata_processor_capability_sha256"],
        "preparation_qacct_receipt_sha256": sha256(
            run_dir / "receipts" / "preparation" / "qacct.pass.json"),
    })
    return receipt


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--run-dir", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    args = parser.parse_args()
    receipt_path = args.output_dir / "acceptance.json"
    require(args.output_dir.is_dir() and not receipt_path.exists(), "invalid collection output")
    receipt = aggregate(args.run_dir, args.output_dir)
    receipt_path.write_text(json.dumps(receipt, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print("VCKSS_CMG_CANDIDATE_QUALIFICATION_ACCEPTANCE_PASS tasks=72 calls=144")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
