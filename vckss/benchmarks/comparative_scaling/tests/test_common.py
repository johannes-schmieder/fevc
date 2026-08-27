from __future__ import annotations

import os
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from common import parse_gnu_time, parse_matlab_pcg  # noqa: E402
from cpu_subset import subset  # noqa: E402


def test_cpu_subset_is_deterministic(monkeypatch) -> None:
    monkeypatch.setattr(os, "sched_getaffinity", lambda _: {9, 3, 7, 1, 5},
                        raising=False)
    assert subset(4) == (1, 3, 5, 7)


def test_parse_gnu_time(tmp_path: Path) -> None:
    receipt = tmp_path / "resources.txt"
    receipt.write_text(
        "User time (seconds): 12.5\n"
        "System time (seconds): 1.5\n"
        "Elapsed (wall clock) time (h:mm:ss or m:ss): 1:02.25\n"
        "Maximum resident set size (kbytes): 2048\n",
        encoding="utf-8",
    )
    value = parse_gnu_time(receipt)
    assert value["wall_seconds"] == 62.25
    assert value["maximum_rss_bytes"] == 2 * 1024**2


def test_parse_matlab_pcg_convergence(tmp_path: Path) -> None:
    log = tmp_path / "matlab.application.txt"
    log.write_text(
        "pcg converged at iteration 23 to a solution with relative "
        "residual 9e-11.\n", encoding="utf-8")
    assert parse_matlab_pcg(log) == {
        "converged": True,
        "termination_iteration": 23,
        "returned_iteration": 23,
        "relative_residual": 9e-11,
    }


def test_parse_matlab_pcg_nonconvergence_is_preserved(tmp_path: Path) -> None:
    log = tmp_path / "matlab.application.txt"
    log.write_text(
        "pcg stopped at iteration 1000 without converging to the desired "
        "tolerance because the maximum number of iterations was reached.\n"
        "The iterate returned (number 998) has relative residual 2e-7.\n",
        encoding="utf-8")
    value = parse_matlab_pcg(log)
    assert value == {
        "converged": False,
        "termination_iteration": 1000,
        "returned_iteration": 998,
        "relative_residual": 2e-7,
    }
