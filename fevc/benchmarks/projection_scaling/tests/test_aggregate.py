from __future__ import annotations

import importlib.util
from pathlib import Path

import pytest


MODULE_PATH = Path(__file__).resolve().parents[1] / "aggregate.py"
SPEC = importlib.util.spec_from_file_location("projection_scaling_aggregate", MODULE_PATH)
assert SPEC is not None and SPEC.loader is not None
AGGREGATE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(AGGREGATE)


def test_registered_layout_and_order_rotation() -> None:
    assert AGGREGATE.expected_layout(1) == (6000, 1, 1256, "matlab_rust")
    assert AGGREGATE.expected_layout(2) == (6000, 2, 1256, "rust_matlab")
    assert AGGREGATE.expected_layout(7) == (96000, 1, 1656, "matlab_rust")
    assert AGGREGATE.expected_layout(8) == (96000, 2, 1656, "rust_matlab")
    assert AGGREGATE.expected_layout(9) == (96000, 3, 1656, "matlab_rust")


def test_terminal_solver_failures_are_typed() -> None:
    rust = AGGREGATE.failure_detail(
        "fevc",
        "model PCG did not converge in 20000 iterations; "
        "final reduced residual was 0.03474903664692074",
    )
    assert rust["failure_code"] == "PCG_MAXITER"
    assert rust["solver_iterations"] == 20000
    assert rust["reported_residual"] == pytest.approx(0.03474903664692074)
    assert rust["residual_kind"] == "reduced_model"

    matlab = AGGREGATE.failure_detail(
        "matlab",
        "The iterate returned (number 982) has relative residual 1.7e-07.\n"
        "Grounded fit did not converge.",
    )
    assert matlab["failure_code"] == "MATLAB_FIT_PCG_MAXITER"
    assert matlab["solver_iterations"] == 982
    assert matlab["solver_max_iterations"] == 1000
    assert matlab["reported_residual"] == pytest.approx(1.7e-7)
    assert matlab["residual_kind"] == "grounded_normal_equation"


def test_qacct_requires_all_nine_structural_records(tmp_path: Path) -> None:
    receipt = tmp_path / "array-qacct.txt"
    receipt.write_text(
        "\n".join(
            f"TASK={task}\njobnumber 7368481\ntaskid {task}\nfailed 0\nexit_status "
            f"{0 if task <= 6 else 1}"
            for task in range(1, 10)
        )
        + "\n",
        encoding="utf-8",
    )
    parsed = AGGREGATE.parse_qacct(receipt)
    assert parsed[1]["jobnumber"] == "7368481"
    assert parsed[1]["exit_status"] == "0"
    assert parsed[9]["exit_status"] == "1"

    receipt.write_text(receipt.read_text(encoding="utf-8").split("TASK=9")[0])
    with pytest.raises(ValueError, match="incomplete array qacct tasks"):
        AGGREGATE.parse_qacct(receipt)
