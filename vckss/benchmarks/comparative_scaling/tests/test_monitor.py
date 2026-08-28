from __future__ import annotations

import json
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[4]
BENCHMARK = ROOT / "vckss" / "benchmarks" / "comparative_scaling"
sys.path.insert(0, str(BENCHMARK))

import monitor_process_tree as monitor_module  # noqa: E402
from monitor_process_tree import descendant_pids, load_identity  # noqa: E402


def test_descendant_selection_sums_only_the_owned_tree() -> None:
    snapshot = {
        100: (1, 10),
        101: (100, 20),
        102: (101, 30),
        200: (1, 40),
    }
    assert descendant_pids(snapshot, 100) == {100, 101, 102}
    assert descendant_pids(snapshot, 999) == set()


@pytest.mark.parametrize("workers", [1, 2, 4, 8, 16])
def test_matlab_identity_accepts_registered_worker_counts(
    tmp_path: Path, workers: int
) -> None:
    path = tmp_path / "identity.json"
    path.write_text(json.dumps({
        "status": "PASS",
        "expected_pool_workers": workers,
        "client_pid": 100,
        "worker_pids": list(range(101, 101 + workers)),
    }), encoding="utf-8")
    identity = load_identity(path, workers)
    assert identity["client_pid"] == 100
    assert len(identity["worker_pids"]) == workers


def test_matlab_identity_rejects_wrong_worker_count(tmp_path: Path) -> None:
    path = tmp_path / "identity.json"
    path.write_text(json.dumps({
        "status": "PASS",
        "expected_pool_workers": 4,
        "client_pid": 100,
        "worker_pids": [101, 102, 103, 104],
    }), encoding="utf-8")
    with pytest.raises(ValueError, match="worker count"):
        load_identity(path, 8)


def test_monitor_rescans_end_marker_when_root_exits(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch,
) -> None:
    phase_start = tmp_path / "phase.start"
    phase_end = tmp_path / "phase.end"
    output = tmp_path / "process_tree.json"
    phase_start.write_text("START\n", encoding="utf-8")
    snapshots = iter(({100: (1, 2048)}, {}))

    monkeypatch.setattr(
        monitor_module, "process_snapshot", lambda _root: next(snapshots)
    )

    def finish_during_last_interval(_seconds: float) -> None:
        phase_end.write_text("END\n", encoding="utf-8")

    monkeypatch.setattr(monitor_module.time, "sleep", finish_during_last_interval)

    rc = monitor_module.monitor(
        100, output, interval=0.05, phase_start=phase_start,
        phase_end=phase_end, proc_root=tmp_path / "proc",
    )
    receipt = json.loads(output.read_text(encoding="utf-8"))
    assert rc == 0
    assert receipt["status"] == "PASS"
    assert receipt["phase_start_observed"] is True
    assert receipt["phase_end_observed"] is True
    assert receipt["phase_peak_rss_kib"] == 2048
