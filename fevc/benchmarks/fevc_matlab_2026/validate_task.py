#!/usr/bin/env python3
"""Validate one FEVC--MATLAB 2026 cell against scheduler and application evidence."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

try:
    from .common import (
        NODE_SCHEMA,
        RESULT_SCHEMA,
        SMOKE_TASK_SCHEMA,
        TARGETS,
        finite,
        integer,
        key_values,
        load_json,
        one_csv,
        parse_gnu_time,
        parse_matlab_pcg,
        parse_memory,
        parse_qacct,
        read_single_task,
        read_task,
        require,
        sha256,
    )
except ImportError:
    from common import (  # type: ignore
        NODE_SCHEMA,
        RESULT_SCHEMA,
        SMOKE_TASK_SCHEMA,
        TARGETS,
        finite,
        integer,
        key_values,
        load_json,
        one_csv,
        parse_gnu_time,
        parse_matlab_pcg,
        parse_memory,
        parse_qacct,
        read_single_task,
        read_task,
        require,
        sha256,
    )


APPLICATION_RESULT_SCHEMA = "FEVC-MATLAB-2026-APPLICATION-RESULT-V1"


def memory_receipt(role_dir: Path, expected_workers: int | None) -> dict[str, Any]:
    value = load_json(role_dir / "process_tree.json")
    require(value.get("schema") == "FEVC-MATLAB-2026-PROCESS-TREE-V1" and
            value.get("status") == "PASS", "process-tree monitor failed")
    for marker in ("empty_ready_observed", "data_ready_observed",
                   "phase_start_observed", "phase_end_observed"):
        require(value.get(marker) is True, f"memory marker missing: {marker}")
    empty = integer(value.get("empty_rss_kib_median"), "empty RSS", 1) * 1024
    data = integer(value.get("data_rss_kib_median"), "data RSS", 1) * 1024
    peak = integer(value.get("phase_peak_rss_kib"), "phase peak RSS", 1) * 1024
    whole = integer(value.get("whole_peak_rss_kib"), "whole peak RSS", 1) * 1024
    require(data >= empty and peak >= data and whole >= peak,
            "memory phase ordering failed")
    if expected_workers is None:
        require(value.get("expected_pool_workers") is None,
                "Rust process-tree worker identity changed")
    else:
        require(integer(value.get("expected_pool_workers"), "MATLAB workers", 1)
                == expected_workers and value.get("identity_all_named_observed") is True,
                "MATLAB client/worker process tree changed")
    return {
        "empty_rss_bytes": empty,
        "data_rss_bytes": data,
        "data_footprint_bytes": data - empty,
        "estimator_increment_bytes": peak - data,
        "estimator_peak_rss_bytes": peak,
        "whole_process_peak_rss_bytes": whole,
        "memory_amplification": peak / data,
        "empty_pss_bytes": integer(value.get("empty_pss_kib_last"),
                                   "empty PSS", 1) * 1024,
        "data_pss_bytes": integer(value.get("data_pss_kib_last"),
                                  "data PSS", 1) * 1024,
        "estimator_start_pss_bytes": integer(
            value.get("estimator_start_pss_kib"), "estimator-start PSS", 1) * 1024,
        "estimator_end_pss_bytes": integer(
            value.get("estimator_end_pss_kib"), "estimator-end PSS", 0) * 1024,
        "sample_interval_seconds": finite(value.get("sample_interval_seconds"),
                                          "memory sample interval"),
    }


def role_status(role_dir: Path, role: str, active_cores: int) -> dict[str, str]:
    status = key_values(role_dir / "status.tsv")
    require(status.get("schema") == "FEVC-MATLAB-2026-ROLE-STATUS-V1" and
            status.get("role") == role and
            status.get("application_exit_status") == "0" and
            status.get("monitor_exit_status") == "0" and
            status.get("timed_out") == "0" and
            status.get("application_receipt_valid") == "1" and
            integer(status.get("active_cores"), "role active cores", 1)
            == active_cores and
            integer(status.get("effective_role_cores"), "effective cores", 1)
            == active_cores,
            f"{role} status failed")
    return status


def rust_receipt(role_dir: Path, task: dict[str, str], input_sha: str) -> dict[str, Any]:
    value = one_csv(role_dir / "result.csv")
    require(value.get("schema") == "FEVC-MATLAB-2026-RUST-V1" and
            value.get("role") == "rust" and value.get("application_status") == "PASS",
            "Rust result schema changed")
    require(value.get("source_commit") == task["source_commit"] and
            value.get("input_sha256") == input_sha,
            "Rust source/input identity changed")
    for field in ("rows", "workers", "firms", "cells_per_worker", "probes",
                  "seed", "active_cores", "rust_threads"):
        expected = task["active_cores"] if field == "rust_threads" else task[field]
        require(integer(value.get(field), f"Rust {field}") == int(expected),
                f"Rust dimension changed: {field}")
    require(value.get("estimator_status") ==
            "KSS_SCALE_EXPERIMENTAL_POINT_ESTIMATES" and
            value.get("engine") == "compressed" and
            value.get("cmg_backend") == "CMG_FULL_V2" and
            integer(value.get("sample_count"), "Rust sample count") == int(task["rows"]) and
            finite(value.get("max_complete_residual"), "Rust residual") <=
            finite(value.get("residual_acceptance"), "Rust residual tolerance"),
            "Rust numerical receipt failed")
    targets = {name: finite(value.get(f"corrected_{name}"), f"Rust {name}")
               for name in TARGETS}
    return {
        "scientific_status": "PASS",
        "command_seconds": finite(value.get("command_seconds"), "Rust command"),
        "import_seconds": finite(value.get("import_seconds"), "Rust import"),
        "solver_iterations": integer(value.get("solver_iterations"),
                                     "Rust iterations"),
        "max_complete_residual": finite(value.get("max_complete_residual"),
                                        "Rust residual"),
        "targets": targets,
        "phases": {name: finite(value.get(name), f"Rust {name}") for name in (
            "selection_seconds", "graph_seconds", "compression_seconds",
            "setup_seconds", "work_seconds", "fit_seconds", "leverage_seconds",
            "target_seconds", "correction_seconds", "rng_seconds", "schur_seconds",
            "pcg_seconds", "rust_ingest_seconds", "rust_canonicalize_seconds",
            "rust_graph_seconds", "rust_compress_seconds", "rust_plan_seconds",
            "rust_stayer_augmentation_seconds", "rust_solve_seconds",
            "rust_native_total_seconds",
        )},
    }


def matlab_receipt(role_dir: Path, task: dict[str, str], input_sha: str) -> dict[str, Any]:
    value = load_json(role_dir / "matlab.json")
    require(value.get("schema") == "FEVC-MATLAB-2026-MATLAB-V1" and
            value.get("status") == "PASS" and value.get("role") == "matlab",
            "MATLAB result schema changed")
    require(value.get("source_commit") == task["source_commit"] and
            value.get("input_sha256") == input_sha,
            "MATLAB source/input identity changed")
    mapping = {"active_cores": "active_cores", "matlab_workers": "pool_workers"}
    for task_field, result_field in mapping.items():
        require(integer(value.get(result_field), f"MATLAB {result_field}") ==
                int(task[task_field]), f"MATLAB {result_field} changed")
    for field in ("rows", "workers", "firms", "cells_per_worker", "probes", "seed"):
        require(integer(value.get(field), f"MATLAB {field}") == int(task[field]),
                f"MATLAB dimension changed: {field}")
    require(integer(value.get("detail_matches"), "MATLAB retained matches") ==
            int(task["rows"]) and
            finite(value.get("target_identity_scaled_error"), "MATLAB identity") <= 1e-12,
            "MATLAB retained sample or target identity changed")
    pcg = parse_matlab_pcg(role_dir / "application.txt")
    targets = {name: finite(value.get(f"corrected_{name}"), f"MATLAB {name}")
               for name in TARGETS}
    return {
        "scientific_status": "PASS" if pcg["converged"] else "NUMERICAL_REJECTED",
        "command_seconds": finite(value.get("command_seconds"), "MATLAB command"),
        "import_seconds": finite(value.get("import_seconds"), "MATLAB import"),
        "pool_startup_seconds": finite(value.get("pool_startup_seconds"),
                                       "MATLAB pool startup"),
        "pool_teardown_seconds": finite(value.get("pool_teardown_seconds"),
                                        "MATLAB pool teardown"),
        "solver_iterations": pcg["termination_iteration"],
        "max_complete_residual": pcg["relative_residual"],
        "targets": targets,
        "matlab_upstream_commit": value.get("matlab_upstream_commit"),
        "matlab_runtime_tree_sha256": value.get("matlab_runtime_tree_sha256"),
    }


def validate_application(job_dir: Path) -> dict[str, Any]:
    run_dir = job_dir.parents[3]
    task = read_single_task(job_dir / "task.tsv")
    if task["task_schema"] == SMOKE_TASK_SCHEMA:
        canonical = read_single_task(run_dir / "input" / "smoke.tsv")
    else:
        canonical = read_task(run_dir / "input" / "tasks.tsv", int(task["task_id"]))
    require(task == canonical, "task row differs from immutable manifest")
    task_sha = sha256(job_dir / "task.tsv")
    require((job_dir / "task.sha256").read_text().strip() == task_sha,
            "task hash changed")
    input_sha = (job_dir / "input.sha256").read_text().strip()
    require(len(input_sha) == 64, "input hash changed")
    input_receipt = one_csv(job_dir / "input_receipt.csv")
    require(input_receipt.get("schema") == "FEVC-MATLAB-2026-INPUT-V1",
            "input schema changed")
    for field in ("rows", "workers", "firms", "cells_per_worker"):
        require(integer(input_receipt.get(field), f"input {field}") == int(task[field]),
                f"input dimension changed: {field}")

    node = key_values(job_dir / "node_receipt.tsv")
    require(node.get("schema") == NODE_SCHEMA and node.get("status") == "PASS" and
            node.get("source_commit") == task["source_commit"] and
            node.get("task_sha256") == task_sha and
            node.get("input_sha256") == input_sha and
            node.get("requested_slots") == node.get("actual_slots") ==
            task["requested_slots"] and
            integer(node.get("active_cores"), "node active cores") ==
            int(task["active_cores"]), "node receipt changed")
    cpu_model = node.get("cpu_model", "")
    if task["task_schema"] != SMOKE_TASK_SCHEMA:
        require("E5-2680 v4" in cpu_model,
                "primary benchmark ran on wrong CPU model")
    require(len(node.get("assigned_cpu_affinity", "").split(",")) ==
            int(task["requested_slots"]) and
            len(node.get("active_cpu_affinity", "").split(",")) ==
            int(task["active_cores"]), "CPU affinity changed")

    active = int(task["active_cores"])
    rust_dir = job_dir / "rust"
    matlab_dir = job_dir / "matlab"
    role_status(rust_dir, "rust", active)
    role_status(matlab_dir, "matlab", active)
    rust = rust_receipt(rust_dir, task, input_sha)
    matlab = matlab_receipt(matlab_dir, task, input_sha)
    rust.update(memory_receipt(rust_dir, None))
    matlab.update(memory_receipt(matlab_dir, active))
    rust_time = parse_gnu_time(rust_dir / "resources.txt")
    matlab_time = parse_gnu_time(matlab_dir / "resources.txt")
    rust["process_wall_seconds"] = rust_time["wall_seconds"]
    rust["process_cpu_seconds"] = rust_time["user_seconds"] + rust_time["system_seconds"]
    matlab["process_wall_seconds"] = matlab_time["wall_seconds"]
    matlab["process_cpu_seconds"] = matlab_time["user_seconds"] + matlab_time["system_seconds"]

    require((job_dir / "wrapper.pass").is_file() and
            not (job_dir / "wrapper.fail").exists(), "cell wrapper failed")

    gaps = {name: abs(rust["targets"][name] - matlab["targets"][name])
            for name in TARGETS}
    scaled_gaps = {name: gaps[name] /
                   (1 + abs(rust["targets"][name]) + abs(matlab["targets"][name]))
                   for name in TARGETS}
    rankable = (rust["scientific_status"] == "PASS" and
                matlab["scientific_status"] == "PASS")
    return {
        "schema": APPLICATION_RESULT_SCHEMA,
        "status": "PASS",
        "rankable": rankable,
        "task": {key: (int(value) if key in {
            "task_id", "cells_per_worker", "rows", "workers", "firms",
            "active_cores", "stata_processors", "rust_threads", "matlab_workers",
            "replicate", "seed", "probes", "requested_slots", "mem_per_core_gib",
            "command_memory_gib", "hard_wall_seconds", "estimator_timeout_seconds",
        } else value) for key, value in task.items()},
        "node": node,
        "input_sha256": input_sha,
        "task_sha256": task_sha,
        "roles": {"rust": rust, "matlab": matlab},
        "role_status_sha256": {
            "rust": sha256(rust_dir / "status.tsv"),
            "matlab": sha256(matlab_dir / "status.tsv"),
        },
        "absolute_target_gaps": gaps,
        "scaled_target_gaps": scaled_gaps,
    }


def validate(job_dir: Path, qacct_path: Path) -> dict[str, Any]:
    value = validate_application(job_dir)
    task = value["task"]
    node = value["node"]
    qacct = parse_qacct(qacct_path, expected_slots=int(task["requested_slots"]))
    require(qacct["jobnumber"] == node.get("job_id") and
            integer(qacct["taskid"], "qacct bundle task") ==
            integer(node.get("scheduler_task_id"), "node bundle task"),
            "qacct job/bundle identity changed")
    value["schema"] = RESULT_SCHEMA
    value["qacct"] = {
        "jobnumber": qacct["jobnumber"], "taskid": int(qacct["taskid"]),
        "hostname": qacct["hostname"],
        "wall_seconds": finite(qacct["ru_wallclock"], "qacct wall"),
        "cpu_seconds": finite(qacct["cpu"], "qacct CPU"),
        "maxvmem_bytes": parse_memory(qacct["maxvmem"]),
    }
    return value


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--job-dir", type=Path, required=True)
    parser.add_argument("--qacct", type=Path)
    parser.add_argument("--application-only", action="store_true")
    parser.add_argument("--require-rankable", action="store_true")
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    require(not args.output.exists(), "validation target exists")
    require(args.application_only == (args.qacct is None),
            "choose application-only or supply qacct")
    value = (validate_application(args.job_dir) if args.application_only else
             validate(args.job_dir, args.qacct))
    require(not args.require_rankable or value["rankable"],
            "pilot application is not rankable")
    args.output.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n",
                           encoding="utf-8")
    print(f"FEVC_MATLAB_2026_CELL_VALIDATION_PASS {value['task']['experiment_id']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
