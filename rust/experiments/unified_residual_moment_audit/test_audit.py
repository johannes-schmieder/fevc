"""Independent arithmetic and failure-accounting regressions for the auditor."""
import copy
import importlib.util
import json
import os
from pathlib import Path

import pytest

SPEC = importlib.util.spec_from_file_location("independent_unified_audit", Path(__file__).with_name("audit.py"))
A = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(A)


def call():
    q = [None] * 20
    for i, value in {2: 4, 5: 0, 6: 9, 9: 6, 10: -1, 11: 3, 16: 0, 17: 1}.items():
        q[i] = value
    return {"status": "success", "point": [1] * 4, "targets": [4, 0] * 4, "q1": q * 4}


def test_q0_arithmetic():
    row = A.target_record(call(), 0, 0, False)
    assert row["error"] == 1 and row["available"] and row["covered"]
    assert row["se"] == 2
    assert row["width"] == pytest.approx(4 * 1.959963984540054)


def test_q1_uses_native_endpoints():
    assert A.target_record(call(), 3, 0, True)["covered"]
    assert not A.target_record(call(), 3.1, 0, True)["covered"]
    assert A.target_record(call(), 0, 0, True)["width"] == 4


def test_shared_failure_counted_not_dropped():
    good = A.target_record(call(), 0, 0, False)
    bad = A.target_record({"status": "shared_failure", "detail": "moment solve"}, 0, 0, False)
    result = A.summarize([good, bad])
    assert result["attempts"] == 2 and result["point_estimates"] == 1
    assert result["coverage"] == 1 and result["coverage_among_all_attempts"] == .5
    assert result["success_rate"] == .5 and result["se_denominator"] == 1


def test_target_failure_keeps_point_for_sd():
    bad = call()
    bad["point"][0] = 3
    bad["targets"][1] = 4
    result = A.summarize([A.target_record(call(), 0, 0, False), A.target_record(bad, 0, 0, False)])
    assert result["bias"] == 2 and result["empirical_sd"] == pytest.approx(2 ** .5)
    assert result["mean_se"] == 2 and result["se_denominator"] == 1


@pytest.mark.parametrize("index,value", [(2, 0), (6, 0), (9, 0), (17, .5), (5, 6), (10, 4)])
def test_invalid_q1_rejected(index, value):
    bad = copy.deepcopy(call())
    bad["q1"][index] = value
    with pytest.raises(ValueError):
        A.target_record(bad, 0, 0, True)


def test_unavailable_q1_must_withhold_endpoints():
    bad = call()
    bad["q1"][16] = 3
    with pytest.raises(ValueError, match="endpoints"):
        A.target_record(bad, 0, 0, True)
    bad["q1"][10:12] = [None, None]
    assert not A.target_record(bad, 0, 0, True)["available"]


def test_all_failures_have_explicit_denominators():
    result = A.summarize([A.target_record({"status": "shared_failure", "detail": "fit"}, 0, 0, False)])
    assert result["attempts"] == 1 and result["successes"] == 0
    assert result["coverage"] is None and result["coverage_among_all_attempts"] == 0
    assert result["se_denominator"] == 0 and result["empirical_sd"] is None


@pytest.fixture
def completed_run(tmp_path):
    source = os.environ.get("FEVC_UNIFIED_AUDIT_RUN")
    if not source:
        pytest.skip("set FEVC_UNIFIED_AUDIT_RUN for completed-run corruption checks")
    source = Path(source).resolve()
    for name in ("manifest.json", "source.tar.gz", "tasks"):
        (tmp_path / name).symlink_to(source / name)
    result = json.loads((source / "result.json").read_text())
    return tmp_path, result


@pytest.mark.parametrize("mutation,message", [
    ("duplicate", "summary inventory"),
    ("missing", "summary inventory"),
    ("coverage", "summary mismatch"),
    ("counts", "complete call count"),
    ("decision", "overall decision mismatch"),
])
def test_completed_run_corruption_rejected(completed_run, mutation, message):
    folder, result = completed_run
    if mutation == "duplicate":
        result["summaries"].append(result["summaries"][0])
    elif mutation == "missing":
        result["summaries"].pop()
    elif mutation == "coverage":
        result["summaries"][0]["coverage"] = -1
    elif mutation == "counts":
        result["native_calls"] -= 1
    else:
        result["status"] = "FORGED_PASS"
    (folder / "result.json").write_text(json.dumps(result))
    with pytest.raises(ValueError, match=message):
        A.audit(folder)
