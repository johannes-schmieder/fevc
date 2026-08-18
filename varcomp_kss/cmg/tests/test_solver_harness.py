from __future__ import annotations

import csv
import shutil
import subprocess
import tempfile
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[3]
DRIVER = ROOT / "varcomp_kss" / "cmg" / "benchmarks" / "solver_benchmark.do"
MAC_STATA = Path("/Applications/Stata/StataMP.app/Contents/MacOS/stata-mp")


def _stata() -> Path | None:
    discovered = shutil.which("stata-mp")
    if discovered:
        return Path(discovered)
    return MAC_STATA if MAC_STATA.is_file() else None


def _run_path(vertices: int, columns: int, maxiter: int) -> dict[str, str]:
    stata = _stata()
    if stata is None:
        pytest.skip("Stata/MP is unavailable")
    with tempfile.TemporaryDirectory(prefix="cmg-solver-harness-") as directory:
        output = Path(directory) / "solver.csv"
        completed = subprocess.run(
            [
                str(stata),
                "-q",
                "do",
                str(DRIVER),
                str(ROOT),
                "path",
                str(vertices),
                str(columns),
                "1e-8",
                str(maxiter),
                str(output),
            ],
            cwd=directory,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            check=False,
        )
        assert completed.returncode == 0, completed.stdout
        assert "CMG SOLVER BENCHMARK PASS" in completed.stdout
        with output.open(newline="", encoding="utf-8-sig") as handle:
            return next(csv.DictReader(handle))


def test_forced_cmg_pcg_rescues_a_path_and_rechecks_residual() -> None:
    row = _run_path(vertices=300, columns=4, maxiter=1000)

    assert row["diagonal_status"] == "CONVERGED"
    assert row["cmg_status"] == "CONVERGED"
    assert float(row["diagonal_max_relres"]) <= 1e-8
    assert float(row["cmg_max_relres"]) <= 1e-8
    diagonal_iterations = float(row["diagonal_max_iterations"])
    cmg_iterations = float(row["cmg_max_iterations"])
    assert cmg_iterations <= 250
    assert diagonal_iterations >= 4 * cmg_iterations


def test_capped_diagonal_is_not_reported_as_success_but_cmg_can_rescue() -> None:
    row = _run_path(vertices=1000, columns=4, maxiter=100)

    assert row["diagonal_status"] == "MAXITER"
    assert float(row["diagonal_max_relres"]) > 1e-8
    assert row["cmg_status"] == "CONVERGED"
    assert float(row["cmg_max_relres"]) <= 1e-8
    assert float(row["cmg_max_iterations"]) <= 100
