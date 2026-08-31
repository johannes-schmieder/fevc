from __future__ import annotations

import hashlib
import importlib.util
import json
import sys
from pathlib import Path

import pytest


ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from common import (  # noqa: E402
    CORES,
    ROWS,
    feasibility_tasks,
    gate_task,
    read_manifest,
    topup_tasks,
    write_manifest,
)
from generate_input import generate  # noqa: E402
from finalize_stage import parse_qacct, stage_is_accepted  # noqa: E402
from validate import optional_int, validate_matlab_nonconvergence  # noqa: E402


def test_registered_feasibility_grid_and_rotation() -> None:
    dimensions = {
        480_000: (80_000, 40_000, 1_888, 10_800, 40_000, 64),
        1_920_000: (320_000, 160_000, 2_088, 36_000, 40_000, 128),
        7_680_000: (1_280_000, 640_000, 2_288, 43_200, 40_000, 256),
    }
    for rows in ROWS:
        tasks = feasibility_tasks(rows)
        assert [value.cores for value in tasks] == list(CORES)
        expected = dimensions[rows]
        assert (tasks[0].workers, tasks[0].firms, tasks[0].probes,
                tasks[0].role_cap_seconds, tasks[0].maxiter,
                tasks[0].memory_gib) == expected
        assert {value.order for value in tasks} == {"rust_matlab", "matlab_rust"}


def test_gate_and_topup_manifests_round_trip(tmp_path: Path) -> None:
    gate_path = tmp_path / "gate.tsv"
    write_manifest(gate_path, [gate_task()])
    assert read_manifest(gate_path)[0]["replicate"] == "0"
    assert read_manifest(gate_path)[0]["maxiter"] == "20000"

    topup_path = tmp_path / "topup-480000.tsv"
    values = topup_tasks([(480_000, 4), (480_000, 16)])
    write_manifest(topup_path, values)
    parsed = read_manifest(topup_path)
    assert len(parsed) == 4
    assert [int(value["replicate"]) for value in parsed] == [2, 3, 2, 3]
    assert {value["stage"] for value in parsed} == {"topup-480000"}


def test_small_input_is_streamed_and_hash_bound(tmp_path: Path) -> None:
    output = tmp_path / "input.csv"
    receipt_path = tmp_path / "input.json"
    receipt = generate(6_000, output, receipt_path)
    assert receipt["workers"] == 1_000
    assert receipt["firms"] == 500
    assert receipt["sha256"] == hashlib.sha256(output.read_bytes()).hexdigest()
    assert len(output.read_text(encoding="utf-8").splitlines()) == 6_001
    assert json.loads(receipt_path.read_text())["status"] == "PASS"


def test_harness_enforces_cmg_and_failure_preservation() -> None:
    stata = (ROOT / "stata_run.do").read_text(encoding="utf-8")
    wrapper = (ROOT / "run_pair.sh").read_text(encoding="utf-8")
    matlab = (ROOT / "matlab_run.m").read_text(encoding="utf-8")
    assert "preconditioner(cmg)" in stata
    assert '"`e(preconditioner_selected)' in stata and '=="CMG"' in stata
    assert 'for role in "$first_role" "$second_role"' in wrapper
    assert "RIGHT_CENSORED" in wrapper
    assert "MATLAB_FIT_NONCONVERGENCE" in wrapper
    assert 'maxiter(`maxiter\')' in stata
    assert "--expected-pool-workers" in wrapper
    assert "fit_flag==0 && fit_relres<=1e-10" in matlab
    assert "lincom_KSS" in matlab
    assert "exist('corr','file')~=2" in matlab
    assert "e(rust_selected_route)==3" in stata
    assert "e(rust_solver_fallback)==0" in stata


def test_missing_generic_rust_cmg_structure_is_explicit() -> None:
    assert optional_int("") is None
    assert optional_int("3") == 3


def test_matlab_fit_nonconvergence_censor_is_exactly_identified(tmp_path: Path) -> None:
    expected = {
        "source_commit": "a" * 40,
        "input_sha256": "b" * 64,
        "rows": "480000",
        "cores": "4",
    }
    failure = {
        "schema": "FEVC-PROJECTION-AKM-MATLAB-FAILURE-V1",
        "status": "FAIL",
        "source_commit": expected["source_commit"],
        "input_sha256": expected["input_sha256"],
        "rows": 480_000,
        "active_cores": 4,
        "identifier": "fevc:projectionAkm:Fit",
        "message": "Grounded fit did not converge.",
    }
    (tmp_path / "failure.json").write_text(json.dumps(failure))
    validate_matlab_nonconvergence(tmp_path, expected)
    failure["message"] = "different failure"
    (tmp_path / "failure.json").write_text(json.dumps(failure))
    with pytest.raises(ValueError, match="censor identity"):
        validate_matlab_nonconvergence(tmp_path, expected)


def test_full_array_qacct_is_split_by_task_identity(tmp_path: Path) -> None:
    receipt = tmp_path / "qacct.txt"
    receipt.write_text(
        "==============================================================\n"
        "qname all.q\njobnumber 7370843\ntaskid 1\nfailed 0\nexit_status 0\n"
        "==============================================================\n"
        "qname all.q\njobnumber 7370843\ntaskid 2\nfailed 0\nexit_status 0\n"
    )
    records = parse_qacct(receipt, 2, 7_370_843)
    assert records[1]["exit_status"] == "0"
    assert records[2]["failed"] == "0"


def test_stage_accepts_reason_validated_matlab_censoring() -> None:
    assert stage_is_accepted(["RIGHT_CENSORED", "RIGHT_CENSORED"])
    assert stage_is_accepted(["PASS", "RIGHT_CENSORED"])
    assert not stage_is_accepted(["PASS", "FAIL"])


def test_aggregate_medians_require_three_passes() -> None:
    spec = importlib.util.spec_from_file_location("vpa_aggregate", ROOT / "aggregate.py")
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    cells = []
    for replicate in (1, 2):
        for role in ("fevc", "matlab"):
            cell = module.base_cell(480_000, 4, replicate, role, "a" * 40)
            cell.update({"status": "PASS", "command_seconds": replicate,
                         "whole_wall_seconds": replicate,
                         "whole_peak_rss_kib": 100 + replicate,
                         "incremental_rss_kib": 10 + replicate})
            cells.append(cell)
    summary = [value for value in module.summaries(cells)
               if value["rows"] == 480_000 and value["cores"] == 4]
    assert all(value["rankable"] is False for value in summary)
