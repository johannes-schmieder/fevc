#!/usr/bin/env python3
"""Verify exact-source small and worst pilots before production submission."""

from __future__ import annotations

import argparse
from pathlib import Path

try:
    from .common import load_json, require
    from .validate_pilot import PILOT_SCHEMA, RUN_SCHEMA
except ImportError:
    from common import load_json, require  # type: ignore
    from validate_pilot import PILOT_SCHEMA, RUN_SCHEMA  # type: ignore


SHARED_FIELDS = (
    "source_commit", "bundle_sha256", "source_manifest_sha256",
    "task_manifest_sha256", "bundle_manifest_sha256",
    "stata_spi_manifest_sha256", "artifact_source_run_id",
    "mem_per_core_gib", "command_memory_gib", "required_stata_processors",
    "required_rust_threads", "required_matlab_workers",
)


def verify(production_path: Path, root: Path) -> dict[str, object]:
    production = load_json(production_path)
    require(production.get("schema") == RUN_SCHEMA and
            production.get("status") == "PASS" and
            production.get("run_kind") == "production",
            "production identity changed")
    small_id = production.get("pilot_small_run_id")
    worst_id = production.get("pilot_worst_run_id")
    require(isinstance(small_id, str) and isinstance(worst_id, str) and
            small_id != worst_id, "production pilot IDs changed")
    small = load_json(root / small_id / "receipts" / "pilot.pass.json")
    worst = load_json(root / worst_id / "receipts" / "pilot.pass.json")
    for pilot, kind, task_id, run_id in (
        (small, "pilot-small", 1, small_id),
        (worst, "pilot-worst", 235, worst_id),
    ):
        require(pilot.get("schema") == PILOT_SCHEMA and
                pilot.get("status") == "PASS" and
                pilot.get("run_kind") == kind and pilot.get("task_id") == task_id and
                pilot.get("run_id") == run_id,
                f"{kind} receipt changed")
        for field in SHARED_FIELDS:
            require(pilot.get(field) == production.get(field),
                    f"{kind} {field} differs from production")
    require(small.get("binary_manifest_sha256") ==
            worst.get("binary_manifest_sha256"), "pilot binaries differ")
    return {
        "production_run_id": production["run_id"],
        "pilot_small_run_id": small_id,
        "pilot_worst_run_id": worst_id,
        "source_commit": production["source_commit"],
        "binary_manifest_sha256": small["binary_manifest_sha256"],
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--production", type=Path, required=True)
    parser.add_argument("--root", type=Path, required=True)
    args = parser.parse_args()
    value = verify(args.production, args.root)
    print("FEVC_MATLAB_2026_PILOT_GATE_PASS "
          f"production={value['production_run_id']} "
          f"small={value['pilot_small_run_id']} worst={value['pilot_worst_run_id']} "
          f"source={value['source_commit']} binary={value['binary_manifest_sha256']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
