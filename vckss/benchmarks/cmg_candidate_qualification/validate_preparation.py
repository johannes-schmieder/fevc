#!/usr/bin/env python3
"""Validate the scheduled two-binary preparation before qualification submission."""

from __future__ import annotations

import argparse
import json
from pathlib import Path

from common import RUN_SCHEMA, key_values, load_json, require, sha256


def raw_qacct(path: Path) -> dict[str, str]:
    values: dict[str, str] = {}
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        fields = line.strip().split(None, 1)
        if len(fields) == 2:
            values[fields[0]] = fields[1]
    require({"jobnumber", "project", "granted_pe", "slots", "failed", "exit_status"}
            <= values.keys(), "incomplete preparation qacct")
    return values


def validate(run_dir: Path) -> dict:
    identity = load_json(run_dir / "run_identity.json")
    receipt_dir = run_dir / "receipts" / "preparation"
    preparation = key_values(receipt_dir / "preparation.tsv")
    require(identity.get("schema") == RUN_SCHEMA and identity.get("status") == "PASS",
            "run identity changed")
    require(preparation.get("schema") ==
            "VCKSS-CMG-CANDIDATE-QUALIFICATION-PREPARATION-V1" and
            preparation.get("status") == "PASS" and
            preparation.get("candidate_commit") == identity.get("candidate_commit") and
            preparation.get("comparison_commit") == identity.get("comparison_commit"),
            "preparation identity changed")
    require((receipt_dir / "wrapper.pass").read_text(encoding="utf-8").strip() ==
            f"VCKSS_CMG_CANDIDATE_QUALIFICATION_PREPARE_PASS {identity['candidate_commit']} {identity['comparison_commit']}" and
            not (receipt_dir / "wrapper.fail").exists(), "preparation wrapper failed")
    qacct_path = receipt_dir / "qacct.txt"
    qacct = raw_qacct(qacct_path)
    require(qacct["jobnumber"] == preparation.get("job_id") and
            qacct["project"] == "welfgr" and qacct["slots"] == "4" and
            qacct["granted_pe"] in {"omp", "omp16"} and
            qacct["failed"] == qacct["exit_status"] == "0",
            "preparation scheduler gate failed")
    return {
        "schema": "VCKSS-CMG-CANDIDATE-QUALIFICATION-PREPARATION-QACCT-V1",
        "status": "PASS", "job_id": qacct["jobnumber"],
        "candidate_commit": identity["candidate_commit"],
        "comparison_commit": identity["comparison_commit"],
        "candidate_binary_manifest_sha256":
            preparation["candidate_binary_manifest_sha256"],
        "comparison_binary_manifest_sha256":
            preparation["comparison_binary_manifest_sha256"],
        "qacct_sha256": sha256(qacct_path),
        "preparation_receipt_sha256": sha256(receipt_dir / "preparation.tsv"),
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--run-dir", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    require(not args.output.exists(), "preparation accounting target exists")
    value = validate(args.run_dir)
    args.output.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"VCKSS_CMG_CANDIDATE_QUALIFICATION_PREPARATION_QACCT_PASS {value['job_id']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
