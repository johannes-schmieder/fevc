from __future__ import annotations

from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SOLVER = (ROOT / "fevc_solver.mata").read_text(encoding="utf-8")
ADO = (ROOT / "fevc.ado").read_text(encoding="utf-8")


def _production_route() -> str:
    start = SOLVER.index(
        "struct vckss_route_result scalar vckss_solver__jla_routed("
    )
    stop = SOLVER.index("void vckss__stata_jla_routed(", start)
    return SOLVER[start:stop]


def test_installed_route_is_structural_and_has_no_trial_solves() -> None:
    routed = _production_route()
    assert "vckss_cmg__preflight(" in routed
    assert "vckss_solver__hierarchy_cells(" in routed
    assert "vckss__diagonal_backend()" in routed
    assert "vckss_solver__cmg_backend(" in routed
    for forbidden in (
        "pilot_rhs",
        "diagonal_pilots",
        "cmg_pilots",
        "projected-work",
        "NO_REALISTIC_SOLVER_ROUTE",
        "vckss_solver__fallback_work_ok",
        "vckss_solver__cmg_work_wins",
    ):
        assert forbidden not in routed


def test_auto_fallback_and_forced_cmg_are_typed_before_rng() -> None:
    routed = _production_route()
    forced = routed.index('requested_route == "CMG"')
    forced_failure = routed.index('"FORCED_CMG_FAILED"', forced)
    fallback = routed.index('out.route = "DIAGONAL"', forced_failure)
    estimator = routed.index("out.estimator = (*estimator_callback)")
    assert forced < forced_failure < fallback < estimator
    assert 'out.fallback_status = cmg_failure_status' in routed
    assert 'out.fallback_message = cmg_failure_message' in routed


def test_direct_memory_gate_precedes_estimator_rng() -> None:
    routed = _production_route()
    resource = routed.index("!vckss_solver__resource_apply(forecast_peak)")
    local_budget = routed.index("forecast_peak > solver_memory_bytes")
    estimator = routed.index("out.estimator = (*estimator_callback)")
    assert resource < local_budget < estimator


def test_active_public_diagnostics_reserve_retired_pilot_columns() -> None:
    routed = _production_route()
    assert "preflight.predicted_scratch_bytes,.,.,.,.,hierarchy_seconds,.,.," in (
        "".join(routed.split())
    )
    assert 'ereturn local route_api "KSS-ROUTE-STRUCTURAL-V1"' in ADO
    assert "ereturn matrix route_diagnostics" in ADO
    assert "ereturn matrix route_pilot_diagnostics" not in ADO
    assert "ereturn scalar route_pilot_iterations" not in ADO


def test_historical_pilot_router_is_not_the_installed_entrypoint() -> None:
    assert "vckss_solver__pilot_legacy(" in SOLVER
    assert SOLVER.index("vckss_solver__pilot_legacy(") < SOLVER.index(
        "vckss_solver__jla_routed("
    )
    bridge = SOLVER[SOLVER.index("void vckss__stata_jla_routed(") :]
    assert "routed = vckss_solver__jla_routed(" in bridge
    assert "solver__pilot_legacy" not in bridge
