// SPDX-License-Identifier: GPL-3.0-only

//! Deterministic routing for exact, diagonal-PCG, and CMG-PCG solves.
//!
//! Fallback is permitted only from automatic CMG setup to diagonal PCG and
//! only before an iterative solve begins. A numerical failure after the route
//! is selected is returned without changing the preconditioner or tolerance.

use crate::batch::{solve_two_way_pcg_batch_with_interrupt, TwoWayBatchedPcgSolve};
use crate::cmg::{CmgOptions, CmgPreconditioner, CmgReceipt};
use crate::error::{BackendError, ErrorCode, Result};
use crate::exact::{
    solve_two_way_exact_factored_with_interrupt, ExactFactorization, ExactSolveReceipt,
};
#[cfg(feature = "cmg-full-spike")]
use crate::full_cmg_spike::{FullCmgDirectSolver, FullCmgSpikePhase};
use crate::interrupt::{InterruptCheck, NeverInterrupt};
use crate::krylov::{
    pcg_with_interrupt, DiagonalPreconditioner, PcgOptions, PcgReceipt, Preconditioner,
};
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
                residual_replacement_interval: 100,
            },
            cmg: CmgOptions::default(),
            full_residual_tolerance: 1.0e-9,
        }
    }
}

impl LinearSolverOptions {
    #[must_use]
    pub fn required_full_residual_tolerance(self) -> f64 {
        (10.0 * self.pcg.tolerance).max(1.0e-11)
    }

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
        let required = self.required_full_residual_tolerance();
        if self.full_residual_tolerance != required {
            return Err(BackendError::invalid(
                "solver_router",
                format!(
                    "full residual tolerance {} must equal the required gate {required}",
                    self.full_residual_tolerance
                ),
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

#[derive(Clone, Debug)]
pub struct PreparedSolverReceipt {
    pub requested: LinearSolverRoute,
    pub selected: LinearSolverRoute,
    pub dimension: usize,
    pub cmg: Option<CmgReceipt>,
    pub fallback: Option<SolverFallback>,
}

#[derive(Clone, Debug)]
pub struct RoutedTwoWayBatchSolve {
    pub solution: Vec<TwoWaySolution>,
    pub receipt: Vec<RoutedSolveReceipt>,
}

#[derive(Debug)]
enum PreparedSolverBackend {
    Exact(ExactFactorization),
    Diagonal(DiagonalPreconditioner),
    Cmg(Box<CmgPreconditioner>),
    #[cfg(feature = "cmg-full-spike")]
    FullCmg(Box<FullCmgDirectSolver>),
}

/// A route-frozen two-way solver. Automatic CMG setup and its only permitted
/// fallback happen in `prepare`, so every subsequent right-hand side reuses
/// the identical operator, route, preconditioner, tolerance, and setup receipt.
#[derive(Debug)]
pub struct PreparedTwoWaySolver<'a> {
    operator: TwoWayOperator<'a>,
    options: LinearSolverOptions,
    receipt: PreparedSolverReceipt,
    backend: PreparedSolverBackend,
}

impl<'a> PreparedTwoWaySolver<'a> {
    pub fn prepare(problem: &'a CompressedProblem, options: LinearSolverOptions) -> Result<Self> {
        let mut interrupt = NeverInterrupt;
        Self::prepare_with_interrupt(problem, options, &mut interrupt)
    }

    pub fn prepare_with_interrupt(
        problem: &'a CompressedProblem,
        options: LinearSolverOptions,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<Self> {
        Self::prepare_impl(problem, options, interrupt, false)
    }

    #[cfg(feature = "cmg-full-spike")]
    pub(crate) fn prepare_full_cmg_spike_with_interrupt(
        problem: &'a CompressedProblem,
        options: LinearSolverOptions,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<Self> {
        Self::prepare_impl(problem, options, interrupt, true)
    }

    fn prepare_impl(
        problem: &'a CompressedProblem,
        options: LinearSolverOptions,
        interrupt: &mut dyn InterruptCheck,
        private_full_cmg: bool,
    ) -> Result<Self> {
        interrupt.checkpoint("solver_prepare_entry")?;
        let options = options.validate()?;
        let operator = TwoWayOperator::new_with_interrupt(problem, interrupt)?;
        let requested = options.route;
        let selected = route_decision(&operator, options)?;
        #[cfg(feature = "cmg-full-spike")]
        if private_full_cmg && selected != LinearSolverRoute::CmgPcg {
            return Err(BackendError::new(
                ErrorCode::UnsupportedFeature,
                "cmg_full_spike",
                "the private direct full-CMG route requires a frozen CMG-PCG selection",
            ));
        }
        #[cfg(not(feature = "cmg-full-spike"))]
        if private_full_cmg {
            return Err(BackendError::new(
                ErrorCode::UnsupportedFeature,
                "solver_router",
                "the private full-CMG spike was requested from an ordinary build",
            ));
        }
        let mut fallback = None;
        let (selected, backend, cmg) = match selected {
            LinearSolverRoute::Exact => {
                let factor = ExactFactorization::prepare_two_way(&operator, interrupt)?;
                (
                    LinearSolverRoute::Exact,
                    PreparedSolverBackend::Exact(factor),
                    None,
                )
            }
            LinearSolverRoute::DiagonalPcg => {
                let preconditioner = DiagonalPreconditioner::new(operator.reduced_diagonal())?;
                (
                    LinearSolverRoute::DiagonalPcg,
                    PreparedSolverBackend::Diagonal(preconditioner),
                    None,
                )
            }
            LinearSolverRoute::CmgPcg => {
                #[cfg(feature = "cmg-full-spike")]
                if private_full_cmg {
                    let direct = FullCmgDirectSolver::prepare_with_interrupt(
                        problem,
                        options.pcg,
                        options.cmg.memory_limit_bytes,
                        interrupt,
                    )?;
                    let receipt = direct.compatibility_receipt().clone();
                    let receipt = PreparedSolverReceipt {
                        requested,
                        selected: LinearSolverRoute::CmgPcg,
                        dimension: operator.firm_quotient_parameter_count(),
                        cmg: Some(receipt),
                        fallback: None,
                    };
                    return Ok(Self {
                        operator,
                        options,
                        receipt,
                        backend: PreparedSolverBackend::FullCmg(Box::new(direct)),
                    });
                }
                match CmgPreconditioner::new_with_interrupt(problem, options.cmg, interrupt) {
                    Ok(preconditioner) => {
                        let receipt = preconditioner.receipt().clone();
                        (
                            LinearSolverRoute::CmgPcg,
                            PreparedSolverBackend::Cmg(Box::new(preconditioner)),
                            Some(receipt),
                        )
                    }
                    Err(error)
                        if requested == LinearSolverRoute::Auto
                            && options.allow_automatic_cmg_setup_fallback
                            && is_setup_fallback_error(&error) =>
                    {
                        fallback = Some(SolverFallback {
                            from: LinearSolverRoute::CmgPcg,
                            to: LinearSolverRoute::DiagonalPcg,
                            code: error.code,
                            message: error.to_string(),
                        });
                        let preconditioner =
                            DiagonalPreconditioner::new(operator.reduced_diagonal())?;
                        (
                            LinearSolverRoute::DiagonalPcg,
                            PreparedSolverBackend::Diagonal(preconditioner),
                            None,
                        )
                    }
                    Err(error) => return Err(error),
                }
            }
            LinearSolverRoute::Auto => {
                return Err(BackendError::invariant(
                    "solver_router",
                    "route decision returned unresolved automatic routing",
                ));
            }
        };
        let receipt = PreparedSolverReceipt {
            requested,
            selected,
            dimension: operator.firm_quotient_parameter_count(),
            cmg,
            fallback,
        };
        Ok(Self {
            operator,
            options,
            receipt,
            backend,
        })
    }

    #[must_use]
    pub const fn operator(&self) -> &TwoWayOperator<'a> {
        &self.operator
    }

    #[must_use]
    pub const fn receipt(&self) -> &PreparedSolverReceipt {
        &self.receipt
    }

    #[must_use]
    pub fn maximum_full_residual_tolerance(&self) -> f64 {
        match &self.backend {
            #[cfg(feature = "cmg-full-spike")]
            PreparedSolverBackend::FullCmg(solver) => solver.maximum_complete_residual_tolerance(),
            _ => self.options.full_residual_tolerance,
        }
    }

    pub(crate) fn map_independent_ordered<Input, Output, Operation>(
        &self,
        input: Vec<Input>,
        operation: Operation,
    ) -> Vec<Output>
    where
        Input: Send,
        Output: Send,
        Operation: Fn(Input) -> Output + Send + Sync,
    {
        #[cfg(feature = "cmg-full-spike")]
        if let PreparedSolverBackend::FullCmg(solver) = &self.backend {
            return solver.map_independent_ordered(input, operation);
        }
        input.into_iter().map(operation).collect()
    }

    pub fn solve(&self, worker_rhs: &[f64], firm_rhs: &[f64]) -> Result<RoutedTwoWaySolve> {
        let mut interrupt = NeverInterrupt;
        self.solve_with_interrupt(worker_rhs, firm_rhs, &mut interrupt)
    }

    pub fn solve_with_interrupt(
        &self,
        worker_rhs: &[f64],
        firm_rhs: &[f64],
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<RoutedTwoWaySolve> {
        match &self.backend {
            PreparedSolverBackend::Exact(factor) => {
                let solved = solve_two_way_exact_factored_with_interrupt(
                    &self.operator,
                    factor,
                    worker_rhs,
                    firm_rhs,
                    self.options.full_residual_tolerance,
                    interrupt,
                )?;
                Ok(RoutedTwoWaySolve {
                    solution: solved.solution,
                    receipt: self.route_receipt(Some(solved.receipt), None),
                })
            }
            PreparedSolverBackend::Diagonal(preconditioner) => {
                self.solve_preconditioned(worker_rhs, firm_rhs, preconditioner, interrupt)
            }
            PreparedSolverBackend::Cmg(preconditioner) => {
                self.solve_preconditioned(worker_rhs, firm_rhs, preconditioner.as_ref(), interrupt)
            }
            #[cfg(feature = "cmg-full-spike")]
            PreparedSolverBackend::FullCmg(solver) => {
                let mut solved = solver.solve_batch_with_interrupt(
                    &self.operator,
                    worker_rhs,
                    firm_rhs,
                    1,
                    FullCmgSpikePhase::Fit,
                    interrupt,
                )?;
                if solved.solution.len() != 1 || solved.pcg.len() != 1 {
                    return Err(BackendError::invariant(
                        "prepared_solver",
                        "private full-CMG scalar solve did not return exactly one RHS",
                    ));
                }
                let solution = solved.solution.pop().expect("validated one solution");
                let pcg = solved.pcg.pop().expect("validated one PCG receipt");
                Ok(RoutedTwoWaySolve {
                    solution,
                    receipt: self.route_receipt(None, Some(pcg)),
                })
            }
        }
    }

    pub fn solve_batch(
        &self,
        worker_rhs: &[f64],
        firm_rhs: &[f64],
        columns: usize,
    ) -> Result<RoutedTwoWayBatchSolve> {
        let mut interrupt = NeverInterrupt;
        self.solve_batch_with_interrupt(worker_rhs, firm_rhs, columns, &mut interrupt)
    }

    pub fn solve_batch_with_interrupt(
        &self,
        worker_rhs: &[f64],
        firm_rhs: &[f64],
        columns: usize,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<RoutedTwoWayBatchSolve> {
        if columns == 0 {
            return Err(BackendError::invalid(
                "prepared_solver",
                "batched solve width must be positive",
            ));
        }
        match &self.backend {
            PreparedSolverBackend::Exact(factor) => {
                let workers = self.operator.problem().workers();
                let firms = self.operator.problem().firms();
                if worker_rhs.len()
                    != workers.checked_mul(columns).ok_or_else(|| {
                        BackendError::new(
                            ErrorCode::ResourceLimit,
                            "prepared_solver",
                            "worker RHS length overflow",
                        )
                    })?
                    || firm_rhs.len()
                        != firms.checked_mul(columns).ok_or_else(|| {
                            BackendError::new(
                                ErrorCode::ResourceLimit,
                                "prepared_solver",
                                "firm RHS length overflow",
                            )
                        })?
                {
                    return Err(BackendError::invalid(
                        "prepared_solver",
                        "exact batched RHS arrays have incompatible dimensions",
                    ));
                }
                let mut solution = Vec::with_capacity(columns);
                let mut receipt = Vec::with_capacity(columns);
                for column in 0..columns {
                    interrupt.checkpoint("prepared_exact_rhs")?;
                    let solved = solve_two_way_exact_factored_with_interrupt(
                        &self.operator,
                        factor,
                        &worker_rhs[column * workers..(column + 1) * workers],
                        &firm_rhs[column * firms..(column + 1) * firms],
                        self.options.full_residual_tolerance,
                        interrupt,
                    )
                    .map_err(|error| {
                        BackendError::new(
                            error.code,
                            "prepared_solver",
                            format!("zero-based RHS column {column}: {error}"),
                        )
                    })?;
                    receipt.push(self.route_receipt(Some(solved.receipt), None));
                    solution.push(solved.solution);
                }
                Ok(RoutedTwoWayBatchSolve { solution, receipt })
            }
            PreparedSolverBackend::Diagonal(preconditioner) => self.solve_preconditioned_batch(
                worker_rhs,
                firm_rhs,
                columns,
                preconditioner,
                interrupt,
            ),
            PreparedSolverBackend::Cmg(preconditioner) => self.solve_preconditioned_batch(
                worker_rhs,
                firm_rhs,
                columns,
                preconditioner.as_ref(),
                interrupt,
            ),
            #[cfg(feature = "cmg-full-spike")]
            PreparedSolverBackend::FullCmg(solver) => {
                let solved = solver.solve_batch_with_interrupt(
                    &self.operator,
                    worker_rhs,
                    firm_rhs,
                    columns,
                    FullCmgSpikePhase::Probe,
                    interrupt,
                )?;
                let _batch_receipt = solved.receipt;
                if solved.solution.len() != solved.pcg.len() {
                    return Err(BackendError::invariant(
                        "prepared_solver",
                        "private full-CMG solution and receipt counts differ",
                    ));
                }
                let receipt = solved
                    .pcg
                    .into_iter()
                    .map(|pcg| self.route_receipt(None, Some(pcg)))
                    .collect();
                Ok(RoutedTwoWayBatchSolve {
                    solution: solved.solution,
                    receipt,
                })
            }
        }
    }

    fn solve_preconditioned(
        &self,
        worker_rhs: &[f64],
        firm_rhs: &[f64],
        preconditioner: &impl Preconditioner,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<RoutedTwoWaySolve> {
        let reduced_rhs = self
            .operator
            .schur_rhs_with_interrupt(worker_rhs, firm_rhs, interrupt)?;
        let reduced = pcg_with_interrupt(
            &self.operator,
            preconditioner,
            &reduced_rhs,
            self.options.pcg,
            interrupt,
        )?;
        let firm = self.operator.expand_firm(&reduced.solution)?;
        let worker = self
            .operator
            .reconstruct_worker_with_interrupt(worker_rhs, &firm, interrupt)?;
        let residual = self
            .operator
            .full_residual_with_interrupt(&worker, &firm, worker_rhs, firm_rhs, interrupt)?;
        if residual.relative_norm > self.options.full_residual_tolerance {
            return Err(full_residual_error(
                residual.relative_norm,
                self.options.full_residual_tolerance,
            ));
        }
        Ok(RoutedTwoWaySolve {
            solution: TwoWaySolution {
                worker,
                firm,
                reduced_firm: reduced.solution,
                residual,
            },
            receipt: self.route_receipt(None, Some(reduced.receipt)),
        })
    }

    fn solve_preconditioned_batch(
        &self,
        worker_rhs: &[f64],
        firm_rhs: &[f64],
        columns: usize,
        preconditioner: &impl Preconditioner,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<RoutedTwoWayBatchSolve> {
        let solved = solve_two_way_pcg_batch_with_interrupt(
            &self.operator,
            preconditioner,
            worker_rhs,
            firm_rhs,
            columns,
            self.options.pcg,
            self.options.full_residual_tolerance,
            interrupt,
        )?;
        self.finish_pcg_batch(solved)
    }

    fn finish_pcg_batch(&self, solved: TwoWayBatchedPcgSolve) -> Result<RoutedTwoWayBatchSolve> {
        if solved.solution.len() != solved.pcg.len() {
            return Err(BackendError::invariant(
                "prepared_solver",
                "batched PCG solution and receipt counts differ",
            ));
        }
        let receipt = solved
            .pcg
            .into_iter()
            .map(|pcg| self.route_receipt(None, Some(pcg)))
            .collect();
        Ok(RoutedTwoWayBatchSolve {
            solution: solved.solution,
            receipt,
        })
    }

    fn route_receipt(
        &self,
        exact: Option<ExactSolveReceipt>,
        pcg: Option<PcgReceipt>,
    ) -> RoutedSolveReceipt {
        RoutedSolveReceipt {
            requested: self.receipt.requested,
            selected: self.receipt.selected,
            dimension: self.receipt.dimension,
            exact,
            pcg,
            cmg: self.receipt.cmg.clone(),
            fallback: self.receipt.fallback.clone(),
        }
    }
}

pub fn solve_two_way_routed(
    problem: &CompressedProblem,
    worker_rhs: &[f64],
    firm_rhs: &[f64],
    options: LinearSolverOptions,
) -> Result<RoutedTwoWaySolve> {
    PreparedTwoWaySolver::prepare(problem, options)?.solve(worker_rhs, firm_rhs)
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

fn full_residual_error(residual: f64, tolerance: f64) -> BackendError {
    BackendError::new(
        ErrorCode::FullResidualFailed,
        "solver_router",
        format!("complete normal-equation residual {residual} exceeds tolerance {tolerance}"),
    )
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

    struct BreakOnPhase {
        phase: &'static str,
        break_call: usize,
        calls: usize,
    }

    impl InterruptCheck for BreakOnPhase {
        fn checkpoint(&mut self, phase: &'static str) -> Result<()> {
            if phase == self.phase {
                self.calls += 1;
                if self.calls == self.break_call {
                    return Err(BackendError::new(
                        ErrorCode::UserBreak,
                        phase,
                        "injected CMG setup break",
                    ));
                }
            }
            Ok(())
        }
    }

    #[derive(Default)]
    struct CountExactFactor {
        calls: usize,
    }

    impl InterruptCheck for CountExactFactor {
        fn checkpoint(&mut self, phase: &'static str) -> Result<()> {
            if phase == "exact_factor" {
                self.calls += 1;
            }
            Ok(())
        }
    }

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
                firm.push(u64::try_from((worker_index + offset) % firms + 1).expect("firm"));
                deletion.push(u64::try_from(worker_index * 4 + offset + 1).expect("deletion"));
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
        let mut options = LinearSolverOptions {
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
            full_residual_tolerance: 1.0e-10,
            ..LinearSolverOptions::default()
        };
        options.full_residual_tolerance = options.required_full_residual_tolerance();
        options
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
                firm.push(u64::try_from((worker_index + offset) % firms + 1).expect("firm"));
                deletion.push(u64::try_from(2 * worker_index + offset + 1).expect("deletion"));
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

    fn large_cmg_setup_problem(workers: usize, firms: usize) -> CompressedProblem {
        let mut worker = Vec::with_capacity(2 * workers);
        let mut firm = Vec::with_capacity(2 * workers);
        for worker_index in 0..workers {
            let worker_label = u64::try_from(worker_index + 1).expect("worker");
            worker.extend([worker_label, worker_label]);
            firm.extend([
                u64::try_from(worker_index % firms + 1).expect("firm"),
                u64::try_from((worker_index + 1) % firms + 1).expect("firm"),
            ]);
        }
        let rows = worker.len();
        CanonicalInput::from_validated(
            InputColumns {
                worker,
                firm,
                deletion: (1..=u64::try_from(rows).expect("rows")).collect(),
                outcome: vec![0.0; rows],
                frequency: vec![1; rows],
                target_weight: vec![1.0; rows],
                controls: Vec::new(),
            }
            .validate()
            .expect("large CMG setup fixture"),
        )
        .expect("large CMG canonical")
        .compress(&vec![true; rows])
        .expect("large CMG compressed")
    }

    fn dense_zero_sum_predictions(problem: &CompressedProblem) -> Vec<f64> {
        let coefficients = problem.workers() + problem.firms();
        let dimension = coefficients + 1;
        let constraint = coefficients;
        let mut matrix = vec![0.0; dimension * dimension];
        let mut rhs = vec![0.0; dimension];
        for row in 0..problem.outcome.len() {
            let worker = usize::try_from(problem.row_worker[row]).expect("worker");
            let firm = problem.workers() + usize::try_from(problem.row_firm[row]).expect("firm");
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
                let firm =
                    problem.workers() + usize::try_from(problem.row_firm[row]).expect("firm");
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
                    matrix[row * dimension + entry] -= factor * matrix[column * dimension + entry];
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
            full_residual_tolerance: 1.0e-11,
            ..base_options()
        };
        let solve = solve_two_way_routed(problem, &worker_rhs, &firm_rhs, options)
            .expect("relabeling route must remain valid");
        let prediction = fitted_values(problem, &solve.solution.worker, &solve.solution.firm)
            .expect("prediction");
        (solve, prediction)
    }

    fn mapped_firm_residual(firm_label: [u64; 3], solve: &RoutedTwoWaySolve) -> Vec<f64> {
        let mut sorted = firm_label;
        sorted.sort_unstable();
        firm_label
            .iter()
            .map(|label| {
                let dense = sorted
                    .iter()
                    .position(|candidate| candidate == label)
                    .expect("firm");
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

    #[cfg(feature = "cmg-full-spike")]
    #[test]
    fn private_full_cmg_direct_hybrid_matches_exact_and_batches_deterministically() {
        static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let _guard = ENV_LOCK.lock().expect("private CMG environment lock");
        std::env::set_var("VCKSS_PRIVATE_CMG_THREADS", "2");
        std::env::set_var("VCKSS_PRIVATE_CMG_PROBE_TOLERANCE", "1e-6");

        let problem = fixture();
        let operator = TwoWayOperator::new(&problem).expect("operator");
        let (worker_rhs, firm_rhs) = operator.outcome_rhs().expect("outcome RHS");
        let exact = PreparedTwoWaySolver::prepare(
            &problem,
            LinearSolverOptions {
                route: LinearSolverRoute::Exact,
                exact_dimension_limit: 64,
                ..base_options()
            },
        )
        .expect("exact prepare")
        .solve(&worker_rhs, &firm_rhs)
        .expect("exact solve");

        let options = LinearSolverOptions {
            route: LinearSolverRoute::CmgPcg,
            pcg: PcgOptions {
                tolerance: 1.0e-10,
                maximum_iterations: 2_000,
                residual_replacement_interval: 100,
            },
            full_residual_tolerance: 1.0e-9,
            ..base_options()
        };
        let direct = PreparedTwoWaySolver::prepare_full_cmg_spike_with_interrupt(
            &problem,
            options,
            &mut NeverInterrupt,
        )
        .expect("private full-CMG prepare");
        assert_eq!(direct.receipt().selected, LinearSolverRoute::CmgPcg);
        assert!(direct
            .receipt()
            .cmg
            .as_ref()
            .is_some_and(|value| value.levels > 0));

        let scalar = direct
            .solve(&worker_rhs, &firm_rhs)
            .expect("private full-CMG scalar solve");
        assert!(scalar.solution.residual.relative_norm <= 1.0e-9);
        let exact_prediction =
            fitted_values(&problem, &exact.solution.worker, &exact.solution.firm)
                .expect("exact predictions");
        let direct_prediction =
            fitted_values(&problem, &scalar.solution.worker, &scalar.solution.firm)
                .expect("direct predictions");
        assert_close(&exact_prediction, &direct_prediction, 2.0e-9);

        let worker_batch = [worker_rhs.as_slice(), worker_rhs.as_slice()].concat();
        let firm_batch = [firm_rhs.as_slice(), firm_rhs.as_slice()].concat();
        let batch = direct
            .solve_batch(&worker_batch, &firm_batch, 2)
            .expect("private full-CMG batch solve");
        assert_eq!(batch.solution.len(), 2);
        assert_eq!(batch.receipt.len(), 2);
        assert_eq!(batch.solution[0].worker, batch.solution[1].worker);
        assert_eq!(batch.solution[0].firm, batch.solution[1].firm);
        assert_eq!(
            batch.receipt[0].pcg.as_ref().unwrap().iterations,
            batch.receipt[1].pcg.as_ref().unwrap().iterations
        );
        assert!(batch
            .solution
            .iter()
            .all(|value| value.residual.relative_norm <= 1.0e-5));

        std::env::remove_var("VCKSS_PRIVATE_CMG_THREADS");
        std::env::remove_var("VCKSS_PRIVATE_CMG_PROBE_TOLERANCE");
    }

    #[test]
    fn full_residual_gate_is_derived_exactly_from_pcg_tolerance() {
        let mut options = LinearSolverOptions::default();
        assert_eq!(options.required_full_residual_tolerance(), 1.0e-9);
        options.validate().expect("default exact residual gate");

        options.full_residual_tolerance = 2.0e-9;
        assert_eq!(
            options.validate().expect_err("weaker full gate").code,
            ErrorCode::InvalidInput
        );
        options.full_residual_tolerance = 5.0e-10;
        assert_eq!(
            options.validate().expect_err("stricter full gate").code,
            ErrorCode::InvalidInput
        );

        options.pcg.tolerance = 1.0e-13;
        options.full_residual_tolerance = 1.0e-11;
        assert_eq!(options.required_full_residual_tolerance(), 1.0e-11);
        options
            .validate()
            .expect("absolute residual floor boundary");
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
    fn prepared_exact_batch_reuses_one_factorization() {
        let problem = fixture();
        let mut interrupt = CountExactFactor::default();
        let solver =
            PreparedTwoWaySolver::prepare_with_interrupt(&problem, base_options(), &mut interrupt)
                .expect("prepared exact solver");
        assert_eq!(solver.receipt().selected, LinearSolverRoute::Exact);
        assert!(interrupt.calls > 0);
        let factor_calls = interrupt.calls;
        let (worker_rhs, firm_rhs) = solver.operator().outcome_rhs().expect("RHS");
        let columns = 3;
        let solved = solver
            .solve_batch_with_interrupt(
                &worker_rhs.repeat(columns),
                &firm_rhs.repeat(columns),
                columns,
                &mut interrupt,
            )
            .expect("factored exact batch");
        assert_eq!(interrupt.calls, factor_calls);
        assert_eq!(solved.solution.len(), columns);
        assert_eq!(solved.receipt.len(), columns);
        for solution in &solved.solution[1..] {
            assert_close(&solved.solution[0].worker, &solution.worker, 0.0);
            assert_close(&solved.solution[0].firm, &solution.firm, 0.0);
        }
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
        let solve =
            solve_two_way_routed(&problem, &worker_rhs, &firm_rhs, options).expect("CMG solve");
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
            route_decision(
                &TwoWayOperator::new(&above_limit).expect("operator"),
                options
            )
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
            full_residual_tolerance: 1.0e-11,
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

    #[test]
    fn user_break_during_forced_and_automatic_cmg_setup_is_never_fallbacked() {
        let problem = large_cmg_setup_problem(4_097, 16);
        for route in [LinearSolverRoute::CmgPcg, LinearSolverRoute::Auto] {
            let mut options = base_options();
            options.route = route;
            options.exact_dimension_limit = 1;
            options.cmg_minimum_dimension = 2;
            options.allow_automatic_cmg_setup_fallback = true;
            let mut interrupt = BreakOnPhase {
                phase: "cmg_graph_worker_count",
                break_call: 2,
                calls: 0,
            };
            let error =
                PreparedTwoWaySolver::prepare_with_interrupt(&problem, options, &mut interrupt)
                    .expect_err("CMG setup break must escape routing");
            assert_eq!(error.code, ErrorCode::UserBreak);
            assert_eq!(error.phase, "cmg_graph_worker_count");
            assert_eq!(interrupt.calls, 2);
        }
    }
}
