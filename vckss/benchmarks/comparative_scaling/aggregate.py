#!/usr/bin/env python3
"""Aggregate 300 validated SCC tasks into compact source-bound evidence."""

from __future__ import annotations

import argparse
import json
import shutil
import statistics
from pathlib import Path
from typing import Any

try:
    from .common import (
        COLLECTION_SCHEMA,
        ESTIMATORS,
        key_values,
        REPLICATES,
        RESULT_SCHEMA,
        ROW_GRID,
        CORE_GRID,
        STRUCTURES,
        TARGETS,
        load_json,
        require,
        sha256,
        write_tsv,
    )
except ImportError:
    from common import (  # type: ignore
        COLLECTION_SCHEMA,
        ESTIMATORS,
        key_values,
        REPLICATES,
        RESULT_SCHEMA,
        ROW_GRID,
        CORE_GRID,
        STRUCTURES,
        TARGETS,
        load_json,
        require,
        sha256,
        write_tsv,
    )


RESULT_FIELDS = (
    "result_schema", "task_id", "experiment_id", "source_commit",
    "bundle_sha256", "structure", "connectivity", "cells_per_worker",
    "rows", "workers", "firms", "active_cores", "replicate", "seed",
    "execution_position", "hostname", "cpu_model", "role", "scientific_status",
    "application_exit_status",
    "monitor_exit_status", "timed_out", "command_seconds", "process_wall_seconds",
    "process_cpu_seconds", "import_seconds", "pool_startup_seconds",
    "pool_teardown_seconds", "selection_seconds", "graph_seconds",
    "compression_seconds", "setup_seconds", "work_seconds", "fit_seconds",
    "leverage_seconds", "target_seconds", "correction_seconds", "rng_seconds",
    "schur_seconds", "pcg_seconds", "estimator_phase_peak_rss_bytes",
    "whole_process_peak_rss_bytes", "gnu_peak_rss_bytes", "qacct_maxvmem_bytes",
    "resource_peak_bytes", "memory_forecast_bytes", "cmg_admitted_peak_bytes",
    "cmg_actual_retained_bytes", "cmg_threads_requested", "cmg_threads_used",
    "cmg_fit_tolerance", "cmg_probe_tolerance", "cmg_graph_seconds",
    "cmg_hierarchy_seconds", "cmg_rhs_seconds", "cmg_solve_seconds",
    "cmg_extraction_seconds", "cmg_operator_applications",
    "cmg_preconditioner_applications", "max_complete_residual",
    "residual_acceptance", "solver_iterations", "cmg_backend",
    "matlab_pcg_converged", "matlab_pcg_termination_iteration",
    "matlab_pcg_returned_iteration", "matlab_pcg_relative_residual",
    "matlab_tolerance_policy", "matlab_rng_policy", "corrected_worker", "corrected_firm",
    "corrected_covariance", "corrected_total", "mcse_worker", "mcse_firm",
    "mcse_covariance", "mcse_total", "rust_mata_gate",
)

CELL_FIELDS = (
    "structure", "connectivity", "cells_per_worker", "rows", "workers", "firms",
    "active_cores", "role", "cell_status", "successful_repetitions",
    "command_seconds_median", "command_seconds_min", "command_seconds_max",
    "phase_rss_bytes_median", "phase_rss_bytes_min", "phase_rss_bytes_max",
    "process_rss_bytes_median", "process_rss_bytes_min", "process_rss_bytes_max",
    "distinct_hosts", "distinct_cpu_models",
    "rust_to_matlab_time_ratio", "rust_to_mata_time_ratio",
    "rust_to_matlab_memory_ratio", "rust_to_mata_memory_ratio", "fastest_role",
)

SCHEDULER_FIELDS = (
    "task_id", "experiment_id", "jobnumber", "taskid", "hostname",
    "qacct_wall_seconds", "qacct_cpu_seconds", "qacct_maxvmem_bytes",
    "validation_sha256",
)

INPUT_HASH_FIELDS = ("structure", "rows", "input_sha256")


def blank(value: Any) -> Any:
    return "" if value is None else value


def role_row(payload: dict[str, Any], role: str) -> dict[str, Any]:
    task = payload["task"]
    value = payload["roles"][role]
    order = task["execution_order"].split(",")
    targets = value.get("targets") or {}
    mcse = value.get("mcse") or {}
    return {
        "result_schema": RESULT_SCHEMA,
        "task_id": task["task_id"],
        "experiment_id": task["experiment_id"],
        "source_commit": task["source_commit"],
        "bundle_sha256": task["bundle_sha256"],
        "structure": task["structure"],
        "connectivity": task["connectivity"],
        "cells_per_worker": task["cells_per_worker"],
        "rows": task["rows"],
        "workers": task["workers"],
        "firms": task["firms"],
        "active_cores": task["active_cores"],
        "replicate": task["replicate"],
        "seed": task["seed"],
        "execution_position": order.index(role) + 1,
        "hostname": payload["node"]["hostname"],
        "cpu_model": payload["node"]["cpu_model"],
        "role": role,
        "scientific_status": value["scientific_status"],
        "application_exit_status": value["application_exit_status"],
        "monitor_exit_status": value["monitor_exit_status"],
        "timed_out": int(value["timed_out"]),
        "command_seconds": blank(value.get("command_seconds")),
        "process_wall_seconds": blank(value.get("process_wall_seconds")),
        "process_cpu_seconds": blank(value.get("process_cpu_seconds")),
        "import_seconds": blank(value.get("import_seconds")),
        "pool_startup_seconds": blank(value.get("pool_startup_seconds")),
        "pool_teardown_seconds": blank(value.get("pool_teardown_seconds")),
        **{name: blank(value.get(name)) for name in (
            "selection_seconds", "graph_seconds", "compression_seconds",
            "setup_seconds", "work_seconds", "fit_seconds",
            "leverage_seconds", "target_seconds", "correction_seconds",
            "rng_seconds", "schur_seconds", "pcg_seconds",
        )},
        "estimator_phase_peak_rss_bytes": blank(
            value.get("estimator_phase_peak_rss_bytes")),
        "whole_process_peak_rss_bytes": blank(
            value.get("whole_process_peak_rss_bytes")),
        "gnu_peak_rss_bytes": blank(value.get("gnu_peak_rss_bytes")),
        "qacct_maxvmem_bytes": payload["qacct"]["maxvmem_bytes"],
        "resource_peak_bytes": blank(value.get("resource_peak_bytes")),
        "memory_forecast_bytes": blank(value.get("memory_forecast_bytes")),
        "cmg_admitted_peak_bytes": blank(value.get("cmg_admitted_peak_bytes")),
        "cmg_actual_retained_bytes": blank(value.get("cmg_actual_retained_bytes")),
        **{name: blank(value.get(name)) for name in (
            "cmg_threads_requested", "cmg_threads_used", "cmg_fit_tolerance",
            "cmg_probe_tolerance", "cmg_graph_seconds", "cmg_hierarchy_seconds",
            "cmg_rhs_seconds", "cmg_solve_seconds", "cmg_extraction_seconds",
            "cmg_operator_applications", "cmg_preconditioner_applications",
        )},
        "max_complete_residual": blank(value.get("max_complete_residual")),
        "residual_acceptance": blank(value.get("residual_acceptance")),
        "solver_iterations": blank(value.get("solver_iterations")),
        "cmg_backend": blank(value.get("cmg_backend")),
        **{name: blank(value.get(name)) for name in (
            "matlab_pcg_converged", "matlab_pcg_termination_iteration",
            "matlab_pcg_returned_iteration", "matlab_pcg_relative_residual",
            "matlab_tolerance_policy", "matlab_rng_policy",
        )},
        **{f"corrected_{name}": blank(targets.get(name)) for name in TARGETS},
        **{f"mcse_{name}": blank(mcse.get(name)) for name in TARGETS},
        "rust_mata_gate": payload["rust_mata_independent_probe_gate"]["status"],
    }


def triplet(values: list[float]) -> tuple[float, float, float]:
    require(len(values) == 3, "complete cell must contain three repetitions")
    return statistics.median(values), min(values), max(values)


def paired_ratio(
    rows: dict[tuple[int, str], dict[str, Any]],
    numerator: str,
    denominator: str,
    field: str,
) -> float:
    return triplet([
        float(rows[(replicate, numerator)][field])
        / float(rows[(replicate, denominator)][field])
        for replicate, _, _ in REPLICATES
    ])[0]


def deterministic_input_hashes(payloads: list[dict[str, Any]]) -> list[dict[str, Any]]:
    output: list[dict[str, Any]] = []
    for structure in STRUCTURES:
        for rows in ROW_GRID:
            selected = [item for item in payloads
                        if item["task"]["structure"] == structure and
                        int(item["task"]["rows"]) == rows]
            require(len(selected) == len(CORE_GRID) * 3,
                    "input-hash cell cardinality changed")
            hashes = {str(item["input_sha256"]) for item in selected}
            require(len(hashes) == 1, "deterministic input hash changed")
            output.append({"structure": structure, "rows": rows,
                           "input_sha256": hashes.pop()})
    require(len(output) == 20, "input-hash inventory must contain 20 rows")
    return output


def preparation_identity(run_dir: Path, source_commit: str,
                         bundle_sha: str) -> tuple[dict[str, str], list[Path]]:
    receipt_dir = run_dir / "receipts" / "preparation"
    preparation = key_values(receipt_dir / "preparation.tsv")
    require(preparation.get("schema") ==
            "VCKSS-COMPARATIVE-SCALING-PREPARATION-V1" and
            preparation.get("status") == "PASS" and
            preparation.get("source_commit") == source_commit and
            preparation.get("bundle_sha256") == bundle_sha,
            "preparation receipt changed")
    require((receipt_dir / "wrapper.pass").read_text(encoding="utf-8").strip() ==
            f"VCKSS_COMPARATIVE_SCALING_PREPARE_PASS {source_commit} {bundle_sha}",
            "preparation wrapper changed")
    matlab = load_json(receipt_dir / "matlab_source_identity.json")
    require(matlab.get("status") == "PASS", "MATLAB source identity failed")
    qacct_path = receipt_dir / "qacct.txt"
    require(qacct_path.is_file() and not qacct_path.is_symlink(),
            "preparation qacct is missing")
    qacct: dict[str, str] = {}
    for line in qacct_path.read_text(encoding="utf-8").splitlines():
        fields = line.strip().split(None, 1)
        if len(fields) == 2:
            qacct[fields[0]] = fields[1]
    require(qacct.get("failed") == qacct.get("exit_status") == "0" and
            qacct.get("project") == "welfgr" and qacct.get("slots") == "4" and
            qacct.get("jobnumber") == preparation.get("job_id"),
            "preparation scheduler receipt failed")
    identity = {
        "rustc": preparation["rustc"], "cargo": preparation["cargo"],
        "stata_module": preparation["stata_module"],
        "matlab_module": preparation["matlab_module"],
        "plugin_sha256": preparation["plugin_sha256"],
        "binary_manifest_sha256": preparation["binary_manifest_sha256"],
        "matlab_upstream_commit": str(matlab["matlab_upstream_commit"]),
        "matlab_runtime_tree_sha256": str(matlab["matlab_runtime_tree_sha256"]),
        "matlab_core_sha256": str(matlab["matlab_core_sha256"]),
        "preparation_job_id": preparation["job_id"],
        "preparation_hostname": preparation["hostname"],
    }
    sources = [
        run_dir / "input" / "tasks.tsv",
        run_dir / "input" / "source.files.sha256",
        receipt_dir / "preparation.tsv",
        receipt_dir / "binary_manifest.sha256",
        receipt_dir / "matlab_source_identity.json",
        receipt_dir / "qacct.txt",
        receipt_dir / "wrapper.pass",
    ]
    require(all(path.is_file() and not path.is_symlink() for path in sources),
            "compact provenance source changed")
    return identity, sources


def cell_rows(results: list[dict[str, Any]]) -> list[dict[str, Any]]:
    grouped: dict[tuple[str, int, int], list[dict[str, Any]]] = {}
    for row in results:
        key = (str(row["structure"]), int(row["rows"]), int(row["active_cores"]))
        grouped.setdefault(key, []).append(row)
    require(len(grouped) == 100, "cell grid must contain 100 graph-size-core cells")
    output: list[dict[str, Any]] = []
    for structure in STRUCTURES:
        connectivity, degree, _ = STRUCTURES[structure]
        for rows in ROW_GRID:
            for cores in CORE_GRID:
                members = grouped[(structure, rows, cores)]
                require(len(members) == 9, "each cell must contain nine role-replicates")
                complete = (
                    all(row["scientific_status"] == "PASS" for row in members)
                    and all(row["rust_mata_gate"] == "PASS" for row in members)
                )
                medians: dict[str, dict[str, float]] = {}
                if complete:
                    for role in ESTIMATORS:
                        selected = [row for row in members if row["role"] == role]
                        medians[role] = {
                            "time": triplet([float(row["command_seconds"])
                                             for row in selected])[0],
                            "phase_rss": triplet([
                                float(row["estimator_phase_peak_rss_bytes"])
                                for row in selected])[0],
                            "process_rss": triplet([
                                float(row["whole_process_peak_rss_bytes"])
                                for row in selected])[0],
                        }
                    fastest = min(ESTIMATORS, key=lambda role: medians[role]["time"])
                else:
                    fastest = "NONE"
                distinct_hosts = len({str(row["hostname"]) for row in members})
                distinct_cpu_models = len({str(row["cpu_model"]) for row in members})
                by_replicate = {
                    (int(row["replicate"]), str(row["role"])): row
                    for row in members
                }
                for role in ESTIMATORS:
                    selected = [row for row in members if row["role"] == role]
                    successful = [row for row in selected
                                  if row["scientific_status"] == "PASS"]
                    if len(successful) == 3:
                        time_values = [float(row["command_seconds"]) for row in successful]
                        phase_values = [float(row["estimator_phase_peak_rss_bytes"])
                                        for row in successful]
                        process_values = [float(row["whole_process_peak_rss_bytes"])
                                          for row in successful]
                        time_median, time_min, time_max = triplet(time_values)
                        phase_median, phase_min, phase_max = triplet(phase_values)
                        process_median, process_min, process_max = triplet(process_values)
                    else:
                        time_median = time_min = time_max = ""
                        phase_median = phase_min = phase_max = ""
                        process_median = process_min = process_max = ""
                    ratios = {name: "" for name in (
                        "rust_to_matlab_time_ratio", "rust_to_mata_time_ratio",
                        "rust_to_matlab_memory_ratio", "rust_to_mata_memory_ratio")}
                    if complete:
                        paired = {
                            (replicate, estimator):
                                by_replicate[(replicate, estimator)]
                            for replicate, _, _ in REPLICATES
                            for estimator in ESTIMATORS
                        }
                        ratios = {
                            "rust_to_matlab_time_ratio":
                                paired_ratio(paired, "rust", "matlab",
                                             "command_seconds"),
                            "rust_to_mata_time_ratio":
                                paired_ratio(paired, "rust", "mata",
                                             "command_seconds"),
                            "rust_to_matlab_memory_ratio":
                                paired_ratio(
                                    paired, "rust", "matlab",
                                    "estimator_phase_peak_rss_bytes"),
                            "rust_to_mata_memory_ratio":
                                paired_ratio(
                                    paired, "rust", "mata",
                                    "estimator_phase_peak_rss_bytes"),
                        }
                    output.append({
                        "structure": structure,
                        "connectivity": connectivity,
                        "cells_per_worker": degree,
                        "rows": rows,
                        "workers": rows // degree,
                        "firms": rows // degree // 40,
                        "active_cores": cores,
                        "role": role,
                        "cell_status": "COMPLETE" if complete else "INCOMPLETE",
                        "successful_repetitions": len(successful),
                        "command_seconds_median": time_median,
                        "command_seconds_min": time_min,
                        "command_seconds_max": time_max,
                        "phase_rss_bytes_median": phase_median,
                        "phase_rss_bytes_min": phase_min,
                        "phase_rss_bytes_max": phase_max,
                        "process_rss_bytes_median": process_median,
                        "process_rss_bytes_min": process_min,
                        "process_rss_bytes_max": process_max,
                        "distinct_hosts": distinct_hosts,
                        "distinct_cpu_models": distinct_cpu_models,
                        **ratios,
                        "fastest_role": fastest,
                    })
    require(len(output) == 300, "cell summary must contain 300 role rows")
    return output


def collect(run_dir: Path, output_dir: Path) -> dict[str, Any]:
    require(output_dir.is_dir(), "collection output directory is missing")
    paths = sorted((run_dir / "validations").glob("*.json"))
    require(len(paths) == 300, "collection requires exactly 300 validations")
    pairs = [(path, load_json(path)) for path in paths]
    payloads = [item for _, item in pairs]
    require(all(item.get("schema") == RESULT_SCHEMA and item.get("status") == "PASS"
                for item in payloads), "validation set changed")
    pairs.sort(key=lambda pair: int(pair[1]["task"]["task_id"]))
    paths = [path for path, _ in pairs]
    payloads = [item for _, item in pairs]
    require([int(item["task"]["task_id"]) for item in payloads] ==
            list(range(1, 301)), "validation task IDs changed")
    identities = {(item["task"]["source_commit"], item["task"]["bundle_sha256"])
                  for item in payloads}
    require(len(identities) == 1, "source identities differ across tasks")
    source_commit, bundle_sha = identities.pop()
    runtime_identity, provenance_sources = preparation_identity(
        run_dir, source_commit, bundle_sha)
    results = [role_row(payload, role) for payload in payloads for role in ESTIMATORS]
    require(len(results) == 900, "result ledger must contain 900 calls")
    cells = cell_rows(results)
    scheduler = [{
        "task_id": item["task"]["task_id"],
        "experiment_id": item["task"]["experiment_id"],
        "jobnumber": item["qacct"]["jobnumber"],
        "taskid": item["qacct"]["taskid"],
        "hostname": item["qacct"]["hostname"],
        "qacct_wall_seconds": item["qacct"]["wall_seconds"],
        "qacct_cpu_seconds": item["qacct"]["cpu_seconds"],
        "qacct_maxvmem_bytes": item["qacct"]["maxvmem_bytes"],
        "validation_sha256": sha256(path),
    } for path, item in zip(paths, payloads)]
    results_path = output_dir / "results_900.tsv"
    cells_path = output_dir / "cell_summary_300.tsv"
    scheduler_path = output_dir / "scheduler_index_300.tsv"
    input_hashes_path = output_dir / "input_hashes_20.tsv"
    for target in (results_path, cells_path, scheduler_path, input_hashes_path):
        require(not target.exists(), f"collection target exists: {target}")
    write_tsv(results_path, RESULT_FIELDS, results)
    write_tsv(cells_path, CELL_FIELDS, cells)
    write_tsv(scheduler_path, SCHEDULER_FIELDS, scheduler)
    write_tsv(input_hashes_path, INPUT_HASH_FIELDS,
              deterministic_input_hashes(payloads))
    provenance_names = (
        "task_manifest_300.tsv", "source.files.sha256", "preparation.tsv",
        "binary_manifest.sha256", "matlab_source_identity.json",
        "preparation_qacct.txt", "preparation_wrapper.pass",
    )
    provenance_paths: list[Path] = []
    for source, name in zip(provenance_sources, provenance_names):
        target = output_dir / name
        require(not target.exists(), f"collection target exists: {target}")
        shutil.copy2(source, target)
        provenance_paths.append(target)
    receipt = {
        "schema": COLLECTION_SCHEMA,
        "status": "PASS",
        "source_commit": source_commit,
        "bundle_sha256": bundle_sha,
        "validated_tasks": 300,
        "estimator_calls": 900,
        "complete_cells": sum(row["cell_status"] == "COMPLETE" and
                              row["role"] == "rust" for row in cells),
        "runtime_identity": runtime_identity,
        "scientific_status_counts": {
            status: sum(row["scientific_status"] == status for row in results)
            for status in sorted({str(row["scientific_status"]) for row in results})
        },
        "artifact_sha256": {path.name: sha256(path) for path in (
            results_path, cells_path, scheduler_path, input_hashes_path,
            *provenance_paths)},
    }
    return receipt


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--run-dir", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    args = parser.parse_args()
    receipt_path = args.output_dir / "collection.json"
    require(not receipt_path.exists(), "collection receipt exists")
    value = collect(args.run_dir, args.output_dir)
    receipt_path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n",
                            encoding="utf-8")
    print("VCKSS_COMPARATIVE_SCALING_COLLECTION_PASS tasks=300 calls=900")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
