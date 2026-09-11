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
use crate::interrupt::{CancellationInterrupt, CancellationToken, InterruptCheck, NeverInterrupt};
use crate::krylov::{PcgOptions, PcgReceipt};
use crate::operator::{stable_norm, SymmetricOperator, TwoWayOperator, TwoWaySolution};
use crate::problem::CompressedProblem;

pub const FULL_CMG_SCHEMA: &str = "CMG_FULL_V2";
pub const CMG_SOURCE_COMMIT: &str = "92a12f2d572ca56b30a035220953f9dd4bced999";
const MAX_COMPRESSED_BATCH_RHS: usize = 64;
const DEFAULT_PROBE_TOLERANCE: f64 = 1.0e-6;
// The deterministic fit remains deliberately tighter because it has no Monte
// Carlo envelope. Probe solves use the public phase tolerance directly; the
// independent complete-system gate, not an internal Schur/Krylov intermediate,
// is the release-blocking numerical certificate.
const FIT_INNER_TOLERANCE_RATIO: f64 = 0.01;
const PROBE_INNER_TOLERANCE_RATIO: f64 = 1.0;
const ALLOCATOR_ALLOWANCE_DIVISOR: u64 = 5;
const REFINEMENT_FACTORS: [f64; 3] = [0.1, 0.01, 0.001];
// Original RHS, refinement RHS, warm-start block, and newly solved block can
// coexist while a failing batch is refined. Parallel RHS assembly additionally
// retains one worker-scaled temporary per active column. All are admitted
// before RNG.
const MAXIMUM_LIVE_BATCH_VECTOR_BLOCKS: u64 = 4;

#[derive(Clone, Copy, Debug)]
pub struct FullCmgPlanOptions {
    pub memory_budget: crate::memory::MemoryBudget,
    pub threads: usize,
    pub fit_tolerance: f64,
    pub probe_tolerance: f64,
    pub maximum_batch_rhs: usize,
    pub preparation_peak_bytes: u64,
    pub prepared_persistent_bytes: u64,
    pub non_cmg_command_peak_bytes: u64,
}

impl FullCmgPlanOptions {
    pub fn production(threads: usize, fit_tolerance: f64, probe_tolerance: Option<f64>) -> Self {
        Self {
            memory_budget: crate::memory::MemoryBudget::Legacy,
            threads,
            fit_tolerance,
            probe_tolerance: probe_tolerance.unwrap_or(DEFAULT_PROBE_TOLERANCE),
            maximum_batch_rhs: MAX_COMPRESSED_BATCH_RHS,
            preparation_peak_bytes: 0,
            prepared_persistent_bytes: 0,
            non_cmg_command_peak_bytes: 0,
        }
    }

    #[must_use]
    pub fn with_prepared_memory(
        mut self,
        preparation_peak_bytes: u64,
        prepared_persistent_bytes: u64,
    ) -> Self {
        self.preparation_peak_bytes = preparation_peak_bytes;
        self.prepared_persistent_bytes = prepared_persistent_bytes;
        self
    }

    #[must_use]
    pub fn with_non_cmg_command_peak(mut self, non_cmg_command_peak_bytes: u64) -> Self {
        self.non_cmg_command_peak_bytes = non_cmg_command_peak_bytes;
        self
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
        if self.non_cmg_command_peak_bytes != 0
            && self.non_cmg_command_peak_bytes < self.prepared_persistent_bytes
        {
            return Err(BackendError::invariant(
                "cmg_full_v2",
                "non-CMG command peak is below prepared persistent storage",
            ));
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
    pub preparation_peak_bytes: u64,
    pub prepared_persistent_bytes: u64,
    pub non_cmg_command_peak_bytes: u64,
    pub pre_rng_forecast_bytes: u64,
    pub actual_retained_bytes: u64,
    pub allocator_allowance_bytes: u64,
    pub maximum_batch_rhs: usize,
    pub workspace_count: usize,
    pub fit_effective_tolerance: f64,
    pub probe_effective_tolerance: f64,
    pub fit_inner_tolerance: f64,
    pub probe_inner_tolerance: f64,
    pub graph_nanoseconds: u128,
    pub solver_nanoseconds: u128,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct FullCmgPrebuildMemory {
    pub hybrid_bytes: u64,
    pub graph_bytes: u64,
    pub hierarchy_bytes: u64,
    pub plan_bytes: u64,
    pub workspace_pool_bytes: u64,
    pub batch_vectors_bytes: u64,
    pub allocator_allowance_bytes: u64,
    pub full_cmg_peak_bytes: u64,
    pub whole_command_peak_bytes: u64,
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

    fn record_solve(
        &mut self,
        logical_rhs_count: usize,
        batches: &[FullCmgBatchReceipt],
        pcg_attempts: &[PcgReceipt],
        solutions: &[TwoWaySolution],
        refinement_attempts: usize,
        refined_columns: usize,
    ) -> Result<()> {
        self.rhs_count = checked_add_receipt(
            self.rhs_count,
            to_u64(logical_rhs_count, "logical RHS count")?,
            "RHS count",
        )?;
        self.refinement_attempts = checked_add_receipt(
            self.refinement_attempts,
            to_u64(refinement_attempts, "refinement attempt count")?,
            "refinement attempt count",
        )?;
        self.refined_columns = checked_add_receipt(
            self.refined_columns,
            to_u64(refined_columns, "refined column count")?,
            "refined column count",
        )?;
        for batch in batches {
            self.batch_calls = checked_add_receipt(self.batch_calls, 1, "batch call count")?;
            self.maximum_concurrency = self.maximum_concurrency.max(batch.concurrency);
            let counter = match batch.execution {
                FullCmgExecution::Serial => &mut self.serial_batches,
                FullCmgExecution::Planned => &mut self.planned_batches,
                FullCmgExecution::AcrossRightHandSides => &mut self.across_rhs_batches,
            };
            *counter = checked_add_receipt(*counter, 1, "batch strategy count")?;
            self.rhs_nanoseconds = self.rhs_nanoseconds.saturating_add(batch.rhs_nanoseconds);
            self.solve_nanoseconds = self
                .solve_nanoseconds
                .saturating_add(batch.solve_nanoseconds);
            self.extraction_nanoseconds = self
                .extraction_nanoseconds
                .saturating_add(batch.extraction_nanoseconds);
        }
        for receipt in pcg_attempts {
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
    cancellation: Option<CancellationToken>,
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
        let cancellation = interrupt.cancellation_token();
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
        let prebuild = prebuild_memory_forecast(problem, plan)?;
        if plan.memory_budget == crate::memory::MemoryBudget::Legacy
            && prebuild.whole_command_peak_bytes > memory_limit_bytes
        {
            return Err(BackendError::new(
                ErrorCode::ResourceLimit,
                "cmg_full_v2_memory_preflight",
                format!(
                    "full-CMG pre-RNG forecast {} exceeds the declared command limit {}",
                    prebuild.whole_command_peak_bytes, memory_limit_bytes
                ),
            ));
        }
        let workspace_budget = plan
            .memory_budget
            .hard_limit(memory_limit_bytes)
            .map(usize::try_from)
            .transpose()
            .map_err(|_| {
                BackendError::new(
                    ErrorCode::ResourceLimit,
                    "cmg_full_v2",
                    "workspace budget is not representable",
                )
            })?;

        let graph_start = Instant::now();
        let hybrid = HybridGraph::from_problem_degree_four_with_interrupt(problem, interrupt)?;
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
        let parallel_options = ParallelOptions {
            threads: plan.threads,
            workspace_memory_budget_bytes: workspace_budget,
            ..ParallelOptions::default()
        };
        let solver = match &cancellation {
            Some(cancellation) => ParallelPcgSolver::build_cancellable(
                &graph,
                FullCmgOptions::default(),
                parallel_options,
                cancellation.atomic_flag(),
            ),
            None => ParallelPcgSolver::build(&graph, FullCmgOptions::default(), parallel_options),
        }
        .map_err(|error| map_setup_error(error, "hierarchy construction"))?;
        let solver_nanoseconds = solver_start.elapsed().as_nanos();
        interrupt.checkpoint("cmg_full_v2_solver_complete")?;

        let maximum_batch = solver
            .select_batch_execution(plan.maximum_batch_rhs)
            .map_err(|error| map_setup_error(error, "batch admission"))?;
        let graph_copy_bytes = graph_storage_bytes(&graph)?;
        let hierarchy_bytes = if plan.memory_budget == crate::memory::MemoryBudget::Legacy {
            hierarchy_storage_bytes(&solver)?
        } else {
            to_u64(
                solver.preconditioner().retained_bytes(),
                "retained hierarchy capacity",
            )?
        };
        let plan_bytes = to_u64(solver.plan().byte_len(), "standalone plan bytes")?;
        let workspace_bytes_each = to_u64(
            maximum_batch.workspace_bytes_each(),
            "standalone workspace bytes",
        )?;
        let admitted_workspace_pool_bytes = to_u64(
            maximum_batch.workspace_pool_bytes(),
            "standalone workspace pool bytes",
        )?;
        let batch_vectors = admitted_batch_vector_bytes(
            to_u64(hybrid.vertices(), "hybrid vertices")?,
            to_u64(problem.workers(), "worker count")?,
            plan,
        )?;
        let full_cmg_retained = checked_sum_u64(&[
            hybrid.predicted_bytes(),
            if plan.memory_budget == crate::memory::MemoryBudget::Legacy {
                graph_copy_bytes
            } else {
                0
            },
            hierarchy_bytes,
            plan_bytes,
            admitted_workspace_pool_bytes,
            if plan.memory_budget == crate::memory::MemoryBudget::Legacy {
                batch_vectors
            } else {
                0
            },
        ])?;
        let allocator_allowance = full_cmg_retained / ALLOCATOR_ALLOWANCE_DIVISOR;
        let full_cmg_peak = checked_sum_u64(&[full_cmg_retained, allocator_allowance])?;
        let admitted_peak_bytes = plan.preparation_peak_bytes.max(checked_sum_u64(&[
            plan.non_cmg_command_peak_bytes,
            full_cmg_peak,
        ])?);
        if hybrid.predicted_bytes() > prebuild.hybrid_bytes
            || graph_copy_bytes > prebuild.graph_bytes
            || hierarchy_bytes > prebuild.hierarchy_bytes
            || plan_bytes > prebuild.plan_bytes
            || admitted_workspace_pool_bytes > prebuild.workspace_pool_bytes
            || batch_vectors > prebuild.batch_vectors_bytes
            || admitted_peak_bytes > prebuild.whole_command_peak_bytes
        {
            return Err(BackendError::invariant(
                "cmg_full_v2_memory_reconcile",
                "retained full-CMG allocation exceeded its pre-RNG component forecast",
            ));
        }
        if plan.memory_budget == crate::memory::MemoryBudget::Legacy
            && plan
                .memory_budget
                .rejects(admitted_peak_bytes, memory_limit_bytes)
        {
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
            preparation_peak_bytes: plan.preparation_peak_bytes,
            prepared_persistent_bytes: plan.prepared_persistent_bytes,
            non_cmg_command_peak_bytes: plan.non_cmg_command_peak_bytes,
            pre_rng_forecast_bytes: if plan.memory_budget == crate::memory::MemoryBudget::Legacy {
                prebuild.whole_command_peak_bytes
            } else {
                admitted_peak_bytes
            },
            actual_retained_bytes: full_cmg_retained,
            allocator_allowance_bytes: allocator_allowance,
            maximum_batch_rhs: plan.maximum_batch_rhs,
            workspace_count: maximum_batch.concurrency(),
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
            cancellation,
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

    pub(crate) fn reconcile_memory(&self, solve_peak: u64) -> Result<()> {
        let mut receipt = self
            .receipt
            .lock()
            .map_err(|_| BackendError::invariant("cmg_full_v2", "receipt lock poisoned"))?;
        let setup = &mut receipt.setup;
        // The final phase forecast includes the hierarchy, pool and reserve
        // once. Replace the preliminary maximum-width planning figure.
        setup.non_cmg_command_peak_bytes = solve_peak
            .checked_sub(setup.actual_retained_bytes + setup.allocator_allowance_bytes)
            .ok_or_else(|| {
                BackendError::invariant("cmg_full_v2", "final forecast omits retained CMG")
            })?;
        setup.admitted_peak_bytes = setup.preparation_peak_bytes.max(solve_peak);
        setup.pre_rng_forecast_bytes = setup.admitted_peak_bytes;
        Ok(())
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
        let rhs_values = self.hybrid.vertices().checked_mul(columns).ok_or_else(|| {
            BackendError::new(
                ErrorCode::ResourceLimit,
                "cmg_full_v2_rhs",
                "direct hybrid RHS block length overflow",
            )
        })?;
        let mut right_hand_sides = fallible_zeroed(rhs_values, "direct hybrid RHS block")?;
        if columns > 1 && self.setup.threads > 1 {
            let mut column_indices = Vec::new();
            column_indices.try_reserve_exact(columns).map_err(|_| {
                BackendError::new(
                    ErrorCode::AllocationFailed,
                    "cmg_full_v2_rhs",
                    "could not allocate the admitted RHS-column index",
                )
            })?;
            column_indices.extend(0..columns);
            let schur_columns = self.solver.vckss_map_ordered(column_indices, |column| {
                let worker_begin = column * operator.problem().workers();
                let firm_begin = column * operator.problem().firms();
                match &self.cancellation {
                    Some(cancellation) => operator.schur_rhs_with_interrupt(
                        &worker_rhs[worker_begin..worker_begin + operator.problem().workers()],
                        &firm_rhs[firm_begin..firm_begin + operator.problem().firms()],
                        &mut CancellationInterrupt::new(cancellation.clone()),
                    ),
                    None => operator.schur_rhs(
                        &worker_rhs[worker_begin..worker_begin + operator.problem().workers()],
                        &firm_rhs[firm_begin..firm_begin + operator.problem().firms()],
                    ),
                }
            });
            for (column, schur) in schur_columns.into_iter().enumerate() {
                interrupt.checkpoint("cmg_full_v2_rhs_column_complete")?;
                let rhs_begin = column * self.hybrid.vertices();
                right_hand_sides[rhs_begin..rhs_begin + self.hybrid.firms()]
                    .copy_from_slice(&schur?);
            }
        } else {
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
                right_hand_sides[rhs_begin..rhs_begin + self.hybrid.firms()]
                    .copy_from_slice(&schur);
            }
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
            self.solve_scalar_columns(&right_hand_sides, None, columns, full_options)?;
        let solve_nanoseconds = solve_start.elapsed().as_nanos();
        interrupt.checkpoint("cmg_full_v2_solve_complete")?;

        let extraction_start = Instant::now();
        let mut initial_index = Vec::new();
        initial_index.try_reserve_exact(columns).map_err(|_| {
            BackendError::new(
                ErrorCode::AllocationFailed,
                "cmg_full_v2_extract",
                "could not allocate the admitted source-column index",
            )
        })?;
        initial_index.extend(0..columns);
        let extracted = self.extract_columns_with_interrupt(
            operator,
            worker_rhs,
            firm_rhs,
            &right_hand_sides,
            &initial_index,
            solved_columns,
            interrupt,
        )?;
        let extraction_nanoseconds = extraction_start.elapsed().as_nanos();
        let extraction_concurrency = self.setup.threads.min(columns).max(1);
        let mut solution = Vec::new();
        let mut receipts = Vec::new();
        let mut pcg_attempts = Vec::new();
        solution.try_reserve_exact(columns).map_err(|_| {
            BackendError::new(
                ErrorCode::AllocationFailed,
                "cmg_full_v2_extract",
                "could not allocate the admitted final-column solution",
            )
        })?;
        receipts.try_reserve_exact(columns).map_err(|_| {
            BackendError::new(
                ErrorCode::AllocationFailed,
                "cmg_full_v2_extract",
                "could not allocate the admitted final-column receipt",
            )
        })?;
        pcg_attempts.try_reserve_exact(columns).map_err(|_| {
            BackendError::new(
                ErrorCode::AllocationFailed,
                "cmg_full_v2_extract",
                "could not allocate the admitted PCG-attempt receipt",
            )
        })?;
        for value in extracted {
            pcg_attempts.push(value.receipt.clone());
            receipts.push(value.receipt);
            solution.push(value.solution);
        }
        let initial_receipt = FullCmgBatchReceipt {
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
        let mut batch_receipts = vec![initial_receipt];
        let mut failing = failing_columns(&solution, &receipts, full_residual_tolerance)?;
        let mut refinement_attempts = 0_usize;
        let mut refined_columns = 0_usize;
        for factor in REFINEMENT_FACTORS {
            if failing.is_empty() {
                break;
            }
            interrupt.checkpoint("cmg_full_v2_refinement")?;
            refinement_attempts += 1;
            refined_columns = refined_columns.checked_add(failing.len()).ok_or_else(|| {
                BackendError::new(
                    ErrorCode::ResourceLimit,
                    "cmg_full_v2_refinement",
                    "refined-column count overflow",
                )
            })?;
            let refinement_rhs_start = Instant::now();
            let mut refinement_rhs = Vec::new();
            let mut refinement_initial = Vec::new();
            let refinement_values = self
                .hybrid
                .vertices()
                .checked_mul(failing.len())
                .ok_or_else(|| {
                    BackendError::new(
                        ErrorCode::ResourceLimit,
                        "cmg_full_v2_refinement",
                        "refinement RHS block length overflow",
                    )
                })?;
            refinement_rhs
                .try_reserve_exact(refinement_values)
                .map_err(|_| {
                    BackendError::new(
                        ErrorCode::AllocationFailed,
                        "cmg_full_v2_refinement",
                        "could not allocate the admitted refinement RHS block",
                    )
                })?;
            refinement_initial
                .try_reserve_exact(refinement_values)
                .map_err(|_| {
                    BackendError::new(
                        ErrorCode::AllocationFailed,
                        "cmg_full_v2_refinement",
                        "could not allocate the admitted refinement warm-start block",
                    )
                })?;
            refinement_initial.resize(refinement_values, 0.0);
            for (refinement_column, &column) in failing.iter().enumerate() {
                interrupt.checkpoint("cmg_full_v2_refinement_rhs")?;
                let begin = column * self.hybrid.vertices();
                refinement_rhs
                    .extend_from_slice(&right_hand_sides[begin..begin + self.hybrid.vertices()]);
                let initial_begin = refinement_column * self.hybrid.vertices();
                self.hybrid.lift_firm_into(
                    &solution[column].firm,
                    &mut refinement_initial[initial_begin..initial_begin + self.hybrid.vertices()],
                )?;
            }
            let refinement_rhs_nanoseconds = refinement_rhs_start.elapsed().as_nanos();
            let mut refinement_options = pcg;
            refinement_options.tolerance *= factor;
            let refinement_solve_start = Instant::now();
            let (refinement_execution, refined_solved) = self.solve_scalar_columns(
                &refinement_rhs,
                Some(&refinement_initial),
                failing.len(),
                full_pcg_options(refinement_options)?,
            )?;
            let refinement_solve_nanoseconds = refinement_solve_start.elapsed().as_nanos();
            interrupt.checkpoint("cmg_full_v2_refinement_solve_complete")?;
            let refinement_extract_start = Instant::now();
            let refined = self.extract_columns_with_interrupt(
                operator,
                worker_rhs,
                firm_rhs,
                &refinement_rhs,
                &failing,
                refined_solved,
                interrupt,
            )?;
            let refinement_extraction_nanoseconds = refinement_extract_start.elapsed().as_nanos();
            for (&column, value) in failing.iter().zip(refined) {
                pcg_attempts.push(value.receipt.clone());
                solution[column] = value.solution;
                receipts[column] = value.receipt;
            }
            batch_receipts.push(FullCmgBatchReceipt {
                execution: refinement_execution,
                rhs_count: failing.len(),
                concurrency: self
                    .solver
                    .select_batch_execution(failing.len())
                    .map_err(|error| map_solve_error(error, "refinement routing receipt"))?
                    .concurrency(),
                extraction_concurrency: self.setup.threads.min(failing.len()).max(1),
                rhs_nanoseconds: refinement_rhs_nanoseconds,
                solve_nanoseconds: refinement_solve_nanoseconds,
                extraction_nanoseconds: refinement_extraction_nanoseconds,
            });
            failing = failing_columns(&solution, &receipts, full_residual_tolerance)?;
        }
        if let Some(&column) = failing.first() {
            return Err(BackendError::new(
                ErrorCode::FullResidualFailed,
                "cmg_full_v2_refinement",
                format!(
                    "zero-based RHS column {column}: reduced residual {} or complete residual {} exceeds tolerance {full_residual_tolerance} after {} frozen same-route refinements",
                    receipts[column].relative_residual,
                    solution[column].residual.relative_norm,
                    REFINEMENT_FACTORS.len()
                ),
            ));
        }
        self.receipt
            .lock()
            .map_err(|_| {
                BackendError::new(
                    ErrorCode::ContextPoisoned,
                    "cmg_full_v2",
                    "full-CMG receipt mutex is poisoned",
                )
            })?
            .record_solve(
                columns,
                &batch_receipts,
                &pcg_attempts,
                &solution,
                refinement_attempts,
                refined_columns,
            )?;
        Ok(FullCmgDirectSolve {
            solution,
            pcg: receipts,
            receipt: initial_receipt,
        })
    }

    fn solve_scalar_columns(
        &self,
        right_hand_sides: &[f64],
        initial_guesses: Option<&[f64]>,
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
        let solved = match (&self.cancellation, initial_guesses) {
            (Some(cancellation), Some(initial_guesses)) => self
                .solver
                .vckss_solve_contiguous_columns_from_initial_guesses_with_workspace_cancellable(
                    right_hand_sides,
                    initial_guesses,
                    columns,
                    options,
                    &mut workspace,
                    cancellation.atomic_flag(),
                ),
            (None, Some(initial_guesses)) => self
                .solver
                .vckss_solve_contiguous_columns_from_initial_guesses_with_workspace(
                    right_hand_sides,
                    initial_guesses,
                    columns,
                    options,
                    &mut workspace,
                ),
            (Some(cancellation), None) => self
                .solver
                .vckss_solve_contiguous_columns_with_workspace_cancellable(
                    right_hand_sides,
                    columns,
                    options,
                    &mut workspace,
                    cancellation.atomic_flag(),
                ),
            (None, None) => self.solver.vckss_solve_contiguous_columns_with_workspace(
                right_hand_sides,
                columns,
                options,
                &mut workspace,
            ),
        }
        .map_err(|error| map_solve_error(error, "direct hybrid batch"))?;
        let execution = scalar_execution(report.execution());
        let mut columns = Vec::new();
        columns.try_reserve_exact(solved.len()).map_err(|_| {
            BackendError::new(
                ErrorCode::AllocationFailed,
                "cmg_full_v2_solve",
                "could not allocate the admitted solved-column receipt",
            )
        })?;
        columns.extend(solved.into_iter().map(scalar_column));
        Ok((execution, columns))
    }

    #[allow(clippy::too_many_arguments)]
    fn extract_columns_with_interrupt(
        &self,
        operator: &TwoWayOperator<'_>,
        worker_rhs: &[f64],
        firm_rhs: &[f64],
        right_hand_sides: &[f64],
        source_columns: &[usize],
        solved_columns: Vec<FullCmgSolvedColumn>,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<Vec<FullCmgExtractedColumn>> {
        if source_columns.len() != solved_columns.len() {
            return Err(BackendError::invariant(
                "cmg_full_v2_extract",
                "source-column and solved-column counts differ",
            ));
        }
        let extraction_concurrency = self.setup.threads.min(source_columns.len()).max(1);
        let mut output = Vec::new();
        output
            .try_reserve_exact(source_columns.len())
            .map_err(|_| {
                BackendError::new(
                    ErrorCode::AllocationFailed,
                    "cmg_full_v2_extract",
                    "could not allocate the admitted extracted-column result",
                )
            })?;
        let mut pending = source_columns
            .iter()
            .copied()
            .zip(solved_columns)
            .enumerate();
        loop {
            interrupt.checkpoint("cmg_full_v2_extract_chunk")?;
            let mut chunk = Vec::new();
            chunk
                .try_reserve_exact(extraction_concurrency)
                .map_err(|_| {
                    BackendError::new(
                        ErrorCode::AllocationFailed,
                        "cmg_full_v2_extract",
                        "could not allocate the admitted extraction chunk",
                    )
                })?;
            chunk.extend(pending.by_ref().take(extraction_concurrency));
            if chunk.is_empty() {
                break;
            }
            let extracted = self.solver.vckss_map_ordered(
                chunk,
                |(rhs_column, (source_column, solved_column))| match &self.cancellation {
                    Some(cancellation) => self.extract_solved_column(
                        operator,
                        worker_rhs,
                        firm_rhs,
                        right_hand_sides,
                        source_column,
                        rhs_column,
                        solved_column,
                        &mut CancellationInterrupt::new(cancellation.clone()),
                    ),
                    None => self.extract_solved_column(
                        operator,
                        worker_rhs,
                        firm_rhs,
                        right_hand_sides,
                        source_column,
                        rhs_column,
                        solved_column,
                        &mut NeverInterrupt,
                    ),
                },
            );
            for extracted_column in extracted {
                interrupt.checkpoint("cmg_full_v2_extract_complete")?;
                output.push(extracted_column?);
            }
        }
        Ok(output)
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
        let mut firm = Vec::new();
        firm.try_reserve_exact(self.hybrid.firms()).map_err(|_| {
            BackendError::new(
                ErrorCode::AllocationFailed,
                "cmg_full_v2_extract",
                "could not allocate the admitted firm solution",
            )
        })?;
        firm.extend_from_slice(&hybrid_solution[..self.hybrid.firms()]);
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
        source_column: usize,
        rhs_column: usize,
        solved_column: FullCmgSolvedColumn,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<FullCmgExtractedColumn> {
        let worker_begin = source_column * operator.problem().workers();
        let firm_begin = source_column * operator.problem().firms();
        let (firm, reduced, mut receipt) = self.finish_solved_column(operator, solved_column)?;
        let rhs_begin = rhs_column * self.hybrid.vertices();
        receipt.relative_residual = reduced_relative_residual(
            operator,
            &reduced,
            &right_hand_sides[rhs_begin..rhs_begin + self.hybrid.firms()],
            interrupt,
        )?;
        receipt.zero_rhs =
            stable_norm(&right_hand_sides[rhs_begin..rhs_begin + self.hybrid.firms()]) == 0.0;
        let worker = operator.reconstruct_worker_with_interrupt(
            &worker_rhs[worker_begin..worker_begin + operator.problem().workers()],
            &firm,
            interrupt,
        )?;
        let residual = operator.full_residual_with_interrupt(
            &worker,
            &firm,
            &worker_rhs[worker_begin..worker_begin + operator.problem().workers()],
            &firm_rhs[firm_begin..firm_begin + operator.problem().firms()],
            interrupt,
        )?;
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
    let worker_values = operator.problem().workers().checked_mul(columns);
    let firm_values = operator.problem().firms().checked_mul(columns);
    if columns == 0
        || worker_values != Some(worker_rhs.len())
        || firm_values != Some(firm_rhs.len())
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
    let mut action = fallible_zeroed(rhs.len(), "reduced residual workspace")?;
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

fn failing_columns(
    solutions: &[TwoWaySolution],
    receipts: &[PcgReceipt],
    tolerance: f64,
) -> Result<Vec<usize>> {
    if solutions.len() != receipts.len() {
        return Err(BackendError::invalid(
            "cmg_full_v2_refinement",
            "solution and reduced-residual receipt counts differ",
        ));
    }
    let mut failing = Vec::new();
    failing.try_reserve_exact(solutions.len()).map_err(|_| {
        BackendError::new(
            ErrorCode::AllocationFailed,
            "cmg_full_v2_refinement",
            "could not allocate the admitted refinement-column index",
        )
    })?;
    for (column, (solution, receipt)) in solutions.iter().zip(receipts).enumerate() {
        let complete = solution.residual.relative_norm;
        let reduced = receipt.relative_residual;
        if !complete.is_finite()
            || complete > tolerance
            || !reduced.is_finite()
            || reduced > tolerance
        {
            failing.push(column);
        }
    }
    Ok(failing)
}

fn fallible_zeroed(length: usize, context: &'static str) -> Result<Vec<f64>> {
    let mut values = Vec::new();
    values.try_reserve_exact(length).map_err(|_| {
        BackendError::new(
            ErrorCode::AllocationFailed,
            "cmg_full_v2_memory",
            format!("could not allocate the admitted {context}"),
        )
    })?;
    values.resize(length, 0.0);
    Ok(values)
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

pub(crate) fn prebuild_memory_forecast(
    problem: &CompressedProblem,
    plan: FullCmgPlanOptions,
) -> Result<FullCmgPrebuildMemory> {
    let firms = to_u64(problem.firms(), "firm count")?;
    let workers = to_u64(problem.workers(), "worker count")?;
    let cells = to_u64(problem.cells(), "cell count")?;
    prebuild_memory_forecast_counts(firms, workers, cells, plan)
}

fn prebuild_memory_forecast_counts(
    firms: u64,
    workers: u64,
    cells: u64,
    plan: FullCmgPlanOptions,
) -> Result<FullCmgPrebuildMemory> {
    let vertices = firms.checked_add(workers).ok_or_else(|| {
        BackendError::new(
            ErrorCode::ResourceLimit,
            "cmg_full_v2_memory_preflight",
            "hybrid vertex upper bound overflow",
        )
    })?;
    // Degree four contributes six clique edges for four compressed cells;
    // all other degrees contribute no more than one edge per cell. Retain a
    // checked conservative upper bound before constructing the graph or RNG.
    let edges = cells
        .checked_mul(3)
        .and_then(|n| n.checked_add(1))
        .ok_or_else(|| {
            BackendError::new(
                ErrorCode::ResourceLimit,
                "cmg_full_v2_memory_preflight",
                "degree-four edge bound overflow",
            )
        })?
        / 2;
    let hybrid_bytes = checked_sum_u64(&[
        checked_product_u64(&[vertices, 48])?,
        checked_product_u64(&[edges, 40])?,
        checked_product_u64(&[workers, 4])?,
    ])?;
    let graph_bytes = checked_sum_u64(&[
        checked_product_u64(&[edges, 16])?,
        checked_product_u64(&[vertices, 8])?,
    ])?;
    let initial_nonzeros = edges
        .checked_mul(2)
        .and_then(|value| value.checked_add(vertices))
        .ok_or_else(|| {
            BackendError::new(
                ErrorCode::ResourceLimit,
                "cmg_full_v2_memory_preflight",
                "hybrid matrix-nonzero upper bound overflow",
            )
        })?;
    // The pinned hierarchy retains at most the fine graph plus five times its
    // initial matrix nonzeros before its deterministic fill guard terminates.
    // The byte multipliers cover graph/aggregation/centering metadata, CSR
    // plan copies, and every vector owned by one PCG/V-cycle workspace.
    let hierarchy_bytes = checked_product_u64(&[initial_nonzeros, 512])?;
    let plan_bytes = checked_product_u64(&[initial_nonzeros, 128])?;
    let workspace_each = checked_product_u64(&[initial_nonzeros, 512])?;
    let workspace_count = to_u64(plan.threads.min(plan.maximum_batch_rhs), "workspace count")?;
    let workspace_pool_bytes = checked_product_u64(&[workspace_each, workspace_count])?;
    let batch_vectors_bytes = admitted_batch_vector_bytes(vertices, workers, plan)?;
    let retained = checked_sum_u64(&[
        hybrid_bytes,
        graph_bytes,
        hierarchy_bytes,
        plan_bytes,
        workspace_pool_bytes,
        batch_vectors_bytes,
    ])?;
    let allocator_allowance_bytes = retained / ALLOCATOR_ALLOWANCE_DIVISOR;
    let full_cmg_peak_bytes = checked_sum_u64(&[retained, allocator_allowance_bytes])?;
    let whole_command_peak_bytes = plan.preparation_peak_bytes.max(checked_sum_u64(&[
        plan.non_cmg_command_peak_bytes,
        full_cmg_peak_bytes,
    ])?);
    Ok(FullCmgPrebuildMemory {
        hybrid_bytes,
        graph_bytes,
        hierarchy_bytes,
        plan_bytes,
        workspace_pool_bytes,
        batch_vectors_bytes,
        allocator_allowance_bytes,
        full_cmg_peak_bytes,
        whole_command_peak_bytes,
    })
}

fn admitted_batch_vector_bytes(
    vertices: u64,
    workers: u64,
    plan: FullCmgPlanOptions,
) -> Result<u64> {
    let maximum_batch_rhs = to_u64(plan.maximum_batch_rhs, "maximum batch RHS")?;
    let live_blocks = checked_product_u64(&[
        vertices,
        maximum_batch_rhs,
        8,
        MAXIMUM_LIVE_BATCH_VECTOR_BLOCKS,
    ])?;
    let parallel_rhs_workers = if plan.threads > 1 && plan.maximum_batch_rhs > 1 {
        checked_product_u64(&[
            workers,
            to_u64(
                plan.threads.min(plan.maximum_batch_rhs),
                "parallel RHS columns",
            )?,
            8,
        ])?
    } else {
        0
    };
    checked_sum_u64(&[live_blocks, parallel_rhs_workers])
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
        CmgError::Cancelled { .. } => ErrorCode::UserBreak,
        CmgError::MemoryBudgetExceeded { .. } => ErrorCode::ResourceLimit,
        _ => ErrorCode::CmgSetupFailed,
    };
    BackendError::new(code, "cmg_full_v2", format!("{context}: {error}"))
}

fn map_solve_error(error: CmgError, context: &'static str) -> BackendError {
    let code = match error {
        CmgError::Cancelled { .. } => ErrorCode::UserBreak,
        CmgError::MaximumIterations { .. } => ErrorCode::PcgMaxIterations,
        CmgError::PcgBreakdown { .. } => ErrorCode::PcgCurvatureBreakdown,
        CmgError::MemoryBudgetExceeded { .. } => ErrorCode::ResourceLimit,
        CmgError::ResidualVerificationFailed { .. } => ErrorCode::FullResidualFailed,
        _ => ErrorCode::CmgApplyFailed,
    };
    BackendError::new(code, "cmg_full_v2", format!("{context}: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interrupt::NeverInterrupt;
    use crate::operator::FullResidual;
    use crate::problem::CanonicalInput;
    use crate::types::InputColumns;

    fn fixture() -> CompressedProblem {
        CanonicalInput::from_validated(
            InputColumns {
                worker: vec![1, 1, 2, 2, 3, 3, 4, 4],
                firm: vec![1, 2, 2, 3, 3, 4, 4, 1],
                deletion: (1..=8).collect(),
                outcome: vec![1.0, -0.5, 0.75, -1.25, 0.25, 1.5, -0.75, 0.5],
                frequency: vec![1; 8],
                target_weight: vec![1.0; 8],
                controls: Vec::new(),
            }
            .validate()
            .expect("fixture"),
        )
        .expect("canonical")
        .compress(&[true; 8])
        .expect("compressed")
    }

    fn test_plan() -> FullCmgPlanOptions {
        FullCmgPlanOptions::production(1, 1.0e-10, None)
            .with_prepared_memory(10_000, 2_000)
            .with_non_cmg_command_peak(20_000)
    }

    fn solution_with_residual(relative_norm: f64) -> TwoWaySolution {
        TwoWaySolution {
            worker: Vec::new(),
            firm: Vec::new(),
            reduced_firm: Vec::new(),
            residual: FullResidual {
                worker: Vec::new(),
                firm: Vec::new(),
                absolute_norm: relative_norm,
                relative_norm,
                rhs_norm: 1.0,
            },
        }
    }

    fn receipt_with_reduced_residual(relative_residual: f64) -> PcgReceipt {
        PcgReceipt {
            iterations: 1,
            relative_residual,
            residual_replacements: 1,
            operator_applications: 2,
            preconditioner_applications: 1,
            zero_rhs: false,
        }
    }

    #[test]
    fn memory_forecast_overflow_is_typed_before_allocation() {
        let error = prebuild_memory_forecast_counts(u64::MAX, 1, 1, test_plan())
            .expect_err("overflow must fail");
        assert_eq!(error.code, ErrorCode::ResourceLimit);
        assert_eq!(error.phase, "cmg_full_v2_memory_preflight");
    }

    #[test]
    fn degree_four_edge_forecast_rounds_up_and_rejects_overflow() {
        for (cells, edges) in [(0, 0), (1, 2), (2, 3), (3, 5), (4, 6), (5, 8)] {
            let forecast = prebuild_memory_forecast_counts(5, 1, cells, test_plan())
                .expect("bounded degree-four forecast");
            assert_eq!(forecast.graph_bytes, edges * 16 + 6 * 8);
            assert_eq!(forecast.hybrid_bytes, 6 * 48 + edges * 40 + 4);
        }
        let error = prebuild_memory_forecast_counts(5, 1, u64::MAX / 3 + 1, test_plan())
            .expect_err("degree-four edge multiplication must be checked");
        assert_eq!(error.code, ErrorCode::ResourceLimit);
        assert_eq!(error.phase, "cmg_full_v2_memory_preflight");
    }

    #[test]
    fn memory_forecast_boundary_and_retained_reconciliation_are_enforced() {
        let problem = fixture();
        let mut plan = test_plan();
        plan.maximum_batch_rhs = 2;
        let forecast = prebuild_memory_forecast(&problem, plan).expect("forecast");
        assert_eq!(
            forecast.batch_vectors_bytes,
            u64::try_from(problem.firms() + problem.workers()).unwrap()
                * u64::try_from(plan.maximum_batch_rhs).unwrap()
                * 8
                * MAXIMUM_LIVE_BATCH_VECTOR_BLOCKS
        );
        let pcg = PcgOptions {
            tolerance: 1.0e-10,
            maximum_iterations: 200,
            residual_replacement_interval: 20,
        };
        let error = FullCmgDirectSolver::prepare_with_interrupt(
            &problem,
            pcg,
            forecast.whole_command_peak_bytes - 1,
            plan,
            &mut NeverInterrupt,
        )
        .expect_err("one byte below the forecast must fail");
        assert_eq!(error.code, ErrorCode::ResourceLimit);
        assert_eq!(error.phase, "cmg_full_v2_memory_preflight");

        let solver = FullCmgDirectSolver::prepare_with_interrupt(
            &problem,
            pcg,
            forecast.whole_command_peak_bytes,
            plan,
            &mut NeverInterrupt,
        )
        .expect("exact forecast boundary");
        let receipt = solver.receipt().expect("receipt");
        assert_eq!(
            receipt.setup.pre_rng_forecast_bytes,
            forecast.whole_command_peak_bytes
        );
        assert!(receipt.setup.admitted_peak_bytes <= receipt.setup.pre_rng_forecast_bytes);
        assert_eq!(
            receipt.setup.allocator_allowance_bytes,
            receipt.setup.actual_retained_bytes / ALLOCATOR_ALLOWANCE_DIVISOR
        );
        assert_eq!(receipt.setup.maximum_batch_rhs, 2);
        assert_eq!(receipt.setup.workspace_count, 1);
    }

    #[test]
    fn parallel_rhs_forecast_admits_worker_scaled_temporaries() {
        let problem = fixture();
        let mut plan = test_plan();
        plan.threads = 2;
        plan.maximum_batch_rhs = 3;
        let forecast = prebuild_memory_forecast(&problem, plan).expect("parallel forecast");
        let vertices = u64::try_from(problem.firms() + problem.workers()).unwrap();
        let live_blocks = vertices
            * u64::try_from(plan.maximum_batch_rhs).unwrap()
            * 8
            * MAXIMUM_LIVE_BATCH_VECTOR_BLOCKS;
        let worker_temporaries =
            u64::try_from(problem.workers()).unwrap() * u64::try_from(plan.threads).unwrap() * 8;
        assert_eq!(
            forecast.batch_vectors_bytes,
            live_blocks + worker_temporaries
        );
    }

    #[test]
    fn refinement_schedule_and_column_gate_are_frozen() {
        assert_eq!(REFINEMENT_FACTORS, [0.1, 0.01, 0.001]);
        let solutions = vec![
            solution_with_residual(1.0e-7),
            solution_with_residual(1.0e-5),
            solution_with_residual(1.0e-7),
            solution_with_residual(f64::NAN),
        ];
        let receipts = vec![
            receipt_with_reduced_residual(1.0e-7),
            receipt_with_reduced_residual(1.0e-7),
            receipt_with_reduced_residual(1.0e-5),
            receipt_with_reduced_residual(1.0e-7),
        ];
        assert_eq!(
            failing_columns(&solutions, &receipts, 1.0e-6).expect("selector"),
            vec![1, 2, 3]
        );
    }
}
