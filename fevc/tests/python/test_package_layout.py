from __future__ import annotations

import re
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parents[2]
VERSION = "0.5.0-alpha.1"
API_LEVEL = 21


def test_package_manifest_is_complete() -> None:
    manifest = (ROOT / "fevc.pkg").read_text(encoding="utf-8").splitlines()
    shipped = {
        line.removeprefix("f ").strip()
        for line in manifest
        if line.startswith("f ")
    }
    assert shipped == {
        "LICENSE",
        "THIRD_PARTY_NOTICES.txt",
        "fevc.ado",
        "vckss.mata",
        "vckss_inference.mata",
        "vckss_graph.mata",
        "vckss_cmg.mata",
        "vckss_solver.mata",
        "vckss_rng.mata",
        "vckss_scale.mata",
        "vckss_resource.mata",
        "vckss_scale_engine.mata",
        "vckss_scale_runtime.mata",
        "vckss_lifecycle.ado",
        "fevc_run.ado",
        "fevc_rust.ado",
        "_vckss_rust_plugin_call.ado",
        "_vckss_rust_solve_v4.ado",
        "_vckss_rust_solve_v5.ado",
        "_vckss_rust_plan_receipt.ado",
        "_vckss_rust_reconcile_comp_v7.ado",
        "_vckss_rust_reconcile_exact_v7.ado",
        "_vckss_rust_post_comp_v7.ado",
        "_vckss_rust_post_exact_v7.ado",
        "_vckss_rust_capture_stayers.ado",
        "_vckss_rust_post_stayer_hybrid.ado",
        "_vckss_rust_macos.ado",
        "_vckss_rust_windows.ado",
        "_vckss_rust_linux.ado",
        "_vckss_rust_public_call.ado",
        "fevc.sthlp",
    }
    for relative in shipped:
        assert (ROOT / relative).is_file()


def test_macos_qualifier_reports_stayer_hybrid_coverage() -> None:
    qualifier = (
        ROOT.parent / "rust" / "stata_backend" / "qualify_macos.sh"
    ).read_text(encoding="utf-8")
    route = (
        "public-exact-stayer-hybrid-backend-rust-stayers-both-"
        "mover-headline-mixed-deletion-augmentation-reconciliation-"
        "differential-oracle-zero-rng-lifecycle"
    )
    assert route in qualifier
    assert "native-Intel,stayers,scale" not in qualifier
    for architecture in ("arm64", "x86_64"):
        for artifact in ("", "universal_"):
            key = f"{architecture}_{artifact}public_stayer_hybrid"
            command = f"command.test_{key}="
            assert f"{key}=PASS test_stayers_hybrid.do" in qualifier
            assert command in qualifier


def test_version_identifiers_agree() -> None:
    assert VERSION in (ROOT / "fevc.ado").read_text(encoding="utf-8")
    assert VERSION in (ROOT / "vckss.mata").read_text(encoding="utf-8")
    assert VERSION in (ROOT / "fevc.sthlp").read_text(encoding="utf-8")
    toc = (ROOT / "stata.toc").read_text(encoding="utf-8").splitlines()
    assert toc[0] == f"v {VERSION}"


def test_mata_api_guard_agrees() -> None:
    ado = (ROOT / "fevc.ado").read_text(encoding="utf-8")
    mata = (ROOT / "vckss.mata").read_text(encoding="utf-8")
    assert f"vckss__api_level() == {API_LEVEL}" in ado
    assert f"return({API_LEVEL})" in mata
    build_id = "fevc-api21-stayer-hybrid"
    assert f'local expected_mata_build "{build_id}"' in ado
    assert 'vckss__build_id() == "`expected_mata_build\'"' in ado
    assert f'return("{build_id}")' in mata
    graph = (ROOT / "vckss_graph.mata").read_text(encoding="utf-8")
    assert "vckss_graph__api_level()" in graph
    assert "return(21)" in graph
    assert "vckss-graph-api21-prep-map1-retained" in graph
    solver = (ROOT / "vckss_solver.mata").read_text(encoding="utf-8")
    assert "vckss_solver__api_level()" in solver
    assert "return(26)" in solver
    assert "vckss-solver-api26-gpl-mata-cmg" in solver
    assert "vckss_solver__route_api()" in solver
    assert "vckss_solver__pilot_api()" not in solver
    resource = (ROOT / "vckss_resource.mata").read_text(encoding="utf-8")
    assert "vckss_resource__api_level()" in resource
    assert "return(10)" in resource
    assert "vckss-resource-api10-fe-buf1-buffered" in resource
    rng = (ROOT / "vckss_rng.mata").read_text(encoding="utf-8")
    assert "return(4)" in rng
    assert "vckss-rng-numeric-ranks-v4" in rng
    cmg = (ROOT / "vckss_cmg.mata").read_text(encoding="utf-8")
    assert "vckss_cmg__api_level()" in cmg
    assert "return(8)" in cmg
    assert '"STALE_MATA_RUNTIME"' in ado
    assert '"AMBIGUOUS_PROBE_ORDER"' not in ado
    assert "`target'/`frequency'" in ado
    assert "local semantic_key `id_worker' `id_firm'" in ado
    assert "PROBEOrder(varname numeric)" in ado
    assert '"INVALID_PROBE_ORDER"' in ado
    assert "`semantic_key' `probeorder'" in ado
    assert "sort `semantic_key' `controlvars'" not in ado
    assert 'ereturn local fe_buffer_profile_schema "FE-BUF-PERF-V1"' in ado
    assert "ereturn matrix fe_buffer_profile" in ado
    assert (
        'ereturn local prep_boundary_profile_schema "PREP-BND-PERF-V1"' in ado
    )
    assert (
        'ereturn local prep_boundary_counts_schema "PREP-BND-COUNTS-V1"' in ado
    )
    assert "ereturn matrix prep_boundary_profile" in ado
    assert "ereturn matrix prep_boundary_counts" in ado


def test_control_and_frequency_certificates_are_fail_closed() -> None:
    ado = (ROOT / "fevc.ado").read_text(encoding="utf-8")
    compact_ado = re.sub(r"\s+", "", ado)
    mata = re.sub(
        r"\s+", "", (ROOT / "vckss.mata").read_text(encoding="utf-8")
    )
    assert "vckss__exact_physical_total" in ado
    assert '"PHYSICAL_TOTAL_LIMIT"' in ado
    assert "ifscalar(`retained_physical_total')>`physical_limit'" in compact_ado
    assert "if(control_count>32)" in mata
    assert "operator_residual=sqrt(dimension)*max_column_relres" in mata
    assert "denominator=reciprocal_condition-operator_residual" in mata
    assert "projection_error=vckss__norm2(projector*projector-projector)" in mata
    assert "span_error=vckss__norm2(out.controls-orthonormal*checked)" in mata
    assert "return(1e-8)" in mata
    assert mata.count("vckss__propagate_error(") >= 6


def test_runtime_has_no_external_language_dependency() -> None:
    runtime = "\n".join(
        (ROOT / name).read_text(encoding="utf-8").lower()
        for name in (
            "fevc.ado",
            "vckss.mata",
            "vckss_graph.mata",
            "vckss_cmg.mata",
            "vckss_solver.mata",
            "vckss_rng.mata",
            "vckss_scale.mata",
            "vckss_resource.mata",
            "vckss_scale_engine.mata",
            "vckss_scale_runtime.mata",
            "vckss_lifecycle.ado",
            "fevc_run.ado",
        )
    )
    external_invocation = re.compile(
        r"(?m)^\s*(?:shell\b|!\s*(?:python|matlab)\b|python:|rcall\b|matlab\s+-)"
    )
    assert external_invocation.search(runtime) is None


def test_help_examples_are_installed_and_uniquely_marked() -> None:
    help_text = (ROOT / "fevc.sthlp").read_text(encoding="utf-8")
    runner = (ROOT / "fevc_run.ado").read_text(encoding="utf-8")
    examples = ("exact_controls", "jla_controls", "weights_targets")
    for example in examples:
        marker = f"{{* example_start - {example}}}{{...}}"
        assert help_text.count(marker) == 1
        assert (
            f"fevc_run {example} using fevc.sthlp"
            in help_text
        )
    assert help_text.count("{* example_end}{...}") == len(examples)
    assert help_text.count(
        'display as text _newline "True DGP worker-firm components (population):"'
    ) == len(examples)
    assert help_text.count('display as text "  Var(worker effect)') == len(examples)
    assert help_text.count('display as text "  Var(firm effect)') == len(examples)
    assert help_text.count('display as text "  Cov(worker, firm)') == len(examples)
    assert help_text.count('display as text "  Var(worker + firm)') == len(examples)
    assert "program define fevc_run" in runner
    assert "preserve" in runner
    assert "capture restore" in runner
    assert "exit `example_rc'" in runner


def test_package_records_internal_license_boundary() -> None:
    manifest = (ROOT / "fevc.pkg").read_text(encoding="utf-8").lower()
    readme = " ".join(
        (ROOT / "README.md").read_text(encoding="utf-8").lower().split()
    )
    assert "gpl-3.0-only development package" in manifest
    assert "no public release has yet been issued" in manifest
    assert "gpl-3.0-only" in readme
    assert "human package-boundary and provenance review" in readme
    assert "was completed on 29 august 2026" in readme
    assert (ROOT.parent / "LICENSE").read_bytes() == (ROOT / "LICENSE").read_bytes()
    assert (ROOT / "THIRD_PARTY_NOTICES.txt").is_file()


def test_public_command_is_a_hard_cut_without_predecessor_alias() -> None:
    ado = (ROOT / "fevc.ado").read_text(encoding="utf-8")
    assert "program define fevc, eclass" in ado
    assert 'ereturn local cmd "fevc"' in ado
    assert 'ereturn local cmdline `"fevc `0\'"\'' in ado
    assert not (ROOT / "kss_bc.ado").exists()


def test_mata_uses_valid_noncolliding_profile_timers() -> None:
    runtime = (ROOT / "vckss.mata").read_text(encoding="utf-8")
    timer_ids = {
        int(value)
        for value in re.findall(r"timer_(?:clear|on|off|value)\((\d+)\)", runtime)
    }
    assert timer_ids == {91, 92, 93, 94, 95, 96, 97, 98}
    assert all(1 <= value <= 100 for value in timer_ids)


def test_scc_outer_command_clock_does_not_use_mata_profile_timers() -> None:
    driver = (
        ROOT / "benchmarks" / "scc" / "kss_prod_driver.do"
    ).read_text(encoding="utf-8")
    runtime = (ROOT / "vckss.mata").read_text(encoding="utf-8")
    driver_ids = {
        int(value)
        for value in re.findall(r"timer (?:clear|on|off|list) (\d+)", driver)
    }
    runtime_ids = {
        int(value)
        for value in re.findall(r"timer_(?:clear|on|off|value)\((\d+)\)", runtime)
    }
    assert driver_ids == set()
    assert driver_ids.isdisjoint(runtime_ids)
    assert driver.count('clock(c(current_date)+" "+c(current_time)') == 2


def test_public_solver_diagnostics_are_posted() -> None:
    ado = (ROOT / "fevc.ado").read_text(encoding="utf-8")
    benchmark = (ROOT / "benchmarks" / "synthetic_benchmark.do").read_text(
        encoding="utf-8"
    )
    for name in (
        "setup_seconds",
        "schur_seconds",
        "preconditioner_apply_seconds",
        "pcg_seconds",
        "solver_backend_seconds",
    ):
        assert f"ereturn scalar {name}" in ado
        assert name in benchmark
    assert "ereturn matrix solver_rhs_diagnostics" in ado
    assert "relative_residual converged" in benchmark


def test_api19_public_routing_surface_is_typed() -> None:
    ado = (ROOT / "fevc.ado").read_text(encoding="utf-8")
    for token in (
        "PREConditioner(string)",
        "MEMory_gib(real 4)",
        'local batch_requested = lower(strtrim("`batch\'"))',
        "vckss__stata_jla_routed",
        "ereturn matrix route_diagnostics",
        "ereturn local preconditioner_requested",
        "ereturn local preconditioner_selected",
        "ereturn local routing_reason",
        "ereturn local fallback_status",
        "ereturn local fallback_message",
        'ereturn local route_api "KSS-ROUTE-STRUCTURAL-V1"',
        "ereturn scalar memory_gib",
    ):
        assert token in ado
    assert ado.index("vckss__stata_jla_routed") < ado.index(
        "ereturn post `corrected'"
    )


def test_scc_validator_requires_numopt_evidence() -> None:
    validator = (ROOT / "benchmarks" / "validate_scc.py").read_text()
    for field in (
        "setup_seconds",
        "schur_seconds",
        "preconditioner_apply_seconds",
        "pcg_seconds",
        "solver_schur_actions",
        "solver_precond_applications",
        "relative_residual",
    ):
        assert field in validator
    assert 'diagnostic["stata_version"].startswith("19")' in validator
    assert 'diagnostic["stata_flavor"] in {"IC", "MP"}' in validator
    assert 'require(converged == 1' in validator


def test_production_finite_projection_uses_mixed_coefficient_one() -> None:
    runtime = re.sub(
        r"\s+", "", (ROOT / "vckss.mata").read_text(encoding="utf-8")
    )
    mixed_term = "(m_constrained-p_constrained):*mixed_second"
    assert runtime.count(mixed_term) == 1
    assert f"2:*{mixed_term}" not in runtime


def test_final_corrected_target_subtraction_is_fail_closed() -> None:
    runtime = re.sub(
        r"\s+", "", (ROOT / "vckss.mata").read_text(encoding="utf-8")
    )
    assert runtime.count("corrected=plugin-correction") == 3
    gate = (
        'if(hasmissing(corrected)){return(vckss__failure('
        '"NONFINITE_CORRECTED_TARGET"'
    )
    assert runtime.count(gate) == 3
    assert runtime.count("out.corrected=corrected") == 3


def test_observation_frequency_tracks_literal_physical_copy_moments() -> None:
    runtime = re.sub(
        r"\s+", "", (ROOT / "vckss.mata").read_text(encoding="utf-8")
    )
    assert "physical_random_batch=J(physical_count,batch_columns,.)" in runtime
    assert (
        "copy_first_correlation=copy_first_correlation+"
        "physical_random:*physical_projected" in runtime
    )
    assert (
        "copy_third_correlation=copy_third_correlation+"
        "physical_random:*physical_projected:^3" in runtime
    )
    assert (
        "m_second=probes:+6:*p_first:+p_second:-"
        "4:*copy_first_correlation:-4:*copy_third_correlation" in runtime
    )
    assert (
        "inverse_weight=panelsum(copy_inverse_weight,physical_panel):/frequency"
        in runtime
    )
    assert "observation_residual_square" not in runtime


def test_joint_solver_gates_each_batched_rhs_column() -> None:
    runtime = re.sub(
        r"\s+", "", (ROOT / "vckss.mata").read_text(encoding="utf-8")
    )
    assert "out.rhs_relres=vckss__column_relres(residual,right_hand_side)" in runtime
    assert "out.relres=max(out.rhs_relres)" in runtime
    assert (
        "out.relres=vckss__norm2(residual)/(1+vckss__norm2(right_hand_side))"
        not in runtime
    )
    assert "out.relres=vckss__max_column_relres(residual,I(dimension))" in runtime
    assert (
        "out.preparation_relres=vckss__max_column_relres("
        "schur-checked_schur,checked_schur)" in runtime
    )
    assert "tolerance*reduced_scale" in runtime
    assert "tolerance*(1+reduced_scale)" not in runtime
    assert "out.relres=vckss__norm2(full_residual)/full_scale" in runtime
    assert "full_firm_rhs=firm_rhs\\(sum(worker_rhs)-sum(firm_rhs))" in runtime
    assert "firm_coefficient=firm_coefficient:-normalization" in runtime
    assert "explicit_residual=reduced_rhs-action" in runtime
    assert "if(restart[column])" in runtime
    assert "out.rhs_iterations[column]=iteration" in runtime
    assert "fitted=(firm_coefficient\\0)[design.firm]" not in runtime
    assert "out.controls=orthonormal*anchor'*anchor_inverse.inverse" in runtime


def test_only_factor_metadata_can_remove_an_omitted_control() -> None:
    runtime = (ROOT / "fevc.ado").read_text(encoding="utf-8")
    assert "fvexpand `controls' if `touse'" in runtime
    assert "_ms_parse_parts `term'" in runtime
    assert "if !r(omit) local controlvars" in runtime
    assert "count if `control' != 0" not in runtime


def test_rank_certificate_subtracts_numerical_margin() -> None:
    runtime = re.sub(
        r"\s+", "", (ROOT / "vckss.mata").read_text(encoding="utf-8")
    )
    assert (
        "out.gap=min((1-out.max_loss,minimum_deleted_eigen))-"
        "whitening_error-threshold" in runtime
    )
    assert "centered=controls-cell_mean[cell_of_row,.]" in runtime
    assert "transformed=centered*whitener" in runtime
    assert "deleted_inverse=vckss__inverse(deleted_within,rank_tolerance)" in runtime
    assert "min((1-out.max_loss,minimum_deleted_eigen))" in runtime
    assert (
        "deleted_information_inverse=vckss__inverse("
        "deleted_information,rank_tolerance)" in runtime
    )
    assert "inverse_forward_bound>=0.01" in runtime
    assert "solver_residual=max((solver_residual,reduced_maker.relres))" in runtime
    assert (
        "residual=out.actions-factor*(factor'*out.actions)-right_hand_side"
        in runtime
    )


def test_scc_harness_is_project_scoped_and_public_only() -> None:
    scc = ROOT / "benchmarks" / "scc"
    submit = (scc / "submit_one.sh").read_text(encoding="utf-8")
    portability = (scc / "run_portability.sge").read_text(encoding="utf-8")
    stata_oracle = (
        ROOT / "benchmarks" / "oracle" / "stata_oracle.do"
    ).read_text(encoding="utf-8")
    scale_driver = (
        ROOT / "benchmarks" / "synthetic_benchmark.do"
    ).read_text(encoding="utf-8")
    job_scripts = [
        (scc / name).read_text(encoding="utf-8")
        for name in ("run_portability.sge", "run_oracle.sge", "run_scale.sge")
    ]
    assert "qsub -terse -P welfgr" in submit
    assert submit.count("-pe omp 4") == 3
    assert "large) job_runtime=18:00:00; job_memory=8G" in submit
    assert "#$ -l h_rt=18:00:00" in job_scripts[2]
    assert all("#$ -P welfgr" in script for script in job_scripts)
    assert all("#$ -pe omp 4" in script for script in job_scripts)
    executable_harness = submit + "\n" + "\n".join(job_scripts)
    assert "application/data" not in executable_harness
    assert "separations" not in executable_harness.lower()
    assert '> "$job_dir/portability.pass"' in portability
    assert "clear varnames(1) asdouble" in stata_oracle
    assert "tolerance(1e-8)" in scale_driver


def test_scc_scale_controls_have_stable_canonical_anchors() -> None:
    controls = np.array(
        [[-0.5, -0.8], [0.5, 0.3], [-0.5, 0.6], [0.5, -0.2]],
        dtype=float,
    )
    frequency = np.array([2.0, 2.0, 1.0, 1.0])
    margin = 1e-7  # 1,000 times the registered rank tolerance.

    for workers in (5_000, 50_000, 250_000):
        gram = workers * controls.T @ np.diag(frequency) @ controls
        orthonormal = controls @ np.linalg.cholesky(np.linalg.inv(gram))
        residualized = orthonormal.copy()
        selected: list[int] = []
        for _ in range(controls.shape[1]):
            score = np.sum(residualized**2, axis=1)
            cutoff = np.max(score) - margin * max(1.0, np.max(score))
            assert np.min(np.abs(score - cutoff)) > margin / 2
            eligible = np.flatnonzero(score > cutoff)
            assert eligible.size == 1
            selected.append(int(eligible[0]))
            anchor = orthonormal[selected, :]
            projector = anchor.T @ np.linalg.inv(anchor @ anchor.T) @ anchor
            residualized = orthonormal - orthonormal @ projector


def test_cmg_route_is_installed_behind_package_solver_contract() -> None:
    ado = (ROOT / "fevc.ado").read_text(encoding="utf-8").lower()
    mata = (ROOT / "vckss.mata").read_text(encoding="utf-8")
    solver = (ROOT / "vckss_solver.mata").read_text(encoding="utf-8")
    adapter = (ROOT / "tests/support/vckss_cmg_adapter.mata").read_text(
        encoding="utf-8"
    )
    bridge = (ROOT / "tests/support/vckss_cmg_bridge_override.mata").read_text(
        encoding="utf-8"
    )
    manifest = (ROOT / "fevc.pkg").read_text(encoding="utf-8")
    assert "preconditioner(string)" in ado
    assert "fevc/cmg" not in manifest
    assert "f vckss_cmg.mata" in manifest
    assert "f vckss_solver.mata" in manifest
    assert "struct vckss_solver_backend" in mata
    assert "vckss__fe_solve_matrix_backend" in mata
    assert "design,residual[.,active_index]" in mata
    assert "vckss__stata_jla_routed" in solver
    assert "fallback_status" in solver and "fallback_message" in solver
    assert "vckss__fe_solve_matrix_backend(" in adapter
    assert "vckss_cmg__apply_kss" in adapter
    assert "hybrid_vertices" in adapter and "hybrid_edges" in adapter
    end_to_end = (ROOT / "tests/stata/test_forced_cmg_e2e.do").read_text(
        encoding="utf-8"
    ).lower()
    assert "preconditioner(diagonal)" in end_to_end
    assert "preconditioner(cmg)" in end_to_end
    assert "e(preconditioner_selected)" in end_to_end
    assert '== "cmg"' in end_to_end
    assert "mata drop vckss__stata_jla()" not in end_to_end
    assert "VCKSS_CMG_ROUTE_DIAGNOSTICS" in bridge


def test_numopt_and_real_data_harnesses_enforce_bounded_routes() -> None:
    benchmark = (ROOT / "benchmarks/estimator_cmg_benchmark.do").read_text(
        encoding="utf-8"
    )
    real_benchmark = (
        ROOT / "benchmarks/separations_wage_estimator.do"
    ).read_text(encoding="utf-8")
    scc = ROOT / "benchmarks/scc"
    numopt_submit = (scc / "submit_numopt.sh").read_text(encoding="utf-8")
    real_submit = (scc / "submit_separations.sh").read_text(encoding="utf-8")
    wrappers = "\n".join(
        (scc / name).read_text(encoding="utf-8")
        for name in (
            "run_numopt.sge",
            "run_separations_estimator.sge",
            "run_separations_matlab.sge",
        )
    )
    for driver in (benchmark, real_benchmark):
        assert "`projected_seconds' > 5400" in driver
        assert "tolerance(1e-10)" in driver
        assert "seed(`benchmark_seed')" in driver
        assert "probes(`probes')" in driver
    # This legacy paired B1/CMG harness must keep exercising the general
    # engine after API 19 makes the compressed scale path automatic.
    assert "engine(generic) backend(mata) rng(stata) nodisplay" in benchmark
    assert "engine(generic) nodisplay" in real_benchmark
    processor_benchmark = (
        ROOT / "benchmarks/local_processor_scaling.do"
    ).read_text(encoding="utf-8")
    assert "engine(generic) nodisplay" in processor_benchmark
    assert "generate double hybrid_vertices" in real_benchmark
    assert "generate double hybrid_edges" in real_benchmark
    assert "projected_seconds" in numopt_submit
    assert "projected_seconds" in real_submit
    assert wrappers.count("/usr/bin/timeout --signal=TERM 5400") == 3
    assert wrappers.count("#$ -pe omp 4") == 3
    assert wrappers.count("#$ -l mem_per_core=16G") == 3
    assert "KSS_MEMORY_GIB" in wrappers
    assert "KSS_MATLAB_CORE_SHA256" in wrappers
    assert "KSS_MATLAB_CMG_SHA256" in wrappers
    assert "KSS_MATLAB_CMG_MEX_SHA256" in wrappers
    assert "KSS_MATLAB_CMG_SOLVER_SHA256" in wrappers


def test_separations_harness_is_read_only_and_aggregate_collectable() -> None:
    preparer = (ROOT / "benchmarks/separations_wage_prepare.do").read_text(
        encoding="utf-8"
    )
    submit = (
        ROOT / "benchmarks/scc/submit_separations.sh"
    ).read_text(encoding="utf-8")
    matlab = (ROOT / "benchmarks/separations_kss_reference.m").read_text(
        encoding="utf-8"
    )
    validator = (ROOT / "benchmarks/validate_separations.py").read_text(
        encoding="utf-8"
    )
    subset_validator = (
        ROOT / "benchmarks/validate_matlab_subset.py"
    ).read_text(encoding="utf-8")
    sample = (ROOT / "benchmarks/separations_matlab_sample.do").read_text(
        encoding="utf-8"
    )
    sample_mata = (ROOT / "benchmarks/separations_sample.mata").read_text(
        encoding="utf-8"
    )
    sample_wrapper = (
        ROOT / "benchmarks/scc/run_separations_matlab_sample.sge"
    ).read_text(encoding="utf-8")
    assert "confirm file" in preparer
    assert "save `\"`wage_input'" not in preparer
    assert "KSS_MATLAB_MEX_DIR" in matlab
    assert "mex('-silent', '-largeArrayDims', '-outdir', mex_output_dir" in matlab
    assert "addpath(mex_output_dir, '-begin')" in matlab
    assert "graphprofile_path = which('graphprofile')" in matlab
    assert "startsWith(graphprofile_path, [mex_output_dir filesep])" in matlab
    assert "preconditioner_path = which('mx_d_preconditioner')" in matlab
    assert "matlab_cmg_solver_sha256" in validator
    assert 'parser.add_argument("--matlab-only", action="store_true")' in validator
    assert "FEVC SEPARATIONS MATLAB-ONLY EVIDENCE PASS" in validator
    assert "matlab_cmg_mex_sha256" in validator
    assert "logrwage-xb" in preparer
    assert "keep if estabfe < ." in preparer
    assert "isid persid estabid time" in preparer
    assert "isid worker firm period" not in preparer
    assert "isid worker firm period" not in (
        ROOT / "benchmarks/separations_wage_estimator.do"
    ).read_text(encoding="utf-8")
    assert "aggregate_duplicate_rows" in preparer
    assert "semantic_tie_rows" in preparer
    assert "isid observation_key" in preparer
    assert "sort persid time estabid" in preparer
    assert "sort worker observation_key" in preparer
    assert "dense_mover_core" in preparer
    assert "total(`pair_tag'*`firm_workers')" in preparer
    assert "keep if `worker_tag' & `worker_firms' > 1" in preparer
    assert "probeorder(observation_key)" in (
        ROOT / "benchmarks/separations_wage_estimator.do"
    ).read_text(encoding="utf-8")
    assert "/projectnb/welfgr/separations/*" in submit
    assert "sha256sum" not in preparer
    assert "leave_out_KSS" in matlab
    assert "probes ~= 200" in matlab
    assert "rng(seed, 'twister')" in matlab
    assert "sortrows([worker period firm], [1 2 3])" in matlab
    assert "addpath(genpath(fullfile(kss_root, 'CMG')))" in matlab
    assert "automatic routing remains disabled" in validator
    assert 'parser.add_argument(\n        "--omit-exact"' in subset_validator
    assert "if not args.omit_exact:" in subset_validator
    assert '"--oracle-run-dir"' in subset_validator
    assert '"--oracle-label"' in subset_validator
    assert "post-oracle step requires oracle run" in subset_validator
    assert "oracle run source mismatch" in subset_validator
    assert "post-oracle scale step" in subset_validator
    assert 'parser.add_argument("--include-matlab", action="store_true")' in subset_validator
    assert "MATLAB/B1 retained samples differ" in subset_validator
    assert "matlab_retained_bridge_core" in sample
    assert "prepared.csv" in sample
    assert "vckss__stata_prune_graph" in sample
    assert "vckss_sep__stata_prune_bridges" in sample
    assert "low[node] > discovery[parent]" in sample_mata
    assert "KSS_MATLAB_RUN" in sample_wrapper
    assert 'case "$KSS_MATLAB_RUN" in /projectnb/welfgr/fevc/runs/*)' in sample_wrapper
    assert "sha256sum \"$matlab_detail\"" in sample_wrapper
    assert 'sha256sum "$output_dir/prepared.csv"' in sample_wrapper
    assert 'case "$job" in' in submit and "matlab-sample)" in submit
    assert submit.count("memory=16G") == 3
    assert 'exact|b1|cmg)' in submit
