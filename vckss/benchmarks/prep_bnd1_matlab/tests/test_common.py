from __future__ import annotations

import pytest
from common import EvidenceError, parse_matlab_pcg, qacct, same_host


def qacct_text(*, failed: str = "0", exit_status: str = "0", slots: str = "4") -> str:
    return f"""jobnumber 123
taskid undefined
project welfgr
granted_pe omp
slots {slots}
failed {failed}
exit_status {exit_status}
ru_wallclock 12
cpu 10.5
maxvmem 2.1G
hostname scc-test
"""


def test_qacct_accepts_complete_scalar_job(tmp_path) -> None:
    path = tmp_path / "qacct.txt"
    path.write_text(qacct_text(), encoding="utf-8")
    assert qacct(path)["jobnumber"] == "123"


@pytest.mark.parametrize(("left", "right"), [
    ("scc-ei3", "scc-ei3.bu.edu"),
    ("scc-ei3.bu.edu", "scc-ei3"),
    ("scc-ei3", "scc-ei3"),
])
def test_same_host_accepts_short_and_fully_qualified_names(left, right) -> None:
    assert same_host(left, right)


@pytest.mark.parametrize(("left", "right"), [
    ("", "scc-ei3.bu.edu"),
    ("scc-ei3", "scc-ei4.bu.edu"),
])
def test_same_host_rejects_missing_or_different_names(left, right) -> None:
    assert not same_host(left, right)


@pytest.mark.parametrize("kwargs", [
    {"failed": "100"}, {"exit_status": "1"}, {"slots": "8"},
])
def test_qacct_rejects_scheduler_or_resource_change(tmp_path, kwargs) -> None:
    path = tmp_path / "qacct.txt"
    path.write_text(qacct_text(**kwargs), encoding="utf-8")
    with pytest.raises(EvidenceError):
        qacct(path)


def test_matlab_pcg_convergence_is_parsed(tmp_path) -> None:
    path = tmp_path / "application.txt"
    path.write_text(
        "pcg converged at iteration 23 to a solution with relative residual 9e-11.\n",
        encoding="utf-8",
    )
    assert parse_matlab_pcg(path) == {
        "converged": True,
        "termination_iteration": 23,
        "returned_iteration": 23,
        "relative_residual": 9e-11,
    }


def test_matlab_pcg_nonconvergence_is_a_typed_result(tmp_path) -> None:
    path = tmp_path / "application.txt"
    path.write_text(
        "pcg stopped at iteration 1000 without converging to the desired tolerance "
        "1e-10\nThe iterate returned (number 998) has relative residual 3.8e-06.\n",
        encoding="utf-8",
    )
    result = parse_matlab_pcg(path)
    assert result["converged"] is False
    assert result["returned_iteration"] == 998
