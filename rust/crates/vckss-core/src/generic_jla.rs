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

pub(crate) mod residual_moment_attachment;
mod spectrum_batches;
mod statistical_batches;

#[path = "generic_component_batches.rs"]
mod component_batches;
use component_batches::ComponentExecution;
pub use component_batches::{ComponentBatchPolicy, ComponentBatchReceipt};

#[path = "generic_diagonal_queue.rs"]
mod diagonal;
#[cfg(test)]
#[path = "generic_diagonal_tests.rs"]
mod diagonal_tests;
#[path = "generic_full_cmg.rs"]
mod direct;
#[cfg(test)]
#[path = "generic_full_cmg_tests.rs"]
mod direct_tests;
use crate::full_cmg::{FullCmgPlanOptions, FullCmgReceipt, FullCmgSetupReceipt};

use crate::batch_plan::{
    plan_batches_with_forecasts, BatchPlanReceipt, BatchPlannerCaps, BatchRequest,
    BATCH_WIDTH_CANDIDATES,
};
use crate::component_inference::{
    fill_gaussian_pseudo_outcome, finish_component_covariance_with_reporting, finish_influence,
    finish_q1_influence, finish_q1_interval, finish_q1_target, finish_spectrum_diagnostics,
    primitive_plugins, primitive_target_rhs, probe_scalar, q1_probe_scalar, q1_remainder_ratio,
    reported_target_rhs, target_ratios, ComponentInferenceResult, ComponentInferenceSolvePhase,
    ComponentInferenceSolveReceipt, ComponentInferenceUnit, ComponentQ1Status,
    ComponentQ1TargetResult, ComponentReferenceDistribution, ComponentSpectrumDiagnostics,
    ComponentVarianceSource, JointProbeMoments, PreparedComponentInference, PRIMITIVE_TARGETS,
    REPORTED_TARGETS,
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
use crate::exact_estimator::ExactStayerHybridPlan;
use crate::generic_batch::ModelPcgReceipt;
use crate::interrupt::{
    checkpoint_chunk, stable_sort_by_with_interrupt, InterruptCheck, NeverInterrupt,
};
use crate::jla::{JlaPlan, VarianceComponents};
use crate::model_operator::{
    checked_matrix_length, reserve_exact, zeroed_f64_with_interrupt, CanonicalModelData, ModelRhs,
};
use crate::model_solver::{
    is_model_cmg_setup_fallback_error, ControlRankReceipt, ModelCoefficients, ModelRoutingOptions,
    ModelSolve, ModelSolverFallback, ModelSolverOptions, ModelSolverRoute, PreparedModelSolver,
    PreparedModelSolverReceipt,
};
use crate::pipeline_profile::{Phase as ProfilePhase, Scope as ProfileScope};
use crate::problem::CompressedProblem;
use crate::projection::{
    accumulate_projection_covariance, projection_coefficients, PreparedProjection, ProjectionResult,
};
use crate::rng::{CounterRng, ProbeDomain, MAX_PHYSICAL_WORDS_PER_ATOM};
use crate::structured_variance::{
    fit_grouped_structured_variance_with_interrupt, fit_structured_variance_with_interrupt,
    STRUCTURED_OUTER_FOLDS,
};
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
    Projection,
    ComponentInference,
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
    pub projection_peak_bytes: u64,
    /// Conservative incremental peak of the private oracle-variance
    /// component-inference attachment; zero for every public generation.
    pub component_inference_peak_bytes: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GenericJlaThreadReceipt {
    pub requested: usize,
    pub used: usize,
    pub parallel_regions: usize,
}

#[derive(Clone, Debug)]
pub struct GenericJlaExecutionReceipt {
    /// Additive internal diagnostics; frozen native receipts are unchanged.
    pub component_batch: Option<ComponentBatchReceipt>,
    pub full_cmg: Option<FullCmgReceipt>,
    /// Internal-only execution accounting; no frozen native receipt changes.
    pub diagonal_queue: Option<GenericDiagonalQueueReceipt>,
    /// Internal attachment execution; never encoded as the point-only native
    /// model receipt or appended to a frozen ABI layout.
    pub direct_attachments: Option<GenericDirectAttachmentReceipt>,
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

#[derive(Clone, Copy, Debug)]
pub struct GenericDiagonalQueueReceipt {
    pub plan: crate::model_solver::diagonal_queue::DiagonalQueuePlan,
    pub work: crate::model_solver::diagonal_queue::DiagonalQueueWorkReceipt,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GenericDirectAttachmentReceipt {
    pub fit_rhs: usize,
    pub control_projection_rhs: usize,
    pub point_probe_rhs: usize,
    pub projection_rhs: usize,
    pub component_rhs: usize,
    pub gram_rhs: usize,
    pub logical_rhs: usize,
    pub control_refinement_rhs: u64,
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
    pub memory_budget: crate::memory::MemoryBudget,
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
    /// Number of deterministic fixed-effect projection columns attached to
    /// this solve. Zero means no projection request.
    pub projection_columns: usize,
    /// Native projection result payload retained beside the estimator result.
    pub projection_result_bytes: u64,
    /// Simultaneously live caller matrices used to export the projection.
    pub projection_export_bytes: u64,
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
            memory_budget: crate::memory::MemoryBudget::Legacy,
            memory_limit_bytes: u64::MAX,
            prepared_persistent_bytes: 0,
            retained_mask_bytes: 0,
            rhs_export_bytes: 0,
            projection_columns: 0,
            projection_result_bytes: 0,
            projection_export_bytes: 0,
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
        if self.memory_budget.limit(self.memory_limit_bytes) == Some(0) {
            return Err(invalid("whole-command memory limit must be positive"));
        }
        if self.retained_mask_bytes > self.prepared_persistent_bytes {
            return Err(invalid(
                "retained-mask bytes cannot exceed prepared persistent bytes",
            ));
        }
        let projection_memory_absent =
            self.projection_result_bytes == 0 && self.projection_export_bytes == 0;
        let projection_memory_complete =
            self.projection_result_bytes > 0 && self.projection_export_bytes > 0;
        if (self.projection_columns == 0 && !projection_memory_absent)
            || (self.projection_columns > 0 && !projection_memory_complete)
        {
            return Err(invalid(
                "projection dimensions and result/export memory must be supplied together",
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
    pub projection_peak_forecast_bytes: u64,
    pub component_inference_peak_forecast_bytes: u64,
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
    Projection,
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
    pub projection: Option<ProjectionResult>,
    /// Component covariance attachment. The observation variant has a
    /// versioned plugin/Stata boundary; the fixed-offset collapsed-match
    /// variant remains an internal development result.
    pub component_inference: Option<ComponentInferenceResult>,
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
    projection: u64,
    component_inference: u64,
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
    full_cmg: Option<FullCmgSetupReceipt>,
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
        .and_then(|value| value.checked_add(options.projection_columns))
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
    run_generic_jla_routed_with_projection_and_interrupt(
        problem,
        execution_options,
        None,
        interrupt,
    )
}

#[allow(clippy::too_many_lines)]
pub fn run_generic_jla_routed_with_projection_and_interrupt(
    problem: &CompressedProblem,
    execution_options: GenericJlaExecutionOptions,
    projection: Option<&PreparedProjection>,
    interrupt: &mut dyn InterruptCheck,
) -> Result<GenericJlaResult> {
    run_generic_jla_routed_with_projection_and_hybrid_interrupt(
        problem,
        execution_options,
        projection,
        None,
        interrupt,
    )
}

/// Run generic JLA with an optional mixed mover-match/stayer-observation
/// deletion partition.  The hybrid uses one combined fit and pooled target;
/// only the deletion correction is partitioned by source.
#[allow(clippy::too_many_lines)]
pub fn run_generic_jla_routed_with_projection_and_hybrid_interrupt(
    problem: &CompressedProblem,
    execution_options: GenericJlaExecutionOptions,
    projection: Option<&PreparedProjection>,
    hybrid: Option<&ExactStayerHybridPlan>,
    interrupt: &mut dyn InterruptCheck,
) -> Result<GenericJlaResult> {
    run_generic_jla_routed_with_attachments_and_hybrid_interrupt(
        problem,
        execution_options,
        projection,
        None,
        hybrid,
        interrupt,
    )
}

/// Private prepared-generation entrypoint for the oracle-variance component
/// inference MVP. Existing callers continue through the wrapper above with no
/// component attachment and therefore retain identical estimator work.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
pub fn run_generic_jla_routed_with_attachments_and_hybrid_interrupt(
    problem: &CompressedProblem,
    execution_options: GenericJlaExecutionOptions,
    projection: Option<&PreparedProjection>,
    component_inference: Option<&PreparedComponentInference>,
    hybrid: Option<&ExactStayerHybridPlan>,
    interrupt: &mut dyn InterruptCheck,
) -> Result<GenericJlaResult> {
    run_generic_jla_with_direct_solver_interrupt(
        problem,
        execution_options,
        projection,
        component_inference,
        hybrid,
        None,
        interrupt,
    )
}

/// Additive solver selection; existing generic/inference callers retain their
/// original route. This does not change deletion or target construction.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
pub fn run_generic_jla_with_direct_solver_interrupt(
    problem: &CompressedProblem,
    execution_options: GenericJlaExecutionOptions,
    projection: Option<&PreparedProjection>,
    component_inference: Option<&PreparedComponentInference>,
    hybrid: Option<&ExactStayerHybridPlan>,
    full_cmg: Option<FullCmgPlanOptions>,
    interrupt: &mut dyn InterruptCheck,
) -> Result<GenericJlaResult> {
    run_generic_jla_with_execution_interrupt(
        problem,
        execution_options,
        projection,
        component_inference,
        hybrid,
        full_cmg,
        None,
        false,
        ComponentBatchPolicy::Literal,
        interrupt,
    )
}

/// Isolated development entrypoint: no ABI/Ado route selects this executor.
#[doc(hidden)]
pub fn run_generic_jla_with_diagonal_queue_interrupt(
    problem: &CompressedProblem,
    execution_options: GenericJlaExecutionOptions,
    projection: Option<&PreparedProjection>,
    hybrid: Option<&ExactStayerHybridPlan>,
    threads: usize,
    interrupt: &mut dyn InterruptCheck,
) -> Result<GenericJlaResult> {
    run_generic_jla_with_diagonal_queue_attachments_interrupt(
        problem,
        execution_options,
        projection,
        None,
        hybrid,
        threads,
        interrupt,
    )
}

/// Internal diagonal execution only. The existing attachment capability checks
/// still apply; no frozen native request or public routing is changed.
#[doc(hidden)]
#[allow(clippy::too_many_arguments)]
pub fn run_generic_jla_with_diagonal_queue_attachments_interrupt(
    problem: &CompressedProblem,
    execution_options: GenericJlaExecutionOptions,
    projection: Option<&PreparedProjection>,
    component_inference: Option<&PreparedComponentInference>,
    hybrid: Option<&ExactStayerHybridPlan>,
    threads: usize,
    interrupt: &mut dyn InterruptCheck,
) -> Result<GenericJlaResult> {
    if execution_options.routing.route != ModelSolverRoute::Diagonal || threads == 0 {
        return Err(BackendError::invalid(
            "generic_diagonal_queue",
            "positive threads and explicit diagonal required",
        ));
    }
    run_generic_jla_with_execution_interrupt(
        problem,
        execution_options,
        projection,
        component_inference,
        hybrid,
        None,
        Some(threads),
        false,
        ComponentBatchPolicy::Literal,
        interrupt,
    )
}

/// Preserve an original automatic solver request. The registered firm/RHS
/// rule selects the queued diagonal or direct CMG executor before RNG. Only a
/// typed automatic-CMG setup/resource failure may retry the queued diagonal
/// executor; numerical, cancellation and post-RNG failures remain fail-closed.
#[doc(hidden)]
#[allow(clippy::too_many_arguments)]
pub fn run_generic_jla_with_resolved_execution_interrupt(
    problem: &CompressedProblem,
    execution_options: GenericJlaExecutionOptions,
    projection: Option<&PreparedProjection>,
    component_inference: Option<&PreparedComponentInference>,
    hybrid: Option<&ExactStayerHybridPlan>,
    threads: usize,
    full_cmg: FullCmgPlanOptions,
    interrupt: &mut dyn InterruptCheck,
) -> Result<GenericJlaResult> {
    if !matches!(
        execution_options.routing.route,
        ModelSolverRoute::Auto | ModelSolverRoute::Diagonal
    ) || threads == 0
    {
        return Err(BackendError::invalid(
            "generic_execution",
            "resolved execution requires an automatic or diagonal route and positive threads",
        ));
    }
    if full_cmg.threads != threads {
        return Err(BackendError::invalid(
            "generic_execution",
            "resolved execution requires matching queue and CMG thread counts",
        ));
    }
    run_generic_jla_with_execution_interrupt(
        problem,
        execution_options,
        projection,
        component_inference,
        hybrid,
        Some(full_cmg),
        Some(threads),
        true,
        ComponentBatchPolicy::Literal,
        interrupt,
    )
}

/// Internal direct-CMG attachments only. Ordinary native callers retain their
/// point-only guard, request layouts and receipt meanings.
#[doc(hidden)]
#[allow(clippy::too_many_arguments)]
pub fn run_generic_jla_with_direct_attachments_interrupt(
    problem: &CompressedProblem,
    execution_options: GenericJlaExecutionOptions,
    projection: Option<&PreparedProjection>,
    component_inference: Option<&PreparedComponentInference>,
    hybrid: Option<&ExactStayerHybridPlan>,
    plan: FullCmgPlanOptions,
    interrupt: &mut dyn InterruptCheck,
) -> Result<GenericJlaResult> {
    if execution_options.routing.route != ModelSolverRoute::Cmg
        || (projection.is_none() && component_inference.is_none())
    {
        return Err(BackendError::invalid(
            "generic_full_cmg",
            "explicit CMG and attachment required",
        ));
    }
    run_generic_jla_with_execution_interrupt(
        problem,
        execution_options,
        projection,
        component_inference,
        hybrid,
        Some(plan),
        None,
        true,
        ComponentBatchPolicy::Literal,
        interrupt,
    )
}

/// Execution-only automatic inference widths. Legacy entrypoints and numeric
/// augmentation widths are unchanged. Selection and admission precede RNG.
#[doc(hidden)]
#[allow(clippy::too_many_arguments)]
pub fn run_generic_jla_with_automatic_component_batches_interrupt(
    problem: &CompressedProblem,
    execution_options: GenericJlaExecutionOptions,
    component: &PreparedComponentInference,
    threads: usize,
    full_cmg: Option<FullCmgPlanOptions>,
    interrupt: &mut dyn InterruptCheck,
) -> Result<GenericJlaResult> {
    let direct = full_cmg.is_some();
    if threads == 0
        || full_cmg.is_some_and(|plan| plan.threads != threads)
        || execution_options.routing.route
            != if direct {
                ModelSolverRoute::Cmg
            } else {
                ModelSolverRoute::Diagonal
            }
    {
        return Err(invalid(
            "automatic component batches require a matching explicit executor and positive threads",
        ));
    }
    component.options.validate()?;
    ComponentExecution::new(component).cap(threads)?;
    run_generic_jla_with_execution_interrupt(
        problem,
        execution_options,
        None,
        Some(component),
        None,
        full_cmg,
        if direct { None } else { Some(threads) },
        direct,
        ComponentBatchPolicy::Automatic,
        interrupt,
    )
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn run_generic_jla_with_execution_interrupt(
    problem: &CompressedProblem,
    execution_options: GenericJlaExecutionOptions,
    projection: Option<&PreparedProjection>,
    component_inference: Option<&PreparedComponentInference>,
    hybrid: Option<&ExactStayerHybridPlan>,
    full_cmg: Option<FullCmgPlanOptions>,
    diagonal_threads: Option<usize>,
    direct_attachments: bool,
    component_policy: ComponentBatchPolicy,
    interrupt: &mut dyn InterruptCheck,
) -> Result<GenericJlaResult> {
    crate::progress::stage(crate::progress::SETUP);
    interrupt.checkpoint("generic_jla_entry")?;
    let _profile = ProfileScope::new(ProfilePhase::Command);
    let component_prepared = component_inference;
    let mut component_view = component_prepared.map(ComponentExecution::new);
    let component_inference = component_view.as_ref();
    if full_cmg.is_some()
        && (((projection.is_some() || component_inference.is_some()) && !direct_attachments)
            || execution_options.leverage_batch != BatchRequest::Auto
            || execution_options.target_batch != BatchRequest::Auto
            || execution_options.routing.route == ModelSolverRoute::Diagonal)
    {
        return Err(BackendError::new(
            ErrorCode::UnsupportedFeature,
            "generic_full_cmg",
            "direct generic CMG requires point-only automatic batches",
        ));
    }
    let mut routing = execution_options.routing;
    if execution_options.estimator.memory_budget != crate::memory::MemoryBudget::Legacy {
        routing.cmg.memory_budget = execution_options.estimator.memory_budget;
    }
    let routing = routing.validate()?;
    let mut options = execution_options.estimator;
    options.solver = routing.solver;
    let mut options = options.validate()?;
    validate_problem(problem)?;
    let rows = problem.outcome.len();
    let workers = problem.workers();
    let firms = problem.firms();
    let controls = problem.controls.len();
    // Controlled models retain their existing stricter estimation tolerance.
    // Rank preparation supplies its separate fixed 1e-13 solve options.
    let full_cmg = full_cmg.map(|mut plan| {
        if controls > 0 || direct_attachments {
            plan.fit_tolerance = options.solver.pcg.tolerance;
            plan.probe_tolerance = options.solver.pcg.tolerance;
        }
        plan
    });
    let projection_columns = projection.map_or(0, |value| value.columns);
    if projection_columns != options.projection_columns {
        return Err(BackendError::invariant(
            "generic_jla_projection",
            "the attached projection does not reconcile with the solve memory plan",
        ));
    }
    validate_component_inference_request(
        problem,
        routing,
        options,
        projection,
        component_inference,
        hybrid,
    )?;
    validate_hybrid_plan(problem, options.deletion, projection, hybrid)?;
    let residual_moment_mode =
        component_inference.is_some_and(|prepared| prepared.residual_moments.is_some());
    let observation_residual_moment_mode = residual_moment_mode
        && component_inference
            .is_some_and(|prepared| prepared.inference_unit == ComponentInferenceUnit::Observation);
    // An explicit zero-stayer certificate is valid: in samples without an
    // eligible attached stayer, the MATLAB-style default reduces exactly to
    // the mover estimator while retaining the requested public convention.
    let hybrid = hybrid.filter(|plan| plan.stayer_rows.iter().any(|&value| value));
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
        .and_then(|value| value.checked_add(projection_columns))
        .and_then(|value| {
            component_inference.map_or(Some(value), |prepared| {
                usize::try_from(prepared.options.probes)
                    .ok()
                    .and_then(|probes| value.checked_add(PRIMITIVE_TARGETS)?.checked_add(probes))
                    .and_then(|value| {
                        value.checked_add(prepared.residual_moments.map_or(0, |fit| fit.probes))
                    })
            })
        })
        .ok_or_else(|| resource("generic-JLA planned RHS count overflow"))?;
    let automatic_route = if firms < GENERIC_JLA_AUTO_FIRM_THRESHOLD_V1
        || planned_rhs < GENERIC_JLA_AUTO_PLANNED_RHS_THRESHOLD_V1
    {
        ModelSolverRoute::Diagonal
    } else {
        ModelSolverRoute::Cmg
    };
    let direct_selected = full_cmg.is_some()
        && (!direct_attachments
            || routing.route == ModelSolverRoute::Cmg
            || (routing.route == ModelSolverRoute::Auto
                && automatic_route == ModelSolverRoute::Cmg));
    let diagonal_threads = if direct_selected {
        None
    } else {
        diagonal_threads
    };
    let active_direct_attachments = direct_attachments && direct_selected;
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

    let design_only_order = component_inference.is_some_and(|prepared| prepared.design_only_order);
    let control_order = if design_only_order {
        residual_moment_attachment::canonical_design_order(problem, interrupt)?
    } else if observation_residual_moment_mode {
        residual_moment_attachment::design_order(problem, interrupt)?
    } else {
        control_semantic_order(problem, options.rank_tolerance, interrupt)?
    };
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
    let row_order = if design_only_order {
        residual_moment_attachment::canonical_design_order(problem, interrupt)?
    } else if observation_residual_moment_mode {
        residual_moment_attachment::design_order(problem, interrupt)?
    } else {
        canonical_row_order(problem, &canonical.columns, interrupt)?
    };
    let full_data = CanonicalModelData {
        workers,
        firms,
        row_worker: &problem.row_worker,
        row_firm: &problem.row_firm,
        weight: &weights,
        controls: &canonical.columns,
    };
    let mut controlled_batch = None;
    let mut diagonal_plan = None;
    let mut component_batch = None;
    let prepared_solvers = if let Some(threads) = diagonal_threads {
        let (batch, plan, inference) = diagonal::plan(
            problem,
            options,
            full_parameters,
            hybrid.is_some(),
            component_inference,
            component_policy,
            execution_options.leverage_batch,
            execution_options.target_batch,
            threads,
            memory_facts(problem, interrupt)?,
            interrupt,
        )?;
        component_batch = inference;
        // Forecast already includes the complete generic lifetime; the runtime
        // validates the same increment again before allocating its owned pool.
        let budget = match options.memory_budget {
            crate::memory::MemoryBudget::Legacy => crate::memory::MemoryBudget::Explicit {
                bytes: options.memory_limit_bytes,
                check: crate::memory::MemoryCheck::Error,
            },
            budget => budget,
        };
        interrupt.checkpoint("generic_diagonal_queue_admitted")?;
        let queue = std::sync::Arc::new(
            crate::model_solver::diagonal_queue::DiagonalQueueRuntime::new(
                [workers, firms, controls],
                threads,
                plan.maximum_rhs,
                plan.command_peak_bytes - plan.incremental_peak_bytes,
                budget,
                interrupt,
            )?,
        );
        if queue.plan() != plan {
            return Err(BackendError::invariant(
                "generic_diagonal_queue",
                "runtime admission differs from frozen plan",
            ));
        }
        diagonal_plan = Some(plan);
        controlled_batch = Some(batch);
        PreparedModelSolver::prepare_generic_jla_queued_diagonal(
            full_data,
            routed_prepare,
            queue,
            interrupt,
        )?
    } else if direct_selected {
        let mut plan = full_cmg.ok_or_else(|| {
            BackendError::invariant(
                "generic_full_cmg",
                "resolved direct execution lost its CMG plan",
            )
        })?;
        plan.memory_budget = options.memory_budget;
        let (_, _, maximum) = crate::full_cmg_batch_policy::caps(
            options.probes as usize,
            plan.threads,
            crate::full_cmg_batch_policy::SELECTED_K,
        )?;
        plan.maximum_batch_rhs = maximum;
        if component_policy == ComponentBatchPolicy::Automatic {
            if let Some(component) = component_inference {
                plan.maximum_batch_rhs = maximum.max(component.cap(plan.threads)?);
            }
        }
        let minimum_component = component_inference.map(|value| value.select(component_policy, 1));
        plan.non_cmg_command_peak_bytes = direct::attachments::forecast(
            problem,
            options,
            full_parameters,
            hybrid.is_some(),
            if active_direct_attachments {
                minimum_component.as_ref()
            } else {
                None
            },
            RouteMemory::default(),
            memory_facts(problem, interrupt)?,
            1,
            1,
        )?
        .peak;
        let prepared_direct = if active_direct_attachments {
            direct::attachments::prepare(
                problem,
                full_data,
                routed_prepare,
                plan,
                options,
                full_parameters,
                hybrid.is_some(),
                component_inference,
                component_policy,
                interrupt,
            )
            .map(|(pair, batch, inference)| (pair, Some(batch), inference))
        } else if controls == 0 {
            PreparedModelSolver::prepare_generic_jla_direct(
                problem,
                full_data,
                routed_prepare,
                plan,
                options.memory_limit_bytes,
                interrupt,
            )
            .map(|pair| (pair, None, None))
        } else {
            direct::prepare_controlled(
                problem,
                full_data,
                routed_prepare,
                plan,
                options,
                full_parameters,
                hybrid.is_some(),
                interrupt,
            )
            .map(|(pair, batch)| (pair, Some(batch), None))
        };
        match prepared_direct {
            Ok((pair, batch, inference)) => {
                component_batch = inference;
                controlled_batch = batch;
                pair
            }
            Err(error)
                if routing.route == ModelSolverRoute::Auto
                    && routing.allow_automatic_cmg_setup_fallback
                    && is_model_cmg_setup_fallback_error(&error) =>
            {
                let fallback = ModelSolverFallback {
                    from: ModelSolverRoute::Cmg,
                    to: ModelSolverRoute::Diagonal,
                    code: error.code,
                    message: error.to_string(),
                };
                crate::progress::report(crate::progress::FALLBACK, [1, 0, 0, 0, 0, 0, 0, 0]);
                let mut fallback_options = execution_options;
                fallback_options.routing.route = ModelSolverRoute::Diagonal;
                fallback_options.routing.allow_automatic_cmg_setup_fallback = false;
                let mut result = run_generic_jla_with_execution_interrupt(
                    problem,
                    fallback_options,
                    projection,
                    component_prepared,
                    hybrid,
                    None,
                    Some(plan.threads),
                    false,
                    component_policy,
                    interrupt,
                )?;
                result.receipt.execution.requested_route = ModelSolverRoute::Auto;
                result.receipt.execution.fallback = Some(fallback.clone());
                result.receipt.execution.full_solver_setup.requested = ModelSolverRoute::Auto;
                result.receipt.execution.full_solver_setup.fallback = Some(fallback);
                return Ok(result);
            }
            Err(error) => return Err(error),
        }
    } else {
        PreparedModelSolver::prepare_generic_jla_routed_with_interrupt(
            full_data,
            routed_prepare,
            interrupt,
        )?
    };
    if let (Some(prepared), Some(receipt)) = (&mut component_view, component_batch) {
        // The selected component and Gram widths may have different probe caps.
        prepared.options.batch_width = receipt.component_width;
        if let Some(gram) = &mut prepared.residual_moments {
            gram.batch_width = receipt.gram_width;
        }
    }
    let component_inference = component_view.as_ref();
    let fe_hierarchy_reused = prepared_solvers.fe_hierarchy_reused;
    let mut full_solver = prepared_solvers.full;
    let mut fe_solver = prepared_solvers.fe;
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
    let mut route_memory = route_memory_forecast(problem, &full_solver, controls)?;
    let memory_facts = memory_facts(problem, interrupt)?;
    let mut batch = if let Some(batch) = controlled_batch {
        batch
    } else {
        plan_generic_jla_batches(
            problem,
            options,
            full_parameters,
            hybrid.is_some(),
            execution_options.leverage_batch,
            execution_options.target_batch,
            route_memory,
            memory_facts,
            interrupt,
        )?
    };
    options.leverage_batch_width = batch.leverage_active_width;
    options.target_batch_width = batch.target_active_width;
    if direct_selected && !active_direct_attachments {
        let maximum = options
            .target_batch_width
            .checked_mul(2)
            .ok_or_else(|| resource("target RHS capacity overflow"))?
            .max(options.leverage_batch_width);
        if controls == 0 {
            full_solver.configure_full_cmg_capacity(maximum)?;
        } else if full_solver
            .full_cmg_receipt()?
            .is_none_or(|receipt| receipt.setup.maximum_batch_rhs != maximum)
        {
            return Err(BackendError::invariant(
                "generic_full_cmg",
                "control preparation and final batch capacities disagree",
            ));
        }
        route_memory = route_memory_forecast(problem, &full_solver, controls)?;
        let before = memory_forecast(
            problem,
            options,
            full_parameters,
            hybrid.is_some(),
            route_memory,
            memory_facts,
            options.leverage_batch_width,
            options.target_batch_width,
        )?;
        if options
            .memory_budget
            .rejects(before.peak, options.memory_limit_bytes)
        {
            admit_memory(before.peak, options.memory_limit_bytes)?;
        }
        interrupt.checkpoint("generic_full_cmg_pools_admitted")?;
        if controls == 0 {
            full_solver.allocate_full_cmg_pools()?;
        }
        fe_solver.refresh_full_cmg_receipt()?;
        route_memory = route_memory_forecast(problem, &full_solver, controls)?;
        direct::refresh_batch(
            problem,
            options,
            full_parameters,
            hybrid.is_some(),
            route_memory,
            memory_facts,
            &mut batch,
        )?;
        if batch.plan.selected_command_peak_bytes > before.peak {
            return Err(BackendError::invariant(
                "generic_full_cmg",
                "retained pool exceeds admitted forecast",
            ));
        }
    }
    let mut memory = memory_forecast(
        problem,
        options,
        full_parameters,
        hybrid.is_some(),
        route_memory,
        memory_facts,
        options.leverage_batch_width,
        options.target_batch_width,
    )?;
    if diagonal_plan.is_none()
        && !active_direct_attachments
        && memory.peak != batch.plan.selected_command_peak_bytes
    {
        return Err(BackendError::invariant(
            "generic_jla_plan",
            "selected batch plan and final memory forecast disagree",
        ));
    }
    if let Some(prepared) = component_inference {
        memory.component_inference = component_inference_peak_forecast(
            problem,
            prepared,
            full_parameters,
            route_memory,
            if active_direct_attachments {
                full_solver.owned_batch_capacity()?
            } else {
                diagonal_plan.map(|plan| plan.maximum_rhs)
            },
            full_solver.owned_batch_workers()?,
        )?;
        // This first private implementation admits a deliberately
        // conservative lifetime bound: the existing complete command peak and
        // every component-attachment allocation may coexist. Later profiling
        // may tighten the bound without changing the statistical result.
        memory.peak = checked_sum(&[memory.peak, memory.component_inference])?;
        memory.peak_phase = GenericJlaMemoryPeakPhase::ComponentInference;
    }
    if let Some(plan) = diagonal_plan {
        diagonal::add_increment(&mut memory, plan.incremental_peak_bytes)?;
        if memory.peak != batch.plan.selected_command_peak_bytes
            || memory.peak != plan.command_peak_bytes
        {
            return Err(BackendError::invariant(
                "generic_diagonal_queue",
                "attachment lifetime differs from pre-pool admission",
            ));
        }
    }
    if active_direct_attachments && memory.peak != batch.plan.selected_command_peak_bytes {
        return Err(BackendError::invariant(
            "generic_full_cmg",
            "attachment lifetime differs from pre-pool admission",
        ));
    }
    if options
        .memory_budget
        .rejects(memory.peak, options.memory_limit_bytes)
    {
        admit_memory(memory.peak, options.memory_limit_bytes)?;
    }
    let wall = wall_work_receipt(
        generic_jla_wall_work(problem, options, full_parameters, hybrid.is_some())?,
        execution_options.wallseconds,
        WallCalibration::Uncalibrated,
    )?;
    full_solver.reconcile_full_cmg_memory(memory.peak)?;
    let mut execution = GenericJlaExecutionReceipt {
        component_batch,
        full_cmg: None,
        diagonal_queue: None,
        direct_attachments: None,
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
            projection_peak_bytes: memory.projection,
            component_inference_peak_bytes: memory.component_inference,
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
    if crate::progress::enabled() {
        crate::progress::report(
            crate::progress::PLAN,
            [
                2,
                2,
                if full_solver.receipt().selected == ModelSolverRoute::Cmg {
                    2
                } else {
                    1
                },
                full_solver.owned_batch_workers().unwrap_or(1) as u64,
                options.leverage_batch_width as u64,
                options.target_batch_width as u64,
                0,
                0,
            ],
        );
        crate::progress::report(
            crate::progress::MEMORY,
            [memory.peak, memory.peak, 0, 0, 0, 0, 0, 0],
        );
    }
    crate::progress::stage(crate::progress::FIT);
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
            hybrid,
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
    let projection_coefficients = projection
        .map(|prepared| projection_coefficients(prepared, &working_fit.coefficients))
        .transpose()?;
    let inference_coefficients = component_inference
        .is_some()
        .then(|| working_fit.coefficients.clone());
    drop(working_fit);

    let row_rank = if observation_residual_moment_mode {
        residual_moment_attachment::row_ranks(&row_order)
    } else {
        match options.deletion {
            DeletionMode::Match => semantic_row_ranks(problem, &canonical.columns, interrupt)?,
            DeletionMode::Observation => {
                observation_row_ranks(problem, &canonical.columns, interrupt)?
            }
        }
    };
    let structured_fold_entity = component_inference
        .filter(|prepared| {
            prepared.inference_unit == ComponentInferenceUnit::Observation
                && prepared.variance_source.structured_model().is_some()
                && prepared.residual_moments.is_none()
        })
        .map(|_| structured_variance_row_ranks(problem, &canonical.columns, interrupt))
        .transpose()?;
    let mut structured_fold_entity = structured_fold_entity;
    let target_plan = target_plan(
        problem,
        &row_rank,
        if hybrid.is_some() {
            DeletionMode::Observation
        } else {
            options.deletion
        },
        interrupt,
    )?;
    let target_counter = plan_counter_phase(
        options.probes,
        &target_plan.physical_count,
        GeneratorEvaluationModel::PackedWords,
    )?;
    let mut prepared_match_plan = None;
    let mut prepared_observation_classes = None;
    let leverage_counter = if let Some(hybrid) = hybrid {
        let mut plan = match_plan(problem, &row_rank, options, interrupt)?;
        retain_mover_match_groups(&mut plan, hybrid.mover_deletion_units)?;
        let match_counter = plan_counter_phase(
            options.probes,
            &plan.physical_count,
            GeneratorEvaluationModel::PackedWords,
        )?;
        let observation_rank = observation_row_ranks(problem, &canonical.columns, interrupt)?;
        let classes =
            stayer_observation_classes(problem, &observation_rank, &hybrid.stayer_rows, interrupt)?;
        let mut physical_count = Vec::new();
        reserve_exact(
            &mut physical_count,
            classes.len(),
            "hybrid observation Counter accounting counts",
        )?;
        let mut stayer_physical = 0_u64;
        for class in &classes {
            physical_count.push(class.physical_count);
            stayer_physical = stayer_physical
                .checked_add(class.physical_count)
                .ok_or_else(|| resource("hybrid stayer physical count overflow"))?;
        }
        let observation_counter = plan_counter_phase_with_logical_atoms(
            options.probes,
            stayer_physical,
            &physical_count,
            GeneratorEvaluationModel::PhysicalTrials { passes: 2 },
        )?;
        prepared_match_plan = Some(plan);
        prepared_observation_classes = Some(classes);
        combine_counter_phases(match_counter, observation_counter)?.total
    } else {
        match options.deletion {
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
        }
    };
    let component_addresses = match component_inference {
        Some(prepared) if prepared.inference_unit == ComponentInferenceUnit::Observation => {
            Some(observation_inference_addresses(
                problem,
                prepared_observation_classes.as_deref().ok_or_else(|| {
                    BackendError::invariant(
                        "generic_jla_component_inference",
                        "component-inference Counter preflight is missing observation classes",
                    )
                })?,
                interrupt,
            )?)
        }
        Some(prepared) if prepared.inference_unit == ComponentInferenceUnit::Match => {
            let (addresses, folds) = grouped_component_addresses_and_folds(
                prepared_match_plan.as_ref().ok_or_else(|| {
                    BackendError::invariant(
                        "generic_jla_component_inference",
                        "grouped component-inference preflight is missing its match plan",
                    )
                })?,
                interrupt,
            )?;
            if prepared.variance_source.structured_model().is_some()
                && prepared.residual_moments.is_none()
            {
                structured_fold_entity = Some(folds);
            }
            Some(addresses)
        }
        None => None,
        Some(_) => unreachable!("component inference unit is exhaustive"),
    };
    let component_counter_plan = match component_inference {
        Some(prepared) if prepared.inference_unit == ComponentInferenceUnit::Observation => {
            Some(plan_component_inference_counter(
                prepared,
                prepared_observation_classes.as_deref().ok_or_else(|| {
                    BackendError::invariant(
                        "generic_jla_component_inference",
                        "component-inference Counter preflight is missing observation classes",
                    )
                })?,
                structured_fold_entity
                    .as_ref()
                    .and_then(|entity| entity.iter().copied().max()),
            )?)
        }
        Some(prepared) if prepared.inference_unit == ComponentInferenceUnit::Match => {
            Some(plan_grouped_component_inference_counter(
                prepared,
                component_addresses
                    .as_ref()
                    .expect("grouped addresses prepared"),
                structured_fold_entity
                    .as_ref()
                    .and_then(|entity| entity.iter().copied().max()),
            )?)
        }
        None => None,
        Some(_) => unreachable!("component inference unit is exhaustive"),
    };
    execution.counter = combine_counter_phases(leverage_counter, target_counter)?;
    let rng = CounterRng::new(options.seed);
    // Match row order remains live through the target contractions. Observation
    // contractions use the global canonical row order directly.
    let mut match_rows_for_target = None;
    let mut component_geometry = None;
    let (
        deleted_adjusted,
        maximum_leverage,
        maximum_maker_relres,
        leverage_solve_relres,
        deletion_units,
    ) = if let Some(hybrid_plan) = hybrid {
        let match_plan = prepared_match_plan
            .take()
            .expect("hybrid match plan was frozen before Counter-V1 addressing");
        let observation_classes = prepared_observation_classes
            .take()
            .expect("hybrid observation classes were frozen before Counter-V1 addressing");
        let group_count = match_plan.rows.len();
        let (match_moments, observation_moments, signs, solve_relres) = hybrid_leverage_moments(
            problem,
            &match_plan,
            &observation_classes,
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
        let mover_adjusted = match_deleted_adjustment(
            problem,
            &match_plan,
            &match_moments,
            &residual,
            active_geometry,
            options,
            false,
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
                "generic_jla_hybrid_allocate",
            )?;
            &zero_control_leverage
        };
        let stayer_adjusted = observation_deleted_adjustment(
            problem,
            &observation_classes,
            &observation_moments,
            &signs,
            &residual,
            active_leverage,
            &row_order,
            options,
            false,
            interrupt,
        )?;
        let adjusted = combine_hybrid_adjustments(
            &mover_adjusted.values,
            &stayer_adjusted.values,
            &hybrid_plan.stayer_rows,
            interrupt,
        )?;
        let stayer_physical = hybrid_plan
            .stayer_rows
            .iter()
            .enumerate()
            .filter(|(_, value)| **value)
            .try_fold(0_u64, |total, (row, _)| {
                total
                    .checked_add(problem.frequency[row])
                    .ok_or_else(|| resource("hybrid stayer deletion-unit count overflow"))
            })?;
        match_rows_for_target = Some(match_plan);
        (
            adjusted,
            mover_adjusted
                .maximum_leverage
                .max(stayer_adjusted.maximum_leverage),
            mover_adjusted.maximum_relres,
            solve_relres,
            u64::try_from(group_count)
                .map_err(|_| resource("hybrid mover deletion-unit count overflow"))?
                .checked_add(stayer_physical)
                .ok_or_else(|| resource("hybrid deletion-unit count overflow"))?,
        )
    } else {
        match options.deletion {
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
                let mut adjusted = match_deleted_adjustment(
                    problem,
                    &plan,
                    &moments,
                    &residual,
                    active_geometry,
                    options,
                    component_inference.is_some(),
                    interrupt,
                )?;
                if component_inference.is_some() {
                    component_geometry = Some((
                        adjusted.inference_leverage.take().ok_or_else(|| {
                            BackendError::invariant(
                                "generic_jla_component_inference",
                                "grouped inference leverage was not retained",
                            )
                        })?,
                        adjusted.inference_maker_inverse.take().ok_or_else(|| {
                            BackendError::invariant(
                                "generic_jla_component_inference",
                                "grouped inference maker inverse was not retained",
                            )
                        })?,
                    ));
                }
                match_rows_for_target = Some(plan);
                (
                    adjusted.values,
                    adjusted.maximum_leverage,
                    adjusted.maximum_relres,
                    solve_relres,
                    u64::try_from(group_count)
                        .map_err(|_| resource("deletion-unit count overflow"))?,
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
                let mut adjusted = observation_deleted_adjustment(
                    problem,
                    &classes,
                    &moments,
                    &signs,
                    &residual,
                    active_leverage,
                    &row_order,
                    options,
                    component_inference.is_some(),
                    interrupt,
                )?;
                if component_inference.is_some() {
                    component_geometry = Some((
                        adjusted.inference_leverage.take().ok_or_else(|| {
                            BackendError::invariant(
                                "generic_jla_component_inference",
                                "inference leverage was not retained",
                            )
                        })?,
                        adjusted.inference_maker_inverse.take().ok_or_else(|| {
                            BackendError::invariant(
                                "generic_jla_component_inference",
                                "inference maker inverse was not retained",
                            )
                        })?,
                    ));
                }
                (
                    adjusted.values,
                    adjusted.maximum_leverage,
                    0.0,
                    solve_relres,
                    problem.physical_total,
                )
            }
        }
    };
    drop(geometry);
    drop(row_rank);

    let target = target_correction(
        problem,
        &target_plan,
        &working_y,
        &deleted_adjusted,
        working_solver,
        &row_order,
        match_rows_for_target
            .as_ref()
            .map(|plan| plan.rows.as_slice()),
        hybrid.map(|plan| plan.stayer_rows.as_slice()),
        rng,
        options,
        component_inference.is_some(),
        &mut rhs_receipts,
        interrupt,
    )?;
    let target_strata = target_plan.cell.len();
    drop(target_plan);
    let mut component_inference_result = match (
        component_inference,
        inference_coefficients.as_ref(),
        target.diagonal.as_ref(),
        component_addresses.as_ref(),
        component_geometry.as_ref(),
        component_counter_plan,
    ) {
        (
            Some(prepared),
            Some(coefficients),
            Some(diagonal),
            Some(addresses),
            Some((leverage, maker_inverse)),
            Some(counter_plan),
        ) => Some(match prepared.inference_unit {
            ComponentInferenceUnit::Observation => run_component_inference_attachment(
                problem,
                prepared,
                working_solver,
                coefficients,
                ComponentInferenceRows::Observation {
                    controls: &canonical.columns,
                    row_order: &row_order,
                },
                &working_y,
                &residual,
                &deleted_adjusted,
                leverage,
                maker_inverse,
                diagonal,
                addresses,
                structured_fold_entity.as_deref(),
                None,
                target.mean,
                memory.component_inference,
                counter_plan,
                interrupt,
            )?,
            ComponentInferenceUnit::Match => {
                let match_plan = match_rows_for_target.as_ref().ok_or_else(|| {
                    BackendError::invariant(
                        "generic_jla_component_inference",
                        "grouped component inference lost its match plan",
                    )
                })?;
                let collapsed = collapse_match_component_data(
                    problem,
                    match_plan,
                    &working_y,
                    &residual,
                    &deleted_adjusted,
                    diagonal,
                    interrupt,
                )?;
                run_component_inference_attachment(
                    problem,
                    prepared,
                    working_solver,
                    coefficients,
                    ComponentInferenceRows::Match { plan: match_plan },
                    &collapsed.outcome,
                    &collapsed.residual,
                    &collapsed.deleted_adjusted,
                    leverage,
                    maker_inverse,
                    &collapsed.target_diagonal,
                    addresses,
                    structured_fold_entity.as_deref(),
                    Some(&collapsed.mass),
                    target.mean,
                    memory.component_inference,
                    counter_plan,
                    interrupt,
                )?
            }
        }),
        (None, None, None, None, None, None) => None,
        _ => {
            return Err(BackendError::invariant(
                "generic_jla_component_inference",
                "the private component-inference attachment state is incomplete",
            ));
        }
    };
    let projection = match (projection, projection_coefficients) {
        (Some(prepared), Some(coefficients)) => {
            crate::progress::stage(crate::progress::PROJECTION);
            let _profile = ProfileScope::new(ProfilePhase::Projection);
            let q = prepared.columns;
            let active_controls: &[Vec<f64>] = if options.nuisance == NuisanceMode::Joint {
                &canonical.columns
            } else {
                &[]
            };
            let control_rhs = vec![0.0; active_controls.len().saturating_mul(q)];
            let solved = working_solver.solve_batch_with_interrupt(
                &prepared.worker_rhs,
                &prepared.firm_rhs,
                &control_rhs,
                q,
                q.min(options.target_batch_width.max(1)),
                interrupt,
            )?;
            for (column, solution) in solved.solution.iter().enumerate() {
                rhs_receipts.push(rhs_receipt(
                    GenericJlaRhsPhase::Projection,
                    GenericJlaRhsSide::Joint,
                    Some(u32::try_from(column).map_err(|_| {
                        resource("projection RHS index is not representable as u32")
                    })?),
                    solution,
                ));
            }
            let _statistics_profile = ProfileScope::new(ProfilePhase::ProjectionStatistics);
            let mut result = accumulate_projection_covariance(
                problem,
                prepared,
                coefficients,
                &solved.solution,
                active_controls,
                &working_y,
                &deleted_adjusted,
                &residual,
                &row_order,
                options.deletion,
                match_rows_for_target
                    .as_ref()
                    .map(|plan| plan.rows.as_slice()),
                hybrid.map(|plan| plan.stayer_rows.as_slice()),
                interrupt,
            )?;
            result.projection_peak_forecast_bytes = memory.projection;
            Some(result)
        }
        (None, None) => None,
        _ => {
            return Err(BackendError::invariant(
                "generic_jla_projection",
                "projection coefficient state is incomplete",
            ));
        }
    };
    drop(match_rows_for_target);
    drop(deleted_adjusted);
    drop(working_y);
    drop(residual);
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
        .max(target.maximum_solve_relres)
        .max(
            component_inference_result
                .as_ref()
                .map_or(0.0, |value| value.maximum_reduced_residual),
        )
        .max(
            projection
                .as_ref()
                .map_or(0.0, |value| value.maximum_reduced_residual),
        );
    if rhs_receipts.len() != rhs_receipt_capacity {
        return Err(BackendError::invariant(
            "generic_jla_receipts",
            "generic-JLA RHS receipt count does not match its preflighted capacity",
        ));
    }
    let (maximum_reduced_residual, mut maximum_complete_residual) =
        rhs_residual_maxima(&rhs_receipts);
    if let Some(inference) = &component_inference_result {
        maximum_complete_residual =
            maximum_complete_residual.max(inference.maximum_complete_residual);
    }
    drop(fe_solver);
    execution.full_cmg = full_solver.full_cmg_receipt()?;
    if active_direct_attachments {
        let cmg = execution.full_cmg.as_ref().ok_or_else(|| {
            BackendError::invariant("generic_full_cmg", "lost attachment solver receipt")
        })?;
        let component_rhs = component_inference_result
            .as_ref()
            .map_or(0, |value| value.solve_receipts.len());
        let gram_rhs = component_inference_result
            .as_ref()
            .and_then(|value| value.residual_moments.as_ref())
            .map_or(0, |fit| fit.projections.len());
        let logical_rhs = rhs_receipts
            .len()
            .checked_add(component_rhs)
            .and_then(|count| count.checked_add(gram_rhs))
            .ok_or_else(|| resource("direct attachment logical RHS count overflow"))?;
        let expected = u64::try_from(logical_rhs)
            .ok()
            .and_then(|count| count.checked_add(cmg.model_diagnostics.control_refinement_rhs_count))
            .ok_or_else(|| resource("direct attachment refinement count overflow"))?;
        if cmg.rhs_count != expected
            || cmg.model_diagnostics.explicit_options_rhs_count != controls as u64
        {
            return Err(BackendError::invariant(
                "generic_full_cmg",
                "attachment solve work does not reconcile",
            ));
        }
        execution.direct_attachments = Some(GenericDirectAttachmentReceipt {
            fit_rhs: 1 + usize::from(options.nuisance == NuisanceMode::FixedOffset && controls > 0),
            control_projection_rhs: controls,
            point_probe_rhs: (options.probes as usize)
                .checked_mul(3)
                .ok_or_else(|| resource("direct probe count overflow"))?,
            projection_rhs: projection_columns,
            component_rhs,
            gram_rhs,
            logical_rhs,
            control_refinement_rhs: cmg.model_diagnostics.control_refinement_rhs_count,
        });
    }
    if let Some(plan) = diagonal_plan {
        let work = full_solver.diagonal_queue_receipt()?.ok_or_else(|| {
            BackendError::invariant("generic_diagonal_queue", "lost shared queue receipt")
        })?;
        let expected = (options.probes as usize)
            .checked_mul(3)
            .and_then(|count| count.checked_add(controls))
            .and_then(|count| count.checked_add(options.projection_columns))
            .and_then(|count| {
                component_inference_result
                    .as_ref()
                    .map_or(Some(count), |inference| {
                        count
                            .checked_add(inference.solve_receipts.len())?
                            .checked_add(
                                inference
                                    .residual_moments
                                    .as_ref()
                                    .map_or(0, |fit| fit.projections.len()),
                            )
                    })
            })
            .ok_or_else(|| resource("queued logical RHS count overflow"))?;
        if work.completed_rhs != expected {
            return Err(BackendError::invariant(
                "generic_diagonal_queue",
                "queued RHS count does not reconcile",
            ));
        }
        execution.threads = GenericJlaThreadReceipt {
            requested: plan.permitted_threads,
            used: plan.workers,
            parallel_regions: work.parallel_batches,
        };
        execution.diagonal_queue = Some(GenericDiagonalQueueReceipt { plan, work });
    }
    if let Some(receipt) = &execution.full_cmg {
        execution.threads = GenericJlaThreadReceipt {
            requested: receipt.setup.threads,
            used: receipt.setup.threads,
            parallel_regions: usize::from(
                receipt.planned_batches > 0 || receipt.across_rhs_batches > 0,
            ),
        };
    }
    drop(full_solver);
    drop(row_order);
    drop(weights);
    drop(canonical);
    crate::progress::stage(crate::progress::VALIDATION);
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
        projection,
        component_inference: component_inference_result.take(),
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
            full_residual_tolerance: full_cmg
                .map_or(options.solver.full_residual_tolerance(), |plan| {
                    (10.0 * plan.fit_tolerance.max(plan.probe_tolerance)).max(1e-11)
                }),
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
            projection_peak_forecast_bytes: memory.projection,
            component_inference_peak_forecast_bytes: memory.component_inference,
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

fn validate_hybrid_plan(
    problem: &CompressedProblem,
    deletion: DeletionMode,
    _projection: Option<&PreparedProjection>,
    hybrid: Option<&ExactStayerHybridPlan>,
) -> Result<()> {
    let Some(plan) = hybrid else {
        return Ok(());
    };
    if deletion != DeletionMode::Match {
        return Err(BackendError::new(
            ErrorCode::UnsupportedFeature,
            "generic_jla_hybrid",
            "the stayer hybrid requires match deletion for movers",
        ));
    }
    if plan.stayer_rows.len() != problem.outcome.len()
        || plan.mover_deletion_units == 0
        || plan.mover_deletion_units > problem.deletion_units()
    {
        return Err(BackendError::invariant(
            "generic_jla_hybrid",
            "the mixed-deletion partition has invalid dimensions",
        ));
    }
    for row in 0..problem.outcome.len() {
        let mover_group = (problem.row_deletion[row] as usize) < plan.mover_deletion_units;
        if mover_group == plan.stayer_rows[row] {
            return Err(BackendError::invariant(
                "generic_jla_hybrid",
                "the mixed-deletion row mask disagrees with deletion-group ordering",
            ));
        }
    }
    Ok(())
}

fn validate_component_inference_request(
    problem: &CompressedProblem,
    routing: ModelRoutingOptions,
    options: GenericJlaOptions,
    projection: Option<&PreparedProjection>,
    prepared: Option<&ComponentExecution<'_>>,
    hybrid: Option<&ExactStayerHybridPlan>,
) -> Result<()> {
    let Some(prepared) = prepared else {
        return Ok(());
    };
    if prepared.residual_moments.is_some() && prepared.variance_source.structured_model().is_none()
    {
        return Err(invalid(
            "residual moments require inference with a structured basis",
        ));
    }
    let expected_variance = match prepared.inference_unit {
        ComponentInferenceUnit::Observation => problem.outcome.len(),
        ComponentInferenceUnit::Match => problem.deletion_units(),
    };
    let variance_shape_valid = match prepared.variance_source {
        ComponentVarianceSource::Oracle => prepared.variance.len() == expected_variance,
        ComponentVarianceSource::StructuredCommon | ComponentVarianceSource::StructuredLeverage => {
            prepared.variance.is_empty()
        }
    };
    if prepared.schema_version != crate::component_inference::COMPONENT_INFERENCE_SCHEMA_VERSION
        || !variance_shape_valid
    {
        return Err(BackendError::invariant(
            "generic_jla_component_inference",
            "the prepared component-inference attachment is incompatible",
        ));
    }
    match prepared.inference_unit {
        ComponentInferenceUnit::Observation => {
            if options.deletion != DeletionMode::Observation {
                return Err(BackendError::new(
                    ErrorCode::UnsupportedFeature,
                    "generic_jla_component_inference",
                    "observation component inference requires observation deletion",
                ));
            }
            if options.nuisance != NuisanceMode::Joint {
                return Err(BackendError::new(
                    ErrorCode::UnsupportedFeature,
                    "generic_jla_component_inference",
                    "observation component inference requires the full joint model operator",
                ));
            }
            if problem.frequency.iter().any(|&value| value != 1) {
                return Err(BackendError::new(
                    ErrorCode::UnsupportedFeature,
                    "generic_jla_component_inference",
                    "observation component inference requires unit frequency weights",
                ));
            }
        }
        ComponentInferenceUnit::Match => {
            if options.deletion != DeletionMode::Match
                || options.nuisance != NuisanceMode::FixedOffset
            {
                return Err(BackendError::new(
                    ErrorCode::UnsupportedFeature,
                    "generic_jla_component_inference",
                    "grouped component inference requires match deletion and nuisance(fixedoffset)",
                ));
            }
        }
    }
    if hybrid.is_some() {
        return Err(BackendError::new(
            ErrorCode::UnsupportedFeature,
            "generic_jla_component_inference",
            "eligible-stayer or mixed-deletion inference is outside the MVP",
        ));
    }
    if projection.is_some() {
        return Err(BackendError::new(
            ErrorCode::UnsupportedFeature,
            "generic_jla_component_inference",
            "simultaneous fixed-effect projection and component inference is outside the MVP",
        ));
    }
    if routing.route == ModelSolverRoute::Auto {
        return Err(BackendError::new(
            ErrorCode::UnsupportedFeature,
            "generic_jla_component_inference",
            "component inference requires an explicit diagonal or CMG solver route",
        ));
    }
    // Match movers need independent deletion blocks, even when their model
    // firm is constant. Observation inference retains its firm-based scope.
    let history = match options.deletion {
        DeletionMode::Match => &problem.row_deletion,
        DeletionMode::Observation => &problem.row_firm,
    };
    let mut first_unit = vec![None; problem.workers()];
    let mut mover = vec![false; problem.workers()];
    for (row, &unit) in history.iter().enumerate() {
        let worker = problem.row_worker[row] as usize;
        if let Some(first) = first_unit[worker] {
            mover[worker] |= first != unit;
        } else {
            first_unit[worker] = Some(unit);
        }
    }
    if mover.iter().any(|value| !value) {
        return Err(BackendError::new(
            ErrorCode::UnsupportedFeature,
            "generic_jla_component_inference",
            "component inference requires a mover-only retained sample",
        ));
    }
    Ok(())
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
    transpose_outcome_rhs_by(
        problem,
        controls,
        row_order,
        |row| weights[row] * outcome[row],
        interrupt,
    )
}

fn transpose_outcome_rhs_by(
    problem: &CompressedProblem,
    controls: &[Vec<f64>],
    row_order: &[usize],
    value_at: impl Fn(usize) -> f64,
    interrupt: &mut dyn InterruptCheck,
) -> Result<(Vec<f64>, Vec<f64>, Vec<f64>)> {
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
        let w = value_at(row);
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

fn retain_mover_match_groups(plan: &mut MatchPlan, mover_groups: usize) -> Result<()> {
    if mover_groups == 0 || mover_groups > plan.rows.len() {
        return Err(BackendError::invariant(
            "generic_jla_hybrid",
            "the mover match-group prefix is invalid",
        ));
    }
    plan.rows.truncate(mover_groups);
    plan.cell.truncate(mover_groups);
    plan.physical_count.truncate(mover_groups);
    plan.entity.truncate(mover_groups);
    Ok(())
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

fn stayer_observation_classes(
    problem: &CompressedProblem,
    row_rank: &[u64],
    stayer_rows: &[bool],
    interrupt: &mut dyn InterruptCheck,
) -> Result<Vec<ObservationClass>> {
    if stayer_rows.len() != problem.outcome.len() {
        return Err(BackendError::invariant(
            "generic_jla_hybrid",
            "the stayer observation mask has the wrong length",
        ));
    }
    let classes = observation_classes(problem, row_rank, interrupt)?;
    let mut output = Vec::new();
    reserve_exact(
        &mut output,
        classes.len(),
        "hybrid stayer observation classes",
    )?;
    for (position, class) in classes.into_iter().enumerate() {
        checkpoint_chunk(interrupt, position, "generic_jla_hybrid_classes")?;
        let rows: Vec<usize> = class
            .rows
            .into_iter()
            .filter(|&row| stayer_rows[row])
            .collect();
        if rows.is_empty() {
            continue;
        }
        let physical_count = rows.iter().try_fold(0_u64, |total, &row| {
            total
                .checked_add(problem.frequency[row])
                .ok_or_else(|| resource("hybrid stayer physical count overflow"))
        })?;
        preflight_trials(physical_count, "hybrid stayer observation class")?;
        output.push(ObservationClass {
            rows,
            entity: class.entity,
            physical_count,
        });
    }
    if output.is_empty() {
        return Err(BackendError::invariant(
            "generic_jla_hybrid",
            "the hybrid contains no stayer observation classes",
        ));
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

/// Outcome-free exact-design classes for deterministic variance-regression
/// folds. Deletion identifiers, outcomes, and optional probe-order variables
/// are intentionally excluded: they are neither conditioning variables in
/// the registered variance model nor permitted to leak response information
/// into the fold assignment.
fn structured_variance_row_ranks(
    problem: &CompressedProblem,
    controls: &[Vec<f64>],
    interrupt: &mut dyn InterruptCheck,
) -> Result<Vec<u64>> {
    let rows = problem.outcome.len();
    let mut order = index_vector(rows, interrupt, "structured_variance_semantic_sort")?;
    stable_sort_by_with_interrupt(
        &mut order,
        |&left, &right| compare_structured_variance_rows(problem, controls, left, right),
        interrupt,
        "structured_variance_semantic_sort",
    )?;
    let mut rank = repeated(
        rows,
        0_u64,
        "structured variance design-class ranks",
        interrupt,
        "structured_variance_semantic_allocate",
    )?;
    let mut current = 0_u64;
    let mut previous: Option<usize> = None;
    for (position, row) in order.into_iter().enumerate() {
        checkpoint_chunk(interrupt, position, "structured_variance_semantic_rank")?;
        if previous.is_none_or(|prior| {
            compare_structured_variance_rows(problem, controls, prior, row) != Ordering::Equal
        }) {
            current = current
                .checked_add(1)
                .ok_or_else(|| resource("structured variance design-class rank overflow"))?;
        }
        rank[row] = current;
        previous = Some(row);
    }
    Ok(rank)
}

fn compare_structured_variance_rows(
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
        });
    for column in controls {
        ordering = ordering
            .then_with(|| canonical_zero(column[left]).total_cmp(&canonical_zero(column[right])));
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
    hybrid: Option<&ExactStayerHybridPlan>,
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
            let match_groups =
                hybrid.map_or(problem.deletion_units(), |plan| plan.mover_deletion_units);
            for group in 0..match_groups {
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
            if let Some(plan) = hybrid {
                for (position, &row) in row_order.iter().enumerate() {
                    checkpoint_chunk(interrupt, position, "generic_jla_rank_stayer")?;
                    if plan.stayer_rows[row] {
                        certify_observation_deletion_rank(
                            problem,
                            row,
                            &transformed,
                            &transformed_cell_sum,
                            &cell_frequency,
                            &checked,
                            q,
                            threshold,
                            options,
                            &mut maximum_loss,
                            &mut minimum_deleted,
                            interrupt,
                            "generic_jla_rank_stayer",
                        )?;
                    }
                }
            }
        }
        DeletionMode::Observation => {
            for (position, &row) in row_order.iter().enumerate() {
                checkpoint_chunk(interrupt, position, "generic_jla_rank_observation")?;
                certify_observation_deletion_rank(
                    problem,
                    row,
                    &transformed,
                    &transformed_cell_sum,
                    &cell_frequency,
                    &checked,
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
fn certify_observation_deletion_rank(
    problem: &CompressedProblem,
    row: usize,
    transformed: &[Vec<f64>],
    transformed_cell_sum: &[f64],
    cell_frequency: &[f64],
    checked: &[f64],
    q: usize,
    threshold: f64,
    options: GenericJlaOptions,
    maximum_loss: &mut f64,
    minimum_deleted: &mut f64,
    interrupt: &mut dyn InterruptCheck,
    phase: &'static str,
) -> Result<()> {
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
                - (transformed_cell_sum[cell * q + left] - transformed[left][row]) / remaining;
            for right in 0..q {
                checkpoint_chunk(interrupt, left * q + right, phase)?;
                let right_gap = transformed[right][row]
                    - (transformed_cell_sum[cell * q + right] - transformed[right][row])
                        / remaining;
                loss[left * q + right] = remaining / cell_frequency[cell] * left_gap * right_gap;
            }
        }
    }
    certify_deleted_scatter(
        checked,
        &loss,
        q,
        threshold,
        options,
        maximum_loss,
        minimum_deleted,
        interrupt,
        phase,
    )
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
    let _profile = ProfileScope::new(ProfilePhase::Leverage);
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
        crate::progress::advance(crate::progress::LEVERAGE, first, options.probes as usize);
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
        fill_probe_atoms(
            fe_solver,
            rng,
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
        fe_solver.statistical_work(
            worker_rhs
                .chunks_mut(problem.workers())
                .zip(firm_rhs.chunks_mut(problem.firms())),
            "generic_jla_match_leverage_rhs",
            |column, (worker, firm), interrupt| {
                for group in 0..groups {
                    checkpoint_chunk(
                        interrupt,
                        column * groups + group,
                        "generic_jla_match_leverage_rhs",
                    )?;
                    let cell = plan.cell[group] as usize;
                    let atom = atoms[column * groups + group] as f64;
                    worker[problem.cell_worker[cell] as usize] += atom;
                    firm[problem.cell_firm[cell] as usize] += atom;
                }
                Ok(())
            },
            interrupt,
        )?;
        let solved = fe_solver.solve_batch_with_interrupt(
            &worker_rhs,
            &firm_rhs,
            &[],
            width,
            width,
            interrupt,
        )?;
        let _statistics_profile = ProfileScope::new(ProfilePhase::LeverageStatistics);
        statistical_batches::validate_predictions(fe_solver, &solved.solution, interrupt)?;
        for (column, solution) in solved.solution.iter().enumerate() {
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
        }
        statistical_batches::match_moments(
            fe_solver,
            plan,
            &solved.solution,
            &atoms,
            &mut moments,
            interrupt,
        )?;
    }
    crate::progress::advance(
        crate::progress::LEVERAGE,
        options.probes as usize,
        options.probes as usize,
    );
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
    let (moments, correlations, match_moments, relres) = observation_and_match_leverage_moments(
        problem,
        classes,
        None,
        fe_solver,
        rng,
        options,
        rhs_receipts,
        interrupt,
    )?;
    debug_assert!(match_moments.is_none());
    Ok((moments, correlations, relres))
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn hybrid_leverage_moments(
    problem: &CompressedProblem,
    match_plan: &MatchPlan,
    classes: &[ObservationClass],
    fe_solver: &PreparedModelSolver<'_>,
    rng: CounterRng,
    options: GenericJlaOptions,
    rhs_receipts: &mut Vec<GenericJlaRhsReceipt>,
    interrupt: &mut dyn InterruptCheck,
) -> Result<(
    Vec<FiveMoments>,
    ObservationMoments,
    ObservationCorrelations,
    f64,
)> {
    let (moments, correlations, match_moments, relres) = observation_and_match_leverage_moments(
        problem,
        classes,
        Some(match_plan),
        fe_solver,
        rng,
        options,
        rhs_receipts,
        interrupt,
    )?;
    Ok((
        match_moments.ok_or_else(|| {
            BackendError::invariant("generic_jla_hybrid", "hybrid match moments are missing")
        })?,
        moments,
        correlations,
        relres,
    ))
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn observation_and_match_leverage_moments(
    problem: &CompressedProblem,
    classes: &[ObservationClass],
    match_plan: Option<&MatchPlan>,
    fe_solver: &PreparedModelSolver<'_>,
    rng: CounterRng,
    options: GenericJlaOptions,
    rhs_receipts: &mut Vec<GenericJlaRhsReceipt>,
    interrupt: &mut dyn InterruptCheck,
) -> Result<(
    ObservationMoments,
    ObservationCorrelations,
    Option<Vec<FiveMoments>>,
    f64,
)> {
    let _profile = ProfileScope::new(ProfilePhase::Leverage);
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
    let mut match_moments = match match_plan {
        Some(plan) => Some(repeated(
            plan.rows.len(),
            FiveMoments::default(),
            "hybrid match leverage moments",
            interrupt,
            "generic_jla_hybrid_leverage_allocate",
        )?),
        None => None,
    };
    let mut maximum_relres = 0.0_f64;
    let addresses = statistical_batches::observation_addresses(problem, classes, interrupt)?;
    for first_probe in (0..options.probes as usize).step_by(options.leverage_batch_width) {
        crate::progress::advance(
            crate::progress::LEVERAGE,
            first_probe,
            options.probes as usize,
        );
        interrupt.checkpoint("generic_jla_observation_leverage_batch")?;
        let rhs_profile = ProfileScope::new(ProfilePhase::LeverageRhs);
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
        let mut match_atoms = Vec::new();
        if let Some(plan) = match_plan {
            match_atoms = repeated(
                checked_matrix_length(plan.rows.len(), width, "hybrid match leverage atoms")?,
                0_i64,
                "hybrid match leverage atoms",
                interrupt,
                "generic_jla_hybrid_leverage_allocate",
            )?;
            fill_probe_atoms(
                fe_solver,
                rng,
                ProbeDomain::Leverage,
                first_probe as u64,
                width,
                &plan.entity,
                &plan.physical_count,
                &mut match_atoms,
                interrupt,
            )?;
        }
        fe_solver.statistical_work(
            worker_rhs
                .chunks_mut(problem.workers())
                .zip(firm_rhs.chunks_mut(problem.firms())),
            "generic_jla_observation_leverage_rhs",
            |column, (worker, firm), interrupt| {
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
                        worker[problem.row_worker[row] as usize] += atom as f64;
                        firm[problem.row_firm[row] as usize] += atom as f64;
                    }
                    debug_assert_eq!(class_offset, class.physical_count);
                }
                if let Some(plan) = match_plan {
                    for group in 0..plan.rows.len() {
                        checkpoint_chunk(
                            interrupt,
                            column * plan.rows.len() + group,
                            "generic_jla_hybrid_match_rhs",
                        )?;
                        let cell = plan.cell[group] as usize;
                        let atom = match_atoms[column * plan.rows.len() + group] as f64;
                        worker[problem.cell_worker[cell] as usize] += atom;
                        firm[problem.cell_firm[cell] as usize] += atom;
                    }
                }
                Ok(())
            },
            interrupt,
        )?;
        drop(rhs_profile);
        let solved = fe_solver.solve_batch_with_interrupt(
            &worker_rhs,
            &firm_rhs,
            &[],
            width,
            width,
            interrupt,
        )?;
        let _statistics_profile = ProfileScope::new(ProfilePhase::LeverageStatistics);
        statistical_batches::validate_predictions(fe_solver, &solved.solution, interrupt)?;
        for (column, solution) in solved.solution.iter().enumerate() {
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
        }
        statistical_batches::observation_moments(
            fe_solver,
            &addresses,
            &solved.solution,
            &mut projection_square_sum,
            &mut projection_fourth_sum,
            &mut projection_square_correction,
            &mut projection_fourth_correction,
            interrupt,
        )?;
        statistical_batches::observation_correlations(
            fe_solver,
            &addresses,
            &row_offset,
            &solved.solution,
            rng,
            first_probe,
            &mut first_correlation,
            &mut third_correlation,
            &mut first_correction,
            &mut third_correction,
            interrupt,
        )?;
        if let (Some(plan), Some(moments)) = (match_plan, match_moments.as_mut()) {
            statistical_batches::match_moments(
                fe_solver,
                plan,
                &solved.solution,
                &match_atoms,
                moments,
                interrupt,
            )?;
        }
    }
    crate::progress::advance(
        crate::progress::LEVERAGE,
        options.probes as usize,
        options.probes as usize,
    );
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
        match_moments,
        maximum_relres,
    ))
}

#[derive(Clone, Debug)]
struct DeletedAdjustment {
    values: Vec<f64>,
    maximum_leverage: f64,
    maximum_relres: f64,
    inference_leverage: Option<Vec<f64>>,
    inference_maker_inverse: Option<Vec<f64>>,
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
    retain_inference_geometry: bool,
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
    let mut values = repeated(
        rows,
        f64::NAN,
        "observation deleted adjustments",
        interrupt,
        "generic_jla_observation_adjustment_allocate",
    )?;
    let mut active = repeated(
        rows,
        false,
        "observation active-row mask",
        interrupt,
        "generic_jla_observation_adjustment_allocate",
    )?;
    for class in _classes {
        for &row in &class.rows {
            active[row] = true;
        }
    }
    let mut maximum_leverage = 0.0_f64;
    let mut inference_leverage = retain_inference_geometry.then(|| vec![f64::NAN; rows]);
    let mut inference_maker_inverse = retain_inference_geometry.then(|| vec![f64::NAN; rows]);
    for (position, &row) in row_order.iter().enumerate() {
        checkpoint_chunk(interrupt, position, "generic_jla_observation_adjustment")?;
        if !active[row] {
            continue;
        }
        let mut inverse_sum = StableAccumulator::default();
        let mut leverage_sum = StableAccumulator::default();
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
            let total_leverage = finite.projection + control_leverage[row];
            leverage_sum.add(total_leverage);
            maximum_leverage = maximum_leverage.max(total_leverage);
        }
        let frequency = problem.frequency[row] as f64;
        let maker_inverse = inverse_sum.finish() / frequency;
        values[row] = residual[row] * maker_inverse;
        if let (Some(leverage), Some(inverse)) = (
            inference_leverage.as_mut(),
            inference_maker_inverse.as_mut(),
        ) {
            leverage[row] = leverage_sum.finish() / frequency;
            inverse[row] = maker_inverse;
        }
        if !values[row].is_finite() {
            return Err(nonfinite("observation deleted residual is nonfinite"));
        }
    }
    Ok(DeletedAdjustment {
        values,
        maximum_leverage,
        maximum_relres: 0.0,
        inference_leverage,
        inference_maker_inverse,
    })
}

fn combine_hybrid_adjustments(
    mover: &[f64],
    stayer: &[f64],
    stayer_rows: &[bool],
    interrupt: &mut dyn InterruptCheck,
) -> Result<Vec<f64>> {
    if mover.len() != stayer.len() || mover.len() != stayer_rows.len() {
        return Err(BackendError::invariant(
            "generic_jla_hybrid",
            "hybrid deleted-adjustment dimensions disagree",
        ));
    }
    let mut output = zeroed_f64_with_interrupt(
        mover.len(),
        "hybrid deleted adjustments",
        interrupt,
        "generic_jla_hybrid_adjustment_allocate",
    )?;
    for row in 0..output.len() {
        checkpoint_chunk(interrupt, row, "generic_jla_hybrid_adjustment")?;
        output[row] = if stayer_rows[row] {
            stayer[row]
        } else {
            mover[row]
        };
        if !output[row].is_finite() {
            return Err(nonfinite(
                "hybrid deleted residual is nonfinite or incomplete",
            ));
        }
    }
    Ok(output)
}

fn match_deleted_adjustment(
    problem: &CompressedProblem,
    plan: &MatchPlan,
    moments: &[FiveMoments],
    residual: &[f64],
    geometry: Option<&ControlGeometry>,
    options: GenericJlaOptions,
    retain_inference_geometry: bool,
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
    let mut inference_leverage = retain_inference_geometry.then(|| vec![f64::NAN; plan.rows.len()]);
    let mut inference_maker_inverse =
        retain_inference_geometry.then(|| vec![f64::NAN; plan.rows.len()]);
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
        if let (Some(leverage), Some(maker_inverse)) = (
            inference_leverage.as_mut(),
            inference_maker_inverse.as_mut(),
        ) {
            let effective = common_inverse + finite.bias * common_inverse.powi(2)
                - finite.variance * common_inverse.powi(3);
            if !effective.is_finite() || effective <= 0.0 || finite.residual <= 0.0 {
                return Err(BackendError::new(
                    ErrorCode::BlockInverseFailed,
                    "generic_jla_match_adjustment",
                    "the collapsed-match finite-projection maker is nonpositive",
                ));
            }
            leverage[group] = finite.projection;
            maker_inverse[group] = effective;
        }
        for (local, &row) in rows.iter().enumerate() {
            values[row] = transformed[local]
                + finite.bias * inverse_common[local] * common_transformed
                - finite.variance * inverse_common[local] * common_inverse * common_transformed;
        }
    }
    for (group, rows) in plan.rows.iter().enumerate() {
        checkpoint_chunk(interrupt, group, "generic_jla_match_adjustment_validate")?;
        for &row in rows {
            if !values[row].is_finite() {
                return Err(nonfinite(
                    "match deleted residual is nonfinite or incomplete",
                ));
            }
        }
    }
    Ok(DeletedAdjustment {
        values,
        maximum_leverage,
        maximum_relres,
        inference_leverage,
        inference_maker_inverse,
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
    diagonal: Option<[Vec<f64>; PRIMITIVE_TARGETS]>,
}

#[derive(Clone, Debug)]
struct ComponentInferenceAddresses {
    entity: Vec<u64>,
    subdraw: Vec<u64>,
}

#[derive(Clone, Copy)]
enum ComponentInferenceRows<'a> {
    Observation {
        controls: &'a [Vec<f64>],
        row_order: &'a [usize],
    },
    Match {
        plan: &'a MatchPlan,
    },
}

impl<'a> ComponentInferenceRows<'a> {
    fn len(self, problem: &CompressedProblem) -> usize {
        match self {
            Self::Observation { .. } => problem.outcome.len(),
            Self::Match { plan } => plan.rows.len(),
        }
    }

    fn controls(self) -> usize {
        match self {
            Self::Observation { controls, .. } => controls.len(),
            Self::Match { .. } => 0,
        }
    }

    fn control_columns(self) -> &'a [Vec<f64>] {
        match self {
            Self::Observation { controls, .. } => controls,
            Self::Match { .. } => &[],
        }
    }
}

#[derive(Clone, Debug)]
struct CollapsedMatchComponentData {
    outcome: Vec<f64>,
    residual: Vec<f64>,
    deleted_adjusted: Vec<f64>,
    mass: Vec<f64>,
    target_diagonal: [Vec<f64>; PRIMITIVE_TARGETS],
}

#[derive(Clone, Copy, Debug)]
struct ComponentInferenceCounterPlan {
    logical_atoms: u64,
    unique_words: u64,
}

fn plan_component_inference_counter(
    prepared: &ComponentExecution<'_>,
    classes: &[ObservationClass],
    structured_class_count: Option<u64>,
) -> Result<ComponentInferenceCounterPlan> {
    let mut atoms_per_probe = 0_u64;
    let mut words_per_probe = 0_u64;
    for class in classes {
        let words = class
            .physical_count
            .checked_mul(2)
            .ok_or_else(|| resource("component-inference Gaussian word count overflow"))?;
        if words > MAX_PHYSICAL_WORDS_PER_ATOM {
            return Err(BackendError::new(
                ErrorCode::ResourceLimit,
                "generic_jla_component_counter",
                format!(
                    "a component-inference semantic entity requires {words} Gaussian words, above the registered limit {MAX_PHYSICAL_WORDS_PER_ATOM}"
                ),
            ));
        }
        atoms_per_probe = atoms_per_probe
            .checked_add(class.physical_count)
            .ok_or_else(|| resource("component-inference Gaussian atom count overflow"))?;
        words_per_probe = words_per_probe
            .checked_add(words)
            .ok_or_else(|| resource("component-inference Gaussian word count overflow"))?;
    }
    let probes = u64::from(prepared.options.probes)
        .checked_add(u64::from(prepared.options.spectrum_probes))
        .and_then(|value| value.checked_add(2))
        .and_then(|value| {
            value.checked_add(prepared.residual_moments.map_or(0, |fit| fit.probes as u64))
        })
        .ok_or_else(|| resource("component-inference total probe count overflow"))?;
    let row_atoms = atoms_per_probe
        .checked_mul(probes)
        .ok_or_else(|| resource("component-inference logical Counter atom overflow"))?;
    let row_words = words_per_probe
        .checked_mul(probes)
        .ok_or_else(|| resource("component-inference unique Counter word overflow"))?;
    let (critical_atoms, critical_words) =
        if prepared.options.reference_distribution == ComponentReferenceDistribution::Q1 {
            let target_simulations = u64::from(prepared.options.critical_simulations)
                .checked_mul(REPORTED_TARGETS as u64)
                .ok_or_else(|| resource("q=1 critical simulation count overflow"))?;
            (
                target_simulations
                    .checked_mul(2)
                    .ok_or_else(|| resource("q=1 critical Counter atom overflow"))?,
                target_simulations
                    .checked_mul(4)
                    .ok_or_else(|| resource("q=1 critical Counter word overflow"))?,
            )
        } else {
            (0, 0)
        };
    let structured_atoms = if prepared.residual_moments.is_some() {
        if structured_class_count.is_some() {
            return Err(invalid("residual moments must not retain CV fold classes"));
        }
        0
    } else {
        match prepared.variance_source {
            ComponentVarianceSource::Oracle => {
                if structured_class_count.is_some() {
                    return Err(BackendError::invariant(
                        "generic_jla_component_counter",
                        "oracle component inference unexpectedly retained structured fold classes",
                    ));
                }
                0
            }
            ComponentVarianceSource::StructuredCommon
            | ComponentVarianceSource::StructuredLeverage => structured_class_count
                .ok_or_else(|| {
                    BackendError::invariant(
                        "generic_jla_component_counter",
                        "structured component inference is missing its design-class count",
                    )
                })?
                .checked_mul(STRUCTURED_OUTER_FOLDS as u64)
                .ok_or_else(|| resource("structured variance fold Counter atom overflow"))?,
        }
    };
    Ok(ComponentInferenceCounterPlan {
        logical_atoms: row_atoms
            .checked_add(critical_atoms)
            .and_then(|value| value.checked_add(structured_atoms))
            .ok_or_else(|| resource("component-inference total Counter atom overflow"))?,
        unique_words: row_words
            .checked_add(critical_words)
            .and_then(|value| value.checked_add(structured_atoms))
            .ok_or_else(|| resource("component-inference total Counter word overflow"))?,
    })
}

fn observation_inference_addresses(
    problem: &CompressedProblem,
    classes: &[ObservationClass],
    interrupt: &mut dyn InterruptCheck,
) -> Result<ComponentInferenceAddresses> {
    let rows = problem.outcome.len();
    let mut entity = repeated(
        rows,
        u64::MAX,
        "component-inference entity addresses",
        interrupt,
        "generic_jla_component_address_allocate",
    )?;
    let mut subdraw = repeated(
        rows,
        u64::MAX,
        "component-inference subdraw addresses",
        interrupt,
        "generic_jla_component_address_allocate",
    )?;
    let mut covered = 0_usize;
    for (class_index, class) in classes.iter().enumerate() {
        checkpoint_chunk(interrupt, class_index, "generic_jla_component_addresses")?;
        let mut offset = 0_u64;
        for &row in &class.rows {
            if problem.frequency[row] != 1 || entity[row] != u64::MAX {
                return Err(BackendError::invariant(
                    "generic_jla_component_addresses",
                    "component-inference observation addressing requires unique unit-frequency rows",
                ));
            }
            entity[row] = class.entity;
            subdraw[row] = offset;
            offset = offset
                .checked_add(1)
                .ok_or_else(|| resource("component-inference subdraw offset overflow"))?;
            covered = covered
                .checked_add(1)
                .ok_or_else(|| resource("component-inference row accounting overflow"))?;
        }
        if offset != class.physical_count {
            return Err(BackendError::invariant(
                "generic_jla_component_addresses",
                "component-inference address count does not match its semantic class",
            ));
        }
    }
    if covered != rows || entity.contains(&u64::MAX) || subdraw.contains(&u64::MAX) {
        return Err(BackendError::invariant(
            "generic_jla_component_addresses",
            "component-inference observation addresses are incomplete",
        ));
    }
    Ok(ComponentInferenceAddresses { entity, subdraw })
}

fn grouped_component_addresses_and_folds(
    plan: &MatchPlan,
    interrupt: &mut dyn InterruptCheck,
) -> Result<(ComponentInferenceAddresses, Vec<u64>)> {
    let groups = plan.rows.len();
    let mut order = index_vector(groups, interrupt, "grouped_component_design_order")?;
    stable_sort_by_with_interrupt(
        &mut order,
        |&left, &right| {
            plan.cell[left]
                .cmp(&plan.cell[right])
                .then_with(|| plan.physical_count[left].cmp(&plan.physical_count[right]))
                .then_with(|| left.cmp(&right))
        },
        interrupt,
        "grouped_component_design_order",
    )?;
    let mut fold_entity = vec![0_u64; groups];
    let mut subdraw = vec![0_u64; groups];
    let mut entity = vec![0_u64; groups];
    let mut class = 0_u64;
    let mut prior: Option<usize> = None;
    let mut within_class = 0_u64;
    for (position, &group) in order.iter().enumerate() {
        checkpoint_chunk(interrupt, position, "grouped_component_design_classes")?;
        let changed = prior.is_none_or(|previous| {
            plan.cell[previous] != plan.cell[group]
                || plan.physical_count[previous] != plan.physical_count[group]
        });
        if changed {
            class = class
                .checked_add(1)
                .ok_or_else(|| resource("grouped component design-class count overflow"))?;
            within_class = 0;
        }
        fold_entity[group] = class;
        entity[group] = class;
        subdraw[group] = within_class;
        within_class = within_class
            .checked_add(1)
            .ok_or_else(|| resource("grouped component subdraw overflow"))?;
        prior = Some(group);
    }
    Ok((ComponentInferenceAddresses { entity, subdraw }, fold_entity))
}

fn plan_grouped_component_inference_counter(
    prepared: &ComponentExecution<'_>,
    addresses: &ComponentInferenceAddresses,
    structured_class_count: Option<u64>,
) -> Result<ComponentInferenceCounterPlan> {
    let groups = u64::try_from(addresses.entity.len())
        .map_err(|_| resource("grouped component-inference match count"))?;
    if addresses.subdraw.len() != addresses.entity.len() {
        return Err(BackendError::invariant(
            "generic_jla_component_counter",
            "grouped component-inference Counter addresses disagree",
        ));
    }
    for &subdraw in &addresses.subdraw {
        let words = subdraw
            .checked_add(1)
            .and_then(|value| value.checked_mul(2))
            .ok_or_else(|| resource("grouped component-inference Gaussian word count"))?;
        if words > MAX_PHYSICAL_WORDS_PER_ATOM {
            return Err(BackendError::new(
                ErrorCode::ResourceLimit,
                "generic_jla_component_counter",
                "a grouped component design class exceeds the registered Gaussian word limit",
            ));
        }
    }
    let probes = u64::from(prepared.options.probes)
        .checked_add(u64::from(prepared.options.spectrum_probes))
        .and_then(|value| value.checked_add(2))
        .and_then(|value| {
            value.checked_add(prepared.residual_moments.map_or(0, |fit| fit.probes as u64))
        })
        .ok_or_else(|| resource("grouped component-inference total probe count"))?;
    let row_atoms = groups
        .checked_mul(probes)
        .ok_or_else(|| resource("grouped component-inference Counter atom count"))?;
    let row_words = row_atoms
        .checked_mul(2)
        .ok_or_else(|| resource("grouped component-inference Counter word count"))?;
    let (critical_atoms, critical_words) =
        if prepared.options.reference_distribution == ComponentReferenceDistribution::Q1 {
            let target_simulations = u64::from(prepared.options.critical_simulations)
                .checked_mul(REPORTED_TARGETS as u64)
                .ok_or_else(|| resource("grouped q=1 critical simulation count"))?;
            (
                target_simulations
                    .checked_mul(2)
                    .ok_or_else(|| resource("grouped q=1 critical Counter atom count"))?,
                target_simulations
                    .checked_mul(4)
                    .ok_or_else(|| resource("grouped q=1 critical Counter word count"))?,
            )
        } else {
            (0, 0)
        };
    let structured_atoms = if prepared.residual_moments.is_some() {
        if structured_class_count.is_some() {
            return Err(invalid(
                "residual-moment match inference retained CV classes",
            ));
        }
        0
    } else {
        match prepared.variance_source {
            ComponentVarianceSource::Oracle => {
                if structured_class_count.is_some() {
                    return Err(BackendError::invariant(
                        "generic_jla_component_counter",
                        "grouped oracle inference unexpectedly retained structured fold classes",
                    ));
                }
                0
            }
            ComponentVarianceSource::StructuredCommon
            | ComponentVarianceSource::StructuredLeverage => structured_class_count
                .ok_or_else(|| {
                    BackendError::invariant(
                        "generic_jla_component_counter",
                        "grouped structured inference is missing design classes",
                    )
                })?
                .checked_mul(STRUCTURED_OUTER_FOLDS as u64)
                .ok_or_else(|| resource("grouped structured fold Counter atom count"))?,
        }
    };
    Ok(ComponentInferenceCounterPlan {
        logical_atoms: row_atoms
            .checked_add(critical_atoms)
            .and_then(|value| value.checked_add(structured_atoms))
            .ok_or_else(|| resource("grouped component-inference total Counter atoms"))?,
        unique_words: row_words
            .checked_add(critical_words)
            .and_then(|value| value.checked_add(structured_atoms))
            .ok_or_else(|| resource("grouped component-inference total Counter words"))?,
    })
}

fn collapse_match_component_data(
    problem: &CompressedProblem,
    plan: &MatchPlan,
    working_y: &[f64],
    residual: &[f64],
    deleted_adjusted: &[f64],
    target_diagonal: &[Vec<f64>; PRIMITIVE_TARGETS],
    interrupt: &mut dyn InterruptCheck,
) -> Result<CollapsedMatchComponentData> {
    let rows = problem.outcome.len();
    if working_y.len() != rows
        || residual.len() != rows
        || deleted_adjusted.len() != rows
        || target_diagonal.iter().any(|column| column.len() != rows)
    {
        return Err(BackendError::invariant(
            "grouped_component_collapse",
            "collapsed-match source arrays have incompatible dimensions",
        ));
    }
    let groups = plan.rows.len();
    let mut outcome = vec![0.0; groups];
    let mut collapsed_residual = vec![0.0; groups];
    let mut collapsed_adjusted = vec![0.0; groups];
    let mut mass = vec![0.0; groups];
    let mut diagonal: [Vec<f64>; PRIMITIVE_TARGETS] = core::array::from_fn(|_| vec![0.0; groups]);
    for group in 0..groups {
        checkpoint_chunk(interrupt, group, "grouped_component_collapse")?;
        let root_mass = (plan.physical_count[group] as f64).sqrt();
        if !root_mass.is_finite() || root_mass <= 0.0 {
            return Err(BackendError::invariant(
                "grouped_component_collapse",
                "a declared match has nonpositive regression mass",
            ));
        }
        let mut y_sum = StableAccumulator::default();
        let mut residual_sum = StableAccumulator::default();
        let mut adjusted_sum = StableAccumulator::default();
        let mut diagonal_sum = [StableAccumulator::default(); PRIMITIVE_TARGETS];
        for (local, &row) in plan.rows[group].iter().enumerate() {
            checkpoint_chunk(interrupt, local, "grouped_component_collapse_rows")?;
            let frequency = problem.frequency[row] as f64;
            y_sum.add(frequency * working_y[row]);
            residual_sum.add(frequency * residual[row]);
            adjusted_sum.add(frequency.sqrt() * deleted_adjusted[row]);
            for target in 0..PRIMITIVE_TARGETS {
                diagonal_sum[target].add(frequency * target_diagonal[target][row]);
            }
        }
        outcome[group] = y_sum.finish() / root_mass;
        collapsed_residual[group] = residual_sum.finish() / root_mass;
        collapsed_adjusted[group] = adjusted_sum.finish() / root_mass;
        mass[group] = plan.physical_count[group] as f64;
        for target in 0..PRIMITIVE_TARGETS {
            diagonal[target][group] = diagonal_sum[target].finish();
        }
        if [
            outcome[group],
            collapsed_residual[group],
            collapsed_adjusted[group],
            mass[group],
        ]
        .iter()
        .any(|value| !value.is_finite())
            || diagonal.iter().any(|column| !column[group].is_finite())
        {
            return Err(nonfinite(
                "collapsed-match component statistic is nonfinite",
            ));
        }
    }
    Ok(CollapsedMatchComponentData {
        outcome,
        residual: collapsed_residual,
        deleted_adjusted: collapsed_adjusted,
        mass,
        target_diagonal: diagonal,
    })
}

fn component_transpose_rhs(
    problem: &CompressedProblem,
    rows: ComponentInferenceRows<'_>,
    outcome: &[f64],
    interrupt: &mut dyn InterruptCheck,
) -> Result<(Vec<f64>, Vec<f64>, Vec<f64>)> {
    if outcome.len() != rows.len(problem) {
        return Err(BackendError::invariant(
            "generic_jla_component_inference",
            "component inference outcome has the wrong unit count",
        ));
    }
    component_transpose_rhs_by(problem, rows, |row| outcome[row], interrupt)
}

fn component_transpose_rhs_by(
    problem: &CompressedProblem,
    rows: ComponentInferenceRows<'_>,
    value_at: impl Fn(usize) -> f64,
    interrupt: &mut dyn InterruptCheck,
) -> Result<(Vec<f64>, Vec<f64>, Vec<f64>)> {
    match rows {
        ComponentInferenceRows::Observation {
            controls,
            row_order,
        } => transpose_outcome_rhs_by(problem, controls, row_order, value_at, interrupt),
        ComponentInferenceRows::Match { plan } => {
            let mut worker = zeroed_f64_with_interrupt(
                problem.workers(),
                "component worker RHS",
                interrupt,
                "grouped_component_transpose",
            )?;
            let mut firm = zeroed_f64_with_interrupt(
                problem.firms(),
                "component firm RHS",
                interrupt,
                "grouped_component_transpose",
            )?;
            for group in 0..plan.rows.len() {
                checkpoint_chunk(interrupt, group, "grouped_component_transpose")?;
                let cell = plan.cell[group] as usize;
                let value = (plan.physical_count[group] as f64).sqrt() * value_at(group);
                worker[problem.cell_worker[cell] as usize] += value;
                firm[problem.cell_firm[cell] as usize] += value;
            }
            Ok((worker, firm, Vec::new()))
        }
    }
}

fn component_predict(
    problem: &CompressedProblem,
    rows: ComponentInferenceRows<'_>,
    solver: &PreparedModelSolver<'_>,
    coefficients: &ModelCoefficients,
    output: &mut [f64],
    interrupt: &mut dyn InterruptCheck,
) -> Result<()> {
    if output.len() != rows.len(problem) {
        return Err(BackendError::invariant(
            "generic_jla_component_inference",
            "component prediction has the wrong unit count",
        ));
    }
    match rows {
        ComponentInferenceRows::Observation { .. } => {
            solver.operator().predict_into_with_interrupt(
                &coefficients.worker,
                &coefficients.firm,
                &coefficients.control,
                output,
                interrupt,
            )
        }
        ComponentInferenceRows::Match { plan } => {
            if !coefficients.control.is_empty() {
                return Err(BackendError::invariant(
                    "grouped_component_predict",
                    "fixed-offset grouped inference received control coefficients",
                ));
            }
            for group in 0..plan.rows.len() {
                checkpoint_chunk(interrupt, group, "grouped_component_predict")?;
                let cell = plan.cell[group] as usize;
                output[group] = (plan.physical_count[group] as f64).sqrt()
                    * (coefficients.worker[problem.cell_worker[cell] as usize]
                        + coefficients.firm[problem.cell_firm[cell] as usize]);
                if !output[group].is_finite() {
                    return Err(nonfinite("collapsed-match prediction is nonfinite"));
                }
            }
            Ok(())
        }
    }
}

/// Write a prediction directly in the residual-probe permutation, avoiding
/// one N-vector per active column. No summation order changes within a row.
fn component_predict_in_order(
    problem: &CompressedProblem,
    rows: ComponentInferenceRows<'_>,
    solver: &PreparedModelSolver<'_>,
    coefficients: &ModelCoefficients,
    order: &[usize],
    output: &mut [f64],
    interrupt: &mut dyn InterruptCheck,
) -> Result<()> {
    if output.len() != order.len() || order.len() != rows.len(problem) {
        return Err(invalid("permuted component prediction dimensions disagree"));
    }
    match rows {
        ComponentInferenceRows::Observation { .. } => {
            solver.operator().validate_prediction_coefficients(
                &coefficients.worker,
                &coefficients.firm,
                &coefficients.control,
                interrupt,
            )?;
            for (position, &row) in order.iter().enumerate() {
                checkpoint_chunk(interrupt, position, "residual_moment_prediction")?;
                output[position] = solver.operator().prediction_at(
                    &coefficients.worker,
                    &coefficients.firm,
                    &coefficients.control,
                    row,
                )?;
            }
        }
        ComponentInferenceRows::Match { plan } => {
            if !coefficients.control.is_empty()
                || coefficients.worker.len() != problem.workers()
                || coefficients.firm.len() != problem.firms()
            {
                return Err(invalid("permuted match prediction coefficients disagree"));
            }
            for (position, &group) in order.iter().enumerate() {
                checkpoint_chunk(interrupt, position, "residual_moment_prediction")?;
                let cell = plan.cell[group] as usize;
                output[position] = (plan.physical_count[group] as f64).sqrt()
                    * (coefficients.worker[problem.cell_worker[cell] as usize]
                        + coefficients.firm[problem.cell_firm[cell] as usize]);
                if !output[position].is_finite() {
                    return Err(nonfinite("collapsed-match prediction is nonfinite"));
                }
            }
        }
    }
    Ok(())
}

#[derive(Clone, Debug)]
struct ComponentSpectrumVector {
    coefficients: ModelCoefficients,
    prediction: Vec<f64>,
}

#[derive(Clone, Debug)]
struct ComponentSpectrumResult {
    diagnostics: [ComponentSpectrumDiagnostics; REPORTED_TARGETS],
    leading_mode: [ComponentSpectrumVector; REPORTED_TARGETS],
}

#[derive(Clone, Copy, Debug, Default)]
struct ComponentTraceMoments {
    count: u64,
    sum: StableAccumulator,
    square: StableAccumulator,
}

impl ComponentTraceMoments {
    fn push(&mut self, value: f64) -> Result<()> {
        if !value.is_finite() || value < 0.0 {
            return Err(BackendError::new(
                ErrorCode::JlaConstraintFailed,
                "component_inference_spectrum",
                "a target trace-square probe was negative or nonfinite",
            ));
        }
        self.count = self
            .count
            .checked_add(1)
            .ok_or_else(|| resource("component spectrum probe count overflow"))?;
        self.sum.add(value);
        self.square.add(value * value);
        Ok(())
    }

    fn finish(self) -> Result<(f64, f64)> {
        if self.count < 2 {
            return Err(invalid(
                "at least two component spectrum probes are required",
            ));
        }
        let count = self.count as f64;
        let mean = self.sum.finish() / count;
        let sample_variance =
            ((self.square.finish() - count * mean * mean) / (count - 1.0)).max(0.0);
        let mcse = (sample_variance / count).sqrt();
        if !mean.is_finite() || mean <= 0.0 || !mcse.is_finite() {
            return Err(BackendError::new(
                ErrorCode::JlaConstraintFailed,
                "component_inference_spectrum",
                "the randomized target trace square is zero or nonfinite",
            ));
        }
        Ok((mean, mcse))
    }
}

fn component_coefficient_dot_rhs(
    coefficients: &ModelCoefficients,
    worker_rhs: &[f64],
    firm_rhs: &[f64],
    control_rhs: &[f64],
) -> Result<f64> {
    if coefficients.worker.len() != worker_rhs.len()
        || coefficients.firm.len() != firm_rhs.len()
        || coefficients.control.len() != control_rhs.len()
    {
        return Err(BackendError::invariant(
            "component_inference_spectrum",
            "a target-spectrum coefficient/RHS pair has inconsistent dimensions",
        ));
    }
    let mut value = StableAccumulator::default();
    for (&coefficient, &rhs) in coefficients.worker.iter().zip(worker_rhs) {
        value.add(coefficient * rhs);
    }
    for (&coefficient, &rhs) in coefficients.firm.iter().zip(firm_rhs) {
        value.add(coefficient * rhs);
    }
    for (&coefficient, &rhs) in coefficients.control.iter().zip(control_rhs) {
        value.add(coefficient * rhs);
    }
    let value = value.finish();
    if !value.is_finite() {
        return Err(nonfinite(
            "component target-spectrum inner product is nonfinite",
        ));
    }
    Ok(value)
}

fn component_prediction_inner(left: &[f64], right: &[f64]) -> Result<f64> {
    if left.len() != right.len() {
        return Err(BackendError::invariant(
            "component_inference_spectrum",
            "target-spectrum predictions have inconsistent dimensions",
        ));
    }
    let mut value = StableAccumulator::default();
    for (&left, &right) in left.iter().zip(right) {
        value.add(left * right);
    }
    let value = value.finish();
    if !value.is_finite() {
        return Err(nonfinite("component target-spectrum norm is nonfinite"));
    }
    Ok(value)
}

fn component_linear_combination(
    left: &ComponentSpectrumVector,
    right: &ComponentSpectrumVector,
    left_scale: f64,
    right_scale: f64,
) -> Result<ComponentSpectrumVector> {
    if left.prediction.len() != right.prediction.len()
        || left.coefficients.worker.len() != right.coefficients.worker.len()
        || left.coefficients.firm.len() != right.coefficients.firm.len()
        || left.coefficients.control.len() != right.coefficients.control.len()
    {
        return Err(BackendError::invariant(
            "component_inference_spectrum",
            "target-spectrum vectors have inconsistent dimensions",
        ));
    }
    let combine = |left: &[f64], right: &[f64]| {
        left.iter()
            .zip(right)
            .map(|(&left, &right)| left_scale.mul_add(left, right_scale * right))
            .collect::<Vec<_>>()
    };
    let output = ComponentSpectrumVector {
        coefficients: ModelCoefficients {
            worker: combine(&left.coefficients.worker, &right.coefficients.worker),
            firm: combine(&left.coefficients.firm, &right.coefficients.firm),
            control: combine(&left.coefficients.control, &right.coefficients.control),
        },
        prediction: combine(&left.prediction, &right.prediction),
    };
    if output
        .prediction
        .iter()
        .chain(&output.coefficients.worker)
        .chain(&output.coefficients.firm)
        .chain(&output.coefficients.control)
        .any(|value| !value.is_finite())
    {
        return Err(nonfinite("component target-spectrum vector is nonfinite"));
    }
    Ok(output)
}

fn component_normalize(mut value: ComponentSpectrumVector) -> Result<ComponentSpectrumVector> {
    let norm_square = component_prediction_inner(&value.prediction, &value.prediction)?;
    if norm_square <= f64::MIN_POSITIVE {
        return Err(BackendError::new(
            ErrorCode::JlaConstraintFailed,
            "component_inference_spectrum",
            "a target-spectrum iteration lost numerical rank",
        ));
    }
    let inverse = norm_square.sqrt().recip();
    for entry in value
        .prediction
        .iter_mut()
        .chain(&mut value.coefficients.worker)
        .chain(&mut value.coefficients.firm)
        .chain(&mut value.coefficients.control)
    {
        *entry *= inverse;
    }
    if let Some((_, &orientation)) = value.prediction.iter().enumerate().max_by(|left, right| {
        left.1
            .abs()
            .partial_cmp(&right.1.abs())
            .unwrap_or(Ordering::Equal)
            .then_with(|| right.0.cmp(&left.0))
    }) {
        if orientation < 0.0 {
            for entry in value
                .prediction
                .iter_mut()
                .chain(&mut value.coefficients.worker)
                .chain(&mut value.coefficients.firm)
                .chain(&mut value.coefficients.control)
            {
                *entry = -*entry;
            }
        }
    }
    Ok(value)
}

fn component_orthonormalize_pair(
    value: [ComponentSpectrumVector; 2],
) -> Result<[ComponentSpectrumVector; 2]> {
    let [first, second] = value;
    let first = component_normalize(first)?;
    let projection = component_prediction_inner(&first.prediction, &second.prediction)?;
    let second = component_linear_combination(&second, &first, 1.0, -projection)?;
    Ok([first, component_normalize(second)?])
}

#[allow(clippy::too_many_arguments)]
fn component_apply_target_pair(
    problem: &CompressedProblem,
    inference_rows: ComponentInferenceRows<'_>,
    solver: &PreparedModelSolver<'_>,
    target: usize,
    input: &[ComponentSpectrumVector; 2],
    phase_index: u32,
    solve_receipts: &mut Vec<ComponentInferenceSolveReceipt>,
    interrupt: &mut dyn InterruptCheck,
) -> Result<[ComponentSpectrumVector; 2]> {
    let controls = inference_rows.controls();
    let mut worker_rhs = vec![0.0; 2 * problem.workers()];
    let mut firm_rhs = vec![0.0; 2 * problem.firms()];
    let mut control_rhs = vec![0.0; 2 * controls];
    for column in 0..2 {
        let rhs = reported_target_rhs(problem, &input[column].coefficients, target, interrupt)?;
        worker_rhs[column * problem.workers()..(column + 1) * problem.workers()]
            .copy_from_slice(&rhs.0);
        firm_rhs[column * problem.firms()..(column + 1) * problem.firms()].copy_from_slice(&rhs.1);
        if controls > 0 {
            control_rhs[column * controls..(column + 1) * controls].copy_from_slice(&rhs.2);
        }
    }
    let solved =
        solver.solve_batch_with_interrupt(&worker_rhs, &firm_rhs, &control_rhs, 2, 2, interrupt)?;
    let mut output = Vec::with_capacity(2);
    for (column, solution) in solved.solution.into_iter().enumerate() {
        let mut prediction = vec![0.0; inference_rows.len(problem)];
        component_predict(
            problem,
            inference_rows,
            solver,
            &solution.coefficients,
            &mut prediction,
            interrupt,
        )?;
        solve_receipts.push(component_solve_receipt(
            ComponentInferenceSolvePhase::SpectrumIteration,
            phase_index
                .checked_mul(2)
                .and_then(|value| value.checked_add(column as u32))
                .ok_or_else(|| resource("component spectrum receipt index overflow"))?,
            &solution,
        ));
        output.push(ComponentSpectrumVector {
            coefficients: solution.coefficients,
            prediction,
        });
    }
    output.try_into().map_err(|_| {
        BackendError::invariant(
            "component_inference_spectrum",
            "a two-column target application returned the wrong width",
        )
    })
}

fn component_ritz_rotate(
    basis: &[ComponentSpectrumVector; 2],
    action: &[ComponentSpectrumVector; 2],
) -> Result<[ComponentSpectrumVector; 2]> {
    let first = component_prediction_inner(&basis[0].prediction, &action[0].prediction)?;
    let second = 0.5
        * (component_prediction_inner(&basis[0].prediction, &action[1].prediction)?
            + component_prediction_inner(&basis[1].prediction, &action[0].prediction)?);
    let fourth = component_prediction_inner(&basis[1].prediction, &action[1].prediction)?;
    let center = 0.5 * (first + fourth);
    let radius = (0.25 * (first - fourth) * (first - fourth) + second * second).sqrt();
    let plus = center + radius;
    let minus = center - radius;
    let eigenvector = |eigenvalue: f64| {
        // The two equivalent null-row formulas are (b, lambda-a) and
        // (lambda-d, b). Use the larger one: near a diagonal matrix the
        // smaller formula subtracts nearly equal eigenvalues, magnifying
        // rounding into a spurious rotation and failed mode certification.
        let (mut left, mut right) = if (eigenvalue - first).abs() >= (eigenvalue - fourth).abs() {
            (second, eigenvalue - first)
        } else {
            (eigenvalue - fourth, second)
        };
        let norm = left.hypot(right);
        if norm <= f64::MIN_POSITIVE {
            if (eigenvalue - first).abs() <= (eigenvalue - fourth).abs() {
                left = 1.0;
                right = 0.0;
            } else {
                left = 0.0;
                right = 1.0;
            }
        } else {
            left /= norm;
            right /= norm;
        }
        (left, right)
    };
    let selected = if plus.abs() >= minus.abs() {
        plus
    } else {
        minus
    };
    let (left, right) = eigenvector(selected);
    let leading = component_linear_combination(&basis[0], &basis[1], left, right)?;
    let second = component_linear_combination(&basis[0], &basis[1], -right, left)?;
    component_orthonormalize_pair([leading, second])
}

#[test]
fn component_ritz_nearly_diagonal_signed_spectra_keep_small_residuals() {
    let vector = |prediction: Vec<f64>| ComponentSpectrumVector {
        coefficients: ModelCoefficients {
            worker: prediction.clone(),
            firm: vec![],
            control: vec![],
        },
        prediction,
    };
    let basis = [vector(vec![1.0, 0.0]), vector(vec![0.0, 1.0])];
    for (a, d) in [
        (0.21921914782479462, 0.03178002726656686),
        (0.03178002726656686, 0.21921914782479462),
        (-0.21921914782479462, 0.03178002726656686),
        (0.03178002726656686, -0.21921914782479462),
        (0.25, 0.25),
        (-0.25, 0.25),
    ] {
        for b in [0.0, 1e-18, 2e-17, -2e-17, 1e-10, -0.03] {
            let actions = [vector(vec![a, b]), vector(vec![b, d])];
            let modes = component_ritz_rotate(&basis, &actions).unwrap();
            for mode in modes {
                let x = mode.prediction[0];
                let y = mode.prediction[1];
                let ax = a * x + b * y;
                let ay = b * x + d * y;
                let lambda = x * ax + y * ay;
                let residual = (ax - lambda * x).hypot(ay - lambda * y);
                assert!(
                    residual <= 2e-15,
                    "matrix=({a},{b},{d}) vector=({x},{y}) residual={residual}"
                );
            }
        }
    }
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn run_component_spectrum(
    problem: &CompressedProblem,
    prepared: &ComponentExecution<'_>,
    solver: &PreparedModelSolver<'_>,
    inference_rows: ComponentInferenceRows<'_>,
    addresses: &ComponentInferenceAddresses,
    solve_receipts: &mut Vec<ComponentInferenceSolveReceipt>,
    interrupt: &mut dyn InterruptCheck,
) -> Result<ComponentSpectrumResult> {
    let rows = inference_rows.len(problem);
    let controls = inference_rows.control_columns();
    let unit_variance = vec![1.0; rows];
    let rng = CounterRng::new(prepared.options.seed);
    let trace_probes = prepared.options.spectrum_probes as usize;
    let batch_width = prepared.options.batch_width.min(trace_probes);
    let trace_offset = u64::from(prepared.options.probes);
    let mut trace_moments = [ComponentTraceMoments::default(); REPORTED_TARGETS];
    for first in (0..trace_probes).step_by(batch_width) {
        crate::progress::advance(crate::progress::SPECTRUM, first, trace_probes);
        interrupt.checkpoint("generic_jla_component_spectrum_trace_batch")?;
        let width = batch_width.min(trace_probes - first);
        let mut outcome = vec![0.0; rows * width];
        let mut worker_rhs = vec![0.0; problem.workers() * width];
        let mut firm_rhs = vec![0.0; problem.firms() * width];
        let mut control_rhs = vec![0.0; controls.len() * width];
        for local in 0..width {
            fill_gaussian_pseudo_outcome(
                rng,
                trace_offset + (first + local) as u64,
                &addresses.entity,
                &addresses.subdraw,
                &unit_variance,
                &mut outcome[local * rows..(local + 1) * rows],
                interrupt,
            )?;
            let rhs = component_transpose_rhs(
                problem,
                inference_rows,
                &outcome[local * rows..(local + 1) * rows],
                interrupt,
            )?;
            worker_rhs[local * problem.workers()..(local + 1) * problem.workers()]
                .copy_from_slice(&rhs.0);
            firm_rhs[local * problem.firms()..(local + 1) * problem.firms()]
                .copy_from_slice(&rhs.1);
            control_rhs[local * controls.len()..(local + 1) * controls.len()]
                .copy_from_slice(&rhs.2);
        }
        let base = solver.solve_batch_with_interrupt(
            &worker_rhs,
            &firm_rhs,
            &control_rhs,
            width,
            width,
            interrupt,
        )?;
        for (local, solution) in base.solution.iter().enumerate() {
            solve_receipts.push(component_solve_receipt(
                ComponentInferenceSolvePhase::SpectrumTrace,
                u32::try_from(first + local)
                    .map_err(|_| resource("component spectrum probe index overflow"))?,
                solution,
            ));
        }
        let columns = width
            .checked_mul(REPORTED_TARGETS)
            .ok_or_else(|| resource("component spectrum target batch width overflow"))?;
        let mut target_worker_rhs = vec![0.0; problem.workers() * columns];
        let mut target_firm_rhs = vec![0.0; problem.firms() * columns];
        let mut target_control_rhs = vec![0.0; controls.len() * columns];
        for target in 0..REPORTED_TARGETS {
            for local in 0..width {
                let column = target * width + local;
                let rhs = reported_target_rhs(
                    problem,
                    &base.solution[local].coefficients,
                    target,
                    interrupt,
                )?;
                target_worker_rhs[column * problem.workers()..(column + 1) * problem.workers()]
                    .copy_from_slice(&rhs.0);
                target_firm_rhs[column * problem.firms()..(column + 1) * problem.firms()]
                    .copy_from_slice(&rhs.1);
                if !controls.is_empty() {
                    target_control_rhs[column * controls.len()..(column + 1) * controls.len()]
                        .copy_from_slice(&rhs.2);
                }
            }
        }
        let target = solver.solve_batch_with_interrupt(
            &target_worker_rhs,
            &target_firm_rhs,
            &target_control_rhs,
            columns,
            prepared.options.batch_width.min(columns),
            interrupt,
        )?;
        for target_index in 0..REPORTED_TARGETS {
            for local in 0..width {
                let column = target_index * width + local;
                let solution = &target.solution[column];
                let worker = &target_worker_rhs
                    [column * problem.workers()..(column + 1) * problem.workers()];
                let firm =
                    &target_firm_rhs[column * problem.firms()..(column + 1) * problem.firms()];
                let control =
                    &target_control_rhs[column * controls.len()..(column + 1) * controls.len()];
                trace_moments[target_index].push(component_coefficient_dot_rhs(
                    &solution.coefficients,
                    worker,
                    firm,
                    control,
                )?)?;
                let receipt_index = (target_index * trace_probes + first + local) as u32;
                solve_receipts.push(component_solve_receipt(
                    ComponentInferenceSolvePhase::SpectrumTrace,
                    receipt_index,
                    solution,
                ));
            }
        }
    }
    crate::progress::advance(crate::progress::SPECTRUM, trace_probes, trace_probes);
    let trace = trace_moments.map(ComponentTraceMoments::finish);
    let trace: [(f64, f64); REPORTED_TARGETS] = trace
        .into_iter()
        .collect::<Result<Vec<_>>>()?
        .try_into()
        .map_err(|_| BackendError::invariant("component_inference_spectrum", "trace width"))?;

    crate::progress::stage(crate::progress::SPECTRUM_ITERATIONS);
    let start_offset = trace_offset + u64::from(prepared.options.spectrum_probes);
    let mut start_worker_rhs = vec![0.0; 2 * problem.workers()];
    let mut start_firm_rhs = vec![0.0; 2 * problem.firms()];
    let mut start_control_rhs = vec![0.0; 2 * controls.len()];
    for column in 0..2 {
        let mut outcome = vec![0.0; rows];
        fill_gaussian_pseudo_outcome(
            rng,
            start_offset + column as u64,
            &addresses.entity,
            &addresses.subdraw,
            &unit_variance,
            &mut outcome,
            interrupt,
        )?;
        let rhs = component_transpose_rhs(problem, inference_rows, &outcome, interrupt)?;
        start_worker_rhs[column * problem.workers()..(column + 1) * problem.workers()]
            .copy_from_slice(&rhs.0);
        start_firm_rhs[column * problem.firms()..(column + 1) * problem.firms()]
            .copy_from_slice(&rhs.1);
        if !controls.is_empty() {
            start_control_rhs[column * controls.len()..(column + 1) * controls.len()]
                .copy_from_slice(&rhs.2);
        }
    }
    let solved = solver.solve_batch_with_interrupt(
        &start_worker_rhs,
        &start_firm_rhs,
        &start_control_rhs,
        2,
        2,
        interrupt,
    )?;
    let mut start = Vec::with_capacity(2);
    for (column, solution) in solved.solution.into_iter().enumerate() {
        let mut prediction = vec![0.0; rows];
        component_predict(
            problem,
            inference_rows,
            solver,
            &solution.coefficients,
            &mut prediction,
            interrupt,
        )?;
        solve_receipts.push(component_solve_receipt(
            ComponentInferenceSolvePhase::SpectrumStart,
            column as u32,
            &solution,
        ));
        start.push(ComponentSpectrumVector {
            coefficients: solution.coefficients,
            prediction,
        });
    }
    let start: [ComponentSpectrumVector; 2] = start.try_into().map_err(|_| {
        BackendError::invariant(
            "component_inference_spectrum",
            "the spectrum start solve returned the wrong width",
        )
    })?;
    let start = component_orthonormalize_pair(start)?;

    let iteration_receipt_start = solve_receipts.len();
    let mut batched = if let Some(width) = spectrum_batches::width(
        prepared.options.batch_width,
        solver.owned_batch_capacity()?,
        solver.owned_batch_workers()?,
    ) {
        Some(spectrum_batches::iterate(
            problem,
            inference_rows,
            solver,
            &start,
            prepared.options.spectrum_iterations,
            width,
            solve_receipts,
            interrupt,
        )?)
    } else {
        None
    };
    let mut output = [ComponentSpectrumDiagnostics::default(); REPORTED_TARGETS];
    let mut leading_mode = Vec::with_capacity(REPORTED_TARGETS);
    for target in 0..REPORTED_TARGETS {
        output[target] = ComponentSpectrumDiagnostics {
            trace_square_raw: trace[target].0,
            trace_square_mcse: trace[target].1,
            probes: prepared.options.spectrum_probes,
            iterations: prepared.options.spectrum_iterations,
            leading_eigenvalue: f64::NAN,
            second_eigenvalue: f64::NAN,
            leading_residual: f64::NAN,
            second_residual: f64::NAN,
            ..ComponentSpectrumDiagnostics::default()
        };
        let target_mode = (|| -> Result<ComponentSpectrumVector> {
            let basis = if let Some(batched) = &mut batched {
                batched[target].take().ok_or_else(|| {
                    BackendError::invariant(
                        "component_inference_spectrum",
                        "missing cross-target iteration result",
                    )
                })??
            } else {
                let mut basis = start.clone();
                for iteration in 0..prepared.options.spectrum_iterations {
                    let phase_index = (target as u32)
                        .checked_mul(prepared.options.spectrum_iterations)
                        .and_then(|value| value.checked_add(iteration))
                        .and_then(|value| value.checked_mul(2))
                        .ok_or_else(|| resource("component spectrum iteration index overflow"))?;
                    let once = component_apply_target_pair(
                        problem,
                        inference_rows,
                        solver,
                        target,
                        &basis,
                        phase_index,
                        solve_receipts,
                        interrupt,
                    )?;
                    let twice = component_apply_target_pair(
                        problem,
                        inference_rows,
                        solver,
                        target,
                        &once,
                        phase_index + 1,
                        solve_receipts,
                        interrupt,
                    )?;
                    basis = component_orthonormalize_pair(twice)?;
                }
                basis
            };
            let final_phase = REPORTED_TARGETS as u32 * prepared.options.spectrum_iterations * 2
                + target as u32 * 2;
            let ritz_action = component_apply_target_pair(
                problem,
                inference_rows,
                solver,
                target,
                &basis,
                final_phase,
                solve_receipts,
                interrupt,
            )?;
            let modes = component_ritz_rotate(&basis, &ritz_action)?;
            let actions = component_apply_target_pair(
                problem,
                inference_rows,
                solver,
                target,
                &modes,
                final_phase + 1,
                solve_receipts,
                interrupt,
            )?;
            let mut eigenvalue = [0.0; 2];
            let mut residual = [0.0; 2];
            for mode in 0..2 {
                eigenvalue[mode] =
                    component_prediction_inner(&modes[mode].prediction, &actions[mode].prediction)?;
            }
            let scale = eigenvalue[0].abs().max(f64::MIN_POSITIVE);
            for mode in 0..2 {
                let difference = actions[mode]
                    .prediction
                    .iter()
                    .zip(&modes[mode].prediction)
                    .map(|(&action, &vector)| action - eigenvalue[mode] * vector)
                    .collect::<Vec<_>>();
                residual[mode] =
                    component_prediction_inner(&difference, &difference)?.sqrt() / scale;
            }
            let maximum_mode_weight_squared = modes[0]
                .prediction
                .iter()
                .map(|value| value * value)
                .fold(0.0_f64, f64::max);
            output[target].leading_eigenvalue = eigenvalue[0];
            output[target].second_eigenvalue = eigenvalue[1];
            output[target].leading_residual = residual[0];
            output[target].second_residual = residual[1];
            output[target].maximum_mode_weight_squared = maximum_mode_weight_squared;
            output[target] = finish_spectrum_diagnostics(
                eigenvalue[0],
                eigenvalue[1],
                trace[target].0,
                trace[target].1,
                maximum_mode_weight_squared,
                residual[0],
                residual[1],
                prepared.options.spectrum_probes,
                prepared.options.spectrum_iterations,
                prepared.options.spectrum_tolerance,
            )?;
            Ok(modes[0].clone())
        })();
        match target_mode {
            Ok(mode) => leading_mode.push(mode),
            Err(error)
                if (prepared.options.reference_distribution
                    == ComponentReferenceDistribution::Q1
                    || prepared.individual_intervals)
                    && error.code == ErrorCode::JlaConstraintFailed
                    && error.phase == "component_inference_spectrum" =>
            {
                // A failed target mode is never used for q1 studentization.
                // A zero action keeps the shared solve/probe layout unchanged.
                leading_mode.push(component_linear_combination(
                    &start[0], &start[0], 0.0, 0.0,
                )?);
                output[target].leading_share = f64::NAN;
                output[target].leading_share_mcse_trace_only = f64::NAN;
                output[target].remainder_leading_share = f64::NAN;
            }
            Err(error) => return Err(error),
        }
    }
    if batched.is_some() {
        // Keep the historical target-major receipt order, including each
        // target's final Ritz actions. Execution order is not a public key.
        let per_target = prepared.options.spectrum_iterations * 4;
        let final_start = REPORTED_TARGETS as u32 * per_target;
        solve_receipts[iteration_receipt_start..].sort_unstable_by_key(|receipt| {
            let index = receipt.target_or_probe;
            if index < final_start {
                (index / per_target, index % per_target)
            } else {
                (
                    (index - final_start) / 4,
                    per_target + (index - final_start) % 4,
                )
            }
        });
    }
    Ok(ComponentSpectrumResult {
        diagnostics: output,
        leading_mode: leading_mode.try_into().map_err(|_| {
            BackendError::invariant(
                "component_inference_spectrum",
                "the target spectrum returned the wrong number of leading modes",
            )
        })?,
    })
}

#[derive(Clone, Copy, Debug, Default)]
struct ComponentScalarMoments {
    count: u64,
    first: StableAccumulator,
    second: StableAccumulator,
    third: StableAccumulator,
    fourth: StableAccumulator,
}

impl ComponentScalarMoments {
    fn push(&mut self, value: f64) -> Result<()> {
        if !value.is_finite() {
            return Err(nonfinite("a q=1 remainder probe is nonfinite"));
        }
        self.count = self
            .count
            .checked_add(1)
            .ok_or_else(|| resource("q=1 remainder probe count overflow"))?;
        let square = value * value;
        self.first.add(value);
        self.second.add(square);
        self.third.add(square * value);
        self.fourth.add(square * square);
        Ok(())
    }

    fn finish(self) -> Result<(f64, f64)> {
        if self.count < 2 {
            return Err(invalid("at least two q=1 remainder probes are required"));
        }
        let count = self.count as f64;
        let mean = self.first.finish() / count;
        let raw_second = self.second.finish() / count;
        let population_variance = (raw_second - mean * mean).max(0.0);
        let variance = population_variance * count / (count - 1.0);
        let central_fourth = self.fourth.finish() / count
            - 4.0 * mean * self.third.finish() / count
            + 6.0 * mean * mean * raw_second
            - 3.0 * mean.powi(4);
        let influence_variance =
            (central_fourth - population_variance * population_variance).max(0.0);
        let mcse = (influence_variance / count).sqrt() * count / (count - 1.0);
        if !variance.is_finite() || !mcse.is_finite() {
            return Err(nonfinite("q=1 remainder variance moment is nonfinite"));
        }
        Ok((variance, mcse))
    }
}

#[derive(Clone, Debug)]
struct PreparedComponentQ1 {
    status: [ComponentQ1Status; REPORTED_TARGETS],
    eigenvalue: [f64; REPORTED_TARGETS],
    mode: [Vec<f64>; REPORTED_TARGETS],
    ratio: [Vec<f64>; REPORTED_TARGETS],
    influence: [Vec<f64>; REPORTED_TARGETS],
    point_estimate: [f64; REPORTED_TARGETS],
    leading_score: [f64; REPORTED_TARGETS],
    leading_variance_correction: [f64; REPORTED_TARGETS],
    direct_remainder_estimate: [f64; REPORTED_TARGETS],
    remainder_identity_tolerance: [f64; REPORTED_TARGETS],
    probe: [ComponentScalarMoments; REPORTED_TARGETS],
}

fn component_reported_values(primitive: [f64; PRIMITIVE_TARGETS]) -> [f64; REPORTED_TARGETS] {
    [
        primitive[0],
        primitive[1],
        primitive[2],
        primitive[0] + primitive[1] + 2.0 * primitive[2],
    ]
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn prepare_component_q1(
    problem: &CompressedProblem,
    solver: &PreparedModelSolver<'_>,
    coefficients: &ModelCoefficients,
    inference_rows: ComponentInferenceRows<'_>,
    controls: &[Vec<f64>],
    working_y: &[f64],
    residual: &[f64],
    deleted_adjusted: &[f64],
    maker_inverse: &[f64],
    target_diagonal: &[Vec<f64>; PRIMITIVE_TARGETS],
    point_correction: VarianceComponents,
    row_order: &[usize],
    spectrum: &ComponentSpectrumResult,
    solve_receipts: &mut Vec<ComponentInferenceSolveReceipt>,
    interrupt: &mut dyn InterruptCheck,
) -> Result<PreparedComponentQ1> {
    let rows = inference_rows.len(problem);
    let primitive_plugin = primitive_plugins(problem, coefficients, interrupt)?;
    let plugin = component_reported_values(primitive_plugin);
    let correction = [
        point_correction.worker,
        point_correction.firm,
        point_correction.covariance,
        point_correction.total,
    ];
    let point_estimate = core::array::from_fn(|target| plugin[target] - correction[target]);
    let reported_diagonal: [Vec<f64>; REPORTED_TARGETS] = core::array::from_fn(|target| {
        if target < PRIMITIVE_TARGETS {
            target_diagonal[target].clone()
        } else {
            (0..rows)
                .map(|row| {
                    target_diagonal[0][row]
                        + target_diagonal[1][row]
                        + 2.0 * target_diagonal[2][row]
                })
                .collect()
        }
    });
    let mut leading_score = [0.0; REPORTED_TARGETS];
    let mut leading_variance_correction = [0.0; REPORTED_TARGETS];
    let mut ratio: [Vec<f64>; REPORTED_TARGETS] = core::array::from_fn(|_| vec![0.0; rows]);
    for target in 0..REPORTED_TARGETS {
        let mode = &spectrum.leading_mode[target].prediction;
        let lambda = if spectrum.diagnostics[target].certified {
            spectrum.diagnostics[target].leading_eigenvalue
        } else {
            0.0
        };
        leading_score[target] = component_prediction_inner(mode, working_y)?;
        let mut correction = StableAccumulator::default();
        for (position, &row) in row_order.iter().enumerate() {
            checkpoint_chunk(
                interrupt,
                position,
                "component_q1_leading_variance_correction",
            )?;
            correction.add(mode[row] * mode[row] * working_y[row] * deleted_adjusted[row]);
        }
        leading_variance_correction[target] = correction.finish();
        if !leading_variance_correction[target].is_finite() {
            return Err(nonfinite(
                "the q=1 leave-out leading-variance correction is nonfinite",
            ));
        }
        ratio[target] =
            q1_remainder_ratio(&reported_diagonal[target], maker_inverse, mode, lambda)?;
    }

    let columns = REPORTED_TARGETS;
    let mut worker_rhs = vec![0.0; problem.workers() * columns];
    let mut firm_rhs = vec![0.0; problem.firms() * columns];
    let mut control_rhs = vec![0.0; controls.len() * columns];
    for target in 0..REPORTED_TARGETS {
        let target_rhs = reported_target_rhs(problem, coefficients, target, interrupt)?;
        let scaled_outcome = ratio[target]
            .iter()
            .zip(working_y)
            .map(|(&ratio, &outcome)| 0.5 * ratio * outcome)
            .collect::<Vec<_>>();
        let score = component_transpose_rhs(problem, inference_rows, &scaled_outcome, interrupt)?;
        for worker in 0..problem.workers() {
            worker_rhs[target * problem.workers() + worker] =
                target_rhs.0[worker] + score.0[worker];
        }
        for firm in 0..problem.firms() {
            firm_rhs[target * problem.firms() + firm] = target_rhs.1[firm] + score.1[firm];
        }
        for control in 0..controls.len() {
            control_rhs[target * controls.len() + control] =
                target_rhs.2[control] + score.2[control];
        }
    }
    let solved = solver.solve_batch_with_interrupt(
        &worker_rhs,
        &firm_rhs,
        &control_rhs,
        columns,
        columns,
        interrupt,
    )?;
    let mut influence: [Vec<f64>; REPORTED_TARGETS] = core::array::from_fn(|_| Vec::new());
    let mut direct_remainder_estimate = [0.0; REPORTED_TARGETS];
    let mut remainder_identity_tolerance = [0.0; REPORTED_TARGETS];
    for target in 0..REPORTED_TARGETS {
        let solution = &solved.solution[target];
        let mut prediction = vec![0.0; rows];
        component_predict(
            problem,
            inference_rows,
            solver,
            &solution.coefficients,
            &mut prediction,
            interrupt,
        )?;
        influence[target] = finish_q1_influence(
            &prediction,
            &ratio[target],
            working_y,
            residual,
            &spectrum.leading_mode[target].prediction,
            if spectrum.diagnostics[target].certified {
                spectrum.diagnostics[target].leading_eigenvalue
            } else {
                0.0
            },
            leading_score[target],
        )?;
        direct_remainder_estimate[target] =
            component_prediction_inner(working_y, &influence[target])?;
        remainder_identity_tolerance[target] =
            (8.0 * solution.receipt.full_residual).max(256.0 * f64::EPSILON);
        solve_receipts.push(component_solve_receipt(
            ComponentInferenceSolvePhase::Influence,
            PRIMITIVE_TARGETS as u32 + target as u32,
            solution,
        ));
    }
    Ok(PreparedComponentQ1 {
        status: core::array::from_fn(|target| {
            if spectrum.diagnostics[target].certified {
                ComponentQ1Status::Computed
            } else {
                ComponentQ1Status::ModeNotCertified
            }
        }),
        eigenvalue: core::array::from_fn(|target| {
            if spectrum.diagnostics[target].certified {
                spectrum.diagnostics[target].leading_eigenvalue
            } else {
                0.0
            }
        }),
        mode: core::array::from_fn(|target| spectrum.leading_mode[target].prediction.clone()),
        ratio,
        influence,
        point_estimate,
        leading_score,
        leading_variance_correction,
        direct_remainder_estimate,
        remainder_identity_tolerance,
        probe: [ComponentScalarMoments::default(); REPORTED_TARGETS],
    })
}

fn finish_component_q1(
    prepared: &ComponentExecution<'_>,
    variance: &[f64],
    state: PreparedComponentQ1,
) -> Result<[ComponentQ1TargetResult; REPORTED_TARGETS]> {
    let mut output = [ComponentQ1TargetResult::default(); REPORTED_TARGETS];
    for target in 0..REPORTED_TARGETS {
        let (trace_variance, trace_mcse) = state.probe[target].finish()?;
        let mut target_result = finish_q1_target(
            state.point_estimate[target],
            state.leading_score[target],
            state.leading_variance_correction[target],
            state.direct_remainder_estimate[target],
            state.remainder_identity_tolerance[target],
            state.eigenvalue[target],
            &state.mode[target],
            &state.influence[target],
            variance,
            trace_variance,
            trace_mcse,
            prepared.options.psd_tolerance,
        )
        .map_err(|mut error| {
            error.message = format!("q1 target {target}: {}", error.message);
            error
        })?;
        if state.status[target] != ComponentQ1Status::Computed {
            target_result.status = state.status[target];
        }
        output[target] = finish_q1_interval(
            target_result,
            prepared.options.seed,
            target,
            state.eigenvalue[target],
            prepared.options.confidence_level,
            prepared.options.critical_simulations,
        )?;
    }
    Ok(output)
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn run_component_inference_attachment(
    problem: &CompressedProblem,
    prepared: &ComponentExecution<'_>,
    solver: &PreparedModelSolver<'_>,
    coefficients: &ModelCoefficients,
    inference_rows: ComponentInferenceRows<'_>,
    working_y: &[f64],
    residual: &[f64],
    deleted_adjusted: &[f64],
    leverage: &[f64],
    maker_inverse: &[f64],
    target_diagonal: &[Vec<f64>; PRIMITIVE_TARGETS],
    addresses: &ComponentInferenceAddresses,
    structured_fold_entity: Option<&[u64]>,
    match_mass: Option<&[f64]>,
    point_correction: VarianceComponents,
    peak_forecast_bytes: u64,
    counter_plan: ComponentInferenceCounterPlan,
    interrupt: &mut dyn InterruptCheck,
) -> Result<ComponentInferenceResult> {
    crate::progress::stage(crate::progress::INFERENCE);
    interrupt.checkpoint("generic_jla_component_inference_entry")?;
    let _profile = ProfileScope::new(ProfilePhase::Component);
    let rows = inference_rows.len(problem);
    let controls = inference_rows.control_columns();
    let unit_order = match inference_rows {
        ComponentInferenceRows::Observation { row_order, .. } => row_order.to_vec(),
        ComponentInferenceRows::Match { .. } => (0..rows).collect(),
    };
    if (prepared.inference_unit == ComponentInferenceUnit::Match) != match_mass.is_some() {
        return Err(BackendError::invariant(
            "generic_jla_component_inference",
            "component inference unit and collapsed-match mass state disagree",
        ));
    }
    if working_y.len() != rows
        || residual.len() != rows
        || deleted_adjusted.len() != rows
        || leverage.len() != rows
        || maker_inverse.len() != rows
        || addresses.entity.len() != rows
        || addresses.subdraw.len() != rows
        || structured_fold_entity.is_some_and(|entity| entity.len() != rows)
        || match_mass.is_some_and(|mass| mass.len() != rows)
        || controls.iter().any(|column| column.len() != rows)
        || target_diagonal.iter().any(|value| value.len() != rows)
    {
        return Err(BackendError::invariant(
            "generic_jla_component_inference",
            "component-inference retained state has inconsistent dimensions",
        ));
    }
    let mut structured_variance = if prepared.variance_source.structured_model().is_some()
        && prepared.residual_moments.is_none()
    {
        let fold_entity = structured_fold_entity.ok_or_else(|| {
            BackendError::invariant(
                "generic_jla_component_inference",
                "structured component inference is missing outcome-free fold entities",
            )
        })?;
        let mut proxy = vec![0.0; rows];
        for (position, &row) in unit_order.iter().enumerate() {
            checkpoint_chunk(interrupt, position, "structured_variance_proxy")?;
            proxy[row] = working_y[row] * deleted_adjusted[row];
            if !proxy[row].is_finite() {
                return Err(BackendError::new(
                    ErrorCode::CorrectionNonFinite,
                    "structured_variance_proxy",
                    "the leave-out variance proxy is nonfinite",
                ));
            }
        }
        Some(if let Some(match_mass) = match_mass {
            fit_grouped_structured_variance_with_interrupt(
                &proxy,
                residual,
                maker_inverse,
                leverage,
                target_diagonal,
                match_mass,
                fold_entity,
                prepared.structured_options,
                interrupt,
            )?
        } else {
            fit_structured_variance_with_interrupt(
                &proxy,
                residual,
                maker_inverse,
                leverage,
                target_diagonal,
                fold_entity,
                prepared.structured_options,
                interrupt,
            )?
        })
    } else {
        if structured_fold_entity.is_some() {
            return Err(BackendError::invariant(
                "generic_jla_component_inference",
                "oracle component inference unexpectedly retained structured fold entities",
            ));
        }
        None
    };
    let mut residual_moments = if prepared.residual_moments.is_some() {
        Some(residual_moment_attachment::fit(
            problem,
            prepared,
            solver,
            inference_rows,
            residual,
            leverage,
            target_diagonal,
            addresses,
            match_mass,
            interrupt,
        )?)
    } else {
        None
    };
    let variance = if let Some(fitted) = residual_moments.as_ref() {
        &fitted.fit.positive_variance
    } else {
        match prepared.variance_source.structured_model() {
            Some(model) => structured_variance
                .as_ref()
                .expect("structured variance was fitted for a structured source")
                .selected(model),
            None => &prepared.variance,
        }
    };
    if variance.len() != rows {
        return Err(BackendError::invariant(
            "generic_jla_component_inference",
            "component-inference variance vector has the wrong length",
        ));
    }
    if prepared.inference_unit == ComponentInferenceUnit::Match {
        for row in 0..rows {
            let reconstructed = maker_inverse[row] * residual[row];
            let scale = reconstructed
                .abs()
                .max(deleted_adjusted[row].abs())
                .max(1.0);
            if !reconstructed.is_finite()
                || (reconstructed - deleted_adjusted[row]).abs() > 1.0e-10 * scale
            {
                return Err(BackendError::new(
                    ErrorCode::TargetIdentityFailed,
                    "grouped_component_maker_identity",
                    "the scalar maker does not reproduce the original-row adjusted match residual contraction",
                ));
            }
        }
    }
    let ratios = target_ratios(target_diagonal, maker_inverse)?;
    let mut identity_error = 0.0_f64;
    let expected_correction = [
        point_correction.worker,
        point_correction.firm,
        point_correction.covariance,
    ];
    for target in 0..PRIMITIVE_TARGETS {
        let mut value = StableAccumulator::default();
        for (position, &row) in unit_order.iter().enumerate() {
            checkpoint_chunk(interrupt, position, "generic_jla_component_point_identity")?;
            value.add(working_y[row] * deleted_adjusted[row] * target_diagonal[target][row]);
        }
        let value = value.finish();
        let scale = value.abs().max(expected_correction[target].abs()).max(1.0);
        let error = (value - expected_correction[target]).abs();
        identity_error = identity_error.max(error);
        if !error.is_finite() || error > 1.0e-10 * scale {
            return Err(BackendError::new(
                ErrorCode::TargetIdentityFailed,
                "generic_jla_component_point_identity",
                "retained target diagonals do not reproduce the unchanged JLA point correction",
            ));
        }
    }

    let columns = PRIMITIVE_TARGETS;
    let mut worker_rhs = vec![0.0; problem.workers() * columns];
    let mut firm_rhs = vec![0.0; problem.firms() * columns];
    let mut control_rhs = vec![0.0; controls.len() * columns];
    for target in 0..PRIMITIVE_TARGETS {
        let (target_worker, target_firm, target_control) =
            primitive_target_rhs(problem, coefficients, target, interrupt)?;
        let scaled_outcome = ratios[target]
            .iter()
            .zip(working_y)
            .map(|(&ratio, &outcome)| 0.5 * ratio * outcome)
            .collect::<Vec<_>>();
        let score = component_transpose_rhs(problem, inference_rows, &scaled_outcome, interrupt)?;
        for worker in 0..problem.workers() {
            worker_rhs[target * problem.workers() + worker] =
                target_worker[worker] + score.0[worker];
        }
        for firm in 0..problem.firms() {
            firm_rhs[target * problem.firms() + firm] = target_firm[firm] + score.1[firm];
        }
        for control in 0..controls.len() {
            control_rhs[target * controls.len() + control] =
                target_control[control] + score.2[control];
        }
    }
    let solved = solver.solve_batch_with_interrupt(
        &worker_rhs,
        &firm_rhs,
        &control_rhs,
        columns,
        columns,
        interrupt,
    )?;
    let mut influence = core::array::from_fn(|_| Vec::new());
    let spectrum_receipts = (prepared.options.spectrum_probes as usize)
        .checked_mul(5)
        .and_then(|value| {
            value.checked_add((prepared.options.spectrum_iterations as usize).checked_mul(16)?)
        })
        .and_then(|value| value.checked_add(18))
        .ok_or_else(|| resource("component spectrum receipt count overflow"))?;
    let receipt_capacity = columns
        .checked_add(prepared.options.probes as usize)
        .and_then(|value| value.checked_add(spectrum_receipts))
        .and_then(|value| {
            value.checked_add(
                if prepared.options.reference_distribution == ComponentReferenceDistribution::Q1 {
                    REPORTED_TARGETS
                } else {
                    0
                },
            )
        })
        .ok_or_else(|| resource("component-inference receipt count overflow"))?;
    let mut solve_receipts = Vec::with_capacity(receipt_capacity);
    let mut prediction = vec![0.0; rows];
    for target in 0..PRIMITIVE_TARGETS {
        let solution = &solved.solution[target];
        component_predict(
            problem,
            inference_rows,
            solver,
            &solution.coefficients,
            &mut prediction,
            interrupt,
        )?;
        influence[target] = finish_influence(&prediction, &ratios[target], working_y, residual)?;
        solve_receipts.push(component_solve_receipt(
            ComponentInferenceSolvePhase::Influence,
            target as u32,
            solution,
        ));
    }

    let spectrum_profile = ProfileScope::new(ProfilePhase::ComponentSpectrum);
    let spectrum = run_component_spectrum(
        problem,
        prepared,
        solver,
        inference_rows,
        addresses,
        &mut solve_receipts,
        interrupt,
    )?;
    drop(spectrum_profile);
    let mut q1 = if prepared.options.reference_distribution == ComponentReferenceDistribution::Q1 {
        Some(prepare_component_q1(
            problem,
            solver,
            coefficients,
            inference_rows,
            controls,
            working_y,
            residual,
            deleted_adjusted,
            maker_inverse,
            target_diagonal,
            point_correction,
            &unit_order,
            &spectrum,
            &mut solve_receipts,
            interrupt,
        )?)
    } else {
        None
    };

    let rng = CounterRng::new(prepared.options.seed);
    let mut moments = JointProbeMoments::default();
    let probes = prepared.options.probes as usize;
    let batch_width = prepared.options.batch_width.min(probes);
    for first in (0..probes).step_by(batch_width) {
        crate::progress::advance(crate::progress::INFERENCE, first, probes);
        interrupt.checkpoint("generic_jla_component_covariance_batch")?;
        let prepare_profile = ProfileScope::new(ProfilePhase::ComponentPrepare);
        let width = batch_width.min(probes - first);
        let mut pseudo_outcome = vec![0.0; rows * width];
        let mut pseudo_worker_rhs = vec![0.0; problem.workers() * width];
        let mut pseudo_firm_rhs = vec![0.0; problem.firms() * width];
        let mut pseudo_control_rhs = vec![0.0; controls.len() * width];
        for local in 0..width {
            let probe = first + local;
            let outcome = &mut pseudo_outcome[local * rows..(local + 1) * rows];
            fill_gaussian_pseudo_outcome(
                rng,
                probe as u64,
                &addresses.entity,
                &addresses.subdraw,
                variance,
                outcome,
                interrupt,
            )?;
            let rhs = component_transpose_rhs(problem, inference_rows, outcome, interrupt)?;
            pseudo_worker_rhs[local * problem.workers()..(local + 1) * problem.workers()]
                .copy_from_slice(&rhs.0);
            pseudo_firm_rhs[local * problem.firms()..(local + 1) * problem.firms()]
                .copy_from_slice(&rhs.1);
            if !controls.is_empty() {
                pseudo_control_rhs[local * controls.len()..(local + 1) * controls.len()]
                    .copy_from_slice(&rhs.2);
            }
        }
        drop(prepare_profile);
        let solved = solver.solve_batch_with_interrupt(
            &pseudo_worker_rhs,
            &pseudo_firm_rhs,
            &pseudo_control_rhs,
            width,
            width,
            interrupt,
        )?;
        let _statistics_profile = ProfileScope::new(ProfilePhase::ComponentStatistics);
        let mut pseudo_fit = vec![0.0; rows];
        let mut pseudo_residual = vec![0.0; rows];
        for local in 0..width {
            let probe = first + local;
            let solution = &solved.solution[local];
            component_predict(
                problem,
                inference_rows,
                solver,
                &solution.coefficients,
                &mut pseudo_fit,
                interrupt,
            )?;
            let outcome = &pseudo_outcome[local * rows..(local + 1) * rows];
            for row in 0..rows {
                pseudo_residual[row] = outcome[row] - pseudo_fit[row];
            }
            let plugins = primitive_plugins(problem, &solution.coefficients, interrupt)?;
            let mut scalar = [0.0; PRIMITIVE_TARGETS];
            for target in 0..PRIMITIVE_TARGETS {
                scalar[target] =
                    probe_scalar(plugins[target], &pseudo_residual, &ratios[target], outcome)?;
            }
            moments.push(scalar)?;
            if let Some(q1) = &mut q1 {
                let reported_plugin = component_reported_values(plugins);
                for target in 0..REPORTED_TARGETS {
                    let scalar = q1_probe_scalar(
                        reported_plugin[target],
                        &pseudo_residual,
                        &q1.ratio[target],
                        outcome,
                        &q1.mode[target],
                        q1.eigenvalue[target],
                    )?;
                    q1.probe[target].push(scalar)?;
                }
            }
            solve_receipts.push(component_solve_receipt(
                ComponentInferenceSolvePhase::CovarianceProbe,
                probe as u32,
                solution,
            ));
        }
    }
    crate::progress::advance(crate::progress::INFERENCE, probes, probes);
    let mut result = finish_component_covariance_with_reporting(
        &influence,
        variance,
        moments.finish()?,
        prepared.options.psd_tolerance,
        prepared.individual_intervals,
        interrupt,
    )?;
    let q1 = q1
        .map(|state| finish_component_q1(prepared, variance, state))
        .transpose()?;
    let mut maximum_iterations = 0_u32;
    let mut maximum_reduced_residual = 0.0_f64;
    let mut maximum_complete_residual = 0.0_f64;
    for receipt in &solve_receipts {
        maximum_iterations = maximum_iterations.max(receipt.iterations);
        maximum_reduced_residual = maximum_reduced_residual.max(receipt.reduced_residual);
        maximum_complete_residual = maximum_complete_residual.max(receipt.complete_residual);
    }
    if let Some(fit) = &residual_moments {
        for receipt in &fit.projections {
            maximum_iterations = maximum_iterations.max(receipt.iterations);
            maximum_reduced_residual = maximum_reduced_residual.max(receipt.reduced_residual);
            maximum_complete_residual = maximum_complete_residual.max(receipt.complete_residual);
        }
    }
    result.solve_receipts = solve_receipts;
    result.maximum_iterations = maximum_iterations;
    result.maximum_reduced_residual = maximum_reduced_residual;
    result.maximum_complete_residual = maximum_complete_residual;
    result.full_residual_tolerance = solver.options().full_residual_tolerance();
    result.peak_forecast_bytes = peak_forecast_bytes;
    result.leverage = leverage.to_vec();
    result.maker_inverse = maker_inverse.to_vec();
    result.target_diagonal = target_diagonal.clone();
    result.influence = influence;
    result.point_correction_identity_error = identity_error;
    // Target-keyed streams are unaffected by another target's unavailability.
    // The plan budgets all four streams; receipts count only executed draws.
    let skipped_draws = q1.as_ref().map_or(0, |targets| {
        targets
            .iter()
            .map(|target| u64::from(prepared.options.critical_simulations - target.critical_draws))
            .sum::<u64>()
    });
    result.counter_atoms = counter_plan.logical_atoms - 2 * skipped_draws;
    result.counter_words = counter_plan.unique_words - 4 * skipped_draws;
    result.critical_simulations = if q1.is_some() {
        prepared.options.critical_simulations
    } else {
        0
    };
    result.spectrum = spectrum.diagnostics;
    if prepared.individual_intervals {
        for target in 0..REPORTED_TARGETS {
            if !result.spectrum[target].certified {
                result.q0_status[target] =
                    crate::component_inference::ComponentQ0Status::ModeNotCertified;
            }
        }
    }
    result.q1 = q1;
    result.inference_unit = prepared.inference_unit;
    result.independent_units =
        u64::try_from(rows).map_err(|_| resource("component-inference independent unit count"))?;
    result.nuisance_uncertainty_conditioned_away =
        prepared.inference_unit == ComponentInferenceUnit::Match;
    if let Some(match_mass) = match_mass {
        let total = match_mass.iter().sum::<f64>();
        let square = match_mass.iter().map(|value| value * value).sum::<f64>();
        if !total.is_finite() || total <= 0.0 || !square.is_finite() || square <= 0.0 {
            return Err(nonfinite("collapsed-match mass diagnostics are invalid"));
        }
        result.effective_match_count = total * total / square;
        result.largest_match_mass_share =
            match_mass.iter().copied().fold(0.0_f64, f64::max) / total;
        result.largest_match_leverage = leverage.iter().copied().fold(0.0_f64, f64::max);
        result.smallest_maker_denominator = maker_inverse
            .iter()
            .map(|value| value.recip())
            .fold(f64::INFINITY, f64::min);
    }
    result.variance_source = prepared.variance_source;
    result.structured_variance = structured_variance.take();
    result.residual_moments = residual_moments.take();
    Ok(result)
}

fn component_solve_receipt(
    phase: ComponentInferenceSolvePhase,
    target_or_probe: u32,
    solve: &ModelSolve,
) -> ComponentInferenceSolveReceipt {
    ComponentInferenceSolveReceipt {
        phase,
        target_or_probe,
        iterations: solve.receipt.pcg.iterations,
        reduced_residual: solve.receipt.pcg.relative_residual,
        complete_residual: solve.receipt.full_residual,
        full_residual_tolerance: solve.receipt.full_residual_tolerance,
    }
}

fn component_inference_peak_forecast(
    problem: &CompressedProblem,
    prepared: &ComponentExecution<'_>,
    full_parameters: usize,
    route_memory: RouteMemory,
    queue_capacity: Option<usize>,
    spectrum_workers: usize,
) -> Result<u64> {
    let rows = u64::try_from(problem.outcome.len())
        .map_err(|_| resource("component-inference row forecast is not representable"))?;
    let original_parameters = problem
        .workers()
        .checked_add(problem.firms())
        .and_then(|value| value.checked_add(problem.controls.len()))
        .ok_or_else(|| resource("component-inference parameter forecast overflow"))?;
    let original_parameters = u64::try_from(original_parameters)
        .map_err(|_| resource("component-inference parameter forecast is not representable"))?;
    let reduced_parameters = u64::try_from(full_parameters)
        .map_err(|_| resource("component-inference reduced dimension is not representable"))?;
    let workers = u64::try_from(problem.workers())
        .map_err(|_| resource("component-inference worker count is not representable"))?;
    let probes = u64::from(prepared.options.probes);
    let spectrum_probes = u64::from(prepared.options.spectrum_probes);
    let batch =
        u64::try_from(prepared.options.batch_width.min(
            (prepared.options.probes as usize).max(prepared.options.spectrum_probes as usize),
        ))
        .map_err(|_| resource("component-inference batch width is not representable"))?;
    let fixed_rows = checked_product(&[rows, 32, 8], "component-inference fixed row state")?;
    let batch_rows = checked_product(
        &[
            rows,
            batch
                .checked_add(2)
                .ok_or_else(|| resource("component-inference batch row overflow"))?,
            8,
        ],
        "component-inference probe outcomes, fits, and residuals",
    )?;
    let external_batch = checked_product(
        &[original_parameters, batch, 8, 16],
        "component-inference complete RHS and solution batch",
    )?;
    let solver_batch = checked_sum(&[
        checked_product(
            &[reduced_parameters, batch, 8, 11],
            "component-inference PCG batch",
        )?,
        checked_product(
            &[workers, batch, 8, 3],
            "component-inference worker workspace",
        )?,
        checked_product(&[batch, 8, 8], "component-inference scalar workspace")?,
        checked_product(&[batch, 96], "component-inference solver receipts")?,
        checked_product(
            &[route_memory.cmg_batch_workspace_per_column, batch],
            "component-inference CMG batch workspace",
        )?,
    ])?;
    let spectrum_receipts = spectrum_probes
        .checked_mul(5)
        .and_then(|value| {
            value.checked_add(u64::from(prepared.options.spectrum_iterations).checked_mul(16)?)
        })
        .and_then(|value| value.checked_add(18))
        .ok_or_else(|| resource("component spectrum receipt forecast overflow"))?;
    let receipt_count = probes
        .checked_add(PRIMITIVE_TARGETS as u64)
        .and_then(|value| value.checked_add(spectrum_receipts))
        .and_then(|value| {
            value.checked_add(
                u64::from(
                    prepared.options.reference_distribution == ComponentReferenceDistribution::Q1,
                ) * REPORTED_TARGETS as u64,
            )
        })
        .ok_or_else(|| resource("component-inference receipt count overflow"))?;
    let retained_receipts = checked_product(
        &[
            receipt_count,
            u64::try_from(core::mem::size_of::<ComponentInferenceSolveReceipt>())
                .map_err(|_| resource("component-inference receipt size is not representable"))?,
        ],
        "component-inference retained receipts",
    )?;
    let critical_workspace =
        if prepared.options.reference_distribution == ComponentReferenceDistribution::Q1 {
            checked_product(
                &[u64::from(prepared.options.critical_simulations), 8],
                "q=1 critical-value workspace",
            )?
        } else {
            0
        };
    checked_sum(&[
        prepared.persistent_bytes,
        fixed_rows,
        spectrum_batches::extra_bytes(
            rows,
            original_parameters,
            spectrum_batches::width(
                prepared.options.batch_width,
                queue_capacity,
                spectrum_workers,
            )
            .is_some(),
        )?,
        batch_rows,
        external_batch,
        solver_batch,
        route_memory.full_cmg.map_or(Ok(0), |setup| {
            // Trace actions retain four transformed columns per probe. Admit
            // logical output lifetimes even when physical chunks are smaller.
            direct::solve_bytes(
                workers,
                problem.firms() as u64,
                problem.controls.len() as u64,
                checked_product(&[batch.max(1), 4], "component direct columns")?,
                setup,
            )
        })?,
        retained_receipts,
        critical_workspace,
        // V5 exports all matrices synchronously through Rust/C into Stata.
        // Admit both copies and both receipts, including unavailable buffers.
        if prepared.individual_intervals {
            2 * (846 * 8 + 288)
        } else {
            0
        },
        if prepared.residual_moments.is_some() {
            residual_moment_attachment::peak_bytes(problem, prepared, route_memory, queue_capacity)?
        } else {
            0
        },
        2_048,
    ])
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
    stayer_rows: Option<&[bool]>,
    rng: CounterRng,
    options: GenericJlaOptions,
    retain_diagonal: bool,
    rhs_receipts: &mut Vec<GenericJlaRhsReceipt>,
    interrupt: &mut dyn InterruptCheck,
) -> Result<TargetCorrection> {
    let _profile = ProfileScope::new(ProfilePhase::Target);
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
    let mut diagonal_sum = if retain_diagonal {
        let mut allocate = || {
            repeated(
                problem.outcome.len(),
                StableAccumulator::default(),
                "target diagonal accumulators",
                interrupt,
                "generic_jla_target_allocate",
            )
        };
        Some([allocate()?, allocate()?, allocate()?])
    } else {
        None
    };
    for first in (0..probes).step_by(options.target_batch_width) {
        crate::progress::advance(crate::progress::TARGETS, first, probes);
        interrupt.checkpoint("generic_jla_target_batch")?;
        let rng_profile = ProfileScope::new(ProfilePhase::TargetRng);
        let width = options.target_batch_width.min(probes - first);
        let mut atoms = repeated(
            checked_matrix_length(plan.cell.len(), width, "target atoms")?,
            0_i64,
            "target atoms",
            interrupt,
            "generic_jla_target_allocate",
        )?;
        fill_probe_atoms(
            solver,
            rng,
            ProbeDomain::Target,
            first as u64,
            width,
            &plan.entity,
            &plan.physical_count,
            &mut atoms,
            interrupt,
        )?;
        drop(rng_profile);
        let rhs_profile = ProfileScope::new(ProfilePhase::TargetRhs);
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
        let worker_pair = checked_matrix_length(workers, 2, "target worker pair")?;
        let firm_pair = checked_matrix_length(firms, 2, "target firm pair")?;
        solver.statistical_work(
            worker_rhs
                .chunks_mut(worker_pair)
                .zip(firm_rhs.chunks_mut(firm_pair))
                .zip(worker_correction.chunks_mut(worker_pair))
                .zip(firm_correction.chunks_mut(firm_pair)),
            "generic_jla_target_rhs",
            |local, (((worker_rhs, firm_rhs), worker_correction), firm_correction), interrupt| {
                let mut total_direction = StableAccumulator::default();
                let mut reference_scale = StableAccumulator::default();
                let worker_column = 0;
                let firm_column = 1;
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
                        worker_rhs,
                        worker_correction,
                        worker_column * workers + problem.cell_worker[cell] as usize,
                        direction,
                    );
                    stable_add_index(
                        firm_rhs,
                        firm_correction,
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
                    let centered =
                        plan.target_mass[stratum] / problem.target_total * total_direction;
                    let cell = plan.cell[stratum] as usize;
                    stable_add_index(
                        worker_rhs,
                        worker_correction,
                        worker_column * workers + problem.cell_worker[cell] as usize,
                        -centered,
                    );
                    stable_add_index(
                        firm_rhs,
                        firm_correction,
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
                Ok(())
            },
            interrupt,
        )?;
        drop(worker_correction);
        drop(firm_correction);
        drop(rhs_profile);
        let solved = solver.solve_batch_with_interrupt(
            &worker_rhs,
            &firm_rhs,
            &control_rhs,
            columns,
            columns,
            interrupt,
        )?;
        let _statistics_profile = ProfileScope::new(ProfilePhase::TargetStatistics);
        statistical_batches::validate_predictions(solver, &solved.solution, interrupt)?;
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
        }
        solver.statistical_work(
            draws[first..first + width].iter_mut(),
            "generic_jla_target_statistics",
            |local, draw, interrupt| {
                *draw = statistical_batches::target_draw(
                    problem,
                    solver,
                    &solved.solution[2 * local..2 * local + 2],
                    working_y,
                    deleted_adjusted,
                    options.deletion,
                    row_order,
                    match_rows,
                    stayer_rows,
                    interrupt,
                )?;
                Ok(())
            },
            interrupt,
        )?;
        if let Some(diagonal) = diagonal_sum.as_mut() {
            statistical_batches::target_diagonal(solver, &solved.solution, diagonal, interrupt)?;
        }
    }
    crate::progress::advance(crate::progress::TARGETS, probes, probes);
    let mean = component_mean(&draws, interrupt)?;
    let mcse = component_mcse(&draws, mean, interrupt)?;
    Ok(TargetCorrection {
        mean,
        mcse,
        maximum_solve_relres,
        diagonal: diagonal_sum.map(|values| {
            let probes = f64::from(options.probes);
            values.map(|target| {
                target
                    .into_iter()
                    .map(|value| value.finish() / probes)
                    .collect()
            })
        }),
    })
}

#[cfg(test)]
fn target_contraction(
    problem: &CompressedProblem,
    working_y: &[f64],
    deleted_adjusted: &[f64],
    projection: &[f64],
    deletion: DeletionMode,
    row_order: &[usize],
    match_rows: Option<&[Vec<usize>]>,
    stayer_rows: Option<&[bool]>,
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
            if let Some(stayer_rows) = stayer_rows {
                if stayer_rows.len() != rows {
                    return Err(BackendError::invariant(
                        "generic_jla_target_contraction",
                        "hybrid stayer mask has the wrong length",
                    ));
                }
                for (position, &row) in row_order.iter().enumerate() {
                    checkpoint_chunk(interrupt, position, "generic_jla_target_stayer")?;
                    if stayer_rows[row] {
                        output.add(
                            problem.frequency[row] as f64
                                * working_y[row]
                                * deleted_adjusted[row]
                                * projection[row]
                                * projection[row],
                        );
                    }
                }
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
    if let Some(receipt) = solver.full_cmg_receipt()? {
        return direct::route_memory(problem, receipt.setup);
    }
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
        full_cmg: None,
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
    hybrid: bool,
    leverage_request: BatchRequest,
    target_request: BatchRequest,
    route_memory: RouteMemory,
    memory_facts: MemoryFacts,
    interrupt: &mut dyn InterruptCheck,
) -> Result<GenericJlaBatchExecutionReceipt> {
    if let Some(setup) = route_memory.full_cmg {
        return direct::plan_batches(
            problem,
            options,
            full_parameters,
            hybrid,
            route_memory,
            memory_facts,
            setup,
            leverage_request,
            target_request,
        );
    }
    let width_one = memory_forecast(
        problem,
        options,
        full_parameters,
        hybrid,
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
        hybrid,
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
        hybrid,
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
            memory_budget: options.memory_budget,
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
    hybrid: bool,
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
                hybrid,
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
                hybrid,
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
    hybrid: bool,
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
        leverage: checked_product(&[rows, probes, if hybrid { 3 } else { 2 }], "wall leverage")?,
        target: checked_product(&[rows, probes, 2], "wall target")?,
        result_export: checked_sum(&[rows, parameters, probes])?,
    })
}

fn memory_forecast(
    problem: &CompressedProblem,
    options: GenericJlaOptions,
    full_parameters: usize,
    hybrid: bool,
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
        u64::try_from(options.projection_columns)
            .map_err(|_| resource("projection column count is not representable"))?,
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
        if let Some(setup) = route_memory.full_cmg {
            return direct::solve_bytes(workers, firms, controls, columns, setup);
        }
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
            &[
                rows,
                size_of::<statistical_batches::ObservationAddress>() as u64,
            ],
            "observation statistical addresses",
        )?,
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
        if hybrid {
            checked_sum(&[match_leverage, observation_leverage])?
        } else if options.deletion == DeletionMode::Observation {
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
        let match_maker = checked_sum(&[
            maker_base,
            checked_product(
                &[maximum_block, maker_rank, f64_bytes],
                "match low-rank maker",
            )?,
            checked_product(&[maximum_block, f64_bytes, 8], "match block vectors")?,
            checked_product(&[maker_square, 8], "maker inverse and factor work")?,
            checked_product(&[maker_rank, f64_bytes, 2], "reduced maker vectors")?,
        ])?;
        if hybrid {
            // The observation correlations and moments returned by the joint
            // leverage pass remain live while the mover match blocks are
            // inverted and the two deleted-fit vectors are combined.
            checked_sum(&[
                match_maker,
                checked_product(
                    &[physical, f64_bytes, 2],
                    "hybrid retained observation correlations",
                )?,
                checked_product(
                    &[observation_offset_count, usize_bytes],
                    "hybrid retained observation offsets",
                )?,
                checked_product(&[row_f64, 8], "hybrid observation moments and deleted fits")?,
                checked_product(
                    &[deletion_units, 5, f64_bytes],
                    "hybrid retained match moments",
                )?,
            ])?
        } else {
            match_maker
        }
    } else {
        maker_base
    };
    let projection_columns = u64::try_from(options.projection_columns)
        .map_err(|_| resource("projection column count is not representable"))?;
    let target_columns = checked_product(&[target_width, 2], "target solver columns")?;
    let target_retained_row_vectors = checked_sum(&[6, u64::from(projection_columns > 0)])?;
    let target = checked_sum(&[
        prepared,
        canonical_live,
        semantic_plans,
        checked_product(
            &[row_f64, target_retained_row_vectors],
            "target retained row vectors",
        )?,
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
    let projection = if projection_columns == 0 {
        0
    } else {
        let projection_square = checked_product(
            &[projection_columns, projection_columns, f64_bytes],
            "projection covariance matrices",
        )?;
        checked_sum(&[
            prepared,
            canonical_live,
            semantic_plans,
            checked_product(&[row_f64, 3], "projection retained row vectors")?,
            checked_product(
                &[original_parameters, projection_columns, f64_bytes, 2],
                "projection complete solutions",
            )?,
            checked_product(&[projection_square, 4], "projection covariance work")?,
            options.projection_result_bytes,
            solver_batch(projection_columns, reduced_parameters, "projection PCG")?,
        ])?
    };
    // During the solve-to-result transition the complete prepared context is
    // still live beside the newly retained result. Once the prepared problem
    // is dropped, only its shared retained-mask backing survives, while the C
    // V2 row buffer and caller's fifteen-column matrix coexist with the native
    // result. Count each allocation exactly once in its actual lifetime.
    let native_result_payload = checked_sum(&[
        1024,
        rhs_receipt_bytes,
        control_projection_receipt_bytes,
        options.projection_result_bytes,
    ])?;
    let result_transition = checked_sum(&[prepared, native_result_payload])?;
    let result_export = checked_sum(&[
        native_result_payload,
        options.retained_mask_bytes,
        options.rhs_export_bytes,
        options.projection_export_bytes,
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
        (GenericJlaMemoryPeakPhase::Projection, projection),
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
        projection,
        component_inference: 0,
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

/// Keep four adjacent Philox lanes together, so parallelism does not repeat a
/// complete counter block for each scalar lane. This is RNG packing only,
/// not experimental CMG RHS fusion. Addresses and admitted payloads are fixed.
#[allow(clippy::too_many_arguments)]
fn fill_probe_atoms(
    solver: &PreparedModelSolver<'_>,
    rng: CounterRng,
    domain: ProbeDomain,
    first: u64,
    columns: usize,
    entity: &[u64],
    physical: &[u64],
    atoms: &mut [i64],
    interrupt: &mut dyn InterruptCheck,
) -> Result<()> {
    if columns == 0 || entity.is_empty() || entity.len() != physical.len() {
        return rng.fill_rademacher_sums_with_interrupt(
            domain, first, columns, entity, physical, atoms, interrupt,
        );
    }
    if atoms.len() != checked_matrix_length(entity.len(), columns, "statistical RNG output")? {
        return Err(BackendError::invariant(
            "generic_jla_rng",
            "RNG column shape mismatch",
        ));
    }
    first
        .checked_add((columns - 1) as u64)
        .ok_or_else(|| resource("statistical RNG probe index overflow"))?;
    let job_width = checked_matrix_length(entity.len(), 4, "packed statistical RNG job")?;
    solver.statistical_work(
        atoms.chunks_mut(job_width),
        "generic_jla_rng",
        |block, atoms, check| {
            let probe = first
                .checked_add(
                    (block as u64)
                        .checked_mul(4)
                        .ok_or_else(|| resource("statistical RNG block overflow"))?,
                )
                .ok_or_else(|| resource("statistical RNG probe index overflow"))?;
            rng.fill_rademacher_sums_with_interrupt(
                domain,
                probe,
                atoms.len() / entity.len(),
                entity,
                physical,
                atoms,
                check,
            )
        },
        interrupt,
    )
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

fn rhs_residual_maxima(receipts: &[GenericJlaRhsReceipt]) -> (f64, f64) {
    receipts
        .iter()
        .fold((0.0_f64, 0.0_f64), |maximum, receipt| {
            (
                maximum.0.max(receipt.pcg.relative_residual),
                maximum.1.max(receipt.complete_residual),
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_rhs(reduced: f64, complete: f64) -> GenericJlaRhsReceipt {
        GenericJlaRhsReceipt {
            phase: GenericJlaRhsPhase::Projection,
            side: GenericJlaRhsSide::Joint,
            probe: Some(0),
            pcg: ModelPcgReceipt {
                status: crate::generic_batch::ModelPcgStatus::Converged,
                iterations: 3,
                relative_residual: reduced,
                residual_replacements: 0,
                operator_applications: 4,
                preconditioner_applications: 3,
            },
            complete_residual: complete,
        }
    }

    #[test]
    fn rhs_maxima_keep_reduced_and_complete_residual_spaces_distinct() {
        let receipts = [test_rhs(8.0e-9, 2.0e-9), test_rhs(3.0e-9, 7.0e-9)];
        let (reduced, complete) = rhs_residual_maxima(&receipts);
        assert_eq!(reduced.to_bits(), 8.0e-9_f64.to_bits());
        assert_eq!(complete.to_bits(), 7.0e-9_f64.to_bits());
    }

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

    #[test]
    fn component_gaussian_counter_is_preflighted_by_semantic_entity() {
        let prepared = PreparedComponentInference {
            schema_version: crate::component_inference::COMPONENT_INFERENCE_SCHEMA_VERSION,
            inference_unit: ComponentInferenceUnit::Observation,
            variance_source: ComponentVarianceSource::Oracle,
            variance: Vec::new(),
            options: crate::component_inference::ComponentInferenceOptions {
                probes: 7,
                spectrum_probes: 2,
                ..crate::component_inference::ComponentInferenceOptions::default()
            },
            structured_options: crate::structured_variance::StructuredVarianceOptions::default(),
            persistent_bytes: 0,
            individual_intervals: false,
            residual_moments: None,
            design_only_order: false,
            unified_variance_fit: false,
        };
        let prepared = ComponentExecution::new(&prepared);
        let plan = plan_component_inference_counter(
            &prepared,
            &[
                ObservationClass {
                    rows: Vec::new(),
                    entity: 1,
                    physical_count: 3,
                },
                ObservationClass {
                    rows: Vec::new(),
                    entity: 2,
                    physical_count: 5,
                },
            ],
            None,
        )
        .expect("small Gaussian Counter plan");
        assert_eq!(plan.logical_atoms, 88);
        assert_eq!(plan.unique_words, 176);

        let error = plan_component_inference_counter(
            &prepared,
            &[ObservationClass {
                rows: Vec::new(),
                entity: 3,
                physical_count: MAX_PHYSICAL_WORDS_PER_ATOM / 2 + 1,
            }],
            None,
        )
        .expect_err("oversized Gaussian semantic entity rejects before RNG");
        assert_eq!(error.code, ErrorCode::ResourceLimit);
        assert_eq!(error.phase, "generic_jla_component_counter");
    }
}
