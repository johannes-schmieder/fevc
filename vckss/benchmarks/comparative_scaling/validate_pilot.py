#!/usr/bin/env python3
"""Promote one validated pilot task to an immutable pilot-pass receipt."""

from __future__ import annotations

import argparse
import json
import re
from pathlib import Path
from typing import Any

try:
    from .common import ESTIMATORS, RESULT_SCHEMA, finite, load_json, require, sha256
except ImportError:
    from common import (  # type: ignore
        ESTIMATORS,
        RESULT_SCHEMA,
        finite,
        load_json,
        require,
        sha256,
    )


PILOT_SCHEMA = "VCKSS-COMPARATIVE-SCALING-PILOT-V2"
RUN_SCHEMA = "VCKSS-COMPARATIVE-SCALING-STAGED-RUN-V3"
HEX64 = re.compile(r"[0-9a-f]{64}")
PILOT_TASKS = {"pilot-small": 7, "pilot-worst": 298}


def positive(value: Any, label: str) -> float:
    result = finite(value, label)
    require(result > 0, f"nonpositive {label}")
    return result


def validate_pilot(run_dir: Path, attempt_id: str, task_id: int) -> dict[str, Any]:
    identity_path = run_dir / "run_identity.json"
    identity = load_json(identity_path)
    require(identity.get("schema") == RUN_SCHEMA and identity.get("status") == "PASS",
            "staged-run identity changed")
    run_kind = str(identity.get("run_kind", ""))
    require(run_kind in PILOT_TASKS and PILOT_TASKS[run_kind] == task_id,
            "pilot kind or task changed")
    require(re.fullmatch(r"[A-Za-z0-9._-]+", attempt_id) is not None,
            "invalid pilot attempt ID")

    validation_path = run_dir / "attempts" / attempt_id / "validations" / f"{task_id}.json"
    payload = load_json(validation_path)
    require(payload.get("schema") == RESULT_SCHEMA and payload.get("status") == "PASS",
            "pilot task validation failed")
    task = payload.get("task", {})
    node = payload.get("node", {})
    qacct = payload.get("qacct", {})
    require(str(task.get("task_id")) == str(task_id), "pilot task ID changed")
    require(task.get("source_commit") == identity.get("source_commit") and
            task.get("bundle_sha256") == identity.get("bundle_sha256"),
            "pilot source identity changed")
    require(node.get("attempt_id") == attempt_id and
            node.get("source_commit") == identity.get("source_commit") and
            node.get("bundle_sha256") == identity.get("bundle_sha256") and
            node.get("source_manifest_sha256") ==
            identity.get("source_manifest_sha256"),
            "pilot attempt identity changed")
    for field in ("binary_manifest_sha256", "task_sha256", "input_sha256"):
        require(HEX64.fullmatch(str(node.get(field, ""))) is not None,
                f"invalid pilot {field}")
    require(node.get("task_sha256") == payload.get("task_sha256") and
            node.get("input_sha256") == payload.get("input_sha256"),
            "pilot task or input hash changed")
    preparation = load_json(
        run_dir / "receipts" / "preparation" / "qacct.pass.json")
    require(identity.get("required_stata_processors") ==
            preparation.get("required_stata_processors") == 4 and
            int(preparation.get("licensed_stata_processors", 0)) >= 4 and
            identity.get("required_rust_threads") ==
            preparation.get("required_rust_threads") == 16,
            "pilot Stata processor capability changed")

    roles = payload.get("roles", {})
    require(set(roles) == set(ESTIMATORS), "pilot estimator inventory changed")
    require(payload.get("rust_mata_independent_probe_gate", {}).get("status") == "PASS",
            "pilot Rust-Mata scientific gate failed")
    scheduler_bytes = int(identity["mem_per_core_gib"]) * 16 * 1024**3
    command_bytes = int(identity["command_memory_gib"]) * 1024**3
    max_qacct = positive(qacct.get("maxvmem_bytes"), "pilot qacct maxvmem")
    require(max_qacct <= scheduler_bytes, "pilot exceeded scheduler memory allocation")
    role_memory: dict[str, dict[str, int]] = {}
    for role in ESTIMATORS:
        value = roles[role]
        require(value.get("scientific_status") == "PASS" and
                value.get("application_receipt_valid") is True and
                int(value.get("application_exit_status", -1)) == 0 and
                int(value.get("monitor_exit_status", -1)) == 0 and
                value.get("timed_out") is False,
                f"pilot {role} application or scientific gate failed")
        positive(value.get("command_seconds"), f"pilot {role} command time")
        phase = int(positive(value.get("estimator_phase_peak_rss_bytes"),
                             f"pilot {role} phase RSS"))
        whole = int(positive(value.get("whole_process_peak_rss_bytes"),
                             f"pilot {role} process RSS"))
        gnu = int(positive(value.get("gnu_peak_rss_bytes"),
                           f"pilot {role} GNU RSS"))
        require(max(phase, whole, gnu) <= scheduler_bytes,
                f"pilot {role} exceeded scheduler memory allocation")
        if role in {"mata", "rust"}:
            forecast = int(positive(value.get("memory_forecast_bytes"),
                                    f"pilot {role} memory forecast"))
            resource = int(positive(value.get("resource_peak_bytes"),
                                    f"pilot {role} resource peak"))
            require(max(forecast, resource) <= command_bytes,
                    f"pilot {role} exceeded command memory envelope")
        role_memory[role] = {
            "estimator_phase_peak_rss_bytes": phase,
            "whole_process_peak_rss_bytes": whole,
            "gnu_peak_rss_bytes": gnu,
        }

    return {
        "schema": PILOT_SCHEMA,
        "status": "PASS",
        "run_id": identity["run_id"],
        "run_kind": run_kind,
        "attempt_id": attempt_id,
        "task_id": task_id,
        "source_commit": identity["source_commit"],
        "bundle_sha256": identity["bundle_sha256"],
        "source_manifest_sha256": identity["source_manifest_sha256"],
        "task_manifest_sha256": identity["task_manifest_sha256"],
        "stata_spi_manifest_sha256": identity["stata_spi_manifest_sha256"],
        "binary_manifest_sha256": node["binary_manifest_sha256"],
        "task_sha256": payload["task_sha256"],
        "input_sha256": payload["input_sha256"],
        "mem_per_core_gib": identity["mem_per_core_gib"],
        "command_memory_gib": identity["command_memory_gib"],
        "required_stata_processors": identity["required_stata_processors"],
        "licensed_stata_processors": preparation["licensed_stata_processors"],
        "maximum_mata_cores": identity["maximum_mata_cores"],
        "required_rust_threads": identity["required_rust_threads"],
        "required_matlab_workers": identity["required_matlab_workers"],
        "benchmark_thread_contract": preparation["benchmark_thread_contract"],
        "benchmark_ado_adapter_sha256":
            preparation["benchmark_ado_adapter_sha256"],
        "benchmark_ado_receipt_sha256":
            preparation["benchmark_ado_receipt_sha256"],
        "stata_processor_capability_sha256":
            preparation["stata_processor_capability_sha256"],
        "qacct_jobnumber": qacct["jobnumber"],
        "qacct_taskid": qacct["taskid"],
        "qacct_maxvmem_bytes": int(max_qacct),
        "role_memory": role_memory,
        "validation_sha256": sha256(validation_path),
        "run_identity_sha256": sha256(identity_path),
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--run-dir", type=Path, required=True)
    parser.add_argument("--attempt-id", required=True)
    parser.add_argument("--task-id", type=int, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    require(not args.output.exists(), "pilot receipt target already exists")
    value = validate_pilot(args.run_dir, args.attempt_id, args.task_id)
    args.output.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n",
                           encoding="utf-8")
    print(f"VCKSS_COMPARATIVE_SCALING_PILOT_PASS {value['run_kind']} "
          f"task={value['task_id']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
