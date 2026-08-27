#!/usr/bin/env python3
"""Verify exact-source small and worst-case pilots before production submission."""

from __future__ import annotations

import argparse
from pathlib import Path

try:
    from .common import key_values, load_json, require
    from .validate_pilot import PILOT_SCHEMA, RUN_SCHEMA
except ImportError:
    from common import key_values, load_json, require  # type: ignore
    from validate_pilot import PILOT_SCHEMA, RUN_SCHEMA  # type: ignore


SHARED_FIELDS = (
    "source_commit",
    "bundle_sha256",
    "source_manifest_sha256",
    "task_manifest_sha256",
    "stata_spi_manifest_sha256",
    "mem_per_core_gib",
    "command_memory_gib",
    "required_stata_processors",
)


def verify(production_path, small_path, worst_path):
    production = load_json(production_path)
    small = load_json(small_path)
    worst = load_json(worst_path)
    require(production.get("schema") == RUN_SCHEMA and
            production.get("status") == "PASS" and
            production.get("run_kind") == "production",
            "production staged-run identity changed")
    expected = ((small, "pilot-small", 7, production.get("pilot_small_run_id")),
                (worst, "pilot-worst", 298, production.get("pilot_worst_run_id")))
    for pilot, run_kind, task_id, run_id in expected:
        require(pilot.get("schema") == PILOT_SCHEMA and
                pilot.get("status") == "PASS" and
                pilot.get("run_kind") == run_kind and
                pilot.get("task_id") == task_id and
                pilot.get("run_id") == run_id,
                f"{run_kind} receipt changed")
        for field in SHARED_FIELDS:
            require(pilot.get(field) == production.get(field),
                    f"{run_kind} {field} differs from production")
    preparation = key_values(
        production_path.parent / "receipts" / "preparation" / "preparation.tsv")
    preparation_qacct = load_json(
        production_path.parent / "receipts" / "preparation" / "qacct.pass.json")
    require(preparation.get("schema") ==
            "VCKSS-COMPARATIVE-SCALING-PREPARATION-V1" and
            preparation.get("status") == "PASS" and
            preparation.get("source_commit") == production.get("source_commit") and
            preparation.get("bundle_sha256") == production.get("bundle_sha256"),
            "production preparation identity changed")
    binary_sha = preparation.get("binary_manifest_sha256")
    require(preparation_qacct.get("schema") ==
            "VCKSS-COMPARATIVE-SCALING-PREPARATION-QACCT-V1" and
            preparation_qacct.get("status") == "PASS" and
            preparation_qacct.get("source_commit") == production.get("source_commit") and
            preparation_qacct.get("bundle_sha256") == production.get("bundle_sha256") and
            preparation_qacct.get("binary_manifest_sha256") == binary_sha and
            preparation_qacct.get("required_stata_processors") ==
            production.get("required_stata_processors") == 16 and
            int(preparation_qacct.get("licensed_stata_processors", 0)) >= 16 and
            preparation.get("required_stata_processors") == "16" and
            preparation.get("licensed_stata_processors") ==
            str(preparation_qacct.get("licensed_stata_processors")) and
            preparation.get("stata_processor_capability_sha256") ==
            preparation_qacct.get("stata_processor_capability_sha256"),
            "production preparation accounting changed")
    require(small.get("licensed_stata_processors") ==
            worst.get("licensed_stata_processors") ==
            preparation_qacct.get("licensed_stata_processors") and
            small.get("stata_processor_capability_sha256") ==
            worst.get("stata_processor_capability_sha256") ==
            preparation_qacct.get("stata_processor_capability_sha256"),
            "pilot and production Stata capability differs")
    require(bool(binary_sha) and small.get("binary_manifest_sha256") == binary_sha and
            worst.get("binary_manifest_sha256") == binary_sha,
            "pilot and production binaries differ")
    return {
        "production_run_id": production["run_id"],
        "pilot_small_run_id": small["run_id"],
        "pilot_worst_run_id": worst["run_id"],
        "source_commit": production["source_commit"],
        "binary_manifest_sha256": binary_sha,
        "required_stata_processors": production["required_stata_processors"],
        "licensed_stata_processors": preparation_qacct["licensed_stata_processors"],
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--production", type=Path, required=True)
    parser.add_argument("--small", type=Path, required=True)
    parser.add_argument("--worst", type=Path, required=True)
    args = parser.parse_args()
    value = verify(args.production, args.small, args.worst)
    print("VCKSS_COMPARATIVE_SCALING_PILOT_GATE_PASS "
          f"production={value['production_run_id']} "
          f"small={value['pilot_small_run_id']} worst={value['pilot_worst_run_id']} "
          f"source={value['source_commit']} binary={value['binary_manifest_sha256']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
