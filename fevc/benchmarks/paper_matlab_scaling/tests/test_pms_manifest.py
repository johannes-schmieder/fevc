from __future__ import annotations

import csv

import pytest

from fevc.benchmarks.paper_matlab_scaling.build_manifest import (
    build_rows,
    main,
)
from fevc.benchmarks.paper_matlab_scaling.common import (
    ORDERS,
    ROW_GRID,
    SEEDS,
    STRUCTURES,
    TASK_FIELDS,
    EvidenceError,
    read_task,
)

COMMIT = "a" * 40
BUNDLE = "b" * 64


def test_matrix_has_registered_structures_sizes_seeds_and_orders() -> None:
    rows = build_rows(COMMIT, BUNDLE)
    assert len(rows) == 120
    assert {(row["structure"], row["rows"]) for row in rows} == {
        (structure, row_count)
        for structure in STRUCTURES
        for row_count in ROW_GRID
    }
    for structure in STRUCTURES:
        for row_count in ROW_GRID:
            cell = [row for row in rows
                    if row["structure"] == structure and row["rows"] == row_count]
            assert len(cell) == 6
            assert {row["seed"] for row in cell} == set(SEEDS)
            assert {row["order"] for row in cell} == set(ORDERS)


def test_common_row_grid_holds_aspect_ratio_and_changes_density() -> None:
    rows = build_rows(COMMIT, BUNDLE)
    for row in rows:
        assert row["rows"] == row["workers"] * row["cells_per_worker"]
        assert row["workers"] == 40 * row["firms"]
    baseline = [row for row in rows if row["rows"] == ROW_GRID[-1] and
                row["seed"] == SEEDS[0] and row["order"] == ORDERS[0]]
    assert {row["structure"]: row["firms"] for row in baseline} == {
        "strong_d2": 24576,
        "strong_d3": 16384,
        "strong_d6": 8192,
        "weak_d3": 16384,
    }


def test_main_materializes_valid_one_row_tasks(tmp_path, monkeypatch) -> None:
    manifest = tmp_path / "tasks.tsv"
    task_dir = tmp_path / "tasks"
    task_dir.mkdir()
    monkeypatch.setattr(
        "sys.argv",
        ["build_manifest.py", "--output", str(manifest), "--task-dir",
         str(task_dir), "--source-commit", COMMIT, "--bundle", BUNDLE],
    )
    assert main() == 0
    tasks = sorted(task_dir.glob("*.tsv"))
    assert len(tasks) == 120
    for path in tasks:
        assert read_task(path)["experiment_id"] == path.stem
    with manifest.open(newline="", encoding="utf-8") as handle:
        reader = csv.DictReader(handle, delimiter="\t")
        assert tuple(reader.fieldnames or ()) == TASK_FIELDS
        assert len(list(reader)) == 120


def test_task_tampering_fails_closed(tmp_path) -> None:
    path = tmp_path / "task.tsv"
    row = build_rows(COMMIT, BUNDLE)[0]
    row["cells_per_worker"] = 3
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=TASK_FIELDS, delimiter="\t",
                                lineterminator="\n")
        writer.writeheader()
        writer.writerow(row)
    with pytest.raises(EvidenceError, match="degree changed"):
        read_task(path)
