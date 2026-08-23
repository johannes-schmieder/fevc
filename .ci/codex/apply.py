from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    target = Path(path)
    text = target.read_text(encoding="utf-8")
    count = text.count(old)
    if count != 1:
        raise RuntimeError(f"expected exactly one match in {path}, found {count}")
    target.write_text(text.replace(old, new, 1), encoding="utf-8")


replace_once(
    "rust/crates/vckss-core/src/engine.rs",
    """    pub requested_solver_route: LinearSolverRoute,
    pub selected_solver_route: LinearSolverRoute,
    pub solver_setup: PreparedSolverReceipt,
""",
    """    pub requested_solver_route: LinearSolverRoute,
    pub selected_solver_route: LinearSolverRoute,
    pub planned_rhs: u64,
    pub solver_setup: PreparedSolverReceipt,
""",
)

replace_once(
    "rust/crates/vckss-core/src/engine.rs",
    """    let mut selected = estimator;
    selected.leverage_batch_width = batch.leverage_active_width;
    selected.target_batch_width = batch.target_active_width;
    let mut forecast_selected = selected;
""",
    """    let mut selected = estimator;
    selected.leverage_batch_width = batch.leverage_active_width;
    selected.target_batch_width = batch.target_active_width;
    let planned_rhs = u64::from(selected.probes)
        .checked_mul(3)
        .and_then(|value| value.checked_add(1))
        .ok_or_else(|| {
            BackendError::new(
                ErrorCode::ResourceLimit,
                "jla_plan",
                "compressed planned RHS count overflow",
            )
        })?;
    let mut forecast_selected = selected;
""",
)

replace_once(
    "rust/crates/vckss-core/src/engine.rs",
    """            requested_solver_route: selected.solver.route,
            selected_solver_route,
            solver_setup,
            batch,
""",
    """            requested_solver_route: selected.solver.route,
            selected_solver_route,
            planned_rhs,
            solver_setup,
            batch,
""",
)

replace_once(
    "rust/crates/vckss-core/src/engine.rs",
    """        assert_eq!(
            automatic.execution.selected_engine,
            SelectedEngine::Compressed
        );
        assert_eq!(automatic.execution.batch.leverage_active_width, 32);
""",
    """        assert_eq!(
            automatic.execution.selected_engine,
            SelectedEngine::Compressed
        );
        assert_eq!(
            automatic.execution.planned_rhs,
            1 + 3 * u64::from(estimator.probes)
        );
        assert_eq!(automatic.execution.batch.leverage_active_width, 32);
""",
)

replace_once(
    "rust/crates/vckss-plugin/src/ffi_engine.rs",
    """            applicability: VCKSS_PLAN_APPLICABILITY_COMPRESSED,
            full_solver_dimension: to_u64(
""",
    """            applicability: VCKSS_PLAN_APPLICABILITY_COMPRESSED,
            planned_rhs: execution.planned_rhs,
            full_solver_dimension: to_u64(
""",
)

replace_once(
    "rust/crates/vckss-plugin/tests/engine_ffi.rs",
    """        assert_eq!(detailed.execution, plan);
        assert_eq!(detailed.v6.engine_selected, expected_engine);
""",
    """        assert_eq!(
            plan.solver.planned_rhs,
            detailed.v6.v5.v4.v3.rhs_receipt_rows
        );
        assert_eq!(detailed.execution, plan);
        assert_eq!(detailed.v6.engine_selected, expected_engine);
""",
)
