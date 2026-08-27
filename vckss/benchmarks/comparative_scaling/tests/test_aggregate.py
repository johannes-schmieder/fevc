from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from aggregate import cell_rows  # noqa: E402
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
                            "replicate": replicate,
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
