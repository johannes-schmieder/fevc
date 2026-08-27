from __future__ import annotations

import csv
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from validate_task import role_result  # noqa: E402


TASK = {
    "source_commit": "1" * 40, "rows": "7680", "workers": "3840",
    "firms": "96", "cells_per_worker": "2", "probes": "200",
    "seed": "104729", "active_cores": "1",
}
TASK_SHA = "2" * 64
INPUT_SHA = "3" * 64


def write_status(path: Path, role: str, *, app: int = 0, monitor: int = 0,
                 timeout: bool = False, valid: bool = True) -> None:
    path.write_text(
        "key\tvalue\n"
        "schema\tVCKSS-COMPARATIVE-SCALING-ROLE-STATUS-V1\n"
        f"role\t{role}\napplication_exit_status\t{app}\n"
        f"monitor_exit_status\t{monitor}\ntimed_out\t{int(timeout)}\n"
        f"application_receipt_valid\t{int(valid)}\nactive_cores\t1\n"
        "cpu_affinity\t7\n", encoding="utf-8")


def write_runtime_receipts(role_dir: Path) -> None:
    (role_dir / "process_tree.json").write_text(json.dumps({
        "schema": "VCKSS-COMPARATIVE-SCALING-PROCESS-TREE-V1",
        "status": "PASS", "phase_start_observed": True,
        "phase_end_observed": True, "whole_peak_rss_kib": 2048,
        "phase_peak_rss_kib": 1024,
    }), encoding="utf-8")
    (role_dir / "resources.txt").write_text(
        "User time (seconds): 2\nSystem time (seconds): 1\n"
        "Elapsed (wall clock) time (h:mm:ss or m:ss): 0:04\n"
        "Maximum resident set size (kbytes): 3072\n", encoding="utf-8")


def test_timeout_is_retained_without_application_receipt(tmp_path: Path) -> None:
    role_dir = tmp_path / "mata"
    role_dir.mkdir()
    write_status(role_dir / "status.tsv", "mata", app=124, timeout=True,
                 valid=False)
    value = role_result(tmp_path, "mata", TASK, TASK_SHA, INPUT_SHA)
    assert value["scientific_status"] == "TIMEOUT"
    assert value["timed_out"] is True


def test_valid_mata_result_reconciles_phase_and_state_receipts(tmp_path: Path) -> None:
    role_dir = tmp_path / "mata"
    role_dir.mkdir()
    write_status(role_dir / "status.tsv", "mata")
    write_runtime_receipts(role_dir)
    row = {
        "schema": "VCKSS-COMPARATIVE-SCALING-STATA-V1",
        "application_status": "PASS", "role": "mata",
        "source_commit": TASK["source_commit"], "task_sha256": TASK_SHA,
        "input_sha256": INPUT_SHA, **TASK, "sample_count": TASK["rows"],
        "data_restored": "1", "rng_restored": "1", "sort_rng_restored": "1",
        "max_complete_residual": "1e-7", "residual_acceptance": "1e-5",
        "target_identity_residual": "0", "command_seconds": "3.5",
        "import_seconds": "0.2", "resource_peak_bytes": "1000",
        "memory_forecast_bytes": "2000", "solver_iterations": "12",
        "cmg_backend": "NONE",
    }
    for field in (
        "selection_seconds", "graph_seconds", "compression_seconds",
        "setup_seconds", "work_seconds", "fit_seconds", "leverage_seconds",
        "target_seconds", "correction_seconds", "rng_seconds", "schur_seconds",
        "pcg_seconds",
    ):
        row[field] = "0.1"
    for target in ("worker", "firm", "covariance", "total"):
        row[f"corrected_{target}"] = "1.0"
        row[f"mcse_{target}"] = "0.01"
    with (role_dir / "result.csv").open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=list(row))
        writer.writeheader(); writer.writerow(row)
    value = role_result(tmp_path, "mata", TASK, TASK_SHA, INPUT_SHA)
    assert value["scientific_status"] == "PASS"
    assert value["pcg_seconds"] == 0.1
    assert value["estimator_phase_peak_rss_bytes"] == 1024**2


def test_matlab_nonconvergence_keeps_timing_but_rejects_ranking(tmp_path: Path) -> None:
    role_dir = tmp_path / "matlab"
    role_dir.mkdir()
    write_status(role_dir / "status.tsv", "matlab")
    write_runtime_receipts(role_dir)
    (role_dir / "application.txt").write_text(
        "pcg stopped at iteration 1000 without converging to the desired tolerance.\n"
        "The iterate returned (number 999) has relative residual 2e-6.\n",
        encoding="utf-8")
    payload = {
        "schema": "VCKSS-COMPARATIVE-SCALING-MATLAB-V1", "status": "PASS",
        "role": "matlab", "source_commit": TASK["source_commit"],
        "task_sha256": TASK_SHA, "input_sha256": INPUT_SHA,
        **{field: int(TASK[field]) for field in (
            "rows", "workers", "firms", "cells_per_worker", "probes", "seed",
            "active_cores")},
        "detail_matches": int(TASK["rows"]), "target_identity_scaled_error": 0,
        "command_seconds": 7.0, "import_seconds": 0.3,
        "pool_startup_seconds": 1.0, "pool_teardown_seconds": 0.2,
        "matlab_upstream_commit": "4" * 40, "matlab_runtime_tree_sha256": "5" * 64,
        "rng_policy": "MATLAB_TWISTER_SEED_PLUS_WORKER_INDEX",
        "tolerance_policy": "MAINTAINED_UPSTREAM_INTERNAL",
        "numerical_status_source": "APPLICATION_LOG_PCG_AND_OUTPUT_GATES",
    }
    for target in ("worker", "firm", "covariance", "total"):
        payload[f"corrected_{target}"] = 1.0
    (role_dir / "matlab.json").write_text(json.dumps(payload), encoding="utf-8")
    value = role_result(tmp_path, "matlab", TASK, TASK_SHA, INPUT_SHA)
    assert value["scientific_status"] == "NUMERICAL_REJECTED"
    assert value["command_seconds"] == 7.0
    assert value["matlab_pcg_converged"] is False
