from __future__ import annotations

from pathlib import Path
import re


ROOT = Path(__file__).resolve().parents[2]
VERSION = "0.1.0-dev"
API_LEVEL = 12


def test_package_manifest_is_complete() -> None:
    manifest = (ROOT / "kss_bc.pkg").read_text(encoding="utf-8").splitlines()
    shipped = {
        line.removeprefix("f ").strip()
        for line in manifest
        if line.startswith("f ")
    }
    assert shipped == {"kss_bc.ado", "kss_bc.mata", "kss_bc.sthlp"}
    for relative in shipped:
        assert (ROOT / relative).is_file()


def test_version_identifiers_agree() -> None:
    assert VERSION in (ROOT / "kss_bc.ado").read_text(encoding="utf-8")
    assert VERSION in (ROOT / "kss_bc.mata").read_text(encoding="utf-8")
    assert VERSION in (ROOT / "kss_bc.sthlp").read_text(encoding="utf-8")
    toc = (ROOT / "stata.toc").read_text(encoding="utf-8").splitlines()
    assert toc[0] == f"v {VERSION}"


def test_mata_api_guard_agrees() -> None:
    ado = (ROOT / "kss_bc.ado").read_text(encoding="utf-8")
    mata = (ROOT / "kss_bc.mata").read_text(encoding="utf-8")
    assert f"kssbc__api_level() == {API_LEVEL}" in ado
    assert f"return({API_LEVEL})" in mata
    build_id = "kss-bc-api12-certified-anchor-finite-corrected"
    assert f'local expected_mata_build "{build_id}"' in ado
    assert 'kssbc__build_id() == "`expected_mata_build\'"' in ado
    assert f'return("{build_id}")' in mata
    assert '"STALE_MATA_RUNTIME"' in ado
    assert '"AMBIGUOUS_PROBE_ORDER"' in ado
    assert "`target'/`frequency'" in ado


def test_runtime_has_no_external_language_dependency() -> None:
    runtime = "\n".join(
        (ROOT / name).read_text(encoding="utf-8").lower()
        for name in ("kss_bc.ado", "kss_bc.mata")
    )
    external_invocation = re.compile(
        r"(?m)^\s*(?:shell\b|!\s*(?:python|matlab)\b|python:|rcall\b|matlab\s+-)"
    )
    assert external_invocation.search(runtime) is None


def test_package_records_internal_license_boundary() -> None:
    manifest = (ROOT / "kss_bc.pkg").read_text(encoding="utf-8").lower()
    readme = " ".join(
        (ROOT / "README.md").read_text(encoding="utf-8").lower().split()
    )
    assert "public redistribution is not authorized" in manifest
    assert "no public release license" in readme


def test_mata_uses_valid_noncolliding_profile_timers() -> None:
    runtime = (ROOT / "kss_bc.mata").read_text(encoding="utf-8")
    timer_ids = {
        int(value)
        for value in re.findall(r"timer_(?:clear|on|off|value)\((\d+)\)", runtime)
    }
    assert timer_ids == {91, 92, 93, 94, 95}
    assert all(1 <= value <= 100 for value in timer_ids)


def test_production_finite_projection_uses_mixed_coefficient_one() -> None:
    runtime = re.sub(
        r"\s+", "", (ROOT / "kss_bc.mata").read_text(encoding="utf-8")
    )
    mixed_term = "(m_constrained-p_constrained):*mixed_second"
    assert runtime.count(mixed_term) == 1
    assert f"2:*{mixed_term}" not in runtime


def test_final_corrected_target_subtraction_is_fail_closed() -> None:
    runtime = re.sub(
        r"\s+", "", (ROOT / "kss_bc.mata").read_text(encoding="utf-8")
    )
    assert runtime.count("corrected=plugin-correction") == 2
    gate = (
        'if(hasmissing(corrected)){return(kssbc__failure('
        '"NONFINITE_CORRECTED_TARGET"'
    )
    assert runtime.count(gate) == 2
    assert runtime.count("out.corrected=corrected") == 2


def test_observation_frequency_tracks_literal_physical_copy_moments() -> None:
    runtime = re.sub(
        r"\s+", "", (ROOT / "kss_bc.mata").read_text(encoding="utf-8")
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
        r"\s+", "", (ROOT / "kss_bc.mata").read_text(encoding="utf-8")
    )
    assert (
        "out.relres=kssbc__max_column_relres(residual,right_hand_side)"
        in runtime
    )
    assert (
        "out.relres=kssbc__norm2(residual)/(1+kssbc__norm2(right_hand_side))"
        not in runtime
    )
    assert "out.relres=kssbc__max_column_relres(residual,I(dimension))" in runtime
    assert (
        "out.preparation_relres=kssbc__max_column_relres("
        "schur-checked_schur,checked_schur)" in runtime
    )
    assert "tolerance*reduced_scale" in runtime
    assert "tolerance*(1+reduced_scale)" not in runtime
    assert "out.relres=kssbc__norm2(full_residual)/full_scale" in runtime
    assert "full_firm_rhs=firm_rhs\\(sum(worker_rhs)-sum(firm_rhs))" in runtime
    assert "firm_coefficient=firm_coefficient:-normalization" in runtime
    assert "fitted=(firm_coefficient\\0)[design.firm]" not in runtime
    assert "out.controls=orthonormal*anchor'*anchor_inverse.inverse" in runtime


def test_only_factor_metadata_can_remove_an_omitted_control() -> None:
    runtime = (ROOT / "kss_bc.ado").read_text(encoding="utf-8")
    assert "fvexpand `controls' if `touse'" in runtime
    assert "_ms_parse_parts `term'" in runtime
    assert "if !r(omit) local controlvars" in runtime
    assert "count if `control' != 0" not in runtime


def test_rank_certificate_subtracts_numerical_margin() -> None:
    runtime = re.sub(
        r"\s+", "", (ROOT / "kss_bc.mata").read_text(encoding="utf-8")
    )
    assert (
        "out.gap=min((1-out.max_loss,minimum_deleted_eigen))-"
        "whitening_error-threshold" in runtime
    )
    assert "centered=controls-cell_mean[cell_of_row,.]" in runtime
    assert "transformed=centered*whitener" in runtime
    assert "deleted_inverse=kssbc__inverse(deleted_within,rank_tolerance)" in runtime
    assert "min((1-out.max_loss,minimum_deleted_eigen))" in runtime
    assert (
        "deleted_information_inverse=kssbc__inverse("
        "deleted_information,rank_tolerance)" in runtime
    )
    assert "inverse_forward_bound>=0.01" in runtime
    assert "solver_residual=max((solver_residual,maker_inverse.relres))" in runtime


def test_scc_harness_is_project_scoped_and_public_only() -> None:
    scc = ROOT / "benchmarks" / "scc"
    submit = (scc / "submit_one.sh").read_text(encoding="utf-8")
    portability = (scc / "run_portability.sge").read_text(encoding="utf-8")
    job_scripts = [
        (scc / name).read_text(encoding="utf-8")
        for name in ("run_portability.sge", "run_oracle.sge", "run_scale.sge")
    ]
    assert "qsub -terse -P welfgr" in submit
    assert submit.count("-pe omp 4") == 3
    assert all("#$ -P welfgr" in script for script in job_scripts)
    assert all("#$ -pe omp 4" in script for script in job_scripts)
    executable_harness = submit + "\n" + "\n".join(job_scripts)
    assert "application/data" not in executable_harness
    assert "separations" not in executable_harness.lower()
    assert '> "$job_dir/portability.pass"' in portability
