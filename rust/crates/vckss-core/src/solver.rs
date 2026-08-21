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
use crate::operator::{TwoWayOperator, TwoWaySolution};
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
    let requested = options.route;
    let selected = route_decision(&operator, options)?;
    match selected {
        LinearSolverRoute::Exact => solve_exact(
            &operator,
            worker_rhs,
            firm_rhs,
            options,
            requested,
            None,
        ),
        LinearSolverRoute::DiagonalPcg => solve_diagonal(
            &operator,
            worker_rhs,
            firm_rhs,
            options,
            requested,
            None,
        ),
        LinearSolverRoute::CmgPcg if requested == LinearSolverRoute::Auto => {
            solve_auto_cmg(&operator, worker_rhs, firm_rhs, options)
        }
        LinearSolverRoute::CmgPcg => solve_cmg(
            &operator,
            worker_rhs,
            firm_rhs,
            options,
            requested,
            None,
        ),
        LinearSolverRoute::Auto => Err(BackendError::invariant(
            "solver_router",
            "route decision returned unresolved automatic routing",
        )),
    }
}

fn route_decision(
    operator: &TwoWayOperator<'_>,
    options: LinearSolverOptions,
) -> Result<LinearSolverRoute> {
    let parameters = operator.firm_quotient_parameter_count();
    match options.route {
        LinearSolverRoute::Auto if parameters <= options.exact_dimension_limit => {
            Ok(LinearSolverRoute::Exact)
        }
        LinearSolverRoute::Auto if parameters < options.cmg_minimum_dimension => {
            Ok(LinearSolverRoute::DiagonalPcg)
        }
        LinearSolverRoute::Auto => Ok(LinearSolverRoute::CmgPcg),
        LinearSolverRoute::Exact if parameters > options.exact_dimension_limit => {
            Err(BackendError::new(
                ErrorCode::ResourceLimit,
                "solver_router",
                format!(
                    "forced exact solve dimension {parameters} exceeds the configured limit {}",
                    options.exact_dimension_limit
                ),
            ))
        }
        route => Ok(route),
    }
}

fn solve_auto_cmg(
    operator: &TwoWayOperator<'_>,
    worker_rhs: &[f64],
    firm_rhs: &[f64],
    options: LinearSolverOptions,
) -> Result<RoutedTwoWaySolve> {
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
            dimension: operator.firm_quotient_parameter_count(),
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
            dimension: operator.firm_quotient_parameter_count(),
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
    use crate::jla::fitted_values;
    use crate::operator::SymmetricOperator;
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
                let local_perturbation = if worker_index == 0 && offset == 0 {
                    0.5
                } else {
                    0.0
                };
                outcome.push(
                    sign * f64::from(u32::try_from(offset + 1).expect("offset"))
                        + local_perturbation,
                );
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

    fn uneven_problem(firm_label: [u64; 3]) -> CompressedProblem {
        let firm_concept = [0_usize, 1, 0, 2, 1, 2, 0, 1, 2, 0, 2];
        let rows = firm_concept.len();
        CanonicalInput::from_validated(
            InputColumns {
                worker: vec![1, 1, 2, 2, 3, 3, 4, 4, 4, 5, 5],
                firm: firm_concept
                    .iter()
                    .map(|&concept| firm_label[concept])
                    .collect(),
                deletion: (1..=u64::try_from(rows).expect("rows")).collect(),
                outcome: vec![2.4, -0.3, 1.1, 4.2, -1.7, 0.8, 3.3, -2.2, 0.4, 5.1, -0.6],
                frequency: vec![2, 1, 1, 4, 3, 1, 2, 5, 1, 7, 2],
                target_weight: vec![1.0; rows],
                controls: Vec::new(),
            }
            .validate()
            .expect("uneven fixture"),
        )
        .expect("canonical uneven fixture")
        .compress(&vec![true; rows])
        .expect("compressed uneven fixture")
    }

    fn routing_problem(firms: usize) -> CompressedProblem {
        let mut worker = Vec::with_capacity(2 * firms);
        let mut firm = Vec::with_capacity(2 * firms);
        let mut deletion = Vec::with_capacity(2 * firms);
        for worker_index in 0..firms {
            for offset in 0..2 {
                worker.push(u64::try_from(worker_index + 1).expect("worker"));
                firm.push(
                    u64::try_from((worker_index + offset) % firms + 1).expect("firm"),
                );
                deletion.push(
                    u64::try_from(2 * worker_index + offset + 1).expect("deletion"),
                );
            }
        }
        let rows = worker.len();
        CanonicalInput::from_validated(
            InputColumns {
                worker,
                firm,
                deletion,
                outcome: vec![0.0; rows],
                frequency: vec![1; rows],
                target_weight: vec![1.0; rows],
                controls: Vec::new(),
            }
            .validate()
            .expect("routing fixture"),
        )
        .expect("canonical routing fixture")
        .compress(&vec![true; rows])
        .expect("compressed routing fixture")
    }

    fn dense_zero_sum_predictions(problem: &CompressedProblem) -> Vec<f64> {
        let coefficients = problem.workers() + problem.firms();
        let dimension = coefficients + 1;
        let constraint = coefficients;
        let mut matrix = vec![0.0; dimension * dimension];
        let mut rhs = vec![0.0; dimension];
        for row in 0..problem.outcome.len() {
            let worker = usize::try_from(problem.row_worker[row]).expect("worker");
            let firm = problem.workers()
                + usize::try_from(problem.row_firm[row]).expect("firm");
            let weight = problem.frequency[row] as f64;
            let weighted_outcome = weight * problem.outcome[row];
            matrix[worker * dimension + worker] += weight;
            matrix[firm * dimension + firm] += weight;
            matrix[worker * dimension + firm] += weight;
            matrix[firm * dimension + worker] += weight;
            rhs[worker] += weighted_outcome;
            rhs[firm] += weighted_outcome;
        }
        for firm in 0..problem.firms() {
            let coordinate = problem.workers() + firm;
            matrix[coordinate * dimension + constraint] = 1.0;
            matrix[constraint * dimension + coordinate] = 1.0;
        }
        let coefficient = dense_solve(matrix, rhs);
        (0..problem.outcome.len())
            .map(|row| {
                let worker = usize::try_from(problem.row_worker[row]).expect("worker");
                let firm = problem.workers()
                    + usize::try_from(problem.row_firm[row]).expect("firm");
                coefficient[worker] + coefficient[firm]
            })
            .collect()
    }

    fn dense_solve(mut matrix: Vec<f64>, mut rhs: Vec<f64>) -> Vec<f64> {
        let dimension = rhs.len();
        assert_eq!(matrix.len(), dimension * dimension);
        for column in 0..dimension {
            let pivot = (column..dimension)
                .max_by(|&left, &right| {
                    matrix[left * dimension + column]
                        .abs()
                        .total_cmp(&matrix[right * dimension + column].abs())
                })
                .expect("pivot");
            assert!(matrix[pivot * dimension + column].abs() > 1.0e-12);
            if pivot != column {
                for entry in 0..dimension {
                    matrix.swap(column * dimension + entry, pivot * dimension + entry);
                }
                rhs.swap(column, pivot);
            }
            let pivot_value = matrix[column * dimension + column];
            for row in (column + 1)..dimension {
                let factor = matrix[row * dimension + column] / pivot_value;
                matrix[row * dimension + column] = 0.0;
                for entry in (column + 1)..dimension {
                    matrix[row * dimension + entry] -=
                        factor * matrix[column * dimension + entry];
                }
                rhs[row] -= factor * rhs[column];
            }
        }
        let mut solution = vec![0.0; dimension];
        for row in (0..dimension).rev() {
            let mut value = rhs[row];
            for column in (row + 1)..dimension {
                value -= matrix[row * dimension + column] * solution[column];
            }
            solution[row] = value / matrix[row * dimension + row];
        }
        assert!(solution.iter().all(|value| value.is_finite()));
        solution
    }

    fn route_solution(
        problem: &CompressedProblem,
        route: LinearSolverRoute,
    ) -> (RoutedTwoWaySolve, Vec<f64>) {
        let operator = TwoWayOperator::new(problem).expect("operator");
        let (worker_rhs, firm_rhs) = operator.outcome_rhs().expect("RHS");
        let options = LinearSolverOptions {
            route,
            exact_dimension_limit: 64,
            pcg: PcgOptions {
                tolerance: 1.0e-13,
                maximum_iterations: 500,
                residual_replacement_interval: 7,
            },
            full_residual_tolerance: 1.0e-10,
            ..base_options()
        };
        let solve = solve_two_way_routed(problem, &worker_rhs, &firm_rhs, options)
            .expect("relabeling route must remain valid");
        let prediction = fitted_values(problem, &solve.solution.worker, &solve.solution.firm)
            .expect("prediction");
        (solve, prediction)
    }

    fn mapped_firm_residual(
        firm_label: [u64; 3],
        solve: &RoutedTwoWaySolve,
    ) -> Vec<f64> {
        let mut sorted = firm_label;
        sorted.sort_unstable();
        firm_label
            .iter()
            .map(|label| {
                let dense = sorted.iter().position(|candidate| candidate == label).expect("firm");
                solve.solution.residual.firm[dense]
            })
            .collect()
    }

    fn assert_close(left: &[f64], right: &[f64], tolerance: f64) {
        assert_eq!(left.len(), right.len());
        for (index, (&left, &right)) in left.iter().zip(right).enumerate() {
            assert!(
                (left - right).abs() <= tolerance,
                "entry {index} differs: {left} versus {right}"
            );
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
        let pcg = solve.receipt.pcg.as_ref().expect("PCG receipt");
        assert!(!pcg.zero_rhs);
        assert!(pcg.iterations > 0);
        assert!(pcg.operator_applications > 0);
        assert!(pcg.preconditioner_applications > 0);
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
    fn forced_exact_boundary_uses_f_minus_one_parameters() {
        let at_limit = routing_problem(501);
        let above_limit = routing_problem(502);
        let at_limit_operator = TwoWayOperator::new(&at_limit).expect("at-limit operator");
        let above_limit_operator = TwoWayOperator::new(&above_limit).expect("above-limit operator");
        let options = LinearSolverOptions {
            route: LinearSolverRoute::Exact,
            exact_dimension_limit: 500,
            ..base_options()
        };
        assert_eq!(at_limit_operator.dimension(), 501);
        assert_eq!(at_limit_operator.firm_quotient_parameter_count(), 500);
        assert_eq!(
            route_decision(&at_limit_operator, options).expect("F=501 exact admission"),
            LinearSolverRoute::Exact
        );
        let error = route_decision(&above_limit_operator, options)
            .expect_err("F=502 must exceed exact_limit=500");
        assert_eq!(error.code, ErrorCode::ResourceLimit);
    }

    #[test]
    fn automatic_exact_boundary_uses_f_minus_one_parameters() {
        let at_limit = routing_problem(501);
        let above_limit = routing_problem(502);
        let options = LinearSolverOptions {
            route: LinearSolverRoute::Auto,
            exact_dimension_limit: 500,
            cmg_minimum_dimension: 2_000,
            ..base_options()
        };
        assert_eq!(
            route_decision(&TwoWayOperator::new(&at_limit).expect("operator"), options)
                .expect("F=501 auto route"),
            LinearSolverRoute::Exact
        );
        assert_eq!(
            route_decision(&TwoWayOperator::new(&above_limit).expect("operator"), options)
                .expect("F=502 auto route"),
            LinearSolverRoute::DiagonalPcg
        );
    }

    #[test]
    fn automatic_cmg_boundary_uses_f_minus_one_parameters() {
        let below_cmg = routing_problem(2_000);
        let at_cmg = routing_problem(2_001);
        let options = LinearSolverOptions {
            route: LinearSolverRoute::Auto,
            exact_dimension_limit: 500,
            cmg_minimum_dimension: 2_000,
            ..base_options()
        };
        assert_eq!(
            route_decision(&TwoWayOperator::new(&below_cmg).expect("operator"), options)
                .expect("F=2000 auto route"),
            LinearSolverRoute::DiagonalPcg
        );
        assert_eq!(
            route_decision(&TwoWayOperator::new(&at_cmg).expect("operator"), options)
                .expect("F=2001 auto route"),
            LinearSolverRoute::CmgPcg
        );
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

    #[test]
    fn full_zero_sum_routes_are_stable_when_the_numeric_last_firm_changes() {
        let original_label = [10_u64, 20, 30];
        let relabeled = [30_u64, 10, 20];
        let original_last = original_label
            .iter()
            .enumerate()
            .max_by_key(|(_, label)| *label)
            .map(|(concept, _)| concept)
            .expect("last firm");
        let relabeled_last = relabeled
            .iter()
            .enumerate()
            .max_by_key(|(_, label)| *label)
            .map(|(concept, _)| concept)
            .expect("last firm");
        assert_ne!(original_last, relabeled_last);

        let original = uneven_problem(original_label);
        let permuted = uneven_problem(relabeled);
        let original_oracle = dense_zero_sum_predictions(&original);
        let permuted_oracle = dense_zero_sum_predictions(&permuted);
        assert_close(&original_oracle, &permuted_oracle, 1.0e-12);

        for route in [LinearSolverRoute::Exact, LinearSolverRoute::DiagonalPcg] {
            let (original_solve, original_prediction) = route_solution(&original, route);
            let (permuted_solve, permuted_prediction) = route_solution(&permuted, route);
            assert_eq!(original_solve.receipt.selected, route);
            assert_eq!(permuted_solve.receipt.selected, route);
            assert_eq!(original_solve.solution.reduced_firm.len(), original.firms());
            assert_eq!(permuted_solve.solution.reduced_firm.len(), permuted.firms());
            assert!(
                original_solve
                    .solution
                    .reduced_firm
                    .iter()
                    .copied()
                    .sum::<f64>()
                    .abs()
                    <= 1.0e-12
            );
            assert!(
                permuted_solve
                    .solution
                    .reduced_firm
                    .iter()
                    .copied()
                    .sum::<f64>()
                    .abs()
                    <= 1.0e-12
            );
            assert_close(&original_prediction, &original_oracle, 1.0e-10);
            assert_close(&permuted_prediction, &permuted_oracle, 1.0e-10);
            assert_close(&original_prediction, &permuted_prediction, 1.0e-10);
            assert_close(
                &original_solve.solution.residual.worker,
                &permuted_solve.solution.residual.worker,
                1.0e-10,
            );
            assert_close(
                &mapped_firm_residual(original_label, &original_solve),
                &mapped_firm_residual(relabeled, &permuted_solve),
                1.0e-10,
            );
            assert!(original_solve.solution.residual.relative_norm <= 1.0e-10);
            assert!(permuted_solve.solution.residual.relative_norm <= 1.0e-10);
            if route == LinearSolverRoute::DiagonalPcg {
                let original_pcg = original_solve.receipt.pcg.as_ref().expect("PCG receipt");
                let permuted_pcg = permuted_solve.receipt.pcg.as_ref().expect("PCG receipt");
                assert!(!original_pcg.zero_rhs && original_pcg.iterations > 0);
                assert!(!permuted_pcg.zero_rhs && permuted_pcg.iterations > 0);
            }
        }
    }
}
