from __future__ import annotations

import json
import runpy
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[3]
HARNESS = REPO_ROOT / "vckss/benchmarks/full_cmg_spike"


def test_scc_spike_uses_the_registered_linux_plugin_name() -> None:
    wrapper = (HARNESS / "run_scc_smoke.sge").read_text(encoding="utf-8")
    driver = (HARNESS / "stata_run.do").read_text(encoding="utf-8")
    assert wrapper.count("vckss_rust_linux_x64.plugin") == 4
    assert "vckss_rust_linux_x64.plugin" in driver
    assert "vckss_rust_unix.plugin" not in wrapper + driver


def test_cz18_driver_binds_restricted_sample_and_state_gates() -> None:
    driver = (HARNESS / "stata_run_cz18.do").read_text(encoding="utf-8")
    assert "1748ca2a6a46f248e05c0329407e7e7708ec7628c1ffce5f0e06ee264bdf0575" in driver
    for dimension in ("8201888", "117529", "10603", "311730"):
        assert dimension in driver
    assert "probeorder(observation_key)" in driver
    assert "backend(rust) rng(counter_v1)" in driver
    assert "deletion(match)" in driver
    assert "e(complete_residual_max)<=e(residual_acceptance_tolerance)" in driver
    assert "e(target_identity_residual)<=1e-12" in driver
    assert "data_restored" in driver
    assert "rng_restored" in driver
    assert "sort_rng_restored" in driver


def test_local_spike_uses_common_draw_corrected_target_policy() -> None:
    runner = runpy.run_path(str(HARNESS / "run_local.py"))
    left = {"corrected1": 100.0, "mcse1": 1e-4}
    right = {"corrected1": 100.0 + 5e-6, "mcse1": 8e-5}
    difference, limit, ratio = runner["common_draw_acceptance"](left, right, 1)
    assert difference < limit
    assert limit == 1e-5
    assert ratio < 1
    source = (HARNESS / "run_local.py").read_text(encoding="utf-8")
    assert "SCIENCE_TOLERANCE" not in source
    assert "a_c_secondary_differences" in source


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
    assert 'cmg_commit=${expected_cmg_commit}' in builder
    assert 'cmg_checkout_head=$(git -C "${cmg_root}" rev-parse HEAD)' in builder
    assert 'cp "${repo_root}/rust/full_cmg_spike/cmg_fused.rs"' in builder
    assert "fused_source_sha256" in builder
    assert '--manifest-path "${cmg_source}/Cargo.toml"' in builder
    assert "requires a clean CMG checkout" not in builder
    assert 'git -C "$cmg_root" status --porcelain' not in submit
    assert 'git -C "$cmg_root" cat-file -e "$cmg_commit^{commit}"' in submit
    assert 'git -C "$cmg_root" rev-parse HEAD' not in submit


def test_scc_spike_injects_fused_extension_only_into_scratch_copy() -> None:
    wrapper = (HARNESS / "run_scc_smoke.sge").read_text(encoding="utf-8")
    assert 'cp -R "$cmg_root/." "$scratch/cmg-source/"' in wrapper
    assert '"$scratch/cmg-source/src/vckss_fused.rs"' in wrapper
    assert '} >> "$scratch/cmg-source/src/lib.rs"' in wrapper
    assert '--manifest-path "$scratch/cmg-source/Cargo.toml"' in wrapper
    assert '>> "$cmg_root/src/lib.rs"' not in wrapper


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
    assert "row_reduced_tolerance" in reconciler
    assert "expected_max_reduced" in reconciler


def test_mixed_precision_spike_remains_private_and_f64_certified() -> None:
    source = (REPO_ROOT / "rust/crates/vckss-core/src/full_cmg_spike.rs").read_text(
        encoding="utf-8"
    )
    fused = (REPO_ROOT / "rust/full_cmg_spike/cmg_fused.rs").read_text(
        encoding="utf-8"
    )
    assert '"VCKSS_PRIVATE_CMG_MIXED_V1"' in source
    assert "requires {PRIVATE_FUSED_ENV}=1" in source
    assert "VckssFusedPcgSolver::build_mixed" in source
    assert "FusedCsrF32" in fused
    assert "mixed_output" in fused
    assert "fresh_residual: Vec<f64>" in fused
    assert "reduction_sums: Vec<f64>" in fused


def test_probe_inner_tolerance_is_not_silently_tightened() -> None:
    source = (REPO_ROOT / "rust/crates/vckss-core/src/full_cmg_spike.rs").read_text(
        encoding="utf-8"
    )
    assert "PRIVATE_FIT_INNER_TOLERANCE_RATIO: f64 = 0.01" in source
    assert "PRIVATE_PROBE_INNER_TOLERANCE_RATIO: f64 = 1.0" in source
    assert "probe_tolerance * PRIVATE_PROBE_INNER_TOLERANCE_RATIO" in source


def test_mixed_precision_receipt_disables_the_failed_candidate() -> None:
    receipt = json.loads(
        (HARNESS / "mixed_precision_2026-08-25.json").read_text(encoding="utf-8")
    )
    assert receipt["source_commit"] == "2f94e361f2e6da25d5d897be78355568b2e8ae1e"
    assert (
        receipt["published_source_equivalent_commit"]
        == "de866c21e6b11cd5e248ec523936c6174730da75"
    )
    assert receipt["source_equivalence"]["active_solver_source_changed"] is False
    assert receipt["decision"] == "PRESERVE_PRIVATE_AND_DISABLE"
    assert receipt["ratios"]["mixed_over_f64_command"] > 0.9
    assert receipt["ratios"]["mixed_over_f64_admitted_peak"] > 1
    assert receipt["gates"]["statistical_and_residual_gates_pass"] is True
    assert receipt["gates"]["enable_mixed_precision"] is False


def test_direct_probe_tolerance_receipt_keeps_scientific_gates() -> None:
    receipt = json.loads(
        (HARNESS / "probe_tolerance_direct_2026-08-25.json").read_text(
            encoding="utf-8"
        )
    )
    assert receipt["source_commit"] == "f615918794e01fb3e96ceec83c119981b8981834"
    assert receipt["case"]["probe_effective_tolerance"] == 1e-6
    assert receipt["case"]["probe_inner_tolerance"] == 1e-6
    assert receipt["solver"]["maximum_complete_residual"] < 1e-5
    assert receipt["ratios"]["direct_over_registered_matlab_command"] < 1
    assert receipt["gates"]["common_probe_statistical_pass"] is True
    assert receipt["gates"]["two_x_matlab_promotion_pass"] is False


def test_packed_counter_receipt_preserves_counter_and_scientific_contracts() -> None:
    receipt = json.loads(
        (HARNESS / "packed_counter_2026-08-25.json").read_text(encoding="utf-8")
    )
    assert receipt["source_commit"] == "9dbd03e06759a638eae9abea3386d752c577b5cc"
    assert receipt["ratios"]["packed_over_direct_counter_generation"] < 0.31
    assert receipt["ratios"]["packed_over_registered_matlab_command"] < 1
    assert receipt["solver"]["maximum_complete_residual"] < 1e-5
    assert receipt["gates"]["counter_scalar_oracle_pass"] is True
    assert receipt["gates"]["corrected_targets_bit_identical"] is True
    assert receipt["gates"]["two_x_matlab_promotion_pass"] is False


def test_parallel_residual_receipt_keeps_order_and_independent_gate() -> None:
    receipt = json.loads(
        (HARNESS / "parallel_residual_2026-08-25.json").read_text(encoding="utf-8")
    )
    assert receipt["source_commit"] == "e6f6b89856d0f5edcdd5a579dfe6c5a01a57dc32"
    assert receipt["ratios"]["parallel_over_packed_extraction"] < 0.25
    assert receipt["ratios"]["parallel_over_registered_matlab_command"] > 0.5
    assert receipt["solver"]["maximum_complete_residual"] < 1e-5
    assert receipt["gates"]["ordered_results_and_receipts_pass"] is True
    assert receipt["gates"]["worker_stata_api_absent"] is True
    assert receipt["gates"]["corrected_targets_bit_identical"] is True
    assert receipt["gates"]["two_x_matlab_promotion_pass"] is False


def test_alpha_headline_receipt_crosses_only_the_single_run_gate() -> None:
    receipt = json.loads(
        (HARNESS / "alpha_headline_2026-08-25.json").read_text(encoding="utf-8")
    )
    assert receipt["source_commit"] == "598a08d5c0792519b3d87d6f56f743cacbf93a24"
    assert receipt["ratios"]["headline_over_registered_matlab_command"] < 0.5
    assert receipt["solver"]["maximum_complete_residual"] < 1e-5
    assert receipt["solver"]["maximum_probe_iterations"] == 8
    assert receipt["gates"]["corrected_targets_bit_identical"] is True
    assert receipt["gates"]["single_run_two_x_matlab_pass"] is True
    assert receipt["gates"]["alternating_median_two_x_matlab_pass"] is False
    assert receipt["gates"]["cz18_two_x_matlab_pass"] is False
    assert receipt["gates"]["promotion_pass"] is False


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
