from __future__ import annotations

from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SOLVER = (ROOT / "kss_bc_solver.mata").read_text(encoding="utf-8")


def _between(start: str, stop: str) -> str:
    left = SOLVER.index(start)
    right = SOLVER.index(stop, left)
    return SOLVER[left:right]


def test_route_result_retains_fixed_pilot_evidence_schema() -> None:
    assert "real scalar kssbc_solver__pilot_api()" in SOLVER
    assert "return(1)" in _between(
        "real scalar kssbc_solver__pilot_api()",
        "struct kssbc_route_result",
    )
    route_struct = _between(
        "struct kssbc_route_result\n{",
        "struct kssbc_solver_pilot_evidence",
    )
    assert "real matrix pilot_diagnostics" in route_struct
    assert "string rowvector pilot_status" in route_struct
    assert "string rowvector pilot_failure_reason" in route_struct
    assert "out.pilot_diagnostics = J(8,16,.)" in SOLVER
    assert 'out.pilot_status = J(1,8,"NOT_RUN")' in SOLVER
    assert 'out.pilot_failure_reason = J(1,8,"not attempted")' in SOLVER


def test_malformed_and_complete_residual_failures_are_typed() -> None:
    helper = _between(
        "kssbc_solver__pilot_evidence(\n",
        "struct kssbc_solver_cmg_context",
    )
    assert 'out.status[pilot] = "MALFORMED_DIAGNOSTICS"' in helper
    assert 'out.diagnostics[pilot,16] = 2' in helper
    assert '"complete original-system residual is missing"' in helper
    assert '"complete original-system residual exceeds the gate"' in helper
    assert 'out.diagnostics[pilot,16] = 8' in helper
    assert "residual_gate = max((1e-11,10*tolerance))" in helper
    assert 'solved.status + "/" + rhs_status' in helper
    assert '"; RHS status " + rhs_status' in helper


def test_b1_and_cmg_evidence_is_captured_before_decisions() -> None:
    routed = _between(
        "struct kssbc_route_result scalar kssbc_solver__jla_routed(",
        "void kssbc__stata_jla_routed(",
    )
    diagonal_solve = routed.index(
        "diagonal_pilots = kssbc__fe_solve_matrix_backend("
    )
    diagonal_snapshot = routed.index(
        "diagonal_evidence = kssbc_solver__pilot_evidence("
    )
    diagonal_decision = routed.index(
        "diagonal_valid = kssbc_solver__pilots_valid("
    )
    assert diagonal_solve < diagonal_snapshot < diagonal_decision

    cmg_solve = routed.index("cmg_pilots = kssbc__fe_solve_matrix_backend(")
    cmg_snapshot = routed.index("cmg_evidence = kssbc_solver__pilot_evidence(")
    cmg_decision = routed.index("cmg_valid = kssbc_solver__pilots_valid(")
    assert cmg_solve < cmg_snapshot < cmg_decision

    failure = routed.index('"NO_REALISTIC_SOLVER_ROUTE"')
    estimator_rng_boundary = routed.index("out.estimator = kssbc__jla_backend(")
    assert diagonal_snapshot < cmg_snapshot < failure < estimator_rng_boundary


def test_optional_stata_bridge_preserves_existing_callers() -> None:
    bridge = _between("void kssbc__stata_jla_routed(\n", "\nend")
    assert "| string scalar pilot_diagnostics_name" in bridge
    assert "if (args() >= 33)" in bridge
    assert "if (args() >= 34)" in bridge
    assert "if (args() >= 35)" in bridge
    assert "if (args() >= 31)" not in bridge
    assert "if (args() >= 32)" not in bridge
    assert "st_matrix(pilot_diagnostics_name,routed.pilot_diagnostics)" in bridge
    assert 'invtokens(routed.pilot_status,"|")' in bridge
    assert 'invtokens(routed.pilot_failure_reason,"|")' in bridge


def test_work_and_action_evidence_uses_existing_route_model() -> None:
    helper = _between(
        "kssbc_solver__pilot_evidence(\n",
        "struct kssbc_solver_cmg_context",
    )
    assert "kssbc_solver__diagonal_work(" in helper
    assert "kssbc_solver__cmg_work(" in helper
    assert "kssbc_solver__fallback_work_ok(work)" in helper
    assert "out.diagnostics[pilot,9] = solved.schur_actions" in helper
    assert (
        "out.diagnostics[pilot,10] = solved.preconditioner_applications"
        in helper
    )
