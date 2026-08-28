#!/usr/bin/env python3
"""Validate one comparative-scaling preparation job and its SCC accounting."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

try:
    from .common import key_values, load_json, require, sha256
    from .validate_pilot import RUN_SCHEMA
except ImportError:
    from common import key_values, load_json, require, sha256  # type: ignore
    from validate_pilot import RUN_SCHEMA  # type: ignore


PREPARATION_QACCT_SCHEMA = "VCKSS-COMPARATIVE-SCALING-PREPARATION-QACCT-V2"
ARTIFACT_SOURCE_SCHEMA = "VCKSS-COMPARATIVE-SCALING-ARTIFACT-SOURCE-V1"


def raw_qacct(path: Path) -> dict[str, str]:
    require(path.is_file() and not path.is_symlink(), "preparation qacct is missing")
    values: dict[str, str] = {}
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        fields = line.strip().split(None, 1)
        if len(fields) == 2:
            values[fields[0]] = fields[1]
    required = {"jobnumber", "project", "granted_pe", "slots", "failed",
                "exit_status", "hostname"}
    require(required <= values.keys(), "incomplete preparation qacct")
    return values


def validate(run_dir: Path) -> dict[str, Any]:
    identity = load_json(run_dir / "run_identity.json")
    receipt_dir = run_dir / "receipts" / "preparation"
    preparation = key_values(receipt_dir / "preparation.tsv")
    capability_path = receipt_dir / "stata_processor_capability.tsv"
    capability = key_values(capability_path)
    require(identity.get("schema") == RUN_SCHEMA and identity.get("status") == "PASS",
            "staged-run identity changed")
    require(preparation.get("schema") ==
            "VCKSS-COMPARATIVE-SCALING-PREPARATION-V2" and
            preparation.get("status") == "PASS" and
            preparation.get("source_commit") == identity.get("source_commit") and
            preparation.get("bundle_sha256") == identity.get("bundle_sha256") and
            preparation.get("source_manifest_sha256") ==
            identity.get("source_manifest_sha256"),
            "preparation identity changed")
    artifact_source_run_id = identity.get("artifact_source_run_id")
    artifact_mode = preparation.get("artifact_mode")
    artifact_source_receipt_sha256: str | None = None
    if identity.get("run_kind") == "preparation":
        require(artifact_source_run_id is None and
                artifact_mode == "BUILT_CANONICAL" and
                preparation.get("artifact_source_run_id") == "NONE" and
                preparation.get("artifact_source_receipt_sha256") == "NONE",
                "canonical artifact preparation identity changed")
    else:
        require(isinstance(artifact_source_run_id, str) and
                artifact_mode == "IMPORTED_CANONICAL" and
                preparation.get("artifact_source_run_id") ==
                artifact_source_run_id,
                "imported artifact preparation identity changed")
        artifact_source_path = receipt_dir / "artifact_source.pass.json"
        artifact_source = load_json(artifact_source_path)
        artifact_source_receipt_sha256 = sha256(artifact_source_path)
        require(artifact_source.get("schema") == ARTIFACT_SOURCE_SCHEMA and
                artifact_source.get("status") == "PASS" and
                artifact_source.get("artifact_source_run_id") ==
                artifact_source_run_id and
                artifact_source.get("source_commit") ==
                identity.get("source_commit") and
                artifact_source.get("bundle_sha256") ==
                identity.get("bundle_sha256") and
                artifact_source.get("binary_manifest_sha256") ==
                preparation.get("binary_manifest_sha256") and
                preparation.get("artifact_source_receipt_sha256") ==
                artifact_source_receipt_sha256,
                "canonical artifact-source receipt changed")
    required = identity.get("required_stata_processors")
    require(isinstance(required, int) and required == 4,
            "required Stata processor identity changed")
    licensed_text = capability.get("licensed_processors", "")
    require(capability.get("schema") == "VCKSS-STATA-PROCESSOR-CAPABILITY-V1" and
            capability.get("status") == "PASS" and
            capability.get("required_processors") == str(required) and
            licensed_text.isdigit() and int(licensed_text) >= required and
            preparation.get("required_stata_processors") == str(required) and
            preparation.get("licensed_stata_processors") == licensed_text and
            preparation.get("stata_processor_capability_sha256") ==
            sha256(capability_path),
            "Stata processor capability gate failed")
    required_rust = identity.get("required_rust_threads")
    adapter_path = receipt_dir / "benchmark_ado_adapter.json"
    adapter_source = (run_dir / "source" / "vckss" / "benchmarks" /
                      "comparative_scaling" / "build_benchmark_ado.py")
    adapter = load_json(adapter_path)
    require(required_rust == 16 and
            preparation.get("required_rust_threads") == "16" and
            preparation.get("benchmark_thread_contract") ==
            "VCKSS-BENCHMARK-THREADS-V1" and
            adapter.get("schema") == "VCKSS-BENCHMARK-ADO-ADAPTER-V1" and
            adapter.get("status") == "PASS" and
            adapter.get("thread_contract") == "VCKSS-BENCHMARK-THREADS-V1" and
            adapter.get("maximum_stata_processors") == 4 and
            adapter.get("allowed_native_threads") == [1, 2, 4, 8, 16] and
            preparation.get("benchmark_ado_adapter_sha256") ==
            sha256(adapter_source) and
            preparation.get("benchmark_ado_receipt_sha256") == sha256(adapter_path),
            "benchmark thread adapter gate failed")
    require((receipt_dir / "wrapper.pass").read_text(encoding="utf-8").strip() ==
            f"VCKSS_COMPARATIVE_SCALING_PREPARE_PASS {identity['source_commit']} "
            f"{identity['bundle_sha256']}" and
            not (receipt_dir / "wrapper.fail").exists(),
            "preparation wrapper failed")

    qacct_path = receipt_dir / "qacct.txt"
    qacct = raw_qacct(qacct_path)
    require(qacct["jobnumber"] == preparation.get("job_id") and
            qacct["project"] == "welfgr" and qacct["slots"] == "4" and
            qacct["granted_pe"] in {"omp", "omp4"} and
            qacct["failed"] == qacct["exit_status"] == "0",
            "preparation scheduler gate failed")
    effective_path = run_dir / "submissions" / "prepare.effective-sge.json"
    effective = load_json(effective_path)
    require(effective.get("schema") == "VCKSS-SGE-EFFECTIVE-SUBMISSION-V1" and
            effective.get("status") == "PASS" and
            effective.get("parallel_environment_request") == "omp 4" and
            effective.get("hard_resources") == {
                "h_rt": "7200", "mem_per_core": "4G", "no_gpu": "TRUE"
            },
            "preparation effective submission changed")

    return {
        "schema": PREPARATION_QACCT_SCHEMA,
        "status": "PASS",
        "run_id": identity["run_id"],
        "run_kind": identity["run_kind"],
        "job_id": qacct["jobnumber"],
        "hostname": qacct["hostname"],
        "source_commit": identity["source_commit"],
        "bundle_sha256": identity["bundle_sha256"],
        "source_manifest_sha256": identity["source_manifest_sha256"],
        "binary_manifest_sha256": preparation["binary_manifest_sha256"],
        "artifact_mode": artifact_mode,
        "artifact_source_run_id": artifact_source_run_id,
        "artifact_source_receipt_sha256": artifact_source_receipt_sha256,
        "required_stata_processors": required,
        "licensed_stata_processors": int(licensed_text),
        "required_rust_threads": required_rust,
        "benchmark_thread_contract": preparation["benchmark_thread_contract"],
        "benchmark_ado_adapter_sha256":
            preparation["benchmark_ado_adapter_sha256"],
        "benchmark_ado_receipt_sha256": sha256(adapter_path),
        "stata_processor_capability_sha256": sha256(capability_path),
        "qacct_sha256": sha256(qacct_path),
        "preparation_receipt_sha256": sha256(receipt_dir / "preparation.tsv"),
        "effective_submission_sha256": sha256(effective_path),
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--run-dir", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    require(not args.output.exists(), "preparation accounting target exists")
    value = validate(args.run_dir)
    args.output.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n",
                           encoding="utf-8")
    print(f"VCKSS_COMPARATIVE_SCALING_PREPARATION_QACCT_PASS {value['job_id']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
