from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

import pytest

from aggregate import (  # noqa: E402
    cell_rows,
    deterministic_input_hashes,
    overlap_diagnostics,
)
from common import EvidenceError  # noqa: E402
from common import CORE_GRID, ESTIMATORS, REPLICATES, ROW_GRID, STRUCTURES  # noqa: E402


def complete_results() -> list[dict[str, object]]:
    values: list[dict[str, object]] = []
    for structure, (_, degree, _) in STRUCTURES.items():
        for rows in ROW_GRID:
            for cores in CORE_GRID:
                for replicate, _, _ in REPLICATES:
                    for role_index, role in enumerate(ESTIMATORS, start=1):
                        values.append({
                            "structure": structure,
                            "rows": rows,
                            "active_cores": cores,
                            "stata_processors": min(cores, 4),
                            "effective_role_cores": (
                                min(cores, 4) if role == "mata" else cores
                            ),
                            "mata_active_cores": min(cores, 4),
                            "rust_threads": cores,
                            "matlab_workers": cores,
                            "replicate": replicate,
                            "hostname": f"host-{replicate}",
                            "cpu_model": "scheduler-assigned test CPU",
                            "role": role,
                            "scientific_status": "PASS",
                            "rust_mata_gate": "PASS",
                            "command_seconds": float(role_index * rows / cores),
                            "estimator_phase_peak_rss_bytes": float(role_index * rows),
                            "whole_process_peak_rss_bytes": float(role_index * rows + 1),
                            "cells_per_worker": degree,
                        })
    return values


def test_complete_cell_summary_cardinality_and_ratios() -> None:
    rows = cell_rows(complete_results())
    assert len(rows) == 300
    assert all(row["cell_status"] == "COMPLETE" for row in rows)
    assert all(row["successful_repetitions"] == 3 for row in rows)
    rust = next(row for row in rows if row["role"] == "rust")
    assert rust["rust_to_matlab_time_ratio"] == 2 / 3
    assert rust["rust_to_mata_time_ratio"] == 2


def test_one_failure_makes_only_its_cell_incomplete() -> None:
    values = complete_results()
    values[0]["scientific_status"] = "TIMEOUT"
    rows = cell_rows(values)
    assert sum(row["cell_status"] == "INCOMPLETE" for row in rows) == 3
    assert sum(row["cell_status"] == "COMPLETE" for row in rows) == 297


def validation_payloads() -> list[dict[str, object]]:
    values = []
    for structure in STRUCTURES:
        for rows in ROW_GRID:
            input_hash = f"{len(values) % 16:x}" * 64
            for cores in CORE_GRID:
                for replicate, _, _ in REPLICATES:
                    values.append({
                        "task": {"structure": structure, "rows": rows,
                                 "active_cores": cores, "replicate": replicate},
                        "input_sha256": input_hash,
                    })
    return values


def test_deterministic_input_hash_inventory() -> None:
    rows = deterministic_input_hashes(validation_payloads())
    assert len(rows) == 20
    assert rows[0]["structure"] == "strong_d2"
    assert rows[-1]["rows"] == max(ROW_GRID)


def test_deterministic_input_hash_mismatch_is_rejected() -> None:
    values = validation_payloads()
    values[1]["input_sha256"] = "f" * 64
    with pytest.raises(EvidenceError, match="deterministic input hash"):
        deterministic_input_hashes(values)


def test_own_array_overlap_is_host_and_interval_specific() -> None:
    generation = {"source_commit": "a" * 40, "bundle_sha256": "b" * 64}
    values = [
        {"task": {"task_id": "1", **generation},
         "node": {"hostname": "a.example", "task_start_epoch": 0,
                  "task_end_epoch": 10}},
        {"task": {"task_id": "2", **generation},
         "node": {"hostname": "a", "task_start_epoch": 4,
                  "task_end_epoch": 8}},
        {"task": {"task_id": "3", **generation},
         "node": {"hostname": "b", "task_start_epoch": 3,
                  "task_end_epoch": 9}},
        {"task": {"task_id": "4", "source_commit": "c" * 40,
                  "bundle_sha256": "d" * 64},
         "node": {"hostname": "a", "task_start_epoch": 2,
                  "task_end_epoch": 7}},
    ]
    diagnostics = overlap_diagnostics(values)
    assert diagnostics[1]["overlapping_own_tasks"] == 1
    assert diagnostics[1]["own_array_overlap_seconds"] == 4
    assert diagnostics[1]["maximum_own_array_concurrency"] == 2
    assert diagnostics[3]["overlapping_own_tasks"] == 0
    assert diagnostics[4]["overlapping_own_tasks"] == 0
