// SPDX-License-Identifier: GPL-3.0-only

//! Deterministic routing for exact, diagonal-PCG, and CMG-PCG solves.
//!
//! Fallback is permitted only from automatic CMG setup to diagonal PCG and
//! only before an iterative solve begins. A numerical failure after the route
//! is selected is returned without changing the preconditioner or tolerance.

use crate::cmg::{CmgOptions, CmgPreconditioner, CmgReceipt};
use crate::error::{BackendError, ErrorCode, Result};
use crate::exact::{solve_two_way_exact, ExactSolveReceipt};
use crate::krylov::{pcg, DiagonalPreconditioner, PcgOptions, PcgReceipt, Preconditioner};
use crate::operator::{SymmetricOperator, TwoWayOperator, TwoWaySolution};
use crate::problem::CompressedProblem;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LinearSolverRoute {
    Auto,
    Exact,
    DiagonalPcg,
    CmgPcg,
}

#[derive(Clone, Copy, Debug)]
pub struct LinearSolverOptions {
    pub route: LinearSolverRoute,
    pub exact_dimension_limit: usize,
    pub cmg_minimum_dimension: usize,
    pub allow_automatic_cmg_setup_fallback: bool,
    pub pcg: PcgOptions,
    pub cmg: CmgOptions,
    pub full_residual_tolerance: f64,
}

impl Default for LinearSolverOptions {
    fn default() -> Self {
        Self {
            route: LinearSolverRoute::Auto,
            exact_dimension_limit: 500,
            cmg_minimum_dimension: 2_000,
            allow_automatic_cmg_setup_fallback: true,
            pcg: PcgOptions {
                tolerance: 1.0e-10,
                maximum_iterations: 10_000,
                residual_replacement_interval: 50,
            },
            cmg: CmgOptions::default(),
            full_residual_tolerance: 1.0e-9,
        }
    }
}

impl LinearSolverOptions {
    pub fn validate(self) -> Result<Self> {
        if self.exact_dimension_limit == 0 || self.cmg_minimum_dimension == 0 {
            return Err(BackendError::invalid(
                "solver_router",
                "exact and CMG routing dimensions must be positive",
            ));
        }
        self.pcg.validate()?;
        self.cmg.validate()?;
        if !self.full_residual_tolerance.is_finite() || self.full_residual_tolerance <= 0.0 {
            return Err(BackendError::invalid(
                "solver_router",
                "full residual tolerance must be positive and finite",
            ));
        }
        Ok(self)
    }
}

#[derive(Clone, Debug)]
pub struct SolverFallback {
    pub from: LinearSolverRoute,
    pub to: LinearSolverRoute,
    pub code: ErrorCode,
    pub message: String,
}

#[derive(Clone, Debug)]
pub struct RoutedSolveReceipt {
    pub requested: LinearSolverRoute,
    pub selected: LinearSolverRoute,
    pub dimension: usize,
    pub exact: Option<ExactSolveReceipt>,
    pub pcg: Option<PcgReceipt>,
    pub cmg: Option<CmgReceipt>,
    pub fallback: Option<SolverFallback>,
}

#[derive(Clone, Debug)]
pub struct RoutedTwoWaySolve {
    pub solution: TwoWaySolution,
    pub receipt: RoutedSolveReceipt,
}

pub fn solve_two_way_routed(
    problem: &CompressedProblem,
    worker_rhs: &[f64],
    firm_rhs: &[f64],
    options: LinearSolverOptions,
) -> Result<RoutedTwoWaySolve> {
    let options = options.validate()?;
    let operator = TwoWayOperator::new(problem)?;
    let dimension = operator.dimension();
    match options.route {
        LinearSolverRoute::Auto => solve_auto(&operator, worker_rhs, firm_rhs, options),
        LinearSolverRoute::Exact => {
            if dimension > options.exact_dimension_limit {
                return Err(BackendError::new(
                    ErrorCode::ResourceLimit,
                    "solver_router",
                    format!(
                        "forced exact solve dimension {dimension} exceeds the configured limit {}",
                        options.exact_dimension_limit
                    ),
                ));
            }
            solve_exact(
                &operator,
                worker_rhs,
                firm_rhs,
                options,
                LinearSolverRoute::Exact,
                None,
            )
        }
        LinearSolverRoute::DiagonalPcg => solve_diagonal(
            &operator,
            worker_rhs,
            firm_rhs,
            options,
            LinearSolverRoute::DiagonalPcg,
            None,
        ),
        LinearSolverRoute::CmgPcg => solve_cmg(
            &operator,
            worker_rhs,
            firm_rhs,
            options,
            LinearSolverRoute::CmgPcg,
            None,
        ),
    }
}

fn solve_auto(
    operator: &TwoWayOperator<'_>,
    worker_rhs: &[f64],
    firm_rhs: &[f64],
    options: LinearSolverOptions,
) -> Result<RoutedTwoWaySolve> {
    if operator.dimension() <= options.exact_dimension_limit {
        return solve_exact(
            operator,
            worker_rhs,
            firm_rhs,
            options,
            LinearSolverRoute::Auto,
            None,
        );
    }
    if operator.dimension() < options.cmg_minimum_dimension {
        return solve_diagonal(
            operator,
            worker_rhs,
            firm_rhs,
            options,
            LinearSolverRoute::Auto,
            None,
        );
    }

    match CmgPreconditioner::new(operator.problem(), options.cmg) {
        Ok(preconditioner) => solve_preconditioned(
            operator,
            worker_rhs,
            firm_rhs,
            &preconditioner,
            options,
            LinearSolverRoute::Auto,
            LinearSolverRoute::CmgPcg,
            Some(preconditioner.receipt().clone()),
            None,
        ),
        Err(error)
            if options.allow_automatic_cmg_setup_fallback && is_setup_fallback_error(&error) =>
        {
            let fallback = SolverFallback {
                from: LinearSolverRoute::CmgPcg,
                to: LinearSolverRoute::DiagonalPcg,
                code: error.code,
                message: error.to_string(),
            };
            solve_diagonal(
                operator,
                worker_rhs,
                firm_rhs,
                options,
                LinearSolverRoute::Auto,
                Some(fallback),
            )
        }
        Err(error) => Err(error),
    }
}

fn solve_exact(
    operator: &TwoWayOperator<'_>,
    worker_rhs: &[f64],
    firm_rhs: &[f64],
    options: LinearSolverOptions,
    requested: LinearSolverRoute,
    fallback: Option<SolverFallback>,
) -> Result<RoutedTwoWaySolve> {
    let result = solve_two_way_exact(
        operator,
        worker_rhs,
        firm_rhs,
        options.full_residual_tolerance,
    )?;
    Ok(RoutedTwoWaySolve {
        solution: result.solution,
        receipt: RoutedSolveReceipt {
            requested,
            selected: LinearSolverRoute::Exact,
            dimension: operator.dimension(),
            exact: Some(result.receipt),
            pcg: None,
            cmg: None,
            fallback,
        },
    })
}

fn solve_diagonal(
    operator: &TwoWayOperator<'_>,
    worker_rhs: &[f64],
    firm_rhs: &[f64],
    options: LinearSolverOptions,
    requested: LinearSolverRoute,
    fallback: Option<SolverFallback>,
) -> Result<RoutedTwoWaySolve> {
    let preconditioner = DiagonalPreconditioner::new(operator.reduced_diagonal())?;
    solve_preconditioned(
        operator,
        worker_rhs,
        firm_rhs,
        &preconditioner,
        options,
        requested,
        LinearSolverRoute::DiagonalPcg,
        None,
        fallback,
    )
}

fn solve_cmg(
    operator: &TwoWayOperator<'_>,
    worker_rhs: &[f64],
    firm_rhs: &[f64],
    options: LinearSolverOptions,
    requested: LinearSolverRoute,
    fallback: Option<SolverFallback>,
) -> Result<RoutedTwoWaySolve> {
    let preconditioner = CmgPreconditioner::new(operator.problem(), options.cmg)?;
    let receipt = preconditioner.receipt().clone();
    solve_preconditioned(
        operator,
        worker_rhs,
        firm_rhs,
        &preconditioner,
        options,
        requested,
        LinearSolverRoute::CmgPcg,
        Some(receipt),
        fallback,
    )
}

#[allow(clippy::too_many_arguments)]
fn solve_preconditioned(
    operator: &TwoWayOperator<'_>,
    worker_rhs: &[f64],
    firm_rhs: &[f64],
    preconditioner: &impl Preconditioner,
    options: LinearSolverOptions,
    requested: LinearSolverRoute,
    selected: LinearSolverRoute,
    cmg: Option<CmgReceipt>,
    fallback: Option<SolverFallback>,
) -> Result<RoutedTwoWaySolve> {
    let reduced_rhs = operator.schur_rhs(worker_rhs, firm_rhs)?;
    let reduced = pcg(operator, preconditioner, &reduced_rhs, options.pcg)?;
    let firm = operator.expand_firm(&reduced.solution)?;
    let worker = operator.reconstruct_worker(worker_rhs, &firm)?;
    let residual = operator.full_residual(&worker, &firm, worker_rhs, firm_rhs)?;
    if residual.relative_norm > options.full_residual_tolerance {
        return Err(BackendError::new(
            ErrorCode::FullResidualFailed,
            "solver_router",
            format!(
                "complete normal-equation residual {} exceeds tolerance {}",
                residual.relative_norm, options.full_residual_tolerance
            ),
        ));
    }
    Ok(RoutedTwoWaySolve {
        solution: TwoWaySolution {
            worker,
            firm,
            reduced_firm: reduced.solution,
            residual,
        },
        receipt: RoutedSolveReceipt {
            requested,
            selected,
            dimension: operator.dimension(),
            exact: None,
            pcg: Some(reduced.receipt),
            cmg,
            fallback,
        },
    })
}

fn is_setup_fallback_error(error: &BackendError) -> bool {
    matches!(
        error.code,
        ErrorCode::CmgSetupFailed | ErrorCode::ResourceLimit | ErrorCode::AllocationFailed
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::problem::CanonicalInput;
    use crate::types::InputColumns;

    fn fixture() -> CompressedProblem {
        let workers = 24_usize;
        let firms = 8_usize;
        let mut worker = Vec::with_capacity(workers * 4);
        let mut firm = Vec::with_capacity(workers * 4);
        let mut deletion = Vec::with_capacity(workers * 4);
        let mut outcome = Vec::with_capacity(workers * 4);
        for worker_index in 0..workers {
            for offset in 0..4 {
                worker.push(u64::try_from(worker_index + 1).expect("worker"));
                firm.push(
                    u64::try_from((worker_index + offset) % firms + 1).expect("firm"),
                );
                deletion.push(
                    u64::try_from(worker_index * 4 + offset + 1).expect("deletion"),
                );
                let sign = if offset % 2 == 0 { 1.0 } else { -1.0 };
                outcome.push(sign * f64::from(u32::try_from(offset + 1).expect("offset")));
            }
        }
        let rows = worker.len();
        CanonicalInput::from_validated(
            InputColumns {
                worker,
                firm,
                deletion,
                outcome,
                frequency: vec![1; rows],
                target_weight: vec![1.0; rows],
                controls: Vec::new(),
            }
            .validate()
            .expect("fixture"),
        )
        .expect("canonical")
        .compress(&vec![true; rows])
        .expect("compressed")
    }

    fn base_options() -> LinearSolverOptions {
        LinearSolverOptions {
            exact_dimension_limit: 32,
            cmg_minimum_dimension: 2,
            pcg: PcgOptions {
                tolerance: 1.0e-11,
                maximum_iterations: 500,
                residual_replacement_interval: 20,
            },
            cmg: CmgOptions {
                terminal_vertices: 8,
                dense_vertex_cap: 64,
                memory_limit_bytes: 64_u64 << 20,
                ..CmgOptions::default()
            },
            full_residual_tolerance: 1.0e-9,
            ..LinearSolverOptions::default()
        }
    }

    #[test]
    fn automatic_small_problem_uses_exact() {
        let problem = fixture();
        let operator = TwoWayOperator::new(&problem).expect("operator");
        let (worker_rhs, firm_rhs) = operator.outcome_rhs().expect("RHS");
        let solve = solve_two_way_routed(&problem, &worker_rhs, &firm_rhs, base_options())
            .expect("routed solve");
        assert_eq!(solve.receipt.requested, LinearSolverRoute::Auto);
        assert_eq!(solve.receipt.selected, LinearSolverRoute::Exact);
        assert!(solve.receipt.exact.is_some());
        assert!(solve.receipt.pcg.is_none());
    }

    #[test]
    fn forced_cmg_route_is_certified() {
        let problem = fixture();
        let operator = TwoWayOperator::new(&problem).expect("operator");
        let (worker_rhs, firm_rhs) = operator.outcome_rhs().expect("RHS");
        let options = LinearSolverOptions {
            route: LinearSolverRoute::CmgPcg,
            ..base_options()
        };
        let solve = solve_two_way_routed(&problem, &worker_rhs, &firm_rhs, options)
            .expect("CMG solve");
        assert_eq!(solve.receipt.selected, LinearSolverRoute::CmgPcg);
        assert!(solve.receipt.cmg.is_some());
        assert!(solve.receipt.pcg.is_some());
        assert!(solve.solution.residual.relative_norm <= options.full_residual_tolerance);
    }

    #[test]
    fn auto_falls_back_only_on_cmg_setup_failure() {
        let problem = fixture();
        let operator = TwoWayOperator::new(&problem).expect("operator");
        let (worker_rhs, firm_rhs) = operator.outcome_rhs().expect("RHS");
        let options = LinearSolverOptions {
            route: LinearSolverRoute::Auto,
            exact_dimension_limit: 1,
            cmg: CmgOptions {
                memory_limit_bytes: 1,
                ..base_options().cmg
            },
            ..base_options()
        };
        let solve = solve_two_way_routed(&problem, &worker_rhs, &firm_rhs, options)
            .expect("diagonal fallback");
        assert_eq!(solve.receipt.selected, LinearSolverRoute::DiagonalPcg);
        let fallback = solve.receipt.fallback.expect("fallback receipt");
        assert_eq!(fallback.from, LinearSolverRoute::CmgPcg);
        assert_eq!(fallback.to, LinearSolverRoute::DiagonalPcg);
        assert_eq!(fallback.code, ErrorCode::ResourceLimit);
    }

    #[test]
    fn forced_exact_respects_the_dimension_limit() {
        let problem = fixture();
        let operator = TwoWayOperator::new(&problem).expect("operator");
        let (worker_rhs, firm_rhs) = operator.outcome_rhs().expect("RHS");
        let options = LinearSolverOptions {
            route: LinearSolverRoute::Exact,
            exact_dimension_limit: 1,
            ..base_options()
        };
        let error = solve_two_way_routed(&problem, &worker_rhs, &firm_rhs, options)
            .expect_err("oversized exact solve must fail");
        assert_eq!(error.code, ErrorCode::ResourceLimit);
    }

    #[test]
    fn iterative_failure_does_not_change_route() {
        let problem = fixture();
        let operator = TwoWayOperator::new(&problem).expect("operator");
        let (worker_rhs, firm_rhs) = operator.outcome_rhs().expect("RHS");
        let options = LinearSolverOptions {
            route: LinearSolverRoute::CmgPcg,
            pcg: PcgOptions {
                tolerance: 1.0e-15,
                maximum_iterations: 1,
                residual_replacement_interval: 1,
            },
            ..base_options()
        };
        let error = solve_two_way_routed(&problem, &worker_rhs, &firm_rhs, options)
            .expect_err("one iteration must not silently reroute");
        assert!(matches!(
            error.code,
            ErrorCode::PcgMaxIterations
                | ErrorCode::PcgCurvatureBreakdown
                | ErrorCode::PcgPreconditionerBreakdown
        ));
    }
}
