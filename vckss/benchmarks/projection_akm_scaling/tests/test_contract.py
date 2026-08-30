from __future__ import annotations

import hashlib
import importlib.util
import json
import sys
from pathlib import Path


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


def test_registered_feasibility_grid_and_rotation() -> None:
    dimensions = {
        480_000: (80_000, 40_000, 1_888, 10_800, 64),
        1_920_000: (320_000, 160_000, 2_088, 36_000, 128),
        7_680_000: (1_280_000, 640_000, 2_288, 43_200, 256),
    }
    for rows in ROWS:
        tasks = feasibility_tasks(rows)
        assert [value.cores for value in tasks] == list(CORES)
        expected = dimensions[rows]
        assert (tasks[0].workers, tasks[0].firms, tasks[0].probes,
                tasks[0].role_cap_seconds, tasks[0].memory_gib) == expected
        assert {value.order for value in tasks} == {"rust_matlab", "matlab_rust"}


def test_gate_and_topup_manifests_round_trip(tmp_path: Path) -> None:
    gate_path = tmp_path / "gate.tsv"
    write_manifest(gate_path, [gate_task()])
    assert read_manifest(gate_path)[0]["replicate"] == "0"

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
    assert "--expected-pool-workers" in wrapper
    assert "fit_flag==0 && fit_relres<=1e-10" in matlab
    assert "lincom_KSS" in matlab
    assert "exist('corr','file')~=2" in matlab


def test_aggregate_medians_require_three_passes() -> None:
    spec = importlib.util.spec_from_file_location("vpa_aggregate", ROOT / "aggregate.py")
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    cells = []
    for replicate in (1, 2):
        for role in ("vckss", "matlab"):
            cell = module.base_cell(480_000, 4, replicate, role, "a" * 40)
            cell.update({"status": "PASS", "command_seconds": replicate,
                         "whole_wall_seconds": replicate,
                         "whole_peak_rss_kib": 100 + replicate,
                         "incremental_rss_kib": 10 + replicate})
            cells.append(cell)
    summary = [value for value in module.summaries(cells)
               if value["rows"] == 480_000 and value["cores"] == 4]
    assert all(value["rankable"] is False for value in summary)
