#!/usr/bin/env python3
"""Registered censoring contract for the three nonterminating MATLAB calls."""

from __future__ import annotations

from pathlib import Path
from typing import Any

try:
    from .common import key_values, require, sha256
except ImportError:
    from common import key_values, require, sha256  # type: ignore


SCHEMA = "FEVC-COMPARATIVE-SCALING-CENSOR-V1"
CENSORED_TASK_IDS = (61, 62, 63)
CENSORED_CELL = ("strong_d2", 1_966_080, 1)
CENSOR_LOWER_BOUND_SECONDS = 10_800
CENSOR_STATUS = "RIGHT_CENSORED_MATLAB_TIMEOUT"

CENSOR_FIELDS = (
    "schema", "status", "task_id", "experiment_id", "source_commit",
    "bundle_sha256", "structure", "rows", "active_cores", "replicate",
    "execution_order", "role", "censoring", "lower_bound_seconds",
    "application_exit_status", "monitor_exit_status", "timed_out",
    "jobnumber", "scheduler_task_id", "hostname", "qacct_wall_seconds",
    "qacct_cpu_seconds", "qacct_maxvmem", "qacct_sha256",
    "matlab_status_sha256", "matlab_application_sha256", "wrapper_fail_sha256",
)


def qacct_fields(path: Path) -> dict[str, str]:
    require(path.is_file() and not path.is_symlink(), f"invalid qacct: {path}")
    values: dict[str, str] = {}
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        fields = line.strip().split(None, 1)
        if len(fields) == 2:
            values[fields[0]] = fields[1]
    required = {
        "jobnumber", "taskid", "hostname", "project", "granted_pe", "slots",
        "failed", "exit_status", "ru_wallclock", "cpu", "maxvmem",
    }
    require(required <= values.keys(), f"incomplete qacct: {path}")
    require(values["project"] == "welfgr" and values["slots"] == "16" and
            values["granted_pe"] in {"omp", "omp16"},
            "censored scheduler allocation changed")
    return values


def validate_censored_task(
    task: dict[str, str],
    job_dir: Path,
    qacct_path: Path,
    *,
    job_id: str,
    scheduler_task_id: int,
) -> dict[str, Any]:
    """Validate one exact registered timeout and return its compact ledger row."""
    task_id = int(task["task_id"])
    replicate = int(task["replicate"])
    require(task_id in CENSORED_TASK_IDS and
            (task["structure"], int(task["rows"]),
             int(task["active_cores"])) == CENSORED_CELL and
            replicate == task_id - 60 and
            task["experiment_id"] ==
            f"scale_strong_d2_n1966080_c1_r{replicate}" and
            int(task["estimator_timeout_seconds"]) ==
            CENSOR_LOWER_BOUND_SECONDS,
            "registered censored task identity changed")

    qacct = qacct_fields(qacct_path)
    require(qacct["jobnumber"] == job_id and
            int(qacct["taskid"]) == scheduler_task_id and
            qacct["failed"] == "0" and qacct["exit_status"] == "1",
            "registered censor accounting changed")

    matlab_status_path = job_dir / "matlab" / "status.tsv"
    matlab_application_path = job_dir / "matlab" / "application.txt"
    wrapper_fail_path = job_dir / "wrapper.fail"
    matlab = key_values(matlab_status_path)
    require(matlab.get("schema") ==
            "FEVC-COMPARATIVE-SCALING-ROLE-STATUS-V2" and
            matlab.get("role") == "matlab" and
            matlab.get("application_exit_status") == "124" and
            matlab.get("monitor_exit_status") == "2" and
            matlab.get("timed_out") == "1" and
            matlab.get("application_receipt_valid") == "0" and
            matlab.get("active_cores") == "1" and
            matlab.get("effective_role_cores") == "1",
            "registered MATLAB timeout signature changed")
    require(matlab_application_path.is_file() and
            not matlab_application_path.is_symlink() and
            not (job_dir / "matlab" / "result.csv").exists(),
            "censored MATLAB application evidence changed")
    expected_wrapper = (
        "VCKSS_COMPARATIVE_SCALING_WRAPPER_FAIL "
        f"{task['experiment_id']} stage=applications rc=1"
    )
    require(wrapper_fail_path.is_file() and
            wrapper_fail_path.read_text(encoding="utf-8").strip() ==
            expected_wrapper,
            "censored wrapper failure signature changed")

    return {
        "schema": SCHEMA,
        "status": CENSOR_STATUS,
        "task_id": task_id,
        "experiment_id": task["experiment_id"],
        "source_commit": task["source_commit"],
        "bundle_sha256": task["bundle_sha256"],
        "structure": task["structure"],
        "rows": int(task["rows"]),
        "active_cores": int(task["active_cores"]),
        "replicate": replicate,
        "execution_order": task["execution_order"],
        "role": "matlab",
        "censoring": "RIGHT",
        "lower_bound_seconds": CENSOR_LOWER_BOUND_SECONDS,
        "application_exit_status": 124,
        "monitor_exit_status": 2,
        "timed_out": 1,
        "jobnumber": qacct["jobnumber"],
        "scheduler_task_id": int(qacct["taskid"]),
        "hostname": qacct["hostname"],
        "qacct_wall_seconds": float(qacct["ru_wallclock"]),
        "qacct_cpu_seconds": float(qacct["cpu"]),
        "qacct_maxvmem": qacct["maxvmem"],
        "qacct_sha256": sha256(qacct_path),
        "matlab_status_sha256": sha256(matlab_status_path),
        "matlab_application_sha256": sha256(matlab_application_path),
        "wrapper_fail_sha256": sha256(wrapper_fail_path),
    }
