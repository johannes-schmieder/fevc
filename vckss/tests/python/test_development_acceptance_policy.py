from __future__ import annotations

import json
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[3]
POLICY_PATH = REPO_ROOT / "vckss/docs/development_acceptance_v1.json"


def test_performance_first_development_policy_is_explicit() -> None:
    policy = json.loads(POLICY_PATH.read_text(encoding="utf-8"))
    assert policy["schema"] == "VCKSS_DEVELOPMENT_ACCEPTANCE_V1"
    assert policy["status"] == "ACTIVE"
    assert policy["priority_order"][:2] == [
        "same_estimand_sample_target_and_statistical_result",
        "matlab_competitive_end_to_end_runtime",
    ]

    equivalence = policy["point_estimate_equivalence"]
    assert equivalence["scale_relative_tolerance"] == 1e-8
    assert equivalence["randomized_mcse_multiplier"] == 6.0
    assert policy["performance"]["competitive_matlab_ratio_max"] == 1.0
    assert policy["performance"]["development_target_matlab_ratio"] == 0.5

    blocking = policy["blocking"]
    assert blocking["bitwise_identity"] is False
    assert blocking["ulp_identity"] is False
    assert blocking["legacy_fixed_roundoff_gate"] is False


def test_active_instructions_reference_the_registered_policy() -> None:
    required = (
        REPO_ROOT / "AGENTS.md",
        REPO_ROOT / "vckss/AGENTS.md",
        REPO_ROOT / "vckss/PLAN.md",
        REPO_ROOT / "vckss/docs/DECISIONS.md",
        REPO_ROOT / "vckss/TESTING.md",
        REPO_ROOT / "rust/TEST_PLAN.md",
    )
    for path in required:
        assert "development_acceptance_v1.json" in path.read_text(encoding="utf-8")


def test_full_cmg_spike_is_equivalent_but_not_matlab_competitive() -> None:
    policy = json.loads(POLICY_PATH.read_text(encoding="utf-8"))
    baseline_covariance = 0.0903438541067414
    candidate_covariance = 0.0903438541045195
    scale = max(1.0, abs(baseline_covariance), abs(candidate_covariance))
    limit = policy["point_estimate_equivalence"]["scale_relative_tolerance"] * scale
    assert abs(candidate_covariance - baseline_covariance) <= limit

    candidate_seconds = 223.232
    matlab_seconds = 171.732766
    assert (
        candidate_seconds / matlab_seconds
        > policy["performance"]["competitive_matlab_ratio_max"]
    )


def test_active_alpha_harness_uses_policy_not_exact_repeatability() -> None:
    runner = (REPO_ROOT / "vckss/benchmarks/alpha/run_local.py").read_text(
        encoding="utf-8"
    )
    analyzer = (REPO_ROOT / "vckss/benchmarks/alpha/analyze.py").read_text(
        encoding="utf-8"
    )
    assert "COMMON_DRAW_TOLERANCE" in runner
    assert 'float(row["result_diff"]) != 0' not in runner
    assert "PRIMARY_RESULT_FIELDS" in analyzer
    assert "randomized_mcse_multiplier" in analyzer
    assert 'as_float(row, "result_diff") != 0' not in analyzer
    assert "matlab_performance_complete = False" in analyzer
    assert '"mata_performance_diagnostic_only": True' in analyzer
    assert "headline_speedups_at_least_two" not in analyzer
