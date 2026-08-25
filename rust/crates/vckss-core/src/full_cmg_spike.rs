// SPDX-License-Identifier: GPL-3.0-only

//! Private direct-hybrid performance spike backed by the standalone CMG crate.
//!
//! This module is deliberately absent from ordinary builds. It does not add a
//! public solver route or receipt identity; benchmark logs carry the private
//! `CMG_FULL_SPIKE_V1` marker instead.

use std::sync::Mutex;
use std::time::Instant;

use cmg_full::{
    CmgError, CmgOptions as FullCmgOptions, Laplacian, ParallelOptions, ParallelPcgBatchReport,
    ParallelPcgExecution, ParallelPcgSolver, ParallelPcgWorkspace, PcgOptions as FullPcgOptions,
    PcgResult, ValidationOptions,
};

use crate::cmg::{AggregationMethod, CmgLevelReceipt, CmgReceipt, HybridGraph};
use crate::error::{BackendError, ErrorCode, Result};
use crate::interrupt::InterruptCheck;
use crate::krylov::{PcgOptions, PcgReceipt};
use crate::operator::{stable_norm, SymmetricOperator, TwoWayOperator, TwoWaySolution};
use crate::problem::CompressedProblem;

pub(crate) const SPIKE_SCHEMA: &str = "CMG_FULL_SPIKE_V1";
pub(crate) const CMG_SOURCE_COMMIT: &str = "dbefbc5e3b442c6dde6e7861a66d82fd5ed24f10";
const PRIVATE_ENABLE_ENV: &str = "VCKSS_PRIVATE_CMG_FULL_V1";
const PRIVATE_THREADS_ENV: &str = "VCKSS_PRIVATE_CMG_THREADS";
const PRIVATE_DIAGNOSTICS_ENV: &str = "VCKSS_PRIVATE_CMG_DIAGNOSTICS";
const MAX_COMPRESSED_BATCH_RHS: usize = 64;
const PRIVATE_HYBRID_RELATIVE_TOLERANCE: f64 = 1.0e-15;

#[derive(Clone, Copy, Debug)]
pub(crate) struct FullCmgSpikeSetupReceipt {
    pub threads: usize,
    pub vertices: usize,
    pub edges: usize,
    pub graph_copy_bytes: u64,
    pub hierarchy_bytes: u64,
    pub plan_bytes: u64,
    pub workspace_bytes_each: u64,
    pub admitted_workspace_pool_bytes: u64,
    pub admitted_peak_bytes: u64,
    pub graph_nanoseconds: u128,
    pub solver_nanoseconds: u128,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct FullCmgSpikeBatchReceipt {
    pub execution: ParallelPcgExecution,
    pub rhs_count: usize,
    pub concurrency: usize,
    pub rhs_nanoseconds: u128,
    pub solve_nanoseconds: u128,
    pub extraction_nanoseconds: u128,
}

#[derive(Debug)]
pub(crate) struct FullCmgDirectSolve {
    pub solution: Vec<TwoWaySolution>,
    pub pcg: Vec<PcgReceipt>,
    pub receipt: FullCmgSpikeBatchReceipt,
}

#[derive(Debug)]
pub(crate) struct FullCmgDirectSolver {
    hybrid: HybridGraph,
    solver: ParallelPcgSolver,
    workspace: Mutex<ParallelPcgWorkspace>,
    setup: FullCmgSpikeSetupReceipt,
    compatibility_receipt: CmgReceipt,
    diagnostics: bool,
}

pub(crate) fn private_spike_requested() -> Result<bool> {
    match std::env::var_os(PRIVATE_ENABLE_ENV) {
        None => Ok(false),
        Some(value) if value == "1" => Ok(true),
        Some(_) => Err(BackendError::invalid(
            "cmg_full_spike",
            format!("{PRIVATE_ENABLE_ENV} must equal 1 when supplied"),
        )),
    }
}

impl FullCmgDirectSolver {
    pub(crate) fn prepare_with_interrupt(
        problem: &CompressedProblem,
        pcg: PcgOptions,
        memory_limit_bytes: u64,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<Self> {
        interrupt.checkpoint("cmg_full_spike_prepare")?;
        let threads = private_threads()?;
        let diagnostics =
            std::env::var_os(PRIVATE_DIAGNOSTICS_ENV).is_some_and(|value| value == "1");
        let workspace_budget = usize::try_from(memory_limit_bytes).map_err(|_| {
            BackendError::new(
                ErrorCode::ResourceLimit,
                "cmg_full_spike",
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
        interrupt.checkpoint("cmg_full_spike_graph_complete")?;

        let solver_start = Instant::now();
        let solver = ParallelPcgSolver::build(
            &graph,
            FullCmgOptions::default(),
            ParallelOptions {
                threads,
                workspace_memory_budget_bytes: Some(workspace_budget),
                ..ParallelOptions::default()
            },
        )
        .map_err(|error| map_setup_error(error, "hierarchy construction"))?;
        let solver_nanoseconds = solver_start.elapsed().as_nanos();
        interrupt.checkpoint("cmg_full_spike_solver_complete")?;

        let maximum_batch = solver
            .select_batch_execution(MAX_COMPRESSED_BATCH_RHS)
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
            to_u64(MAX_COMPRESSED_BATCH_RHS, "maximum batch RHS")?,
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
                "cmg_full_spike",
                "private whole-process memory admission overflow",
            )
        })?;
        if admitted_peak_bytes > memory_limit_bytes {
            return Err(BackendError::new(
                ErrorCode::ResourceLimit,
                "cmg_full_spike",
                format!(
                    "private full-CMG forecast {admitted_peak_bytes} exceeds CMG memory limit {memory_limit_bytes}"
                ),
            ));
        }

        let setup = FullCmgSpikeSetupReceipt {
            threads,
            vertices: hybrid.vertices(),
            edges: hybrid.edges().len(),
            graph_copy_bytes,
            hierarchy_bytes,
            plan_bytes,
            workspace_bytes_each,
            admitted_workspace_pool_bytes,
            admitted_peak_bytes,
            graph_nanoseconds,
            solver_nanoseconds,
        };
        let compatibility_receipt = compatibility_receipt(&solver, &setup)?;
        let workspace = Mutex::new(solver.workspace());
        let prepared = Self {
            hybrid,
            solver,
            workspace,
            setup,
            compatibility_receipt,
            diagnostics,
        };
        prepared.log_setup();
        pcg.validate()?;
        Ok(prepared)
    }

    pub(crate) fn compatibility_receipt(&self) -> &CmgReceipt {
        &self.compatibility_receipt
    }

    pub(crate) fn solve_batch_with_interrupt(
        &self,
        operator: &TwoWayOperator<'_>,
        worker_rhs: &[f64],
        firm_rhs: &[f64],
        columns: usize,
        pcg: PcgOptions,
        full_residual_tolerance: f64,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<FullCmgDirectSolve> {
        validate_rhs(operator, worker_rhs, firm_rhs, columns)?;
        interrupt.checkpoint("cmg_full_spike_rhs")?;
        let rhs_start = Instant::now();
        let mut right_hand_sides = Vec::with_capacity(columns);
        for column in 0..columns {
            interrupt.checkpoint("cmg_full_spike_rhs_column")?;
            let worker_begin = column * operator.problem().workers();
            let firm_begin = column * operator.problem().firms();
            let schur = operator.schur_rhs_with_interrupt(
                &worker_rhs[worker_begin..worker_begin + operator.problem().workers()],
                &firm_rhs[firm_begin..firm_begin + operator.problem().firms()],
                interrupt,
            )?;
            let mut rhs = vec![0.0; self.hybrid.vertices()];
            rhs[..self.hybrid.firms()].copy_from_slice(&schur);
            right_hand_sides.push(rhs);
        }
        let rhs_nanoseconds = rhs_start.elapsed().as_nanos();
        interrupt.checkpoint("cmg_full_spike_solve")?;

        let solve_start = Instant::now();
        let report = self
            .solver
            .select_batch_execution(columns)
            .map_err(|error| map_solve_error(error, "batch routing"))?;
        let mut workspace = self.workspace.lock().map_err(|_| {
            BackendError::new(
                ErrorCode::ContextPoisoned,
                "cmg_full_spike",
                "standalone CMG workspace mutex is poisoned",
            )
        })?;
        let solved = self
            .solver
            .solve_batch_with_workspace(&right_hand_sides, full_pcg_options(pcg)?, &mut workspace)
            .map_err(|error| map_solve_error(error, "direct hybrid batch"))?;
        let solve_nanoseconds = solve_start.elapsed().as_nanos();
        drop(workspace);
        interrupt.checkpoint("cmg_full_spike_solve_complete")?;

        let extraction_start = Instant::now();
        let mut solution = Vec::with_capacity(columns);
        let mut receipts = Vec::with_capacity(columns);
        for (column, pcg_result) in solved.into_results().into_iter().enumerate() {
            interrupt.checkpoint("cmg_full_spike_extract")?;
            let worker_begin = column * operator.problem().workers();
            let firm_begin = column * operator.problem().firms();
            let (firm, reduced, mut receipt) = self.finish_pcg_result(operator, pcg_result)?;
            receipt.relative_residual = reduced_relative_residual(
                operator,
                &reduced,
                &right_hand_sides[column][..self.hybrid.firms()],
                interrupt,
            )?;
            receipt.zero_rhs = stable_norm(&right_hand_sides[column][..self.hybrid.firms()]) == 0.0;
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
            if residual.relative_norm > full_residual_tolerance {
                return Err(BackendError::new(
                    ErrorCode::FullResidualFailed,
                    "cmg_full_spike",
                    format!(
                        "zero-based RHS column {column}: complete residual {} exceeds tolerance {full_residual_tolerance}",
                        residual.relative_norm
                    ),
                ));
            }
            solution.push(TwoWaySolution {
                worker,
                firm,
                reduced_firm: reduced,
                residual,
            });
            receipts.push(receipt);
        }
        let extraction_nanoseconds = extraction_start.elapsed().as_nanos();
        let receipt = batch_receipt(
            report,
            rhs_nanoseconds,
            solve_nanoseconds,
            extraction_nanoseconds,
        );
        self.log_batch(receipt, &receipts, &solution);
        Ok(FullCmgDirectSolve {
            solution,
            pcg: receipts,
            receipt,
        })
    }

    fn finish_pcg_result(
        &self,
        operator: &TwoWayOperator<'_>,
        result: PcgResult,
    ) -> Result<(Vec<f64>, Vec<f64>, PcgReceipt)> {
        let iterations = u32::try_from(result.iterations()).map_err(|_| {
            BackendError::new(
                ErrorCode::ResourceLimit,
                "cmg_full_spike",
                "standalone CMG iteration count exceeds u32",
            )
        })?;
        let replacements = u32::try_from(result.restarts()).map_err(|_| {
            BackendError::new(
                ErrorCode::ResourceLimit,
                "cmg_full_spike",
                "standalone CMG restart count exceeds u32",
            )
        })?;
        let zero_rhs = result.initial_residual_norm() == 0.0;
        let mut hybrid_solution = result.into_solution();
        self.hybrid.normalize_firm_mean(&mut hybrid_solution)?;
        let firm = hybrid_solution[..self.hybrid.firms()].to_vec();
        let reduced = operator.reduce_full_firm(&firm)?;
        let operator_applications = iterations.checked_add(replacements).ok_or_else(|| {
            BackendError::new(
                ErrorCode::ResourceLimit,
                "cmg_full_spike",
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

    fn log_setup(&self) {
        if !self.diagnostics {
            return;
        }
        eprintln!(
            "{SPIKE_SCHEMA} SETUP cmg_commit={CMG_SOURCE_COMMIT} threads={} vertices={} edges={} graph_ns={} solver_ns={} graph_bytes={} hierarchy_bytes={} plan_bytes={} workspace_each={} workspace_pool={} admitted_peak={}",
            self.setup.threads,
            self.setup.vertices,
            self.setup.edges,
            self.setup.graph_nanoseconds,
            self.setup.solver_nanoseconds,
            self.setup.graph_copy_bytes,
            self.setup.hierarchy_bytes,
            self.setup.plan_bytes,
            self.setup.workspace_bytes_each,
            self.setup.admitted_workspace_pool_bytes,
            self.setup.admitted_peak_bytes,
        );
    }

    fn log_batch(
        &self,
        receipt: FullCmgSpikeBatchReceipt,
        pcg: &[PcgReceipt],
        solution: &[TwoWaySolution],
    ) {
        if !self.diagnostics {
            return;
        }
        let max_iterations = pcg.iter().map(|value| value.iterations).max().unwrap_or(0);
        let total_iterations = pcg
            .iter()
            .map(|value| u64::from(value.iterations))
            .sum::<u64>();
        let total_operator_applications = pcg
            .iter()
            .map(|value| u64::from(value.operator_applications))
            .sum::<u64>();
        let total_preconditioner_applications = pcg
            .iter()
            .map(|value| u64::from(value.preconditioner_applications))
            .sum::<u64>();
        let max_reduced_residual = pcg
            .iter()
            .map(|value| value.relative_residual)
            .fold(0.0_f64, f64::max);
        let complete_residual = solution
            .iter()
            .map(|value| value.residual.relative_norm)
            .fold(0.0_f64, f64::max);
        eprintln!(
            "{SPIKE_SCHEMA} BATCH execution={} rhs={} concurrency={} rhs_ns={} solve_ns={} extraction_ns={} max_iterations={} total_iterations={} total_operator_applications={} total_preconditioner_applications={} max_reduced_residual={} max_complete_residual={complete_residual}",
            execution_name(receipt.execution),
            receipt.rhs_count,
            receipt.concurrency,
            receipt.rhs_nanoseconds,
            receipt.solve_nanoseconds,
            receipt.extraction_nanoseconds,
            max_iterations,
            total_iterations,
            total_operator_applications,
            total_preconditioner_applications,
            max_reduced_residual,
        );
    }
}

fn private_threads() -> Result<usize> {
    let raw = std::env::var(PRIVATE_THREADS_ENV).map_err(|_| {
        BackendError::invalid(
            "cmg_full_spike",
            format!("{PRIVATE_THREADS_ENV} is required for the private spike"),
        )
    })?;
    let threads = raw.parse::<usize>().map_err(|_| {
        BackendError::invalid(
            "cmg_full_spike",
            format!("{PRIVATE_THREADS_ENV} must be a positive integer"),
        )
    })?;
    if threads == 0 {
        return Err(BackendError::invalid(
            "cmg_full_spike",
            format!("{PRIVATE_THREADS_ENV} must be positive"),
        ));
    }
    Ok(threads)
}

fn full_pcg_options(options: PcgOptions) -> Result<FullPcgOptions> {
    options.validate()?;
    Ok(FullPcgOptions {
        // The hybrid-system residual can be amplified when worker effects are
        // reconstructed. Keep the public VCkss tolerance unchanged and solve
        // the private hybrid system more strictly so the independent complete
        // worker-plus-firm gate remains authoritative.
        relative_tolerance: options.tolerance.min(PRIVATE_HYBRID_RELATIVE_TOLERANCE),
        absolute_tolerance: 0.0,
        max_iterations: usize::try_from(options.maximum_iterations).map_err(|_| {
            BackendError::new(
                ErrorCode::ResourceLimit,
                "cmg_full_spike",
                "maximum iteration count is not representable",
            )
        })?,
        residual_recompute_interval: usize::try_from(options.residual_replacement_interval)
            .map_err(|_| {
                BackendError::new(
                    ErrorCode::ResourceLimit,
                    "cmg_full_spike",
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
            "cmg_full_spike",
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
    setup: &FullCmgSpikeSetupReceipt,
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
                    "cmg_full_spike",
                    "compatibility structural-byte count overflow",
                )
            })?,
        workspace_bytes: setup.workspace_bytes_each,
        preconditioner_bytes: 0,
        dense_factor_bytes: 0,
        level,
    })
}

fn batch_receipt(
    report: ParallelPcgBatchReport,
    rhs_nanoseconds: u128,
    solve_nanoseconds: u128,
    extraction_nanoseconds: u128,
) -> FullCmgSpikeBatchReceipt {
    FullCmgSpikeBatchReceipt {
        execution: report.execution(),
        rhs_count: report.rhs_count(),
        concurrency: report.concurrency(),
        rhs_nanoseconds,
        solve_nanoseconds,
        extraction_nanoseconds,
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
                "cmg_full_spike",
                "standalone hierarchy byte forecast overflow",
            )
        })?;
    }
    let terminal_bytes = to_u64(
        solver
            .preconditioner()
            .terminal_factor()
            .map_or(0, cmg_full::GroundedLdl::byte_len),
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
                "cmg_full_spike",
                "standalone hierarchy metadata forecast overflow",
            )
        })?;
    Ok(bytes)
}

fn to_u64(value: usize, context: &'static str) -> Result<u64> {
    u64::try_from(value).map_err(|_| {
        BackendError::new(
            ErrorCode::ResourceLimit,
            "cmg_full_spike",
            format!("{context} is not representable as u64"),
        )
    })
}

fn checked_product_u64(values: &[u64]) -> Result<u64> {
    values.iter().try_fold(1_u64, |product, value| {
        product.checked_mul(*value).ok_or_else(|| {
            BackendError::new(
                ErrorCode::ResourceLimit,
                "cmg_full_spike",
                "private memory product overflow",
            )
        })
    })
}

fn checked_sum_u64(values: &[u64]) -> Result<u64> {
    values.iter().try_fold(0_u64, |sum, value| {
        sum.checked_add(*value).ok_or_else(|| {
            BackendError::new(
                ErrorCode::ResourceLimit,
                "cmg_full_spike",
                "private memory sum overflow",
            )
        })
    })
}

fn map_setup_error(error: CmgError, context: &'static str) -> BackendError {
    let code = match error {
        CmgError::MemoryBudgetExceeded { .. } => ErrorCode::ResourceLimit,
        _ => ErrorCode::CmgSetupFailed,
    };
    BackendError::new(code, "cmg_full_spike", format!("{context}: {error}"))
}

fn map_solve_error(error: CmgError, context: &'static str) -> BackendError {
    let code = match error {
        CmgError::MaximumIterations { .. } => ErrorCode::PcgMaxIterations,
        CmgError::PcgBreakdown { .. } => ErrorCode::PcgCurvatureBreakdown,
        CmgError::MemoryBudgetExceeded { .. } => ErrorCode::ResourceLimit,
        CmgError::ResidualVerificationFailed { .. } => ErrorCode::FullResidualFailed,
        _ => ErrorCode::CmgApplyFailed,
    };
    BackendError::new(code, "cmg_full_spike", format!("{context}: {error}"))
}

const fn execution_name(execution: ParallelPcgExecution) -> &'static str {
    match execution {
        ParallelPcgExecution::Serial => "serial",
        ParallelPcgExecution::Planned => "planned",
        ParallelPcgExecution::AcrossRightHandSides => "across_rhs",
    }
}
