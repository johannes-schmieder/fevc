from __future__ import annotations

import csv

import pytest
from build_manifest import build_rows, main
from common import TASK_FIELDS, EvidenceError, read_task

COMMIT = "a" * 40
BUNDLE = "b" * 64


def test_matrix_has_all_shapes_seeds_and_orders() -> None:
    rows = build_rows(COMMIT, BUNDLE)
    assert len(rows) == 30
    assert {(row["firms"], row["probes"]) for row in rows} == {
        (64, 20), (256, 20), (1024, 20), (256, 200), (1024, 200)
    }
    for cell in {(row["firms"], row["probes"]) for row in rows}:
        selected = [row for row in rows if (row["firms"], row["probes"]) == cell]
        assert {row["seed"] for row in selected} == {104729, 8675309, 20260819}
        assert {row["order"] for row in selected} == {
            "stata_matlab", "matlab_stata"
        }


def test_main_materializes_hashable_one_row_tasks(tmp_path, monkeypatch) -> None:
    manifest = tmp_path / "manifest.tsv"
    task_dir = tmp_path / "tasks"
    task_dir.mkdir()
    monkeypatch.setattr(
        "sys.argv",
        ["build_manifest.py", "--output", str(manifest), "--task-dir",
         str(task_dir), "--source-commit", COMMIT, "--bundle", BUNDLE],
    )
    assert main() == 0
    tasks = sorted(task_dir.glob("*.tsv"))
    assert len(tasks) == 30
    for path in tasks:
        task = read_task(path)
        assert task["experiment_id"] == path.stem
    with manifest.open(newline="", encoding="utf-8") as handle:
        reader = csv.DictReader(handle, delimiter="\t")
        assert tuple(reader.fieldnames or ()) == TASK_FIELDS
        assert len(list(reader)) == 30


def test_task_tampering_fails_closed(tmp_path) -> None:
    path = tmp_path / "task.tsv"
    row = build_rows(COMMIT, BUNDLE)[0]
    row["target_contract"] = "weighted"
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=TASK_FIELDS, delimiter="\t",
                                lineterminator="\n")
        writer.writeheader()
        writer.writerow(row)
    with pytest.raises(EvidenceError, match="comparison contract changed"):
        read_task(path)
