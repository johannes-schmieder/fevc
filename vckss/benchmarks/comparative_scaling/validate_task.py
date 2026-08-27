#!/usr/bin/env python3
"""Validate one exact-source comparative-scaling SCC task."""

from __future__ import annotations

import argparse
import json
import math
import re
from pathlib import Path
from typing import Any

try:
    from .common import (
        ESTIMATORS,
        NODE_SCHEMA,
        RESULT_SCHEMA,
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
        require,
        sha256,
    )
except ImportError:
    from common import (  # type: ignore
        ESTIMATORS,
        NODE_SCHEMA,
        RESULT_SCHEMA,
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
        require,
        sha256,
    )


def same_host(left: str, right: str) -> bool:
    return bool(left and right and left.split(".", 1)[0] == right.split(".", 1)[0])


def validate_status(role_dir: Path, role: str, active_cores: int) -> dict[str, str]:
    value = key_values(role_dir / "status.tsv")
    require(value.get("schema") == "VCKSS-COMPARATIVE-SCALING-ROLE-STATUS-V1",
            f"{role} status schema changed")
    require(value.get("role") == role, f"{role} status identity changed")
    require(integer(value.get("active_cores"), f"{role} active cores", 1)
            == active_cores, f"{role} active cores changed")
    cpus = value.get("cpu_affinity", "").split(",")
    require(len(cpus) == active_cores and len(set(cpus)) == active_cores and
            all(item.isdigit() for item in cpus), f"{role} affinity changed")
    require(value.get("application_receipt_valid") in {"0", "1"},
            f"{role} application-valid flag changed")
    require(value.get("timed_out") in {"0", "1"},
            f"{role} timeout flag changed")
    integer(value.get("application_exit_status"), f"{role} exit status", 0)
    integer(value.get("monitor_exit_status"), f"{role} monitor status", 0)
    return value


def role_result(
    job_dir: Path,
    role: str,
    task: dict[str, str],
    task_sha: str,
    input_sha: str,
) -> dict[str, Any]:
    role_dir = job_dir / role
    active_cores = integer(task["active_cores"], "active cores", 1)
    status = validate_status(role_dir, role, active_cores)
    app_rc = integer(status["application_exit_status"], f"{role} exit status")
    monitor_rc = integer(status["monitor_exit_status"], f"{role} monitor status")
    timed_out = status["timed_out"] == "1"
    valid = status["application_receipt_valid"] == "1"
    require(not valid or (app_rc == 0 and monitor_rc == 0 and not timed_out),
            f"{role} valid receipt contradicts process status")
    base: dict[str, Any] = {
        "role": role,
        "application_exit_status": app_rc,
        "monitor_exit_status": monitor_rc,
        "timed_out": timed_out,
        "application_receipt_valid": valid,
        "scientific_status": "TIMEOUT" if timed_out else "REJECTED",
    }
    if not valid:
        return base

    tree = load_json(role_dir / "process_tree.json")
    require(tree.get("schema") ==
            "VCKSS-COMPARATIVE-SCALING-PROCESS-TREE-V1" and
            tree.get("status") == "PASS", f"{role} process tree failed")
    require(bool(tree.get("phase_start_observed")) and
            bool(tree.get("phase_end_observed")), f"{role} phase markers failed")
    resources = parse_gnu_time(role_dir / "resources.txt")
    base.update({
        "scientific_status": "PASS",
        "process_wall_seconds": resources["wall_seconds"],
        "process_cpu_seconds": (
            float(resources["user_seconds"]) + float(resources["system_seconds"])
        ),
        "gnu_peak_rss_bytes": resources["maximum_rss_bytes"],
        "whole_process_peak_rss_bytes":
            integer(tree.get("whole_peak_rss_kib"), f"{role} whole RSS", 1) * 1024,
        "estimator_phase_peak_rss_bytes":
            integer(tree.get("phase_peak_rss_kib"), f"{role} phase RSS", 1) * 1024,
    })

    if role in {"mata", "rust"}:
        value = one_csv(role_dir / "result.csv")
        require(value.get("schema") == "VCKSS-COMPARATIVE-SCALING-STATA-V1" and
                value.get("application_status") == "PASS" and
                value.get("role") == role, f"{role} result schema changed")
        require(value.get("source_commit") == task["source_commit"] and
                value.get("task_sha256") == task_sha and
                value.get("input_sha256") == input_sha,
                f"{role} source/input identity changed")
        for field in ("rows", "workers", "firms", "cells_per_worker",
                      "probes", "seed", "active_cores", "sample_count"):
            expected = task[field] if field in task else task["rows"]
            require(integer(value[field], f"{role} {field}") == int(expected),
                    f"{role} dimension changed: {field}")
        require(value.get("data_restored") == value.get("rng_restored") ==
                value.get("sort_rng_restored") == "1", f"{role} state not restored")
        residual = finite(value["max_complete_residual"], f"{role} residual")
        acceptance = finite(value["residual_acceptance"], f"{role} acceptance")
        require(residual <= acceptance, f"{role} residual failed")
        require(finite(value["target_identity_residual"], f"{role} identity")
                <= 1e-12, f"{role} identity failed")
        if role == "rust":
            require(value.get("cmg_backend") == "CMG_FULL_V2" and
                    value.get("cmg_source_commit") ==
                    "dbefbc5e3b442c6dde6e7861a66d82fd5ed24f10",
                    "Rust CMG identity changed")
            require(integer(value["cmg_threads_requested"], "Rust requested threads")
                    == active_cores and
                    integer(value["cmg_threads_used"], "Rust used threads")
                    == active_cores, "Rust thread receipt changed")
        targets = {name: finite(value[f"corrected_{name}"],
                                f"{role} {name}") for name in TARGETS}
        mcse = {name: finite(value[f"mcse_{name}"], f"{role} MCSE {name}")
                for name in TARGETS}
        phase_fields = (
            "selection_seconds", "graph_seconds", "compression_seconds",
            "setup_seconds", "work_seconds", "fit_seconds",
            "leverage_seconds", "target_seconds", "correction_seconds",
            "rng_seconds", "schur_seconds", "pcg_seconds",
        )
        phases = {field: finite(value[field], f"{role} {field}")
                  for field in phase_fields}
        require(all(item >= 0 for item in phases.values()),
                f"{role} negative phase timing")
        base.update({
            "command_seconds": finite(value["command_seconds"], f"{role} command"),
            "import_seconds": finite(value["import_seconds"], f"{role} import"),
            "targets": targets,
            "mcse": mcse,
            "max_complete_residual": residual,
            "residual_acceptance": acceptance,
            "resource_peak_bytes": integer(value["resource_peak_bytes"],
                                              f"{role} resource peak"),
            "memory_forecast_bytes": integer(value["memory_forecast_bytes"],
                                                f"{role} memory forecast"),
            "cmg_backend": value.get("cmg_backend"),
            "cmg_admitted_peak_bytes": (
                None if role == "mata" else integer(
                    value["cmg_admitted_peak_bytes"], "Rust admitted peak")
            ),
            "cmg_actual_retained_bytes": (
                None if role == "mata" else integer(
                    value["cmg_actual_retained_bytes"], "Rust retained bytes")
            ),
            "solver_iterations": finite(value["solver_iterations"],
                                         f"{role} iterations"),
            **phases,
        })
        if role == "rust":
            cmg_fields = (
                "cmg_graph_seconds", "cmg_hierarchy_seconds",
                "cmg_rhs_seconds", "cmg_solve_seconds",
                "cmg_extraction_seconds",
            )
            cmg_phases = {field: finite(value[field], f"Rust {field}")
                          for field in cmg_fields}
            require(all(item >= 0 for item in cmg_phases.values()),
                    "Rust negative CMG phase timing")
            require(abs(finite(value["cmg_fit_tolerance"], "Rust fit tolerance")
                        - 1e-10) <= 1e-20 and
                    abs(finite(value["cmg_probe_tolerance"],
                               "Rust probe tolerance") - 1e-6) <= 1e-16,
                    "Rust default tolerance receipt changed")
            base.update({
                "cmg_threads_requested": active_cores,
                "cmg_threads_used": active_cores,
                "cmg_fit_tolerance": 1e-10,
                "cmg_probe_tolerance": 1e-6,
                "cmg_operator_applications": integer(
                    value["cmg_operator_applications"],
                    "Rust CMG operator applications"),
                "cmg_preconditioner_applications": integer(
                    value["cmg_preconditioner_applications"],
                    "Rust CMG preconditioner applications"),
                **cmg_phases,
            })
    else:
        value = load_json(role_dir / "matlab.json")
        require(value.get("schema") == "VCKSS-COMPARATIVE-SCALING-MATLAB-V1" and
                value.get("status") == "PASS" and value.get("role") == role,
                "MATLAB result schema changed")
        require(value.get("source_commit") == task["source_commit"] and
                value.get("task_sha256") == task_sha and
                value.get("input_sha256") == input_sha,
                "MATLAB source/input identity changed")
        for field in ("rows", "workers", "firms", "cells_per_worker",
                      "probes", "seed", "active_cores"):
            require(integer(value.get(field), f"MATLAB {field}") == int(task[field]),
                    f"MATLAB dimension changed: {field}")
        require(integer(value.get("detail_matches"), "MATLAB retained matches") ==
                int(task["rows"]), "MATLAB retained rows changed")
        require(finite(value.get("target_identity_scaled_error"), "MATLAB identity")
                <= 1e-12, "MATLAB identity failed")
        targets = {name: finite(value.get(f"corrected_{name}"), f"MATLAB {name}")
                   for name in TARGETS}
        require(value.get("rng_policy") ==
                "MATLAB_TWISTER_SEED_PLUS_WORKER_INDEX" and
                value.get("tolerance_policy") == "MAINTAINED_UPSTREAM_INTERNAL" and
                value.get("numerical_status_source") ==
                "APPLICATION_LOG_PCG_AND_OUTPUT_GATES",
                "MATLAB numerical-policy receipt changed")
        pcg = parse_matlab_pcg(role_dir / "application.txt")
        base.update({
            "scientific_status": ("PASS" if pcg["converged"] else
                                  "NUMERICAL_REJECTED"),
            "command_seconds": finite(value.get("command_seconds"), "MATLAB command"),
            "import_seconds": finite(value.get("import_seconds"), "MATLAB import"),
            "pool_startup_seconds": finite(value.get("pool_startup_seconds"),
                                             "MATLAB pool startup"),
            "pool_teardown_seconds": finite(value.get("pool_teardown_seconds"),
                                              "MATLAB pool teardown"),
            "targets": targets,
            "mcse": None,
            "matlab_upstream_commit": value.get("matlab_upstream_commit"),
            "matlab_runtime_tree_sha256": value.get("matlab_runtime_tree_sha256"),
            "matlab_pcg_converged": pcg["converged"],
            "matlab_pcg_termination_iteration": pcg["termination_iteration"],
            "matlab_pcg_returned_iteration": pcg["returned_iteration"],
            "matlab_pcg_relative_residual": pcg["relative_residual"],
            "matlab_tolerance_policy": value["tolerance_policy"],
            "matlab_rng_policy": value["rng_policy"],
        })
    return base


def validate(job_dir: Path, qacct_path: Path) -> dict[str, Any]:
    task = read_single_task(job_dir / "task.tsv")
    task_sha = sha256(job_dir / "task.tsv")
    require((job_dir / "task.sha256").read_text().strip() == task_sha,
            "task hash changed")
    input_sha = (job_dir / "input.sha256").read_text().strip()
    require(re.fullmatch(r"[0-9a-f]{64}", input_sha) is not None,
            "invalid input hash")
    input_receipt = one_csv(job_dir / "input_receipt.csv")
    require(input_receipt.get("schema") ==
            "VCKSS-COMPARATIVE-SCALING-INPUT-V1", "input schema changed")
    for field in ("structure", "connectivity"):
        require(input_receipt.get(field) == task[field], f"input {field} changed")
    for field in ("rows", "workers", "firms", "cells_per_worker"):
        require(integer(input_receipt.get(field), f"input {field}") == int(task[field]),
                f"input {field} changed")

    node = key_values(job_dir / "node_receipt.tsv")
    require(node.get("schema") == NODE_SCHEMA and node.get("status") == "PASS",
            "node receipt failed")
    require(node.get("experiment_id") == task["experiment_id"] and
            node.get("source_commit") == task["source_commit"] and
            node.get("bundle_sha256") == task["bundle_sha256"] and
            node.get("task_sha256") == task_sha and
            node.get("input_sha256") == input_sha, "node identity changed")
    require(bool(node.get("hostname")) and bool(node.get("cpu_model")),
            "node identity is incomplete")
    require(node.get("requested_slots") == node.get("actual_slots") == "16",
            "node slot contract changed")
    require(integer(node.get("active_cores"), "node active cores") ==
            int(task["active_cores"]), "node active cores changed")
    require((job_dir / "wrapper.pass").read_text().strip() ==
            f"VCKSS_COMPARATIVE_SCALING_TASK_CAPTURED {task['experiment_id']} {task_sha}",
            "wrapper receipt changed")
    require(not (job_dir / "wrapper.fail").exists(), "wrapper failure present")

    qacct = parse_qacct(qacct_path)
    require(integer(qacct["taskid"], "qacct task") == int(task["task_id"]),
            "qacct task identity changed")
    require(same_host(qacct["hostname"], node["hostname"]), "qacct host changed")
    roles = {role: role_result(job_dir, role, task, task_sha, input_sha)
             for role in ESTIMATORS}

    rust_mata_gate: dict[str, Any] = {"status": "NOT_COMPARABLE"}
    if roles["rust"]["scientific_status"] == roles["mata"]["scientific_status"] == "PASS":
        checks: dict[str, Any] = {}
        passed = True
        for target in TARGETS:
            left = roles["rust"]["targets"][target]
            right = roles["mata"]["targets"][target]
            left_mcse = roles["rust"]["mcse"][target]
            right_mcse = roles["mata"]["mcse"][target]
            scale = max(1.0, abs(left), abs(right))
            envelope = max(1e-8 * scale, 6.0 * math.hypot(left_mcse, right_mcse))
            difference = abs(left - right)
            accepted = difference <= envelope
            passed = passed and accepted
            checks[target] = {
                "difference": difference,
                "envelope": envelope,
                "accepted": accepted,
            }
        rust_mata_gate = {"status": "PASS" if passed else "FAIL", "targets": checks}

    return {
        "schema": RESULT_SCHEMA,
        "status": "PASS",
        "task": task,
        "task_sha256": task_sha,
        "input_sha256": input_sha,
        "node": {
            "hostname": node["hostname"],
            "cpu_model": node["cpu_model"],
            "scheduler_cpu_affinity": node["scheduler_cpu_affinity"],
        },
        "qacct": {
            "jobnumber": qacct["jobnumber"],
            "taskid": qacct["taskid"],
            "hostname": qacct["hostname"],
            "wall_seconds": finite(qacct["ru_wallclock"], "qacct wall"),
            "cpu_seconds": finite(qacct["cpu"], "qacct cpu"),
            "maxvmem_bytes": parse_memory(qacct["maxvmem"]),
        },
        "roles": roles,
        "rust_mata_independent_probe_gate": rust_mata_gate,
        "artifact_sha256": {
            name: sha256(job_dir / name) for name in (
                "task.tsv", "input_receipt.csv", "input.sha256",
                "order.tsv", "node_receipt.tsv", "wrapper.pass",
            )
        },
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--job-dir", type=Path, required=True)
    parser.add_argument("--qacct", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    require(not args.output.exists(), "validation target already exists")
    payload = validate(args.job_dir, args.qacct)
    args.output.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n",
                           encoding="utf-8")
    print(f"VCKSS_COMPARATIVE_SCALING_VALIDATION_PASS "
          f"{payload['task']['experiment_id']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
