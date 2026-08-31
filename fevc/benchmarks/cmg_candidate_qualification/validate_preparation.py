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
    capability_path = receipt_dir / "stata_processor_capability.tsv"
    capability = key_values(capability_path)
    require(identity.get("schema") == RUN_SCHEMA and identity.get("status") == "PASS",
            "run identity changed")
    require(preparation.get("schema") ==
            "FEVC-CMG-CANDIDATE-QUALIFICATION-PREPARATION-V2" and
            preparation.get("status") == "PASS" and
            preparation.get("candidate_commit") == identity.get("candidate_commit") and
            preparation.get("comparison_commit") == identity.get("comparison_commit"),
            "preparation identity changed")
    required = identity.get("required_stata_processors")
    require(isinstance(required, int) and required == 4,
            "required Stata processor identity changed")
    licensed_text = capability.get("licensed_processors", "")
    require(capability.get("schema") == "FEVC-STATA-PROCESSOR-CAPABILITY-V1" and
            capability.get("status") == "PASS" and
            capability.get("required_processors") == str(required) and
            licensed_text.isdigit() and int(licensed_text) >= required and
            preparation.get("required_stata_processors") == str(required) and
            preparation.get("licensed_stata_processors") == licensed_text and
            preparation.get("stata_processor_capability_sha256") ==
            sha256(capability_path),
            "Stata processor capability gate failed")
    required_rust = identity.get("required_rust_threads")
    adapter_source = (run_dir / "sources" / "candidate" / "fevc" /
                      "benchmarks" / "comparative_scaling" /
                      "build_benchmark_ado.py")
    candidate_adapter_path = receipt_dir / "candidate_benchmark_ado_adapter.json"
    comparison_adapter_path = receipt_dir / "comparison_benchmark_ado_adapter.json"
    candidate_adapter = load_json(candidate_adapter_path)
    comparison_adapter = load_json(comparison_adapter_path)
    require(required_rust == 16 and
            preparation.get("required_rust_threads") == "16" and
            preparation.get("benchmark_thread_contract") ==
            "FEVC-BENCHMARK-THREADS-V1" and
            preparation.get("benchmark_ado_adapter_sha256") ==
            sha256(adapter_source) and
            preparation.get("candidate_benchmark_ado_receipt_sha256") ==
            sha256(candidate_adapter_path) and
            preparation.get("comparison_benchmark_ado_receipt_sha256") ==
            sha256(comparison_adapter_path) and
            all(value.get("schema") == "FEVC-BENCHMARK-ADO-ADAPTER-V1" and
                value.get("status") == "PASS" and
                value.get("thread_contract") == "FEVC-BENCHMARK-THREADS-V1" and
                value.get("maximum_stata_processors") == 4 and
                value.get("allowed_native_threads") == [1, 2, 4, 8, 16]
                for value in (candidate_adapter, comparison_adapter)),
            "paired benchmark thread adapter gate failed")
    require((receipt_dir / "wrapper.pass").read_text(encoding="utf-8").strip() ==
            f"VCKSS_CMG_CANDIDATE_QUALIFICATION_PREPARE_PASS {identity['candidate_commit']} {identity['comparison_commit']}" and
            not (receipt_dir / "wrapper.fail").exists(), "preparation wrapper failed")
    qacct_path = receipt_dir / "qacct.txt"
    qacct = raw_qacct(qacct_path)
    require(qacct["jobnumber"] == preparation.get("job_id") and
            qacct["project"] == "welfgr" and qacct["slots"] == "4" and
            qacct["granted_pe"] in {"omp", "omp4"} and
            qacct["failed"] == qacct["exit_status"] == "0",
            "preparation scheduler gate failed")
    return {
        "schema": "FEVC-CMG-CANDIDATE-QUALIFICATION-PREPARATION-QACCT-V2",
        "status": "PASS", "job_id": qacct["jobnumber"],
        "candidate_commit": identity["candidate_commit"],
        "comparison_commit": identity["comparison_commit"],
        "candidate_binary_manifest_sha256":
            preparation["candidate_binary_manifest_sha256"],
        "comparison_binary_manifest_sha256":
            preparation["comparison_binary_manifest_sha256"],
        "required_stata_processors": required,
        "licensed_stata_processors": int(licensed_text),
        "required_rust_threads": required_rust,
        "benchmark_thread_contract": preparation["benchmark_thread_contract"],
        "benchmark_ado_adapter_sha256":
            preparation["benchmark_ado_adapter_sha256"],
        "candidate_benchmark_ado_receipt_sha256":
            sha256(candidate_adapter_path),
        "comparison_benchmark_ado_receipt_sha256":
            sha256(comparison_adapter_path),
        "stata_processor_capability_sha256": sha256(capability_path),
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
