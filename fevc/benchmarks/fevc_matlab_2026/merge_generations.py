#!/usr/bin/env python3
"""Merge original and one retry generation without mutating either attempt."""

from __future__ import annotations

import argparse
import json
import shutil
from pathlib import Path
from typing import Any

try:
    from .collect_generation import SCHEMA
    from .common import load_json, require, sha256
except ImportError:
    from collect_generation import SCHEMA  # type: ignore
    from common import load_json, require, sha256  # type: ignore


def merge(
    run_dir: Path,
    original_path: Path,
    retry_path: Path,
    validation_dir: Path,
) -> dict[str, Any]:
    identity = load_json(run_dir / "run_identity.json")
    original = load_json(original_path)
    retry = load_json(retry_path)
    require(
        identity.get("schema") == "FEVC-MATLAB-2026-CAMPAIGN-V2"
        and identity.get("source_mode") == "CLEAN_COMMIT",
        "merged evidence requires a clean campaign",
    )
    require(
        original.get("schema") == SCHEMA
        and retry.get("schema") == SCHEMA
        and original.get("status") == retry.get("status") == "PASS"
        and original.get("run_id") == retry.get("run_id") == identity.get("run_id")
        and original.get("attempt_id") == "production"
        and retry.get("attempt_id") == "retry",
        "generation inventories are incompatible",
    )
    original_cells = {int(value) for value in original["validated_cell_ids"]}
    retry_cells = {int(value) for value in retry["validated_cell_ids"]}
    require(not original_cells & retry_cells, "generation cells overlap")
    require(original_cells | retry_cells == set(range(1, 241)),
            "merged generation is incomplete")
    retry_bundles = set(retry["expected_bundle_ids"])
    require(set(original["skipped_bundle_ids"]) == retry_bundles,
            "retry bundle inventory changed")

    require(not validation_dir.exists(), "merged validation target exists")
    validation_dir.mkdir(parents=True)
    hashes: dict[str, str] = {}
    for attempt_id, cells, inventory in (
        ("production", original_cells, original),
        ("retry", retry_cells, retry),
    ):
        for cell_id in sorted(cells):
            source = run_dir / "attempts" / attempt_id / "validations" / f"{cell_id}.json"
            require(
                source.is_file()
                and sha256(source) == inventory["validation_sha256"][str(cell_id)],
                "validation inventory hash changed",
            )
            target = validation_dir / source.name
            shutil.copyfile(source, target)
            hashes[str(cell_id)] = sha256(target)

    return {
        "schema": SCHEMA,
        "status": "PASS",
        "terminal": True,
        "run_id": identity["run_id"],
        "stage": "production",
        "attempt_id": "production+retry",
        "job_ids": [original["job_id"], retry["job_id"]],
        "expected_bundle_ids": list(range(1, 25)),
        "retry_bundle_ids": sorted(retry_bundles),
        "validated_cell_ids": list(range(1, 241)),
        "validated_cells": 240,
        "cell_filter": "NONE",
        "validation_sha256": hashes,
        "source_inventories_sha256": {
            "production": sha256(original_path),
            "retry": sha256(retry_path),
        },
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--run-dir", type=Path, required=True)
    parser.add_argument("--original", type=Path, required=True)
    parser.add_argument("--retry", type=Path, required=True)
    parser.add_argument("--validation-dir", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    require(not args.output.exists(), "merged generation target exists")
    value = merge(args.run_dir, args.original, args.retry, args.validation_dir)
    args.output.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n",
                           encoding="utf-8")
    print("FEVC_MATLAB_2026_GENERATION_MERGE_PASS cells=240")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
