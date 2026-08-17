from __future__ import annotations

from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
RESOURCE = ROOT / "kss_bc_resource.mata"
SOLVER = ROOT / "kss_bc_solver.mata"
ADO = ROOT / "kss_bc.ado"
STATA_TEST = ROOT / "tests" / "stata" / "test_scale_resource.do"


def test_resource_module_models_every_registered_allocation_family() -> None:
    source = RESOURCE.read_text(encoding="utf-8")
    for field in (
        "runtime_resident_bytes",
        "raw_stata_bytes",
        "cell_bytes",
        "deletion_unit_bytes",
        "target_stratum_bytes",
        "cmg_hierarchy_bytes",
        "phase_scratch_bytes",
        "sorting_compression_bytes",
        "solve_ahead_bytes",
        "output_certificate_bytes",
        "preservation_transition_bytes",
    ):
        assert f"real scalar {field}" in source
        assert f'"{field}"' in source


def test_overlap_model_has_four_distinct_lifecycle_peaks() -> None:
    source = RESOURCE.read_text(encoding="utf-8")
    for field in (
        "selection_peak_bytes",
        "transition_peak_bytes",
        "numerical_peak_bytes",
        "restoration_peak_bytes",
    ):
        assert source.count(field) >= 4
    assert '(route == "generic")*components.raw_stata_bytes' in source
    assert "out.peak_bytes = max(peaks)" in source
    assert 'out.peak_phase = "transition"' in source
    assert 'out.peak_phase = "numerical"' in source
    assert "kssbc_resource__nonsolver_peak" in source
    assert "components.runtime_resident_bytes" in source
    assert "out.non_solver_numerical_bytes+out.routed_solver_peak_bytes" in (
        "".join(source.split())
    )


def test_admission_uses_hard_limits_and_required_headroom_before_rng() -> None:
    source = RESOURCE.read_text(encoding="utf-8")
    compact = "".join(source.split())
    assert "return(56*kssbc_resource__gib())" in compact
    assert "return(12*60*60)" in compact
    assert "return(0.50)" in compact
    assert "return(96*1024^2)" in compact
    assert "22*n_rows" in compact
    assert "memory_headroom_fraction<0.25" in compact
    assert "memory_headroom_fraction>0.30" in compact
    assert "ceil(out.peak_bytes*(1+memory_headroom_fraction))" in compact
    assert (
        "ceil(wall_forecast_upper_seconds*"
        "(1+kssbc_resource__wall_margin()))"
    ) in compact
    assert "out.before_rng=1" in compact


def test_generic_fallback_has_independent_typed_resource_gate() -> None:
    source = RESOURCE.read_text(encoding="utf-8")
    selector = source[source.index("kssbc_resource__select_auto(") :]
    assert 'out.status = "RESOURCE_ADMISSION_FAILED"' in source
    assert 'out.status = "GENERIC_RESOURCE_ADMISSION_FAILED"' in source
    assert 'out.route = "generic"' in selector
    assert "out.status = generic.status" in selector
    assert "out.admitted = generic.admitted" in selector
    assert "if (compressed_eligible)" in selector


def test_reconciliation_blocks_scale_progression_after_underforecast() -> None:
    source = RESOURCE.read_text(encoding="utf-8")
    reconcile = source[source.index("kssbc_resource__reconcile(") :]
    assert "qacct_maxvmem_bytes" in reconcile
    assert "out.comparison_peak_bytes = max" in reconcile
    assert 'out.status = "FORECAST_UNDERESTIMATED"' in reconcile
    assert "out.next_scale_allowed =" in reconcile
    assert "out.within_phase_forecasts &" in reconcile
    assert "out.within_memory_forecast &" in reconcile
    assert "out.within_wall_forecast &" in reconcile


def test_stata_test_covers_boundary_and_route_cases() -> None:
    test = STATA_TEST.read_text(encoding="utf-8")
    for token in (
        "55*gib",
        "8*60*60+1",
        '"RESOURCE_ADMISSION_FAILED"',
        '"GENERIC_RESOURCE_ADMISSION_FAILED"',
        '"FORECAST_UNDERESTIMATED"',
        "kssbc_resource__select_auto(0",
        "kssbc_resource__select_auto(1",
        "kssbc_resource__route_reconcile",
        "chunk_heavy.rng_total_calls == 80000000",
    ):
        assert token in test


def test_physical_mass_and_registered_rng_calls_enter_scale_model() -> None:
    source = RESOURCE.read_text(encoding="utf-8")
    compact = "".join(source.split())
    for field in (
        "physical_scale",
        "n_physical",
        "leverage_rng_calls_per_probe",
        "target_rng_calls_per_probe",
        "rng_call_seconds_upper",
        "rng_total_calls",
        "rng_wall_upper_seconds",
    ):
        assert f"real scalar {field}" in source
    assert "n_physical<n_rows" in compact
    assert "n_physical>2^53-1" in compact
    assert "out.rng_total_calls=probes*rng_calls_per_probe" in compact
    assert (
        "out.rng_wall_upper_seconds="
        "out.rng_total_calls*rng_call_seconds_upper"
    ) in compact
    assert "max((out.row_scale,out.physical_scale," in source
    assert "14*n_rows+12*parameters+n_physical" in source


def test_final_route_admission_uses_actual_solver_peak_and_thirty_percent() -> None:
    source = RESOURCE.read_text(encoding="utf-8")
    compact = "".join(source.split())
    routed = compact[compact.index("kssbc_resource__route_reconcile(") :]
    assert "routed_solver_peak_bytes" in routed
    assert (
        "out.numerical_peak_bytes="
        "out.non_solver_numerical_bytes+routed_solver_peak_bytes"
    ) in routed
    assert (
        "out.components.cmg_hierarchy_bytes=routed_solver_peak_bytes"
    ) in routed
    assert (
        "out.component_bytes=kssbc_resource__component_vector(out.components)"
    ) in routed
    assert "out.memory_headroom_fraction=0.30" in routed
    assert "out.memory_admission_bytes=ceil(1.30*out.peak_bytes)" in routed
    assert "out.memory_admission_bytes<=out.hard_memory_bytes" in routed
    assert "out.before_rng=1" in compact


def test_solver_enforces_whole_command_gate_before_estimator_rng() -> None:
    source = SOLVER.read_text(encoding="utf-8")
    compact = "".join(source.split())
    assert "return(22)" in source
    assert "kss-bc-solver-api22-runtime-residency-receipt" in source
    assert (
        "floor(KSSBC_SOLVER_RESOURCE_GATE.hard_memory_bytes/1.30)-"
        "KSSBC_SOLVER_RESOURCE_GATE.non_solver_numerical_bytes"
    ) in compact
    assert "return(0.65*memory_envelope_bytes)" in compact
    gate = source.index("!kssbc_solver__resource_apply(forecast_peak)")
    callback = source.index("if (use_callback)", gate)
    backend = source.index("out.estimator = kssbc__jla_backend(", callback)
    assert gate < callback < backend


def test_ado_passes_physical_rng_and_final_route_receipts() -> None:
    source = ADO.read_text(encoding="utf-8")
    compact = "".join(source.split())
    assert "kssbc_resource__api_level()==5" in compact
    assert "kss-bc-resource-api5-transition-highwater" in source
    assert "`N_retained',`retained_physical'" in compact
    assert "`leverage_rng_calls_per_probe'" in source
    assert "`target_rng_calls_per_probe'" in source
    assert "kssbc_resource__rng_call_upper()" in source
    assert "kssbc_solver__stata_res_rcpt(" in source
    assert '"`resource_components\'"' in source
    assert "resource_routed_solver_bytes" in source
    assert '"RAW_MEMORY_MEASUREMENT_FAILED"' in source
    assert "max(128*1024^2,160*`N_retained')" not in source
    assert "physical_scale physical_observations" in source
    assert "resource_rng_total_calls" in source
    assert "resource_runtime_resident_bytes" in source
    assert "_kss_bc_lifecycle_phase" in source


def test_route_receipt_atomically_reconciles_components_and_forecast() -> None:
    source = SOLVER.read_text(encoding="utf-8")
    compact = "".join(source.split())
    receipt = compact[compact.index("voidkssbc_solver__stata_res_rcpt(") :]
    assert "stringscalarcomponents_name" in receipt
    assert "cols(components)==11" in receipt
    assert (
        "components[forecast_row,5]="
        "KSSBC_SOLVER_RESOURCE_GATE.routed_solver_peak_bytes"
    ) in receipt
    assert "components[forecast_row,(2,3,4,5,6,8,9,11)]" in receipt
    assert 'KSSBC_SOLVER_RESOURCE_GATE.route=="generic"' in receipt
    assert "reconstructed_numerical-forecast_vector[3]" in receipt
    assert "st_matrix(components_name,components)" in receipt
    assert "st_matrix(forecasts_name,forecasts)" in receipt


def test_generic_complete_residual_uses_emitted_rhs_certificates() -> None:
    solver = SOLVER.read_text(encoding="utf-8")
    ado = ADO.read_text(encoding="utf-8")
    compact = "".join(ado.split())
    assert "realscalarkssbc_solver__rhs_resid_max(" in "".join(
        solver.split()
    )
    assert "kssbc_solver__rhs_resid_max(" in compact
    assert 'st_matrix("`solver_rhs_diagnostics\'")' in compact
    assert "ereturnscalarcomplete_residual_max=" in compact
    assert "`generic_complete_resid'" in compact
    assert "ereturnscalarsolver_max_residual=`diagnostics'[1,12]" in compact
