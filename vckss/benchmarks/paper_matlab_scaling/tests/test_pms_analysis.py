from __future__ import annotations

import json
from pathlib import Path

import pytest

from vckss.benchmarks.paper_matlab_scaling.analyze import analyze
from vckss.benchmarks.paper_matlab_scaling.build_manifest import build_rows
from vckss.benchmarks.paper_matlab_scaling.common import (
    TASK_FIELDS,
    EvidenceError,
)

SOURCE = "a" * 40
BUNDLE = "b" * 64


def write_manifest(path: Path, rows: list[dict[str, object]]) -> None:
    import csv

    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=TASK_FIELDS, delimiter="\t",
                                lineterminator="\n")
        writer.writeheader()
        writer.writerows(rows)


def write_evidence(root: Path, rows: list[dict[str, object]]) -> None:
    for index, row in enumerate(rows):
        job = root / str(row["experiment_id"])
        job.mkdir()
        stata_seconds = float(row["rows"]) / 1000 + index % 3
        matlab_seconds = float(row["rows"]) / 500 + index % 2
        value = {
            "schema": "PAPER-MATLAB-SCALING-VALIDATION-V1",
            "status": "PASS_NUMERICAL_AND_TIMING",
            "experiment_id": row["experiment_id"],
            "structure": row["structure"],
            "connectivity": row["connectivity"],
            "cells_per_worker": int(row["cells_per_worker"]),
            "rows": int(row["rows"]),
            "workers": int(row["workers"]),
            "firms": int(row["firms"]),
            "probes": int(row["probes"]),
            "seed": int(row["seed"]),
            "order": row["order"],
            "source_commit": SOURCE,
            "bundle_sha256": BUNDLE,
            "task_sha256": f"{index:064x}",
            "input_sha256": f"{row['structure']}-{row['rows']}",
            "hostname": "node.example",
            "job_id": str(index + 1),
            "stata_command_seconds": stata_seconds,
            "matlab_command_seconds": matlab_seconds,
            "stata_over_matlab_command_ratio": stata_seconds / matlab_seconds,
            "matlab_numerical_result_accepted": True,
            "qacct_wall_seconds": stata_seconds + matlab_seconds,
            "qacct_maxvmem_bytes": 100,
            "stata_gnu_max_rss_bytes": 50,
            "matlab_process_tree_peak_rss_bytes": 75,
        }
        (job / "validation.json").write_text(json.dumps(value), encoding="utf-8")


def test_complete_matrix_summarizes_twenty_cells(tmp_path: Path) -> None:
    rows = build_rows(SOURCE, BUNDLE)
    manifest = tmp_path / "manifest.tsv"
    evidence = tmp_path / "evidence"
    evidence.mkdir()
    write_manifest(manifest, rows)
    write_evidence(evidence, rows)
    jobs, cells, summary = analyze(evidence, manifest)
    assert len(jobs) == 120
    assert len(cells) == 20
    assert summary["job_count"] == 120
    assert summary["cell_count"] == 20
    assert summary["status"] == "PASS_NUMERICAL_AND_TIMING"
    assert all(cell["jobs"] == 6 for cell in cells)


def test_incomplete_matrix_is_rejected(tmp_path: Path) -> None:
    rows = build_rows(SOURCE, BUNDLE)
    manifest = tmp_path / "manifest.tsv"
    evidence = tmp_path / "evidence"
    evidence.mkdir()
    write_manifest(manifest, rows)
    write_evidence(evidence, rows[:-1])
    with pytest.raises(EvidenceError, match="incomplete"):
        analyze(evidence, manifest)
