from __future__ import annotations

import json
import shutil
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from common import EvidenceError, RESULT_SCHEMA, sha256  # noqa: E402
from validate_preparation import (  # noqa: E402
    PREPARATION_QACCT_SCHEMA,
    validate as validate_preparation,
)
from validate_pilot import PILOT_SCHEMA, RUN_SCHEMA, validate_pilot  # noqa: E402
from verify_pilots import verify  # noqa: E402


HEX_A = "a" * 64
HEX_B = "b" * 64
HEX_C = "c" * 64
HEX_D = "d" * 64
COMMIT = "1" * 40


def write_json(path: Path, value: dict[str, object]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value) + "\n", encoding="utf-8")


def staged(run_id: str, run_kind: str) -> dict[str, object]:
    return {
        "schema": RUN_SCHEMA,
        "status": "PASS",
        "run_id": run_id,
        "run_kind": run_kind,
        "pilot_small_run_id": None,
        "pilot_worst_run_id": None,
        "source_commit": COMMIT,
        "bundle_sha256": HEX_A,
        "source_manifest_sha256": HEX_B,
        "task_manifest_sha256": HEX_C,
        "stata_spi_manifest_sha256": HEX_D,
        "mem_per_core_gib": 8,
        "command_memory_gib": 112,
        "required_stata_processors": 4,
        "maximum_mata_cores": 4,
        "required_rust_threads": 16,
        "required_matlab_workers": 16,
    }


def role(name: str) -> dict[str, object]:
    value: dict[str, object] = {
        "scientific_status": "PASS",
        "application_receipt_valid": True,
        "application_exit_status": 0,
        "monitor_exit_status": 0,
        "timed_out": False,
        "command_seconds": 2.0,
        "estimator_phase_peak_rss_bytes": 1000,
        "whole_process_peak_rss_bytes": 1100,
        "gnu_peak_rss_bytes": 1200,
    }
    if name in {"mata", "rust"}:
        value.update({"memory_forecast_bytes": 1300,
                      "resource_peak_bytes": 1400})
    return value


def write_capability(path: Path, licensed: int = 4) -> str:
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


def write_preparation_qacct(path: Path, capability_sha: str,
                            licensed: int = 4) -> None:
    write_json(path, {
        "schema": PREPARATION_QACCT_SCHEMA,
        "status": "PASS",
        "source_commit": COMMIT,
        "bundle_sha256": HEX_A,
        "binary_manifest_sha256": HEX_B,
        "required_stata_processors": 4,
        "licensed_stata_processors": licensed,
        "required_rust_threads": 16,
        "benchmark_thread_contract": "VCKSS-BENCHMARK-THREADS-V1",
        "benchmark_ado_adapter_sha256": HEX_C,
        "benchmark_ado_receipt_sha256": HEX_D,
        "stata_processor_capability_sha256": capability_sha,
    })


def stage_adapter(run_dir: Path, preparation: dict[str, str]) -> None:
    source = (run_dir / "source" / "vckss" / "benchmarks" /
              "comparative_scaling" / "build_benchmark_ado.py")
    source.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(ROOT / "build_benchmark_ado.py", source)
    receipt = run_dir / "receipts" / "preparation" / "benchmark_ado_adapter.json"
    write_json(receipt, {
        "schema": "VCKSS-BENCHMARK-ADO-ADAPTER-V1",
        "status": "PASS",
        "thread_contract": "VCKSS-BENCHMARK-THREADS-V1",
        "maximum_stata_processors": 4,
        "allowed_native_threads": [1, 2, 4, 8, 16],
    })
    preparation.update({
        "required_rust_threads": "16",
        "benchmark_thread_contract": "VCKSS-BENCHMARK-THREADS-V1",
        "benchmark_ado_adapter_sha256": sha256(source),
        "benchmark_ado_receipt_sha256": sha256(receipt),
    })


def write_preparation_tsv(path: Path, values: dict[str, str]) -> None:
    path.write_text(
        "key\tvalue\n" + "".join(f"{key}\t{value}\n" for key, value in values.items()),
        encoding="utf-8",
    )


def test_pilot_requires_all_scientific_application_and_memory_gates(
        tmp_path: Path) -> None:
    identity = staged("small-run", "pilot-small")
    write_json(tmp_path / "run_identity.json", identity)
    validation = {
        "schema": RESULT_SCHEMA,
        "status": "PASS",
        "task": {"task_id": "7", "source_commit": COMMIT,
                 "bundle_sha256": HEX_A},
        "task_sha256": HEX_C,
        "input_sha256": HEX_D,
        "node": {"attempt_id": "first", "source_commit": COMMIT,
                 "bundle_sha256": HEX_A, "source_manifest_sha256": HEX_B,
                 "binary_manifest_sha256": HEX_A, "task_sha256": HEX_C,
                 "input_sha256": HEX_D},
        "qacct": {"jobnumber": "123", "taskid": "7",
                  "maxvmem_bytes": 4096},
        "roles": {name: role(name) for name in ("mata", "rust", "matlab")},
        "rust_mata_independent_probe_gate": {"status": "PASS"},
    }
    target = tmp_path / "attempts" / "first" / "validations" / "7.json"
    write_json(target, validation)
    write_preparation_qacct(
        tmp_path / "receipts" / "preparation" / "qacct.pass.json", HEX_D)
    receipt = validate_pilot(tmp_path, "first", 7)
    assert receipt["schema"] == PILOT_SCHEMA
    assert receipt["status"] == "PASS"
    assert receipt["binary_manifest_sha256"] == HEX_A

    validation["roles"]["matlab"]["scientific_status"] = "NUMERICAL_REJECTED"
    write_json(target, validation)
    with pytest.raises(EvidenceError, match="MATLAB|matlab"):
        validate_pilot(tmp_path, "first", 7)


def test_production_gate_requires_source_manifest_and_binary_identity(
        tmp_path: Path) -> None:
    production = staged("production-run", "production")
    production["pilot_small_run_id"] = "small-run"
    production["pilot_worst_run_id"] = "worst-run"
    production_path = tmp_path / "run_identity.json"
    write_json(production_path, production)
    receipt_dir = tmp_path / "receipts" / "preparation"
    receipt_dir.mkdir(parents=True)
    capability_sha = write_capability(
        receipt_dir / "stata_processor_capability.tsv")
    write_preparation_tsv(receipt_dir / "preparation.tsv", {
        "schema": "VCKSS-COMPARATIVE-SCALING-PREPARATION-V2",
        "status": "PASS", "source_commit": COMMIT, "bundle_sha256": HEX_A,
        "binary_manifest_sha256": HEX_B, "required_stata_processors": "4",
        "licensed_stata_processors": "4", "required_rust_threads": "16",
        "stata_processor_capability_sha256": capability_sha,
    })
    write_preparation_qacct(
        receipt_dir / "qacct.pass.json", capability_sha)
    pilots = []
    for run_id, run_kind, task_id in (
        ("small-run", "pilot-small", 7),
        ("worst-run", "pilot-worst", 298),
    ):
        pilot = {**staged(run_id, run_kind), "schema": PILOT_SCHEMA,
                 "task_id": task_id, "binary_manifest_sha256": HEX_B,
                 "licensed_stata_processors": 4,
                 "benchmark_thread_contract": "VCKSS-BENCHMARK-THREADS-V1",
                 "benchmark_ado_adapter_sha256": HEX_C,
                 "benchmark_ado_receipt_sha256": HEX_D,
                 "stata_processor_capability_sha256": capability_sha}
        path = tmp_path / f"{run_kind}.json"
        write_json(path, pilot)
        pilots.append(path)
    value = verify(production_path, pilots[0], pilots[1])
    assert value["binary_manifest_sha256"] == HEX_B

    worst = json.loads(pilots[1].read_text(encoding="utf-8"))
    worst["source_manifest_sha256"] = HEX_A
    write_json(pilots[1], worst)
    with pytest.raises(EvidenceError, match="source_manifest"):
        verify(production_path, pilots[0], pilots[1])


def test_preparation_accounting_is_source_and_effective_submission_bound(
        tmp_path: Path) -> None:
    identity = staged("preparation-run", "preparation")
    write_json(tmp_path / "run_identity.json", identity)
    receipt_dir = tmp_path / "receipts" / "preparation"
    receipt_dir.mkdir(parents=True)
    capability_path = receipt_dir / "stata_processor_capability.tsv"
    capability_sha = write_capability(capability_path)
    preparation = {
        "schema": "VCKSS-COMPARATIVE-SCALING-PREPARATION-V2",
        "status": "PASS", "job_id": "123", "source_commit": COMMIT,
        "bundle_sha256": HEX_A, "source_manifest_sha256": HEX_B,
        "binary_manifest_sha256": HEX_C, "required_stata_processors": "4",
        "licensed_stata_processors": "4",
        "stata_processor_capability_sha256": capability_sha,
    }
    stage_adapter(tmp_path, preparation)
    write_preparation_tsv(receipt_dir / "preparation.tsv", preparation)
    (receipt_dir / "wrapper.pass").write_text(
        f"VCKSS_COMPARATIVE_SCALING_PREPARE_PASS {COMMIT} {HEX_A}\n",
        encoding="utf-8",
    )
    (receipt_dir / "qacct.txt").write_text(
        "jobnumber 123\nhostname test.scc.bu.edu\nproject welfgr\n"
        "granted_pe omp\nslots 4\nfailed 0\nexit_status 0\n",
        encoding="utf-8",
    )
    write_json(tmp_path / "submissions" / "prepare.effective-sge.json", {
        "schema": "VCKSS-SGE-EFFECTIVE-SUBMISSION-V1",
        "status": "PASS",
        "parallel_environment_request": "omp 4",
        "hard_resources": {
            "h_rt": "7200", "mem_per_core": "4G", "no_gpu": "TRUE",
        },
    })

    value = validate_preparation(tmp_path)

    assert value["schema"] == PREPARATION_QACCT_SCHEMA
    assert value["binary_manifest_sha256"] == HEX_C
    assert value["required_stata_processors"] == 4
    assert value["licensed_stata_processors"] == 4
    assert value["required_rust_threads"] == 16

    insufficient_sha = write_capability(capability_path, licensed=3)
    source = (receipt_dir / "preparation.tsv").read_text(encoding="utf-8")
    source = source.replace("licensed_stata_processors\t4",
                            "licensed_stata_processors\t3")
    source = source.replace(capability_sha, insufficient_sha)
    (receipt_dir / "preparation.tsv").write_text(source, encoding="utf-8")
    with pytest.raises(EvidenceError, match="processor capability"):
        validate_preparation(tmp_path)
