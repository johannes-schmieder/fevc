#!/usr/bin/env python3
"""Authorize the exact affected cells for a compatible replacement generation."""

from __future__ import annotations

import argparse
import json
import re
import subprocess
from pathlib import Path
from typing import Any

try:
    from .collect_generation import (
        SCHEMA as INVENTORY_SCHEMA,
        frozen_manifest,
    )
    from .common import key_values, load_json, read_manifest, require, sha256
    from .expand_task_ids import expand
except ImportError:
    from collect_generation import (  # type: ignore
        SCHEMA as INVENTORY_SCHEMA,
        frozen_manifest,
    )
    from common import key_values, load_json, read_manifest, require, sha256  # type: ignore
    from expand_task_ids import expand  # type: ignore


SCHEMA = "FEVC-COMPARATIVE-SCALING-REPLACEMENT-AUTHORIZATION-V1"
PILOT_TASK_IDS = [1, 226]


def affected_task_ids(tasks: list[dict[str, str]]) -> list[int]:
    return [int(task["task_id"]) for task in tasks
            if int(task["active_cores"]) == 1 or
            (task["structure"] == "weak_d3" and int(task["rows"]) == 7_680)]


def replacement_stage(requested: list[int], expected: list[int],
                      completed: list[int]) -> tuple[str, list[int]]:
    """Validate a full submission or the registered pilot/remainder sequence."""
    require(len(completed) == len(set(completed)),
            "replacement has duplicate successful tasks")
    require(set(completed).issubset(expected),
            "replacement has a successful task outside the affected set")
    remaining = sorted(set(expected) - set(completed))
    if not completed:
        require(requested in (PILOT_TASK_IDS, expected),
                "first replacement submission must be the registered pilot or full set")
        return ("PILOT" if requested == PILOT_TASK_IDS else "FULL", remaining)
    require(sorted(completed) == PILOT_TASK_IDS and requested == remaining,
            "post-pilot replacement submission must be the exact remaining set")
    return "REMAINDER", remaining


def authorize(new_run: Path, old_run: Path, task_spec: str,
              *, check_qstat: bool = True) -> dict[str, Any]:
    new_identity = load_json(new_run / "run_identity.json")
    old_identity = load_json(old_run / "run_identity.json")
    require(new_identity.get("status") == old_identity.get("status") == "PASS" and
            new_identity.get("run_kind") == old_identity.get("run_kind") ==
            "production" and
            new_identity.get("replaces_run_id") == old_identity.get("run_id") and
            old_run.name == old_identity.get("run_id"),
            "replacement lineage changed")
    inventory_path = old_run / "receipts" / "generation_inventory.first.json"
    inventory = load_json(inventory_path)
    requested = expand(task_spec)
    old_tasks = frozen_manifest(old_run, old_identity)
    expected = affected_task_ids(old_tasks)
    require(inventory.get("schema") == INVENTORY_SCHEMA and
            inventory.get("status") == "PASS" and inventory.get("terminal") is True and
            inventory.get("run_id") == old_identity.get("run_id") and
            inventory.get("qacct_records") == 300 and
            inventory.get("exit_status_zero_records") == 228 and
            inventory.get("exit_status_nonzero_records") == 72 and
            inventory.get("failed_task_ids") == expected,
            "replacement task inventory differs from terminal failures")
    require(inventory.get("failure_class_counts") == {
        "INSTRUMENTATION_ONE_WORKER_JSON_SCALAR": 57,
        "SCIENTIFIC_SAMPLE_CONTRACT": 15,
    }, "replacement failure classification changed")
    if check_qstat:
        active = subprocess.run(
            ("qstat", "-j", str(inventory["job_id"])),
            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=False,
        )
        require(active.returncode != 0, "replaced production array is still active")

    new_preparation = key_values(
        new_run / "receipts" / "preparation" / "preparation.tsv")
    old_preparation = key_values(
        old_run / "receipts" / "preparation" / "preparation.tsv")
    compatibility_path = new_run / "receipts" / "preparation" / \
        "artifact_source.pass.json"
    compatibility = load_json(compatibility_path)
    old_pilot_gate = old_run / "receipts" / "pilot_gate.pass"
    require(old_pilot_gate.is_file() and
            new_identity.get("pilot_small_run_id") ==
            old_identity.get("pilot_small_run_id") and
            new_identity.get("pilot_worst_run_id") ==
            old_identity.get("pilot_worst_run_id"),
            "replacement pilot lineage changed")
    require(new_preparation.get("status") == old_preparation.get("status") == "PASS" and
            new_preparation.get("binary_manifest_sha256") ==
            old_preparation.get("binary_manifest_sha256") and
            new_preparation.get("plugin_sha256") ==
            old_preparation.get("plugin_sha256") and
            new_preparation.get("benchmark_ado_adapter_sha256") ==
            old_preparation.get("benchmark_ado_adapter_sha256") and
            new_preparation.get("benchmark_ado_receipt_sha256") ==
            old_preparation.get("benchmark_ado_receipt_sha256") and
            new_preparation.get("benchmark_thread_contract") ==
            old_preparation.get("benchmark_thread_contract") ==
            "FEVC-BENCHMARK-THREADS-V1" and
            compatibility.get("status") == "PASS" and
            compatibility.get("compatibility", {}).get("mode") ==
            "SAFE_HARNESS_ONLY" and
            compatibility.get("compatibility", {}).get("replaces_run_id") ==
            old_identity.get("run_id"),
            "replacement estimator artifact compatibility changed")

    new_tasks = read_manifest(new_run / "input" / "tasks.tsv")
    require(affected_task_ids(new_tasks) == expected,
            "new replacement matrix changed affected task IDs")
    old_by_id = {int(task["task_id"]): task for task in old_tasks}
    new_by_id = {int(task["task_id"]): task for task in new_tasks}
    for task_id in set(range(1, 301)) - set(expected):
        ignored = {"source_commit", "bundle_sha256"}
        require({key: value for key, value in old_by_id[task_id].items()
                 if key not in ignored} ==
                {key: value for key, value in new_by_id[task_id].items()
                 if key not in ignored},
                f"unaffected task {task_id} changed scientific contract")

    completed: list[int] = []
    for path in sorted((new_run / "attempts").glob("*/validations/*.json")):
        value = load_json(path)
        require(value.get("status") == "PASS",
                f"replacement validation changed: {path}")
        completed.append(int(value["task"]["task_id"]))
    stage, _ = replacement_stage(requested, expected, completed)
    return {
        "schema": SCHEMA,
        "status": "PASS",
        "replacement_run_id": new_identity["run_id"],
        "replaces_run_id": old_identity["run_id"],
        "replacement_source_commit": new_identity["source_commit"],
        "base_source_commit": old_identity["source_commit"],
        "task_spec": task_spec,
        "stage": stage,
        "task_ids": requested,
        "task_count": len(requested),
        "affected_task_ids": expected,
        "affected_task_count": len(expected),
        "already_validated_task_ids": sorted(completed),
        "carried_validated_tasks": 228,
        "binary_manifest_sha256": new_preparation["binary_manifest_sha256"],
        "plugin_sha256": new_preparation["plugin_sha256"],
        "base_inventory_sha256": sha256(inventory_path),
        "artifact_compatibility_sha256": sha256(compatibility_path),
        "base_pilot_gate_sha256": sha256(old_pilot_gate),
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--new-run", type=Path, required=True)
    parser.add_argument("--old-run", type=Path, required=True)
    parser.add_argument("--task-ids", required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    require(re.fullmatch(r"[0-9,-]+", args.task_ids) is not None,
            "invalid replacement task specification")
    require(not args.output.exists(), "replacement authorization target exists")
    value = authorize(args.new_run, args.old_run, args.task_ids)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n",
                           encoding="utf-8")
    print("VCKSS_COMPARATIVE_SCALING_REPLACEMENT_AUTHORIZED "
          f"tasks={value['task_count']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
