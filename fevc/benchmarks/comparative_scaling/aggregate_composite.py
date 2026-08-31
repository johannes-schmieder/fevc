#!/usr/bin/env python3
"""Aggregate a compatible base/replacement pair with registered censoring."""

from __future__ import annotations

import argparse
import csv
import json
import shutil
from pathlib import Path
from typing import Any

try:
    from .aggregate import (
        CELL_FIELDS,
        CPU_STRATA_FIELDS,
        INPUT_HASH_FIELDS,
        OVERLAP_FIELDS,
        RESULT_FIELDS,
        SCHEDULER_FIELDS,
        cell_rows,
        cpu_strata_rows,
        deterministic_input_hashes,
        overlap_diagnostics,
        overlap_sensitivity_rows,
        preparation_identity,
        role_row,
    )
    from .authorize_replacement import SCHEMA as AUTHORIZATION_SCHEMA
    from .censoring import (
        CENSORED_CELL,
        CENSORED_TASK_IDS,
        CENSOR_FIELDS,
        CENSOR_STATUS,
        SCHEMA as CENSOR_SCHEMA,
    )
    from .collect_censored_replacement import SCHEMA as CENSORED_COLLECTION_SCHEMA
    from .collect_generation import SCHEMA as INVENTORY_SCHEMA
    from .common import ESTIMATORS, load_json, require, sha256, write_tsv
except ImportError:
    from aggregate import (  # type: ignore
        CELL_FIELDS,
        CPU_STRATA_FIELDS,
        INPUT_HASH_FIELDS,
        OVERLAP_FIELDS,
        RESULT_FIELDS,
        SCHEDULER_FIELDS,
        cell_rows,
        cpu_strata_rows,
        deterministic_input_hashes,
        overlap_diagnostics,
        overlap_sensitivity_rows,
        preparation_identity,
        role_row,
    )
    from authorize_replacement import SCHEMA as AUTHORIZATION_SCHEMA  # type: ignore
    from censoring import (  # type: ignore
        CENSORED_CELL,
        CENSORED_TASK_IDS,
        CENSOR_FIELDS,
        CENSOR_STATUS,
        SCHEMA as CENSOR_SCHEMA,
    )
    from collect_censored_replacement import (  # type: ignore
        SCHEMA as CENSORED_COLLECTION_SCHEMA,
    )
    from collect_generation import SCHEMA as INVENTORY_SCHEMA  # type: ignore
    from common import ESTIMATORS, load_json, require, sha256, write_tsv  # type: ignore


SCHEMA = "FEVC-COMPARATIVE-SCALING-COMPOSITE-COLLECTION-V2"


def validation_map(run_dir: Path) -> dict[int, tuple[Path, dict[str, Any]]]:
    output: dict[int, tuple[Path, dict[str, Any]]] = {}
    for path in sorted((run_dir / "attempts").glob("*/validations/*.json")):
        value = load_json(path)
        require(value.get("status") == "PASS", "nonpassing validation changed")
        task_id = int(value["task"]["task_id"])
        require(task_id not in output, f"duplicate successful task {task_id}")
        output[task_id] = (path, value)
    return output


def collect(base_run: Path, replacement_run: Path,
            censor_receipt_path: Path, output_dir: Path) -> dict[str, Any]:
    require(output_dir.is_dir(), "collection output directory is missing")
    base_identity = load_json(base_run / "run_identity.json")
    replacement_identity = load_json(replacement_run / "run_identity.json")
    require(replacement_identity.get("replaces_run_id") ==
            base_identity.get("run_id"), "composite generation lineage changed")
    inventory_path = base_run / "receipts" / "generation_inventory.first.json"
    inventory = load_json(inventory_path)
    authorization_paths = sorted(
        (replacement_run / "receipts" / "replacement_authorizations").glob(
            "*.json"))
    require(bool(authorization_paths),
            "replacement authorization receipts are missing")
    authorizations = [load_json(path) for path in authorization_paths]
    require(inventory.get("schema") == INVENTORY_SCHEMA and
            inventory.get("status") == "PASS" and
            all(value.get("schema") == AUTHORIZATION_SCHEMA and
                value.get("status") == "PASS" and
                value.get("base_inventory_sha256") == sha256(inventory_path)
                for value in authorizations),
            "composite compatibility receipts changed")
    affected = [int(value) for value in authorizations[0]["affected_task_ids"]]
    require(len(affected) == 72 and inventory.get("failed_task_ids") == affected,
            "composite affected-task inventory changed")
    authorized = [int(task_id) for value in authorizations
                  for task_id in value["task_ids"]]
    require(len(authorized) == len(set(authorized)) and
            sorted(authorized) == affected and
            {value["binary_manifest_sha256"] for value in authorizations} ==
            {authorizations[0]["binary_manifest_sha256"]},
            "replacement authorizations do not partition the affected tasks")

    censor_receipt = load_json(censor_receipt_path)
    censor_ledger = censor_receipt_path.parent / "censored_matlab_3.tsv"
    require(censor_receipt.get("schema") == CENSORED_COLLECTION_SCHEMA and
            censor_receipt.get("status") ==
            "PASS_WITH_REGISTERED_CENSORING" and
            censor_receipt.get("run_id") == replacement_identity.get("run_id") and
            censor_receipt.get("source_commit") ==
            replacement_identity.get("source_commit") and
            censor_receipt.get("bundle_sha256") ==
            replacement_identity.get("bundle_sha256") and
            censor_receipt.get("validated_tasks") == 69 and
            censor_receipt.get("censored_tasks") == 3 and
            censor_receipt.get("censored_task_ids") == list(CENSORED_TASK_IDS) and
            censor_receipt.get("censor_schema") == CENSOR_SCHEMA and
            censor_ledger.is_file() and not censor_ledger.is_symlink() and
            censor_receipt.get("censor_ledger_sha256") == sha256(censor_ledger),
            "registered censor receipt changed")
    with censor_ledger.open(encoding="utf-8", newline="") as handle:
        reader = csv.DictReader(handle, delimiter="\t")
        require(tuple(reader.fieldnames or ()) == CENSOR_FIELDS,
                "censor ledger fields changed")
        censor_rows = [dict(row) for row in reader]
    require(len(censor_rows) == 3 and
            [int(row["task_id"]) for row in censor_rows] ==
            list(CENSORED_TASK_IDS) and
            all(row["status"] == CENSOR_STATUS and row["role"] == "matlab" and
                row["censoring"] == "RIGHT" and
                int(row["lower_bound_seconds"]) == 10_800
                for row in censor_rows),
            "registered censor ledger changed")

    base = validation_map(base_run)
    replacement = validation_map(replacement_run)
    require(set(base) == set(range(1, 301)) - set(affected) and
            set(replacement) == set(affected) - set(CENSORED_TASK_IDS) and
            censor_receipt.get("validated_task_ids") == sorted(replacement),
            "composite validation partition changed")
    pairs = [replacement.get(task_id, base.get(task_id))
             for task_id in range(1, 301)
             if task_id not in CENSORED_TASK_IDS]
    require(all(pair is not None for pair in pairs),
            "composite validation task is missing")
    selected = [pair for pair in pairs if pair is not None]
    paths = [pair[0] for pair in selected]
    payloads = [pair[1] for pair in selected]
    require(all(
        all(item["roles"][role]["scientific_status"] == "PASS"
            for role in ESTIMATORS) and
        item["rust_mata_independent_probe_gate"]["status"] == "PASS"
        for item in payloads
    ), "composite application or scientific gate failed")
    binary_identities = {
        item["node"]["binary_manifest_sha256"] for item in payloads
    }
    require(binary_identities ==
            {authorizations[0]["binary_manifest_sha256"]},
            "composite estimator binary identity changed")

    base_runtime, base_provenance = preparation_identity(
        base_run, base_identity["source_commit"], base_identity["bundle_sha256"])
    replacement_runtime, replacement_provenance = preparation_identity(
        replacement_run, replacement_identity["source_commit"],
        replacement_identity["bundle_sha256"])
    for key in (
        "rustc", "cargo", "stata_module", "matlab_module", "plugin_sha256",
        "binary_manifest_sha256", "matlab_upstream_commit",
        "matlab_runtime_tree_sha256", "matlab_core_sha256",
        "required_stata_processors", "required_rust_threads",
        "benchmark_thread_contract",
    ):
        require(base_runtime[key] == replacement_runtime[key],
                f"composite runtime identity changed: {key}")

    results = [role_row(payload, role)
               for payload in payloads for role in ESTIMATORS]
    require(len(results) == 891, "composite result ledger must contain 891 calls")
    cells = cell_rows(results, censored_cells={CENSORED_CELL})
    overlap = overlap_diagnostics(payloads)
    scheduler = [{
        "generation_source_commit": item["task"]["source_commit"],
        "generation_bundle_sha256": item["task"]["bundle_sha256"],
        "attempt_id": item["node"]["attempt_id"],
        "task_id": item["task"]["task_id"],
        "experiment_id": item["task"]["experiment_id"],
        "jobnumber": item["qacct"]["jobnumber"],
        "taskid": item["qacct"]["taskid"],
        "hostname": item["qacct"]["hostname"],
        "cpu_model": item["node"]["cpu_model"],
        "task_start_utc": item["node"]["task_start_utc"],
        "task_end_utc": item["node"]["task_end_utc"],
        "task_start_epoch": item["node"]["task_start_epoch"],
        "task_end_epoch": item["node"]["task_end_epoch"],
        **overlap[int(item["task"]["task_id"])],
        "qacct_wall_seconds": item["qacct"]["wall_seconds"],
        "qacct_cpu_seconds": item["qacct"]["cpu_seconds"],
        "qacct_maxvmem_bytes": item["qacct"]["maxvmem_bytes"],
        "validation_sha256": sha256(path),
    } for path, item in zip(paths, payloads)]

    targets = {
        "results_891.tsv": (RESULT_FIELDS, results),
        "cell_summary_300.tsv": (CELL_FIELDS, cells),
        "scheduler_index_297.tsv": (SCHEDULER_FIELDS, scheduler),
        "input_hashes_20.tsv": (
            INPUT_HASH_FIELDS, deterministic_input_hashes(
                payloads, censored_cells={CENSORED_CELL})),
        "cpu_model_strata.tsv": (CPU_STRATA_FIELDS, cpu_strata_rows(results)),
        "overlap_sensitivity.tsv": (
            OVERLAP_FIELDS, overlap_sensitivity_rows(payloads, overlap)),
    }
    output_paths: list[Path] = []
    for name, (fields, rows) in targets.items():
        path = output_dir / name
        require(not path.exists(), f"collection target exists: {path}")
        write_tsv(path, fields, rows)
        output_paths.append(path)

    provenance_sources = [
        (base_run / "run_identity.json", "base_run_identity.json"),
        (replacement_run / "run_identity.json", "replacement_run_identity.json"),
        (inventory_path, "base_generation_inventory.json"),
        (replacement_run / "receipts" / "preparation" /
         "artifact_source.pass.json", "artifact_compatibility.json"),
        (censor_receipt_path, "censored_replacement.pass.json"),
        (censor_ledger, "censored_matlab_3.tsv"),
    ]
    provenance_sources.extend(
        (path, f"replacement_authorization_{index:02d}.json")
        for index, path in enumerate(authorization_paths, 1))
    provenance_sources.extend(
        (path, f"replacement_task_map_{index:02d}.tsv")
        for index, path in enumerate(sorted(
            (replacement_run / "submissions").glob("*.task-map.tsv")), 1))
    provenance_sources.extend(
        (path, f"base_{index:02d}_{path.name}")
        for index, path in enumerate(base_provenance, 1))
    provenance_sources.extend(
        (path, f"replacement_{index:02d}_{path.name}")
        for index, path in enumerate(replacement_provenance, 1))
    for source, name in provenance_sources:
        target = output_dir / name
        require(source.is_file() and not target.exists(),
                f"composite provenance changed: {source}")
        shutil.copy2(source, target)
        output_paths.append(target)

    return {
        "schema": SCHEMA,
        "status": "PASS_WITH_REGISTERED_CENSORING",
        "source_commit": replacement_identity["source_commit"],
        "bundle_sha256": replacement_identity["bundle_sha256"],
        "registered_tasks": 300,
        "validated_tasks": 297,
        "censored_tasks": 3,
        "estimator_calls": 891,
        "censored_matlab_attempts": 3,
        "complete_cells": 99,
        "censored_task_ids": list(CENSORED_TASK_IDS),
        "censored_cell": {
            "structure": CENSORED_CELL[0],
            "rows": CENSORED_CELL[1],
            "active_cores": CENSORED_CELL[2],
            "lower_bound_seconds": 10_800,
            "future_execution": "DO_NOT_SUBMIT",
        },
        "runtime_identity": replacement_runtime,
        "generation_identities": [
            {
                "role": "BASE_CARRIED",
                "run_id": base_identity["run_id"],
                "source_commit": base_identity["source_commit"],
                "bundle_sha256": base_identity["bundle_sha256"],
                "accepted_tasks": 228,
            },
            {
                "role": "AFFECTED_REPLACEMENT",
                "run_id": replacement_identity["run_id"],
                "source_commit": replacement_identity["source_commit"],
                "bundle_sha256": replacement_identity["bundle_sha256"],
                "accepted_tasks": 69,
                "censored_tasks": 3,
            },
        ],
        "binary_manifest_sha256": authorizations[0]["binary_manifest_sha256"],
        "compatibility_review": {
            "base_inventory_sha256": sha256(inventory_path),
            "replacement_authorization_sha256": {
                path.name: sha256(path) for path in authorization_paths
            },
            "affected_tasks": 72,
            "carried_tasks": 228,
            "replacement_accepted_tasks": 69,
            "right_censored_tasks": 3,
        },
        "scientific_status_counts": {
            "PASS": 891,
            CENSOR_STATUS: 3,
        },
        "artifact_sha256": {path.name: sha256(path) for path in output_paths},
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base-run", type=Path, required=True)
    parser.add_argument("--replacement-run", type=Path, required=True)
    parser.add_argument("--censor-receipt", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    args = parser.parse_args()
    receipt = args.output_dir / "collection.json"
    require(not receipt.exists(), "collection receipt exists")
    value = collect(args.base_run, args.replacement_run,
                    args.censor_receipt, args.output_dir)
    receipt.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n",
                       encoding="utf-8")
    print("VCKSS_COMPARATIVE_SCALING_COMPOSITE_COLLECTION_PASS "
          "validated=297 calls=891 censored=3 generations=2")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
