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


def test_cz18_scc_smoke_keeps_restricted_rows_remote_and_pinned() -> None:
    submit = (HARNESS / "submit_scc_cz18_smoke.sh").read_text(encoding="utf-8")
    wrapper = (HARNESS / "run_scc_cz18_smoke.sge").read_text(encoding="utf-8")
    matlab_driver = (
        REPO_ROOT / "vckss/benchmarks/scc/cmg_cz18_matlab_run.m"
    ).read_text(encoding="utf-8")
    retained_hash = "1748ca2a6a46f248e05c0329407e7e7708ec7628c1ffce5f0e06ee264bdf0575"
    assert retained_hash in submit
    assert retained_hash in wrapper
    assert "scp \"$input_dta\"" not in submit
    assert 'cp "$FCMG_CZ_INPUT_DTA" "$scratch/input/retained_sample.dta"' in wrapper
    assert "-pe omp 14" in submit
    assert "application_threads=4" in submit
    assert "CMG_CZ_M_PROBES=$FCMG_CZ_PROBES" in wrapper
    assert "test \"$FCMG_CZ_PROBES\" = 20 || test \"$FCMG_CZ_PROBES\" = 200" in wrapper
    assert "cz18_p200_decision" in wrapper
    assert "FCMG_CZ_FAST_PREPARATION" in submit
    assert "FCMG_CZ_FAST_PREPARATION" in wrapper
    assert "VCKSS_PRIVATE_CMG_FAST_PREP_V1=1" in wrapper
    assert "FCMG_CZ_RAW_MATCH" in submit
    assert "FCMG_CZ_RAW_MATCH" in wrapper
    assert "VCKSS_PRIVATE_CMG_RAW_MATCH_V1=1" in wrapper
    assert 'cp -R "$cmg_root/." "$scratch/cmg-source/"' in wrapper
    assert '"$scratch/cmg-source/src/vckss_fused.rs"' in wrapper
    assert "VCKSS_FULL_CMG_CZ18_SCC_SMOKE_PASS" in wrapper
    assert "any(probes == [20 200]) && seed == 8675309" in matlab_driver


def test_cz18_validator_applies_active_common_probe_gate() -> None:
    validator = (HARNESS / "validate_scc_cz18_smoke.py").read_text(
        encoding="utf-8"
    )
    assert "1e-8 * scale" in validator
    assert "0.1 * max(left_mcse, right_mcse)" in validator
    assert "common_probe_corrected_target_gates" in validator
    assert "DESCRIPTIVE_SINGLE_SEED_NO_REGISTERED_DISTRIBUTION" in validator
    assert "LEGACY_P20_NODE_COMMIT" in validator
    assert '"1e-8" if probes == 20 else "1e-9"' in validator
    assert "float(expected_inner_tolerance)" in validator
    assert "candidate_fast_preparation" in validator
    assert "fast_preparation=([01])" in validator
    assert "candidate_raw_match" in validator
    assert "raw_match=([01])" in validator
    assert "P200_FAST_PREPARATION_SINGLE_RUN_ONLY" in validator
    assert 'accounting["failed"] == accounting["exit_status"] == "0"' in validator
    assert '"P20_SMOKE_ONLY"' in validator
    assert '"P200_SINGLE_RUN_DECISION_ONLY"' in validator


def test_cz18_p20_reconciliation_checkpoint_stays_smoke_only() -> None:
    receipt = json.loads(
        (HARNESS / "cz18_p20_reconcile_2026-08-25.json").read_text(
            encoding="utf-8"
        )
    )
    assert receipt["status"] == "PASS"
    assert receipt["promotion_status"] == "P20_SMOKE_ONLY"
    assert receipt["source_commit"] == "eee4acc76902bcde0f7c1f1141e860ded7244fef"
    assert receipt["job"]["failed"] == receipt["job"]["exit_status"] == 0
    assert receipt["candidate_science"]["maximum_complete_residual"] < 1e-5
    assert receipt["candidate_science"]["data_restored"] is True
    assert receipt["ratios"]["candidate_over_baseline"] < 1
    assert receipt["ratios"]["candidate_over_matlab"] > 1
    assert receipt["decision"]["p20_promotes_performance"] is False


def test_cz18_p200_inner_tolerance_failure_does_not_weaken_gate() -> None:
    receipt = json.loads(
        (HARNESS / "cz18_p200_inner_1e8_failure_2026-08-25.json").read_text(
            encoding="utf-8"
        )
    )
    assert receipt["status"] == "EXPECTED_GATE_FAILURE"
    assert receipt["source_commit"] == "c1ae402c47227ac0adda6c4e301382c6c25c98cb"
    assert receipt["job"]["failed"] == 0
    assert receipt["job"]["exit_status"] == 1
    assert receipt["failure"]["code"] == "FULL_RESIDUAL_FAILED"
    assert receipt["failure"]["complete_residual"] > receipt["failure"]["limit"]
    assert receipt["decision"]["relax_complete_residual_gate"] is False
    assert receipt["decision"]["next_private_probe_inner_tolerance"] == 1e-9


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


def test_scc_spike_compares_direct_and_fused_at_one_probe_tolerance() -> None:
    wrapper = (HARNESS / "run_scc_smoke.sge").read_text(encoding="utf-8")
    driver = (HARNESS / "stata_run.do").read_text(encoding="utf-8")
    assert 'run_stata candidate "$FCMG_SOURCE_COMMIT"' in wrapper
    assert "run_stata candidate_fused" in wrapper
    assert "export VCKSS_PRIVATE_CMG_FUSED_V1=1" in wrapper
    assert "VCKSS_PRIVATE_CMG_PROBE_INNER_TOLERANCE || true" in wrapper
    assert "candidate_routes=direct,fused_f64" in wrapper
    assert '"candidate_fused"' in driver
    assert "tolerance(" not in driver


def test_spike_builds_archived_cmg_commit_without_touching_dirty_checkout() -> None:
    builder = (REPO_ROOT / "rust/full_cmg_spike/build_macos.sh").read_text(
        encoding="utf-8"
    )
    submit = (HARNESS / "submit_scc_smoke.sh").read_text(encoding="utf-8")
    assert 'git -C "${cmg_root}" archive "${cmg_commit}"' in builder
    assert 'cmg_commit=${expected_cmg_commit}' in builder
    assert 'cmg_checkout_head=$(git -C "${cmg_root}" rev-parse HEAD)' in builder
    assert 'cp "${repo_root}/rust/full_cmg_spike/cmg_fused.rs"' in builder
    assert 'cp "${repo_root}/rust/full_cmg_spike/cmg_pcg_pass_fused.rs"' in builder
    assert 'include!("vckss_pcg_pass_fused.rs")' in builder
    assert "fused_source_sha256" in builder
    assert "pass_fused_source_sha256" in builder
    assert '--manifest-path "${cmg_source}/Cargo.toml"' in builder
    assert "requires a clean CMG checkout" not in builder
    assert 'git -C "$cmg_root" status --porcelain' not in submit
    assert 'git -C "$cmg_root" cat-file -e "$cmg_commit^{commit}"' in submit
    assert 'git -C "$cmg_root" rev-parse HEAD' not in submit


def test_scc_spike_injects_fused_extension_only_into_scratch_copy() -> None:
    wrapper = (HARNESS / "run_scc_smoke.sge").read_text(encoding="utf-8")
    assert 'cp -R "$cmg_root/." "$scratch/cmg-source/"' in wrapper
    assert '"$scratch/cmg-source/src/vckss_fused.rs"' in wrapper
    assert '"$scratch/cmg-source/src/vckss_pcg_pass_fused.rs"' in wrapper
    assert 'include!("vckss_pcg_pass_fused.rs")' in wrapper
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
    assert "expected_fit_reduced_tolerance" in reconciler
    assert (
        "local expected_fit_reduced_tolerance = "
        "`expected_fit_full_tolerance'" in reconciler
    )
    assert "exceeded phase receipt limit" in reconciler
    assert "row_full_tolerance" in reconciler
    assert "row_reduced_tolerance" in reconciler
    assert "expected_max_reduced" in reconciler
    public = (REPO_ROOT / "vckss/vckss.ado").read_text(encoding="utf-8")
    planned = public.split(
        "program define _vckss_rust_generic_planned", maxsplit=1
    )[1].split("program define _vckss_rexact", maxsplit=1)[0]
    assert (
        planned.index(
            "local private_full_cmg_diagnostics : environment "
            "VCKSS_PRIVATE_CMG_DIAGNOSTICS"
        )
        < planned.index("CMG_FULL_SPIKE_V1 STATA_SOLVE_FAIL rc=")
    )
    poster = (REPO_ROOT / "vckss/_vckss_rust_post_comp_v7.ado").read_text(
        encoding="utf-8"
    )
    assert "CMG_FULL_SPIKE_V1 STATA_RECONCILE" in public
    assert "CMG_FULL_SPIKE_V1 STATA_POST rc=" in public
    assert "CMG_FULL_SPIKE_V1 STATA_SOLVE rc=0" in public
    assert "CMG_FULL_SPIKE_V1 STATA_SOLVE_FAIL rc=" in public
    assert "CMG_FULL_SPIKE_V1 STATA_RESULT_EXPORT rc=" in public
    assert "CMG_FULL_SPIKE_V1 STATA_RESULT_CONTEXT engine=" in public
    bridge = (REPO_ROOT / "vckss/vckss_rust.ado").read_text(encoding="utf-8")
    for stage in (
        "plugin_result",
        "rhs_result",
        "plan_receipt",
        "legacy_receipt_mismatch",
    ):
        assert f"stage={stage}" in bridge
    for stage in (
        "validated_context",
        "released_idle",
        "posting_results",
        "posted_results",
        "complete",
    ):
        assert f"stage={stage}" in poster
    for driver_name in ("stata_run.do", "stata_run_cz18.do"):
        driver = (HARNESS / driver_name).read_text(encoding="utf-8")
        assert "tolerance(" not in driver


def test_cz18_private_diagnostics_are_not_suppressed_by_quietly() -> None:
    harness = (HARNESS / "stata_run_cz18.do").read_text(encoding="utf-8")
    assert (
        "local private_full_cmg_diagnostics : environment "
        "VCKSS_PRIVATE_CMG_DIAGNOSTICS" in harness
    )
    assert 'local vckss_prefix "quietly"' in harness
    assert 'local vckss_prefix "noisily"' in harness
    assert "`vckss_prefix' vckss y" in harness


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


def test_direct_spike_consumes_contiguous_rhs_without_vec_of_vec_copy() -> None:
    source = (REPO_ROOT / "rust/crates/vckss-core/src/full_cmg_spike.rs").read_text(
        encoding="utf-8"
    )
    fused = (REPO_ROOT / "rust/full_cmg_spike/cmg_fused.rs").read_text(
        encoding="utf-8"
    )
    assert "VckssContiguousPcgWorkspace" in source
    assert "vckss_solve_contiguous_columns_with_workspace" in source
    assert "let scalar_rhs" not in source
    assert "pub struct VckssContiguousPcgWorkspace" in fused
    assert "rhs_chunk.par_chunks_exact(dimension)" in fused


def test_pass_fused_spike_is_private_pre_rng_and_fail_closed() -> None:
    source = (REPO_ROOT / "rust/crates/vckss-core/src/full_cmg_spike.rs").read_text(
        encoding="utf-8"
    )
    injected = (REPO_ROOT / "rust/full_cmg_spike/cmg_pcg_pass_fused.rs").read_text(
        encoding="utf-8"
    )
    fused = (REPO_ROOT / "rust/full_cmg_spike/cmg_fused.rs").read_text(
        encoding="utf-8"
    )
    assert '"VCKSS_PRIVATE_CMG_PASS_FUSED_V1"' in source
    assert source.index("private_pass_fused_requested()?") < source.index(
        "ParallelPcgSolver::build"
    )
    assert "pass_fused_requested" in source
    assert "pass_fused_used" in source
    assert "vckss_finest_component_count() == 1" in source
    assert "FullCmgSpikeExecution::PassFused" in source
    assert "pass_fused_requested={} pass_fused_used={}" in source
    assert "post-RNG fallback" in source
    assert "vckss_solve_pcg_pass_fused_with_workspace" in fused
    assert "official preconditioner, recurrence" in injected
    assert "original_residual_norm" in injected
    assert "ResidualVerificationFailed" in injected
    assert "MaximumIterations" in injected


def test_fast_preparation_spike_is_private_and_keeps_public_cancellation() -> None:
    source = (REPO_ROOT / "rust/crates/vckss-core/src/full_cmg_spike.rs").read_text(
        encoding="utf-8"
    )
    interrupt = (REPO_ROOT / "rust/crates/vckss-core/src/interrupt.rs").read_text(
        encoding="utf-8"
    )
    assert '"VCKSS_PRIVATE_CMG_FAST_PREP_V1"' in source
    assert "requires {PRIVATE_ENABLE_ENV}=1" in source
    assert "fast_preparation={}" in source
    assert '#[cfg(feature = "cmg-full-spike")]' in interrupt
    assert "values.sort_by" in interrupt
    assert "values.sort_unstable_by" in interrupt
    assert "bounded caller-thread polling inside every merge pass" in interrupt
    assert "merge_pass(values, &mut buffer" in interrupt
    assert "sift_down(values, root, len" in interrupt


def test_raw_match_spike_is_private_narrow_and_natively_reconciled() -> None:
    source = (REPO_ROOT / "rust/crates/vckss-core/src/full_cmg_spike.rs").read_text(
        encoding="utf-8"
    )
    problem = (REPO_ROOT / "rust/crates/vckss-core/src/problem.rs").read_text(
        encoding="utf-8"
    )
    bridge = (REPO_ROOT / "rust/crates/vckss-plugin/src/ffi_engine.rs").read_text(
        encoding="utf-8"
    )
    public = (REPO_ROOT / "vckss/vckss.ado").read_text(encoding="utf-8")
    assert '"VCKSS_PRIVATE_CMG_RAW_MATCH_V1"' in source
    assert "requires {PRIVATE_FAST_PREPARATION_ENV}=1" in source
    assert "raw_match={}" in source
    assert "implicit_match_keys" in problem
    assert ".checked_mul(firm_count)" in problem
    assert "raw match coordinate identifier overflow" in problem
    assert "signed exact binary64 integer identifiers" in public
    assert "allow_signed_identifiers = private_raw_match_requested()?" in bridge
    assert '"signed"' in bridge
    assert '"positive"' in bridge
    assert 'local native_error_phase "private_raw_match_reconcile"' in public
    assert "`g_init_rows'==`g_mover_rows'" in public
    assert "`g_degree_removed'==0" in public
    assert "`p_cells'==`p_units'" in public
    assert "if \"`deletion'\" == \"match\" & !`private_raw_match'" in public


def test_probe_inner_tolerance_is_explicitly_receipted_and_bounded() -> None:
    source = (REPO_ROOT / "rust/crates/vckss-core/src/full_cmg_spike.rs").read_text(
        encoding="utf-8"
    )
    assert "PRIVATE_FIT_INNER_TOLERANCE_RATIO: f64 = 0.01" in source
    assert "PRIVATE_PROBE_INNER_TOLERANCE_RATIO: f64 = 1.0" in source
    assert "probe_tolerance * PRIVATE_PROBE_INNER_TOLERANCE_RATIO" in source
    assert '"VCKSS_PRIVATE_CMG_PROBE_INNER_TOLERANCE"' in source
    assert "probe_inner_tolerance > probe_tolerance" in source
    assert "tolerance: probe_inner_tolerance" in source
    assert "probe_inner_tolerance={}" in source


def test_cz18_smoke_pre_registers_probe_count_specific_private_inner_solve() -> None:
    submit = (HARNESS / "submit_scc_cz18_smoke.sh").read_text(encoding="utf-8")
    wrapper = (HARNESS / "run_scc_cz18_smoke.sge").read_text(encoding="utf-8")
    assert "candidate_probe_inner_tolerance=1e-8" in submit
    assert "candidate_probe_inner_tolerance=1e-9" in submit
    assert "FCMG_CZ_PROBE_INNER_TOLERANCE" in wrapper
    assert "test \"$FCMG_CZ_PROBE_INNER_TOLERANCE\" = 1e-8" in wrapper
    assert "test \"$FCMG_CZ_PROBE_INNER_TOLERANCE\" = 1e-9" in wrapper
    assert (
        'export VCKSS_PRIVATE_CMG_PROBE_INNER_TOLERANCE='
        '"$FCMG_CZ_PROBE_INNER_TOLERANCE"' in wrapper
    )
    assert "unset VCKSS_PRIVATE_CMG_FULL_V1" in wrapper
    assert "VCKSS_PRIVATE_CMG_PROBE_INNER_TOLERANCE" in wrapper
    assert "candidate_fast_preparation=${3:-0}" in submit
    assert "candidate_raw_match=${4:-0}" in submit
    assert 'test "$candidate_fast_preparation" = 0 ||' in submit
    assert "VCKSS_PRIVATE_CMG_FAST_PREP_V1" in wrapper
    assert "VCKSS_PRIVATE_CMG_RAW_MATCH_V1 || true" in wrapper


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


def test_pass_fused_receipt_preserves_but_disables_small_gain() -> None:
    receipt = json.loads(
        (HARNESS / "pass_fused_2026-08-25.json").read_text(encoding="utf-8")
    )
    assert receipt["implementation_commit"] == "08565be7468c09999c984ae327d2498dcfb13fbb"
    assert receipt["benchmark_source_commit"] == "c1ae402c47227ac0adda6c4e301382c6c25c98cb"
    assert receipt["architecture"]["pre_rng_selection"] is True
    assert receipt["architecture"]["post_rng_fallback"] is False
    assert receipt["architecture"]["default_enabled"] is False
    assert receipt["science"]["corrected_targets_identical"] is True
    assert receipt["science"]["maximum_complete_residual"] < 1e-5
    assert receipt["best_adjacent_comparison"]["command_speedup"] < 1.1
    assert receipt["memory_admission"]["admitted_peak_ratio"] > 1
    assert receipt["decision"] == "PRESERVE_PRIVATE_AND_DISABLE"


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


def test_cz18_failure_receipt_preserves_the_unweakened_residual_gate() -> None:
    receipt = json.loads(
        (HARNESS / "cz18_p20_failure_2026-08-25.json").read_text(
            encoding="utf-8"
        )
    )
    assert receipt["source_commit"] == "bea4b7d4a8dec697b7af23603fe0ad18ee25986f"
    assert receipt["candidate"]["status"] == "FULL_RESIDUAL_FAILED"
    assert receipt["candidate"]["probe_effective_tolerance"] == 1e-6
    assert receipt["candidate"]["probe_complete_residual_tolerance"] == 1e-5
    assert receipt["candidate"]["observed_complete_residual"] > 1e-5
    assert receipt["scientific_gate"]["gate_weakened"] is False
    assert receipt["next_experiment"]["private_probe_inner_tolerance"] == 1e-8
    assert receipt["next_experiment"]["post_rng_fallback"] is False


def test_scc_synthetic_checkpoint_rejects_two_x_promotion() -> None:
    receipt = json.loads(
        (HARNESS / "scc_synthetic_2026-08-25.json").read_text(encoding="utf-8")
    )
    assert receipt["source_commit"] == "de63378005bbc370146fd65268df9b8c55435bd6"
    assert receipt["scheduler"]["failed"] == 0
    assert receipt["scheduler"]["exit_status"] == 0
    assert receipt["ratios"]["candidate_over_baseline"] < 0.3
    assert receipt["ratios"]["candidate_over_matlab"] > 0.5
    assert receipt["candidate"]["maximum_complete_residual"] < 1e-5
    assert receipt["gates"]["common_probe_statistical_pass"] is True
    assert receipt["gates"]["single_run_two_x_matlab_pass"] is False
    assert receipt["gates"]["promotion_pass"] is False
    assert receipt["next_action"].startswith("MEASURE_CURRENT_FUSED_BLOCK")


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


def test_scc_direct_fused_checkpoint_keeps_direct_and_rejects_promotion() -> None:
    receipt = json.loads(
        (HARNESS / "scc_direct_fused_2026-08-25.json").read_text(
            encoding="utf-8"
        )
    )
    assert receipt["source_commit"] == "5ea043e2b112d114352ba18e6aba9ddace55b08e"
    assert receipt["decision"] == "KEEP_DIRECT_DISABLE_FUSED_F64"
    assert 0.5 < receipt["ratios"]["direct_over_matlab"] < 1
    assert receipt["ratios"]["fused_over_direct"] > 1.5
    assert receipt["corrected_target_differences"]["direct_minus_fused_f64"] == [
        0.0,
        0.0,
        0.0,
        0.0,
    ]
    assert receipt["solver"]["direct_maximum_complete_residual"] < 1e-5
    assert receipt["gates"]["complete_residual_pass"] is True
    assert receipt["gates"]["single_run_two_x_matlab_pass"] is False
    assert receipt["gates"]["promotion_pass"] is False


def test_cmg_pcg_phase_profile_is_diagnostic_and_bitwise_checked() -> None:
    receipt = json.loads(
        (HARNESS / "cmg_pcg_phase_profile_2026-08-25.json").read_text(
            encoding="utf-8"
        )
    )
    assert receipt["cmg_source_commit"] == (
        "dbefbc5e3b442c6dde6e7861a66d82fd5ed24f10"
    )
    assert receipt["scope"] == "SYNTHETIC_STANDALONE_CMG_NOT_ESTIMATOR_BENCHMARK"
    phases = receipt["timing"]["phases"]
    assert phases["preconditioner"]["share"] > 0.5
    assert phases["centering"]["share"] > 0.13
    assert phases["norms"]["share"] > 0.12
    assert receipt["timing"]["profile_overhead_ratio"] < 1.02
    assert receipt["numerical"]["profiled_solution_bitwise_equal_to_production"]
    assert receipt["decision"] == (
        "PROFILE_REAL_HYBRID_THEN_TEST_DETERMINISTIC_PASS_FUSION"
    )
