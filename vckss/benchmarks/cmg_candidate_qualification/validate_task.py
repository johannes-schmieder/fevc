#!/usr/bin/env python3
"""Validate one same-input/same-CPU candidate-comparison SCC task."""

from __future__ import annotations

import argparse
import json
import math
import re
from pathlib import Path
from typing import Any

from common import (
    CANDIDATE_CMG_COMMIT,
    COMPARISON_CMG_COMMIT,
    NODE_SCHEMA,
    RESULT_SCHEMA,
    finite,
    integer,
    key_values,
    load_json,
    one_csv,
    parse_memory,
    parse_qacct,
    read_single_task,
    read_task,
    require,
    sha256,
)

TARGETS = ("worker", "firm", "covariance", "total")
RUST_PHASE_FIELDS = (
    "rust_ingest_seconds",
    "rust_canonicalize_seconds",
    "rust_graph_seconds",
    "rust_compress_seconds",
    "rust_plan_seconds",
    "rust_stayer_augmentation_seconds",
    "rust_solve_seconds",
    "rust_native_total_seconds",
)


def validate_rust_phases(result: dict[str, str], label: str) -> dict[str, float]:
    """Validate the documented native profile without inventing legacy phases."""
    phases = {field: finite(result.get(field), f"{label} {field}")
              for field in RUST_PHASE_FIELDS}
    total = phases["rust_native_total_seconds"]
    require(all(0 <= value <= total for value in phases.values()),
            f"{label} native Rust phase timing changed")
    return phases


def parse_gnu_time(path: Path) -> dict[str, float | int]:
    source = path.read_text(encoding="utf-8", errors="replace")
    rss = re.search(r"Maximum resident set size \(kbytes\):\s*([0-9]+)", source)
    user = re.search(r"User time \(seconds\):\s*([0-9.]+)", source)
    system = re.search(r"System time \(seconds\):\s*([0-9.]+)", source)
    elapsed = re.search(r"Elapsed \(wall clock\) time \(h:mm:ss or m:ss\):\s*([0-9:.]+)", source)
    require(all(item is not None for item in (rss, user, system, elapsed)),
            "incomplete GNU time receipt")
    assert rss and user and system and elapsed
    pieces = [float(item) for item in elapsed.group(1).split(":")]
    wall = pieces[-1] + 60 * pieces[-2] + (3600 * pieces[0] if len(pieces) == 3 else 0)
    return {"maximum_rss_bytes": int(rss.group(1)) * 1024,
            "cpu_seconds": float(user.group(1)) + float(system.group(1)),
            "wall_seconds": wall}


def same_host(left: str, right: str) -> bool:
    return bool(left and right and left.split(".", 1)[0] == right.split(".", 1)[0])


def parse_cpu_set(value: str) -> set[int]:
    require(re.fullmatch(r"[0-9]+(?:-[0-9]+)?(?:,[0-9]+(?:-[0-9]+)?)*", value)
            is not None, "scheduler CPU affinity syntax changed")
    cpus: set[int] = set()
    for item in value.split(","):
        bounds = item.split("-", 1)
        start = int(bounds[0])
        end = int(bounds[-1])
        require(end >= start, "scheduler CPU affinity range changed")
        cpus.update(range(start, end + 1))
    return cpus


def validate_cpu_block(node: dict[str, str]) -> tuple[set[int], set[int], int, int]:
    scheduler_cpus = parse_cpu_set(node["scheduler_cpu_affinity"])
    assigned_cpus = parse_cpu_set(node["assigned_cpu_affinity"])
    block_index = integer(node.get("cpu_block_index"), "CPU block index")
    block_capacity = integer(node.get("cpu_block_capacity"), "CPU block capacity", 1)
    expected_capacity = len(scheduler_cpus) // 16
    ordered_cpus = sorted(scheduler_cpus)
    expected_block = set(ordered_cpus[block_index * 16:(block_index + 1) * 16])
    require(len(scheduler_cpus) >= 16 and len(assigned_cpus) == 16 and
            block_capacity == expected_capacity and assigned_cpus == expected_block and
            0 <= block_index < block_capacity and
            node.get("binding_enforcement") == "HARNESS_TASKSET_FLOCK_V1",
            "runtime CPU block binding changed")
    return scheduler_cpus, assigned_cpus, block_index, block_capacity


def validate_role(job_dir: Path, label: str, task: dict[str, str], task_sha: str,
                  input_sha: str) -> dict[str, Any]:
    role_dir = job_dir / label
    status = key_values(role_dir / "status.tsv")
    require(status.get("schema") ==
            "VCKSS-CMG-CANDIDATE-QUALIFICATION-ROLE-STATUS-V2" and
            status.get("label") == label, f"{label} role status changed")
    require(status.get("application_exit_status") == "0" and
            status.get("monitor_exit_status") == "0" and
            status.get("timed_out") == "0" and
            status.get("application_receipt_valid") == "1",
            f"{label} application failed")
    cores = int(task["active_cores"])
    cpus = status.get("cpu_affinity", "").split(",")
    require(integer(status.get("active_cores"), f"{label} cores", 1) == cores and
            integer(status.get("stata_processors"),
                    f"{label} Stata processors", 1) ==
            int(task["stata_processors"]) and
            integer(status.get("rust_threads"), f"{label} Rust threads", 1) ==
            int(task["rust_threads"]) and
            len(cpus) == cores and len(set(cpus)) == cores and
            all(cpu.isdigit() for cpu in cpus), f"{label} affinity changed")
    result = one_csv(role_dir / "result.csv")
    expected_commit = task[f"{label}_commit"]
    expected_cmg = CANDIDATE_CMG_COMMIT if label == "candidate" else COMPARISON_CMG_COMMIT
    require(result.get("schema") == "VCKSS-CMG-CANDIDATE-QUALIFICATION-STATA-V3" and
            result.get("application_status") == "PASS" and
            result.get("label") == label and
            result.get("source_commit") == expected_commit and
            result.get("cmg_source_commit") == expected_cmg and
            result.get("task_sha256") == task_sha and
            result.get("input_sha256") == input_sha,
            f"{label} source/application identity changed")
    for field in ("rows", "workers", "firms", "cells_per_worker", "probes",
                  "seed", "active_cores", "sample_count"):
        expected = task[field] if field != "sample_count" else task["rows"]
        require(integer(result.get(field), f"{label} {field}") == int(expected),
                f"{label} dimension or sample changed")
    require(integer(result.get("stata_processors"),
                    f"{label} Stata processors", 1) ==
            int(task["stata_processors"]) and
            integer(result.get("rust_threads"), f"{label} Rust threads", 1) ==
            int(task["rust_threads"]) and
            integer(result.get("cmg_threads_requested"),
                    f"{label} requested threads", 1) ==
            int(task["rust_threads"]) and
            integer(result.get("cmg_threads_used"), f"{label} used threads", 1) ==
            int(task["rust_threads"]), f"{label} thread receipt changed")
    require(result.get("data_restored") == result.get("rng_restored") ==
            result.get("sort_rng_restored") == "1", f"{label} caller state changed")
    residual = finite(result.get("max_complete_residual"), f"{label} residual")
    acceptance = finite(result.get("residual_acceptance"), f"{label} acceptance")
    require(residual <= acceptance and
            finite(result.get("target_identity_residual"), f"{label} identity") <= 1e-12,
            f"{label} numerical gate failed")
    require(abs(finite(result.get("cmg_fit_tolerance"), f"{label} fit tolerance") - 1e-10) <= 1e-20 and
            abs(finite(result.get("cmg_probe_tolerance"), f"{label} probe tolerance") - 1e-6) <= 1e-16,
            f"{label} tolerance changed")
    tree = load_json(role_dir / "process_tree.json")
    require(tree.get("schema") == "VCKSS-COMPARATIVE-SCALING-PROCESS-TREE-V1" and
            tree.get("status") == "PASS" and tree.get("phase_start_observed") is True and
            tree.get("phase_end_observed") is True, f"{label} process tree failed")
    resources = parse_gnu_time(role_dir / "resources.txt")
    command_bytes = int(task["command_memory_gib"]) * 1024**3
    phase_rss = integer(tree.get("phase_peak_rss_kib"), f"{label} phase RSS", 1) * 1024
    whole_rss = integer(tree.get("whole_peak_rss_kib"), f"{label} whole RSS", 1) * 1024
    forecast = integer(result.get("memory_forecast_bytes"), f"{label} forecast", 1)
    resource_peak = integer(result.get("resource_peak_bytes"), f"{label} resource peak", 1)
    require(max(phase_rss, whole_rss, forecast, resource_peak) <= command_bytes,
            f"{label} command memory envelope failed")
    targets = {name: finite(result.get(f"corrected_{name}"), f"{label} {name}")
               for name in TARGETS}
    mcse = {name: finite(result.get(f"mcse_{name}"), f"{label} MCSE {name}")
            for name in TARGETS}
    rust_phases = validate_rust_phases(result, label)
    return {
        "status": "PASS", "source_commit": expected_commit,
        "cmg_source_commit": expected_cmg,
        "active_cores": cores,
        "stata_processors": int(task["stata_processors"]),
        "rust_threads": int(task["rust_threads"]),
        "cpu_affinity": [int(cpu) for cpu in cpus],
        "command_seconds": finite(result.get("command_seconds"), f"{label} command"),
        "process_wall_seconds": resources["wall_seconds"],
        "process_cpu_seconds": resources["cpu_seconds"],
        "gnu_peak_rss_bytes": resources["maximum_rss_bytes"],
        "estimator_phase_peak_rss_bytes": phase_rss,
        "whole_process_peak_rss_bytes": whole_rss,
        "resource_peak_bytes": resource_peak, "memory_forecast_bytes": forecast,
        "targets": targets, "mcse": mcse, "max_complete_residual": residual,
        "residual_acceptance": acceptance,
        **rust_phases,
        **{field: finite(result.get(field), f"{label} {field}") for field in (
            "solver_iterations", "cmg_maximum_concurrency", "cmg_plan_bytes",
            "cmg_admitted_peak_bytes", "cmg_batch_calls", "cmg_rhs_count",
            "cmg_serial_batches", "cmg_planned_batches", "cmg_across_rhs_batches",
            "cmg_operator_applications", "cmg_preconditioner_applications",
            "cmg_graph_seconds", "cmg_hierarchy_seconds", "cmg_rhs_seconds",
            "cmg_solve_seconds", "cmg_extraction_seconds", "cmg_actual_retained_bytes",
        )},
    }


def validate(run_dir: Path, task_id: int, qacct_path: Path) -> dict[str, Any]:
    canonical = read_task(run_dir / "input" / "tasks.tsv", task_id)
    job_dir = run_dir / "tasks" / canonical["experiment_id"]
    task = read_single_task(job_dir / "task.tsv")
    require(task == canonical, "task row differs from immutable manifest")
    task_sha = sha256(job_dir / "task.tsv")
    require((job_dir / "task.sha256").read_text().strip() == task_sha,
            "task hash changed")
    input_sha = (job_dir / "input.sha256").read_text().strip()
    require(re.fullmatch(r"[0-9a-f]{64}", input_sha) is not None, "invalid input hash")
    input_receipt = one_csv(job_dir / "input_receipt.csv")
    require(input_receipt.get("schema") == "VCKSS-COMPARATIVE-SCALING-INPUT-V6" and
            input_receipt.get("structure") == task["structure"] and
            input_receipt.get("connectivity") == task["connectivity"],
            "literal input identity changed")
    for field in ("rows", "workers", "firms", "cells_per_worker"):
        require(integer(input_receipt.get(field), f"input {field}") == int(task[field]),
                f"literal input {field} changed")
    if task["structure"] == "weak_d3":
        weak_leaves = int(task["workers"]) // 5
        require(input_receipt.get("topology_contract") ==
                "shallow_hub_tree_leaf_panel_vector_v1" and
                integer(input_receipt.get("weak_hub_firms"),
                        "weak hub firms") == 1_601 and
                integer(input_receipt.get("weak_leaf_firms"),
                        "weak leaf firms") == weak_leaves == 131_072 and
                integer(input_receipt.get("weak_panel_layers"),
                        "weak panel layers") == 5 and
                integer(input_receipt.get("weak_branch_firms"),
                        "weak branch firms") == 40 and
                integer(input_receipt.get("weak_grandchildren_per_branch"),
                        "weak grandchildren per branch") == 39 and
                integer(input_receipt.get("weak_pattern_stride"),
                        "weak pattern stride") == 5 and
                integer(input_receipt.get("weak_root_patterns"),
                        "weak root patterns") == 7 and
                integer(input_receipt.get("weak_hub_leaf_edges"),
                        "weak hub-leaf edges") == 786_432 and
                integer(input_receipt.get("weak_hub_tree_edges"),
                        "weak hub-tree edges") == 1_600 and
                integer(input_receipt.get("weak_canonical_edges"),
                        "weak canonical edges") == 788_032,
                "weak input topology changed")
    else:
        require(input_receipt.get("topology_contract") ==
                "multi_offset_long_range_v1" and
                integer(input_receipt.get("weak_hub_firms"),
                        "strong weak-hub firms") == 0 and
                integer(input_receipt.get("weak_leaf_firms"),
                        "strong weak-leaf firms") == 0 and
                integer(input_receipt.get("weak_panel_layers"),
                        "strong weak-panel layers") == 0 and
                integer(input_receipt.get("weak_branch_firms"),
                        "strong weak-branch firms") == 0 and
                integer(input_receipt.get("weak_grandchildren_per_branch"),
                        "strong weak-grandchildren per branch") == 0 and
                integer(input_receipt.get("weak_pattern_stride"),
                        "strong weak-pattern stride") == 0 and
                integer(input_receipt.get("weak_root_patterns"),
                        "strong weak-root patterns") == 0 and
                integer(input_receipt.get("weak_hub_leaf_edges"),
                        "strong weak hub-leaf edges") == 0 and
                integer(input_receipt.get("weak_hub_tree_edges"),
                        "strong weak hub-tree edges") == 0 and
                integer(input_receipt.get("weak_canonical_edges"),
                        "strong weak canonical edges") == 0,
                "strong input topology changed")
    node = key_values(job_dir / "node_receipt.tsv")
    require(node.get("schema") == NODE_SCHEMA and node.get("status") == "PASS" and
            node.get("task_id") == str(task_id) and
            node.get("experiment_id") == task["experiment_id"] and
            node.get("candidate_commit") == task["candidate_commit"] and
            node.get("comparison_commit") == task["comparison_commit"] and
            node.get("candidate_bundle_sha256") == task["candidate_bundle_sha256"] and
            node.get("comparison_bundle_sha256") == task["comparison_bundle_sha256"] and
            node.get("task_sha256") == task_sha and node.get("input_sha256") == input_sha,
            "node/source identity changed")
    preparation = load_json(run_dir / "receipts" / "preparation" / "qacct.pass.json")
    require(preparation.get("schema") ==
            "VCKSS-CMG-CANDIDATE-QUALIFICATION-PREPARATION-QACCT-V2" and
            preparation.get("status") == "PASS" and
            preparation.get("benchmark_thread_contract") ==
            "VCKSS-BENCHMARK-THREADS-V1" and
            bool(preparation.get("benchmark_ado_adapter_sha256")) and
            bool(preparation.get("candidate_benchmark_ado_receipt_sha256")) and
            bool(preparation.get("comparison_benchmark_ado_receipt_sha256")) and
            node.get("candidate_binary_manifest_sha256") ==
            preparation.get("candidate_binary_manifest_sha256") and
            node.get("comparison_binary_manifest_sha256") ==
            preparation.get("comparison_binary_manifest_sha256") and
            preparation.get("required_stata_processors") == 4 and
            int(preparation.get("licensed_stata_processors", 0)) >= 4 and
            preparation.get("required_rust_threads") == 16,
            "node/preparation binary identity changed")
    require(node.get("requested_slots") == node.get("actual_slots") == "16" and
            integer(node.get("active_cores"), "node cores", 1) ==
            int(task["active_cores"]) and
            integer(node.get("stata_processors"), "node Stata processors", 1) ==
            int(task["stata_processors"]) and
            integer(node.get("rust_threads"), "node Rust threads", 1) ==
            int(task["rust_threads"]),
            "node resource identity changed")
    require(bool(node.get("hostname")) and bool(node.get("cpu_model")) and
            bool(node.get("scheduler_cpu_affinity")), "node identity is incomplete")
    require(node.get("python_module") == "python3/3.12.4" and
            node.get("python_version") == "Python 3.12.4" and
            node.get("python_executable") ==
            "/share/pkg.8/python3/3.12.4/install/bin/python3",
            "node Python runtime changed")
    start = finite(node.get("task_start_epoch"), "task start")
    end = finite(node.get("task_end_epoch"), "task end")
    require(end >= start, "task interval changed")
    scheduler_cpus, assigned_cpus, block_index, block_capacity = \
        validate_cpu_block(node)
    active_cpus = parse_cpu_set(node["active_cpu_affinity"])
    require(len(active_cpus) == int(task["active_cores"]) and
            active_cpus.issubset(assigned_cpus),
            "node active CPU subset changed")
    utc_pattern = r"[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9:.]+Z"
    require(re.fullmatch(utc_pattern, node.get("task_start_utc", "")) is not None and
            re.fullmatch(utc_pattern, node.get("task_end_utc", "")) is not None,
            "task UTC interval changed")
    require((job_dir / "wrapper.pass").read_text().strip() ==
            f"VCKSS_CMG_CANDIDATE_QUALIFICATION_TASK_CAPTURED {task['experiment_id']} {task_sha}" and
            not (job_dir / "wrapper.fail").exists(), "wrapper gate failed")
    qacct = parse_qacct(qacct_path)
    require(qacct["jobnumber"] == node.get("job_id") and
            qacct["taskid"] == str(task_id) and same_host(qacct["hostname"], node.get("hostname", "")),
            "qacct identity changed")
    qacct_memory = parse_memory(qacct["maxvmem"])
    require(qacct_memory <= int(task["mem_per_core_gib"]) * 16 * 1024**3,
            "qacct memory allocation failed")
    roles = {label: validate_role(job_dir, label, task, task_sha, input_sha)
             for label in ("candidate", "comparison")}
    role_cpu_sets = {tuple(role["cpu_affinity"]) for role in roles.values()}
    require(len(role_cpu_sets) == 1 and
            set(next(iter(role_cpu_sets))) == active_cpus,
            "paired roles did not use the same bound CPU subset")
    checks: dict[str, Any] = {}
    for target in TARGETS:
        left = roles["candidate"]["targets"][target]
        right = roles["comparison"]["targets"][target]
        envelope = max(1e-10 * max(1.0, abs(left), abs(right)),
                       6.0 * math.hypot(roles["candidate"]["mcse"][target],
                                        roles["comparison"]["mcse"][target]))
        difference = abs(left - right)
        checks[target] = {"difference": difference, "envelope": envelope,
                          "accepted": difference <= envelope}
    require(all(value["accepted"] for value in checks.values()),
            "candidate-comparison statistical gate failed")
    vector_only = (int(task["active_cores"]) > 1 and
                   roles["candidate"]["cmg_plan_bytes"] == 0 and
                   roles["comparison"]["cmg_plan_bytes"] == 0 and
                   roles["candidate"]["cmg_planned_batches"] > 0 and
                   roles["comparison"]["cmg_planned_batches"] == 0)
    return {
        "schema": RESULT_SCHEMA, "status": "PASS", "task": task,
        "task_sha256": task_sha, "input_sha256": input_sha,
        "node": {"hostname": node["hostname"], "cpu_model": node["cpu_model"],
                 "scheduler_cpu_affinity": node["scheduler_cpu_affinity"],
                 "assigned_cpu_affinity": node["assigned_cpu_affinity"],
                 "cpu_block_index": block_index,
                 "cpu_block_capacity": block_capacity,
                 "binding_enforcement": node["binding_enforcement"],
                 "python_module": node["python_module"],
                 "python_executable": node["python_executable"],
                 "python_version": node["python_version"],
                 "candidate_binary_manifest_sha256": node["candidate_binary_manifest_sha256"],
                 "comparison_binary_manifest_sha256": node["comparison_binary_manifest_sha256"],
                 "task_start_epoch": start, "task_end_epoch": end},
        "qacct": {"jobnumber": qacct["jobnumber"], "taskid": qacct["taskid"],
                  "hostname": qacct["hostname"], "failed": qacct["failed"],
                  "exit_status": qacct["exit_status"],
                  "wall_seconds": finite(qacct["ru_wallclock"], "qacct wall"),
                  "cpu_seconds": finite(qacct["cpu"], "qacct CPU"),
                  "maxvmem_bytes": qacct_memory},
        "roles": roles, "statistical_gate": {"status": "PASS", "targets": checks},
        "connected_vector_only": vector_only,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--run-dir", type=Path, required=True)
    parser.add_argument("--task-id", type=int, required=True)
    parser.add_argument("--qacct", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    require(not args.output.exists(), "validation target exists")
    value = validate(args.run_dir, args.task_id, args.qacct)
    args.output.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"VCKSS_CMG_CANDIDATE_QUALIFICATION_VALIDATION_PASS task={args.task_id}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
