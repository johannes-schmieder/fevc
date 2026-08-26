// SPDX-License-Identifier: GPL-3.0-only

//! Production direct-hybrid solver backed by the pinned full CMG library.
//!
//! The route solves the existing VCkss hybrid Laplacian directly and reuses
//! one graph, hierarchy, executor, plan, and admitted workspace pool across
//! mathematically independent RHS columns. Experimental fused and mixed-
//! precision implementations are intentionally absent from this module.

use std::sync::Mutex;
use std::time::Instant;

use cmg::{
    CmgError, CmgOptions as FullCmgOptions, Laplacian, ParallelOptions, ParallelPcgExecution,
    ParallelPcgSolver, PcgOptions as FullPcgOptions, PcgResult, ValidationOptions,
    VckssContiguousPcgWorkspace,
};

use crate::cmg::{AggregationMethod, CmgLevelReceipt, CmgReceipt, HybridGraph};
use crate::error::{BackendError, ErrorCode, Result};
use crate::interrupt::{InterruptCheck, NeverInterrupt};
use crate::krylov::{PcgOptions, PcgReceipt};
use crate::operator::{stable_norm, SymmetricOperator, TwoWayOperator, TwoWaySolution};
use crate::problem::CompressedProblem;

pub const FULL_CMG_SCHEMA: &str = "CMG_FULL_V2";
pub const CMG_SOURCE_COMMIT: &str = "dbefbc5e3b442c6dde6e7861a66d82fd5ed24f10";
const MAX_COMPRESSED_BATCH_RHS: usize = 64;
const DEFAULT_PROBE_TOLERANCE: f64 = 1.0e-6;
// The deterministic fit remains deliberately tighter because it has no Monte
// Carlo envelope. Probe solves use the public phase tolerance directly; the
// independent complete-system gate, not an internal Schur/Krylov intermediate,
// is the release-blocking numerical certificate.
const FIT_INNER_TOLERANCE_RATIO: f64 = 0.01;
const PROBE_INNER_TOLERANCE_RATIO: f64 = 1.0;

#[derive(Clone, Copy, Debug)]
pub struct FullCmgPlanOptions {
    pub threads: usize,
    pub fit_tolerance: f64,
    pub probe_tolerance: f64,
    pub maximum_batch_rhs: usize,
}

impl FullCmgPlanOptions {
    pub fn production(threads: usize, fit_tolerance: f64, probe_tolerance: Option<f64>) -> Self {
        Self {
            threads,
            fit_tolerance,
            probe_tolerance: probe_tolerance.unwrap_or(DEFAULT_PROBE_TOLERANCE),
            maximum_batch_rhs: MAX_COMPRESSED_BATCH_RHS,
        }
    }

    fn validate(self) -> Result<Self> {
        if self.threads == 0 || self.maximum_batch_rhs == 0 {
            return Err(BackendError::invalid(
                "cmg_full_v2",
                "thread count and maximum batch width must be positive",
            ));
        }
        for (name, tolerance) in [
            ("fit tolerance", self.fit_tolerance),
            ("probe tolerance", self.probe_tolerance),
        ] {
            if !tolerance.is_finite() || !(1.0e-15..=1.0e-4).contains(&tolerance) {
                return Err(BackendError::invalid(
                    "cmg_full_v2",
                    format!("{name} must lie in [1e-15, 1e-4]"),
                ));
            }
        }
        Ok(self)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum FullCmgPhase {
    Fit,
    Probe,
}

#[derive(Clone, Copy, Debug)]
struct FullCmgTolerances {
    fit_effective: f64,
    probe_effective: f64,
    fit: PcgOptions,
    probe: PcgOptions,
    fit_complete_residual: f64,
    probe_complete_residual: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct FullCmgSetupReceipt {
    pub threads: usize,
    pub vertices: usize,
    pub edges: usize,
    pub hierarchy_levels: usize,
    pub terminal_vertices: usize,
    pub graph_copy_bytes: u64,
    pub hierarchy_bytes: u64,
    pub plan_bytes: u64,
    pub workspace_bytes_each: u64,
    pub admitted_workspace_pool_bytes: u64,
    pub admitted_peak_bytes: u64,
    pub fit_effective_tolerance: f64,
    pub probe_effective_tolerance: f64,
    pub fit_inner_tolerance: f64,
    pub probe_inner_tolerance: f64,
    pub graph_nanoseconds: u128,
    pub solver_nanoseconds: u128,
}

#[derive(Clone, Copy, Debug)]
pub struct FullCmgBatchReceipt {
    pub execution: FullCmgExecution,
    pub rhs_count: usize,
    pub concurrency: usize,
    pub extraction_concurrency: usize,
    pub rhs_nanoseconds: u128,
    pub solve_nanoseconds: u128,
    pub extraction_nanoseconds: u128,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FullCmgExecution {
    Serial,
    Planned,
    AcrossRightHandSides,
}

#[derive(Clone, Debug)]
pub struct FullCmgReceipt {
    pub schema: &'static str,
    pub source_commit: &'static str,
    pub setup: FullCmgSetupReceipt,
    pub batch_calls: u64,
    pub rhs_count: u64,
    pub maximum_concurrency: usize,
    pub serial_batches: u64,
    pub planned_batches: u64,
    pub across_rhs_batches: u64,
    pub total_iterations: u64,
    pub total_operator_applications: u64,
    pub total_preconditioner_applications: u64,
    pub maximum_reduced_residual: f64,
    pub maximum_complete_residual: f64,
    pub rhs_nanoseconds: u128,
    pub solve_nanoseconds: u128,
    pub extraction_nanoseconds: u128,
    pub refinement_attempts: u64,
    pub refined_columns: u64,
}

impl FullCmgReceipt {
    fn new(setup: FullCmgSetupReceipt) -> Self {
        Self {
            schema: FULL_CMG_SCHEMA,
            source_commit: CMG_SOURCE_COMMIT,
            setup,
            batch_calls: 0,
            rhs_count: 0,
            maximum_concurrency: 0,
            serial_batches: 0,
            planned_batches: 0,
            across_rhs_batches: 0,
            total_iterations: 0,
            total_operator_applications: 0,
            total_preconditioner_applications: 0,
            maximum_reduced_residual: 0.0,
            maximum_complete_residual: 0.0,
            rhs_nanoseconds: 0,
            solve_nanoseconds: 0,
            extraction_nanoseconds: 0,
            refinement_attempts: 0,
            refined_columns: 0,
        }
    }

    fn record_batch(
        &mut self,
        batch: FullCmgBatchReceipt,
        pcg: &[PcgReceipt],
        solutions: &[TwoWaySolution],
    ) -> Result<()> {
        self.batch_calls = checked_add_receipt(self.batch_calls, 1, "batch call count")?;
        self.rhs_count = checked_add_receipt(
            self.rhs_count,
            to_u64(batch.rhs_count, "batch RHS count")?,
            "RHS count",
        )?;
        self.maximum_concurrency = self.maximum_concurrency.max(batch.concurrency);
        let counter = match batch.execution {
            FullCmgExecution::Serial => &mut self.serial_batches,
            FullCmgExecution::Planned => &mut self.planned_batches,
            FullCmgExecution::AcrossRightHandSides => &mut self.across_rhs_batches,
        };
        *counter = checked_add_receipt(*counter, 1, "batch strategy count")?;
        for receipt in pcg {
            self.total_iterations = checked_add_receipt(
                self.total_iterations,
                u64::from(receipt.iterations),
                "iteration count",
            )?;
            self.total_operator_applications = checked_add_receipt(
                self.total_operator_applications,
                u64::from(receipt.operator_applications),
                "operator application count",
            )?;
            self.total_preconditioner_applications = checked_add_receipt(
                self.total_preconditioner_applications,
                u64::from(receipt.preconditioner_applications),
                "preconditioner application count",
            )?;
            self.maximum_reduced_residual =
                self.maximum_reduced_residual.max(receipt.relative_residual);
        }
        self.maximum_complete_residual = self.maximum_complete_residual.max(
            solutions
                .iter()
                .map(|solution| solution.residual.relative_norm)
                .fold(0.0_f64, f64::max),
        );
        self.rhs_nanoseconds = self.rhs_nanoseconds.saturating_add(batch.rhs_nanoseconds);
        self.solve_nanoseconds = self
            .solve_nanoseconds
            .saturating_add(batch.solve_nanoseconds);
        self.extraction_nanoseconds = self
            .extraction_nanoseconds
            .saturating_add(batch.extraction_nanoseconds);
        Ok(())
    }
}

#[derive(Debug)]
pub(crate) struct FullCmgDirectSolve {
    pub solution: Vec<TwoWaySolution>,
    pub pcg: Vec<PcgReceipt>,
    pub receipt: FullCmgBatchReceipt,
}

#[derive(Debug)]
struct FullCmgSolvedColumn {
    hybrid_solution: Vec<f64>,
    iterations: usize,
    restarts: usize,
    initial_residual_norm: f64,
}

#[derive(Debug)]
struct FullCmgExtractedColumn {
    solution: TwoWaySolution,
    receipt: PcgReceipt,
}

#[derive(Debug)]
pub(crate) struct FullCmgDirectSolver {
    hybrid: HybridGraph,
    solver: ParallelPcgSolver,
    workspace: Mutex<VckssContiguousPcgWorkspace>,
    setup: FullCmgSetupReceipt,
    receipt: Mutex<FullCmgReceipt>,
    compatibility_receipt: CmgReceipt,
    tolerances: FullCmgTolerances,
}

impl FullCmgDirectSolver {
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
        self.solver.vckss_map_ordered(input, operation)
    }

    pub(crate) fn prepare_with_interrupt(
        problem: &CompressedProblem,
        pcg: PcgOptions,
        memory_limit_bytes: u64,
        plan: FullCmgPlanOptions,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<Self> {
        interrupt.checkpoint("cmg_full_v2_prepare")?;
        let plan = plan.validate()?;
        let fit_tolerance = plan.fit_tolerance;
        let probe_tolerance = plan.probe_tolerance;
        let tolerances = FullCmgTolerances {
            fit_effective: fit_tolerance,
            probe_effective: probe_tolerance,
            fit: PcgOptions {
                tolerance: fit_tolerance * FIT_INNER_TOLERANCE_RATIO,
                ..pcg
            },
            probe: PcgOptions {
                tolerance: probe_tolerance * PROBE_INNER_TOLERANCE_RATIO,
                ..pcg
            },
            fit_complete_residual: complete_residual_tolerance(fit_tolerance),
            probe_complete_residual: complete_residual_tolerance(probe_tolerance),
        };
        tolerances.fit.validate()?;
        tolerances.probe.validate()?;
        let workspace_budget = usize::try_from(memory_limit_bytes).map_err(|_| {
            BackendError::new(
                ErrorCode::ResourceLimit,
                "cmg_full_v2",
                "CMG workspace budget is not representable on this platform",
            )
        })?;

        let graph_start = Instant::now();
        let hybrid = HybridGraph::from_problem_with_interrupt(problem, interrupt)?;
        let graph = Laplacian::from_edges(
            hybrid.vertices(),
            hybrid.edges().iter().map(|edge| {
                (
                    usize::try_from(edge.u).expect("validated compact endpoint"),
                    usize::try_from(edge.v).expect("validated compact endpoint"),
                    edge.weight,
                )
            }),
        )
        .map_err(|error| map_setup_error(error, "graph conversion"))?;
        let graph_nanoseconds = graph_start.elapsed().as_nanos();
        interrupt.checkpoint("cmg_full_v2_graph_complete")?;

        let solver_start = Instant::now();
        let solver = ParallelPcgSolver::build(
            &graph,
            FullCmgOptions::default(),
            ParallelOptions {
                threads: plan.threads,
                workspace_memory_budget_bytes: Some(workspace_budget),
                ..ParallelOptions::default()
            },
        )
        .map_err(|error| map_setup_error(error, "hierarchy construction"))?;
        let solver_nanoseconds = solver_start.elapsed().as_nanos();
        interrupt.checkpoint("cmg_full_v2_solver_complete")?;

        let maximum_batch = solver
            .select_batch_execution(plan.maximum_batch_rhs)
            .map_err(|error| map_setup_error(error, "batch admission"))?;
        let graph_copy_bytes = graph_storage_bytes(&graph)?;
        let hierarchy_bytes = hierarchy_storage_bytes(&solver)?;
        let plan_bytes = to_u64(solver.plan().byte_len(), "standalone plan bytes")?;
        let workspace_bytes_each = to_u64(
            maximum_batch.workspace_bytes_each(),
            "standalone workspace bytes",
        )?;
        let admitted_workspace_pool_bytes = to_u64(
            maximum_batch.workspace_pool_bytes(),
            "standalone workspace pool bytes",
        )?;
        let batch_vectors = checked_product_u64(&[
            to_u64(hybrid.vertices(), "hybrid vertices")?,
            to_u64(plan.maximum_batch_rhs, "maximum batch RHS")?,
            8,
            2,
        ])?;
        let retained = checked_sum_u64(&[
            hybrid.predicted_bytes(),
            graph_copy_bytes,
            hierarchy_bytes,
            plan_bytes,
            admitted_workspace_pool_bytes,
            batch_vectors,
        ])?;
        let allocator_allowance = retained / 5;
        let admitted_peak_bytes = retained.checked_add(allocator_allowance).ok_or_else(|| {
            BackendError::new(
                ErrorCode::ResourceLimit,
                "cmg_full_v2",
                "full-CMG whole-process memory admission overflow",
            )
        })?;
        if admitted_peak_bytes > memory_limit_bytes {
            return Err(BackendError::new(
                ErrorCode::ResourceLimit,
                "cmg_full_v2",
                format!(
                    "full-CMG forecast {admitted_peak_bytes} exceeds CMG memory limit {memory_limit_bytes}"
                ),
            ));
        }

        let setup = FullCmgSetupReceipt {
            threads: plan.threads,
            vertices: hybrid.vertices(),
            edges: hybrid.edges().len(),
            hierarchy_levels: solver.preconditioner().hierarchy().levels().len(),
            terminal_vertices: solver
                .preconditioner()
                .hierarchy()
                .levels()
                .last()
                .expect("full-CMG hierarchy is nonempty")
                .graph()
                .vertex_count(),
            graph_copy_bytes,
            hierarchy_bytes,
            plan_bytes,
            workspace_bytes_each,
            admitted_workspace_pool_bytes,
            admitted_peak_bytes,
            fit_effective_tolerance: tolerances.fit_effective,
            probe_effective_tolerance: tolerances.probe_effective,
            fit_inner_tolerance: tolerances.fit.tolerance,
            probe_inner_tolerance: tolerances.probe.tolerance,
            graph_nanoseconds,
            solver_nanoseconds,
        };
        let compatibility_receipt = compatibility_receipt(&solver, &setup)?;
        let workspace = Mutex::new(solver.vckss_contiguous_workspace());
        let receipt = Mutex::new(FullCmgReceipt::new(setup));
        let prepared = Self {
            hybrid,
            solver,
            workspace,
            setup,
            receipt,
            compatibility_receipt,
            tolerances,
        };
        pcg.validate()?;
        Ok(prepared)
    }

    pub(crate) fn compatibility_receipt(&self) -> &CmgReceipt {
        &self.compatibility_receipt
    }

    pub(crate) fn receipt(&self) -> Result<FullCmgReceipt> {
        self.receipt
            .lock()
            .map(|receipt| receipt.clone())
            .map_err(|_| {
                BackendError::new(
                    ErrorCode::ContextPoisoned,
                    "cmg_full_v2",
                    "full-CMG receipt mutex is poisoned",
                )
            })
    }

    pub(crate) fn maximum_complete_residual_tolerance(&self) -> f64 {
        self.tolerances
            .fit_complete_residual
            .max(self.tolerances.probe_complete_residual)
    }

    pub(crate) fn solve_batch_with_interrupt(
        &self,
        operator: &TwoWayOperator<'_>,
        worker_rhs: &[f64],
        firm_rhs: &[f64],
        columns: usize,
        phase: FullCmgPhase,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<FullCmgDirectSolve> {
        validate_rhs(operator, worker_rhs, firm_rhs, columns)?;
        interrupt.checkpoint("cmg_full_v2_rhs")?;
        let rhs_start = Instant::now();
        let mut right_hand_sides = vec![0.0; self.hybrid.vertices().saturating_mul(columns)];
        for column in 0..columns {
            interrupt.checkpoint("cmg_full_v2_rhs_column")?;
            let worker_begin = column * operator.problem().workers();
            let firm_begin = column * operator.problem().firms();
            let schur = operator.schur_rhs_with_interrupt(
                &worker_rhs[worker_begin..worker_begin + operator.problem().workers()],
                &firm_rhs[firm_begin..firm_begin + operator.problem().firms()],
                interrupt,
            )?;
            let rhs_begin = column * self.hybrid.vertices();
            right_hand_sides[rhs_begin..rhs_begin + self.hybrid.firms()].copy_from_slice(&schur);
        }
        let rhs_nanoseconds = rhs_start.elapsed().as_nanos();
        interrupt.checkpoint("cmg_full_v2_solve")?;

        let solve_start = Instant::now();
        let (pcg, full_residual_tolerance) = match phase {
            FullCmgPhase::Fit => (self.tolerances.fit, self.tolerances.fit_complete_residual),
            FullCmgPhase::Probe => (
                self.tolerances.probe,
                self.tolerances.probe_complete_residual,
            ),
        };
        let full_options = full_pcg_options(pcg)?;
        let (execution, solved_columns) =
            self.solve_scalar_columns(&right_hand_sides, columns, full_options)?;
        let solve_nanoseconds = solve_start.elapsed().as_nanos();
        interrupt.checkpoint("cmg_full_v2_solve_complete")?;

        let extraction_start = Instant::now();
        let mut solution = Vec::with_capacity(columns);
        let mut receipts = Vec::with_capacity(columns);
        let extraction_concurrency = self.setup.threads.min(columns).max(1);
        let mut pending = solved_columns.into_iter().enumerate();
        loop {
            interrupt.checkpoint("cmg_full_v2_extract_chunk")?;
            let chunk = pending
                .by_ref()
                .take(extraction_concurrency)
                .collect::<Vec<_>>();
            if chunk.is_empty() {
                break;
            }
            let extracted = self
                .solver
                .vckss_map_ordered(chunk, |(column, solved_column)| {
                    self.extract_solved_column(
                        operator,
                        worker_rhs,
                        firm_rhs,
                        &right_hand_sides,
                        column,
                        solved_column,
                        full_residual_tolerance,
                    )
                });
            for extracted_column in extracted {
                interrupt.checkpoint("cmg_full_v2_extract_complete")?;
                let extracted_column = extracted_column?;
                solution.push(extracted_column.solution);
                receipts.push(extracted_column.receipt);
            }
        }
        let extraction_nanoseconds = extraction_start.elapsed().as_nanos();
        let receipt = FullCmgBatchReceipt {
            execution,
            rhs_count: columns,
            concurrency: self
                .solver
                .select_batch_execution(columns)
                .map_err(|error| map_solve_error(error, "batch routing receipt"))?
                .concurrency(),
            extraction_concurrency,
            rhs_nanoseconds,
            solve_nanoseconds,
            extraction_nanoseconds,
        };
        self.receipt
            .lock()
            .map_err(|_| {
                BackendError::new(
                    ErrorCode::ContextPoisoned,
                    "cmg_full_v2",
                    "full-CMG receipt mutex is poisoned",
                )
            })?
            .record_batch(receipt, &receipts, &solution)?;
        Ok(FullCmgDirectSolve {
            solution,
            pcg: receipts,
            receipt,
        })
    }

    fn solve_scalar_columns(
        &self,
        right_hand_sides: &[f64],
        columns: usize,
        options: FullPcgOptions,
    ) -> Result<(FullCmgExecution, Vec<FullCmgSolvedColumn>)> {
        let report = self
            .solver
            .select_batch_execution(columns)
            .map_err(|error| map_solve_error(error, "batch routing"))?;
        let mut workspace = self.workspace.lock().map_err(|_| {
            BackendError::new(
                ErrorCode::ContextPoisoned,
                "cmg_full_v2",
                "standalone CMG workspace mutex is poisoned",
            )
        })?;
        let solved = self
            .solver
            .vckss_solve_contiguous_columns_with_workspace(
                right_hand_sides,
                columns,
                options,
                &mut workspace,
            )
            .map_err(|error| map_solve_error(error, "direct hybrid batch"))?;
        let execution = scalar_execution(report.execution());
        Ok((execution, solved.into_iter().map(scalar_column).collect()))
    }

    fn finish_solved_column(
        &self,
        operator: &TwoWayOperator<'_>,
        solved: FullCmgSolvedColumn,
    ) -> Result<(Vec<f64>, Vec<f64>, PcgReceipt)> {
        let iterations = u32::try_from(solved.iterations).map_err(|_| {
            BackendError::new(
                ErrorCode::ResourceLimit,
                "cmg_full_v2",
                "standalone CMG iteration count exceeds u32",
            )
        })?;
        let replacements = u32::try_from(solved.restarts).map_err(|_| {
            BackendError::new(
                ErrorCode::ResourceLimit,
                "cmg_full_v2",
                "standalone CMG restart count exceeds u32",
            )
        })?;
        let zero_rhs = solved.initial_residual_norm == 0.0;
        let mut hybrid_solution = solved.hybrid_solution;
        self.hybrid.normalize_firm_mean(&mut hybrid_solution)?;
        let firm = hybrid_solution[..self.hybrid.firms()].to_vec();
        let reduced = operator.reduce_full_firm(&firm)?;
        let operator_applications = iterations.checked_add(replacements).ok_or_else(|| {
            BackendError::new(
                ErrorCode::ResourceLimit,
                "cmg_full_v2",
                "standalone CMG operator-application count overflow",
            )
        })?;
        Ok((
            firm,
            reduced,
            PcgReceipt {
                iterations,
                relative_residual: 0.0,
                residual_replacements: replacements,
                operator_applications,
                preconditioner_applications: iterations,
                zero_rhs,
            },
        ))
    }

    #[allow(clippy::too_many_arguments)]
    fn extract_solved_column(
        &self,
        operator: &TwoWayOperator<'_>,
        worker_rhs: &[f64],
        firm_rhs: &[f64],
        right_hand_sides: &[f64],
        column: usize,
        solved_column: FullCmgSolvedColumn,
        full_residual_tolerance: f64,
    ) -> Result<FullCmgExtractedColumn> {
        // Stata APIs are caller-thread only. The caller polls before and after
        // every bounded parallel chunk; workers use an inert checker.
        let mut interrupt = NeverInterrupt;
        let worker_begin = column * operator.problem().workers();
        let firm_begin = column * operator.problem().firms();
        let (firm, reduced, mut receipt) = self.finish_solved_column(operator, solved_column)?;
        let rhs_begin = column * self.hybrid.vertices();
        receipt.relative_residual = reduced_relative_residual(
            operator,
            &reduced,
            &right_hand_sides[rhs_begin..rhs_begin + self.hybrid.firms()],
            &mut interrupt,
        )?;
        receipt.zero_rhs =
            stable_norm(&right_hand_sides[rhs_begin..rhs_begin + self.hybrid.firms()]) == 0.0;
        let worker = operator.reconstruct_worker_with_interrupt(
            &worker_rhs[worker_begin..worker_begin + operator.problem().workers()],
            &firm,
            &mut interrupt,
        )?;
        let residual = operator.full_residual_with_interrupt(
            &worker,
            &firm,
            &worker_rhs[worker_begin..worker_begin + operator.problem().workers()],
            &firm_rhs[firm_begin..firm_begin + operator.problem().firms()],
            &mut interrupt,
        )?;
        if residual.relative_norm > full_residual_tolerance {
            return Err(BackendError::new(
                ErrorCode::FullResidualFailed,
                "cmg_full_v2",
                format!(
                    "zero-based RHS column {column}: complete residual {} exceeds tolerance {full_residual_tolerance}",
                    residual.relative_norm
                ),
            ));
        }
        Ok(FullCmgExtractedColumn {
            solution: TwoWaySolution {
                worker,
                firm,
                reduced_firm: reduced,
                residual,
            },
            receipt,
        })
    }
}

fn complete_residual_tolerance(tolerance: f64) -> f64 {
    (10.0 * tolerance).max(1.0e-11)
}

fn full_pcg_options(options: PcgOptions) -> Result<FullPcgOptions> {
    options.validate()?;
    Ok(FullPcgOptions {
        relative_tolerance: options.tolerance,
        absolute_tolerance: 0.0,
        max_iterations: usize::try_from(options.maximum_iterations).map_err(|_| {
            BackendError::new(
                ErrorCode::ResourceLimit,
                "cmg_full_v2",
                "maximum iteration count is not representable",
            )
        })?,
        residual_recompute_interval: usize::try_from(options.residual_replacement_interval)
            .map_err(|_| {
                BackendError::new(
                    ErrorCode::ResourceLimit,
                    "cmg_full_v2",
                    "residual replacement interval is not representable",
                )
            })?,
        validation: ValidationOptions::default(),
    })
}

fn validate_rhs(
    operator: &TwoWayOperator<'_>,
    worker_rhs: &[f64],
    firm_rhs: &[f64],
    columns: usize,
) -> Result<()> {
    if columns == 0
        || worker_rhs.len() != operator.problem().workers().saturating_mul(columns)
        || firm_rhs.len() != operator.problem().firms().saturating_mul(columns)
    {
        return Err(BackendError::invalid(
            "cmg_full_v2",
            "direct hybrid batch RHS arrays have incompatible dimensions",
        ));
    }
    Ok(())
}

fn reduced_relative_residual(
    operator: &TwoWayOperator<'_>,
    solution: &[f64],
    rhs: &[f64],
    interrupt: &mut dyn InterruptCheck,
) -> Result<f64> {
    let mut action = vec![0.0; rhs.len()];
    operator.apply_with_interrupt(solution, &mut action, interrupt)?;
    for (value, expected) in action.iter_mut().zip(rhs) {
        *value = *expected - *value;
    }
    let residual = stable_norm(&action);
    let scale = stable_norm(rhs);
    Ok(if scale == 0.0 {
        residual
    } else {
        residual / scale
    })
}

fn compatibility_receipt(
    solver: &ParallelPcgSolver,
    setup: &FullCmgSetupReceipt,
) -> Result<CmgReceipt> {
    let levels = solver.preconditioner().hierarchy().levels();
    let fine_edges = levels[0].graph().edge_count();
    let fine_vertices = levels[0].graph().vertex_count();
    let edge_total = levels
        .iter()
        .map(|level| level.graph().edge_count())
        .sum::<usize>();
    let vertex_total = levels
        .iter()
        .map(|level| level.graph().vertex_count())
        .sum::<usize>();
    let mut level = Vec::with_capacity(levels.len());
    for (index, current) in levels.iter().enumerate() {
        level.push(CmgLevelReceipt {
            level: index,
            vertices: current.graph().vertex_count(),
            edges: current.graph().edge_count(),
            coarse_vertices: levels
                .get(index + 1)
                .map(|next| next.graph().vertex_count()),
            reduction: levels.get(index + 1).map(|next| {
                1.0 - next.graph().vertex_count() as f64 / current.graph().vertex_count() as f64
            }),
            method: levels
                .get(index + 1)
                .map(|_| AggregationMethod::CanonicalPacking),
        });
    }
    Ok(CmgReceipt {
        levels: levels.len(),
        fine_vertices,
        fine_edges,
        terminal_vertices: levels
            .last()
            .expect("standalone CMG hierarchy is nonempty")
            .graph()
            .vertex_count(),
        edge_complexity: edge_total as f64 / fine_edges.max(1) as f64,
        vertex_complexity: vertex_total as f64 / fine_vertices.max(1) as f64,
        structural_bytes: setup
            .graph_copy_bytes
            .checked_add(setup.hierarchy_bytes)
            .and_then(|value| value.checked_add(setup.plan_bytes))
            .ok_or_else(|| {
                BackendError::new(
                    ErrorCode::ResourceLimit,
                    "cmg_full_v2",
                    "compatibility structural-byte count overflow",
                )
            })?,
        workspace_bytes: setup.workspace_bytes_each,
        preconditioner_bytes: 0,
        dense_factor_bytes: 0,
        level,
    })
}

fn scalar_column(result: PcgResult) -> FullCmgSolvedColumn {
    FullCmgSolvedColumn {
        iterations: result.iterations(),
        restarts: result.restarts(),
        initial_residual_norm: result.initial_residual_norm(),
        hybrid_solution: result.into_solution(),
    }
}

const fn scalar_execution(execution: ParallelPcgExecution) -> FullCmgExecution {
    match execution {
        ParallelPcgExecution::Serial => FullCmgExecution::Serial,
        ParallelPcgExecution::Planned => FullCmgExecution::Planned,
        ParallelPcgExecution::AcrossRightHandSides => FullCmgExecution::AcrossRightHandSides,
    }
}

fn graph_storage_bytes(graph: &Laplacian) -> Result<u64> {
    checked_sum_u64(&[
        checked_product_u64(&[to_u64(graph.edge_count(), "standalone edge count")?, 16])?,
        checked_product_u64(&[to_u64(graph.vertex_count(), "standalone vertex count")?, 8])?,
    ])
}

fn hierarchy_storage_bytes(solver: &ParallelPcgSolver) -> Result<u64> {
    let mut bytes = 0_u64;
    for level in solver.preconditioner().hierarchy().levels() {
        let level_bytes = checked_sum_u64(&[
            checked_product_u64(&[
                to_u64(level.graph().edge_count(), "hierarchy edge count")?,
                32,
            ])?,
            checked_product_u64(&[
                to_u64(level.graph().vertex_count(), "hierarchy vertex count")?,
                64,
            ])?,
        ])?;
        bytes = bytes.checked_add(level_bytes).ok_or_else(|| {
            BackendError::new(
                ErrorCode::ResourceLimit,
                "cmg_full_v2",
                "standalone hierarchy byte forecast overflow",
            )
        })?;
    }
    let terminal_bytes = to_u64(
        solver
            .preconditioner()
            .terminal_factor()
            .map_or(0, cmg::GroundedLdl::byte_len),
        "terminal factor bytes",
    )?;
    let repeat_bytes = checked_product_u64(&[
        to_u64(
            solver.preconditioner().repeat_counts().len(),
            "repeat count length",
        )?,
        to_u64(std::mem::size_of::<usize>(), "usize bytes")?,
    ])?;
    bytes = bytes
        .checked_add(to_u64(
            solver.preconditioner().component_metadata_bytes(),
            "component metadata bytes",
        )?)
        .and_then(|value| value.checked_add(terminal_bytes))
        .and_then(|value| value.checked_add(repeat_bytes))
        .ok_or_else(|| {
            BackendError::new(
                ErrorCode::ResourceLimit,
                "cmg_full_v2",
                "standalone hierarchy metadata forecast overflow",
            )
        })?;
    Ok(bytes)
}

fn to_u64(value: usize, context: &'static str) -> Result<u64> {
    u64::try_from(value).map_err(|_| {
        BackendError::new(
            ErrorCode::ResourceLimit,
            "cmg_full_v2",
            format!("{context} is not representable as u64"),
        )
    })
}

fn checked_product_u64(values: &[u64]) -> Result<u64> {
    values.iter().try_fold(1_u64, |product, value| {
        product.checked_mul(*value).ok_or_else(|| {
            BackendError::new(
                ErrorCode::ResourceLimit,
                "cmg_full_v2",
                "full-CMG memory product overflow",
            )
        })
    })
}

fn checked_sum_u64(values: &[u64]) -> Result<u64> {
    values.iter().try_fold(0_u64, |sum, value| {
        sum.checked_add(*value).ok_or_else(|| {
            BackendError::new(
                ErrorCode::ResourceLimit,
                "cmg_full_v2",
                "full-CMG memory sum overflow",
            )
        })
    })
}

fn checked_add_receipt(left: u64, right: u64, context: &'static str) -> Result<u64> {
    left.checked_add(right).ok_or_else(|| {
        BackendError::new(
            ErrorCode::ResourceLimit,
            "cmg_full_v2",
            format!("full-CMG {context} overflow"),
        )
    })
}

fn map_setup_error(error: CmgError, context: &'static str) -> BackendError {
    let code = match error {
        CmgError::MemoryBudgetExceeded { .. } => ErrorCode::ResourceLimit,
        _ => ErrorCode::CmgSetupFailed,
    };
    BackendError::new(code, "cmg_full_v2", format!("{context}: {error}"))
}

fn map_solve_error(error: CmgError, context: &'static str) -> BackendError {
    let code = match error {
        CmgError::MaximumIterations { .. } => ErrorCode::PcgMaxIterations,
        CmgError::PcgBreakdown { .. } => ErrorCode::PcgCurvatureBreakdown,
        CmgError::MemoryBudgetExceeded { .. } => ErrorCode::ResourceLimit,
        CmgError::ResidualVerificationFailed { .. } => ErrorCode::FullResidualFailed,
        _ => ErrorCode::CmgApplyFailed,
    };
    BackendError::new(code, "cmg_full_v2", format!("{context}: {error}"))
}
