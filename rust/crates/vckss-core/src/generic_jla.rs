// SPDX-License-Identifier: GPL-3.0-only

//! Generic controlled improved-JLA estimator foundation.
//!
//! This module deliberately has no FFI or engine routing.  It is the generic
//! W+F+Q counterpart of the frozen no-control engine and follows the Mata JLA
//! algebra: FE-only leverage probes, an analytic residualized-control Schur
//! correction, and joint or fixed-offset target solves.
//!
//! Counter-V1 fixes semantic entities and scalar arithmetic across stored-row
//! order, order-preserving identifier relabeling, and admitted batch widths.
//! Canonical control-span changes are a numerical certification contract:
//! nonsingular coordinate changes must have the same accept/reject decision
//! and agree within the certified forward-error tolerance, but their rounded
//! coefficients and diagnostic receipts are not promised bitwise identity.

use core::cmp::Ordering;

use crate::batch_plan::{
    plan_batches_with_forecasts, BatchPlanReceipt, BatchPlannerCaps, BatchRequest,
    BATCH_WIDTH_CANDIDATES,
};
use crate::control_basis::{
    canonicalize_controls_in_order_with_interrupt, enforce_downstream_bound,
    refine_control_semantic_order_with_interrupt, MAX_CANONICAL_CONTROLS,
};
use crate::counter_accounting::{
    combine_counter_phases, plan_counter_phase, plan_counter_phase_with_logical_atoms,
    CounterExecutionReceipt, GeneratorEvaluationModel,
};
use crate::dense::{cholesky_factor, frobenius_norm, invert_scaled_spd, symmetric_eigen_extremes};
use crate::error::{BackendError, ErrorCode, Result};
use crate::generic_batch::ModelPcgReceipt;
use crate::interrupt::{
    checkpoint_chunk, stable_sort_by_with_interrupt, InterruptCheck, NeverInterrupt,
};
use crate::jla::{JlaPlan, VarianceComponents};
use crate::model_operator::{
    checked_matrix_length, reserve_exact, zeroed_f64_with_interrupt, CanonicalModelData, ModelRhs,
};
use crate::model_solver::{
    ControlRankReceipt, ModelRoutingOptions, ModelSolve, ModelSolverFallback, ModelSolverOptions,
    ModelSolverRoute, PreparedModelSolver, PreparedModelSolverReceipt,
};
use crate::problem::CompressedProblem;
use crate::rng::{CounterRng, ProbeDomain, MAX_PHYSICAL_WORDS_PER_ATOM};
use crate::types::{DeletionMode, NuisanceMode, MAX_EXACT_BINARY64_INTEGER};
use crate::wall_plan::{wall_work_receipt, WallCalibration, WallWork, WallWorkReceipt};

pub const GENERIC_JLA_EXECUTION_SCHEMA_VERSION: u32 = 2;
pub const GENERIC_JLA_AUTO_FIRM_THRESHOLD_V1: usize = 256;
pub const GENERIC_JLA_AUTO_PLANNED_RHS_THRESHOLD_V1: usize = 8;
pub const GENERIC_JLA_ROUTE_BATCH_WIDTH_CAP_V1: usize = 64;

#[derive(Clone, Copy, Debug)]
pub struct GenericJlaExecutionOptions {
    pub estimator: GenericJlaOptions,
    /// Route, solver, and CMG setup settings. `routing.solver` is
    /// authoritative. For `Auto`, generic JLA uses its receipt-versioned
    /// firm-count/planned-RHS policy rather than `cmg_minimum_dimension`.
    pub routing: ModelRoutingOptions,
    pub leverage_batch: BatchRequest,
    pub target_batch: BatchRequest,
    /// Advisory only. The first registered wall model is deliberately
    /// uncalibrated and cannot affect route, batch width, or probe count.
    pub wallseconds: Option<f64>,
}

impl Default for GenericJlaExecutionOptions {
    fn default() -> Self {
        Self {
            estimator: GenericJlaOptions::default(),
            routing: ModelRoutingOptions::default(),
            leverage_batch: BatchRequest::Auto,
            target_batch: BatchRequest::Auto,
            wallseconds: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GenericJlaMemoryPeakPhase {
    Canonicalization,
    SolverSetup,
    Fit,
    Geometry,
    Leverage,
    Target,
    Maker,
    Result,
}

#[derive(Clone, Debug)]
pub struct GenericJlaMemoryReceipt {
    pub peak_bytes: u64,
    pub peak_phase: GenericJlaMemoryPeakPhase,
    pub setup_peak_bytes: u64,
    pub shared_cmg_persistent_bytes: u64,
    pub full_control_block_persistent_bytes: u64,
    pub setup_transient_bytes: u64,
    pub cmg_preconditioner_workspace_bytes: u64,
    pub cmg_aggregated_cell_capacity_bytes: u64,
    pub cmg_group_index_bytes: u64,
    pub cmg_hybrid_graph_bytes: u64,
    pub retained_nq_bytes: u64,
    pub retained_q2_bytes: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GenericJlaThreadReceipt {
    pub requested: usize,
    pub used: usize,
    pub parallel_regions: usize,
}

#[derive(Clone, Debug)]
pub struct GenericJlaExecutionReceipt {
    pub schema_version: u32,
    pub requested_route: ModelSolverRoute,
    pub selected_route: ModelSolverRoute,
    pub fallback: Option<ModelSolverFallback>,
    pub auto_firm_threshold: usize,
    pub auto_planned_rhs_threshold: usize,
    pub planned_rhs: usize,
    pub auto_route_contract: &'static str,
    pub full_model_setup_complete: bool,
    pub fe_solver_setup_complete: bool,
    pub full_solver_setup: PreparedModelSolverReceipt,
    pub fe_solver_setup: PreparedModelSolverReceipt,
    pub fe_hierarchy_reused: bool,
    pub batch: GenericJlaBatchExecutionReceipt,
    pub wall: WallWorkReceipt,
    pub memory: GenericJlaMemoryReceipt,
    pub counter: CounterExecutionReceipt,
    pub plan_frozen_before_rng: bool,
    pub counter_atoms_before_plan_freeze: u64,
    pub unique_packed_words_before_plan_freeze: u64,
    pub physical_trials_before_plan_freeze: u64,
    pub threads: GenericJlaThreadReceipt,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GenericJlaBatchExecutionReceipt {
    pub plan: BatchPlanReceipt,
    pub leverage_requested: BatchRequest,
    pub leverage_active_width: usize,
    pub target_requested: BatchRequest,
    pub target_active_width: usize,
    pub automatic_ladder_cap: usize,
}

#[derive(Clone, Copy, Debug)]
pub struct GenericJlaOptions {
    pub seed: u64,
    pub probes: u32,
    pub leverage_batch_width: usize,
    pub target_batch_width: usize,
    pub deletion: DeletionMode,
    pub nuisance: NuisanceMode,
    pub rank_tolerance: f64,
    pub block_tolerance: f64,
    pub blocksize_limit: usize,
    pub memory_limit_bytes: u64,
    /// Bytes retained by the caller's prepared problem/context.  They count
    /// against the whole-command limit but are not allocated by this module.
    pub prepared_persistent_bytes: u64,
    /// Bit-packed retained-sample mask bytes that survive after the prepared
    /// problem is dropped and the solved result is retained for export.
    pub retained_mask_bytes: u64,
    /// Simultaneously live native-export and caller-matrix bytes required by
    /// the lossless per-RHS result surface. Direct core callers use zero.
    pub rhs_export_bytes: u64,
    pub solver: ModelSolverOptions,
}

impl Default for GenericJlaOptions {
    fn default() -> Self {
        Self {
            seed: 0,
            probes: 200,
            leverage_batch_width: 8,
            target_batch_width: 8,
            deletion: DeletionMode::Match,
            nuisance: NuisanceMode::Joint,
            rank_tolerance: 1.0e-10,
            block_tolerance: 1.0e-10,
            blocksize_limit: 5_000,
            memory_limit_bytes: u64::MAX,
            prepared_persistent_bytes: 0,
            retained_mask_bytes: 0,
            rhs_export_bytes: 0,
            solver: ModelSolverOptions::default(),
        }
    }
}

impl GenericJlaOptions {
    fn validate(self) -> Result<Self> {
        if self.probes < 2 {
            return Err(invalid("probes must be at least two"));
        }
        if self.leverage_batch_width == 0 || self.target_batch_width == 0 {
            return Err(invalid("JLA batch widths must be positive"));
        }
        if !self.rank_tolerance.is_finite()
            || self.rank_tolerance < 1.0e-14
            || self.rank_tolerance >= 0.1
        {
            return Err(invalid(
                "rank tolerance must be finite and lie in [1e-14, 0.1)",
            ));
        }
        if !self.block_tolerance.is_finite()
            || self.block_tolerance < 1.0e-14
            || self.block_tolerance >= 1.0
        {
            return Err(invalid(
                "block tolerance must be finite and lie in [1e-14, 1)",
            ));
        }
        if self.blocksize_limit == 0 || self.blocksize_limit > 1_000_000 {
            return Err(invalid("block-size limit must lie in [1, 1000000]"));
        }
        if self.memory_limit_bytes == 0 {
            return Err(invalid("whole-command memory limit must be positive"));
        }
        if self.retained_mask_bytes > self.prepared_persistent_bytes {
            return Err(invalid(
                "retained-mask bytes cannot exceed prepared persistent bytes",
            ));
        }
        self.solver.validate()?;
        Ok(self)
    }
}

#[derive(Clone, Debug)]
pub struct GenericJlaReceipt {
    pub execution: GenericJlaExecutionReceipt,
    pub parameters: usize,
    pub full_parameters: usize,
    pub correction_parameters: usize,
    pub deletion_units: u64,
    pub target_strata: usize,
    pub leverage_probes_accepted: u32,
    pub target_probes_accepted: u32,
    pub maximum_leverage: f64,
    pub full_fit_relres: f64,
    pub working_fit_relres: f64,
    pub maximum_solve_relres: f64,
    pub maximum_maker_relres: f64,
    /// Lossless logical-order receipts for every estimator RHS, including the
    /// Q strict control-rank projections reused by control geometry.
    pub rhs: Vec<GenericJlaRhsReceipt>,
    pub full_fit_complete_residual: f64,
    pub working_fit_complete_residual: f64,
    pub maximum_reduced_residual: f64,
    pub maximum_complete_residual: f64,
    pub full_residual_tolerance: f64,
    pub control_rank: ControlRankReceipt,
    pub control_basis_relres: f64,
    pub control_basis_forward_error: f64,
    pub control_schur_rcond: f64,
    pub control_schur_relres: f64,
    pub deletion_rank_gap: f64,
    pub peak_forecast_bytes: u64,
    pub canonicalization_peak_forecast_bytes: u64,
    pub setup_peak_forecast_bytes: u64,
    pub fit_peak_forecast_bytes: u64,
    pub geometry_peak_forecast_bytes: u64,
    pub leverage_peak_forecast_bytes: u64,
    pub target_peak_forecast_bytes: u64,
    pub maker_peak_forecast_bytes: u64,
    pub result_forecast_bytes: u64,
    pub native_result_payload_bytes: u64,
    pub result_transition_peak_forecast_bytes: u64,
    pub result_export_peak_forecast_bytes: u64,
    pub retained_mask_bytes: u64,
    pub rhs_export_bytes: u64,
    pub topology_checksum: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GenericJlaRhsPhase {
    ControlProjection,
    FullJointFit,
    FixedOffsetWorkingFit,
    Leverage,
    Target,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GenericJlaRhsSide {
    Joint,
    Worker,
    Firm,
}

#[derive(Clone, Debug)]
pub struct GenericJlaRhsReceipt {
    pub phase: GenericJlaRhsPhase,
    pub side: GenericJlaRhsSide,
    /// Zero-based logical probe index. Fits use `None`.
    pub probe: Option<u32>,
    pub pcg: ModelPcgReceipt,
    pub complete_residual: f64,
}

#[derive(Clone, Debug)]
pub struct GenericJlaResult {
    pub plugin: VarianceComponents,
    pub correction: VarianceComponents,
    pub corrected: VarianceComponents,
    pub numerical_mcse: VarianceComponents,
    pub weighted_rss: f64,
    pub receipt: GenericJlaReceipt,
}

#[derive(Clone, Copy, Debug, Default)]
struct FiveMoments {
    projection: StableAccumulator,
    residual: StableAccumulator,
    projection_fourth: StableAccumulator,
    residual_fourth: StableAccumulator,
    mixed: StableAccumulator,
}

impl FiveMoments {
    fn from_totals(
        projection: f64,
        residual: f64,
        projection_fourth: f64,
        residual_fourth: f64,
        mixed: f64,
    ) -> Self {
        let mut output = Self::default();
        output.projection.add(projection);
        output.residual.add(residual);
        output.projection_fourth.add(projection_fourth);
        output.residual_fourth.add(residual_fourth);
        output.mixed.add(mixed);
        output
    }

    fn add(&mut self, projection: f64, residual: f64) {
        let p2 = projection * projection;
        let m2 = residual * residual;
        self.projection.add(p2);
        self.residual.add(m2);
        self.projection_fourth.add(p2 * p2);
        self.residual_fourth.add(m2 * m2);
        self.mixed.add(p2 * m2);
    }

    fn finite(self, probes: f64, options: GenericJlaOptions, unit: usize) -> Result<FiniteMoment> {
        let projection_sum = self.projection.finish();
        let residual_sum = self.residual.finish();
        let total = (projection_sum + residual_sum) / probes;
        if !total.is_finite() || total <= options.block_tolerance {
            return Err(BackendError::new(
                ErrorCode::JlaConstraintFailed,
                "generic_jla_leverage",
                format!("projection and residual masses are not positive at unit {unit}"),
            ));
        }
        let projection = (projection_sum / probes) / total;
        let residual = (residual_sum / probes) / total;
        let p4 = self.projection_fourth.finish() / probes;
        let m4 = self.residual_fourth.finish() / probes;
        let mixed = self.mixed.finish() / probes;
        // Improved-JLA uses coefficient one on the mixed fourth moment in
        // finite_bias.  This is intentionally not the historical factor two.
        let bias = (residual * p4 - projection * m4 + (residual - projection) * mixed) / probes;
        let mut variance = (residual * residual * p4 + projection * projection * m4
            - 2.0 * projection * residual * mixed)
            / probes;
        if [projection, residual, bias, variance]
            .iter()
            .any(|value| !value.is_finite())
        {
            return Err(BackendError::new(
                ErrorCode::JlaMomentFailed,
                "generic_jla_leverage",
                format!("finite-projection moment is nonfinite at unit {unit}"),
            ));
        }
        if variance < -100.0 * options.rank_tolerance {
            return Err(BackendError::new(
                ErrorCode::JlaMomentFailed,
                "generic_jla_leverage",
                format!("finite-projection variance is negative at unit {unit}"),
            ));
        }
        variance = variance.max(0.0);
        Ok(FiniteMoment {
            projection,
            residual,
            bias,
            variance,
        })
    }
}

#[derive(Clone, Copy, Debug)]
struct FiniteMoment {
    projection: f64,
    residual: f64,
    bias: f64,
    variance: f64,
}

#[derive(Clone, Debug)]
struct TargetPlan {
    cell: Vec<u32>,
    per_copy_mass: Vec<f64>,
    physical_count: Vec<u64>,
    target_mass: Vec<f64>,
    entity: Vec<u64>,
}

#[derive(Clone, Debug)]
struct MatchPlan {
    rows: Vec<Vec<usize>>,
    cell: Vec<u32>,
    physical_count: Vec<u64>,
    entity: Vec<u64>,
}

#[derive(Clone, Debug)]
struct ObservationClass {
    rows: Vec<usize>,
    entity: u64,
    physical_count: u64,
}

#[derive(Clone, Debug)]
struct ControlGeometry {
    controls: usize,
    /// Control-major N-by-Q residualized controls moved from the strict rank
    /// preparation; no second projection solve is performed.
    residualized: Vec<f64>,
    factor: Vec<f64>,
    leverage: Vec<f64>,
    rcond: f64,
    relres: f64,
}

#[derive(Clone, Copy, Debug)]
struct MemoryForecast {
    peak: u64,
    peak_phase: GenericJlaMemoryPeakPhase,
    canonicalization: u64,
    setup: u64,
    fit: u64,
    geometry: u64,
    leverage: u64,
    target: u64,
    maker: u64,
    result: u64,
    native_result_payload: u64,
    result_transition: u64,
    result_export: u64,
    shared_cmg_persistent: u64,
    full_control_block_persistent: u64,
    setup_transient: u64,
    cmg_preconditioner_workspace: u64,
    cmg_aggregated_cell_capacity: u64,
    cmg_group_index: u64,
    cmg_hybrid_graph: u64,
    retained_nq: u64,
    retained_q2: u64,
}

#[derive(Clone, Copy, Debug, Default)]
struct RouteMemory {
    shared_cmg_persistent: u64,
    full_control_block_persistent: u64,
    setup_transient: u64,
    cmg_preconditioner_workspace: u64,
    cmg_batch_workspace_per_column: u64,
    cmg_aggregated_cell_capacity: u64,
    cmg_group_index: u64,
    cmg_hybrid_graph: u64,
}

#[derive(Clone, Copy, Debug)]
struct MemoryFacts {
    maximum_deletion_block: u64,
}

#[derive(Clone, Copy, Debug, Default)]
struct StableAccumulator {
    sum: f64,
    correction: f64,
}

impl StableAccumulator {
    fn add(&mut self, value: f64) {
        let next = self.sum + value;
        if self.sum.abs() >= value.abs() {
            self.correction += (self.sum - next) + value;
        } else {
            self.correction += (value - next) + self.sum;
        }
        self.sum = next;
    }

    fn finish(self) -> f64 {
        self.sum + self.correction
    }
}

fn stable_add_index(sum: &mut [f64], correction: &mut [f64], index: usize, value: f64) {
    let next = sum[index] + value;
    if sum[index].abs() >= value.abs() {
        correction[index] += (sum[index] - next) + value;
    } else {
        correction[index] += (value - next) + sum[index];
    }
    sum[index] = next;
}

fn finish_stable_vector(
    sum: &mut [f64],
    correction: &[f64],
    interrupt: &mut dyn InterruptCheck,
    phase: &'static str,
) -> Result<()> {
    if sum.len() != correction.len() {
        return Err(BackendError::invariant(
            phase,
            "compensated vector dimensions disagree",
        ));
    }
    for (index, (value, &adjustment)) in sum.iter_mut().zip(correction).enumerate() {
        checkpoint_chunk(interrupt, index, phase)?;
        *value += adjustment;
    }
    Ok(())
}

fn expected_rhs_receipt_count(options: GenericJlaOptions, controls: usize) -> Result<usize> {
    let fits = 1_usize + usize::from(options.nuisance == NuisanceMode::FixedOffset && controls > 0);
    let probes = usize::try_from(options.probes)
        .map_err(|_| resource("probe count is not addressable for RHS receipts"))?;
    let probe_receipts = probes
        .checked_mul(3)
        .ok_or_else(|| resource("generic-JLA RHS receipt count overflow"))?;
    controls
        .checked_add(fits)
        .and_then(|value| value.checked_add(probe_receipts))
        .ok_or_else(|| resource("generic-JLA RHS receipt count overflow"))
}

fn rhs_receipt(
    phase: GenericJlaRhsPhase,
    side: GenericJlaRhsSide,
    probe: Option<u32>,
    solve: &ModelSolve,
) -> GenericJlaRhsReceipt {
    GenericJlaRhsReceipt {
        phase,
        side,
        probe,
        pcg: solve.receipt.pcg.clone(),
        complete_residual: solve.receipt.full_residual,
    }
}

pub fn run_generic_jla(
    problem: &CompressedProblem,
    options: GenericJlaOptions,
) -> Result<GenericJlaResult> {
    run_generic_jla_with_interrupt(problem, options, &mut NeverInterrupt)
}

pub fn run_generic_jla_with_interrupt(
    problem: &CompressedProblem,
    options: GenericJlaOptions,
    interrupt: &mut dyn InterruptCheck,
) -> Result<GenericJlaResult> {
    run_generic_jla_routed_with_interrupt(
        problem,
        GenericJlaExecutionOptions {
            estimator: options,
            routing: ModelRoutingOptions {
                route: ModelSolverRoute::Diagonal,
                allow_automatic_cmg_setup_fallback: false,
                solver: options.solver,
                ..ModelRoutingOptions::default()
            },
            leverage_batch: BatchRequest::Explicit(options.leverage_batch_width),
            target_batch: BatchRequest::Explicit(options.target_batch_width),
            wallseconds: None,
        },
        interrupt,
    )
}

pub fn run_generic_jla_routed(
    problem: &CompressedProblem,
    options: GenericJlaExecutionOptions,
) -> Result<GenericJlaResult> {
    run_generic_jla_routed_with_interrupt(problem, options, &mut NeverInterrupt)
}

#[allow(clippy::too_many_lines)]
pub fn run_generic_jla_routed_with_interrupt(
    problem: &CompressedProblem,
    execution_options: GenericJlaExecutionOptions,
    interrupt: &mut dyn InterruptCheck,
) -> Result<GenericJlaResult> {
    interrupt.checkpoint("generic_jla_entry")?;
    let routing = execution_options.routing.validate()?;
    let mut options = execution_options.estimator;
    options.solver = routing.solver;
    let mut options = options.validate()?;
    validate_problem(problem)?;
    let rows = problem.outcome.len();
    let workers = problem.workers();
    let firms = problem.firms();
    let controls = problem.controls.len();
    let fe_parameters = workers
        .checked_add(firms - 1)
        .ok_or_else(|| resource("FE parameter count overflow"))?;
    let full_parameters = fe_parameters
        .checked_add(controls)
        .ok_or_else(|| resource("full parameter count overflow"))?;
    let correction_parameters = if options.nuisance == NuisanceMode::Joint {
        full_parameters
    } else {
        fe_parameters
    };
    let planned_rhs = usize::try_from(options.probes)
        .map_err(|_| resource("probe count is not addressable for route planning"))?
        .checked_mul(3)
        .and_then(|value| value.checked_add(controls))
        .and_then(|value| value.checked_add(1))
        .and_then(|value| {
            value.checked_add(usize::from(
                options.nuisance == NuisanceMode::FixedOffset && controls > 0,
            ))
        })
        .ok_or_else(|| resource("generic-JLA planned RHS count overflow"))?;
    let automatic_route = if firms < GENERIC_JLA_AUTO_FIRM_THRESHOLD_V1
        || planned_rhs < GENERIC_JLA_AUTO_PLANNED_RHS_THRESHOLD_V1
    {
        ModelSolverRoute::Diagonal
    } else {
        ModelSolverRoute::Cmg
    };
    // The shared model router still records Auto as requested. Its numerical
    // dimension threshold is specialized here to enact the registered
    // generic-JLA structural F/planned-RHS policy.
    let routed_prepare = if routing.route == ModelSolverRoute::Auto {
        ModelRoutingOptions {
            cmg_minimum_dimension: if automatic_route == ModelSolverRoute::Cmg {
                1
            } else {
                usize::MAX
            },
            ..routing
        }
    } else {
        routing
    };
    let rhs_receipt_capacity = expected_rhs_receipt_count(options, controls)?;
    let mut rhs_receipts = Vec::new();
    reserve_exact(
        &mut rhs_receipts,
        rhs_receipt_capacity,
        "generic JLA RHS receipts",
    )?;

    let control_order = control_semantic_order(problem, options.rank_tolerance, interrupt)?;
    let canonical = canonicalize_controls_in_order_with_interrupt(
        &problem.controls,
        &problem.frequency,
        &control_order,
        options.rank_tolerance,
        interrupt,
    )?;
    drop(control_order);
    let control_basis_relres = canonical.receipt.relres;
    let control_basis_forward_error = canonical.receipt.forward_error;
    let weights = exact_weights(problem, interrupt)?;
    let row_order = canonical_row_order(problem, &canonical.columns, interrupt)?;
    let full_data = CanonicalModelData {
        workers,
        firms,
        row_worker: &problem.row_worker,
        row_firm: &problem.row_firm,
        weight: &weights,
        controls: &canonical.columns,
    };
    let prepared_solvers = PreparedModelSolver::prepare_generic_jla_routed_with_interrupt(
        full_data,
        routed_prepare,
        interrupt,
    )?;
    let fe_hierarchy_reused = prepared_solvers.fe_hierarchy_reused;
    let mut full_solver = prepared_solvers.full;
    let fe_solver = prepared_solvers.fe;
    let control_rank = full_solver.control_rank_receipt().clone();
    for projection in &control_rank.projection_rhs {
        rhs_receipts.push(GenericJlaRhsReceipt {
            phase: GenericJlaRhsPhase::ControlProjection,
            side: GenericJlaRhsSide::Joint,
            probe: Some(
                u32::try_from(projection.logical_control).map_err(|_| {
                    resource("control projection index is not representable as u32")
                })?,
            ),
            pcg: projection.pcg.clone(),
            complete_residual: projection.complete_residual,
        });
    }
    if controls > 0 {
        enforce_downstream_bound(
            canonical.receipt.forward_error,
            full_solver.control_rank_receipt().rcond,
            "joint-control conditioning cannot certify canonical-basis invariance",
        )?;
    }
    let route_memory = route_memory_forecast(problem, &full_solver, controls)?;
    let memory_facts = memory_facts(problem, interrupt)?;
    let batch = plan_generic_jla_batches(
        problem,
        options,
        full_parameters,
        execution_options.leverage_batch,
        execution_options.target_batch,
        route_memory,
        memory_facts,
        interrupt,
    )?;
    options.leverage_batch_width = batch.leverage_active_width;
    options.target_batch_width = batch.target_active_width;
    let memory = memory_forecast(
        problem,
        options,
        full_parameters,
        route_memory,
        memory_facts,
        options.leverage_batch_width,
        options.target_batch_width,
    )?;
    admit_memory(memory.peak, options.memory_limit_bytes)?;
    if memory.peak != batch.plan.selected_command_peak_bytes {
        return Err(BackendError::invariant(
            "generic_jla_plan",
            "selected batch plan and final memory forecast disagree",
        ));
    }
    let wall = wall_work_receipt(
        generic_jla_wall_work(problem, options, full_parameters)?,
        execution_options.wallseconds,
        WallCalibration::Uncalibrated,
    )?;
    let mut execution = GenericJlaExecutionReceipt {
        schema_version: GENERIC_JLA_EXECUTION_SCHEMA_VERSION,
        requested_route: full_solver.receipt().requested,
        selected_route: full_solver.receipt().selected,
        fallback: full_solver.receipt().fallback.clone(),
        auto_firm_threshold: GENERIC_JLA_AUTO_FIRM_THRESHOLD_V1,
        auto_planned_rhs_threshold: GENERIC_JLA_AUTO_PLANNED_RHS_THRESHOLD_V1,
        planned_rhs,
        auto_route_contract: "VCKSS-GENERIC-JLA-AUTO-F256-RHS8-V1",
        full_model_setup_complete: true,
        fe_solver_setup_complete: true,
        full_solver_setup: full_solver.receipt().clone(),
        fe_solver_setup: fe_solver.receipt().clone(),
        fe_hierarchy_reused,
        batch,
        wall,
        memory: GenericJlaMemoryReceipt {
            peak_bytes: memory.peak,
            peak_phase: memory.peak_phase,
            setup_peak_bytes: memory.setup,
            shared_cmg_persistent_bytes: memory.shared_cmg_persistent,
            full_control_block_persistent_bytes: memory.full_control_block_persistent,
            setup_transient_bytes: memory.setup_transient,
            cmg_preconditioner_workspace_bytes: memory.cmg_preconditioner_workspace,
            cmg_aggregated_cell_capacity_bytes: memory.cmg_aggregated_cell_capacity,
            cmg_group_index_bytes: memory.cmg_group_index,
            cmg_hybrid_graph_bytes: memory.cmg_hybrid_graph,
            retained_nq_bytes: memory.retained_nq,
            retained_q2_bytes: memory.retained_q2,
        },
        counter: CounterExecutionReceipt::default(),
        plan_frozen_before_rng: true,
        counter_atoms_before_plan_freeze: 0,
        unique_packed_words_before_plan_freeze: 0,
        physical_trials_before_plan_freeze: 0,
        threads: GenericJlaThreadReceipt {
            requested: 1,
            used: 1,
            parallel_regions: 0,
        },
    };
    let full_rhs = transpose_outcome_rhs(
        problem,
        &canonical.columns,
        &weights,
        &problem.outcome,
        &row_order,
        interrupt,
    )?;
    let mut full_fit = Some(full_solver.solve_with_interrupt(
        ModelRhs {
            worker: &full_rhs.0,
            firm: &full_rhs.1,
            control: &full_rhs.2,
        },
        interrupt,
    )?);
    drop(full_rhs);
    let full_fit_relres = full_fit
        .as_ref()
        .expect("full fit is present before nuisance specialization")
        .residual
        .relative_norm;
    let full_fit_complete_residual = full_fit
        .as_ref()
        .expect("full fit is present before nuisance specialization")
        .receipt
        .full_residual;
    let full_fit_rhs_receipt = rhs_receipt(
        GenericJlaRhsPhase::FullJointFit,
        GenericJlaRhsSide::Joint,
        None,
        full_fit
            .as_ref()
            .expect("full fit is present before nuisance specialization"),
    );

    let residualized_controls = full_solver.take_generic_jla_residualized_controls()?;
    let geometry = control_geometry(
        problem,
        &canonical.columns,
        &weights,
        &row_order,
        residualized_controls,
        options,
        interrupt,
    )?;
    let control_schur_rcond = geometry.rcond;
    let control_schur_relres = geometry.relres;
    let maximum_projection_relres = control_rank.maximum_projection_residual;
    if controls > 0 {
        enforce_downstream_bound(
            canonical.receipt.forward_error,
            geometry.rcond,
            "residualized-control conditioning cannot certify canonical-basis invariance",
        )?;
    }
    let deletion_rank_gap = if controls == 0 {
        1.0
    } else {
        deletion_rank_certificate(
            problem,
            &canonical.columns,
            &weights,
            &row_order,
            options,
            interrupt,
        )?
    };
    if controls > 0 {
        enforce_downstream_bound(
            canonical.receipt.forward_error,
            deletion_rank_gap,
            "deletion-rank conditioning cannot certify canonical-basis invariance",
        )?;
    }

    let mut working_y = copy_f64(&problem.outcome, "working outcome", interrupt)?;
    let (working_solver, working_fit, working_fit_is_distinct): (&PreparedModelSolver<'_>, _, _) =
        if options.nuisance == NuisanceMode::FixedOffset && controls > 0 {
            for row in 0..rows {
                checkpoint_chunk(interrupt, row, "generic_jla_fixedoffset")?;
                let mut value = StableAccumulator::default();
                value.add(working_y[row]);
                for control in 0..controls {
                    value.add(
                        -canonical.columns[control][row]
                            * full_fit
                                .as_ref()
                                .expect("fixed-offset full fit is present")
                                .coefficients
                                .control[control],
                    );
                }
                working_y[row] = value.finish();
            }
            let rhs =
                transpose_outcome_rhs(problem, &[], &weights, &working_y, &row_order, interrupt)?;
            let fit = fe_solver.solve_with_interrupt(
                ModelRhs {
                    worker: &rhs.0,
                    firm: &rhs.1,
                    control: &[],
                },
                interrupt,
            )?;
            drop(full_fit.take());
            (&fe_solver, fit, true)
        } else {
            (
                &full_solver,
                full_fit
                    .take()
                    .expect("joint nuisance consumes the full fit"),
                false,
            )
        };
    let working_fit_relres = working_fit.residual.relative_norm;
    let working_fit_complete_residual = working_fit.receipt.full_residual;
    rhs_receipts.push(full_fit_rhs_receipt);
    if working_fit_is_distinct {
        rhs_receipts.push(rhs_receipt(
            GenericJlaRhsPhase::FixedOffsetWorkingFit,
            GenericJlaRhsSide::Joint,
            None,
            &working_fit,
        ));
    }
    let mut fitted = zeroed_f64_with_interrupt(
        rows,
        "generic JLA fitted values",
        interrupt,
        "generic_jla_fit_allocate",
    )?;
    working_solver.operator().predict_into_with_interrupt(
        &working_fit.coefficients.worker,
        &working_fit.coefficients.firm,
        &working_fit.coefficients.control,
        &mut fitted,
        interrupt,
    )?;
    let mut residual = zeroed_f64_with_interrupt(
        rows,
        "generic JLA residuals",
        interrupt,
        "generic_jla_fit_allocate",
    )?;
    let mut rss = StableAccumulator::default();
    for (position, &row) in row_order.iter().enumerate() {
        checkpoint_chunk(interrupt, position, "generic_jla_residual")?;
        residual[row] = working_y[row] - fitted[row];
        rss.add(weights[row] * residual[row] * residual[row]);
    }
    let weighted_rss = rss.finish();
    drop(fitted);
    if !weighted_rss.is_finite() {
        return Err(nonfinite("weighted residual sum of squares is nonfinite"));
    }
    let plugin = plugin_components_canonical(
        problem,
        &working_fit.coefficients.worker,
        &working_fit.coefficients.firm,
        interrupt,
    )?;
    drop(working_fit);

    let row_rank = match options.deletion {
        DeletionMode::Match => semantic_row_ranks(problem, &canonical.columns, interrupt)?,
        DeletionMode::Observation => observation_row_ranks(problem, &canonical.columns, interrupt)?,
    };
    let target_plan = target_plan(problem, &row_rank, options.deletion, interrupt)?;
    let target_counter = plan_counter_phase(
        options.probes,
        &target_plan.physical_count,
        GeneratorEvaluationModel::PackedWords,
    )?;
    let mut prepared_match_plan = None;
    let mut prepared_observation_classes = None;
    let leverage_counter = match options.deletion {
        DeletionMode::Match => {
            let plan = match_plan(problem, &row_rank, options, interrupt)?;
            let counter = plan_counter_phase(
                options.probes,
                &plan.physical_count,
                GeneratorEvaluationModel::PackedWords,
            )?;
            prepared_match_plan = Some(plan);
            counter
        }
        DeletionMode::Observation => {
            let classes = observation_classes(problem, &row_rank, interrupt)?;
            let mut physical_count = Vec::new();
            reserve_exact(
                &mut physical_count,
                classes.len(),
                "observation Counter accounting counts",
            )?;
            for class in &classes {
                physical_count.push(class.physical_count);
            }
            let counter = plan_counter_phase_with_logical_atoms(
                options.probes,
                problem.physical_total,
                &physical_count,
                GeneratorEvaluationModel::PhysicalTrials { passes: 2 },
            )?;
            prepared_observation_classes = Some(classes);
            counter
        }
    };
    execution.counter = combine_counter_phases(leverage_counter, target_counter)?;
    let rng = CounterRng::new(options.seed);
    // Match row order remains live through the target contractions. Observation
    // contractions use the global canonical row order directly.
    let mut match_rows_for_target = None;
    let (
        deleted_adjusted,
        maximum_leverage,
        maximum_maker_relres,
        leverage_solve_relres,
        deletion_units,
    ) = match options.deletion {
        DeletionMode::Match => {
            let plan = prepared_match_plan
                .take()
                .expect("match plan was frozen before Counter-V1 addressing");
            let group_count = plan.rows.len();
            let (moments, solve_relres) = match_leverage_moments(
                problem,
                &plan,
                &fe_solver,
                rng,
                options,
                &mut rhs_receipts,
                interrupt,
            )?;
            let active_geometry = if options.nuisance == NuisanceMode::Joint {
                Some(&geometry)
            } else {
                None
            };
            let adjusted = match_deleted_adjustment(
                problem,
                &plan,
                &moments,
                &residual,
                active_geometry,
                options,
                interrupt,
            )?;
            match_rows_for_target = Some(plan.rows);
            (
                adjusted.values,
                adjusted.maximum_leverage,
                adjusted.maximum_relres,
                solve_relres,
                u64::try_from(group_count).map_err(|_| resource("deletion-unit count overflow"))?,
            )
        }
        DeletionMode::Observation => {
            let classes = prepared_observation_classes
                .take()
                .expect("observation classes were frozen before Counter-V1 addressing");
            let (moments, signs, solve_relres) = observation_leverage_moments(
                problem,
                &classes,
                &fe_solver,
                rng,
                options,
                &mut rhs_receipts,
                interrupt,
            )?;
            let zero_control_leverage;
            let active_leverage = if options.nuisance == NuisanceMode::Joint {
                &geometry.leverage
            } else {
                zero_control_leverage = zeroed_f64_with_interrupt(
                    rows,
                    "fixed-offset zero control leverage",
                    interrupt,
                    "generic_jla_observation_allocate",
                )?;
                &zero_control_leverage
            };
            let adjusted = observation_deleted_adjustment(
                problem,
                &classes,
                &moments,
                &signs,
                &residual,
                active_leverage,
                &row_order,
                options,
                interrupt,
            )?;
            (
                adjusted.values,
                adjusted.maximum_leverage,
                0.0,
                solve_relres,
                problem.physical_total,
            )
        }
    };
    drop(geometry);
    drop(residual);
    drop(row_rank);

    let target = target_correction(
        problem,
        &target_plan,
        &working_y,
        &deleted_adjusted,
        working_solver,
        &row_order,
        match_rows_for_target.as_deref(),
        rng,
        options,
        &mut rhs_receipts,
        interrupt,
    )?;
    let target_strata = target_plan.cell.len();
    drop(target_plan);
    drop(match_rows_for_target);
    drop(deleted_adjusted);
    drop(working_y);
    let correction = target.mean;
    let corrected = subtract_components(plugin, correction)?;
    let numerical_mcse = target.mcse;
    plugin.verify_accounting(1.0e-11)?;
    correction.verify_accounting(1.0e-11)?;
    corrected.verify_accounting(1.0e-11)?;

    let maximum_solve_relres = full_fit_relres
        .max(working_fit_relres)
        .max(maximum_projection_relres)
        .max(leverage_solve_relres)
        .max(target.maximum_solve_relres);
    if rhs_receipts.len() != rhs_receipt_capacity {
        return Err(BackendError::invariant(
            "generic_jla_receipts",
            "generic-JLA RHS receipt count does not match its preflighted capacity",
        ));
    }
    let maximum_reduced_residual = rhs_receipts
        .iter()
        .map(|value| value.pcg.relative_residual)
        .fold(0.0_f64, f64::max);
    let maximum_complete_residual =
        maximum_solve_relres.max(control_rank.maximum_projection_residual);
    drop(fe_solver);
    drop(full_solver);
    drop(row_order);
    drop(weights);
    drop(canonical);
    interrupt.checkpoint("generic_jla_final")?;
    execution.counter = combine_counter_phases(
        execution.counter.leverage.completed(),
        execution.counter.target.completed(),
    )?;
    Ok(GenericJlaResult {
        plugin,
        correction,
        corrected,
        numerical_mcse,
        weighted_rss,
        receipt: GenericJlaReceipt {
            execution,
            parameters: correction_parameters,
            full_parameters,
            correction_parameters,
            deletion_units,
            target_strata,
            leverage_probes_accepted: options.probes,
            target_probes_accepted: options.probes,
            maximum_leverage,
            full_fit_relres,
            working_fit_relres,
            maximum_solve_relres,
            maximum_maker_relres,
            rhs: rhs_receipts,
            full_fit_complete_residual,
            working_fit_complete_residual,
            maximum_reduced_residual,
            maximum_complete_residual,
            full_residual_tolerance: options.solver.full_residual_tolerance(),
            control_rank,
            control_basis_relres,
            control_basis_forward_error,
            control_schur_rcond,
            control_schur_relres,
            deletion_rank_gap,
            peak_forecast_bytes: memory.peak,
            canonicalization_peak_forecast_bytes: memory.canonicalization,
            setup_peak_forecast_bytes: memory.setup,
            fit_peak_forecast_bytes: memory.fit,
            geometry_peak_forecast_bytes: memory.geometry,
            leverage_peak_forecast_bytes: memory.leverage,
            target_peak_forecast_bytes: memory.target,
            maker_peak_forecast_bytes: memory.maker,
            result_forecast_bytes: memory.result,
            native_result_payload_bytes: memory.native_result_payload,
            result_transition_peak_forecast_bytes: memory.result_transition,
            result_export_peak_forecast_bytes: memory.result_export,
            retained_mask_bytes: options.retained_mask_bytes,
            rhs_export_bytes: options.rhs_export_bytes,
            topology_checksum: problem.topology_checksum,
        },
    })
}

fn validate_problem(problem: &CompressedProblem) -> Result<()> {
    let rows = problem.outcome.len();
    if rows == 0 || problem.workers() == 0 || problem.firms() < 2 {
        return Err(invalid(
            "the retained generic-JLA problem is empty or unidentified",
        ));
    }
    if problem.controls.len() > MAX_CANONICAL_CONTROLS {
        return Err(BackendError::new(
            ErrorCode::ResourceLimit,
            "generic_jla_validate",
            format!("at most {MAX_CANONICAL_CONTROLS} controls are supported"),
        ));
    }
    if problem.row_worker.len() != rows
        || problem.row_firm.len() != rows
        || problem.row_cell.len() != rows
        || problem.row_deletion.len() != rows
        || problem.frequency.len() != rows
        || problem.target_weight.len() != rows
        || problem.controls.iter().any(|column| column.len() != rows)
    {
        return Err(BackendError::invariant(
            "generic_jla_validate",
            "compressed row arrays have inconsistent dimensions",
        ));
    }
    if problem.physical_total == 0 || problem.physical_total > MAX_EXACT_BINARY64_INTEGER {
        return Err(BackendError::new(
            ErrorCode::ResourceLimit,
            "generic_jla_validate",
            "physical frequency total is outside the exact binary64 range",
        ));
    }
    if !problem.target_total.is_finite() || problem.target_total <= 0.0 {
        return Err(BackendError::new(
            ErrorCode::InvalidTargetWeight,
            "generic_jla_validate",
            "target mass must be positive and finite",
        ));
    }
    Ok(())
}

fn exact_weights(
    problem: &CompressedProblem,
    interrupt: &mut dyn InterruptCheck,
) -> Result<Vec<f64>> {
    let mut output = Vec::new();
    reserve_exact(
        &mut output,
        problem.frequency.len(),
        "generic JLA exact weights",
    )?;
    for (row, &frequency) in problem.frequency.iter().enumerate() {
        checkpoint_chunk(interrupt, row, "generic_jla_weights")?;
        if frequency == 0 || frequency > MAX_EXACT_BINARY64_INTEGER {
            return Err(BackendError::new(
                ErrorCode::InvalidWeight,
                "generic_jla_weights",
                "frequency must be a positive exactly representable integer",
            ));
        }
        let value = frequency as f64;
        if value as u64 != frequency {
            return Err(resource(
                "frequency is not exactly representable in binary64",
            ));
        }
        output.push(value);
    }
    Ok(output)
}

fn transpose_outcome_rhs(
    problem: &CompressedProblem,
    controls: &[Vec<f64>],
    weights: &[f64],
    outcome: &[f64],
    row_order: &[usize],
    interrupt: &mut dyn InterruptCheck,
) -> Result<(Vec<f64>, Vec<f64>, Vec<f64>)> {
    if weights.len() != outcome.len()
        || outcome.len() != problem.outcome.len()
        || row_order.len() != outcome.len()
    {
        return Err(invalid("outcome-transpose dimensions disagree"));
    }
    let mut worker_sum = repeated(
        problem.workers(),
        StableAccumulator::default(),
        "outcome worker accumulators",
        interrupt,
        "generic_jla_transpose_allocate",
    )?;
    let mut firm_sum = repeated(
        problem.firms(),
        StableAccumulator::default(),
        "outcome firm accumulators",
        interrupt,
        "generic_jla_transpose_allocate",
    )?;
    let mut control_sum = repeated(
        controls.len(),
        StableAccumulator::default(),
        "outcome control accumulators",
        interrupt,
        "generic_jla_transpose_allocate",
    )?;
    for (position, &row) in row_order.iter().enumerate() {
        checkpoint_chunk(interrupt, position, "generic_jla_transpose_outcome")?;
        let w = weights[row] * outcome[row];
        let worker_index = problem.row_worker[row] as usize;
        let firm_index = problem.row_firm[row] as usize;
        worker_sum[worker_index].add(w);
        firm_sum[firm_index].add(w);
        for (control_index, column) in controls.iter().enumerate() {
            checkpoint_chunk(
                interrupt,
                position
                    .checked_mul(controls.len().max(1))
                    .and_then(|value| value.checked_add(control_index))
                    .ok_or_else(|| resource("outcome-transpose work overflow"))?,
                "generic_jla_transpose_outcome",
            )?;
            control_sum[control_index].add(column[row] * w);
        }
    }
    let worker = finish_accumulators(worker_sum, interrupt, "generic_jla_transpose_finish")?;
    let firm = finish_accumulators(firm_sum, interrupt, "generic_jla_transpose_finish")?;
    let control = finish_accumulators(control_sum, interrupt, "generic_jla_transpose_finish")?;
    if worker
        .iter()
        .chain(&firm)
        .chain(&control)
        .any(|value| !value.is_finite())
    {
        return Err(nonfinite("model outcome right-hand side is nonfinite"));
    }
    Ok((worker, firm, control))
}

fn plugin_components_canonical(
    problem: &CompressedProblem,
    worker: &[f64],
    firm: &[f64],
    interrupt: &mut dyn InterruptCheck,
) -> Result<VarianceComponents> {
    if worker.len() != problem.workers() || firm.len() != problem.firms() {
        return Err(invalid("plugin coefficient dimensions disagree"));
    }
    let mut worker_mean = StableAccumulator::default();
    let mut firm_mean = StableAccumulator::default();
    for cell in 0..problem.cells() {
        checkpoint_chunk(interrupt, cell, "generic_jla_plugin_mean")?;
        let target = problem.cell_target_sum[cell];
        worker_mean.add(target * worker[problem.cell_worker[cell] as usize]);
        firm_mean.add(target * firm[problem.cell_firm[cell] as usize]);
    }
    let worker_mean = worker_mean.finish() / problem.target_total;
    let firm_mean = firm_mean.finish() / problem.target_total;
    let mut worker_second = StableAccumulator::default();
    let mut firm_second = StableAccumulator::default();
    let mut covariance = StableAccumulator::default();
    let mut total_second = StableAccumulator::default();
    for cell in 0..problem.cells() {
        checkpoint_chunk(interrupt, cell, "generic_jla_plugin_second")?;
        let target = problem.cell_target_sum[cell];
        let worker_value = worker[problem.cell_worker[cell] as usize] - worker_mean;
        let firm_value = firm[problem.cell_firm[cell] as usize] - firm_mean;
        worker_second.add(target * worker_value * worker_value);
        firm_second.add(target * firm_value * firm_value);
        covariance.add(target * worker_value * firm_value);
        total_second.add(target * (worker_value + firm_value).powi(2));
    }
    let output = VarianceComponents {
        worker: worker_second.finish() / problem.target_total,
        firm: firm_second.finish() / problem.target_total,
        covariance: covariance.finish() / problem.target_total,
        total: total_second.finish() / problem.target_total,
    };
    output.verify_accounting(1.0e-11)?;
    Ok(output)
}

fn control_semantic_order(
    problem: &CompressedProblem,
    rank_tolerance: f64,
    interrupt: &mut dyn InterruptCheck,
) -> Result<Vec<usize>> {
    let mut order = index_vector(
        problem.outcome.len(),
        interrupt,
        "generic_jla_control_order",
    )?;
    stable_sort_by_with_interrupt(
        &mut order,
        |&left, &right| compare_control_coarse_rows(problem, left, right),
        interrupt,
        "generic_jla_control_order",
    )?;
    refine_control_semantic_order_with_interrupt(
        &problem.controls,
        &problem.frequency,
        &order,
        rank_tolerance,
        |left, right| compare_control_coarse_rows(problem, left, right) == Ordering::Equal,
        interrupt,
    )
}

fn compare_control_coarse_rows(problem: &CompressedProblem, left: usize, right: usize) -> Ordering {
    problem.row_worker[left]
        .cmp(&problem.row_worker[right])
        .then_with(|| problem.row_firm[left].cmp(&problem.row_firm[right]))
        .then_with(|| problem.row_deletion[left].cmp(&problem.row_deletion[right]))
        .then_with(|| problem.row_target[left].cmp(&problem.row_target[right]))
        .then_with(|| problem.frequency[left].cmp(&problem.frequency[right]))
        .then_with(|| problem.target_weight[left].total_cmp(&problem.target_weight[right]))
        .then_with(|| problem.outcome[left].total_cmp(&problem.outcome[right]))
}

fn canonical_row_order(
    problem: &CompressedProblem,
    controls: &[Vec<f64>],
    interrupt: &mut dyn InterruptCheck,
) -> Result<Vec<usize>> {
    let mut order = index_vector(
        problem.outcome.len(),
        interrupt,
        "generic_jla_canonical_row_order",
    )?;
    stable_sort_by_with_interrupt(
        &mut order,
        |&left, &right| {
            compare_semantic_rows(problem, controls, left, right).then_with(|| left.cmp(&right))
        },
        interrupt,
        "generic_jla_canonical_row_order",
    )?;
    Ok(order)
}

fn semantic_row_ranks(
    problem: &CompressedProblem,
    controls: &[Vec<f64>],
    interrupt: &mut dyn InterruptCheck,
) -> Result<Vec<u64>> {
    if controls.is_empty() {
        return Ok(
            JlaPlan::build_no_controls_with_interrupt(problem, interrupt)?.row_semantic_rank,
        );
    }
    let rows = problem.outcome.len();
    let mut order = index_vector(rows, interrupt, "generic_jla_semantic_sort")?;
    stable_sort_by_with_interrupt(
        &mut order,
        |&left, &right| compare_semantic_rows(problem, controls, left, right),
        interrupt,
        "generic_jla_semantic_sort",
    )?;
    let mut rank = repeated(
        rows,
        0_u64,
        "semantic row ranks",
        interrupt,
        "generic_jla_semantic_rank_allocate",
    )?;
    let mut current = 0_u64;
    let mut previous: Option<usize> = None;
    for (position, row) in order.into_iter().enumerate() {
        checkpoint_chunk(interrupt, position, "generic_jla_semantic_rank")?;
        if previous.map_or(true, |prior| {
            compare_semantic_rows(problem, controls, prior, row) != Ordering::Equal
        }) {
            current = current
                .checked_add(1)
                .ok_or_else(|| resource("semantic entity rank overflow"))?;
        }
        rank[row] = current;
        previous = Some(row);
    }
    Ok(rank)
}

fn compare_semantic_rows(
    problem: &CompressedProblem,
    controls: &[Vec<f64>],
    left: usize,
    right: usize,
) -> Ordering {
    let mut ordering = problem.row_worker[left]
        .cmp(&problem.row_worker[right])
        .then_with(|| problem.row_firm[left].cmp(&problem.row_firm[right]))
        .then_with(|| problem.row_deletion[left].cmp(&problem.row_deletion[right]))
        .then_with(|| per_copy_mass(problem, left).total_cmp(&per_copy_mass(problem, right)))
        .then_with(|| problem.outcome[left].total_cmp(&problem.outcome[right]));
    for column in controls {
        ordering = ordering
            .then_with(|| canonical_zero(column[left]).total_cmp(&canonical_zero(column[right])));
    }
    if let Some(probe_order) = &problem.probe_order {
        ordering = ordering.then_with(|| {
            canonical_zero(probe_order[left]).total_cmp(&canonical_zero(probe_order[right]))
        });
    }
    ordering
}

fn target_plan(
    problem: &CompressedProblem,
    row_rank: &[u64],
    deletion: DeletionMode,
    interrupt: &mut dyn InterruptCheck,
) -> Result<TargetPlan> {
    if problem.controls.is_empty() && deletion == DeletionMode::Match {
        let plan = JlaPlan::build_no_controls_with_interrupt(problem, interrupt)?;
        plan.validate_against_problem(problem)?;
        for (stratum, &trials) in plan.target.physical_count.iter().enumerate() {
            checkpoint_chunk(interrupt, stratum, "generic_jla_target_preflight")?;
            preflight_trials(trials, "target stratum")?;
        }
        return Ok(TargetPlan {
            cell: plan.target.cell,
            per_copy_mass: plan.target.per_copy_mass,
            physical_count: plan.target.physical_count,
            target_mass: plan.target.target_mass,
            entity: plan.target.semantic_rank,
        });
    }
    let mut order = index_vector(problem.outcome.len(), interrupt, "generic_jla_target_plan")?;
    stable_sort_by_with_interrupt(
        &mut order,
        |&left, &right| {
            problem.row_cell[left]
                .cmp(&problem.row_cell[right])
                .then_with(|| {
                    canonical_zero(per_copy_mass(problem, left))
                        .total_cmp(&canonical_zero(per_copy_mass(problem, right)))
                })
                .then_with(|| row_rank[left].cmp(&row_rank[right]))
        },
        interrupt,
        "generic_jla_target_plan",
    )?;
    let mut output = TargetPlan {
        cell: Vec::new(),
        per_copy_mass: Vec::new(),
        physical_count: Vec::new(),
        target_mass: Vec::new(),
        entity: Vec::new(),
    };
    reserve_exact(&mut output.cell, order.len(), "target-plan cells")?;
    reserve_exact(
        &mut output.per_copy_mass,
        order.len(),
        "target-plan per-copy masses",
    )?;
    reserve_exact(
        &mut output.physical_count,
        order.len(),
        "target-plan physical counts",
    )?;
    reserve_exact(
        &mut output.target_mass,
        order.len(),
        "target-plan target masses",
    )?;
    reserve_exact(&mut output.entity, order.len(), "target-plan entities")?;
    let mut cursor = 0;
    while cursor < order.len() {
        checkpoint_chunk(interrupt, cursor, "generic_jla_target_groups")?;
        let first = order[cursor];
        let cell = problem.row_cell[first];
        let per_copy = canonical_zero(per_copy_mass(problem, first));
        let bits = per_copy.to_bits();
        let mut physical = 0_u64;
        let mut target = StableAccumulator::default();
        let mut entity = u64::MAX;
        let begin = cursor;
        while cursor < order.len()
            && problem.row_cell[order[cursor]] == cell
            && canonical_zero(per_copy_mass(problem, order[cursor])).to_bits() == bits
        {
            checkpoint_chunk(interrupt, cursor - begin, "generic_jla_target_group_rows")?;
            let row = order[cursor];
            physical = physical
                .checked_add(problem.frequency[row])
                .ok_or_else(|| resource("target-stratum physical-count overflow"))?;
            target.add(problem.target_weight[row]);
            entity = entity.min(row_rank[row]);
            cursor += 1;
        }
        preflight_trials(physical, "target stratum")?;
        output.cell.push(cell);
        output.per_copy_mass.push(per_copy);
        output.physical_count.push(physical);
        output.target_mass.push(target.finish());
        output.entity.push(entity);
    }
    validate_target_plan(problem, &output, interrupt)?;
    Ok(output)
}

fn validate_target_plan(
    problem: &CompressedProblem,
    plan: &TargetPlan,
    interrupt: &mut dyn InterruptCheck,
) -> Result<()> {
    let mut cell_frequency = repeated(
        problem.cells(),
        0_u64,
        "target-plan cell frequencies",
        interrupt,
        "generic_jla_target_validate_allocate",
    )?;
    let mut cell_target = repeated(
        problem.cells(),
        StableAccumulator::default(),
        "target-plan cell targets",
        interrupt,
        "generic_jla_target_validate_allocate",
    )?;
    for stratum in 0..plan.cell.len() {
        checkpoint_chunk(interrupt, stratum, "generic_jla_target_validate")?;
        let cell = plan.cell[stratum] as usize;
        cell_frequency[cell] = cell_frequency[cell]
            .checked_add(plan.physical_count[stratum])
            .ok_or_else(|| resource("target-plan cell frequency overflow"))?;
        cell_target[cell].add(plan.target_mass[stratum]);
        let implied = plan.per_copy_mass[stratum] * plan.physical_count[stratum] as f64;
        if !aggregate_close(implied, plan.target_mass[stratum]) {
            return Err(BackendError::new(
                ErrorCode::TargetIdentityFailed,
                "generic_jla_target_plan",
                "a target stratum does not preserve stored-row target mass",
            ));
        }
    }
    for cell in 0..problem.cells() {
        checkpoint_chunk(interrupt, cell, "generic_jla_target_validate")?;
        if cell_frequency[cell] as f64 != problem.cell_weight[cell]
            || !aggregate_close(cell_target[cell].finish(), problem.cell_target_sum[cell])
        {
            return Err(BackendError::new(
                ErrorCode::TargetIdentityFailed,
                "generic_jla_target_plan",
                "target strata do not reproduce a coefficient-cell measure",
            ));
        }
    }
    Ok(())
}

fn aggregate_close(left: f64, right: f64) -> bool {
    let scale = left.abs().max(right.abs()).max(1.0);
    (left - right).abs() <= 64.0 * f64::EPSILON * scale
}

fn match_plan(
    problem: &CompressedProblem,
    row_rank: &[u64],
    options: GenericJlaOptions,
    interrupt: &mut dyn InterruptCheck,
) -> Result<MatchPlan> {
    if problem.controls.is_empty() {
        let legacy = JlaPlan::build_no_controls_with_interrupt(problem, interrupt)?;
        for (group, &trials) in legacy.deletion.physical_count.iter().enumerate() {
            checkpoint_chunk(interrupt, group, "generic_jla_match_preflight")?;
            preflight_trials(trials, "match deletion unit")?;
        }
        let mut rows = Vec::new();
        reserve_exact(&mut rows, problem.deletion_units(), "match-plan row blocks")?;
        for group in 0..problem.deletion_units() {
            checkpoint_chunk(interrupt, group, "generic_jla_match_plan")?;
            let mut indices = group_rows(problem, group, interrupt, "generic_jla_match_plan")?;
            if indices.is_empty() || indices.len() > options.blocksize_limit {
                return Err(BackendError::new(
                    ErrorCode::ResourceLimit,
                    "generic_jla_match_plan",
                    "a match block is empty or exceeds blocksize_limit",
                ));
            }
            stable_sort_by_with_interrupt(
                &mut indices,
                |left, right| row_rank[*left].cmp(&row_rank[*right]),
                interrupt,
                "generic_jla_match_row_sort",
            )?;
            rows.push(indices);
        }
        return Ok(MatchPlan {
            rows,
            cell: legacy.deletion.cell,
            physical_count: legacy.deletion.physical_count,
            entity: legacy.deletion.semantic_rank,
        });
    }
    let groups = problem.deletion_units();
    let mut plan = MatchPlan {
        rows: Vec::new(),
        cell: Vec::new(),
        physical_count: Vec::new(),
        entity: Vec::new(),
    };
    reserve_exact(&mut plan.rows, groups, "match-plan row blocks")?;
    reserve_exact(&mut plan.cell, groups, "match-plan cells")?;
    reserve_exact(&mut plan.physical_count, groups, "match-plan counts")?;
    reserve_exact(&mut plan.entity, groups, "match-plan entities")?;
    for group in 0..groups {
        checkpoint_chunk(interrupt, group, "generic_jla_match_plan")?;
        let mut indices = group_rows(problem, group, interrupt, "generic_jla_match_plan")?;
        if indices.is_empty() || indices.len() > options.blocksize_limit {
            return Err(BackendError::new(
                ErrorCode::ResourceLimit,
                "generic_jla_match_plan",
                "a match block is empty or exceeds blocksize_limit",
            ));
        }
        stable_sort_by_with_interrupt(
            &mut indices,
            |left, right| row_rank[*left].cmp(&row_rank[*right]),
            interrupt,
            "generic_jla_match_row_sort",
        )?;
        let cell = problem.row_cell[indices[0]];
        let mut physical = 0_u64;
        let mut entity = u64::MAX;
        for (local, &row) in indices.iter().enumerate() {
            checkpoint_chunk(interrupt, local, "generic_jla_match_plan_rows")?;
            if problem.row_cell[row] != cell {
                return Err(BackendError::new(
                    ErrorCode::InvalidIdentifier,
                    "generic_jla_match_plan",
                    "one match deletion unit crosses coefficient cells",
                ));
            }
            physical = physical
                .checked_add(problem.frequency[row])
                .ok_or_else(|| resource("match physical-count overflow"))?;
            entity = entity.min(row_rank[row]);
        }
        preflight_trials(physical, "match deletion unit")?;
        plan.rows.push(indices);
        plan.cell.push(cell);
        plan.physical_count.push(physical);
        plan.entity.push(entity);
    }
    Ok(plan)
}

fn observation_classes(
    problem: &CompressedProblem,
    row_rank: &[u64],
    interrupt: &mut dyn InterruptCheck,
) -> Result<Vec<ObservationClass>> {
    let mut order = index_vector(
        problem.outcome.len(),
        interrupt,
        "generic_jla_observation_classes",
    )?;
    stable_sort_by_with_interrupt(
        &mut order,
        |left, right| {
            row_rank[*left]
                .cmp(&row_rank[*right])
                .then_with(|| left.cmp(right))
        },
        interrupt,
        "generic_jla_observation_classes",
    )?;
    let mut output = Vec::new();
    reserve_exact(
        &mut output,
        problem.outcome.len(),
        "observation semantic classes",
    )?;
    let mut cursor = 0;
    while cursor < order.len() {
        checkpoint_chunk(interrupt, cursor, "generic_jla_observation_class_scan")?;
        let entity = row_rank[order[cursor]];
        let begin = cursor;
        while cursor < order.len() && row_rank[order[cursor]] == entity {
            checkpoint_chunk(
                interrupt,
                cursor - begin,
                "generic_jla_observation_class_scan",
            )?;
            cursor += 1;
        }
        let mut rows = Vec::new();
        reserve_exact(&mut rows, cursor - begin, "observation-class rows")?;
        let mut physical = 0_u64;
        for (local, &row) in order[begin..cursor].iter().enumerate() {
            checkpoint_chunk(interrupt, local, "generic_jla_observation_class_rows")?;
            physical = physical
                .checked_add(problem.frequency[row])
                .ok_or_else(|| resource("observation-class physical-count overflow"))?;
            rows.push(row);
        }
        preflight_trials(physical, "observation class")?;
        output.push(ObservationClass {
            rows,
            entity,
            physical_count: physical,
        });
    }
    Ok(output)
}

fn observation_row_ranks(
    problem: &CompressedProblem,
    controls: &[Vec<f64>],
    interrupt: &mut dyn InterruptCheck,
) -> Result<Vec<u64>> {
    let rows = problem.outcome.len();
    let mut order = index_vector(rows, interrupt, "generic_jla_observation_semantic_sort")?;
    stable_sort_by_with_interrupt(
        &mut order,
        |&left, &right| compare_observation_rows(problem, controls, left, right),
        interrupt,
        "generic_jla_observation_semantic_sort",
    )?;
    let mut rank = repeated(
        rows,
        0_u64,
        "observation semantic ranks",
        interrupt,
        "generic_jla_observation_semantic_allocate",
    )?;
    let mut current = 0_u64;
    let mut previous: Option<usize> = None;
    for (position, row) in order.into_iter().enumerate() {
        checkpoint_chunk(interrupt, position, "generic_jla_observation_semantic_rank")?;
        let starts = previous.map_or(true, |prior| {
            compare_observation_rows(problem, controls, prior, row) != Ordering::Equal
        });
        if starts {
            current = current
                .checked_add(1)
                .ok_or_else(|| resource("observation semantic rank overflow"))?;
        }
        rank[row] = current;
        previous = Some(row);
    }
    Ok(rank)
}

fn compare_observation_rows(
    problem: &CompressedProblem,
    controls: &[Vec<f64>],
    left: usize,
    right: usize,
) -> Ordering {
    let mut ordering = problem.row_worker[left]
        .cmp(&problem.row_worker[right])
        .then_with(|| problem.row_firm[left].cmp(&problem.row_firm[right]))
        .then_with(|| {
            canonical_zero(per_copy_mass(problem, left))
                .total_cmp(&canonical_zero(per_copy_mass(problem, right)))
        })
        .then_with(|| problem.outcome[left].total_cmp(&problem.outcome[right]));
    for column in controls {
        ordering = ordering
            .then_with(|| canonical_zero(column[left]).total_cmp(&canonical_zero(column[right])));
    }
    if let Some(probe_order) = &problem.probe_order {
        ordering = ordering.then_with(|| {
            canonical_zero(probe_order[left]).total_cmp(&canonical_zero(probe_order[right]))
        });
    }
    ordering
}

#[inline]
fn per_copy_mass(problem: &CompressedProblem, row: usize) -> f64 {
    problem.target_weight[row] / problem.frequency[row] as f64
}

#[inline]
fn canonical_zero(value: f64) -> f64 {
    if value == 0.0 {
        0.0
    } else {
        value
    }
}

fn control_geometry(
    problem: &CompressedProblem,
    controls: &[Vec<f64>],
    weights: &[f64],
    row_order: &[usize],
    residualized: Vec<f64>,
    options: GenericJlaOptions,
    interrupt: &mut dyn InterruptCheck,
) -> Result<ControlGeometry> {
    let q = controls.len();
    let rows = problem.outcome.len();
    if q == 0 {
        if !residualized.is_empty() {
            return Err(BackendError::invariant(
                "generic_jla_control_geometry",
                "zero-control geometry received a nonempty residualized-control artifact",
            ));
        }
        return Ok(ControlGeometry {
            controls: 0,
            residualized,
            factor: Vec::new(),
            leverage: zeroed_f64_with_interrupt(
                rows,
                "zero control leverage",
                interrupt,
                "generic_jla_control_allocate",
            )?,
            rcond: 1.0,
            relres: 0.0,
        });
    }
    let expected = checked_matrix_length(rows, q, "residualized controls")?;
    if residualized.len() != expected {
        return Err(BackendError::invariant(
            "generic_jla_control_geometry",
            "retained residualized-control artifact has the wrong dimension",
        ));
    }
    for (position, &value) in residualized.iter().enumerate() {
        checkpoint_chunk(
            interrupt,
            position,
            "generic_jla_control_residualized_validate",
        )?;
        if !value.is_finite() {
            return Err(BackendError::invariant(
                "generic_jla_control_geometry",
                "retained residualized-control artifact is nonfinite",
            ));
        }
    }
    let information = weighted_crossproduct_flat(
        &residualized,
        q,
        rows,
        weights,
        row_order,
        interrupt,
        "generic_jla_control_schur",
    )?;
    let inverse = invert_scaled_spd(
        &information,
        q,
        options.rank_tolerance,
        interrupt,
        "generic_jla_control_schur",
    )?;
    let factor = cholesky_factor(&inverse.inverse, q, interrupt, "generic_jla_control_factor")
        .map_err(|error| {
            preserve_break(
                error,
                ErrorCode::InverseResidualFailed,
                "control inverse square root failed",
            )
        })?;
    let factor_error = factor_residual(&factor, &inverse.inverse, q, interrupt)?;
    if !factor_error.is_finite()
        || factor_error > 100.0 * options.rank_tolerance * (1.0 + frobenius_norm(&inverse.inverse))
    {
        return Err(BackendError::new(
            ErrorCode::InverseResidualFailed,
            "generic_jla_control_factor",
            "control inverse square root failed its residual gate",
        ));
    }
    let mut leverage = zeroed_f64_with_interrupt(
        rows,
        "analytic control leverage",
        interrupt,
        "generic_jla_control_allocate",
    )?;
    let mut work = 0_usize;
    for (position, &row) in row_order.iter().enumerate() {
        checkpoint_chunk(interrupt, position, "generic_jla_control_leverage")?;
        let mut value = StableAccumulator::default();
        for left in 0..q {
            for right in 0..q {
                checkpoint_chunk(interrupt, work, "generic_jla_control_leverage")?;
                work = work.saturating_add(1);
                value.add(
                    residualized[left * rows + row]
                        * inverse.inverse[left * q + right]
                        * residualized[right * rows + row],
                );
            }
        }
        let value = value.finish();
        if !value.is_finite() || value < -100.0 * options.rank_tolerance {
            return Err(BackendError::new(
                ErrorCode::JlaMomentFailed,
                "generic_jla_control_leverage",
                "analytic control leverage is negative or nonfinite",
            ));
        }
        leverage[row] = value.max(0.0);
    }
    Ok(ControlGeometry {
        controls: q,
        residualized,
        factor,
        leverage,
        rcond: inverse.rcond,
        relres: inverse
            .relres
            .max(inverse.original_relres)
            .max(factor_error),
    })
}

fn deletion_rank_certificate(
    problem: &CompressedProblem,
    controls: &[Vec<f64>],
    weights: &[f64],
    row_order: &[usize],
    options: GenericJlaOptions,
    interrupt: &mut dyn InterruptCheck,
) -> Result<f64> {
    let q = controls.len();
    if q == 0 {
        return Ok(1.0);
    }
    let rows = problem.outcome.len();
    let cells = problem.cells();
    let mut cell_frequency = zeroed_f64_with_interrupt(
        cells,
        "rank-certificate cell frequencies",
        interrupt,
        "generic_jla_rank_allocate",
    )?;
    let mut cell_sum = zeroed_f64_with_interrupt(
        checked_matrix_length(cells, q, "rank-certificate cell sums")?,
        "rank-certificate cell sums",
        interrupt,
        "generic_jla_rank_allocate",
    )?;
    let mut cell_frequency_correction = zeroed_f64_with_interrupt(
        cells,
        "rank-certificate cell-frequency corrections",
        interrupt,
        "generic_jla_rank_allocate",
    )?;
    let mut cell_sum_correction = zeroed_f64_with_interrupt(
        cell_sum.len(),
        "rank-certificate cell-sum corrections",
        interrupt,
        "generic_jla_rank_allocate",
    )?;
    let mut work = 0_usize;
    for (position, &row) in row_order.iter().enumerate() {
        checkpoint_chunk(interrupt, position, "generic_jla_rank_cell_means")?;
        let cell = problem.row_cell[row] as usize;
        stable_add_index(
            &mut cell_frequency,
            &mut cell_frequency_correction,
            cell,
            weights[row],
        );
        for control in 0..q {
            checkpoint_chunk(interrupt, work, "generic_jla_rank_cell_means")?;
            work = work.saturating_add(1);
            stable_add_index(
                &mut cell_sum,
                &mut cell_sum_correction,
                cell * q + control,
                weights[row] * controls[control][row],
            );
        }
    }
    finish_stable_vector(
        &mut cell_frequency,
        &cell_frequency_correction,
        interrupt,
        "generic_jla_rank_cell_means",
    )?;
    finish_stable_vector(
        &mut cell_sum,
        &cell_sum_correction,
        interrupt,
        "generic_jla_rank_cell_means",
    )?;
    drop(cell_frequency_correction);
    drop(cell_sum_correction);
    let mut centered = zeroed_matrix(
        q,
        rows,
        "rank-certificate centered controls",
        interrupt,
        "generic_jla_rank_allocate",
    )?;
    for control in 0..q {
        for (position, &row) in row_order.iter().enumerate() {
            checkpoint_chunk(
                interrupt,
                control * rows + position,
                "generic_jla_rank_center",
            )?;
            let cell = problem.row_cell[row] as usize;
            centered[control][row] =
                controls[control][row] - cell_sum[cell * q + control] / cell_frequency[cell];
        }
    }
    // Preserve the Mata formula including the small floating-point cell-sum
    // correction after centering.
    let mut centered_cell_sum = zeroed_f64_with_interrupt(
        checked_matrix_length(cells, q, "centered-control cell sums")?,
        "centered-control cell sums",
        interrupt,
        "generic_jla_rank_allocate",
    )?;
    let mut centered_cell_correction = zeroed_f64_with_interrupt(
        centered_cell_sum.len(),
        "centered-control cell-sum corrections",
        interrupt,
        "generic_jla_rank_allocate",
    )?;
    work = 0;
    for (position, &row) in row_order.iter().enumerate() {
        checkpoint_chunk(interrupt, position, "generic_jla_rank_centered_sum")?;
        let cell = problem.row_cell[row] as usize;
        for control in 0..q {
            checkpoint_chunk(interrupt, work, "generic_jla_rank_centered_sum")?;
            work = work.saturating_add(1);
            stable_add_index(
                &mut centered_cell_sum,
                &mut centered_cell_correction,
                cell * q + control,
                weights[row] * centered[control][row],
            );
        }
    }
    finish_stable_vector(
        &mut centered_cell_sum,
        &centered_cell_correction,
        interrupt,
        "generic_jla_rank_centered_sum",
    )?;
    drop(centered_cell_correction);
    let mut within = weighted_crossproduct(
        &centered,
        weights,
        row_order,
        interrupt,
        "generic_jla_rank_within",
    )?;
    for left in 0..q {
        for right in 0..q {
            let mut corrected = StableAccumulator::default();
            corrected.add(within[left * q + right]);
            for cell in 0..cells {
                checkpoint_chunk(interrupt, work, "generic_jla_rank_within_correction")?;
                work = work.saturating_add(1);
                corrected.add(
                    -centered_cell_sum[cell * q + left] * centered_cell_sum[cell * q + right]
                        / cell_frequency[cell],
                );
            }
            within[left * q + right] = corrected.finish();
        }
    }
    symmetrize(&mut within, q, interrupt, "generic_jla_rank_within")?;
    let inverse = invert_scaled_spd(
        &within,
        q,
        options.rank_tolerance,
        interrupt,
        "generic_jla_rank_within",
    )
    .map_err(|error| {
        preserve_break(
            error,
            ErrorCode::UnverifiedDeletionRank,
            "within-cell control variation cannot certify every deletion",
        )
    })?;
    let whitener = cholesky_factor(&inverse.inverse, q, interrupt, "generic_jla_rank_whitener")
        .map_err(|error| {
            preserve_break(
                error,
                ErrorCode::UnverifiedDeletionRank,
                "within-cell control whitening failed",
            )
        })?;
    let mut transformed = zeroed_matrix(
        q,
        rows,
        "rank-certificate transformed controls",
        interrupt,
        "generic_jla_rank_allocate",
    )?;
    work = 0;
    for (position, &row) in row_order.iter().enumerate() {
        checkpoint_chunk(interrupt, position, "generic_jla_rank_transform")?;
        for output in 0..q {
            // transformed row = centered row * L.
            let mut value = StableAccumulator::default();
            for input in 0..q {
                checkpoint_chunk(interrupt, work, "generic_jla_rank_transform")?;
                work = work.saturating_add(1);
                value.add(centered[input][row] * whitener[input * q + output]);
            }
            transformed[output][row] = value.finish();
        }
    }
    let mut transformed_cell_sum = zeroed_f64_with_interrupt(
        checked_matrix_length(cells, q, "transformed-control cell sums")?,
        "transformed-control cell sums",
        interrupt,
        "generic_jla_rank_allocate",
    )?;
    let mut transformed_cell_correction = zeroed_f64_with_interrupt(
        transformed_cell_sum.len(),
        "transformed-control cell-sum corrections",
        interrupt,
        "generic_jla_rank_allocate",
    )?;
    work = 0;
    for (position, &row) in row_order.iter().enumerate() {
        checkpoint_chunk(interrupt, position, "generic_jla_rank_transformed_sum")?;
        let cell = problem.row_cell[row] as usize;
        for control in 0..q {
            checkpoint_chunk(interrupt, work, "generic_jla_rank_transformed_sum")?;
            work = work.saturating_add(1);
            stable_add_index(
                &mut transformed_cell_sum,
                &mut transformed_cell_correction,
                cell * q + control,
                weights[row] * transformed[control][row],
            );
        }
    }
    finish_stable_vector(
        &mut transformed_cell_sum,
        &transformed_cell_correction,
        interrupt,
        "generic_jla_rank_transformed_sum",
    )?;
    drop(transformed_cell_correction);
    let mut checked = weighted_crossproduct(
        &transformed,
        weights,
        row_order,
        interrupt,
        "generic_jla_rank_whitening_check",
    )?;
    for left in 0..q {
        for right in 0..q {
            let mut corrected = StableAccumulator::default();
            corrected.add(checked[left * q + right]);
            for cell in 0..cells {
                checkpoint_chunk(interrupt, work, "generic_jla_rank_whitening_correction")?;
                work = work.saturating_add(1);
                corrected.add(
                    -transformed_cell_sum[cell * q + left] * transformed_cell_sum[cell * q + right]
                        / cell_frequency[cell],
                );
            }
            checked[left * q + right] = corrected.finish();
        }
    }
    for diagonal in 0..q {
        checked[diagonal * q + diagonal] -= 1.0;
    }
    let whitening_error = frobenius_norm(&checked);
    let threshold = (1000.0 * options.rank_tolerance).max(1.0e-10);
    if !whitening_error.is_finite() || whitening_error > threshold {
        return Err(BackendError::new(
            ErrorCode::UnverifiedDeletionRank,
            "generic_jla_rank_whitening_check",
            "within-cell control whitening failed its residual gate",
        ));
    }
    for diagonal in 0..q {
        let mut value = StableAccumulator::default();
        value.add(checked[diagonal * q + diagonal]);
        value.add(1.0);
        checked[diagonal * q + diagonal] = value.finish();
    }

    let mut maximum_loss = 0.0_f64;
    let mut minimum_deleted = f64::INFINITY;
    match options.deletion {
        DeletionMode::Match => {
            for group in 0..problem.deletion_units() {
                checkpoint_chunk(interrupt, group, "generic_jla_rank_match")?;
                let mut indices = group_rows(problem, group, interrupt, "generic_jla_rank_match")?;
                if indices.is_empty() || indices.len() > options.blocksize_limit {
                    return Err(BackendError::new(
                        ErrorCode::ResourceLimit,
                        "generic_jla_rank_match",
                        "a match block is empty or exceeds blocksize_limit",
                    ));
                }
                stable_sort_by_with_interrupt(
                    &mut indices,
                    |left, right| compare_semantic_rows(problem, controls, *left, *right),
                    interrupt,
                    "generic_jla_rank_match_sort",
                )?;
                let cell = problem.row_cell[indices[0]] as usize;
                let block_frequency = stable_sum_indices(
                    &indices,
                    weights,
                    interrupt,
                    "generic_jla_rank_match_frequency",
                )?;
                let remaining = cell_frequency[cell] - block_frequency;
                let mut block_sum = zeroed_f64_with_interrupt(
                    q,
                    "rank-certificate block sums",
                    interrupt,
                    "generic_jla_rank_match_allocate",
                )?;
                let mut deleted_scatter = zeroed_f64_with_interrupt(
                    checked_matrix_length(q, q, "rank-certificate deleted scatter")?,
                    "rank-certificate deleted scatter",
                    interrupt,
                    "generic_jla_rank_match_allocate",
                )?;
                let mut block_sum_correction = zeroed_f64_with_interrupt(
                    q,
                    "rank-certificate block-sum corrections",
                    interrupt,
                    "generic_jla_rank_match_allocate",
                )?;
                let mut deleted_scatter_correction = zeroed_f64_with_interrupt(
                    deleted_scatter.len(),
                    "rank-certificate deleted-scatter corrections",
                    interrupt,
                    "generic_jla_rank_match_allocate",
                )?;
                work = 0;
                for (local, &row) in indices.iter().enumerate() {
                    checkpoint_chunk(interrupt, local, "generic_jla_rank_match_rows")?;
                    if problem.row_cell[row] as usize != cell {
                        return Err(BackendError::new(
                            ErrorCode::InvalidIdentifier,
                            "generic_jla_rank_match",
                            "one match deletion unit crosses coefficient cells",
                        ));
                    }
                    for left in 0..q {
                        stable_add_index(
                            &mut block_sum,
                            &mut block_sum_correction,
                            left,
                            weights[row] * transformed[left][row],
                        );
                        for right in 0..q {
                            checkpoint_chunk(interrupt, work, "generic_jla_rank_match_rows")?;
                            work = work.saturating_add(1);
                            stable_add_index(
                                &mut deleted_scatter,
                                &mut deleted_scatter_correction,
                                left * q + right,
                                weights[row] * transformed[left][row] * transformed[right][row],
                            );
                        }
                    }
                }
                finish_stable_vector(
                    &mut block_sum,
                    &block_sum_correction,
                    interrupt,
                    "generic_jla_rank_match_rows",
                )?;
                finish_stable_vector(
                    &mut deleted_scatter,
                    &deleted_scatter_correction,
                    interrupt,
                    "generic_jla_rank_match_rows",
                )?;
                drop(block_sum_correction);
                drop(deleted_scatter_correction);
                for left in 0..q {
                    for right in 0..q {
                        checkpoint_chunk(
                            interrupt,
                            left * q + right,
                            "generic_jla_rank_match_scatter",
                        )?;
                        let mut value = StableAccumulator::default();
                        value.add(deleted_scatter[left * q + right]);
                        value.add(-block_sum[left] * block_sum[right] / block_frequency);
                        deleted_scatter[left * q + right] = value.finish();
                    }
                }
                symmetrize(
                    &mut deleted_scatter,
                    q,
                    interrupt,
                    "generic_jla_rank_deleted_scatter",
                )?;
                let deleted_extremes = symmetric_eigen_extremes(
                    &deleted_scatter,
                    q,
                    interrupt,
                    "generic_jla_rank_deleted_scatter",
                )
                .map_err(|error| {
                    preserve_break(
                        error,
                        ErrorCode::UnverifiedDeletionRank,
                        "match deleted scatter spectrum failed",
                    )
                })?;
                if !deleted_extremes.smallest_lower.is_finite()
                    || deleted_extremes.smallest_lower < -threshold
                {
                    return Err(BackendError::new(
                        ErrorCode::UnverifiedDeletionRank,
                        "generic_jla_rank_deleted_scatter",
                        "match deleted scatter is numerically indefinite",
                    ));
                }
                let mut loss = deleted_scatter;
                if remaining > 0.0 {
                    for left in 0..q {
                        let left_gap = block_sum[left] / block_frequency
                            - (transformed_cell_sum[cell * q + left] - block_sum[left]) / remaining;
                        for right in 0..q {
                            checkpoint_chunk(
                                interrupt,
                                left * q + right,
                                "generic_jla_rank_match_loss",
                            )?;
                            let right_gap = block_sum[right] / block_frequency
                                - (transformed_cell_sum[cell * q + right] - block_sum[right])
                                    / remaining;
                            let mut value = StableAccumulator::default();
                            value.add(loss[left * q + right]);
                            value.add(
                                block_frequency * remaining / cell_frequency[cell]
                                    * left_gap
                                    * right_gap,
                            );
                            loss[left * q + right] = value.finish();
                        }
                    }
                }
                certify_deleted_scatter(
                    &checked,
                    &loss,
                    q,
                    threshold,
                    options,
                    &mut maximum_loss,
                    &mut minimum_deleted,
                    interrupt,
                    "generic_jla_rank_match",
                )?;
            }
        }
        DeletionMode::Observation => {
            for (position, &row) in row_order.iter().enumerate() {
                checkpoint_chunk(interrupt, position, "generic_jla_rank_observation")?;
                let cell = problem.row_cell[row] as usize;
                let remaining = cell_frequency[cell] - 1.0;
                let mut loss = zeroed_f64_with_interrupt(
                    checked_matrix_length(q, q, "observation scatter loss")?,
                    "observation scatter loss",
                    interrupt,
                    "generic_jla_rank_observation_allocate",
                )?;
                if remaining > 0.0 {
                    for left in 0..q {
                        let left_gap = transformed[left][row]
                            - (transformed_cell_sum[cell * q + left] - transformed[left][row])
                                / remaining;
                        for right in 0..q {
                            checkpoint_chunk(
                                interrupt,
                                left * q + right,
                                "generic_jla_rank_observation_scatter",
                            )?;
                            let right_gap = transformed[right][row]
                                - (transformed_cell_sum[cell * q + right]
                                    - transformed[right][row])
                                    / remaining;
                            loss[left * q + right] =
                                remaining / cell_frequency[cell] * left_gap * right_gap;
                        }
                    }
                }
                certify_deleted_scatter(
                    &checked,
                    &loss,
                    q,
                    threshold,
                    options,
                    &mut maximum_loss,
                    &mut minimum_deleted,
                    interrupt,
                    "generic_jla_rank_observation",
                )?;
            }
        }
    }
    let gap = (1.0 - maximum_loss).min(minimum_deleted) - whitening_error - threshold;
    if !gap.is_finite() || gap <= 0.0 {
        return Err(BackendError::new(
            ErrorCode::UnverifiedDeletionRank,
            "generic_jla_rank",
            "within-cell control variation does not certify rank after every deletion",
        ));
    }
    Ok(gap)
}

#[allow(clippy::too_many_arguments)]
fn certify_deleted_scatter(
    checked: &[f64],
    loss: &[f64],
    q: usize,
    threshold: f64,
    options: GenericJlaOptions,
    maximum_loss: &mut f64,
    minimum_deleted: &mut f64,
    interrupt: &mut dyn InterruptCheck,
    phase: &'static str,
) -> Result<()> {
    let mut checked_loss = copy_f64(loss, "checked deleted-scatter loss", interrupt)?;
    symmetrize(&mut checked_loss, q, interrupt, phase)?;
    let loss_extremes =
        symmetric_eigen_extremes(&checked_loss, q, interrupt, phase).map_err(|error| {
            preserve_break(
                error,
                ErrorCode::UnverifiedDeletionRank,
                "deleted-scatter loss spectrum failed",
            )
        })?;
    if !loss_extremes.smallest_lower.is_finite() || loss_extremes.smallest_lower < -threshold {
        return Err(BackendError::new(
            ErrorCode::UnverifiedDeletionRank,
            phase,
            "deleted-scatter loss is numerically indefinite",
        ));
    }
    let mut trace = StableAccumulator::default();
    for index in 0..q {
        checkpoint_chunk(interrupt, index, phase)?;
        trace.add(checked_loss[index * q + index]);
    }
    let trace = trace.finish();
    if !trace.is_finite() || trace < -threshold {
        return Err(BackendError::new(
            ErrorCode::UnverifiedDeletionRank,
            phase,
            "deletion-rank scatter loss is numerically inconsistent",
        ));
    }
    *maximum_loss = (*maximum_loss).max(trace.max(0.0));
    let mut deleted = copy_f64(checked, "deleted control scatter", interrupt)?;
    for (index, (value, subtract)) in deleted.iter_mut().zip(&checked_loss).enumerate() {
        checkpoint_chunk(interrupt, index, phase)?;
        let mut difference = StableAccumulator::default();
        difference.add(*value);
        difference.add(-subtract);
        *value = difference.finish();
    }
    symmetrize(&mut deleted, q, interrupt, phase)?;
    let extremes = symmetric_eigen_extremes(&deleted, q, interrupt, phase).map_err(|error| {
        preserve_break(
            error,
            ErrorCode::UnverifiedDeletionRank,
            "deleted control scatter spectrum failed",
        )
    })?;
    *minimum_deleted = (*minimum_deleted).min(extremes.smallest_lower);
    invert_scaled_spd(&deleted, q, options.rank_tolerance, interrupt, phase)
        .map(|_| ())
        .map_err(|error| {
            preserve_break(
                error,
                ErrorCode::UnverifiedDeletionRank,
                "deletion does not retain a directly factorable control scatter",
            )
        })
}

fn weighted_crossproduct(
    columns: &[Vec<f64>],
    weights: &[f64],
    row_order: &[usize],
    interrupt: &mut dyn InterruptCheck,
    phase: &'static str,
) -> Result<Vec<f64>> {
    let q = columns.len();
    let mut output = zeroed_f64_with_interrupt(
        checked_matrix_length(q, q, "weighted crossproduct")?,
        "weighted crossproduct",
        interrupt,
        phase,
    )?;
    let mut work = 0;
    for left in 0..q {
        for right in 0..=left {
            let mut value = StableAccumulator::default();
            for &row in row_order {
                checkpoint_chunk(interrupt, work, phase)?;
                work = work.saturating_add(1);
                value.add(weights[row] * columns[left][row] * columns[right][row]);
            }
            let value = value.finish();
            output[left * q + right] = value;
            output[right * q + left] = value;
        }
    }
    Ok(output)
}

fn weighted_crossproduct_flat(
    columns: &[f64],
    column_count: usize,
    rows: usize,
    weights: &[f64],
    row_order: &[usize],
    interrupt: &mut dyn InterruptCheck,
    phase: &'static str,
) -> Result<Vec<f64>> {
    if columns.len() != checked_matrix_length(rows, column_count, "weighted crossproduct")?
        || weights.len() != rows
        || row_order.len() != rows
    {
        return Err(BackendError::invariant(
            phase,
            "flat weighted-crossproduct dimensions disagree",
        ));
    }
    let mut output = zeroed_f64_with_interrupt(
        checked_matrix_length(column_count, column_count, "weighted crossproduct")?,
        "weighted crossproduct",
        interrupt,
        phase,
    )?;
    let mut work = 0_usize;
    for left in 0..column_count {
        for right in 0..=left {
            let mut value = StableAccumulator::default();
            for &row in row_order {
                checkpoint_chunk(interrupt, work, phase)?;
                work = work.saturating_add(1);
                value.add(weights[row] * columns[left * rows + row] * columns[right * rows + row]);
            }
            let value = value.finish();
            output[left * column_count + right] = value;
            output[right * column_count + left] = value;
        }
    }
    Ok(output)
}

fn factor_residual(
    factor: &[f64],
    matrix: &[f64],
    dimension: usize,
    interrupt: &mut dyn InterruptCheck,
) -> Result<f64> {
    let mut square = StableAccumulator::default();
    let mut work = 0_usize;
    for row in 0..dimension {
        for column in 0..dimension {
            let mut value = StableAccumulator::default();
            value.add(-matrix[row * dimension + column]);
            for inner in 0..dimension {
                checkpoint_chunk(interrupt, work, "generic_jla_factor_residual")?;
                work = work.saturating_add(1);
                value.add(factor[row * dimension + inner] * factor[column * dimension + inner]);
            }
            let value = value.finish();
            square.add(value * value);
        }
    }
    Ok(square.finish().sqrt())
}

fn symmetrize(
    matrix: &mut [f64],
    dimension: usize,
    interrupt: &mut dyn InterruptCheck,
    phase: &'static str,
) -> Result<()> {
    let mut work = 0_usize;
    for row in 0..dimension {
        for column in 0..row {
            checkpoint_chunk(interrupt, work, phase)?;
            work = work.saturating_add(1);
            let value = 0.5 * (matrix[row * dimension + column] + matrix[column * dimension + row]);
            matrix[row * dimension + column] = value;
            matrix[column * dimension + row] = value;
        }
    }
    Ok(())
}

fn match_leverage_moments(
    problem: &CompressedProblem,
    plan: &MatchPlan,
    fe_solver: &PreparedModelSolver<'_>,
    rng: CounterRng,
    options: GenericJlaOptions,
    rhs_receipts: &mut Vec<GenericJlaRhsReceipt>,
    interrupt: &mut dyn InterruptCheck,
) -> Result<(Vec<FiveMoments>, f64)> {
    let groups = plan.rows.len();
    let mut moments = repeated(
        groups,
        FiveMoments::default(),
        "match leverage moments",
        interrupt,
        "generic_jla_match_leverage_allocate",
    )?;
    let mut maximum_relres = 0.0_f64;
    for first in (0..options.probes as usize).step_by(options.leverage_batch_width) {
        interrupt.checkpoint("generic_jla_leverage_batch")?;
        let width = options
            .leverage_batch_width
            .min(options.probes as usize - first);
        let mut atoms = repeated(
            checked_matrix_length(groups, width, "match leverage atoms")?,
            0_i64,
            "match leverage atoms",
            interrupt,
            "generic_jla_match_leverage_allocate",
        )?;
        rng.fill_rademacher_sums_with_interrupt(
            ProbeDomain::Leverage,
            first as u64,
            width,
            &plan.entity,
            &plan.physical_count,
            &mut atoms,
            interrupt,
        )?;
        let mut worker_rhs = zeroed_f64_with_interrupt(
            checked_matrix_length(problem.workers(), width, "match leverage worker RHS")?,
            "match leverage worker RHS",
            interrupt,
            "generic_jla_match_leverage_allocate",
        )?;
        let mut firm_rhs = zeroed_f64_with_interrupt(
            checked_matrix_length(problem.firms(), width, "match leverage firm RHS")?,
            "match leverage firm RHS",
            interrupt,
            "generic_jla_match_leverage_allocate",
        )?;
        for column in 0..width {
            for group in 0..groups {
                checkpoint_chunk(
                    interrupt,
                    column * groups + group,
                    "generic_jla_match_leverage_rhs",
                )?;
                let cell = plan.cell[group] as usize;
                let atom = atoms[column * groups + group] as f64;
                worker_rhs[column * problem.workers() + problem.cell_worker[cell] as usize] += atom;
                firm_rhs[column * problem.firms() + problem.cell_firm[cell] as usize] += atom;
            }
        }
        let solved = fe_solver.solve_batch_with_interrupt(
            &worker_rhs,
            &firm_rhs,
            &[],
            width,
            width,
            interrupt,
        )?;
        let mut prediction = zeroed_f64_with_interrupt(
            problem.outcome.len(),
            "match leverage predictions",
            interrupt,
            "generic_jla_match_leverage_allocate",
        )?;
        for column in 0..width {
            let solution = &solved.solution[column];
            rhs_receipts.push(rhs_receipt(
                GenericJlaRhsPhase::Leverage,
                GenericJlaRhsSide::Joint,
                Some(
                    u32::try_from(first + column).map_err(|_| {
                        resource("leverage probe index is not representable as u32")
                    })?,
                ),
                solution,
            ));
            maximum_relres = maximum_relres.max(solution.residual.relative_norm);
            fe_solver.operator().predict_into_with_interrupt(
                &solution.coefficients.worker,
                &solution.coefficients.firm,
                &[],
                &mut prediction,
                interrupt,
            )?;
            for group in 0..groups {
                checkpoint_chunk(interrupt, group, "generic_jla_match_leverage_moments")?;
                let frequency = plan.physical_count[group] as f64;
                let projection = frequency.sqrt() * prediction[plan.rows[group][0]];
                let residual =
                    atoms[column * groups + group] as f64 / frequency.sqrt() - projection;
                if !projection.is_finite() || !residual.is_finite() {
                    return Err(BackendError::new(
                        ErrorCode::JlaMomentFailed,
                        "generic_jla_match_leverage_moments",
                        "match leverage projection is nonfinite",
                    ));
                }
                moments[group].add(projection, residual);
            }
        }
    }
    Ok((moments, maximum_relres))
}

#[derive(Clone, Debug)]
struct ObservationMoments {
    projection_square_sum: Vec<f64>,
    projection_fourth_sum: Vec<f64>,
}

#[derive(Clone, Debug)]
struct ObservationCorrelations {
    row_offset: Vec<usize>,
    first: Vec<f64>,
    third: Vec<f64>,
}

fn observation_leverage_moments(
    problem: &CompressedProblem,
    classes: &[ObservationClass],
    fe_solver: &PreparedModelSolver<'_>,
    rng: CounterRng,
    options: GenericJlaOptions,
    rhs_receipts: &mut Vec<GenericJlaRhsReceipt>,
    interrupt: &mut dyn InterruptCheck,
) -> Result<(ObservationMoments, ObservationCorrelations, f64)> {
    let rows = problem.outcome.len();
    let physical = usize::try_from(problem.physical_total)
        .map_err(|_| resource("physical observation count is not addressable"))?;
    let offset_len = rows
        .checked_add(1)
        .ok_or_else(|| resource("observation row-offset length overflow"))?;
    let mut row_offset = repeated(
        offset_len,
        0_usize,
        "observation row offsets",
        interrupt,
        "generic_jla_observation_allocate",
    )?;
    for row in 0..rows {
        checkpoint_chunk(interrupt, row, "generic_jla_observation_offsets")?;
        row_offset[row + 1] = row_offset[row]
            .checked_add(
                usize::try_from(problem.frequency[row])
                    .map_err(|_| resource("physical row frequency is not addressable"))?,
            )
            .ok_or_else(|| resource("physical observation offset overflow"))?;
    }
    if row_offset[rows] != physical {
        return Err(BackendError::invariant(
            "generic_jla_observation",
            "physical observation offsets do not reproduce the total",
        ));
    }
    let mut first_correlation = zeroed_f64_with_interrupt(
        physical,
        "observation first correlations",
        interrupt,
        "generic_jla_observation_allocate",
    )?;
    let mut third_correlation = zeroed_f64_with_interrupt(
        physical,
        "observation third correlations",
        interrupt,
        "generic_jla_observation_allocate",
    )?;
    let mut projection_square_sum = zeroed_f64_with_interrupt(
        rows,
        "observation projection-square sums",
        interrupt,
        "generic_jla_observation_allocate",
    )?;
    let mut projection_fourth_sum = zeroed_f64_with_interrupt(
        rows,
        "observation projection-fourth sums",
        interrupt,
        "generic_jla_observation_allocate",
    )?;
    let mut first_correction = zeroed_f64_with_interrupt(
        physical,
        "observation first-correlation corrections",
        interrupt,
        "generic_jla_observation_allocate",
    )?;
    let mut third_correction = zeroed_f64_with_interrupt(
        physical,
        "observation third-correlation corrections",
        interrupt,
        "generic_jla_observation_allocate",
    )?;
    let mut projection_square_correction = zeroed_f64_with_interrupt(
        rows,
        "observation projection-square corrections",
        interrupt,
        "generic_jla_observation_allocate",
    )?;
    let mut projection_fourth_correction = zeroed_f64_with_interrupt(
        rows,
        "observation projection-fourth corrections",
        interrupt,
        "generic_jla_observation_allocate",
    )?;
    let mut maximum_relres = 0.0_f64;
    for first_probe in (0..options.probes as usize).step_by(options.leverage_batch_width) {
        interrupt.checkpoint("generic_jla_observation_leverage_batch")?;
        let width = options
            .leverage_batch_width
            .min(options.probes as usize - first_probe);
        let mut worker_rhs = zeroed_f64_with_interrupt(
            checked_matrix_length(problem.workers(), width, "observation worker RHS")?,
            "observation worker RHS",
            interrupt,
            "generic_jla_observation_allocate",
        )?;
        let mut firm_rhs = zeroed_f64_with_interrupt(
            checked_matrix_length(problem.firms(), width, "observation firm RHS")?,
            "observation firm RHS",
            interrupt,
            "generic_jla_observation_allocate",
        )?;
        for column in 0..width {
            let probe = (first_probe + column) as u64;
            for class in classes {
                let mut class_offset = 0_u64;
                for &row in &class.rows {
                    let mut atom = 0_i64;
                    for copy in 0..problem.frequency[row] {
                        checkpoint_chunk(
                            interrupt,
                            usize::try_from(class_offset + copy)
                                .map_err(|_| resource("physical observation index overflow"))?,
                            "generic_jla_observation_atoms",
                        )?;
                        atom += i64::from(rademacher_copy(
                            rng,
                            ProbeDomain::Leverage,
                            probe,
                            class.entity,
                            class_offset + copy,
                        ));
                    }
                    class_offset = class_offset
                        .checked_add(problem.frequency[row])
                        .ok_or_else(|| resource("observation class offset overflow"))?;
                    worker_rhs[column * problem.workers() + problem.row_worker[row] as usize] +=
                        atom as f64;
                    firm_rhs[column * problem.firms() + problem.row_firm[row] as usize] +=
                        atom as f64;
                }
                debug_assert_eq!(class_offset, class.physical_count);
            }
        }
        let solved = fe_solver.solve_batch_with_interrupt(
            &worker_rhs,
            &firm_rhs,
            &[],
            width,
            width,
            interrupt,
        )?;
        let mut prediction = zeroed_f64_with_interrupt(
            rows,
            "observation leverage predictions",
            interrupt,
            "generic_jla_observation_allocate",
        )?;
        for column in 0..width {
            let probe = (first_probe + column) as u64;
            let solution = &solved.solution[column];
            rhs_receipts.push(rhs_receipt(
                GenericJlaRhsPhase::Leverage,
                GenericJlaRhsSide::Joint,
                Some(
                    u32::try_from(first_probe + column).map_err(|_| {
                        resource("leverage probe index is not representable as u32")
                    })?,
                ),
                solution,
            ));
            maximum_relres = maximum_relres.max(solution.residual.relative_norm);
            fe_solver.operator().predict_into_with_interrupt(
                &solution.coefficients.worker,
                &solution.coefficients.firm,
                &[],
                &mut prediction,
                interrupt,
            )?;
            for row in 0..rows {
                checkpoint_chunk(interrupt, row, "generic_jla_observation_moments")?;
                let projected = prediction[row];
                stable_add_index(
                    &mut projection_square_sum,
                    &mut projection_square_correction,
                    row,
                    projected * projected,
                );
                stable_add_index(
                    &mut projection_fourth_sum,
                    &mut projection_fourth_correction,
                    row,
                    projected.powi(4),
                );
            }
            for class in classes {
                let mut class_offset = 0_u64;
                for &row in &class.rows {
                    for copy in 0..problem.frequency[row] {
                        checkpoint_chunk(
                            interrupt,
                            usize::try_from(class_offset + copy)
                                .map_err(|_| resource("physical observation index overflow"))?,
                            "generic_jla_observation_correlations",
                        )?;
                        let sign = f64::from(rademacher_copy(
                            rng,
                            ProbeDomain::Leverage,
                            probe,
                            class.entity,
                            class_offset + copy,
                        ));
                        let physical_index = row_offset[row]
                            + usize::try_from(copy)
                                .map_err(|_| resource("physical copy index overflow"))?;
                        stable_add_index(
                            &mut first_correlation,
                            &mut first_correction,
                            physical_index,
                            sign * prediction[row],
                        );
                        stable_add_index(
                            &mut third_correlation,
                            &mut third_correction,
                            physical_index,
                            sign * prediction[row].powi(3),
                        );
                    }
                    class_offset = class_offset
                        .checked_add(problem.frequency[row])
                        .ok_or_else(|| resource("observation class offset overflow"))?;
                }
            }
        }
    }
    finish_stable_vector(
        &mut first_correlation,
        &first_correction,
        interrupt,
        "generic_jla_observation_correlations",
    )?;
    finish_stable_vector(
        &mut third_correlation,
        &third_correction,
        interrupt,
        "generic_jla_observation_correlations",
    )?;
    finish_stable_vector(
        &mut projection_square_sum,
        &projection_square_correction,
        interrupt,
        "generic_jla_observation_moments",
    )?;
    finish_stable_vector(
        &mut projection_fourth_sum,
        &projection_fourth_correction,
        interrupt,
        "generic_jla_observation_moments",
    )?;
    drop(first_correction);
    drop(third_correction);
    drop(projection_square_correction);
    drop(projection_fourth_correction);
    Ok((
        ObservationMoments {
            projection_square_sum,
            projection_fourth_sum,
        },
        ObservationCorrelations {
            row_offset,
            first: first_correlation,
            third: third_correlation,
        },
        maximum_relres,
    ))
}

#[derive(Clone, Debug)]
struct DeletedAdjustment {
    values: Vec<f64>,
    maximum_leverage: f64,
    maximum_relres: f64,
}

fn observation_deleted_adjustment(
    problem: &CompressedProblem,
    _classes: &[ObservationClass],
    moments: &ObservationMoments,
    correlations: &ObservationCorrelations,
    residual: &[f64],
    control_leverage: &[f64],
    row_order: &[usize],
    options: GenericJlaOptions,
    interrupt: &mut dyn InterruptCheck,
) -> Result<DeletedAdjustment> {
    let rows = problem.outcome.len();
    if control_leverage.len() != rows {
        return Err(BackendError::invariant(
            "generic_jla_observation_adjustment",
            "control leverage has the wrong length",
        ));
    }
    let probes = f64::from(options.probes);
    let mut values = zeroed_f64_with_interrupt(
        rows,
        "observation deleted adjustments",
        interrupt,
        "generic_jla_observation_adjustment_allocate",
    )?;
    let mut maximum_leverage = 0.0_f64;
    for (position, &row) in row_order.iter().enumerate() {
        checkpoint_chunk(interrupt, position, "generic_jla_observation_adjustment")?;
        let mut inverse_sum = StableAccumulator::default();
        for (copy, physical) in
            (correlations.row_offset[row]..correlations.row_offset[row + 1]).enumerate()
        {
            checkpoint_chunk(interrupt, copy, "generic_jla_observation_adjustment_copy")?;
            let p_first = moments.projection_square_sum[row];
            let p_second = moments.projection_fourth_sum[row];
            let first = correlations.first[physical];
            let third = correlations.third[physical];
            let raw = FiveMoments::from_totals(
                p_first,
                probes + p_first - 2.0 * first,
                p_second,
                probes + 6.0 * p_first + p_second - 4.0 * first - 4.0 * third,
                p_first + p_second - 2.0 * third,
            );
            let finite = raw.finite(probes, options, physical)?;
            let total_residual = finite.residual - control_leverage[row];
            if !total_residual.is_finite() || total_residual <= options.block_tolerance {
                return Err(BackendError::new(
                    ErrorCode::NonestimableDeletion,
                    "generic_jla_observation_adjustment",
                    format!("estimated residual leverage is nonpositive for row {row}"),
                ));
            }
            let reciprocal = total_residual.recip();
            let inverse = reciprocal + finite.bias * reciprocal.powi(2)
                - finite.variance * reciprocal.powi(3);
            if !inverse.is_finite() || inverse <= 0.0 {
                return Err(BackendError::new(
                    ErrorCode::BlockInverseFailed,
                    "generic_jla_observation_adjustment",
                    "finite-projection observation inverse is nonpositive",
                ));
            }
            inverse_sum.add(inverse);
            maximum_leverage = maximum_leverage.max(finite.projection + control_leverage[row]);
        }
        values[row] = residual[row] * inverse_sum.finish() / problem.frequency[row] as f64;
        if !values[row].is_finite() {
            return Err(nonfinite("observation deleted residual is nonfinite"));
        }
    }
    Ok(DeletedAdjustment {
        values,
        maximum_leverage,
        maximum_relres: 0.0,
    })
}

fn match_deleted_adjustment(
    problem: &CompressedProblem,
    plan: &MatchPlan,
    moments: &[FiveMoments],
    residual: &[f64],
    geometry: Option<&ControlGeometry>,
    options: GenericJlaOptions,
    interrupt: &mut dyn InterruptCheck,
) -> Result<DeletedAdjustment> {
    let q = geometry.map_or(0, |value| value.controls);
    let rank = 1 + q;
    let probes = f64::from(options.probes);
    let mut values = repeated(
        problem.outcome.len(),
        f64::NAN,
        "match deleted adjustments",
        interrupt,
        "generic_jla_match_adjustment_allocate",
    )?;
    let mut maximum_leverage = 0.0_f64;
    let mut maximum_relres = 0.0_f64;
    for group in 0..plan.rows.len() {
        interrupt.checkpoint("generic_jla_match_adjustment")?;
        let finite = moments[group].finite(probes, options, group)?;
        let rows = &plan.rows[group];
        let width = rows.len();
        let frequency = plan.physical_count[group] as f64;
        let mut common = zeroed_f64_with_interrupt(
            width,
            "match common direction",
            interrupt,
            "generic_jla_match_adjustment_allocate",
        )?;
        let mut low_rank = zeroed_f64_with_interrupt(
            checked_matrix_length(width, rank, "match low-rank matrix")?,
            "match low-rank matrix",
            interrupt,
            "generic_jla_match_adjustment_allocate",
        )?;
        let mut rhs = zeroed_f64_with_interrupt(
            checked_matrix_length(width, 2, "match maker RHS")?,
            "match maker RHS",
            interrupt,
            "generic_jla_match_adjustment_allocate",
        )?;
        let mut work = 0_usize;
        for (local, &row) in rows.iter().enumerate() {
            checkpoint_chunk(interrupt, local, "generic_jla_match_low_rank")?;
            let root = (problem.frequency[row] as f64).sqrt();
            common[local] = root / frequency.sqrt();
            low_rank[local * rank] = finite.projection.sqrt() * common[local];
            rhs[local] = root * residual[row];
            rhs[width + local] = common[local];
            if let Some(geometry) = geometry {
                for output in 0..q {
                    let mut value = StableAccumulator::default();
                    for input in 0..q {
                        checkpoint_chunk(interrupt, work, "generic_jla_match_low_rank")?;
                        work = work.saturating_add(1);
                        value.add(
                            geometry.residualized[input * problem.outcome.len() + row]
                                * geometry.factor[input * q + output],
                        );
                    }
                    low_rank[local * rank + 1 + output] = root * value.finish();
                }
            }
        }
        let action = maker_actions(&low_rank, width, rank, &rhs, 2, options, interrupt)?;
        maximum_leverage = maximum_leverage.max(action.maximum_leverage);
        maximum_relres = maximum_relres.max(action.relres);
        let transformed = &action.values[..width];
        let inverse_common = &action.values[width..];
        let common_transformed = dot(&common, transformed, interrupt, "generic_jla_match_dot")?;
        let common_inverse = dot(&common, inverse_common, interrupt, "generic_jla_match_dot")?;
        for (local, &row) in rows.iter().enumerate() {
            values[row] = transformed[local]
                + finite.bias * inverse_common[local] * common_transformed
                - finite.variance * inverse_common[local] * common_inverse * common_transformed;
        }
    }
    for (row, value) in values.iter().enumerate() {
        checkpoint_chunk(interrupt, row, "generic_jla_match_adjustment_validate")?;
        if !value.is_finite() {
            return Err(nonfinite(
                "match deleted residual is nonfinite or incomplete",
            ));
        }
    }
    Ok(DeletedAdjustment {
        values,
        maximum_leverage,
        maximum_relres,
    })
}

#[derive(Clone, Debug)]
struct MakerAction {
    values: Vec<f64>,
    maximum_leverage: f64,
    relres: f64,
}

#[derive(Clone, Debug)]
struct TargetCorrection {
    mean: VarianceComponents,
    mcse: VarianceComponents,
    maximum_solve_relres: f64,
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn target_correction(
    problem: &CompressedProblem,
    plan: &TargetPlan,
    working_y: &[f64],
    deleted_adjusted: &[f64],
    solver: &PreparedModelSolver<'_>,
    row_order: &[usize],
    match_rows: Option<&[Vec<usize>]>,
    rng: CounterRng,
    options: GenericJlaOptions,
    rhs_receipts: &mut Vec<GenericJlaRhsReceipt>,
    interrupt: &mut dyn InterruptCheck,
) -> Result<TargetCorrection> {
    let workers = problem.workers();
    let firms = problem.firms();
    let q = solver.operator().controls();
    let probes = options.probes as usize;
    let mut draws = repeated(
        probes,
        VarianceComponents::default(),
        "target-probe draws",
        interrupt,
        "generic_jla_target_allocate",
    )?;
    let mut maximum_solve_relres = 0.0_f64;
    for first in (0..probes).step_by(options.target_batch_width) {
        interrupt.checkpoint("generic_jla_target_batch")?;
        let width = options.target_batch_width.min(probes - first);
        let mut atoms = repeated(
            checked_matrix_length(plan.cell.len(), width, "target atoms")?,
            0_i64,
            "target atoms",
            interrupt,
            "generic_jla_target_allocate",
        )?;
        rng.fill_rademacher_sums_with_interrupt(
            ProbeDomain::Target,
            first as u64,
            width,
            &plan.entity,
            &plan.physical_count,
            &mut atoms,
            interrupt,
        )?;
        let columns = width
            .checked_mul(2)
            .ok_or_else(|| resource("target RHS column count overflow"))?;
        let mut worker_rhs = zeroed_f64_with_interrupt(
            checked_matrix_length(workers, columns, "target worker RHS")?,
            "target worker RHS",
            interrupt,
            "generic_jla_target_allocate",
        )?;
        let mut firm_rhs = zeroed_f64_with_interrupt(
            checked_matrix_length(firms, columns, "target firm RHS")?,
            "target firm RHS",
            interrupt,
            "generic_jla_target_allocate",
        )?;
        let mut worker_correction = zeroed_f64_with_interrupt(
            worker_rhs.len(),
            "target worker RHS corrections",
            interrupt,
            "generic_jla_target_allocate",
        )?;
        let mut firm_correction = zeroed_f64_with_interrupt(
            firm_rhs.len(),
            "target firm RHS corrections",
            interrupt,
            "generic_jla_target_allocate",
        )?;
        let control_rhs = zeroed_f64_with_interrupt(
            checked_matrix_length(q, columns, "target control RHS")?,
            "target control RHS",
            interrupt,
            "generic_jla_target_allocate",
        )?;
        for local in 0..width {
            let mut total_direction = StableAccumulator::default();
            let mut reference_scale = StableAccumulator::default();
            let worker_column = 2 * local;
            let firm_column = worker_column + 1;
            for stratum in 0..plan.cell.len() {
                checkpoint_chunk(
                    interrupt,
                    local * plan.cell.len() + stratum,
                    "generic_jla_target_direction",
                )?;
                let direction = (plan.per_copy_mass[stratum] / problem.target_total).sqrt()
                    * atoms[local * plan.cell.len() + stratum] as f64;
                if !direction.is_finite() {
                    return Err(BackendError::new(
                        ErrorCode::TargetCenteringFailed,
                        "generic_jla_target_direction",
                        "target direction is nonfinite",
                    ));
                }
                total_direction.add(direction);
                reference_scale.add(direction.abs());
                let cell = plan.cell[stratum] as usize;
                stable_add_index(
                    &mut worker_rhs,
                    &mut worker_correction,
                    worker_column * workers + problem.cell_worker[cell] as usize,
                    direction,
                );
                stable_add_index(
                    &mut firm_rhs,
                    &mut firm_correction,
                    firm_column * firms + problem.cell_firm[cell] as usize,
                    direction,
                );
            }
            let total_direction = total_direction.finish();
            reference_scale.add(total_direction.abs());
            let reference_scale = reference_scale.finish();
            for stratum in 0..plan.cell.len() {
                checkpoint_chunk(
                    interrupt,
                    local * plan.cell.len() + stratum,
                    "generic_jla_target_centering",
                )?;
                let centered = plan.target_mass[stratum] / problem.target_total * total_direction;
                let cell = plan.cell[stratum] as usize;
                stable_add_index(
                    &mut worker_rhs,
                    &mut worker_correction,
                    worker_column * workers + problem.cell_worker[cell] as usize,
                    -centered,
                );
                stable_add_index(
                    &mut firm_rhs,
                    &mut firm_correction,
                    firm_column * firms + problem.cell_firm[cell] as usize,
                    -centered,
                );
            }
            finish_stable_vector(
                &mut worker_rhs[worker_column * workers..(worker_column + 1) * workers],
                &worker_correction[worker_column * workers..(worker_column + 1) * workers],
                interrupt,
                "generic_jla_target_centering",
            )?;
            finish_stable_vector(
                &mut firm_rhs[firm_column * firms..(firm_column + 1) * firms],
                &firm_correction[firm_column * firms..(firm_column + 1) * firms],
                interrupt,
                "generic_jla_target_centering",
            )?;
            balance_score(
                &mut worker_rhs[worker_column * workers..(worker_column + 1) * workers],
                reference_scale,
                options.rank_tolerance,
                interrupt,
            )?;
            balance_score(
                &mut firm_rhs[firm_column * firms..(firm_column + 1) * firms],
                reference_scale,
                options.rank_tolerance,
                interrupt,
            )?;
        }
        drop(worker_correction);
        drop(firm_correction);
        let solved = solver.solve_batch_with_interrupt(
            &worker_rhs,
            &firm_rhs,
            &control_rhs,
            columns,
            columns,
            interrupt,
        )?;
        let mut worker_projection = zeroed_f64_with_interrupt(
            problem.outcome.len(),
            "worker target projection",
            interrupt,
            "generic_jla_target_allocate",
        )?;
        let mut firm_projection = zeroed_f64_with_interrupt(
            problem.outcome.len(),
            "firm target projection",
            interrupt,
            "generic_jla_target_allocate",
        )?;
        let mut total_projection = zeroed_f64_with_interrupt(
            problem.outcome.len(),
            "total target projection",
            interrupt,
            "generic_jla_target_allocate",
        )?;
        for local in 0..width {
            let worker_solve = &solved.solution[2 * local];
            let firm_solve = &solved.solution[2 * local + 1];
            let probe = Some(
                u32::try_from(first + local)
                    .map_err(|_| resource("target probe index is not representable as u32"))?,
            );
            rhs_receipts.push(rhs_receipt(
                GenericJlaRhsPhase::Target,
                GenericJlaRhsSide::Worker,
                probe,
                worker_solve,
            ));
            rhs_receipts.push(rhs_receipt(
                GenericJlaRhsPhase::Target,
                GenericJlaRhsSide::Firm,
                probe,
                firm_solve,
            ));
            maximum_solve_relres = maximum_solve_relres
                .max(worker_solve.residual.relative_norm)
                .max(firm_solve.residual.relative_norm);
            solver.operator().predict_into_with_interrupt(
                &worker_solve.coefficients.worker,
                &worker_solve.coefficients.firm,
                &worker_solve.coefficients.control,
                &mut worker_projection,
                interrupt,
            )?;
            solver.operator().predict_into_with_interrupt(
                &firm_solve.coefficients.worker,
                &firm_solve.coefficients.firm,
                &firm_solve.coefficients.control,
                &mut firm_projection,
                interrupt,
            )?;
            let worker = target_contraction(
                problem,
                working_y,
                deleted_adjusted,
                &worker_projection,
                options.deletion,
                row_order,
                match_rows,
                interrupt,
            )?;
            let firm = target_contraction(
                problem,
                working_y,
                deleted_adjusted,
                &firm_projection,
                options.deletion,
                row_order,
                match_rows,
                interrupt,
            )?;
            for (position, &row) in row_order.iter().enumerate() {
                checkpoint_chunk(interrupt, position, "generic_jla_target_total_projection")?;
                total_projection[row] = worker_projection[row] + firm_projection[row];
            }
            let total = target_contraction(
                problem,
                working_y,
                deleted_adjusted,
                &total_projection,
                options.deletion,
                row_order,
                match_rows,
                interrupt,
            )?;
            let draw = VarianceComponents {
                worker,
                firm,
                covariance: 0.5 * (total - worker - firm),
                total,
            };
            draw.verify_accounting(1.0e-11)?;
            draws[first + local] = draw;
        }
    }
    let mean = component_mean(&draws, interrupt)?;
    let mcse = component_mcse(&draws, mean, interrupt)?;
    Ok(TargetCorrection {
        mean,
        mcse,
        maximum_solve_relres,
    })
}

fn target_contraction(
    problem: &CompressedProblem,
    working_y: &[f64],
    deleted_adjusted: &[f64],
    projection: &[f64],
    deletion: DeletionMode,
    row_order: &[usize],
    match_rows: Option<&[Vec<usize>]>,
    interrupt: &mut dyn InterruptCheck,
) -> Result<f64> {
    let rows = problem.outcome.len();
    if working_y.len() != rows || deleted_adjusted.len() != rows || projection.len() != rows {
        return Err(BackendError::invariant(
            "generic_jla_target_contraction",
            "target contraction dimensions disagree",
        ));
    }
    let mut output = StableAccumulator::default();
    match deletion {
        DeletionMode::Observation => {
            for (position, &row) in row_order.iter().enumerate() {
                checkpoint_chunk(interrupt, position, "generic_jla_target_observation")?;
                output.add(
                    problem.frequency[row] as f64
                        * working_y[row]
                        * deleted_adjusted[row]
                        * projection[row]
                        * projection[row],
                );
            }
        }
        DeletionMode::Match => {
            let match_rows = match_rows.ok_or_else(|| {
                BackendError::invariant(
                    "generic_jla_target_contraction",
                    "match target contraction has no canonical block rows",
                )
            })?;
            for (group, rows) in match_rows.iter().enumerate() {
                checkpoint_chunk(interrupt, group, "generic_jla_target_match")?;
                let mut first = StableAccumulator::default();
                let mut second = StableAccumulator::default();
                for (local, &row) in rows.iter().enumerate() {
                    checkpoint_chunk(interrupt, local, "generic_jla_target_match_rows")?;
                    let frequency = problem.frequency[row] as f64;
                    first.add(frequency * working_y[row] * projection[row]);
                    second.add(frequency.sqrt() * deleted_adjusted[row] * projection[row]);
                }
                output.add(first.finish() * second.finish());
            }
        }
    }
    let output = output.finish();
    if !output.is_finite() {
        return Err(nonfinite("target-probe contraction is nonfinite"));
    }
    Ok(output)
}

fn balance_score(
    score: &mut [f64],
    reference: f64,
    tolerance: f64,
    interrupt: &mut dyn InterruptCheck,
) -> Result<()> {
    let mut sum = StableAccumulator::default();
    for (index, &value) in score.iter().enumerate() {
        checkpoint_chunk(interrupt, index, "generic_jla_target_balance")?;
        sum.add(value);
    }
    let sum = sum.finish();
    let gate = (100.0 * tolerance).max(1.0e-12) * (1.0 + reference);
    if !sum.is_finite() || sum.abs() > gate || score.is_empty() {
        return Err(BackendError::new(
            ErrorCode::TargetCenteringFailed,
            "generic_jla_target_centering",
            "target score compatibility repair exceeded roundoff",
        ));
    }
    let last = score.len() - 1;
    score[last] -= sum;
    Ok(())
}

fn component_mean(
    draws: &[VarianceComponents],
    interrupt: &mut dyn InterruptCheck,
) -> Result<VarianceComponents> {
    let denominator = draws.len() as f64;
    let mut worker = StableAccumulator::default();
    let mut firm = StableAccumulator::default();
    let mut covariance = StableAccumulator::default();
    let mut total = StableAccumulator::default();
    for (index, draw) in draws.iter().enumerate() {
        checkpoint_chunk(interrupt, index, "generic_jla_target_mean")?;
        worker.add(draw.worker);
        firm.add(draw.firm);
        covariance.add(draw.covariance);
        total.add(draw.total);
    }
    let output = VarianceComponents {
        worker: worker.finish() / denominator,
        firm: firm.finish() / denominator,
        covariance: covariance.finish() / denominator,
        total: total.finish() / denominator,
    };
    output.verify_accounting(1.0e-11)?;
    Ok(output)
}

fn component_mcse(
    draws: &[VarianceComponents],
    mean: VarianceComponents,
    interrupt: &mut dyn InterruptCheck,
) -> Result<VarianceComponents> {
    if draws.len() < 2 {
        return Err(invalid("at least two target probes are required for MCSE"));
    }
    let mut worker = StableAccumulator::default();
    let mut firm = StableAccumulator::default();
    let mut covariance = StableAccumulator::default();
    let mut total = StableAccumulator::default();
    for (index, draw) in draws.iter().enumerate() {
        checkpoint_chunk(interrupt, index, "generic_jla_target_mcse")?;
        worker.add((draw.worker - mean.worker).powi(2));
        firm.add((draw.firm - mean.firm).powi(2));
        covariance.add((draw.covariance - mean.covariance).powi(2));
        total.add((draw.total - mean.total).powi(2));
    }
    let draw_count = draws.len() as f64;
    let scale = (draw_count - 1.0) * draw_count;
    let output = VarianceComponents {
        worker: (worker.finish() / scale).sqrt(),
        firm: (firm.finish() / scale).sqrt(),
        covariance: (covariance.finish() / scale).sqrt(),
        total: (total.finish() / scale).sqrt(),
    };
    if [output.worker, output.firm, output.covariance, output.total]
        .iter()
        .any(|value| !value.is_finite())
    {
        return Err(nonfinite("target-probe MCSE is nonfinite"));
    }
    Ok(output)
}

fn subtract_components(
    plugin: VarianceComponents,
    correction: VarianceComponents,
) -> Result<VarianceComponents> {
    let output = VarianceComponents {
        worker: plugin.worker - correction.worker,
        firm: plugin.firm - correction.firm,
        covariance: plugin.covariance - correction.covariance,
        total: plugin.total - correction.total,
    };
    if [output.worker, output.firm, output.covariance, output.total]
        .iter()
        .any(|value| !value.is_finite())
    {
        return Err(BackendError::new(
            ErrorCode::NonfiniteCorrectedTarget,
            "generic_jla_finalize",
            "corrected target is nonfinite",
        ));
    }
    Ok(output)
}

#[allow(clippy::too_many_arguments)]
fn maker_actions(
    low_rank: &[f64],
    width: usize,
    rank: usize,
    rhs: &[f64],
    columns: usize,
    options: GenericJlaOptions,
    interrupt: &mut dyn InterruptCheck,
) -> Result<MakerAction> {
    let expected_low_rank = checked_matrix_length(width, rank, "match low-rank maker")?;
    let expected_rhs = checked_matrix_length(width, columns, "match maker RHS")?;
    if low_rank.len() != expected_low_rank || rhs.len() != expected_rhs {
        return Err(BackendError::invariant(
            "generic_jla_maker",
            "low-rank maker dimensions disagree",
        ));
    }
    let mut work = 0_usize;
    let (values, minimum_maker, inverse_relres) = if width <= rank {
        let mut maker = zeroed_f64_with_interrupt(
            checked_matrix_length(width, width, "match maker")?,
            "match maker",
            interrupt,
            "generic_jla_maker_allocate",
        )?;
        for row in 0..width {
            for column in 0..width {
                let mut value = StableAccumulator::default();
                value.add(usize::from(row == column) as f64);
                for inner in 0..rank {
                    checkpoint_chunk(interrupt, work, "generic_jla_maker_assembly")?;
                    work = work.saturating_add(1);
                    value.add(-low_rank[row * rank + inner] * low_rank[column * rank + inner]);
                }
                maker[row * width + column] = value.finish();
            }
        }
        let extremes =
            symmetric_eigen_extremes(&maker, width, interrupt, "generic_jla_maker_spectrum")?;
        if !extremes.largest_upper.is_finite() || extremes.smallest_lower <= options.block_tolerance
        {
            return Err(nonestimable("a match residual block is singular"));
        }
        let inverse = invert_scaled_spd(
            &maker,
            width,
            options.rank_tolerance,
            interrupt,
            "generic_jla_maker_inverse",
        )
        .map_err(|error| {
            preserve_break(
                error,
                ErrorCode::BlockInverseFailed,
                "a match residual solve failed",
            )
        })?;
        let mut output = zeroed_f64_with_interrupt(
            checked_matrix_length(width, columns, "match maker output")?,
            "match maker output",
            interrupt,
            "generic_jla_maker_allocate",
        )?;
        for column in 0..columns {
            for row in 0..width {
                let mut value = StableAccumulator::default();
                for inner in 0..width {
                    checkpoint_chunk(interrupt, work, "generic_jla_maker_action")?;
                    work = work.saturating_add(1);
                    value.add(inverse.inverse[row * width + inner] * rhs[column * width + inner]);
                }
                output[column * width + row] = value.finish();
            }
        }
        (
            output,
            extremes.smallest_lower,
            inverse.relres.max(inverse.original_relres),
        )
    } else {
        let mut reduced = zeroed_f64_with_interrupt(
            checked_matrix_length(rank, rank, "reduced match maker")?,
            "reduced match maker",
            interrupt,
            "generic_jla_maker_allocate",
        )?;
        for row in 0..rank {
            for column in 0..rank {
                let mut value = StableAccumulator::default();
                value.add(usize::from(row == column) as f64);
                for inner in 0..width {
                    checkpoint_chunk(interrupt, work, "generic_jla_reduced_maker_assembly")?;
                    work = work.saturating_add(1);
                    value.add(-low_rank[inner * rank + row] * low_rank[inner * rank + column]);
                }
                reduced[row * rank + column] = value.finish();
            }
        }
        let extremes = symmetric_eigen_extremes(
            &reduced,
            rank,
            interrupt,
            "generic_jla_reduced_maker_spectrum",
        )?;
        if !extremes.largest_upper.is_finite() || extremes.smallest_lower <= options.block_tolerance
        {
            return Err(nonestimable("a reduced match residual block is singular"));
        }
        let inverse = invert_scaled_spd(
            &reduced,
            rank,
            options.rank_tolerance,
            interrupt,
            "generic_jla_reduced_maker_inverse",
        )
        .map_err(|error| {
            preserve_break(
                error,
                ErrorCode::BlockInverseFailed,
                "a reduced match residual solve failed",
            )
        })?;
        let mut output = copy_f64(rhs, "reduced match maker output", interrupt)?;
        for column in 0..columns {
            let mut projected = zeroed_f64_with_interrupt(
                rank,
                "reduced maker projection",
                interrupt,
                "generic_jla_maker_allocate",
            )?;
            for inner in 0..rank {
                let mut value = StableAccumulator::default();
                for row in 0..width {
                    checkpoint_chunk(interrupt, work, "generic_jla_reduced_maker_action")?;
                    work = work.saturating_add(1);
                    value.add(low_rank[row * rank + inner] * rhs[column * width + row]);
                }
                projected[inner] = value.finish();
            }
            let mut solved = zeroed_f64_with_interrupt(
                rank,
                "reduced maker solve",
                interrupt,
                "generic_jla_maker_allocate",
            )?;
            for row in 0..rank {
                let mut value = StableAccumulator::default();
                for inner in 0..rank {
                    checkpoint_chunk(interrupt, work, "generic_jla_reduced_maker_action")?;
                    work = work.saturating_add(1);
                    value.add(inverse.inverse[row * rank + inner] * projected[inner]);
                }
                solved[row] = value.finish();
            }
            for row in 0..width {
                let mut value = StableAccumulator::default();
                value.add(output[column * width + row]);
                for inner in 0..rank {
                    checkpoint_chunk(interrupt, work, "generic_jla_reduced_maker_action")?;
                    work = work.saturating_add(1);
                    value.add(low_rank[row * rank + inner] * solved[inner]);
                }
                output[column * width + row] = value.finish();
            }
        }
        (
            output,
            extremes.smallest_lower.min(1.0),
            inverse.relres.max(inverse.original_relres),
        )
    };
    let mut residual_square = StableAccumulator::default();
    let mut rhs_square = StableAccumulator::default();
    for column in 0..columns {
        for row in 0..width {
            let mut value = StableAccumulator::default();
            value.add(values[column * width + row] - rhs[column * width + row]);
            for inner in 0..width {
                let mut low_rank_action = StableAccumulator::default();
                for k in 0..rank {
                    checkpoint_chunk(interrupt, work, "generic_jla_maker_complete_residual")?;
                    work = work.saturating_add(1);
                    low_rank_action.add(low_rank[row * rank + k] * low_rank[inner * rank + k]);
                }
                value.add(-low_rank_action.finish() * values[column * width + inner]);
            }
            let value = value.finish();
            residual_square.add(value * value);
            rhs_square.add(rhs[column * width + row].powi(2));
        }
    }
    let residual_square = residual_square.finish();
    let rhs_square = rhs_square.finish();
    let complete_relres = if rhs_square == 0.0 {
        residual_square.sqrt()
    } else {
        (residual_square / rhs_square).sqrt()
    };
    let relres = complete_relres.max(inverse_relres);
    if !relres.is_finite() || relres > (100.0 * options.rank_tolerance).max(1.0e-10) {
        return Err(BackendError::new(
            ErrorCode::BlockInverseFailed,
            "generic_jla_maker",
            "match residual solve failed its complete residual gate",
        ));
    }
    Ok(MakerAction {
        values,
        maximum_leverage: 1.0 - minimum_maker,
        relres,
    })
}

fn repeated<T: Clone>(
    length: usize,
    value: T,
    label: &str,
    interrupt: &mut dyn InterruptCheck,
    phase: &'static str,
) -> Result<Vec<T>> {
    let mut output = Vec::new();
    reserve_exact(&mut output, length, label)?;
    for index in 0..length {
        checkpoint_chunk(interrupt, index, phase)?;
        output.push(value.clone());
    }
    Ok(output)
}

fn zeroed_matrix(
    columns: usize,
    rows: usize,
    label: &str,
    interrupt: &mut dyn InterruptCheck,
    phase: &'static str,
) -> Result<Vec<Vec<f64>>> {
    let _ = checked_matrix_length(columns, rows, label)?;
    let mut output = Vec::new();
    reserve_exact(&mut output, columns, label)?;
    for column in 0..columns {
        checkpoint_chunk(interrupt, column, phase)?;
        output.push(zeroed_f64_with_interrupt(rows, label, interrupt, phase)?);
    }
    Ok(output)
}

fn copy_f64(values: &[f64], label: &str, interrupt: &mut dyn InterruptCheck) -> Result<Vec<f64>> {
    let mut output = Vec::new();
    reserve_exact(&mut output, values.len(), label)?;
    for (index, &value) in values.iter().enumerate() {
        checkpoint_chunk(interrupt, index, "generic_jla_copy")?;
        output.push(value);
    }
    Ok(output)
}

fn index_vector(
    length: usize,
    interrupt: &mut dyn InterruptCheck,
    phase: &'static str,
) -> Result<Vec<usize>> {
    let mut output = Vec::new();
    reserve_exact(&mut output, length, "generic JLA index vector")?;
    for index in 0..length {
        checkpoint_chunk(interrupt, index, phase)?;
        output.push(index);
    }
    Ok(output)
}

fn finish_accumulators(
    values: Vec<StableAccumulator>,
    interrupt: &mut dyn InterruptCheck,
    phase: &'static str,
) -> Result<Vec<f64>> {
    let mut output = Vec::new();
    reserve_exact(&mut output, values.len(), "generic JLA accumulated values")?;
    for (index, value) in values.into_iter().enumerate() {
        checkpoint_chunk(interrupt, index, phase)?;
        output.push(value.finish());
    }
    Ok(output)
}

fn group_rows(
    problem: &CompressedProblem,
    group: usize,
    interrupt: &mut dyn InterruptCheck,
    phase: &'static str,
) -> Result<Vec<usize>> {
    let begin = usize::try_from(problem.deletion_index.ptr[group])
        .map_err(|_| resource("deletion-group offset is not addressable"))?;
    let end = usize::try_from(problem.deletion_index.ptr[group + 1])
        .map_err(|_| resource("deletion-group offset is not addressable"))?;
    let length = end
        .checked_sub(begin)
        .ok_or_else(|| BackendError::invariant(phase, "deletion-group pointers are reversed"))?;
    let mut output = Vec::new();
    reserve_exact(&mut output, length, "generic JLA deletion-group rows")?;
    for (local, &row) in problem.deletion_index.items[begin..end].iter().enumerate() {
        checkpoint_chunk(interrupt, local, phase)?;
        output.push(
            usize::try_from(row).map_err(|_| resource("deletion-group row is not addressable"))?,
        );
    }
    Ok(output)
}

fn stable_sum_indices(
    indices: &[usize],
    values: &[f64],
    interrupt: &mut dyn InterruptCheck,
    phase: &'static str,
) -> Result<f64> {
    let mut sum = StableAccumulator::default();
    for (position, &index) in indices.iter().enumerate() {
        checkpoint_chunk(interrupt, position, phase)?;
        let value = values
            .get(index)
            .ok_or_else(|| BackendError::invariant(phase, "summation index is out of range"))?;
        sum.add(*value);
    }
    Ok(sum.finish())
}

fn memory_facts(
    problem: &CompressedProblem,
    interrupt: &mut dyn InterruptCheck,
) -> Result<MemoryFacts> {
    let mut maximum_deletion_block = 0_u64;
    for (group, pair) in problem.deletion_index.ptr.windows(2).enumerate() {
        checkpoint_chunk(interrupt, group, "generic_jla_memory_blocks")?;
        let width = pair[1]
            .checked_sub(pair[0])
            .ok_or_else(|| resource("deletion-group pointers are reversed"))?;
        maximum_deletion_block = maximum_deletion_block.max(width);
    }
    Ok(MemoryFacts {
        maximum_deletion_block,
    })
}

fn route_memory_forecast(
    problem: &CompressedProblem,
    solver: &PreparedModelSolver<'_>,
    controls: usize,
) -> Result<RouteMemory> {
    let Some(cmg) = solver.receipt().cmg.as_ref() else {
        return Ok(RouteMemory::default());
    };
    let shared_cmg_persistent = checked_sum(&[
        cmg.structural_bytes,
        cmg.workspace_bytes,
        cmg.preconditioner_bytes,
        cmg.dense_factor_bytes,
    ])?;
    let firms = u64::try_from(problem.firms())
        .map_err(|_| resource("firm count is not representable for CMG memory"))?;
    let controls = u64::try_from(controls)
        .map_err(|_| resource("control count is not representable for CMG memory"))?;
    let rows = u64::try_from(problem.outcome.len())
        .map_err(|_| resource("row count is not representable for CMG memory"))?;
    let cells = u64::try_from(problem.cells())
        .map_err(|_| resource("cell count is not representable for CMG memory"))?;
    let workers = u64::try_from(problem.workers())
        .map_err(|_| resource("worker count is not representable for CMG memory"))?;
    let f64_bytes = u64::try_from(core::mem::size_of::<f64>())
        .map_err(|_| resource("f64 byte size is not representable"))?;
    let u32_bytes = u64::try_from(core::mem::size_of::<u32>())
        .map_err(|_| resource("u32 byte size is not representable"))?;
    let u64_bytes = u64::try_from(core::mem::size_of::<u64>())
        .map_err(|_| resource("u64 byte size is not representable"))?;
    let usize_bytes = u64::try_from(core::mem::size_of::<usize>())
        .map_err(|_| resource("usize byte size is not representable"))?;
    let fine_vertices = u64::try_from(cmg.fine_vertices)
        .map_err(|_| resource("CMG vertex count is not representable"))?;
    let fine_edges = u64::try_from(cmg.fine_edges)
        .map_err(|_| resource("CMG edge count is not representable"))?;
    let auxiliary_vertices = fine_vertices
        .checked_sub(firms)
        .ok_or_else(|| resource("CMG fine vertex count is below the firm count"))?;
    let cross_bytes = checked_product(&[firms, controls, f64_bytes], "CMG F by Q cross block")?;
    let inverse_bytes = checked_product(&[controls, controls, f64_bytes], "CMG Q-square inverse")?;
    let block_workspace_bytes = if controls == 0 {
        0
    } else {
        checked_sum(&[
            checked_product(&[firms, f64_bytes, 2], "CMG firm block workspace")?,
            checked_product(&[controls, f64_bytes, 2], "CMG control block workspace")?,
        ])?
    };
    let full_control_block_persistent =
        checked_sum(&[cross_bytes, inverse_bytes, block_workspace_bytes])?;
    // `model_cmg_problem` reserves all three aggregated cell columns at N,
    // even when only C unique cells are ultimately pushed. Group items use C.
    let cmg_aggregated_cell_capacity = checked_sum(&[
        checked_product(&[rows, u32_bytes, 2], "CMG aggregated cell identifiers")?,
        checked_product(&[rows, f64_bytes], "CMG aggregated cell weights")?,
    ])?;
    let cmg_group_index = checked_sum(&[
        checked_product(&[cells, u32_bytes, 2], "CMG worker and firm group items")?,
        checked_product(
            &[checked_sum(&[workers, firms, 2])?, u64_bytes],
            "CMG group pointers",
        )?,
    ])?;
    let cmg_hybrid_graph = checked_sum(&[
        checked_product(&[fine_vertices, 48], "CMG hybrid vertices")?,
        checked_product(&[fine_edges, 40], "CMG hybrid edges and incidences")?,
        checked_product(&[auxiliary_vertices, u32_bytes], "CMG hybrid auxiliaries")?,
    ])?;
    let initial_sort_peak = checked_product(&[rows, usize_bytes, 2], "CMG row sort")?;
    let grouped_view_peak = checked_sum(&[
        checked_product(&[rows, usize_bytes], "CMG retained row order")?,
        cmg_aggregated_cell_capacity,
        cmg_group_index,
        checked_product(&[cells, usize_bytes, 2], "CMG grouped cell sort")?,
    ])?;
    let problem_view_bytes = checked_sum(&[cmg_aggregated_cell_capacity, cmg_group_index])?;
    // The compressed view remains live while the hybrid and hierarchy are
    // built. Dense terminal construction holds its assembled matrix and
    // Cholesky lower simultaneously. The hybrid is explicitly dropped before
    // hierarchy workspaces and the two full preconditioner vectors are
    // allocated.
    let dense_terminal_build = checked_product(
        &[cmg.dense_factor_bytes, 2],
        "CMG dense terminal matrix and lower",
    )?;
    let hierarchy_build_peak = checked_sum(&[
        problem_view_bytes,
        cmg_hybrid_graph,
        cmg.structural_bytes,
        dense_terminal_build,
    ])?;
    let completed_preconditioner_peak = checked_sum(&[problem_view_bytes, shared_cmg_persistent])?;
    let control_cross_temporary = if controls == 0 {
        0
    } else {
        checked_sum(&[
            checked_product(
                &[checked_sum(&[firms, controls])?, f64_bytes, 2],
                "CMG cross input and action",
            )?,
            checked_product(&[workers, f64_bytes], "CMG cross worker workspace")?,
            checked_product(&[controls, controls, f64_bytes, 8], "CMG Schur dense setup")?,
        ])?
    };
    let control_block_build_peak =
        checked_sum(&[shared_cmg_persistent, cross_bytes, control_cross_temporary])?;
    let setup_transient = initial_sort_peak
        .max(grouped_view_peak)
        .max(hierarchy_build_peak)
        .max(completed_preconditioner_peak)
        .max(control_block_build_peak);
    Ok(RouteMemory {
        shared_cmg_persistent,
        full_control_block_persistent,
        setup_transient,
        cmg_preconditioner_workspace: cmg.preconditioner_bytes,
        cmg_batch_workspace_per_column: cmg.batch_workspace_bytes(1)?,
        cmg_aggregated_cell_capacity,
        cmg_group_index,
        cmg_hybrid_graph,
    })
}

fn plan_generic_jla_batches(
    problem: &CompressedProblem,
    options: GenericJlaOptions,
    full_parameters: usize,
    leverage_request: BatchRequest,
    target_request: BatchRequest,
    route_memory: RouteMemory,
    memory_facts: MemoryFacts,
    interrupt: &mut dyn InterruptCheck,
) -> Result<GenericJlaBatchExecutionReceipt> {
    let width_one = memory_forecast(
        problem,
        options,
        full_parameters,
        route_memory,
        memory_facts,
        1,
        1,
    )?;
    let non_batched_peak_bytes = width_one
        .canonicalization
        .max(width_one.setup)
        .max(width_one.fit)
        .max(width_one.geometry)
        .max(width_one.maker)
        .max(width_one.result);
    let probes = usize::try_from(options.probes)
        .map_err(|_| resource("probe count is not addressable by the batch planner"))?;
    let active_request = |request: BatchRequest| match request {
        BatchRequest::Auto => BatchRequest::Auto,
        BatchRequest::Explicit(width) => BatchRequest::Explicit(width.min(probes)),
    };
    let leverage_active_request = active_request(leverage_request);
    let target_active_request = active_request(target_request);
    let explicit_cap = [leverage_active_request, target_active_request]
        .into_iter()
        .filter_map(|request| match request {
            BatchRequest::Explicit(width) => Some(width),
            BatchRequest::Auto => None,
        })
        .max()
        .unwrap_or(1);
    let leverage_forecasts = precompute_batch_forecasts(
        problem,
        options,
        full_parameters,
        route_memory,
        memory_facts,
        leverage_active_request,
        true,
        interrupt,
    )?;
    let target_forecasts = precompute_batch_forecasts(
        problem,
        options,
        full_parameters,
        route_memory,
        memory_facts,
        target_active_request,
        false,
        interrupt,
    )?;
    let planner = plan_batches_with_forecasts(
        leverage_active_request,
        target_active_request,
        BatchPlannerCaps {
            probes,
            declared_threads: 1,
            columns_per_thread: GENERIC_JLA_ROUTE_BATCH_WIDTH_CAP_V1.max(explicit_cap),
            route_width_cap: GENERIC_JLA_ROUTE_BATCH_WIDTH_CAP_V1.max(explicit_cap),
            non_batched_peak_bytes,
            hard_memory_bytes: options.memory_limit_bytes,
        },
        |width| lookup_batch_forecast(&leverage_forecasts, width),
        |width| lookup_batch_forecast(&target_forecasts, width),
    )
    .map_err(|error| {
        if error.code == ErrorCode::ResourceLimit && error.phase == "batch_plan" {
            BackendError::new(
                ErrorCode::ResourceLimit,
                "generic_jla_memory",
                error.to_string(),
            )
        } else {
            error
        }
    })?;
    Ok(GenericJlaBatchExecutionReceipt {
        leverage_requested: leverage_request,
        leverage_active_width: planner.leverage.selected_width,
        target_requested: target_request,
        target_active_width: planner.target.selected_width,
        automatic_ladder_cap: GENERIC_JLA_ROUTE_BATCH_WIDTH_CAP_V1,
        plan: planner,
    })
}

#[allow(clippy::too_many_arguments)]
fn precompute_batch_forecasts(
    problem: &CompressedProblem,
    options: GenericJlaOptions,
    full_parameters: usize,
    route_memory: RouteMemory,
    memory_facts: MemoryFacts,
    request: BatchRequest,
    leverage: bool,
    interrupt: &mut dyn InterruptCheck,
) -> Result<Vec<(usize, u64)>> {
    let probes = usize::try_from(options.probes)
        .map_err(|_| resource("probe count is not addressable for batch forecasts"))?;
    let mut widths = Vec::new();
    reserve_exact(
        &mut widths,
        BATCH_WIDTH_CANDIDATES.len() + 1,
        "generic JLA batch forecast widths",
    )?;
    match request {
        BatchRequest::Auto => {
            for width in BATCH_WIDTH_CANDIDATES {
                if width <= probes {
                    widths.push(width);
                }
            }
        }
        BatchRequest::Explicit(width) => {
            widths.push(1);
            if width > 1 {
                widths.push(width);
            }
        }
    }
    let mut forecasts = Vec::new();
    reserve_exact(&mut forecasts, widths.len(), "generic JLA batch forecasts")?;
    let phase = if leverage {
        "generic_jla_batch_leverage_forecast"
    } else {
        "generic_jla_batch_target_forecast"
    };
    for width in widths {
        interrupt.checkpoint(phase)?;
        let forecast = if leverage {
            memory_forecast(
                problem,
                options,
                full_parameters,
                route_memory,
                memory_facts,
                width,
                1,
            )?
            .leverage
        } else {
            memory_forecast(
                problem,
                options,
                full_parameters,
                route_memory,
                memory_facts,
                1,
                width,
            )?
            .target
        };
        forecasts.push((width, forecast));
    }
    Ok(forecasts)
}

fn lookup_batch_forecast(forecasts: &[(usize, u64)], width: usize) -> Result<u64> {
    forecasts
        .iter()
        .find_map(|&(candidate, bytes)| (candidate == width).then_some(bytes))
        .ok_or_else(|| {
            BackendError::invariant(
                "generic_jla_plan",
                "batch planner requested a width outside the precomputed forecast table",
            )
        })
}

fn admit_memory(forecast: u64, limit: u64) -> Result<()> {
    if forecast > limit {
        return Err(BackendError::new(
            ErrorCode::ResourceLimit,
            "generic_jla_memory",
            format!(
                "generic-JLA peak forecast {forecast} bytes exceeds the declared limit {limit} bytes"
            ),
        ));
    }
    Ok(())
}

fn generic_jla_wall_work(
    problem: &CompressedProblem,
    options: GenericJlaOptions,
    full_parameters: usize,
) -> Result<WallWork> {
    let rows = u64::try_from(problem.outcome.len())
        .map_err(|_| resource("row count is not representable in wall work"))?;
    let parameters = u64::try_from(full_parameters)
        .map_err(|_| resource("parameter count is not representable in wall work"))?;
    let controls = u64::try_from(problem.controls.len())
        .map_err(|_| resource("control count is not representable in wall work"))?;
    let probes = u64::from(options.probes);
    Ok(WallWork {
        preparation: checked_product(&[rows, checked_sum(&[controls, 4])?], "wall preparation")?,
        engine_setup: checked_product(&[parameters, checked_sum(&[controls, 2])?], "wall setup")?,
        full_fit: checked_product(&[rows, checked_sum(&[parameters, 1])?], "wall full fit")?,
        leverage: checked_product(&[rows, probes, 2], "wall leverage")?,
        target: checked_product(&[rows, probes, 2], "wall target")?,
        result_export: checked_sum(&[rows, parameters, probes])?,
    })
}

fn memory_forecast(
    problem: &CompressedProblem,
    options: GenericJlaOptions,
    full_parameters: usize,
    route_memory: RouteMemory,
    memory_facts: MemoryFacts,
    leverage_batch_width: usize,
    target_batch_width: usize,
) -> Result<MemoryForecast> {
    // The forecast is a phase/lifetime upper bound, not a heap allocator
    // promise. It counts all simultaneously live module-owned vectors plus
    // conservative workspace bounds for the shared canonicalizer and model
    // solver. Admission is completed before the Counter-V1 key is used.
    let rows = u64::try_from(problem.outcome.len())
        .map_err(|_| resource("row count is not representable in the memory forecast"))?;
    let physical = problem.physical_total;
    let workers = u64::try_from(problem.workers())
        .map_err(|_| resource("worker count is not representable in the memory forecast"))?;
    let firms = u64::try_from(problem.firms())
        .map_err(|_| resource("firm count is not representable in the memory forecast"))?;
    let cells = u64::try_from(problem.cells())
        .map_err(|_| resource("cell count is not representable in the memory forecast"))?;
    let controls = u64::try_from(problem.controls.len())
        .map_err(|_| resource("control count is not representable in the memory forecast"))?;
    let parameters = u64::try_from(full_parameters)
        .map_err(|_| resource("parameter count is not representable in the memory forecast"))?;
    let probes = u64::from(options.probes);
    let rhs_receipt_count = checked_sum(&[
        controls,
        1,
        u64::from(options.nuisance == NuisanceMode::FixedOffset && controls > 0),
        checked_product(&[probes, 3], "generic-JLA RHS receipt count")?,
    ])?;
    let rhs_receipt_bytes = checked_product(
        &[
            rhs_receipt_count,
            u64::try_from(core::mem::size_of::<GenericJlaRhsReceipt>())
                .map_err(|_| resource("RHS receipt byte size is not representable"))?,
        ],
        "generic-JLA RHS receipts",
    )?;
    let control_projection_receipt_bytes = checked_product(
        &[
            controls,
            u64::try_from(core::mem::size_of::<
                crate::model_solver::ControlProjectionReceipt,
            >())
            .map_err(|_| resource("control projection receipt byte size is not representable"))?,
        ],
        "control projection receipts",
    )?;
    let leverage_width = u64::try_from(leverage_batch_width.min(options.probes as usize))
        .map_err(|_| resource("leverage batch width is not representable"))?;
    let target_width = u64::try_from(target_batch_width.min(options.probes as usize))
        .map_err(|_| resource("target batch width is not representable"))?;
    let deletion_units = u64::try_from(problem.deletion_units())
        .map_err(|_| resource("deletion-unit count is not representable"))?;
    let target_strata = u64::try_from(problem.target_index.ptr.len().saturating_sub(1))
        .map_err(|_| resource("target-stratum count is not representable"))?;
    let maximum_block = memory_facts.maximum_deletion_block;

    let f64_bytes = u64::try_from(core::mem::size_of::<f64>())
        .map_err(|_| resource("f64 byte size is not representable"))?;
    let u32_bytes = u64::try_from(core::mem::size_of::<u32>())
        .map_err(|_| resource("u32 byte size is not representable"))?;
    let usize_bytes = u64::try_from(core::mem::size_of::<usize>())
        .map_err(|_| resource("usize byte size is not representable"))?;
    let vec_bytes = u64::try_from(core::mem::size_of::<Vec<f64>>())
        .map_err(|_| resource("vector-header byte size is not representable"))?;
    let prepared = options.prepared_persistent_bytes;
    let nq = checked_product(&[rows, controls, f64_bytes], "N by Q matrix")?;
    let cq = checked_product(&[cells, controls, f64_bytes], "C by Q matrix")?;
    let q2 = checked_product(&[controls, controls, f64_bytes], "Q-square matrix")?;
    let row_f64 = checked_product(&[rows, f64_bytes], "row vector")?;
    let cell_f64 = checked_product(&[cells, f64_bytes], "cell vector")?;
    let row_index = checked_product(&[rows, usize_bytes], "row index")?;
    let worker_firm = checked_sum(&[workers, firms])?;
    let original_parameters = checked_sum(&[parameters, 1])?;
    let reduced_parameters = checked_sum(&[firms, controls])?;
    let solver_diagonal_values = checked_sum(&[workers, firms, reduced_parameters])?;
    let semantic_header_count = checked_sum(&[deletion_units, target_strata])?;
    let observation_offset_count = checked_sum(&[rows, 1])?;
    let maker_rank = checked_sum(&[controls, 1])?;
    let solver_operator_one = checked_sum(&[
        row_index,
        checked_product(&[solver_diagonal_values, f64_bytes], "solver diagonals")?,
    ])?;
    // Both the full and FE model operators retain canonical worker--firm pair
    // identifiers and weights. The full operator additionally retains the
    // W-by-Q, F-by-Q, and Q-square sufficient statistics used by every PCG
    // action. Pair storage is conservatively priced at N pairs even when the
    // canonical aggregation contains fewer unique worker--firm cells.
    let sufficient_pair_one = checked_product(
        &[rows, checked_sum(&[u32_bytes, u32_bytes, f64_bytes])?],
        "model sufficient pairs",
    )?;
    let sufficient_controls = checked_product(
        &[
            checked_sum(&[
                checked_product(&[workers, controls], "worker-control statistics")?,
                checked_product(&[firms, controls], "firm-control statistics")?,
                checked_product(&[controls, controls], "control-cross statistics")?,
            ])?,
            f64_bytes,
        ],
        "model sufficient controls",
    )?;
    let sufficient_persistent = checked_sum(&[
        checked_product(&[sufficient_pair_one, 2], "full and FE sufficient pairs")?,
        sufficient_controls,
    ])?;
    let solver_persistent = if route_memory.shared_cmg_persistent == 0 {
        let solver_persistent_one = checked_sum(&[
            solver_operator_one,
            checked_product(&[reduced_parameters, f64_bytes], "solver preconditioner")?,
        ])?;
        checked_sum(&[
            checked_product(&[solver_persistent_one, 2], "full and FE solvers")?,
            sufficient_persistent,
        ])?
    } else {
        checked_sum(&[
            checked_product(&[solver_operator_one, 2], "full and FE solver operators")?,
            sufficient_persistent,
            route_memory.shared_cmg_persistent,
            route_memory.full_control_block_persistent,
        ])?
    };
    let canonical_live = checked_sum(&[
        nq,
        checked_product(&[controls, vec_bytes], "canonical control headers")?,
        row_f64,
        row_index,
        solver_persistent,
        q2,
        rhs_receipt_bytes,
        control_projection_receipt_bytes,
    ])?;
    let canonicalization = checked_sum(&[
        prepared,
        checked_product(
            &[row_index, 5],
            "canonical semantic sort and position buffers",
        )?,
        checked_product(&[rows, 2], "canonical seen flags")?,
        checked_product(&[row_f64, 10], "canonical frequency and kernel signatures")?,
        checked_product(&[nq, 8], "canonicalizer simultaneous N by Q matrices")?,
        checked_product(&[controls, vec_bytes, 10], "canonicalizer matrix headers")?,
        checked_product(&[q2, 20], "canonicalizer square work")?,
    ])?;
    let route_persistent = checked_sum(&[
        route_memory.shared_cmg_persistent,
        route_memory.full_control_block_persistent,
    ])?;
    let canonical_without_route = canonical_live
        .checked_sub(route_persistent)
        .ok_or_else(|| resource("route memory exceeds canonical live memory"))?;
    let completed_setup = checked_sum(&[prepared, canonical_live, nq])?;
    let active_setup = checked_sum(&[
        prepared,
        canonical_without_route,
        nq,
        route_memory.setup_transient,
    ])?;
    let setup = completed_setup.max(active_setup);

    // Batched PCG holds projected RHS, solution, residual, preconditioned
    // residual, direction, action, verified action, plus the worker and
    // entity-major parameter action workspaces.
    let solver_batch = |columns: u64, dimension: u64, label: &'static str| -> Result<u64> {
        checked_sum(&[
            checked_product(&[dimension, columns, f64_bytes, 11], label)?,
            checked_product(&[workers, columns, f64_bytes, 3], "solver worker workspace")?,
            checked_product(&[columns, f64_bytes, 8], "solver scalar workspace")?,
            checked_product(&[columns, 96], "solver receipts and flags")?,
            checked_product(
                &[route_memory.cmg_batch_workspace_per_column, columns],
                "batched CMG workspace",
            )?,
        ])
    };
    let scalar_solve = solver_batch(controls.max(1), reduced_parameters, "fit PCG")?;
    let fit_solution_live = checked_product(
        &[original_parameters, f64_bytes, 2],
        "complete fit coefficients and residual",
    )?;
    let fit = checked_sum(&[
        prepared,
        canonical_live,
        checked_product(
            &[original_parameters, f64_bytes, 5],
            "fit RHS and coefficients",
        )?,
        checked_product(&[row_f64, 4], "working outcome fit residual vectors")?,
        nq,
        scalar_solve,
    ])?;
    let geometry_live = checked_sum(&[nq, q2, row_f64])?;
    let geometry = checked_sum(&[
        prepared,
        canonical_live,
        fit_solution_live,
        checked_product(&[nq, 3], "control geometry and rank matrices")?,
        checked_product(
            &[cq, 4],
            "rank cell sum, centered sum, transformed sum, and compensation",
        )?,
        checked_product(&[cell_f64, 2], "rank cell weights and compensation")?,
        checked_product(&[row_f64, 8], "control geometry and rank row vectors")?,
        checked_product(&[q2, 18], "control geometry dense work")?,
    ])?;

    let semantic_plans = checked_sum(&[
        checked_product(&[row_index, 4], "semantic orders and group rows")?,
        checked_product(&[rows, 8, 4], "semantic rank and plan scalars")?,
        checked_product(&[semantic_header_count, vec_bytes], "semantic plan headers")?,
    ])?;
    let leverage_batch = solver_batch(leverage_width, firms, "leverage FE PCG")?;
    let match_leverage = checked_sum(&[
        checked_product(&[deletion_units, leverage_width, 8], "match leverage atoms")?,
        checked_product(&[deletion_units, 5, f64_bytes], "match leverage moments")?,
        checked_product(
            &[worker_firm, leverage_width, f64_bytes],
            "match leverage RHS",
        )?,
        row_f64,
        leverage_batch,
    ])?;
    let observation_leverage = checked_sum(&[
        checked_product(
            &[physical, f64_bytes, 4],
            "observation physical correlations and compensation",
        )?,
        checked_product(
            &[observation_offset_count, usize_bytes],
            "observation row offsets",
        )?,
        checked_product(
            &[row_f64, 7],
            "observation moments, corrections, and prediction",
        )?,
        checked_product(
            &[worker_firm, leverage_width, f64_bytes],
            "observation leverage RHS",
        )?,
        leverage_batch,
    ])?;
    let leverage = checked_sum(&[
        prepared,
        canonical_live,
        geometry_live,
        semantic_plans,
        checked_product(&[row_f64, 3], "leverage retained fit vectors")?,
        if options.deletion == DeletionMode::Observation {
            observation_leverage
        } else {
            match_leverage
        },
    ])?;

    let maker_base = checked_sum(&[
        prepared,
        canonical_live,
        geometry_live,
        semantic_plans,
        checked_product(&[row_f64, 3], "maker retained outcome and residual vectors")?,
    ])?;
    let maker = if options.deletion == DeletionMode::Match {
        let maker_square = if maximum_block <= maker_rank {
            checked_product(
                &[maximum_block, maximum_block, f64_bytes],
                "match maker square",
            )?
        } else {
            checked_product(&[maker_rank, maker_rank, f64_bytes], "reduced maker square")?
        };
        checked_sum(&[
            maker_base,
            checked_product(
                &[maximum_block, maker_rank, f64_bytes],
                "match low-rank maker",
            )?,
            checked_product(&[maximum_block, f64_bytes, 8], "match block vectors")?,
            checked_product(&[maker_square, 8], "maker inverse and factor work")?,
            checked_product(&[maker_rank, f64_bytes, 2], "reduced maker vectors")?,
        ])?
    } else {
        maker_base
    };
    let target_columns = checked_product(&[target_width, 2], "target solver columns")?;
    let target = checked_sum(&[
        prepared,
        canonical_live,
        semantic_plans,
        checked_product(&[row_f64, 6], "target retained row vectors")?,
        checked_product(&[target_strata, target_width, 8], "target atoms")?,
        checked_product(
            &[original_parameters, target_columns, f64_bytes],
            "target RHS",
        )?,
        checked_product(
            &[worker_firm, target_columns, f64_bytes],
            "target RHS compensation",
        )?,
        solver_batch(target_columns, reduced_parameters, "target PCG")?,
        checked_product(&[probes, 4, f64_bytes], "target component draws")?,
    ])?;
    // During the solve-to-result transition the complete prepared context is
    // still live beside the newly retained result. Once the prepared problem
    // is dropped, only its shared retained-mask backing survives, while the C
    // V2 row buffer and caller's fifteen-column matrix coexist with the native
    // result. Count each allocation exactly once in its actual lifetime.
    let native_result_payload =
        checked_sum(&[1024, rhs_receipt_bytes, control_projection_receipt_bytes])?;
    let result_transition = checked_sum(&[prepared, native_result_payload])?;
    let result_export = checked_sum(&[
        native_result_payload,
        options.retained_mask_bytes,
        options.rhs_export_bytes,
    ])?;
    let result = result_transition.max(result_export);
    let phases = [
        (
            GenericJlaMemoryPeakPhase::Canonicalization,
            canonicalization,
        ),
        (GenericJlaMemoryPeakPhase::SolverSetup, setup),
        (GenericJlaMemoryPeakPhase::Fit, fit),
        (GenericJlaMemoryPeakPhase::Geometry, geometry),
        (GenericJlaMemoryPeakPhase::Leverage, leverage),
        (GenericJlaMemoryPeakPhase::Target, target),
        (GenericJlaMemoryPeakPhase::Maker, maker),
        (GenericJlaMemoryPeakPhase::Result, result),
    ];
    let (peak_phase, peak) = phases
        .into_iter()
        .max_by_key(|(_, bytes)| *bytes)
        .expect("generic-JLA has fixed memory phases");
    Ok(MemoryForecast {
        peak,
        peak_phase,
        canonicalization,
        setup,
        fit,
        geometry,
        leverage,
        target,
        maker,
        result,
        native_result_payload,
        result_transition,
        result_export,
        shared_cmg_persistent: route_memory.shared_cmg_persistent,
        full_control_block_persistent: route_memory.full_control_block_persistent,
        setup_transient: route_memory.setup_transient,
        cmg_preconditioner_workspace: route_memory.cmg_preconditioner_workspace,
        cmg_aggregated_cell_capacity: route_memory.cmg_aggregated_cell_capacity,
        cmg_group_index: route_memory.cmg_group_index,
        cmg_hybrid_graph: route_memory.cmg_hybrid_graph,
        retained_nq: nq,
        retained_q2: q2,
    })
}

fn checked_product(factors: &[u64], label: &'static str) -> Result<u64> {
    factors.iter().try_fold(1_u64, |value, &factor| {
        value
            .checked_mul(factor)
            .ok_or_else(|| resource(format!("{label} memory forecast overflow")))
    })
}

fn checked_sum(values: &[u64]) -> Result<u64> {
    values.iter().try_fold(0_u64, |total, &value| {
        total
            .checked_add(value)
            .ok_or_else(|| resource("generic-JLA memory forecast overflow"))
    })
}

fn preflight_trials(trials: u64, label: &'static str) -> Result<()> {
    if trials == 0 || trials > MAX_EXACT_BINARY64_INTEGER {
        return Err(BackendError::new(
            ErrorCode::RngContractFailed,
            "generic_jla_rng_preflight",
            format!("{label} has an invalid physical trial count"),
        ));
    }
    let words = trials.div_ceil(64);
    if words > MAX_PHYSICAL_WORDS_PER_ATOM {
        return Err(BackendError::new(
            ErrorCode::ResourceLimit,
            "generic_jla_rng_preflight",
            format!("{label} requires {words} counter words, above the registered limit"),
        ));
    }
    Ok(())
}

#[inline]
fn rademacher_copy(rng: CounterRng, domain: ProbeDomain, probe: u64, entity: u64, copy: u64) -> i8 {
    let word = rng.word(domain, probe, entity, copy / 64);
    if (word >> (copy % 64)) & 1 == 0 {
        -1
    } else {
        1
    }
}

fn dot(
    left: &[f64],
    right: &[f64],
    interrupt: &mut dyn InterruptCheck,
    phase: &'static str,
) -> Result<f64> {
    if left.len() != right.len() {
        return Err(BackendError::invariant(
            phase,
            "dot-product dimensions disagree",
        ));
    }
    let mut output = StableAccumulator::default();
    for (index, (&left, &right)) in left.iter().zip(right).enumerate() {
        checkpoint_chunk(interrupt, index, phase)?;
        output.add(left * right);
    }
    Ok(output.finish())
}

fn invalid(message: impl Into<String>) -> BackendError {
    BackendError::invalid("generic_jla", message)
}

fn resource(message: impl Into<String>) -> BackendError {
    BackendError::new(ErrorCode::ResourceLimit, "generic_jla", message)
}

fn nonfinite(message: impl Into<String>) -> BackendError {
    BackendError::new(ErrorCode::CorrectionNonFinite, "generic_jla", message)
}

fn nonestimable(message: impl Into<String>) -> BackendError {
    BackendError::new(ErrorCode::NonestimableDeletion, "generic_jla", message)
}

fn preserve_break(error: BackendError, code: ErrorCode, message: &'static str) -> BackendError {
    if error.code == ErrorCode::UserBreak {
        error
    } else {
        BackendError::new(code, "generic_jla", message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deleted_scatter_rejects_indefinite_positive_trace_loss() {
        let checked = [10.0, 0.0, 0.0, 10.0];
        // Positive trace (4), but eigenvalues 5 and -1. A trace-only shortcut
        // would accept this scientifically invalid deletion loss.
        let loss = [2.0, 3.0, 3.0, 2.0];
        let mut maximum_loss = 0.0;
        let mut minimum_deleted = f64::INFINITY;
        let error = certify_deleted_scatter(
            &checked,
            &loss,
            2,
            1.0e-10,
            GenericJlaOptions::default(),
            &mut maximum_loss,
            &mut minimum_deleted,
            &mut NeverInterrupt,
            "generic_jla_rank_test",
        )
        .expect_err("indefinite loss must fail before trace/rank use");
        assert_eq!(error.code, ErrorCode::UnverifiedDeletionRank);
    }

    #[test]
    fn stable_accumulator_retains_large_deep_cancellation_terms() {
        let mut sum = StableAccumulator::default();
        for _ in 0..5_000 {
            sum.add(1.0e16);
            sum.add(1.0);
            sum.add(-1.0e16);
        }
        assert_eq!(sum.finish().to_bits(), 5_000.0_f64.to_bits());
    }

    #[test]
    fn grouped_projection_rank_and_target_rhs_survive_semantic_cancellation() {
        fn grouped(order: &[usize]) -> f64 {
            let values = [1.0e16, 1.0, -1.0e16];
            let mut sum = [0.0];
            let mut correction = [0.0];
            for &row in order {
                stable_add_index(&mut sum, &mut correction, 0, values[row]);
            }
            finish_stable_vector(
                &mut sum,
                &correction,
                &mut NeverInterrupt,
                "generic_jla_cancellation_test",
            )
            .expect("compensated grouped reduction");
            sum[0]
        }

        // These are the exact grouped reduction primitive and semantic order
        // used by FE projection RHS, rank centering, and target-score RHS.
        for semantic_order in [[0, 1, 2], [2, 1, 0]] {
            assert_eq!(grouped(&semantic_order).to_bits(), 1.0_f64.to_bits());
        }
    }

    #[test]
    fn generic_allocation_overflow_is_typed() {
        let error = repeated(
            usize::MAX,
            0_u64,
            "impossible generic allocation",
            &mut NeverInterrupt,
            "generic_jla_allocation_test",
        )
        .expect_err("overflowing allocation must fail before reserving");
        assert!(matches!(
            error.code,
            ErrorCode::ResourceLimit | ErrorCode::AllocationFailed
        ));
    }
}
