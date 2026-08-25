from __future__ import annotations

import json
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[3]
HARNESS = REPO_ROOT / "vckss/benchmarks/full_cmg_spike"


def test_scc_spike_uses_the_registered_linux_plugin_name() -> None:
    wrapper = (HARNESS / "run_scc_smoke.sge").read_text(encoding="utf-8")
    driver = (HARNESS / "stata_run.do").read_text(encoding="utf-8")
    assert wrapper.count("vckss_rust_linux_x64.plugin") == 4
    assert "vckss_rust_linux_x64.plugin" in driver
    assert "vckss_rust_unix.plugin" not in wrapper + driver


def test_scc_spike_binds_locked_dependency_resolution() -> None:
    wrapper = (HARNESS / "run_scc_smoke.sge").read_text(encoding="utf-8")
    assert wrapper.count("--locked") == 3
    assert "--offline" not in wrapper
    for receipt_key in (
        "baseline_cargo_lock_sha256",
        "candidate_cargo_lock_sha256",
        "cmg_cargo_lock_sha256",
    ):
        assert receipt_key in wrapper


def test_spike_builds_archived_cmg_commit_without_touching_dirty_checkout() -> None:
    builder = (REPO_ROOT / "rust/full_cmg_spike/build_macos.sh").read_text(
        encoding="utf-8"
    )
    submit = (HARNESS / "submit_scc_smoke.sh").read_text(encoding="utf-8")
    assert 'git -C "${cmg_root}" archive "${cmg_commit}"' in builder
    assert '--manifest-path "${cmg_source}/Cargo.toml"' in builder
    assert "requires a clean CMG checkout" not in builder
    assert 'git -C "$cmg_root" status --porcelain' not in submit


def test_private_spike_reconciles_phase_specific_tolerances_only_under_consent() -> None:
    reconciler = (
        REPO_ROOT / "vckss/_vckss_rust_reconcile_comp_v7.ado"
    ).read_text(encoding="utf-8")
    assert "VCKSS_PRIVATE_CMG_FULL_V1" in reconciler
    assert "VCKSS_PRIVATE_CMG_FIT_TOLERANCE" in reconciler
    assert "VCKSS_PRIVATE_CMG_PROBE_TOLERANCE" in reconciler
    assert "expected_fit_full_tolerance" in reconciler
    assert "expected_probe_full_tolerance" in reconciler
    assert "row_full_tolerance" in reconciler


def test_decision_receipt_rejects_promotion_from_accepted_scc_evidence() -> None:
    receipt = json.loads(
        (HARNESS / "decision_receipt.json").read_text(encoding="utf-8")
    )
    assert receipt["schema"] == "VCKSS_FULL_CMG_ARCHITECTURAL_DECISION_V1"
    assert receipt["status"] == "REJECTED_PROMOTION"
    assert receipt["candidate_route"] == "CMG_FULL_SPIKE_V1"
    assert receipt["scc"]["job_id"] == 7306628
    assert receipt["scc"]["qacct_failed"] == 0
    assert receipt["scc"]["qacct_exit_status"] == 0
    assert receipt["scc"]["candidate_over_baseline_ratio"] < 1
    assert receipt["scc"]["candidate_over_matlab_ratio"] > 1
    assert receipt["scientific_gate"]["status"] == "FAIL"
    assert receipt["scientific_gate"]["gate_weakened"] is False
    assert receipt["next_action"] == "DO_NOT_HARDEN_OR_RUN_CZ18_WITH_THIS_ROUTE"
