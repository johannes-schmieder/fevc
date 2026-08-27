from __future__ import annotations

import json
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from common import EvidenceError, RESULT_SCHEMA  # noqa: E402
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
    (receipt_dir / "preparation.tsv").write_text(
        "key\tvalue\n"
        "schema\tVCKSS-COMPARATIVE-SCALING-PREPARATION-V1\n"
        "status\tPASS\n"
        f"source_commit\t{COMMIT}\n"
        f"bundle_sha256\t{HEX_A}\n"
        f"binary_manifest_sha256\t{HEX_B}\n",
        encoding="utf-8",
    )
    pilots = []
    for run_id, run_kind, task_id in (
        ("small-run", "pilot-small", 7),
        ("worst-run", "pilot-worst", 298),
    ):
        pilot = {**staged(run_id, run_kind), "schema": PILOT_SCHEMA,
                 "task_id": task_id, "binary_manifest_sha256": HEX_B}
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
