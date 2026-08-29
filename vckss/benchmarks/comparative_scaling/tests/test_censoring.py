from __future__ import annotations

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from aggregate import cell_rows, deterministic_input_hashes  # noqa: E402
from censoring import CENSORED_CELL, CENSORED_TASK_IDS  # noqa: E402
from common import CORE_GRID, ESTIMATORS, REPLICATES, ROW_GRID, STRUCTURES  # noqa: E402


def complete_results_without_censored_cell() -> list[dict[str, object]]:
    values: list[dict[str, object]] = []
    for structure, (_, degree, _) in STRUCTURES.items():
        for rows in ROW_GRID:
            for cores in CORE_GRID:
                if (structure, rows, cores) == CENSORED_CELL:
                    continue
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
                            "cpu_model": "test CPU",
                            "role": role,
                            "scientific_status": "PASS",
                            "rust_mata_gate": "PASS",
                            "command_seconds": float(role_index * rows / cores),
                            "estimator_phase_peak_rss_bytes": float(role_index * rows),
                            "whole_process_peak_rss_bytes": float(role_index * rows + 1),
                            "cells_per_worker": degree,
                        })
    return values


def test_registered_censor_retains_full_cell_grid_without_ranking() -> None:
    rows = cell_rows(
        complete_results_without_censored_cell(),
        censored_cells={CENSORED_CELL})
    assert len(rows) == 300
    censored = [row for row in rows
                if (row["structure"], row["rows"], row["active_cores"]) ==
                CENSORED_CELL]
    assert len(censored) == 3
    assert {row["role"] for row in censored} == set(ESTIMATORS)
    assert all(row["cell_status"] == "CENSORED" and
               row["successful_repetitions"] == 0 and
               row["fastest_role"] == "NONE" for row in censored)
    assert sum(row["cell_status"] == "COMPLETE" for row in rows) == 297


def test_input_hash_inventory_allows_only_the_registered_missing_cell() -> None:
    payloads = []
    for structure in STRUCTURES:
        for rows in ROW_GRID:
            input_hash = f"{len(payloads) % 16:x}" * 64
            for cores in CORE_GRID:
                if (structure, rows, cores) == CENSORED_CELL:
                    continue
                for replicate, _, _ in REPLICATES:
                    payloads.append({
                        "task": {"structure": structure, "rows": rows,
                                 "active_cores": cores, "replicate": replicate},
                        "input_sha256": input_hash,
                    })
    assert len(deterministic_input_hashes(
        payloads, censored_cells={CENSORED_CELL})) == 20


def test_future_execution_exclusion_is_machine_readable() -> None:
    policy = json.loads((ROOT / "future_exclusions.json").read_text())
    cell = policy["cells"][0]
    assert policy["status"] == "ACTIVE"
    assert cell["historical_task_ids"] == list(CENSORED_TASK_IDS)
    assert cell["future_execution"] == "DO_NOT_SUBMIT"
    assert cell["ranking_treatment"] == "EXCLUDE_ENTIRE_GRAPH_SIZE_CORE_CELL"
