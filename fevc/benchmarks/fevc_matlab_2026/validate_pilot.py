#!/usr/bin/env python3
"""Promote the campaign's validated 28-core dual-boundary pilot."""

from __future__ import annotations

import argparse
import json
import re
from pathlib import Path
from typing import Any

try:
    from .common import (
        ESTIMATORS,
        RESULT_SCHEMA,
        finite,
        key_values,
        load_json,
        require,
        sha256,
    )
except ImportError:
    from common import (  # type: ignore
        ESTIMATORS,
        RESULT_SCHEMA,
        finite,
        key_values,
        load_json,
        require,
        sha256,
    )


RUN_SCHEMA = "FEVC-MATLAB-2026-CAMPAIGN-V2"
PILOT_SCHEMA = "FEVC-MATLAB-2026-PILOT-V2"
PILOT_TASK_IDS = (5, 235)


def validate_pilot(run_dir: Path, attempt_id: str) -> dict[str, Any]:
    identity_path = run_dir / "run_identity.json"
    identity = load_json(identity_path)
    require(identity.get("schema") == RUN_SCHEMA and identity.get("status") == "PASS",
            "campaign identity changed")
    require(attempt_id == "pilot", "pilot attempt ID changed")
    require(re.fullmatch(r"[A-Za-z0-9._-]+", attempt_id) is not None,
            "invalid pilot attempt ID")
    inventory_path = run_dir / "receipts" / f"{attempt_id}.generation.json"
    inventory = load_json(inventory_path)
    require(inventory.get("schema") == "FEVC-MATLAB-2026-GENERATION-INVENTORY-V1" and
            inventory.get("status") == "PASS" and
            inventory.get("validated_cell_ids") == list(PILOT_TASK_IDS),
            "pilot generation inventory changed")
    preparation = key_values(run_dir / "receipts" / "preparation" / "preparation.tsv")
    require(preparation.get("schema") == "FEVC-MATLAB-2026-PREPARATION-V1" and
            preparation.get("status") == "PASS" and
            preparation.get("artifact_mode") == "BUILT_IN_CAMPAIGN" and
            preparation.get("required_rust_threads") == "28",
            "pilot preparation evidence changed")
    scheduler_bytes = int(identity["mem_per_core_gib"]) * 28 * 1024**3
    task_receipts: dict[str, dict[str, Any]] = {}
    for task_id in PILOT_TASK_IDS:
        validation_path = (run_dir / "attempts" / attempt_id /
                           "validations" / f"{task_id}.json")
        payload = load_json(validation_path)
        require(payload.get("schema") == RESULT_SCHEMA and
                payload.get("status") == "PASS" and
                payload.get("rankable") is True and
                int(payload["task"]["task_id"]) == task_id,
                "pilot cell validation failed")
        role_memory: dict[str, dict[str, int]] = {}
        for role in ESTIMATORS:
            value = payload["roles"][role]
            require(value.get("scientific_status") == "PASS",
                    f"pilot {task_id} {role} scientific status failed")
            command = finite(value.get("command_seconds"),
                             f"pilot {task_id} {role} command")
            require(command > 0,
                    f"pilot {task_id} {role} command time is nonpositive")
            fields = {name: int(value[name]) for name in (
                "empty_rss_bytes", "data_rss_bytes", "data_footprint_bytes",
                "estimator_increment_bytes", "estimator_peak_rss_bytes",
                "whole_process_peak_rss_bytes",
            )}
            require(0 <= fields["data_footprint_bytes"] and
                    0 <= fields["estimator_increment_bytes"] and
                    max(fields.values()) <= scheduler_bytes,
                    f"pilot {task_id} {role} violated the memory envelope")
            role_memory[role] = fields
        task_receipts[str(task_id)] = {
            "experiment_id": payload["task"]["experiment_id"],
            "role_memory": role_memory,
            "validation_sha256": sha256(validation_path),
        }
    return {
        "schema": PILOT_SCHEMA,
        "status": "PASS",
        "run_id": identity["run_id"],
        "stage": "pilot",
        "attempt_id": attempt_id,
        "task_ids": list(PILOT_TASK_IDS),
        "source_commit": identity["source_commit"],
        "bundle_sha256": identity["bundle_sha256"],
        "source_manifest_sha256": identity["source_manifest_sha256"],
        "task_manifest_sha256": identity["task_manifest_sha256"],
        "bundle_manifest_sha256": identity["bundle_manifest_sha256"],
        "stata_spi_manifest_sha256": identity["stata_spi_manifest_sha256"],
        "binary_manifest_sha256": preparation["binary_manifest_sha256"],
        "mem_per_core_gib": identity["mem_per_core_gib"],
        "command_memory_gib": identity["command_memory_gib"],
        "required_stata_processors": identity["required_stata_processors"],
        "licensed_stata_processors": int(preparation["licensed_stata_processors"]),
        "required_rust_threads": identity["required_rust_threads"],
        "required_matlab_workers": identity["required_matlab_workers"],
        "scheduler_memory_allocation_bytes": scheduler_bytes,
        "task_receipts": task_receipts,
        "generation_inventory_sha256": sha256(inventory_path),
        "run_identity_sha256": sha256(identity_path),
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--run-dir", type=Path, required=True)
    parser.add_argument("--attempt-id", required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    require(not args.output.exists(), "pilot receipt target exists")
    value = validate_pilot(args.run_dir, args.attempt_id)
    args.output.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n",
                           encoding="utf-8")
    print("FEVC_MATLAB_2026_PILOT_PASS tasks="
          + ",".join(str(item) for item in value["task_ids"]))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
