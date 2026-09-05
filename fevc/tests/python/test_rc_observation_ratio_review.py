from __future__ import annotations

import copy
import hashlib
import importlib.util
import json
import math
from pathlib import Path
import statistics

import pytest


ROOT = Path(__file__).resolve().parents[3]
SPEC = importlib.util.spec_from_file_location(
    "ratio_review", ROOT / "fevc/tools/review_rc_observation_ratio.py")
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)
ERRORS = [-1.2, -0.4, -0.1, 0.3, 0.8, 1.1, 2.5]
SES = [0.8, 1.1, 0.95, 1.5, 1.4, 1.1, 1.8]


def test_ratio_and_jackknife_against_independent_delete_and_recompute():
    value = MODULE.ratio_diagnostics(ERRORS, SES)
    expected = statistics.stdev(ERRORS) / statistics.mean(SES)
    loo = [statistics.stdev(ERRORS[:i] + ERRORS[i + 1:])
           / statistics.mean(SES[:i] + SES[i + 1:]) for i in range(len(ERRORS))]
    logs = [math.log(x) for x in loo]
    variance = (len(logs) - 1) * statistics.pvariance(logs)
    assert value["se_ratio"] == pytest.approx(expected, rel=1e-14)
    assert value["delete_one_ratio_range"] == pytest.approx([min(loo), max(loo)])
    assert value["jackknife_log_ratio_mcse"] ** 2 == pytest.approx(variance, rel=1e-13)
    assert value["sd_over_rms_se"] * value["rms_over_mean_se"] == pytest.approx(expected)
    assert value["mean_variance_over_empirical_variance"] == pytest.approx(
        statistics.mean(s * s for s in SES) / statistics.variance(ERRORS))


def test_delta_accounts_for_paired_covariance():
    value = MODULE.ratio_diagnostics(ERRORS, SES)
    mean = statistics.mean(ERRORS)
    v = statistics.pvariance(ERRORS)
    avg = statistics.mean(SES)
    a = [((x - mean) ** 2 - v) / (2 * v) for x in ERRORS]
    b = [(s - avg) / avg for s in SES]
    expected = (statistics.variance(a) + statistics.variance(b)
                - 2 * statistics.covariance(a, b)) / len(a)
    assert value["delta_log_ratio_mcse"] ** 2 == pytest.approx(expected, rel=1e-13)
    shuffled = MODULE.ratio_diagnostics(ERRORS, list(reversed(SES)))
    assert shuffled["se_ratio"] == pytest.approx(value["se_ratio"])
    assert shuffled["delta_log_ratio_mcse"] != pytest.approx(value["delta_log_ratio_mcse"])


def test_units_location_and_order_invariance():
    baseline = MODULE.ratio_diagnostics(ERRORS, SES)
    transformed = MODULE.ratio_diagnostics(
        [100 + 3 * x for x in reversed(ERRORS)], [3 * s for s in reversed(SES)])
    for key in ("se_ratio", "delta_ratio_mcse", "jackknife_ratio_mcse",
                "sd_over_rms_se", "rms_over_mean_se", "point_error_kurtosis"):
        assert transformed[key] == pytest.approx(baseline[key], rel=1e-12)


@pytest.mark.parametrize("errors,ses", [
    ([1, 2, 3], [1, 1, 1]), ([1, 2, 3, 4], [1, 1, 1]),
    ([1, 2, 3, math.nan], [1, 1, 1, 1]),
    ([1, 2, 3, 4], [1, 1, 1, math.inf]),
    ([1, 2, 3, 4], [1, 1, 1, 0]),
    ([1, 1, 1, 1], [1, 1, 1, 1]),
    ([1, 1, 1, 2], [1, 1, 1, 1]),
])
def test_invalid_or_unidentified_ratios_rejected(errors, ses):
    with pytest.raises(ValueError):
        MODULE.ratio_diagnostics(errors, ses)


def fixture(tmp_path):
    result = json.loads((ROOT / "fevc/docs/rc_observation_inference_v1_result.json").read_text())
    rows = []
    for summary in result["audit"]["summaries"]:
        summary.update(attempts=len(ERRORS), successes=len(ERRORS), coverage=1.0,
                       se_ratio=statistics.stdev(ERRORS) / statistics.mean(SES))
        for rep, (error, se) in enumerate(zip(ERRORS, SES)):
            rows.append({"cell": summary["cell"], "k": summary["k"],
                         "target": summary["target"], "replication": rep,
                         "status": "success", "point_error": error,
                         "estimated_sd": se, "covered": True})
    result["audit"]["row_count"] = len(rows)
    result["audit"]["attempt_status_counts"] = {"success": len(rows)}
    raw = tmp_path / "rows.jsonl"
    record = tmp_path / "result.json"
    write_fixture(raw, record, rows, result)
    return raw, record, rows, result


def write_fixture(raw, record, rows, result):
    raw.write_text("".join(json.dumps(row) + "\n" for row in rows))
    result["audit"]["hashes"]["output/aggregate/aggregate.jsonl"] = hashlib.sha256(raw.read_bytes()).hexdigest()
    record.write_text(json.dumps(result))


def test_complete_review_preserves_original_fail_and_all_attempts(tmp_path):
    raw, record, rows, _ = fixture(tmp_path)
    result = MODULE.review(raw, record)
    assert result["status"] == "DIAGNOSTIC_ONLY"
    assert result["original_confirmation_status"] == "FAIL"
    assert result["reviewed_primary_rows"] == 39
    assert result["full_attempt_count"] == len(rows)
    assert result["unpaired_common_comparison"]["ratio_difference"] == pytest.approx(0)


@pytest.mark.parametrize("kind", ["hash", "duplicate", "missing", "summary", "status"])
def test_corrupt_or_inconsistent_evidence_rejected(tmp_path, kind):
    raw, record, rows, result = fixture(tmp_path)
    if kind == "hash":
        raw.write_text(raw.read_text() + "\n")
    else:
        if kind == "duplicate":
            rows[-1] = copy.deepcopy(rows[0])
        elif kind == "missing":
            rows.pop()
        elif kind == "summary":
            result["audit"]["summaries"][0]["se_ratio"] += 1
        else:
            rows[0]["status"] = "q1_failed"
        write_fixture(raw, record, rows, result)
    with pytest.raises(ValueError):
        MODULE.review(raw, record)
