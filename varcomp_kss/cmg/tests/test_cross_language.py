from __future__ import annotations

import csv
import shutil
import subprocess
import tempfile
from pathlib import Path

import numpy as np
import pytest

from varcomp_kss.cmg.oracle.cmg_oracle import (
    HierarchyOptions,
    build_hierarchy,
    build_hybrid_graph,
    collapse_cells,
    hybrid_firm_schur,
    kss_pullback,
)

ROOT = Path(__file__).resolve().parents[3]
DO_FILE = ROOT / "varcomp_kss" / "cmg" / "tests" / "stata" / "export_oracle_fixture.do"
GENERIC_DO_FILE = (
    ROOT / "varcomp_kss" / "cmg" / "tests" / "stata" / "export_oracle_case.do"
)
MAC_STATA = Path("/Applications/Stata/StataMP.app/Contents/MacOS/stata-mp")


def _stata() -> Path | None:
    discovered = shutil.which("stata-mp")
    if discovered:
        return Path(discovered)
    return MAC_STATA if MAC_STATA.is_file() else None


def _oracle_cycle() -> np.ndarray:
    worker: list[int] = []
    firm: list[int] = []
    for edge in range(7):
        worker.extend([edge, edge])
        firm.extend([edge, edge + 1])
    cells = collapse_cells(worker, firm, [1.0] * len(worker))
    hierarchy = build_hierarchy(
        build_hybrid_graph(cells), HierarchyOptions(coarse_max=2)
    )
    return kss_pullback(hierarchy, 8)


def test_mata_cycle_matches_independent_python_oracle() -> None:
    stata = _stata()
    if stata is None:
        pytest.skip("Stata/MP is unavailable")
    with tempfile.TemporaryDirectory(prefix="cmg-cross-language-") as directory:
        temporary = Path(directory)
        output = temporary / "cycle.csv"
        command = [str(stata), "-q", "do", str(DO_FILE), str(ROOT), str(output)]
        completed = subprocess.run(
            command,
            cwd=temporary,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            check=False,
        )
        assert completed.returncode == 0, completed.stdout
        assert "CMG ORACLE FIXTURE EXPORT PASS" in completed.stdout
        with output.open(newline="", encoding="utf-8-sig") as handle:
            rows = list(csv.DictReader(handle))
        actual = np.asarray(
            [[float(row[f"cmg_fixture{column}"]) for column in range(1, 9)] for row in rows]
        )
    np.testing.assert_allclose(actual, _oracle_cycle(), rtol=1e-11, atol=1e-11)


def _weighted_ring(vertices: int) -> tuple[list[int], list[int], list[float]]:
    worker: list[int] = []
    firm: list[int] = []
    weight: list[float] = []
    for edge in range(vertices):
        worker.extend([edge + 1, edge + 1])
        firm.extend([edge + 1, (edge + 1) % vertices + 1])
        weight.extend([float(1 + edge % 3), float(2 + edge % 5)])
    return worker, firm, weight


def _barbell() -> tuple[list[int], list[int], list[float]]:
    worker: list[int] = []
    firm: list[int] = []
    weight: list[float] = []
    edge_worker = 0
    for group in ((1, 2, 3, 4), (5, 6, 7, 8)):
        for left in range(len(group)):
            for right in range(left + 1, len(group)):
                edge_worker += 1
                worker.extend([edge_worker, edge_worker])
                firm.extend([group[left], group[right]])
                weight.extend([1.0, 1.0])
    edge_worker += 1
    worker.extend([edge_worker, edge_worker])
    firm.extend([4, 5])
    weight.extend([1e-4, 1e-4])
    return worker, firm, weight


def _disconnected_paths() -> tuple[list[int], list[int], list[float]]:
    return (
        [1, 1, 2, 2, 3, 3, 4, 4],
        [1, 2, 2, 3, 4, 5, 5, 6],
        [1.0, 2.0, 3.0, 1.0, 2.0, 5.0, 7.0, 1.0],
    )


ORACLE_CASES = {
    "degree3_degree4": (
        [1, 1, 1, 2, 2, 2, 2, 3, 3],
        [1, 2, 3, 1, 2, 3, 4, 4, 5],
        [1.0, 2.0, 5.0, 3.0, 1.0, 4.0, 2.0, 1.0, 7.0],
    ),
    "weighted_ring": _weighted_ring(8),
    "barbell": _barbell(),
    "disconnected": _disconnected_paths(),
    "duplicates_dynamic_range": (
        [1, 1, 1, 2, 2, 2, 2, 3, 3, 4, 4],
        [1, 1, 2, 2, 3, 3, 4, 1, 4, 3, 5],
        [1e-8, 2e-8, 1e8, 3.0, 1e-8, 2e-8, 4.0, 5.0, 2.0, 7.0, 1.0],
    ),
}


@pytest.mark.parametrize("case_name", ORACLE_CASES, ids=ORACLE_CASES)
def test_mata_matches_oracle_on_adversarial_cases(case_name: str) -> None:
    stata = _stata()
    if stata is None:
        pytest.skip("Stata/MP is unavailable")
    worker, firm, weight = ORACLE_CASES[case_name]
    cells = collapse_cells(worker, firm, weight)
    graph = build_hybrid_graph(cells)
    hierarchy = build_hierarchy(graph, HierarchyOptions(coarse_max=2))
    expected_cycle = kss_pullback(hierarchy, cells.n_firm)

    with tempfile.TemporaryDirectory(prefix=f"cmg-{case_name}-") as directory:
        temporary = Path(directory)
        input_csv = temporary / "cells.csv"
        output_csv = temporary / "matrices.csv"
        with input_csv.open("w", newline="", encoding="utf-8") as handle:
            writer = csv.writer(handle)
            writer.writerow(("worker", "firm", "weight"))
            writer.writerows(zip(worker, firm, weight, strict=True))
        command = [
            str(stata),
            "-q",
            "do",
            str(GENERIC_DO_FILE),
            str(ROOT),
            str(input_csv),
            str(output_csv),
            "2",
        ]
        completed = subprocess.run(
            command,
            cwd=temporary,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            check=False,
        )
        assert completed.returncode == 0, completed.stdout
        assert "CMG GENERIC ORACLE CASE EXPORT PASS" in completed.stdout
        with output_csv.open(newline="", encoding="utf-8-sig") as handle:
            rows = list(csv.DictReader(handle))
    n_firm = cells.n_firm
    actual_cycle = np.asarray(
        [[float(row[f"cycle{column}"]) for column in range(1, n_firm + 1)] for row in rows]
    )
    actual_schur = np.asarray(
        [[float(row[f"schur{column}"]) for column in range(1, n_firm + 1)] for row in rows]
    )
    np.testing.assert_allclose(
        actual_schur, hybrid_firm_schur(graph), rtol=2e-12, atol=2e-12
    )
    # These two fixtures deliberately make the terminal grounded solve ill
    # conditioned.  Mata uses diagonal equilibration while the independent
    # NumPy oracle solves the unscaled block, so compare their preconditioner
    # maps at a conditioning-aware tolerance.  The Schur identity above keeps
    # its strict algebraic tolerance, and package solutions still require the
    # unchanged original-system residual gate.
    cycle_tolerance = {
        "barbell": 5e-8,
        "duplicates_dynamic_range": 1e-8,
    }.get(case_name, 2e-10)
    np.testing.assert_allclose(
        actual_cycle,
        expected_cycle,
        rtol=cycle_tolerance,
        atol=cycle_tolerance,
    )
