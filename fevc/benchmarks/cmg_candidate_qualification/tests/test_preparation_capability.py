from __future__ import annotations

import json
import shutil
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
ADAPTER = ROOT.parent / "comparative_scaling" / "build_benchmark_ado.py"
sys.path.insert(0, str(ROOT))

from common import EvidenceError, RUN_SCHEMA, sha256  # noqa: E402
from validate_preparation import validate  # noqa: E402


def write_capability(path: Path, licensed: int) -> str:
    path.write_text(
        "key\tvalue\n"
        "schema\tVCKSS-STATA-PROCESSOR-CAPABILITY-V1\n"
        f"status\t{'PASS' if licensed >= 4 else 'FAIL'}\n"
        "required_processors\t4\n"
        f"licensed_processors\t{licensed}\n"
        "initial_processors\t4\n"
        "stata_version\t19\n"
        "stata_flavor\tIC\n"
        "stata_mp\t1\n",
        encoding="utf-8",
    )
    return sha256(path)


def staged(tmp_path: Path, licensed: int) -> None:
    identity = {
        "schema": RUN_SCHEMA,
        "status": "PASS",
        "candidate_commit": "1" * 40,
        "comparison_commit": "2" * 40,
        "required_stata_processors": 4,
        "required_rust_threads": 16,
    }
    (tmp_path / "run_identity.json").write_text(
        json.dumps(identity) + "\n", encoding="utf-8")
    receipts = tmp_path / "receipts" / "preparation"
    receipts.mkdir(parents=True)
    capability_sha = write_capability(
        receipts / "stata_processor_capability.tsv", licensed)
    adapter_source = (tmp_path / "sources" / "candidate" / "fevc" /
                      "benchmarks" / "comparative_scaling" /
                      "build_benchmark_ado.py")
    adapter_source.parent.mkdir(parents=True)
    shutil.copy2(ADAPTER, adapter_source)
    adapter_receipt = {
        "schema": "FEVC-BENCHMARK-ADO-ADAPTER-V1",
        "status": "PASS",
        "thread_contract": "FEVC-BENCHMARK-THREADS-V1",
        "maximum_stata_processors": 4,
        "allowed_native_threads": [1, 2, 4, 8, 16],
    }
    for label in ("candidate", "comparison"):
        (receipts / f"{label}_benchmark_ado_adapter.json").write_text(
            json.dumps(adapter_receipt) + "\n", encoding="utf-8")
    candidate_receipt = receipts / "candidate_benchmark_ado_adapter.json"
    comparison_receipt = receipts / "comparison_benchmark_ado_adapter.json"
    (receipts / "preparation.tsv").write_text(
        "key\tvalue\n"
        "schema\tVCKSS-CMG-CANDIDATE-QUALIFICATION-PREPARATION-V2\n"
        "status\tPASS\n"
        "job_id\t123\n"
        f"candidate_commit\t{'1' * 40}\n"
        f"comparison_commit\t{'2' * 40}\n"
        f"required_stata_processors\t4\n"
        f"licensed_stata_processors\t{licensed}\n"
        "required_rust_threads\t16\n"
        "benchmark_thread_contract\tVCKSS-BENCHMARK-THREADS-V1\n"
        f"benchmark_ado_adapter_sha256\t{sha256(adapter_source)}\n"
        f"candidate_benchmark_ado_receipt_sha256\t{sha256(candidate_receipt)}\n"
        f"comparison_benchmark_ado_receipt_sha256\t{sha256(comparison_receipt)}\n"
        f"stata_processor_capability_sha256\t{capability_sha}\n"
        f"candidate_binary_manifest_sha256\t{'a' * 64}\n"
        f"comparison_binary_manifest_sha256\t{'b' * 64}\n",
        encoding="utf-8",
    )
    (receipts / "wrapper.pass").write_text(
        f"VCKSS_CMG_CANDIDATE_QUALIFICATION_PREPARE_PASS {'1' * 40} {'2' * 40}\n",
        encoding="utf-8",
    )
    (receipts / "qacct.txt").write_text(
        "jobnumber 123\nproject welfgr\ngranted_pe omp\nslots 4\n"
        "failed 0\nexit_status 0\n",
        encoding="utf-8",
    )


def test_candidate_preparation_requires_four_licensed_stata_processors(
        tmp_path: Path) -> None:
    staged(tmp_path, 4)
    value = validate(tmp_path)
    assert value["required_stata_processors"] == 4
    assert value["licensed_stata_processors"] == 4
    assert value["required_rust_threads"] == 16


def test_candidate_preparation_rejects_three_core_entitlement(
        tmp_path: Path) -> None:
    staged(tmp_path, 3)
    with pytest.raises(EvidenceError, match="processor capability"):
        validate(tmp_path)
