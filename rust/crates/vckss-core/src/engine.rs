// SPDX-License-Identifier: GPL-3.0-only

//! End-to-end no-control, match-deletion improved-JLA engine.

use core::mem::size_of;

use crate::batch_plan::{
    plan_batches_with_forecasts, BatchPlanReceipt, BatchPlannerCaps, BatchRequest,
};
use crate::counter_accounting::{
    combine_counter_phases, plan_counter_phase, CounterExecutionReceipt, GeneratorEvaluationModel,
};
use crate::engine_plan::SelectedEngine;
use crate::error::{BackendError, ErrorCode, Result};
use crate::full_cmg::{FullCmgPlanOptions, FullCmgReceipt};
use crate::interrupt::{checkpoint_chunk, InterruptCheck, NeverInterrupt};
use crate::jla::{plugin_components_with_interrupt, JlaPlan, VarianceComponents};
use crate::problem::CompressedProblem;
use crate::rng::{CounterRng, ProbeDomain, MAX_PHYSICAL_WORDS_PER_ATOM};
use crate::solver::{
    LinearSolverOptions, LinearSolverRoute, PreparedSolverReceipt, PreparedTwoWaySolver,
    RoutedSolveReceipt,
};
use crate::types::{DeletionMode, RngContract};
use crate::wall_plan::{wall_work_receipt, WallCalibration, WallWork, WallWorkReceipt};

const ROUNDOFF_GATE: f64 = 4096.0 * f64::EPSILON;
const LEVERAGE_MOMENT_BLOCK_GROUPS: usize = 4_096;
const RNG_PROBE_BLOCK_COLUMNS: usize = 4;
pub const COMPRESSED_JLA_AUTO_BATCH_WIDTH_CAP_V1: usize = 32;
pub const COMPRESSED_JLA_EXECUTION_SCHEMA_VERSION: u32 = 2;

#[derive(Clone, Copy, Debug)]
pub struct JlaEngineOptions {
    pub seed: u64,
    pub probes: u32,
    pub leverage_batch_width: usize,
    pub target_batch_width: usize,
    pub deletion: DeletionMode,
    pub rng: RngContract,
    pub rank_tolerance: f64,
    pub block_tolerance: f64,
    pub memory_limit_bytes: u64,
    pub prepared_persistent_bytes: u64,
    pub solver: LinearSolverOptions,
}

impl Default for JlaEngineOptions {
    fn default() -> Self {
        Self {
            seed: 8_675_309,
            probes: 200,
            leverage_batch_width: 8,
            target_batch_width: 8,
            deletion: DeletionMode::Match,
            rng: RngContract::CounterV1,
            rank_tolerance: 1.0e-10,
            block_tolerance: 1.0e-10,
            memory_limit_bytes: 4_u64 << 30,
            prepared_persistent_bytes: 0,
            solver: LinearSolverOptions::default(),
        }
    }
}

impl JlaEngineOptions {
    fn validate(self) -> Result<Self> {
        if self.deletion != DeletionMode::Match {
            return Err(BackendError::new(
                ErrorCode::UnsupportedFeature,
                "jla_validate",
                "the Rust JLA engine supports match deletion only",
            ));
        }
        if self.rng != RngContract::CounterV1 {
            return Err(BackendError::new(
                ErrorCode::UnsupportedFeature,
                "jla_validate",
                "the Rust JLA engine supports VCKSS-COUNTER-V1 only",
            ));
        }
        if self.probes < 2 {
            return Err(BackendError::invalid(
                "jla_validate",
                "JLA probe count must be at least two",
            ));
        }
        if self.leverage_batch_width == 0 || self.target_batch_width == 0 {
            return Err(BackendError::invalid(
                "jla_validate",
                "leverage and target batch widths must be positive",
            ));
        }
        if !self.rank_tolerance.is_finite()
            || self.rank_tolerance < 1.0e-14
            || self.rank_tolerance >= 0.1
        {
            return Err(BackendError::invalid(
                "jla_validate",
                "rank tolerance must be finite and lie in [1e-14, 0.1)",
            ));
        }
        if !self.block_tolerance.is_finite()
            || self.block_tolerance < 1.0e-14
            || self.block_tolerance >= 1.0
        {
            return Err(BackendError::invalid(
                "jla_validate",
                "block tolerance must be finite and lie in [1e-14, 1)",
            ));
        }
        if self.memory_limit_bytes == 0 {
            return Err(BackendError::invalid(
                "jla_validate",
                "whole-command memory limit must be positive",
            ));
        }
        self.solver.validate()?;
        Ok(self)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JlaSolvePhase {
    FullFit,
    Leverage,
    Target,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JlaRhsSide {
    Joint,
    Worker,
    Firm,
}

#[derive(Clone, Debug)]
pub struct JlaRhsReceipt {
    pub phase: JlaSolvePhase,
    pub probe: Option<u64>,
    pub side: JlaRhsSide,
    pub route: LinearSolverRoute,
    pub iterations: u32,
    pub reduced_residual: f64,
    pub complete_residual: f64,
    pub zero_rhs: bool,
}

#[derive(Clone, Debug)]
pub struct JlaEngineReceipt {
    pub seed: u64,
    pub rng: RngContract,
    pub probes_requested: u32,
    pub leverage_probes_accepted: u32,
    pub target_probes_accepted: u32,
    pub leverage_batch_width: usize,
    pub target_batch_width: usize,
    pub rank_tolerance: f64,
    pub block_tolerance: f64,
    pub full_residual_tolerance: f64,
    pub solver: PreparedSolverReceipt,
    pub full_fit: JlaRhsReceipt,
    pub leverage_rhs: Vec<JlaRhsReceipt>,
    pub target_rhs: Vec<JlaRhsReceipt>,
    pub max_reduced_residual: f64,
    pub max_complete_residual: f64,
    pub max_leverage: f64,
    pub max_reciprocal_residual: f64,
    pub accounting_residual: f64,
    pub topology_checksum: u64,
    pub memory: JlaMemoryReceipt,
}

#[derive(Clone, Debug)]
pub struct JlaEngineResult {
    pub plugin: VarianceComponents,
    pub correction: VarianceComponents,
    pub corrected: VarianceComponents,
    pub numerical_mcse: NumericalMcse,
    pub weighted_rss: f64,
    pub fitted_cell: Vec<f64>,
    pub unit_projection_share: Vec<f64>,
    pub unit_residual_share: Vec<f64>,
    pub unit_finite_bias: Vec<f64>,
    pub unit_finite_variance: Vec<f64>,
    pub unit_residual_mass: Vec<f64>,
    pub unit_deleted_mass: Vec<f64>,
    pub cell_correction_weight: Vec<f64>,
    pub target_draws: Vec<VarianceComponents>,
    pub receipt: JlaEngineReceipt,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct JlaMemoryReceipt {
    pub hard_limit_bytes: u64,
    pub prepared_persistent_bytes: u64,
    pub solver_setup_forecast_bytes: u64,
    pub leverage_phase_forecast_bytes: u64,
    pub target_phase_forecast_bytes: u64,
    pub result_forecast_bytes: u64,
    /// Full-fit command peak with neither leverage nor target batch active.
    pub non_batched_phase_forecast_bytes: u64,
    pub solve_peak_forecast_bytes: u64,
}

#[derive(Clone, Copy, Debug)]
pub struct PlannedJlaEngineOptions {
    pub estimator: JlaEngineOptions,
    pub leverage_batch: BatchRequest,
    pub target_batch: BatchRequest,
    pub wallseconds: Option<f64>,
    pub full_cmg: Option<FullCmgPlanOptions>,
}

impl Default for PlannedJlaEngineOptions {
    fn default() -> Self {
        Self {
            estimator: JlaEngineOptions::default(),
            leverage_batch: BatchRequest::Auto,
            target_batch: BatchRequest::Auto,
            wallseconds: None,
            full_cmg: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CompressedJlaBatchReceipt {
    pub plan: BatchPlanReceipt,
    pub leverage_requested: BatchRequest,
    pub leverage_active_width: usize,
    pub target_requested: BatchRequest,
    pub target_active_width: usize,
    pub automatic_ladder_cap: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CompressedJlaThreadReceipt {
    pub requested: usize,
    pub used: usize,
    pub parallel_regions: usize,
}

#[derive(Clone, Debug)]
pub struct CompressedJlaExecutionReceipt {
    pub schema_version: u32,
    pub selected_engine: SelectedEngine,
    pub requested_solver_route: LinearSolverRoute,
    pub selected_solver_route: LinearSolverRoute,
    pub planned_rhs: u64,
    pub solver_setup: PreparedSolverReceipt,
    pub batch: CompressedJlaBatchReceipt,
    pub memory: JlaMemoryReceipt,
    pub wall: WallWorkReceipt,
    pub counter: CounterExecutionReceipt,
    pub plan_frozen_before_rng: bool,
    pub logical_atoms_before_plan_freeze: u64,
    pub unique_packed_words_before_plan_freeze: u64,
    pub physical_trials_before_plan_freeze: u64,
    pub threads: CompressedJlaThreadReceipt,
    pub full_cmg: Option<FullCmgReceipt>,
}

#[derive(Clone, Debug)]
pub struct PlannedJlaEngineResult {
    pub estimator: JlaEngineResult,
    pub execution: CompressedJlaExecutionReceipt,
}

/// Numerical Monte Carlo dispersion of the target-probe average. These four
/// values are not variance components and have no accounting identity.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct NumericalMcse {
    pub worker: f64,
    pub firm: f64,
    pub covariance: f64,
    pub total: f64,
}

#[derive(Clone, Copy, Debug, Default)]
struct StableSum {
    sum: f64,
    correction: f64,
}

impl StableSum {
    fn add(&mut self, value: f64) {
        let updated = self.sum + value;
        self.correction += if self.sum.abs() >= value.abs() {
            (self.sum - updated) + value
        } else {
            (value - updated) + self.sum
        };
        self.sum = updated;
    }

    fn finish(self) -> f64 {
        self.sum + self.correction
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct FiveMoments {
    projection: StableSum,
    residual: StableSum,
    projection_fourth: StableSum,
    residual_fourth: StableSum,
    mixed: StableSum,
}

impl FiveMoments {
    fn add(&mut self, projection: f64, residual: f64) {
        let projection_square = projection * projection;
        let residual_square = residual * residual;
        self.projection.add(projection_square);
        self.residual.add(residual_square);
        self.projection_fourth
            .add(projection_square * projection_square);
        self.residual_fourth.add(residual_square * residual_square);
        self.mixed.add(projection_square * residual_square);
    }
}

/// Run the source-bound no-control match-deletion improved-JLA estimator.
pub fn run_jla_no_controls(
    problem: &CompressedProblem,
    options: JlaEngineOptions,
) -> Result<JlaEngineResult> {
    let mut interrupt = NeverInterrupt;
    run_jla_no_controls_with_interrupt(problem, options, &mut interrupt)
}

pub fn run_jla_no_controls_with_interrupt(
    problem: &CompressedProblem,
    options: JlaEngineOptions,
    interrupt: &mut dyn InterruptCheck,
) -> Result<JlaEngineResult> {
    interrupt.checkpoint("jla_solve_entry")?;
    let options = options.validate()?;
    validate_problem_features(problem)?;
    let plan = JlaPlan::build_no_controls_with_interrupt(problem, interrupt)?;
    run_jla_no_controls_with_validated_plan(problem, &plan, options, interrupt)
}

/// Run with the authoritative preparation-time semantic plan so the complete
/// command does not retain and rebuild two identical large plans.
pub fn run_jla_no_controls_with_plan(
    problem: &CompressedProblem,
    plan: &JlaPlan,
    options: JlaEngineOptions,
) -> Result<JlaEngineResult> {
    let mut interrupt = NeverInterrupt;
    run_jla_no_controls_with_plan_and_interrupt(problem, plan, options, &mut interrupt)
}

pub fn run_jla_no_controls_with_plan_and_interrupt(
    problem: &CompressedProblem,
    plan: &JlaPlan,
    options: JlaEngineOptions,
    interrupt: &mut dyn InterruptCheck,
) -> Result<JlaEngineResult> {
    interrupt.checkpoint("jla_solve_entry")?;
    let options = options.validate()?;
    validate_problem_features(problem)?;
    run_jla_no_controls_with_validated_plan(problem, plan, options, interrupt)
}

pub fn run_jla_no_controls_planned(
    problem: &CompressedProblem,
    options: PlannedJlaEngineOptions,
) -> Result<PlannedJlaEngineResult> {
    run_jla_no_controls_planned_with_interrupt(problem, options, &mut NeverInterrupt)
}

pub fn run_jla_no_controls_planned_with_interrupt(
    problem: &CompressedProblem,
    options: PlannedJlaEngineOptions,
    interrupt: &mut dyn InterruptCheck,
) -> Result<PlannedJlaEngineResult> {
    interrupt.checkpoint("jla_planned_entry")?;
    let estimator = options.estimator.validate()?;
    if options.full_cmg.is_some()
        && (options.leverage_batch != BatchRequest::Auto
            || options.target_batch != BatchRequest::Auto)
    {
        return Err(BackendError::new(
            ErrorCode::UnsupportedFeature,
            "cmg_full_v2",
            "the full-CMG route requires automatic leverage and target batches",
        ));
    }
    validate_problem_features(problem)?;
    let plan = JlaPlan::build_no_controls_with_interrupt(problem, interrupt)?;
    plan.validate_against_problem(problem)?;
    validate_target_geometry(problem, &plan)?;
    preflight_trial_words("leverage", &plan.deletion.physical_count)?;
    preflight_trial_words("target", &plan.target.physical_count)?;
    let prepared = prepared_problem_bytes(problem, &plan)?;
    interrupt.checkpoint("jla_solver_setup")?;
    let full_cmg_plan = if let Some(full_cmg) = options.full_cmg {
        let maximum_width = usize::try_from(estimator.probes)
            .map_err(|_| memory_overflow("full-CMG probe count"))?
            .min(full_cmg.maximum_batch_rhs)
            .max(1);
        let mut memory_estimator = estimator;
        memory_estimator.solver.route = LinearSolverRoute::CmgPcg;
        memory_estimator.leverage_batch_width = maximum_width;
        memory_estimator.target_batch_width = maximum_width;
        let memory_preflight = forecast_jla_memory(problem, &plan, memory_estimator, prepared)?;
        Some(full_cmg.with_non_cmg_command_peak(memory_preflight.solve_peak_forecast_bytes))
    } else {
        None
    };
    let solver = if let Some(full_cmg) = full_cmg_plan {
        PreparedTwoWaySolver::prepare_full_cmg_v2_with_interrupt(
            problem,
            estimator.solver,
            full_cmg,
            interrupt,
        )?
    } else {
        PreparedTwoWaySolver::prepare_with_interrupt(problem, estimator.solver, interrupt)?
    };
    let solver_setup = solver.receipt().clone();
    let selected_solver_route = solver_setup.selected;
    let mut forecast_estimator = estimator;
    forecast_estimator.solver.route = selected_solver_route;
    let batch = plan_compressed_batches(
        problem,
        &plan,
        forecast_estimator,
        prepared,
        options.leverage_batch,
        options.target_batch,
    )?;
    let mut selected = estimator;
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
    forecast_selected.solver.route = selected_solver_route;
    let memory = admit_jla_memory(problem, &plan, forecast_selected, prepared)?;
    if memory.solve_peak_forecast_bytes != batch.plan.selected_command_peak_bytes {
        return Err(BackendError::invariant(
            "jla_plan",
            "selected compressed batch plan and final memory forecast disagree",
        ));
    }
    let wall = wall_work_receipt(
        compressed_jla_wall_work(problem, &plan, forecast_selected)?,
        options.wallseconds,
        WallCalibration::Uncalibrated,
    )?;
    let planned_counter = combine_counter_phases(
        plan_counter_phase(
            selected.probes,
            &plan.deletion.physical_count,
            GeneratorEvaluationModel::PackedWords,
        )?,
        plan_counter_phase(
            selected.probes,
            &plan.target.physical_count,
            GeneratorEvaluationModel::PackedWords,
        )?,
    )?;
    let estimator_result = run_jla_no_controls_with_prepared_solver(
        problem, &plan, selected, memory, &solver, interrupt,
    )?;
    let full_cmg = solver.full_cmg_receipt()?;
    let counter = combine_counter_phases(
        planned_counter.leverage.completed(),
        planned_counter.target.completed(),
    )?;
    Ok(PlannedJlaEngineResult {
        estimator: estimator_result,
        execution: CompressedJlaExecutionReceipt {
            schema_version: COMPRESSED_JLA_EXECUTION_SCHEMA_VERSION,
            selected_engine: SelectedEngine::Compressed,
            requested_solver_route: selected.solver.route,
            selected_solver_route,
            planned_rhs,
            solver_setup,
            batch,
            memory,
            wall,
            counter,
            plan_frozen_before_rng: true,
            logical_atoms_before_plan_freeze: 0,
            unique_packed_words_before_plan_freeze: 0,
            physical_trials_before_plan_freeze: 0,
            threads: CompressedJlaThreadReceipt {
                requested: full_cmg_plan.map_or(1, |plan| plan.threads),
                used: full_cmg.as_ref().map_or(1, |receipt| receipt.setup.threads),
                parallel_regions: full_cmg.as_ref().map_or(0, |receipt| {
                    usize::from(receipt.planned_batches > 0 || receipt.across_rhs_batches > 0)
                }),
            },
            full_cmg,
        },
    })
}

fn plan_compressed_batches(
    problem: &CompressedProblem,
    plan: &JlaPlan,
    estimator: JlaEngineOptions,
    prepared_persistent_bytes: u64,
    leverage_requested: BatchRequest,
    target_requested: BatchRequest,
) -> Result<CompressedJlaBatchReceipt> {
    let probes =
        usize::try_from(estimator.probes).map_err(|_| memory_overflow("JLA probe count"))?;
    let active_request = |requested| match requested {
        BatchRequest::Auto => BatchRequest::Auto,
        BatchRequest::Explicit(width) => BatchRequest::Explicit(width.min(probes)),
    };
    let leverage_active_request = active_request(leverage_requested);
    let target_active_request = active_request(target_requested);
    let width_one = forecast_jla_memory(
        problem,
        plan,
        JlaEngineOptions {
            leverage_batch_width: 1,
            target_batch_width: 1,
            ..estimator
        },
        prepared_persistent_bytes,
    )?;
    let non_batched_peak = width_one.non_batched_phase_forecast_bytes;

    let phase_cap = |request| match request {
        BatchRequest::Auto => COMPRESSED_JLA_AUTO_BATCH_WIDTH_CAP_V1.min(probes),
        BatchRequest::Explicit(width) => width,
    };
    let leverage_cap = phase_cap(leverage_active_request);
    let target_cap = phase_cap(target_active_request);
    let leverage_options = |width| JlaEngineOptions {
        leverage_batch_width: width,
        target_batch_width: 1,
        ..estimator
    };
    let target_options = |width| JlaEngineOptions {
        leverage_batch_width: 1,
        target_batch_width: width,
        ..estimator
    };

    // The common planner has one route cap for both phases. Run it once per
    // independent request so compressed auto retains its registered max-32
    // ladder even when the other phase has an explicit width above 32.
    let leverage_plan = plan_batches_with_forecasts(
        leverage_active_request,
        BatchRequest::Explicit(1),
        BatchPlannerCaps {
            probes,
            declared_threads: 1,
            columns_per_thread: leverage_cap,
            route_width_cap: leverage_cap,
            non_batched_peak_bytes: non_batched_peak,
            hard_memory_bytes: estimator.memory_limit_bytes,
        },
        |width| {
            Ok(forecast_jla_memory(
                problem,
                plan,
                leverage_options(width),
                prepared_persistent_bytes,
            )?
            .solve_peak_forecast_bytes)
        },
        |_| Ok(non_batched_peak),
    )?;
    let target_plan = plan_batches_with_forecasts(
        BatchRequest::Explicit(1),
        target_active_request,
        BatchPlannerCaps {
            probes,
            declared_threads: 1,
            columns_per_thread: target_cap,
            route_width_cap: target_cap,
            non_batched_peak_bytes: non_batched_peak,
            hard_memory_bytes: estimator.memory_limit_bytes,
        },
        |_| Ok(non_batched_peak),
        |width| {
            Ok(forecast_jla_memory(
                problem,
                plan,
                target_options(width),
                prepared_persistent_bytes,
            )?
            .solve_peak_forecast_bytes)
        },
    )?;
    let mut leverage = leverage_plan.leverage;
    leverage.requested = leverage_requested;
    let mut target = target_plan.target;
    target.requested = target_requested;
    let selected_command_peak_bytes = non_batched_peak
        .max(leverage.selected_forecast_bytes)
        .max(target.selected_forecast_bytes);
    let combined_plan = BatchPlanReceipt {
        schema_version: leverage_plan.schema_version,
        deterministic: leverage_plan.deterministic && target_plan.deterministic,
        bitwise_estimator_width_invariance_required: leverage_plan
            .bitwise_estimator_width_invariance_required
            && target_plan.bitwise_estimator_width_invariance_required,
        arithmetic_contract: leverage_plan.arithmetic_contract,
        non_batched_peak_bytes: non_batched_peak,
        selected_command_peak_bytes,
        whole_command_admitted: true,
        leverage,
        target,
    };
    Ok(CompressedJlaBatchReceipt {
        plan: combined_plan,
        leverage_requested,
        leverage_active_width: combined_plan.leverage.selected_width,
        target_requested,
        target_active_width: combined_plan.target.selected_width,
        automatic_ladder_cap: COMPRESSED_JLA_AUTO_BATCH_WIDTH_CAP_V1,
    })
}

fn compressed_jla_wall_work(
    problem: &CompressedProblem,
    plan: &JlaPlan,
    options: JlaEngineOptions,
) -> Result<WallWork> {
    let rows = to_u64_wall(problem.outcome.len(), "rows")?;
    let workers = to_u64_wall(problem.workers(), "workers")?;
    let firms = to_u64_wall(problem.firms(), "firms")?;
    let cells = to_u64_wall(problem.cells(), "cells")?;
    let deletion = to_u64_wall(plan.deletion_units(), "deletion units")?;
    let target = to_u64_wall(plan.target_strata(), "target strata")?;
    let parameters = workers
        .checked_add(firms.saturating_sub(1))
        .ok_or_else(|| wall_overflow("identified parameter count"))?;
    let fit_terms = parameters
        .checked_add(1)
        .ok_or_else(|| wall_overflow("full-fit terms"))?;
    let probes = u64::from(options.probes);
    let engine_setup = match options.solver.route {
        LinearSolverRoute::Exact => {
            wall_product(&[parameters, parameters, parameters], "exact engine setup")?
        }
        LinearSolverRoute::DiagonalPcg => wall_sum(
            &[wall_product(&[cells, 3], "diagonal setup cells")?, firms],
            "diagonal engine setup",
        )?,
        LinearSolverRoute::CmgPcg => wall_sum(
            &[
                wall_product(&[cells, 6], "CMG setup cells")?,
                workers,
                firms,
            ],
            "CMG engine setup",
        )?,
        LinearSolverRoute::Auto => {
            return Err(BackendError::invariant(
                "jla_wall",
                "compressed wall work received an unresolved automatic solver route",
            ));
        }
    };
    Ok(WallWork {
        preparation: wall_product(&[rows, 4], "preparation")?,
        engine_setup,
        full_fit: wall_product(&[rows, fit_terms], "full fit")?,
        leverage: wall_product(&[probes, deletion, 2], "leverage")?,
        target: wall_product(&[probes, target, 2], "target")?,
        result_export: wall_sum(&[cells, deletion, probes], "result export")?,
    })
}

fn to_u64_wall(value: usize, label: &str) -> Result<u64> {
    u64::try_from(value).map_err(|_| wall_overflow(label))
}

fn wall_product(values: &[u64], label: &str) -> Result<u64> {
    values.iter().try_fold(1_u64, |total, &value| {
        total.checked_mul(value).ok_or_else(|| wall_overflow(label))
    })
}

fn wall_sum(values: &[u64], label: &str) -> Result<u64> {
    values.iter().try_fold(0_u64, |total, &value| {
        total.checked_add(value).ok_or_else(|| wall_overflow(label))
    })
}

fn wall_overflow(label: &str) -> BackendError {
    BackendError::new(
        ErrorCode::ResourceLimit,
        "jla_wall",
        format!("{label} wall-work overflow"),
    )
}

fn validate_problem_features(problem: &CompressedProblem) -> Result<()> {
    if !problem.controls.is_empty() {
        return Err(BackendError::new(
            ErrorCode::UnsupportedFeature,
            "jla_validate",
            "the Rust JLA engine supports no-control problems only",
        ));
    }
    Ok(())
}

fn run_jla_no_controls_with_validated_plan(
    problem: &CompressedProblem,
    plan: &JlaPlan,
    options: JlaEngineOptions,
    interrupt: &mut dyn InterruptCheck,
) -> Result<JlaEngineResult> {
    // All unsupported features, tuning, semantic plans, scatter identities,
    // target centering geometry, memory admission, and solver setup are
    // settled before the counter generator is instantiated or any estimator
    // atom is addressed.
    plan.validate_against_problem(problem)?;
    validate_target_geometry(problem, plan)?;
    preflight_trial_words("leverage", &plan.deletion.physical_count)?;
    preflight_trial_words("target", &plan.target.physical_count)?;
    let memory = admit_jla_memory(
        problem,
        plan,
        options,
        prepared_problem_bytes(problem, plan)?,
    )?;
    interrupt.checkpoint("jla_solver_setup")?;
    let solver = PreparedTwoWaySolver::prepare_with_interrupt(problem, options.solver, interrupt)?;

    run_jla_no_controls_with_prepared_solver(problem, plan, options, memory, &solver, interrupt)
}

fn run_jla_no_controls_with_prepared_solver<'a>(
    problem: &'a CompressedProblem,
    plan: &JlaPlan,
    options: JlaEngineOptions,
    memory: JlaMemoryReceipt,
    solver: &PreparedTwoWaySolver<'a>,
    interrupt: &mut dyn InterruptCheck,
) -> Result<JlaEngineResult> {
    interrupt.checkpoint("jla_full_fit")?;
    let (outcome_worker_rhs, outcome_firm_rhs) =
        solver.operator().outcome_rhs_with_interrupt(interrupt)?;
    let full_fit = solver
        .solve_with_interrupt(&outcome_worker_rhs, &outcome_firm_rhs, interrupt)
        .map_err(|error| rhs_error(error, JlaSolvePhase::FullFit, None, JlaRhsSide::Joint))?;
    let full_fit_receipt = rhs_receipt(
        &full_fit.receipt,
        full_fit.solution.residual.relative_norm,
        full_fit.solution.residual.rhs_norm,
        JlaSolvePhase::FullFit,
        None,
        JlaRhsSide::Joint,
    )?;
    let fitted_cell = cell_predictions_with_interrupt(
        problem,
        &full_fit.solution.worker,
        &full_fit.solution.firm,
        interrupt,
    )?;
    let weighted_rss = full_fit_weighted_rss_with_interrupt(problem, &fitted_cell, interrupt)?;
    let plugin = plugin_components_with_interrupt(
        problem,
        &full_fit.solution.worker,
        &full_fit.solution.firm,
        interrupt,
    )?;

    let rng = CounterRng::new(options.seed);
    let groups = plan.deletion_units();
    let mut moments = vec![FiveMoments::default(); groups];
    let mut leverage_receipts = Vec::with_capacity(options.probes as usize);
    for first in (0..options.probes as usize).step_by(options.leverage_batch_width) {
        interrupt.checkpoint("jla_leverage_batch")?;
        let width = options
            .leverage_batch_width
            .min(options.probes as usize - first);
        let atoms = rademacher_atoms_ordered_with_interrupt(
            rng,
            ProbeDomain::Leverage,
            first,
            width,
            &plan.deletion.semantic_rank,
            &plan.deletion.physical_count,
            solver,
            interrupt,
        )?;
        let (worker_rhs, firm_rhs) =
            leverage_rhs_with_interrupt(problem, plan, &atoms, width, interrupt)?;
        let solved = solver
            .solve_batch_with_interrupt(&worker_rhs, &firm_rhs, width, interrupt)
            .map_err(|error| {
                contextual_batch_error(error, JlaSolvePhase::Leverage, first, false)
            })?;
        for column in 0..width {
            interrupt.checkpoint("jla_leverage_probe")?;
            let probe = first + column;
            let solution = &solved.solution[column];
            leverage_receipts.push(rhs_receipt(
                &solved.receipt[column],
                solution.residual.relative_norm,
                solution.residual.rhs_norm,
                JlaSolvePhase::Leverage,
                Some(probe as u64),
                JlaRhsSide::Joint,
            )?);
        }
        interrupt.checkpoint("jla_leverage_moment_blocks")?;
        let blocks = moments
            .chunks_mut(LEVERAGE_MOMENT_BLOCK_GROUPS)
            .enumerate()
            .collect::<Vec<_>>();
        let updated = solver.map_independent_ordered(blocks, |(block_index, block)| {
            let first_group = block_index * LEVERAGE_MOMENT_BLOCK_GROUPS;
            for column in 0..width {
                let probe = first + column;
                let solution = &solved.solution[column];
                for (local_group, moment) in block.iter_mut().enumerate() {
                    let group = first_group + local_group;
                    let cell = usize::try_from(plan.deletion.cell[group])
                        .expect("validated deletion cell");
                    let frequency = plan.deletion.physical_count[group] as f64;
                    let worker = usize::try_from(problem.cell_worker[cell]).expect("worker");
                    let firm = usize::try_from(problem.cell_firm[cell]).expect("firm");
                    let prediction = solution.worker[worker] + solution.firm[firm];
                    let projection = frequency.sqrt() * prediction;
                    let residual =
                        atoms[column * groups + group] as f64 / frequency.sqrt() - projection;
                    if !projection.is_finite() || !residual.is_finite() {
                        return Err(BackendError::new(
                            ErrorCode::JlaMomentFailed,
                            "jla_leverage",
                            format!(
                                "nonfinite leverage moment at probe {probe}, side joint, unit {group}"
                            ),
                        ));
                    }
                    moment.add(projection, residual);
                }
            }
            Ok(())
        });
        for result in updated {
            interrupt.checkpoint("jla_leverage_moment_block_complete")?;
            result?;
        }
    }

    interrupt.checkpoint("jla_leverage_adjustment")?;
    let adjustment = leverage_adjustment_with_interrupt(
        problem,
        plan,
        &fitted_cell,
        &moments,
        options,
        interrupt,
    )?;
    drop(moments);
    let mut target_draws = vec![VarianceComponents::default(); options.probes as usize];
    let mut target_receipts = Vec::with_capacity(2 * options.probes as usize);
    for first in (0..options.probes as usize).step_by(options.target_batch_width) {
        interrupt.checkpoint("jla_target_batch")?;
        let width = options
            .target_batch_width
            .min(options.probes as usize - first);
        let atoms = rademacher_atoms_ordered_with_interrupt(
            rng,
            ProbeDomain::Target,
            first,
            width,
            &plan.target.semantic_rank,
            &plan.target.physical_count,
            solver,
            interrupt,
        )?;
        let (worker_rhs, firm_rhs) = target_rhs_columns_with_interrupt(
            problem, plan, &atoms, width, first, solver, interrupt,
        )?;
        let solved = solver
            .solve_batch_with_interrupt(&worker_rhs, &firm_rhs, 2 * width, interrupt)
            .map_err(|error| contextual_batch_error(error, JlaSolvePhase::Target, first, true))?;
        for column in 0..width {
            interrupt.checkpoint("jla_target_probe")?;
            let probe = first + column;
            let worker_solution = &solved.solution[2 * column];
            let firm_solution = &solved.solution[2 * column + 1];
            target_receipts.push(rhs_receipt(
                &solved.receipt[2 * column],
                worker_solution.residual.relative_norm,
                worker_solution.residual.rhs_norm,
                JlaSolvePhase::Target,
                Some(probe as u64),
                JlaRhsSide::Worker,
            )?);
            target_receipts.push(rhs_receipt(
                &solved.receipt[2 * column + 1],
                firm_solution.residual.relative_norm,
                firm_solution.residual.rhs_norm,
                JlaSolvePhase::Target,
                Some(probe as u64),
                JlaRhsSide::Firm,
            )?);
        }
        interrupt.checkpoint("jla_target_contraction_chunk")?;
        let contracted = solver.map_independent_ordered((0..width).collect(), |column| {
            let probe = first + column;
            let worker_solution = &solved.solution[2 * column];
            let firm_solution = &solved.solution[2 * column + 1];
            let mut worker_interrupt = NeverInterrupt;
            contract_target_solutions_with_interrupt(
                problem,
                &adjustment.cell_correction_weight,
                &worker_solution.worker,
                &worker_solution.firm,
                &firm_solution.worker,
                &firm_solution.firm,
                probe,
                &mut worker_interrupt,
            )
        });
        for (column, draw) in contracted.into_iter().enumerate() {
            interrupt.checkpoint("jla_target_contraction_complete")?;
            target_draws[first + column] = draw?;
        }
    }

    interrupt.checkpoint("jla_finalize")?;
    let correction = mean_components_with_interrupt(&target_draws, interrupt)?;
    let corrected = subtract_components(plugin, correction)?;
    let numerical_mcse = component_mcse_with_interrupt(&target_draws, interrupt)?;
    let accounting_residual = accounting_residuals_with_interrupt(
        plugin,
        correction,
        corrected,
        &target_draws,
        interrupt,
    )?;
    let (max_reduced_residual, max_complete_residual) = residual_maxima_with_interrupt(
        &full_fit_receipt,
        &leverage_receipts,
        &target_receipts,
        interrupt,
    )?;
    let receipt = JlaEngineReceipt {
        seed: options.seed,
        rng: options.rng,
        probes_requested: options.probes,
        leverage_probes_accepted: options.probes,
        target_probes_accepted: options.probes,
        leverage_batch_width: options.leverage_batch_width,
        target_batch_width: options.target_batch_width,
        rank_tolerance: options.rank_tolerance,
        block_tolerance: options.block_tolerance,
        full_residual_tolerance: solver.maximum_full_residual_tolerance(),
        solver: solver.receipt().clone(),
        full_fit: full_fit_receipt,
        leverage_rhs: leverage_receipts,
        target_rhs: target_receipts,
        max_reduced_residual,
        max_complete_residual,
        max_leverage: maximum_with_interrupt(
            &adjustment.projection_share,
            "jla_finalize_leverage",
            interrupt,
        )?,
        max_reciprocal_residual: adjustment.max_reciprocal_residual,
        accounting_residual,
        topology_checksum: problem.topology_checksum,
        memory,
    };
    Ok(JlaEngineResult {
        plugin,
        correction,
        corrected,
        numerical_mcse,
        weighted_rss,
        fitted_cell,
        unit_projection_share: adjustment.projection_share,
        unit_residual_share: adjustment.residual_share,
        unit_finite_bias: adjustment.finite_bias,
        unit_finite_variance: adjustment.finite_variance,
        unit_residual_mass: adjustment.residual_mass,
        unit_deleted_mass: adjustment.deleted_mass,
        cell_correction_weight: adjustment.cell_correction_weight,
        target_draws,
        receipt,
    })
}

/// Direct heap bytes retained by the compressed problem and semantic plan.
/// Capacities, rather than logical lengths, are charged so growth slack is
/// part of the admitted resident allocation.
pub fn compressed_problem_bytes(problem: &CompressedProblem) -> Result<u64> {
    let mut total = u64::try_from(size_of::<CompressedProblem>())
        .map_err(|_| memory_overflow("prepared structure size"))?;
    macro_rules! charge {
        ($value:expr) => {
            total = checked_memory_add(total, vec_allocation_bytes($value)?)?;
        };
    }
    charge!(&problem.retained_rows);
    charge!(&problem.row_worker);
    charge!(&problem.row_firm);
    charge!(&problem.row_deletion);
    charge!(&problem.row_cell);
    charge!(&problem.row_target);
    charge!(&problem.outcome);
    charge!(&problem.frequency);
    charge!(&problem.target_weight);
    for control in &problem.controls {
        charge!(control);
    }
    if let Some(probe_order) = &problem.probe_order {
        charge!(probe_order);
    }
    charge!(&problem.cell_worker);
    charge!(&problem.cell_firm);
    charge!(&problem.cell_weight);
    charge!(&problem.cell_outcome_sum);
    charge!(&problem.cell_target_sum);
    for index in [
        &problem.worker_index,
        &problem.firm_index,
        &problem.deletion_index,
        &problem.target_index,
    ] {
        charge!(&index.ptr);
        charge!(&index.items);
    }
    Ok(total)
}

pub fn prepared_problem_bytes(problem: &CompressedProblem, plan: &JlaPlan) -> Result<u64> {
    let mut total = checked_memory_add(
        compressed_problem_bytes(problem)?,
        u64::try_from(size_of::<JlaPlan>())
            .map_err(|_| memory_overflow("prepared JLA plan structure size"))?,
    )?;
    macro_rules! charge {
        ($value:expr) => {
            total = checked_memory_add(total, vec_allocation_bytes($value)?)?;
        };
    }
    charge!(&plan.row_semantic_rank);
    charge!(&plan.deletion.cell);
    charge!(&plan.deletion.physical_count);
    charge!(&plan.deletion.outcome_sum);
    charge!(&plan.deletion.target_mass);
    charge!(&plan.deletion.semantic_rank);
    charge!(&plan.target.cell);
    charge!(&plan.target.per_copy_mass);
    charge!(&plan.target.physical_count);
    charge!(&plan.target.target_mass);
    charge!(&plan.target.semantic_rank);
    charge!(&plan.target.row_to_stratum);
    charge!(&plan.target.row_index.ptr);
    charge!(&plan.target.row_index.items);
    Ok(total)
}

fn admit_jla_memory(
    problem: &CompressedProblem,
    plan: &JlaPlan,
    options: JlaEngineOptions,
    prepared_persistent_bytes: u64,
) -> Result<JlaMemoryReceipt> {
    let memory = forecast_jla_memory(problem, plan, options, prepared_persistent_bytes)?;
    if memory.solve_peak_forecast_bytes > options.memory_limit_bytes {
        return Err(BackendError::new(
            ErrorCode::ResourceLimit,
            "jla_memory",
            format!(
                "whole-command Rust solve forecast {} bytes exceeds the declared limit {} bytes",
                memory.solve_peak_forecast_bytes, options.memory_limit_bytes
            ),
        ));
    }
    Ok(memory)
}

fn forecast_jla_memory(
    problem: &CompressedProblem,
    plan: &JlaPlan,
    options: JlaEngineOptions,
    prepared_persistent_bytes: u64,
) -> Result<JlaMemoryReceipt> {
    let prepared_persistent_bytes =
        prepared_persistent_bytes.max(options.prepared_persistent_bytes);
    let workers = to_u64_memory(problem.workers(), "workers")?;
    let firms = to_u64_memory(problem.firms(), "firms")?;
    let cells = to_u64_memory(problem.cells(), "cells")?;
    let deletion = to_u64_memory(problem.deletion_units(), "deletion units")?;
    let target = to_u64_memory(plan.target_strata(), "target strata")?;
    let probes = u64::from(options.probes);
    let leverage_width = probes.min(to_u64_memory(
        options.leverage_batch_width,
        "leverage batch width",
    )?);
    let target_width = probes.min(to_u64_memory(
        options.target_batch_width,
        "target batch width",
    )?);
    let route = forecast_route(firms, options.solver);
    let cmg_batch_workspace_per_column = if route == LinearSolverRoute::CmgPcg {
        cmg_batch_workspace_per_column_forecast(problem, options.solver)?
    } else {
        0
    };

    let operator_bytes = memory_product(
        &[
            workers
                .checked_add(
                    firms
                        .checked_mul(2)
                        .ok_or_else(|| memory_overflow("operator firms"))?,
                )
                .ok_or_else(|| memory_overflow("operator entries"))?,
            8,
        ],
        "operator storage",
    )?;
    let backend_bytes = match route {
        LinearSolverRoute::Exact => 0,
        LinearSolverRoute::DiagonalPcg => memory_product(&[firms, 8], "diagonal preconditioner")?,
        LinearSolverRoute::CmgPcg => cmg_setup_forecast(problem, options.solver)?,
        LinearSolverRoute::Auto => {
            return Err(BackendError::invariant(
                "jla_memory",
                "memory forecast retained an unresolved automatic route",
            ));
        }
    };
    let solver_setup_forecast_bytes = checked_memory_add(operator_bytes, backend_bytes)?;
    let result_forecast_bytes = checked_memory_sum(&[
        memory_product(&[cells, 16], "fitted and correction cells")?,
        memory_product(&[deletion, 48], "deletion adjustment result")?,
        memory_product(&[probes, 32], "target draws")?,
        memory_product(
            &[
                probes,
                to_u64_memory(size_of::<JlaRhsReceipt>(), "RHS receipt size")?,
            ],
            "leverage receipts",
        )?,
        memory_product(
            &[
                probes
                    .checked_mul(2)
                    .ok_or_else(|| memory_overflow("target receipt count"))?,
                to_u64_memory(size_of::<JlaRhsReceipt>(), "RHS receipt size")?,
            ],
            "target receipts",
        )?,
        // The Stata adapter materializes one lossless eight-double row and a
        // transient 48-byte C ABI row for the full fit, every leverage RHS,
        // and both target sides per probe. Both caller-owned copies coexist
        // with the retained Rust result and therefore belong in the
        // pre-allocation whole-command forecast (64 + 48 = 112 bytes/row).
        memory_product(
            &[
                probes
                    .checked_mul(3)
                    .and_then(|value| value.checked_add(1))
                    .ok_or_else(|| memory_overflow("caller RHS receipt rows"))?,
                14,
                8,
            ],
            "caller RHS receipt matrix",
        )?,
    ])?;
    let full_fit_phase = checked_memory_sum(&[
        memory_product(
            &[
                workers
                    .checked_add(firms)
                    .ok_or_else(|| memory_overflow("full RHS"))?,
                8,
            ],
            "full-fit RHS",
        )?,
        solver_batch_forecast(route, workers, firms, 1, cmg_batch_workspace_per_column)?,
    ])?;
    let leverage_moment_blocks = deletion
        .div_ceil(u64::try_from(LEVERAGE_MOMENT_BLOCK_GROUPS).expect("moment block size fits u64"));
    let leverage_moment_block_metadata = to_u64_memory(
        size_of::<(usize, &mut [FiveMoments])>() + size_of::<Result<()>>(),
        "leverage moment block metadata size",
    )?;
    let rng_block_metadata = to_u64_memory(
        size_of::<(usize, &mut [i64])>() + size_of::<Result<()>>(),
        "RNG block metadata size",
    )?;
    let leverage_rng_blocks = leverage_width
        .div_ceil(u64::try_from(RNG_PROBE_BLOCK_COLUMNS).expect("RNG block columns fit u64"));
    let leverage_phase_forecast_bytes = checked_memory_sum(&[
        memory_product(
            &[
                deletion,
                to_u64_memory(size_of::<FiveMoments>(), "moment size")?,
            ],
            "leverage moments",
        )?,
        memory_product(
            &[leverage_moment_blocks, leverage_moment_block_metadata],
            "leverage moment block metadata",
        )?,
        memory_product(&[deletion, leverage_width, 8], "leverage atoms")?,
        memory_product(
            &[leverage_rng_blocks, rng_block_metadata],
            "leverage RNG block metadata",
        )?,
        memory_product(
            &[
                workers
                    .checked_add(firms)
                    .ok_or_else(|| memory_overflow("leverage RHS"))?,
                leverage_width,
                8,
            ],
            "leverage RHS",
        )?,
        solver_batch_forecast(
            route,
            workers,
            firms,
            leverage_width,
            cmg_batch_workspace_per_column,
        )?,
    ])?;
    let target_phase_forecast_bytes = target_phase_forecast(
        route,
        workers,
        firms,
        cells,
        target,
        target_width,
        cmg_batch_workspace_per_column,
    )?;
    let largest_phase = full_fit_phase
        .max(leverage_phase_forecast_bytes)
        .max(target_phase_forecast_bytes);
    let non_batched_phase_forecast_bytes = checked_memory_sum(&[
        prepared_persistent_bytes,
        solver_setup_forecast_bytes,
        result_forecast_bytes,
        full_fit_phase,
    ])?;
    let solve_peak_forecast_bytes = checked_memory_sum(&[
        prepared_persistent_bytes,
        solver_setup_forecast_bytes,
        result_forecast_bytes,
        largest_phase,
    ])?;
    Ok(JlaMemoryReceipt {
        hard_limit_bytes: options.memory_limit_bytes,
        prepared_persistent_bytes,
        solver_setup_forecast_bytes,
        leverage_phase_forecast_bytes,
        target_phase_forecast_bytes,
        result_forecast_bytes,
        non_batched_phase_forecast_bytes,
        solve_peak_forecast_bytes,
    })
}

fn target_phase_forecast(
    route: LinearSolverRoute,
    workers: u64,
    firms: u64,
    cells: u64,
    target_strata: u64,
    target_width: u64,
    cmg_batch_workspace_per_column: u64,
) -> Result<u64> {
    let doubled_target_width = target_width
        .checked_mul(2)
        .ok_or_else(|| memory_overflow("paired target width"))?;
    let target_rng_blocks = target_width
        .div_ceil(u64::try_from(RNG_PROBE_BLOCK_COLUMNS).expect("RNG block columns fit u64"));
    let rng_block_metadata = to_u64_memory(
        size_of::<(usize, &mut [i64])>() + size_of::<Result<()>>(),
        "RNG block metadata size",
    )?;
    checked_memory_sum(&[
        memory_product(&[target_strata, target_width, 8], "target atoms")?,
        memory_product(
            &[target_rng_blocks, rng_block_metadata],
            "target RNG block metadata",
        )?,
        memory_product(&[cells, target_width, 8], "target directions")?,
        memory_product(&[target_width, 8], "target reference scale")?,
        memory_product(
            &[
                workers
                    .checked_add(firms)
                    .ok_or_else(|| memory_overflow("target RHS"))?,
                doubled_target_width,
                8,
            ],
            "paired target RHS",
        )?,
        solver_batch_forecast(
            route,
            workers,
            firms,
            doubled_target_width,
            cmg_batch_workspace_per_column,
        )?,
    ])
}

fn forecast_route(firms: u64, options: LinearSolverOptions) -> LinearSolverRoute {
    let parameters = firms.saturating_sub(1);
    match options.route {
        LinearSolverRoute::Auto if parameters <= options.exact_dimension_limit as u64 => {
            LinearSolverRoute::Exact
        }
        LinearSolverRoute::Auto if parameters < options.cmg_minimum_dimension as u64 => {
            LinearSolverRoute::DiagonalPcg
        }
        LinearSolverRoute::Auto => LinearSolverRoute::CmgPcg,
        route => route,
    }
}

fn solver_batch_forecast(
    route: LinearSolverRoute,
    workers: u64,
    firms: u64,
    columns: u64,
    cmg_batch_workspace_per_column: u64,
) -> Result<u64> {
    let solution_entries = workers
        .checked_mul(2)
        .and_then(|value| value.checked_add(firms.checked_mul(3)?))
        .ok_or_else(|| memory_overflow("solution entries"))?;
    let retained_solution = memory_product(
        &[solution_entries, columns, 8],
        "batched retained solutions",
    )?;
    match route {
        LinearSolverRoute::Exact => {
            let dense = memory_product(&[firms, firms, 16], "exact matrix and factor")?;
            checked_memory_add(retained_solution, dense)
        }
        LinearSolverRoute::DiagonalPcg | LinearSolverRoute::CmgPcg => {
            // Batched PCG retains seven full-F Krylov matrices plus the
            // worker-elimination workspace and complete solutions.
            let krylov = memory_product(&[firms, columns, 7, 8], "batched Krylov workspace")?;
            let elimination = memory_product(
                &[
                    workers
                        .checked_mul(2)
                        .and_then(|value| value.checked_add(firms.checked_mul(2)?))
                        .ok_or_else(|| memory_overflow("worker elimination entries"))?,
                    columns,
                    8,
                ],
                "worker elimination workspace",
            )?;
            let cmg_batch = if route == LinearSolverRoute::CmgPcg {
                memory_product(
                    &[cmg_batch_workspace_per_column, columns],
                    "batched CMG workspace",
                )?
            } else {
                0
            };
            checked_memory_sum(&[retained_solution, krylov, elimination, cmg_batch])
        }
        LinearSolverRoute::Auto => Err(BackendError::invariant(
            "jla_memory",
            "batch memory forecast received automatic route",
        )),
    }
}

fn cmg_batch_workspace_per_column_forecast(
    problem: &CompressedProblem,
    options: LinearSolverOptions,
) -> Result<u64> {
    let fine_vertices = to_u64_memory(
        problem
            .firms()
            .checked_add(problem.workers())
            .ok_or_else(|| memory_overflow("CMG batch fine vertices"))?,
        "CMG batch fine vertices",
    )?;
    let hierarchy_vertices = scaled_count(fine_vertices, options.cmg.maximum_vertex_complexity)?;
    checked_memory_sum(&[
        memory_product(
            &[hierarchy_vertices, 6, 8],
            "CMG batch hierarchy workspace per column",
        )?,
        memory_product(&[fine_vertices, 2, 8], "CMG batch full vectors per column")?,
        memory_product(
            &[
                to_u64_memory(options.cmg.maximum_levels, "CMG maximum levels")?,
                8,
            ],
            "CMG batch column sums per column",
        )?,
    ])
}

fn cmg_setup_forecast(problem: &CompressedProblem, options: LinearSolverOptions) -> Result<u64> {
    let vertices = to_u64_memory(
        problem
            .firms()
            .checked_add(problem.workers())
            .ok_or_else(|| memory_overflow("CMG vertices"))?,
        "CMG vertices",
    )?;
    let edges = to_u64_memory(problem.cells(), "CMG cells")?
        .checked_mul(3)
        .ok_or_else(|| memory_overflow("CMG edge bound"))?;
    let hierarchy_vertices = scaled_count(vertices, options.cmg.maximum_vertex_complexity)?;
    let hierarchy_edges = scaled_count(edges, options.cmg.maximum_edge_complexity)?;
    let hybrid = checked_memory_sum(&[
        memory_product(&[vertices, 52], "CMG hybrid vertices")?,
        memory_product(&[edges, 40], "CMG hybrid edges")?,
    ])?;
    let hierarchy = checked_memory_sum(&[
        memory_product(&[hierarchy_vertices, 84], "CMG hierarchy vertices")?,
        memory_product(&[hierarchy_edges, 24], "CMG hierarchy edges")?,
    ])?;
    let terminal = to_u64_memory(
        options.cmg.dense_vertex_cap.saturating_sub(1),
        "CMG dense cap",
    )?;
    let dense_setup = memory_product(&[terminal, terminal, 16], "CMG dense setup")?;
    let admitted_hierarchy =
        checked_memory_add(hierarchy, dense_setup)?.min(options.cmg.memory_limit_bytes);
    checked_memory_add(hybrid, admitted_hierarchy)
}

fn scaled_count(value: u64, factor: f64) -> Result<u64> {
    let scaled = (value as f64) * factor;
    if !scaled.is_finite() || scaled > u64::MAX as f64 {
        return Err(memory_overflow("scaled hierarchy count"));
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    Ok(scaled.ceil() as u64)
}

fn vec_allocation_bytes<T>(value: &Vec<T>) -> Result<u64> {
    memory_product(
        &[
            to_u64_memory(value.capacity(), "vector capacity")?,
            to_u64_memory(size_of::<T>(), "element size")?,
        ],
        "vector allocation",
    )
}

fn memory_product(values: &[u64], label: &str) -> Result<u64> {
    values.iter().try_fold(1_u64, |total, value| {
        total
            .checked_mul(*value)
            .ok_or_else(|| memory_overflow(label))
    })
}

fn checked_memory_sum(values: &[u64]) -> Result<u64> {
    values
        .iter()
        .try_fold(0_u64, |total, value| checked_memory_add(total, *value))
}

fn checked_memory_add(left: u64, right: u64) -> Result<u64> {
    left.checked_add(right)
        .ok_or_else(|| memory_overflow("memory total"))
}

fn to_u64_memory(value: usize, label: &str) -> Result<u64> {
    u64::try_from(value).map_err(|_| memory_overflow(label))
}

fn memory_overflow(label: &str) -> BackendError {
    BackendError::new(
        ErrorCode::ResourceLimit,
        "jla_memory",
        format!("{label} overflow"),
    )
}

fn full_fit_weighted_rss_with_interrupt(
    problem: &CompressedProblem,
    fitted_cell: &[f64],
    interrupt: &mut dyn InterruptCheck,
) -> Result<f64> {
    if fitted_cell.len() != problem.cells() {
        return Err(BackendError::invalid(
            "jla_full_fit",
            "full-fit cell prediction has the wrong dimension",
        ));
    }
    let mut rss = StableSum::default();
    for row in 0..problem.outcome.len() {
        checkpoint_chunk(interrupt, row, "jla_full_fit_rss")?;
        let cell = usize::try_from(problem.row_cell[row]).expect("validated cell");
        let residual = problem.outcome[row] - fitted_cell[cell];
        let contribution = problem.frequency[row] as f64 * residual * residual;
        if !contribution.is_finite() || contribution < 0.0 {
            return Err(BackendError::new(
                ErrorCode::CorrectionNonFinite,
                "jla_full_fit",
                format!("weighted RSS contribution is invalid at retained row {row}"),
            ));
        }
        rss.add(contribution);
    }
    let value = rss.finish();
    if !value.is_finite() || value < 0.0 {
        return Err(BackendError::new(
            ErrorCode::CorrectionNonFinite,
            "jla_full_fit",
            "weighted RSS is invalid",
        ));
    }
    Ok(value)
}

#[derive(Debug)]
struct LeverageAdjustment {
    projection_share: Vec<f64>,
    residual_share: Vec<f64>,
    finite_bias: Vec<f64>,
    finite_variance: Vec<f64>,
    residual_mass: Vec<f64>,
    deleted_mass: Vec<f64>,
    cell_correction_weight: Vec<f64>,
    max_reciprocal_residual: f64,
}

fn leverage_adjustment_with_interrupt(
    problem: &CompressedProblem,
    plan: &JlaPlan,
    fitted_cell: &[f64],
    moments: &[FiveMoments],
    options: JlaEngineOptions,
    interrupt: &mut dyn InterruptCheck,
) -> Result<LeverageAdjustment> {
    let probes = f64::from(options.probes);
    let groups = plan.deletion_units();
    let mut projection_share = Vec::with_capacity(groups);
    let mut residual_share = Vec::with_capacity(groups);
    let mut finite_bias = Vec::with_capacity(groups);
    let mut finite_variance = Vec::with_capacity(groups);
    let mut residual_mass = Vec::with_capacity(groups);
    let mut deleted_mass = Vec::with_capacity(groups);
    let mut cell_weight = vec![StableSum::default(); problem.cells()];
    let mut max_reciprocal_residual = 0.0_f64;
    for group in 0..groups {
        checkpoint_chunk(interrupt, group, "jla_leverage_adjustment")?;
        let moment = moments[group];
        let p_first = moment.projection.finish();
        let m_first = moment.residual.finish();
        let total_mean = (p_first + m_first) / probes;
        if !total_mean.is_finite() || total_mean <= options.block_tolerance {
            return Err(BackendError::new(
                ErrorCode::JlaConstraintFailed,
                "jla_leverage",
                format!(
                    "projection/residual mass failed at unit {group}, probe summary, side joint"
                ),
            ));
        }
        let projection = (p_first / probes) / total_mean;
        let residual = (m_first / probes) / total_mean;
        let p_second = moment.projection_fourth.finish() / probes;
        let m_second = moment.residual_fourth.finish() / probes;
        let mixed = moment.mixed.finish() / probes;
        let bias = (residual * p_second - projection * m_second + (residual - projection) * mixed)
            / probes;
        let mut variance = (residual * residual * p_second + projection * projection * m_second
            - 2.0 * projection * residual * mixed)
            / probes;
        if [projection, residual, bias, variance]
            .iter()
            .any(|value| !value.is_finite())
        {
            return Err(BackendError::new(
                ErrorCode::JlaMomentFailed,
                "jla_leverage",
                format!("finite-projection moment is nonfinite at unit {group}, side joint"),
            ));
        }
        if variance < -100.0 * options.rank_tolerance {
            return Err(BackendError::new(
                ErrorCode::JlaMomentFailed,
                "jla_leverage",
                format!("finite-projection variance is negative at unit {group}, side joint"),
            ));
        }
        variance = variance.max(0.0);
        let maker_residual = 1.0 - projection;
        if maker_residual <= options.block_tolerance {
            return Err(BackendError::new(
                ErrorCode::NonestimableDeletion,
                "jla_leverage",
                format!("match residual block is singular at unit {group}, side joint"),
            ));
        }
        let reciprocal = residual.recip();
        let reciprocal_residual = (residual * reciprocal - 1.0).abs();
        if !reciprocal.is_finite()
            || !reciprocal_residual.is_finite()
            || reciprocal_residual > (100.0 * options.rank_tolerance).max(1.0e-10)
        {
            return Err(BackendError::new(
                ErrorCode::BlockInverseFailed,
                "jla_leverage",
                format!("residual inverse gate failed at unit {group}, side joint"),
            ));
        }
        let cell = usize::try_from(plan.deletion.cell[group]).expect("validated cell");
        let residual_outcome = plan.deletion.outcome_sum[group]
            - plan.deletion.physical_count[group] as f64 * fitted_cell[cell];
        let multiplier = reciprocal + bias * reciprocal.powi(2) - variance * reciprocal.powi(3);
        let deleted = residual_outcome * multiplier;
        if !residual_outcome.is_finite() || !multiplier.is_finite() || !deleted.is_finite() {
            return Err(BackendError::new(
                ErrorCode::CorrectionNonFinite,
                "jla_leverage",
                format!("match correction is nonfinite at unit {group}, side joint"),
            ));
        }
        cell_weight[cell].add(plan.deletion.outcome_sum[group] * deleted);
        projection_share.push(projection);
        residual_share.push(residual);
        finite_bias.push(bias);
        finite_variance.push(variance);
        residual_mass.push(residual_outcome);
        deleted_mass.push(deleted);
        max_reciprocal_residual = max_reciprocal_residual.max(reciprocal_residual);
    }
    let cell_correction_weight = cell_weight
        .into_iter()
        .map(StableSum::finish)
        .collect::<Vec<_>>();
    if cell_correction_weight
        .iter()
        .any(|value| !value.is_finite())
    {
        return Err(BackendError::new(
            ErrorCode::CorrectionNonFinite,
            "jla_leverage",
            "cell correction weight is nonfinite",
        ));
    }
    Ok(LeverageAdjustment {
        projection_share,
        residual_share,
        finite_bias,
        finite_variance,
        residual_mass,
        deleted_mass,
        cell_correction_weight,
        max_reciprocal_residual,
    })
}

fn validate_target_geometry(problem: &CompressedProblem, plan: &JlaPlan) -> Result<()> {
    if problem.cell_target_sum.len() != problem.cells()
        || problem
            .cell_target_sum
            .iter()
            .any(|value| !value.is_finite() || *value < 0.0)
    {
        return Err(BackendError::new(
            ErrorCode::TargetCenteringFailed,
            "jla_validate",
            "cell target masses are invalid",
        ));
    }
    let mut total = StableSum::default();
    for &mass in &problem.cell_target_sum {
        total.add(mass);
    }
    if !aggregate_close(
        total.finish(),
        problem.target_total,
        problem.target_total.abs(),
    ) {
        return Err(BackendError::new(
            ErrorCode::TargetCenteringFailed,
            "jla_validate",
            "cell target masses do not reproduce the target total",
        ));
    }
    if plan
        .target
        .per_copy_mass
        .iter()
        .any(|value| !value.is_finite() || *value < 0.0)
    {
        return Err(BackendError::new(
            ErrorCode::JlaMomentFailed,
            "jla_validate",
            "target per-copy moment scale is invalid",
        ));
    }
    Ok(())
}

fn preflight_trial_words(domain: &str, trials: &[u64]) -> Result<()> {
    for (entity, &count) in trials.iter().enumerate() {
        let words = count.div_ceil(64);
        if count == 0 || words > MAX_PHYSICAL_WORDS_PER_ATOM {
            return Err(BackendError::new(
                ErrorCode::ResourceLimit,
                "jla_validate",
                format!(
                    "{domain} trial preflight failed at semantic entity {entity}: {words} physical words exceeds the registered limit {MAX_PHYSICAL_WORDS_PER_ATOM}"
                ),
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
fn rademacher_atoms_with_interrupt(
    rng: CounterRng,
    domain: ProbeDomain,
    first_probe: usize,
    width: usize,
    entity: &[u64],
    trials: &[u64],
    interrupt: &mut dyn InterruptCheck,
) -> Result<Vec<i64>> {
    let length = entity.len().checked_mul(width).ok_or_else(|| {
        BackendError::new(
            ErrorCode::ResourceLimit,
            "jla_rng",
            "atom matrix length overflow",
        )
    })?;
    let mut atoms = vec![0_i64; length];
    rng.fill_rademacher_sums_with_interrupt(
        domain,
        u64::try_from(first_probe).map_err(|_| {
            BackendError::new(ErrorCode::ResourceLimit, "jla_rng", "probe index overflow")
        })?,
        width,
        entity,
        trials,
        &mut atoms,
        interrupt,
    )?;
    Ok(atoms)
}

fn rademacher_atoms_ordered_with_interrupt(
    rng: CounterRng,
    domain: ProbeDomain,
    first_probe: usize,
    width: usize,
    entity: &[u64],
    trials: &[u64],
    solver: &PreparedTwoWaySolver<'_>,
    interrupt: &mut dyn InterruptCheck,
) -> Result<Vec<i64>> {
    if width == 0 || entity.is_empty() || entity.len() != trials.len() {
        return Err(BackendError::invalid(
            "jla_rng",
            "atom matrix has incompatible dimensions",
        ));
    }
    let length = entity.len().checked_mul(width).ok_or_else(|| {
        BackendError::new(
            ErrorCode::ResourceLimit,
            "jla_rng",
            "atom matrix length overflow",
        )
    })?;
    let block_length = entity
        .len()
        .checked_mul(RNG_PROBE_BLOCK_COLUMNS)
        .ok_or_else(resource_length_error)?;
    let mut atoms = vec![0_i64; length];
    interrupt.checkpoint("jla_rng_probe_blocks")?;
    let blocks = atoms
        .chunks_mut(block_length)
        .enumerate()
        .collect::<Vec<_>>();
    let completed = solver.map_independent_ordered(blocks, |(block_index, output)| {
        let first_column = block_index * RNG_PROBE_BLOCK_COLUMNS;
        let block_width = output.len() / entity.len();
        let logical_probe = first_probe.checked_add(first_column).ok_or_else(|| {
            BackendError::new(
                ErrorCode::ResourceLimit,
                "jla_rng",
                "logical probe index overflow",
            )
        })?;
        let mut worker_interrupt = NeverInterrupt;
        rng.fill_rademacher_sums_with_interrupt(
            domain,
            u64::try_from(logical_probe).map_err(|_| {
                BackendError::new(ErrorCode::ResourceLimit, "jla_rng", "probe index overflow")
            })?,
            block_width,
            entity,
            trials,
            output,
            &mut worker_interrupt,
        )
    });
    for result in completed {
        interrupt.checkpoint("jla_rng_probe_block_complete")?;
        result?;
    }
    Ok(atoms)
}

fn leverage_rhs_with_interrupt(
    problem: &CompressedProblem,
    plan: &JlaPlan,
    atoms: &[i64],
    width: usize,
    interrupt: &mut dyn InterruptCheck,
) -> Result<(Vec<f64>, Vec<f64>)> {
    let groups = plan.deletion_units();
    if atoms.len()
        != groups
            .checked_mul(width)
            .ok_or_else(resource_length_error)?
    {
        return Err(BackendError::invalid(
            "jla_leverage",
            "atom matrix has the wrong size",
        ));
    }
    let worker_length = problem
        .workers()
        .checked_mul(width)
        .ok_or_else(resource_length_error)?;
    let firm_length = problem
        .firms()
        .checked_mul(width)
        .ok_or_else(resource_length_error)?;
    let mut worker_rhs = vec![0.0; worker_length];
    let mut firm_rhs = vec![0.0; firm_length];
    for column in 0..width {
        for group in 0..groups {
            let work = column
                .checked_mul(groups)
                .and_then(|value| value.checked_add(group))
                .ok_or_else(resource_length_error)?;
            checkpoint_chunk(interrupt, work, "jla_leverage_rhs")?;
            let cell = usize::try_from(plan.deletion.cell[group]).expect("validated cell");
            let worker = usize::try_from(problem.cell_worker[cell]).expect("validated worker");
            let firm = usize::try_from(problem.cell_firm[cell]).expect("validated firm");
            let atom = atoms[column * groups + group] as f64;
            // Counter atoms and their complete worker/firm sums are exact
            // binary64 integers under the permanent 2^53 physical-mass gate.
            // Direct scatter is therefore bit-identical to materializing the
            // sparse cell score and transposing it in cell order.
            worker_rhs[column * problem.workers() + worker] += atom;
            firm_rhs[column * problem.firms() + firm] += atom;
        }
    }
    Ok((worker_rhs, firm_rhs))
}

fn target_directions_with_interrupt(
    problem: &CompressedProblem,
    plan: &JlaPlan,
    atoms: &[i64],
    width: usize,
    first_probe: usize,
    interrupt: &mut dyn InterruptCheck,
) -> Result<(Vec<f64>, Vec<f64>)> {
    let strata = plan.target_strata();
    if atoms.len()
        != strata
            .checked_mul(width)
            .ok_or_else(resource_length_error)?
    {
        return Err(BackendError::invalid(
            "jla_target",
            "target atom matrix has the wrong size",
        ));
    }
    let mut direction = vec![0.0; problem.cells() * width];
    let mut reference_scale = vec![0.0; width];
    for column in 0..width {
        let mut first = vec![StableSum::default(); problem.cells()];
        for stratum in 0..strata {
            let work = column
                .checked_mul(strata)
                .and_then(|value| value.checked_add(stratum))
                .ok_or_else(resource_length_error)?;
            checkpoint_chunk(interrupt, work, "jla_target_direction_strata")?;
            let cell = usize::try_from(plan.target.cell[stratum]).expect("validated target cell");
            let scale = (plan.target.per_copy_mass[stratum] / problem.target_total).sqrt();
            first[cell].add(scale * atoms[column * strata + stratum] as f64);
        }
        let mut first_values = Vec::with_capacity(first.len());
        for (cell, value) in first.into_iter().enumerate() {
            checkpoint_chunk(interrupt, cell, "jla_target_direction_reduce")?;
            first_values.push(value.finish());
        }
        let mut total = StableSum::default();
        let mut absolute = StableSum::default();
        for (cell, &value) in first_values.iter().enumerate() {
            checkpoint_chunk(interrupt, cell, "jla_target_direction_total")?;
            total.add(value);
            absolute.add(value.abs());
        }
        let total = total.finish();
        reference_scale[column] = absolute.finish() + total.abs();
        if !reference_scale[column].is_finite() || reference_scale[column] < 0.0 {
            return Err(BackendError::new(
                ErrorCode::TargetCenteringFailed,
                "jla_target",
                format!(
                    "target reference scale is invalid at probe {}, side centered",
                    first_probe + column
                ),
            ));
        }
        for cell in 0..problem.cells() {
            checkpoint_chunk(interrupt, cell, "jla_target_direction_center")?;
            let value =
                first_values[cell] - problem.cell_target_sum[cell] / problem.target_total * total;
            if !value.is_finite() {
                return Err(BackendError::new(
                    ErrorCode::TargetCenteringFailed,
                    "jla_target",
                    format!(
                        "target direction is nonfinite at probe {}, side centered",
                        first_probe + column
                    ),
                ));
            }
            direction[column * problem.cells() + cell] = value;
        }
    }
    Ok((direction, reference_scale))
}

fn target_rhs_columns_with_interrupt(
    problem: &CompressedProblem,
    plan: &JlaPlan,
    atoms: &[i64],
    width: usize,
    first_probe: usize,
    solver: &PreparedTwoWaySolver<'_>,
    interrupt: &mut dyn InterruptCheck,
) -> Result<(Vec<f64>, Vec<f64>)> {
    let strata = plan.target_strata();
    if width == 0
        || atoms.len()
            != strata
                .checked_mul(width)
                .ok_or_else(resource_length_error)?
    {
        return Err(BackendError::invalid(
            "jla_target",
            "target atom matrix has the wrong size",
        ));
    }
    interrupt.checkpoint("jla_target_prepare_columns")?;
    let prepared = solver.map_independent_ordered((0..width).collect(), |column| {
        let mut worker_interrupt = NeverInterrupt;
        let first = column * strata;
        let (direction, reference_scale) = target_directions_with_interrupt(
            problem,
            plan,
            &atoms[first..first + strata],
            1,
            first_probe + column,
            &mut worker_interrupt,
        )?;
        target_rhs_with_interrupt(
            problem,
            &direction,
            &reference_scale,
            1,
            first_probe + column,
            &mut worker_interrupt,
        )
    });
    let worker_length = problem
        .workers()
        .checked_mul(2)
        .and_then(|value| value.checked_mul(width))
        .ok_or_else(resource_length_error)?;
    let firm_length = problem
        .firms()
        .checked_mul(2)
        .and_then(|value| value.checked_mul(width))
        .ok_or_else(resource_length_error)?;
    let mut worker_rhs = Vec::with_capacity(worker_length);
    let mut firm_rhs = Vec::with_capacity(firm_length);
    for column in prepared {
        interrupt.checkpoint("jla_target_prepare_complete")?;
        let (mut worker, mut firm) = column?;
        worker_rhs.append(&mut worker);
        firm_rhs.append(&mut firm);
    }
    debug_assert_eq!(worker_rhs.len(), worker_length);
    debug_assert_eq!(firm_rhs.len(), firm_length);
    Ok((worker_rhs, firm_rhs))
}

#[cfg(test)]
fn target_rhs(
    problem: &CompressedProblem,
    direction: &[f64],
    reference_scale: &[f64],
    width: usize,
    first_probe: usize,
) -> Result<(Vec<f64>, Vec<f64>)> {
    let mut interrupt = NeverInterrupt;
    target_rhs_with_interrupt(
        problem,
        direction,
        reference_scale,
        width,
        first_probe,
        &mut interrupt,
    )
}

fn target_rhs_with_interrupt(
    problem: &CompressedProblem,
    direction: &[f64],
    reference_scale: &[f64],
    width: usize,
    first_probe: usize,
    interrupt: &mut dyn InterruptCheck,
) -> Result<(Vec<f64>, Vec<f64>)> {
    if reference_scale.len() != width {
        return Err(BackendError::new(
            ErrorCode::TargetCenteringFailed,
            "jla_target",
            "target reference-scale vector is invalid",
        ));
    }
    for (index, &value) in reference_scale.iter().enumerate() {
        checkpoint_chunk(interrupt, index, "jla_target_reference_scale")?;
        if !value.is_finite() || value < 0.0 {
            return Err(BackendError::new(
                ErrorCode::TargetCenteringFailed,
                "jla_target",
                "target reference-scale vector is invalid",
            ));
        }
    }
    let (score_worker, score_firm) =
        transpose_cell_rhs_with_interrupt(problem, direction, width, interrupt)?;
    let workers = problem.workers();
    let firms = problem.firms();
    let mut worker_rhs = vec![0.0; workers * 2 * width];
    let mut firm_rhs = vec![0.0; firms * 2 * width];
    for column in 0..width {
        let mut worker = Vec::with_capacity(workers);
        for index in 0..workers {
            checkpoint_chunk(interrupt, index, "jla_target_worker_copy")?;
            worker.push(score_worker[column * workers + index]);
        }
        let mut firm = Vec::with_capacity(firms);
        for index in 0..firms {
            checkpoint_chunk(interrupt, index, "jla_target_firm_copy")?;
            firm.push(score_firm[column * firms + index]);
        }
        let probe = first_probe + column;
        balance_score_with_interrupt(
            &mut worker,
            reference_scale[column],
            probe,
            JlaRhsSide::Worker,
            interrupt,
        )?;
        balance_score_with_interrupt(
            &mut firm,
            reference_scale[column],
            probe,
            JlaRhsSide::Firm,
            interrupt,
        )?;
        for (index, &value) in worker.iter().enumerate() {
            checkpoint_chunk(interrupt, index, "jla_target_worker_scatter")?;
            worker_rhs[2 * column * workers + index] = value;
        }
        for (index, &value) in firm.iter().enumerate() {
            checkpoint_chunk(interrupt, index, "jla_target_firm_scatter")?;
            firm_rhs[(2 * column + 1) * firms + index] = value;
        }
    }
    Ok((worker_rhs, firm_rhs))
}

#[cfg(test)]
fn balance_score(score: &mut [f64], reference: f64, probe: usize, side: JlaRhsSide) -> Result<()> {
    let mut interrupt = NeverInterrupt;
    balance_score_with_interrupt(score, reference, probe, side, &mut interrupt)
}

fn balance_score_with_interrupt(
    score: &mut [f64],
    reference: f64,
    probe: usize,
    side: JlaRhsSide,
    interrupt: &mut dyn InterruptCheck,
) -> Result<()> {
    if score.is_empty() || !reference.is_finite() || reference < 0.0 {
        return Err(BackendError::new(
            ErrorCode::TargetCenteringFailed,
            "jla_target",
            format!("invalid compatibility score at probe {probe}, side {side:?}"),
        ));
    }
    let original = *score.last().expect("nonempty score");
    let mut preceding = StableSum::default();
    for (index, &value) in score[..score.len() - 1].iter().enumerate() {
        checkpoint_chunk(interrupt, index, "jla_target_balance")?;
        preceding.add(value);
    }
    let last = -preceding.finish();
    if (reference == 0.0 && last != original)
        || (reference > 0.0 && (last - original).abs() > ROUNDOFF_GATE * reference)
    {
        return Err(BackendError::new(
            ErrorCode::TargetCenteringFailed,
            "jla_target",
            format!("compatibility repair exceeded roundoff at probe {probe}, side {side:?}"),
        ));
    }
    *score.last_mut().expect("nonempty score") = last;
    Ok(())
}

fn transpose_cell_rhs_with_interrupt(
    problem: &CompressedProblem,
    cell_value: &[f64],
    width: usize,
    interrupt: &mut dyn InterruptCheck,
) -> Result<(Vec<f64>, Vec<f64>)> {
    if cell_value.len()
        != problem
            .cells()
            .checked_mul(width)
            .ok_or_else(resource_length_error)?
    {
        return Err(BackendError::invalid(
            "jla_rhs",
            "cell RHS matrix has the wrong size",
        ));
    }
    let mut worker = vec![0.0; problem.workers() * width];
    let mut firm = vec![0.0; problem.firms() * width];
    for column in 0..width {
        for cell in 0..problem.cells() {
            let work = column
                .checked_mul(problem.cells())
                .and_then(|value| value.checked_add(cell))
                .ok_or_else(resource_length_error)?;
            checkpoint_chunk(interrupt, work, "jla_rhs_transpose")?;
            let value = cell_value[column * problem.cells() + cell];
            let worker_index = usize::try_from(problem.cell_worker[cell]).expect("worker");
            let firm_index = usize::try_from(problem.cell_firm[cell]).expect("firm");
            worker[column * problem.workers() + worker_index] += value;
            firm[column * problem.firms() + firm_index] += value;
        }
    }
    Ok((worker, firm))
}

fn cell_predictions_with_interrupt(
    problem: &CompressedProblem,
    worker: &[f64],
    firm: &[f64],
    interrupt: &mut dyn InterruptCheck,
) -> Result<Vec<f64>> {
    if worker.len() != problem.workers() || firm.len() != problem.firms() {
        return Err(BackendError::invalid(
            "jla_prediction",
            "coefficient dimensions differ",
        ));
    }
    let mut prediction = Vec::with_capacity(problem.cells());
    for cell in 0..problem.cells() {
        checkpoint_chunk(interrupt, cell, "jla_prediction")?;
        let value = worker[usize::try_from(problem.cell_worker[cell]).expect("worker")]
            + firm[usize::try_from(problem.cell_firm[cell]).expect("firm")];
        if !value.is_finite() {
            return Err(BackendError::new(
                ErrorCode::CorrectionNonFinite,
                "jla_prediction",
                "cell prediction is nonfinite",
            ));
        }
        prediction.push(value);
    }
    Ok(prediction)
}

#[allow(clippy::too_many_arguments)]
fn contract_target_solutions_with_interrupt(
    problem: &CompressedProblem,
    weight: &[f64],
    worker_side_worker: &[f64],
    worker_side_firm: &[f64],
    firm_side_worker: &[f64],
    firm_side_firm: &[f64],
    probe: usize,
    interrupt: &mut dyn InterruptCheck,
) -> Result<VarianceComponents> {
    if weight.len() != problem.cells()
        || worker_side_worker.len() != problem.workers()
        || firm_side_worker.len() != problem.workers()
        || worker_side_firm.len() != problem.firms()
        || firm_side_firm.len() != problem.firms()
        || weight.is_empty()
    {
        return Err(BackendError::invalid(
            "jla_target",
            "target fused-contraction dimensions differ",
        ));
    }
    let mut worker_second = StableSum::default();
    let mut firm_second = StableSum::default();
    let mut covariance = StableSum::default();
    for cell in 0..weight.len() {
        checkpoint_chunk(interrupt, cell, "jla_target_contraction")?;
        let worker = usize::try_from(problem.cell_worker[cell]).expect("worker");
        let firm = usize::try_from(problem.cell_firm[cell]).expect("firm");
        let worker_prediction = worker_side_worker[worker] + worker_side_firm[firm];
        let firm_prediction = firm_side_worker[worker] + firm_side_firm[firm];
        if !worker_prediction.is_finite() || !firm_prediction.is_finite() {
            return Err(BackendError::new(
                ErrorCode::CorrectionNonFinite,
                "jla_prediction",
                "cell prediction is nonfinite",
            ));
        }
        worker_second.add(weight[cell] * worker_prediction * worker_prediction);
        firm_second.add(weight[cell] * firm_prediction * firm_prediction);
        covariance.add(weight[cell] * worker_prediction * firm_prediction);
    }
    finish_target_draw(worker_second, firm_second, covariance, probe)
}

#[cfg(test)]
fn contract_target_draw_with_interrupt(
    weight: &[f64],
    worker: &[f64],
    firm: &[f64],
    probe: usize,
    interrupt: &mut dyn InterruptCheck,
) -> Result<VarianceComponents> {
    if weight.len() != worker.len() || weight.len() != firm.len() || weight.is_empty() {
        return Err(BackendError::invalid(
            "jla_target",
            "target contraction dimensions differ",
        ));
    }
    let mut worker_second = StableSum::default();
    let mut firm_second = StableSum::default();
    let mut covariance = StableSum::default();
    for cell in 0..weight.len() {
        checkpoint_chunk(interrupt, cell, "jla_target_contraction")?;
        worker_second.add(weight[cell] * worker[cell] * worker[cell]);
        firm_second.add(weight[cell] * firm[cell] * firm[cell]);
        covariance.add(weight[cell] * worker[cell] * firm[cell]);
    }
    finish_target_draw(worker_second, firm_second, covariance, probe)
}

fn finish_target_draw(
    worker_second: StableSum,
    firm_second: StableSum,
    covariance: StableSum,
    probe: usize,
) -> Result<VarianceComponents> {
    let worker = worker_second.finish();
    let firm = firm_second.finish();
    let covariance = covariance.finish();
    let draw = VarianceComponents {
        worker,
        firm,
        covariance,
        total: worker + firm + 2.0 * covariance,
    };
    if [draw.worker, draw.firm, draw.covariance, draw.total]
        .iter()
        .any(|value| !value.is_finite())
    {
        return Err(BackendError::new(
            ErrorCode::CorrectionNonFinite,
            "jla_target",
            format!("target contraction is nonfinite at probe {probe}"),
        ));
    }
    Ok(draw)
}

#[cfg(test)]
fn mean_components(draws: &[VarianceComponents]) -> Result<VarianceComponents> {
    let mut interrupt = NeverInterrupt;
    mean_components_with_interrupt(draws, &mut interrupt)
}

fn mean_components_with_interrupt(
    draws: &[VarianceComponents],
    interrupt: &mut dyn InterruptCheck,
) -> Result<VarianceComponents> {
    if draws.is_empty() {
        return Err(BackendError::invariant(
            "jla_target",
            "target draw set is empty",
        ));
    }
    let mut sums = [StableSum::default(); 4];
    for (index, draw) in draws.iter().enumerate() {
        checkpoint_chunk(interrupt, index, "jla_finalize_mean")?;
        if !components_finite(*draw) {
            return Err(BackendError::new(
                ErrorCode::CorrectionNonFinite,
                "jla_target",
                "target draw reduction received a nonfinite component",
            ));
        }
        sums[0].add(draw.worker);
        sums[1].add(draw.firm);
        sums[2].add(draw.covariance);
        sums[3].add(draw.total);
    }
    let count = draws.len() as f64;
    let result = VarianceComponents {
        worker: sums[0].finish() / count,
        firm: sums[1].finish() / count,
        covariance: sums[2].finish() / count,
        total: sums[3].finish() / count,
    };
    if !components_finite(result) {
        return Err(BackendError::new(
            ErrorCode::CorrectionNonFinite,
            "jla_target",
            "mean target correction is nonfinite",
        ));
    }
    Ok(result)
}

fn component_mcse_with_interrupt(
    draws: &[VarianceComponents],
    interrupt: &mut dyn InterruptCheck,
) -> Result<NumericalMcse> {
    if draws.len() < 2 {
        return Err(BackendError::invalid(
            "jla_target",
            "MCSE requires at least two draws",
        ));
    }
    let mean = mean_components_with_interrupt(draws, interrupt)?;
    let mut sums = [StableSum::default(); 4];
    for (index, draw) in draws.iter().enumerate() {
        checkpoint_chunk(interrupt, index, "jla_finalize_mcse")?;
        sums[0].add((draw.worker - mean.worker).powi(2));
        sums[1].add((draw.firm - mean.firm).powi(2));
        sums[2].add((draw.covariance - mean.covariance).powi(2));
        sums[3].add((draw.total - mean.total).powi(2));
    }
    let denominator = (draws.len() * (draws.len() - 1)) as f64;
    let value = NumericalMcse {
        worker: (sums[0].finish() / denominator).sqrt(),
        firm: (sums[1].finish() / denominator).sqrt(),
        covariance: (sums[2].finish() / denominator).sqrt(),
        total: (sums[3].finish() / denominator).sqrt(),
    };
    if [value.worker, value.firm, value.covariance, value.total]
        .iter()
        .any(|entry| !entry.is_finite())
    {
        return Err(BackendError::new(
            ErrorCode::CorrectionNonFinite,
            "jla_target",
            "numerical MCSE is nonfinite",
        ));
    }
    Ok(value)
}

fn subtract_components(
    plugin: VarianceComponents,
    correction: VarianceComponents,
) -> Result<VarianceComponents> {
    if !components_finite(plugin) || !components_finite(correction) {
        return Err(BackendError::new(
            ErrorCode::NonfiniteCorrectedTarget,
            "jla_target",
            "plugin or correction component is nonfinite before subtraction",
        ));
    }
    let corrected = VarianceComponents {
        worker: plugin.worker - correction.worker,
        firm: plugin.firm - correction.firm,
        covariance: plugin.covariance - correction.covariance,
        total: plugin.total - correction.total,
    };
    if !components_finite(corrected) {
        return Err(BackendError::new(
            ErrorCode::NonfiniteCorrectedTarget,
            "jla_target",
            "corrected target component is nonfinite after subtraction",
        ));
    }
    Ok(corrected)
}

#[cfg(test)]
fn accounting_residuals(
    plugin: VarianceComponents,
    correction: VarianceComponents,
    corrected: VarianceComponents,
    draws: &[VarianceComponents],
) -> Result<f64> {
    let mut interrupt = NeverInterrupt;
    accounting_residuals_with_interrupt(plugin, correction, corrected, draws, &mut interrupt)
}

fn accounting_residuals_with_interrupt(
    plugin: VarianceComponents,
    correction: VarianceComponents,
    corrected: VarianceComponents,
    draws: &[VarianceComponents],
    interrupt: &mut dyn InterruptCheck,
) -> Result<f64> {
    if !components_finite(plugin) || !components_finite(correction) || !components_finite(corrected)
    {
        return Err(BackendError::new(
            ErrorCode::CorrectionNonFinite,
            "jla_accounting",
            "target accounting input contains a nonfinite component",
        ));
    }
    let mut maximum = component_identity_residual(plugin)
        .max(component_identity_residual(correction))
        .max(component_identity_residual(corrected));
    for (index, draw) in draws.iter().enumerate() {
        checkpoint_chunk(interrupt, index, "jla_finalize_accounting")?;
        if !components_finite(*draw) {
            return Err(BackendError::new(
                ErrorCode::CorrectionNonFinite,
                "jla_accounting",
                "target accounting input contains a nonfinite component",
            ));
        }
        maximum = maximum.max(component_identity_residual(*draw));
    }
    if !maximum.is_finite() {
        return Err(BackendError::new(
            ErrorCode::CorrectionNonFinite,
            "jla_accounting",
            "component accounting residual is nonfinite",
        ));
    }
    if maximum > ROUNDOFF_GATE {
        return Err(BackendError::new(
            ErrorCode::TargetIdentityFailed,
            "jla_accounting",
            format!("component accounting residual {maximum} exceeds {ROUNDOFF_GATE}"),
        ));
    }
    Ok(maximum)
}

fn residual_maxima_with_interrupt(
    full_fit: &JlaRhsReceipt,
    leverage: &[JlaRhsReceipt],
    target: &[JlaRhsReceipt],
    interrupt: &mut dyn InterruptCheck,
) -> Result<(f64, f64)> {
    let mut maximum_reduced = full_fit.reduced_residual;
    let mut maximum_complete = full_fit.complete_residual;
    for (index, receipt) in leverage.iter().chain(target).enumerate() {
        checkpoint_chunk(interrupt, index, "jla_finalize_receipts")?;
        maximum_reduced = maximum_reduced.max(receipt.reduced_residual);
        maximum_complete = maximum_complete.max(receipt.complete_residual);
    }
    Ok((maximum_reduced, maximum_complete))
}

fn maximum_with_interrupt(
    values: &[f64],
    phase: &'static str,
    interrupt: &mut dyn InterruptCheck,
) -> Result<f64> {
    let mut maximum = 0.0_f64;
    for (index, &value) in values.iter().enumerate() {
        checkpoint_chunk(interrupt, index, phase)?;
        maximum = maximum.max(value);
    }
    Ok(maximum)
}

fn components_finite(value: VarianceComponents) -> bool {
    [value.worker, value.firm, value.covariance, value.total]
        .iter()
        .all(|entry| entry.is_finite())
}

fn component_identity_residual(value: VarianceComponents) -> f64 {
    let scale = value
        .worker
        .abs()
        .max(value.firm.abs())
        .max(value.covariance.abs())
        .max(value.total.abs())
        .max(1.0);
    (value.total - value.worker - value.firm - 2.0 * value.covariance).abs() / scale
}

fn rhs_receipt(
    receipt: &RoutedSolveReceipt,
    complete_residual: f64,
    rhs_norm: f64,
    phase: JlaSolvePhase,
    probe: Option<u64>,
    side: JlaRhsSide,
) -> Result<JlaRhsReceipt> {
    let (iterations, reduced_residual, zero_rhs) = if let Some(exact) = &receipt.exact {
        (0, exact.reduced_residual, rhs_norm == 0.0)
    } else if let Some(pcg) = &receipt.pcg {
        (pcg.iterations, pcg.relative_residual, pcg.zero_rhs)
    } else {
        return Err(BackendError::invariant(
            "jla_receipt",
            "accepted RHS has neither exact nor PCG receipt",
        ));
    };
    // A zero RHS is accepted by the solver without operator work.  Normalize
    // both residual surfaces to exact zero so the lossless receipt cannot
    // expose roundoff from reconstructing the complete-space diagnostic.
    let complete_residual = if zero_rhs { 0.0 } else { complete_residual };
    Ok(JlaRhsReceipt {
        phase,
        probe,
        side,
        route: receipt.selected,
        iterations,
        reduced_residual,
        complete_residual,
        zero_rhs,
    })
}

fn rhs_error(
    error: BackendError,
    phase: JlaSolvePhase,
    probe: Option<u64>,
    side: JlaRhsSide,
) -> BackendError {
    BackendError::new(
        error.code,
        phase_name(phase),
        format!("phase {phase:?}, probe {probe:?}, side {side:?}: {error}"),
    )
}

fn contextual_batch_error(
    error: BackendError,
    phase: JlaSolvePhase,
    first_probe: usize,
    paired: bool,
) -> BackendError {
    let column = parse_rhs_column(&error.message).unwrap_or(0);
    let (probe, side) = if paired {
        (
            first_probe + column / 2,
            if column % 2 == 0 {
                JlaRhsSide::Worker
            } else {
                JlaRhsSide::Firm
            },
        )
    } else {
        (first_probe + column, JlaRhsSide::Joint)
    };
    rhs_error(error, phase, Some(probe as u64), side)
}

fn parse_rhs_column(message: &str) -> Option<usize> {
    let marker = "zero-based RHS column ";
    let tail = message.split_once(marker)?.1;
    let digits = tail
        .chars()
        .take_while(char::is_ascii_digit)
        .collect::<String>();
    digits.parse().ok()
}

const fn phase_name(phase: JlaSolvePhase) -> &'static str {
    match phase {
        JlaSolvePhase::FullFit => "jla_full_fit",
        JlaSolvePhase::Leverage => "jla_leverage",
        JlaSolvePhase::Target => "jla_target",
    }
}

fn aggregate_close(reconstructed: f64, reference: f64, absolute_mass: f64) -> bool {
    if !reconstructed.is_finite() || !reference.is_finite() || !absolute_mass.is_finite() {
        return false;
    }
    let scale = reconstructed
        .abs()
        .max(reference.abs())
        .max(absolute_mass.abs())
        .max(1.0);
    (reconstructed - reference).abs() <= ROUNDOFF_GATE * scale
}

fn resource_length_error() -> BackendError {
    BackendError::new(
        ErrorCode::ResourceLimit,
        "jla_engine",
        "matrix length overflow",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interrupt::INTERRUPT_CHECK_CHUNK;
    use crate::krylov::PcgOptions;
    use crate::operator::TwoWayOperator;
    use crate::problem::CanonicalInput;
    use crate::types::InputColumns;

    struct BreakOnPhase {
        phase: &'static str,
        break_call: usize,
        calls: usize,
    }

    impl BreakOnPhase {
        fn new(phase: &'static str, break_call: usize) -> Self {
            Self {
                phase,
                break_call,
                calls: 0,
            }
        }
    }

    impl InterruptCheck for BreakOnPhase {
        fn checkpoint(&mut self, phase: &'static str) -> Result<()> {
            if phase == self.phase {
                self.calls += 1;
                if self.calls == self.break_call {
                    return Err(BackendError::new(
                        ErrorCode::UserBreak,
                        phase,
                        "injected user break",
                    ));
                }
            }
            Ok(())
        }
    }

    fn wide_work_problem(workers: usize) -> CompressedProblem {
        let rows = workers * 2;
        let mut worker = Vec::with_capacity(rows);
        let mut firm = Vec::with_capacity(rows);
        for index in 0..workers {
            let label = u64::try_from(index + 1).expect("worker");
            worker.extend([label, label]);
            firm.extend([1_u64, 2_u64]);
        }
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
            .expect("wide input"),
        )
        .expect("wide canonical")
        .compress(&vec![true; rows])
        .expect("wide compressed")
    }

    fn audit_problem(order: &[usize]) -> CompressedProblem {
        let worker = [1_u64, 1, 1, 1, 2, 2, 2, 2];
        let firm = [1_u64, 1, 2, 2, 1, 1, 2, 2];
        let deletion = [11_u64, 12, 21, 22, 31, 32, 41, 42];
        let outcome = [1.0, 3.0, 0.0, 2.0, -1.0, 1.0, 2.0, -2.0];
        let frequency = [1_u64, 2, 1, 2, 1, 3, 2, 1];
        let target = [1.0, 2.0, 2.0, 2.0, 3.0, 9.0, 8.0, 4.0];
        CanonicalInput::from_validated(
            InputColumns {
                worker: order.iter().map(|&row| worker[row]).collect(),
                firm: order.iter().map(|&row| firm[row]).collect(),
                deletion: order.iter().map(|&row| deletion[row]).collect(),
                outcome: order.iter().map(|&row| outcome[row]).collect(),
                frequency: order.iter().map(|&row| frequency[row]).collect(),
                target_weight: order.iter().map(|&row| target[row]).collect(),
                controls: Vec::new(),
            }
            .validate()
            .expect("audit fixture"),
        )
        .expect("canonical audit fixture")
        .compress(&[true; 8])
        .expect("compressed audit fixture")
    }

    fn three_firm_cmg_problem() -> CompressedProblem {
        let mut worker = Vec::new();
        let mut firm = Vec::new();
        let mut deletion = Vec::new();
        let mut outcome = Vec::new();
        let mut target_weight = Vec::new();
        for worker_index in 0..3_u64 {
            for firm_index in 0..3_u64 {
                for replicate in 0..2_u64 {
                    let row = worker.len() as u64;
                    worker.push(worker_index + 1);
                    firm.push(firm_index + 1);
                    deletion.push(row + 1);
                    outcome.push(
                        0.7 * worker_index as f64 - 0.4 * firm_index as f64
                            + 0.25 * replicate as f64
                            + ((5 * row) % 7) as f64 / 13.0,
                    );
                    target_weight.push(1.0 + ((3 * row) % 5) as f64 / 4.0);
                }
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
                target_weight,
                controls: Vec::new(),
            }
            .validate()
            .expect("three-firm input"),
        )
        .expect("three-firm canonical input")
        .compress(&vec![true; rows])
        .expect("three-firm compressed problem")
    }

    fn relabelled_audit_problem(order: &[usize]) -> CompressedProblem {
        let worker = [101_u64, 101, 101, 101, 909, 909, 909, 909];
        let firm = [17_u64, 17, 83, 83, 17, 17, 83, 83];
        let deletion = [1011_u64, 1012, 1021, 1022, 1031, 1032, 1041, 1042];
        let outcome = [1.0, 3.0, 0.0, 2.0, -1.0, 1.0, 2.0, -2.0];
        let frequency = [1_u64, 2, 1, 2, 1, 3, 2, 1];
        let target = [1.0, 2.0, 2.0, 2.0, 3.0, 9.0, 8.0, 4.0];
        CanonicalInput::from_validated(
            InputColumns {
                worker: order.iter().map(|&row| worker[row]).collect(),
                firm: order.iter().map(|&row| firm[row]).collect(),
                deletion: order.iter().map(|&row| deletion[row]).collect(),
                outcome: order.iter().map(|&row| outcome[row]).collect(),
                frequency: order.iter().map(|&row| frequency[row]).collect(),
                target_weight: order.iter().map(|&row| target[row]).collect(),
                controls: Vec::new(),
            }
            .validate()
            .expect("relabelled audit fixture"),
        )
        .expect("canonical relabelled audit fixture")
        .compress(&[true; 8])
        .expect("compressed relabelled audit fixture")
    }

    fn trial_preflight_problem(
        worker: Vec<u64>,
        firm: Vec<u64>,
        frequency: Vec<u64>,
        target_weight: Vec<f64>,
    ) -> CompressedProblem {
        let rows = worker.len();
        CanonicalInput::from_validated(
            InputColumns {
                worker,
                firm,
                deletion: (1..=u64::try_from(rows).expect("rows")).collect(),
                outcome: vec![0.0; rows],
                frequency,
                target_weight,
                controls: Vec::new(),
            }
            .validate()
            .expect("preflight fixture"),
        )
        .expect("canonical preflight fixture")
        .compress(&vec![true; rows])
        .expect("compressed preflight fixture")
    }

    fn audit_options(route: LinearSolverRoute) -> JlaEngineOptions {
        JlaEngineOptions {
            seed: 8_675_309,
            probes: 5,
            leverage_batch_width: 2,
            target_batch_width: 2,
            rank_tolerance: 1.0e-10,
            block_tolerance: 1.0e-10,
            solver: LinearSolverOptions {
                route,
                exact_dimension_limit: 64,
                pcg: PcgOptions {
                    tolerance: 1.0e-13,
                    maximum_iterations: 500,
                    residual_replacement_interval: 7,
                },
                full_residual_tolerance: 1.0e-11,
                ..LinearSolverOptions::default()
            },
            ..JlaEngineOptions::default()
        }
    }

    fn assert_close(actual: &[f64], expected: &[f64], tolerance: f64) {
        assert_eq!(actual.len(), expected.len());
        for (index, (&actual, &expected)) in actual.iter().zip(expected).enumerate() {
            assert!(
                (actual - expected).abs() <= tolerance,
                "entry {index}: {actual} versus {expected}"
            );
        }
    }

    fn values(value: VarianceComponents) -> [f64; 4] {
        [value.worker, value.firm, value.covariance, value.total]
    }

    fn mcse_values(value: NumericalMcse) -> [f64; 4] {
        [value.worker, value.firm, value.covariance, value.total]
    }

    #[derive(Debug)]
    struct DenseOracle {
        fitted_cell: [f64; 4],
        plugin: [f64; 4],
        correction: [f64; 4],
        corrected: [f64; 4],
        mcse: [f64; 4],
        draws: [[f64; 4]; 5],
    }

    const ORACLE_LEVERAGE_DOMAIN_TAG: u64 = 0x4c45_5645_5241_4745;
    const ORACLE_TARGET_DOMAIN_TAG: u64 = 0x0054_4152_4745_5401;

    #[test]
    fn engine_defaults_match_the_public_counter_contract() {
        let options = JlaEngineOptions::default();
        assert_eq!(options.seed, 8_675_309);
        assert_eq!(options.probes, 200);
        assert_eq!(options.rng, RngContract::CounterV1);
        assert_eq!(options.deletion, DeletionMode::Match);
        assert_eq!(
            options.solver.full_residual_tolerance,
            options.solver.required_full_residual_tolerance()
        );
    }

    #[test]
    fn fused_target_contraction_matches_materialized_predictions_bitwise() {
        let problem = audit_problem(&(0..8).collect::<Vec<_>>());
        let worker_side_worker = (0..problem.workers())
            .map(|index| index as f64 * 0.25 - 0.5)
            .collect::<Vec<_>>();
        let worker_side_firm = (0..problem.firms())
            .map(|index| index as f64 * -0.125 + 0.375)
            .collect::<Vec<_>>();
        let firm_side_worker = (0..problem.workers())
            .map(|index| index as f64 * -0.375 + 0.625)
            .collect::<Vec<_>>();
        let firm_side_firm = (0..problem.firms())
            .map(|index| index as f64 * 0.5 - 0.25)
            .collect::<Vec<_>>();
        let weights = (0..problem.cells())
            .map(|index| index as f64 + 1.0)
            .collect::<Vec<_>>();
        let mut interrupt = NeverInterrupt;
        let worker_prediction = cell_predictions_with_interrupt(
            &problem,
            &worker_side_worker,
            &worker_side_firm,
            &mut interrupt,
        )
        .expect("worker predictions");
        let firm_prediction = cell_predictions_with_interrupt(
            &problem,
            &firm_side_worker,
            &firm_side_firm,
            &mut interrupt,
        )
        .expect("firm predictions");
        let expected = contract_target_draw_with_interrupt(
            &weights,
            &worker_prediction,
            &firm_prediction,
            7,
            &mut interrupt,
        )
        .expect("materialized contraction");
        let actual = contract_target_solutions_with_interrupt(
            &problem,
            &weights,
            &worker_side_worker,
            &worker_side_firm,
            &firm_side_worker,
            &firm_side_firm,
            7,
            &mut interrupt,
        )
        .expect("fused contraction");
        assert_eq!(values(actual), values(expected));
    }

    /// Independent 13-copy oracle: explicitly assembles the constrained
    /// 4-coefficient normal equations and locally reimplements Philox V1.
    /// It does not call compression, JLA-plan, operator, solver, or engine
    /// helpers.
    fn expanded_dense_oracle() -> DenseOracle {
        let stored_worker = [0_usize, 0, 0, 0, 1, 1, 1, 1];
        let stored_firm = [0_usize, 0, 1, 1, 0, 0, 1, 1];
        let stored_y = [1.0, 3.0, 0.0, 2.0, -1.0, 1.0, 2.0, -2.0];
        let stored_frequency = [1_usize, 2, 1, 2, 1, 3, 2, 1];
        let stored_target = [1.0, 2.0, 2.0, 2.0, 3.0, 9.0, 8.0, 4.0];
        let mut physical_stored = Vec::new();
        for (stored, &frequency) in stored_frequency.iter().enumerate() {
            physical_stored.extend(std::iter::repeat(stored).take(frequency));
        }
        assert_eq!(physical_stored.len(), 13);

        let design_row = |stored: usize| {
            let mut row = [0.0; 4];
            row[stored_worker[stored]] = 1.0;
            row[2 + stored_firm[stored]] = 1.0;
            row
        };
        let solve = |rhs: [f64; 4]| {
            let dimension = 5;
            let mut matrix = vec![0.0; dimension * dimension];
            for &stored in &physical_stored {
                let row = design_row(stored);
                for left in 0..4 {
                    for right in 0..4 {
                        matrix[left * dimension + right] += row[left] * row[right];
                    }
                }
            }
            matrix[2 * dimension + 4] = 1.0;
            matrix[3 * dimension + 4] = 1.0;
            matrix[4 * dimension + 2] = 1.0;
            matrix[4 * dimension + 3] = 1.0;
            let mut augmented_rhs = vec![0.0; dimension];
            augmented_rhs[..4].copy_from_slice(&rhs);
            let solution = oracle_gaussian_solve(matrix, augmented_rhs);
            [solution[0], solution[1], solution[2], solution[3]]
        };
        let transpose = |physical: &[f64]| {
            let mut rhs = [0.0; 4];
            for (&stored, &value) in physical_stored.iter().zip(physical) {
                let row = design_row(stored);
                for coordinate in 0..4 {
                    rhs[coordinate] += row[coordinate] * value;
                }
            }
            rhs
        };
        let cell_prediction = |coefficient: [f64; 4]| {
            [
                coefficient[0] + coefficient[2],
                coefficient[0] + coefficient[3],
                coefficient[1] + coefficient[2],
                coefficient[1] + coefficient[3],
            ]
        };
        let physical_y = physical_stored
            .iter()
            .map(|&stored| stored_y[stored])
            .collect::<Vec<_>>();
        let coefficient = solve(transpose(&physical_y));
        let fitted_cell = cell_prediction(coefficient);

        let target_total: f64 = stored_target.iter().sum();
        let mut worker_mean = 0.0;
        let mut firm_mean = 0.0;
        for stored in 0..8 {
            worker_mean += stored_target[stored] * coefficient[stored_worker[stored]];
            firm_mean += stored_target[stored] * coefficient[2 + stored_firm[stored]];
        }
        worker_mean /= target_total;
        firm_mean /= target_total;
        let mut plugin = [0.0; 4];
        for stored in 0..8 {
            let worker = coefficient[stored_worker[stored]] - worker_mean;
            let firm = coefficient[2 + stored_firm[stored]] - firm_mean;
            plugin[0] += stored_target[stored] * worker * worker / target_total;
            plugin[1] += stored_target[stored] * firm * firm / target_total;
            plugin[2] += stored_target[stored] * worker * firm / target_total;
        }
        plugin[3] = plugin[0] + plugin[1] + 2.0 * plugin[2];

        let seed = 8_675_309_u64;
        let probes = 5_usize;
        let mut moment = [[0.0; 5]; 8];
        for probe in 0..probes {
            let mut physical_sign = vec![0.0; 13];
            let mut cursor = 0;
            for (stored, &frequency) in stored_frequency.iter().enumerate() {
                for copy in 0..frequency {
                    physical_sign[cursor] = oracle_sign(
                        seed,
                        ORACLE_LEVERAGE_DOMAIN_TAG,
                        probe as u64,
                        (stored + 1) as u64,
                        copy as u64,
                    );
                    cursor += 1;
                }
            }
            let projection = cell_prediction(solve(transpose(&physical_sign)));
            let mut cursor = 0;
            for stored in 0..8 {
                let frequency = stored_frequency[stored];
                let sign_sum: f64 = physical_sign[cursor..cursor + frequency].iter().sum();
                cursor += frequency;
                let cell = 2 * stored_worker[stored] + stored_firm[stored];
                let pi = (frequency as f64).sqrt() * projection[cell];
                let mu = sign_sum / (frequency as f64).sqrt() - pi;
                let p2 = pi * pi;
                let m2 = mu * mu;
                moment[stored][0] += p2;
                moment[stored][1] += m2;
                moment[stored][2] += p2 * p2;
                moment[stored][3] += m2 * m2;
                moment[stored][4] += p2 * m2;
            }
        }
        let mut cell_correction = [0.0; 4];
        for stored in 0..8 {
            let total = (moment[stored][0] + moment[stored][1]) / probes as f64;
            let projection = (moment[stored][0] / probes as f64) / total;
            let residual = (moment[stored][1] / probes as f64) / total;
            let p2 = moment[stored][2] / probes as f64;
            let m2 = moment[stored][3] / probes as f64;
            let mixed = moment[stored][4] / probes as f64;
            let bias =
                (residual * p2 - projection * m2 + (residual - projection) * mixed) / probes as f64;
            let variance = (residual * residual * p2 + projection * projection * m2
                - 2.0 * projection * residual * mixed)
                / probes as f64;
            let cell = 2 * stored_worker[stored] + stored_firm[stored];
            let outcome_sum = stored_frequency[stored] as f64 * stored_y[stored];
            let residual_mass = outcome_sum - stored_frequency[stored] as f64 * fitted_cell[cell];
            let deleted = residual_mass
                * (residual.recip() + bias / residual.powi(2)
                    - variance.max(0.0) / residual.powi(3));
            cell_correction[cell] += outcome_sum * deleted;
        }

        // (cell, per-copy target mass, physical trials, semantic entity rank)
        let strata = [
            (0_usize, 1.0_f64, 3_usize, 1_u64),
            (1, 1.0, 2, 4),
            (1, 2.0, 1, 3),
            (2, 3.0, 4, 5),
            (3, 4.0, 3, 7),
        ];
        let cell_target = [3.0, 4.0, 12.0, 12.0];
        let mut draws = [[0.0; 4]; 5];
        for probe in 0..probes {
            let mut first = [0.0; 4];
            for &(cell, per_copy, trials, entity) in &strata {
                let sign_sum: f64 = (0..trials)
                    .map(|copy| {
                        oracle_sign(
                            seed,
                            ORACLE_TARGET_DOMAIN_TAG,
                            probe as u64,
                            entity,
                            copy as u64,
                        )
                    })
                    .sum();
                first[cell] += (per_copy / target_total).sqrt() * sign_sum;
            }
            let total: f64 = first.iter().sum();
            let direction = std::array::from_fn::<_, 4, _>(|cell| {
                first[cell] - cell_target[cell] / target_total * total
            });
            let worker_score = [direction[0] + direction[1], direction[2] + direction[3]];
            let firm_score = [direction[0] + direction[2], direction[1] + direction[3]];
            let worker_prediction =
                cell_prediction(solve([worker_score[0], -worker_score[0], 0.0, 0.0]));
            let firm_prediction = cell_prediction(solve([0.0, 0.0, firm_score[0], -firm_score[0]]));
            for cell in 0..4 {
                draws[probe][0] +=
                    cell_correction[cell] * worker_prediction[cell] * worker_prediction[cell];
                draws[probe][1] +=
                    cell_correction[cell] * firm_prediction[cell] * firm_prediction[cell];
                draws[probe][2] +=
                    cell_correction[cell] * worker_prediction[cell] * firm_prediction[cell];
            }
            draws[probe][3] = draws[probe][0] + draws[probe][1] + 2.0 * draws[probe][2];
        }
        let mut correction = [0.0; 4];
        for draw in draws {
            for component in 0..4 {
                correction[component] += draw[component] / probes as f64;
            }
        }
        let corrected = std::array::from_fn(|component| plugin[component] - correction[component]);
        let mut mcse = [0.0; 4];
        for component in 0..4 {
            mcse[component] = (draws
                .iter()
                .map(|draw| (draw[component] - correction[component]).powi(2))
                .sum::<f64>()
                / (probes * (probes - 1)) as f64)
                .sqrt();
        }
        DenseOracle {
            fitted_cell,
            plugin,
            correction,
            corrected,
            mcse,
            draws,
        }
    }

    fn oracle_gaussian_solve(mut matrix: Vec<f64>, mut rhs: Vec<f64>) -> Vec<f64> {
        let dimension = rhs.len();
        for column in 0..dimension {
            let pivot = (column..dimension)
                .max_by(|&left, &right| {
                    matrix[left * dimension + column]
                        .abs()
                        .total_cmp(&matrix[right * dimension + column].abs())
                })
                .expect("oracle pivot");
            assert!(matrix[pivot * dimension + column].abs() > 1.0e-14);
            if pivot != column {
                for entry in 0..dimension {
                    matrix.swap(column * dimension + entry, pivot * dimension + entry);
                }
                rhs.swap(column, pivot);
            }
            let pivot_value = matrix[column * dimension + column];
            for row in column + 1..dimension {
                let factor = matrix[row * dimension + column] / pivot_value;
                for entry in column..dimension {
                    matrix[row * dimension + entry] -= factor * matrix[column * dimension + entry];
                }
                rhs[row] -= factor * rhs[column];
            }
        }
        let mut solution = vec![0.0; dimension];
        for row in (0..dimension).rev() {
            let mut value = rhs[row];
            for column in row + 1..dimension {
                value -= matrix[row * dimension + column] * solution[column];
            }
            solution[row] = value / matrix[row * dimension + row];
        }
        solution
    }

    fn oracle_sign(seed: u64, domain: u64, probe: u64, entity: u64, copy: u64) -> f64 {
        let word = oracle_philox_word(seed, domain, probe, entity, copy / 64);
        if (word >> (copy % 64)) & 1 == 0 {
            -1.0
        } else {
            1.0
        }
    }

    fn oracle_philox_word(seed: u64, domain: u64, probe: u64, entity: u64, word_index: u64) -> u64 {
        let mut counter = [entity, probe / 4, word_index, domain];
        let mut key = [
            oracle_splitmix64(seed),
            oracle_splitmix64(seed ^ 0xd1b5_4a32_d192_ed03),
        ];
        for round in 0..10 {
            let product_0 = u128::from(0xd2b7_4407_b1ce_6e93_u64) * u128::from(counter[0]);
            let product_1 = u128::from(0xca5a_8263_9512_1157_u64) * u128::from(counter[2]);
            counter = [
                (product_1 >> 64) as u64 ^ counter[1] ^ key[0],
                product_1 as u64,
                (product_0 >> 64) as u64 ^ counter[3] ^ key[1],
                product_0 as u64,
            ];
            if round != 9 {
                key[0] = key[0].wrapping_add(0x9e37_79b9_7f4a_7c15);
                key[1] = key[1].wrapping_add(0xbb67_ae85_84ca_a73b);
            }
        }
        counter[(probe % 4) as usize]
    }

    fn oracle_splitmix64(input: u64) -> u64 {
        let mut value = input.wrapping_add(0x9e37_79b9_7f4a_7c15);
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        value ^ (value >> 31)
    }

    #[test]
    fn eight_row_source_audit_stages_match() {
        let oracle = expanded_dense_oracle();
        let result = run_jla_no_controls(
            &audit_problem(&(0..8).collect::<Vec<_>>()),
            audit_options(LinearSolverRoute::Exact),
        )
        .expect("source-bound engine result");
        assert_close(
            &result.fitted_cell,
            &[
                2.022222222222222,
                1.6444444444444444,
                0.7333333333333334,
                0.3555555555555555,
            ],
            2.0e-14,
        );
        assert_close(&oracle.fitted_cell, &result.fitted_cell, 3.0e-13);
        assert_close(
            &values(result.plugin),
            &[
                0.2904135352834625,
                0.03564188538173971,
                -0.006080086329826184,
                0.31389524800554985,
            ],
            2.0e-14,
        );
        assert_close(&oracle.plugin, &values(result.plugin), 3.0e-13);
        assert_close(
            &values(result.correction),
            &[
                0.15479895785739795,
                0.28815866783315836,
                -0.03907951721633791,
                0.3647985912578805,
            ],
            3.0e-13,
        );
        assert_close(&oracle.correction, &values(result.correction), 3.0e-13);
        assert_close(
            &values(result.corrected),
            &[
                0.13561457742606453,
                -0.2525167824514186,
                0.03299943088651172,
                -0.050903343252330646,
            ],
            3.0e-13,
        );
        assert_close(&oracle.corrected, &values(result.corrected), 3.0e-13);
        assert_close(
            &mcse_values(result.numerical_mcse),
            &[
                0.07329438550816288,
                0.15562409207174877,
                0.04398193783561902,
                0.12373916487436575,
            ],
            3.0e-13,
        );
        assert_close(&oracle.mcse, &mcse_values(result.numerical_mcse), 3.0e-13);
        let expected_draws = [
            [
                0.000005919592148325,
                0.14795414857041678,
                -0.000359573162336864,
                0.14724092183789136,
            ],
            [
                0.3238274440379454,
                0.03910659880532012,
                0.04323740813370915,
                0.44940885911068384,
            ],
            [
                0.001030790584914836,
                0.2875533494713059,
                0.006614884114337031,
                0.3018139082848948,
            ],
            [
                0.1208096286496324,
                0.07892603991118426,
                -0.03751791839019003,
                0.12469983178043659,
            ],
            [
                0.32832100642234874,
                0.8872532024075648,
                -0.20737238677720882,
                0.8008294352754959,
            ],
        ];
        for ((draw, expected), oracle_draw) in result
            .target_draws
            .iter()
            .zip(expected_draws)
            .zip(oracle.draws)
        {
            assert_close(&values(*draw), &expected, 3.0e-13);
            assert_close(&oracle_draw, &values(*draw), 3.0e-13);
        }
        assert_eq!(result.receipt.leverage_rhs.len(), 5);
        assert_eq!(result.receipt.target_rhs.len(), 10);
        assert!(result.receipt.max_complete_residual <= 1.0e-11);
        let outcome = [1.0, 3.0, 0.0, 2.0, -1.0, 1.0, 2.0, -2.0];
        let frequency = [1.0, 2.0, 1.0, 2.0, 1.0, 3.0, 2.0, 1.0];
        let row_cell = [0_usize, 0, 1, 1, 2, 2, 3, 3];
        let expected_rss = outcome
            .iter()
            .zip(frequency)
            .zip(row_cell)
            .map(|((&value, weight), cell)| weight * (value - result.fitted_cell[cell]).powi(2))
            .sum::<f64>();
        assert!((result.weighted_rss - expected_rss).abs() <= 2.0e-14);
        assert!(
            result.receipt.memory.solve_peak_forecast_bytes
                <= result.receipt.memory.hard_limit_bytes
        );
        assert!(
            result.receipt.memory.solve_peak_forecast_bytes
                >= result.receipt.memory.prepared_persistent_bytes
        );
    }

    #[test]
    fn whole_solve_memory_is_rejected_before_estimation() {
        let problem = audit_problem(&(0..8).collect::<Vec<_>>());
        let mut options = audit_options(LinearSolverRoute::Exact);
        options.memory_limit_bytes = 1;
        let error = run_jla_no_controls(&problem, options)
            .expect_err("one-byte whole-command limit must fail");
        assert_eq!(error.code, ErrorCode::ResourceLimit);
        assert_eq!(error.phase, "jla_memory");
    }

    #[test]
    fn target_memory_uses_built_plan_cardinality() {
        let problem = audit_problem(&(0..8).collect::<Vec<_>>());
        let plan = JlaPlan::build_no_controls(&problem).expect("audit plan");
        assert_eq!(problem.dimensions.target_strata, 8);
        assert_eq!(plan.target_strata(), 5);

        let options = audit_options(LinearSolverRoute::Exact);
        let prepared = prepared_problem_bytes(&problem, &plan).expect("prepared footprint");
        let receipt = admit_jla_memory(&problem, &plan, options, prepared)
            .expect("the exact plan forecast fits");
        let workers = u64::try_from(problem.workers()).expect("worker count");
        let firms = u64::try_from(problem.firms()).expect("firm count");
        let cells = u64::try_from(problem.cells()).expect("cell count");
        let width = u64::from(options.probes)
            .min(u64::try_from(options.target_batch_width).expect("target batch width"));
        let exact =
            target_phase_forecast(LinearSolverRoute::Exact, workers, firms, cells, 5, width, 0)
                .expect("exact target phase");
        let compression_cardinality =
            target_phase_forecast(LinearSolverRoute::Exact, workers, firms, cells, 8, width, 0)
                .expect("compression target phase");
        assert_eq!(receipt.target_phase_forecast_bytes, exact);
        assert!(receipt.target_phase_forecast_bytes < compression_cardinality);
    }

    #[test]
    fn batch_width_and_row_order_do_not_change_logical_result() {
        let problem = audit_problem(&(0..8).collect::<Vec<_>>());
        let mut one = audit_options(LinearSolverRoute::Exact);
        one.leverage_batch_width = 1;
        one.target_batch_width = 1;
        let mut all = one;
        all.leverage_batch_width = 5;
        all.target_batch_width = 5;
        let left = run_jla_no_controls(&problem, one).expect("scalar batches");
        let right = run_jla_no_controls(&problem, all).expect("wide batches");
        assert_close(&values(left.correction), &values(right.correction), 0.0);

        let permuted = audit_problem(&[7, 2, 5, 0, 6, 1, 4, 3]);
        let reordered = run_jla_no_controls(&permuted, one).expect("permuted rows");
        assert_close(
            &values(left.correction),
            &values(reordered.correction),
            2.0e-13,
        );
    }

    #[test]
    fn order_preserving_identifier_relabeling_is_invariant() {
        let order = [7, 2, 5, 0, 6, 1, 4, 3];
        let original = run_jla_no_controls(
            &audit_problem(&order),
            audit_options(LinearSolverRoute::Exact),
        )
        .expect("original labels");
        let relabelled = run_jla_no_controls(
            &relabelled_audit_problem(&order),
            audit_options(LinearSolverRoute::Exact),
        )
        .expect("order-preserving relabeling");
        assert_close(&original.fitted_cell, &relabelled.fitted_cell, 0.0);
        assert_close(&values(original.plugin), &values(relabelled.plugin), 0.0);
        assert_close(
            &values(original.correction),
            &values(relabelled.correction),
            0.0,
        );
        assert_close(
            &values(original.corrected),
            &values(relabelled.corrected),
            0.0,
        );
        for (left, right) in original.target_draws.iter().zip(&relabelled.target_draws) {
            assert_close(&values(*left), &values(*right), 0.0);
        }
    }

    #[test]
    fn leverage_and_target_counter_domains_are_separate() {
        let rng = CounterRng::new(8_675_309);
        let entity = [1_u64, 4, 7];
        let trials = [3_u64, 2, 9];
        let mut interrupt = NeverInterrupt;
        let leverage = rademacher_atoms_with_interrupt(
            rng,
            ProbeDomain::Leverage,
            0,
            5,
            &entity,
            &trials,
            &mut interrupt,
        )
        .expect("leverage atoms");
        let target = rademacher_atoms_with_interrupt(
            rng,
            ProbeDomain::Target,
            0,
            5,
            &entity,
            &trials,
            &mut interrupt,
        )
        .expect("target atoms");
        assert_ne!(leverage, target);
        assert_ne!(ProbeDomain::Leverage.tag(), ProbeDomain::Target.tag());
    }

    #[test]
    fn ordered_probe_blocks_match_scalar_counter_fill_bitwise() {
        let problem = audit_problem(&(0..8).collect::<Vec<_>>());
        let plan = JlaPlan::build_no_controls(&problem).expect("audit plan");
        let options = audit_options(LinearSolverRoute::Exact);
        let solver =
            PreparedTwoWaySolver::prepare(&problem, options.solver).expect("prepared exact solver");
        for width in 1..=9 {
            let mut interrupt = NeverInterrupt;
            let expected = rademacher_atoms_with_interrupt(
                CounterRng::new(8_675_309),
                ProbeDomain::Target,
                3,
                width,
                &plan.target.semantic_rank,
                &plan.target.physical_count,
                &mut interrupt,
            )
            .expect("scalar counter fill");
            let actual = rademacher_atoms_ordered_with_interrupt(
                CounterRng::new(8_675_309),
                ProbeDomain::Target,
                3,
                width,
                &plan.target.semantic_rank,
                &plan.target.physical_count,
                &solver,
                &mut interrupt,
            )
            .expect("ordered probe blocks");
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn direct_leverage_rhs_matches_materialized_cell_transpose_bitwise() {
        let problem = audit_problem(&(0..8).collect::<Vec<_>>());
        let plan = JlaPlan::build_no_controls(&problem).expect("audit plan");
        let width = 3;
        let mut interrupt = NeverInterrupt;
        let atoms = rademacher_atoms_with_interrupt(
            CounterRng::new(8_675_309),
            ProbeDomain::Leverage,
            2,
            width,
            &plan.deletion.semantic_rank,
            &plan.deletion.physical_count,
            &mut interrupt,
        )
        .expect("leverage atoms");
        let mut cell = vec![0.0; problem.cells() * width];
        for column in 0..width {
            for group in 0..plan.deletion_units() {
                let target = usize::try_from(plan.deletion.cell[group]).expect("validated cell");
                cell[column * problem.cells() + target] +=
                    atoms[column * plan.deletion_units() + group] as f64;
            }
        }
        let expected = transpose_cell_rhs_with_interrupt(&problem, &cell, width, &mut interrupt)
            .expect("materialized transpose");
        let actual = leverage_rhs_with_interrupt(&problem, &plan, &atoms, width, &mut interrupt)
            .expect("direct leverage RHS");
        assert_eq!(actual, expected);
    }

    #[test]
    fn independent_target_preparation_matches_materialized_batch_bitwise() {
        let problem = audit_problem(&(0..8).collect::<Vec<_>>());
        let plan = JlaPlan::build_no_controls(&problem).expect("audit plan");
        let width = 3;
        let first_probe = 2;
        let mut interrupt = NeverInterrupt;
        let atoms = rademacher_atoms_with_interrupt(
            CounterRng::new(8_675_309),
            ProbeDomain::Target,
            first_probe,
            width,
            &plan.target.semantic_rank,
            &plan.target.physical_count,
            &mut interrupt,
        )
        .expect("target atoms");
        let (directions, reference_scale) = target_directions_with_interrupt(
            &problem,
            &plan,
            &atoms,
            width,
            first_probe,
            &mut interrupt,
        )
        .expect("materialized directions");
        let expected = target_rhs_with_interrupt(
            &problem,
            &directions,
            &reference_scale,
            width,
            first_probe,
            &mut interrupt,
        )
        .expect("materialized target RHS");
        let options = audit_options(LinearSolverRoute::Exact);
        let solver =
            PreparedTwoWaySolver::prepare(&problem, options.solver).expect("prepared exact solver");
        let actual = target_rhs_columns_with_interrupt(
            &problem,
            &plan,
            &atoms,
            width,
            first_probe,
            &solver,
            &mut interrupt,
        )
        .expect("independent target RHS");
        assert_eq!(actual, expected);
    }

    #[test]
    fn exact_zero_rhs_receipt_uses_original_rhs_norm() {
        let routed = RoutedSolveReceipt {
            requested: LinearSolverRoute::Exact,
            selected: LinearSolverRoute::Exact,
            dimension: 1,
            exact: Some(crate::exact::ExactSolveReceipt {
                dimension: 1,
                reduced_residual: 0.0,
                full_residual: 0.0,
            }),
            pcg: None,
            cmg: None,
            fallback: None,
        };
        let nonzero = rhs_receipt(
            &routed,
            0.0,
            1.0,
            JlaSolvePhase::FullFit,
            None,
            JlaRhsSide::Joint,
        )
        .expect("nonzero exact RHS receipt");
        let zero = rhs_receipt(
            &routed,
            5.0e-17,
            0.0,
            JlaSolvePhase::FullFit,
            None,
            JlaRhsSide::Joint,
        )
        .expect("zero exact RHS receipt");
        assert!(!nonzero.zero_rhs);
        assert!(zero.zero_rhs);
        assert_eq!(zero.iterations, 0);
        assert_eq!(zero.reduced_residual, 0.0);
        assert_eq!(zero.complete_residual, 0.0);
    }

    #[test]
    fn near_proportional_target_uses_the_uncentered_reference_scale() {
        let mut score = [1.0e-12, -1.0e-12 + 5.0e-14];
        balance_score(&mut score, 1.0, 37, JlaRhsSide::Worker)
            .expect("uncentered target scale admits roundoff-sized repair");
        assert_eq!(score, [1.0e-12, -1.0e-12]);

        let mut centered_scale_score = [1.0e-12, -1.0e-12 + 5.0e-14];
        let error = balance_score(&mut centered_scale_score, 2.0e-12, 37, JlaRhsSide::Worker)
            .expect_err("the obsolete centered scale would falsely reject this boundary");
        assert_eq!(error.code, ErrorCode::TargetCenteringFailed);
        assert!(error.message.contains("probe 37"));
    }

    #[test]
    fn target_centering_failure_reports_the_global_probe() {
        let problem = audit_problem(&(0..8).collect::<Vec<_>>());
        let mut direction = vec![0.0; 2 * problem.cells()];
        direction[problem.cells()] = 1.0;
        let error = target_rhs(&problem, &direction, &[0.0, 1.0e-16], 2, 40)
            .expect_err("incompatible second-column target score");
        assert_eq!(error.code, ErrorCode::TargetCenteringFailed);
        assert!(error.message.contains("probe 41"));
        assert!(error.message.contains("Worker"));
    }

    #[test]
    fn physical_trial_limits_are_preflighted_for_both_counter_domains() {
        let maximum_trials = MAX_PHYSICAL_WORDS_PER_ATOM * 64;
        let leverage = trial_preflight_problem(
            vec![1, 1, 2, 2],
            vec![1, 2, 1, 2],
            vec![maximum_trials + 1, 1, 1, 1],
            vec![1.0; 4],
        );
        let leverage_error = run_jla_no_controls(&leverage, JlaEngineOptions::default())
            .expect_err("oversized deletion atom");
        assert_eq!(leverage_error.code, ErrorCode::ResourceLimit);
        assert_eq!(leverage_error.phase, "jla_validate");
        assert!(leverage_error.message.contains("leverage trial preflight"));

        let half = maximum_trials / 2 + 1;
        let target = trial_preflight_problem(
            vec![1, 1, 1, 2, 2],
            vec![1, 1, 2, 1, 2],
            vec![half, half, 1, 1, 1],
            vec![half as f64, half as f64, 1.0, 1.0, 1.0],
        );
        let target_error = run_jla_no_controls(&target, JlaEngineOptions::default())
            .expect_err("oversized target-stratum atom");
        assert_eq!(target_error.code, ErrorCode::ResourceLimit);
        assert_eq!(target_error.phase, "jla_validate");
        assert!(target_error.message.contains("target trial preflight"));
    }

    #[test]
    fn rank_and_block_tolerance_boundaries_are_enforced() {
        let options = JlaEngineOptions {
            rank_tolerance: 1.0e-14,
            block_tolerance: 1.0e-14,
            ..JlaEngineOptions::default()
        };
        options.validate().expect("inclusive lower boundaries");

        let options = JlaEngineOptions {
            rank_tolerance: 9.999_999_999_999_998e-15,
            ..JlaEngineOptions::default()
        };
        assert_eq!(
            options.validate().expect_err("rank below lower bound").code,
            ErrorCode::InvalidInput
        );
        let options = JlaEngineOptions {
            block_tolerance: 9.999_999_999_999_998e-15,
            ..JlaEngineOptions::default()
        };
        assert_eq!(
            options
                .validate()
                .expect_err("block below lower bound")
                .code,
            ErrorCode::InvalidInput
        );
        let options = JlaEngineOptions {
            rank_tolerance: 0.1,
            ..JlaEngineOptions::default()
        };
        assert_eq!(
            options.validate().expect_err("rank upper boundary").code,
            ErrorCode::InvalidInput
        );
        let options = JlaEngineOptions {
            block_tolerance: 1.0,
            ..JlaEngineOptions::default()
        };
        assert_eq!(
            options.validate().expect_err("block upper boundary").code,
            ErrorCode::InvalidInput
        );
    }

    #[test]
    fn target_identity_and_nonfinite_results_have_distinct_typed_failures() {
        let valid = VarianceComponents {
            worker: 1.0,
            firm: 2.0,
            covariance: 3.0,
            total: 9.0,
        };
        let invalid_identity = VarianceComponents {
            total: 10.0,
            ..valid
        };
        let identity_error = accounting_residuals(valid, valid, valid, &[invalid_identity])
            .expect_err("bad target accounting identity");
        assert_eq!(identity_error.code, ErrorCode::TargetIdentityFailed);
        assert_eq!(identity_error.code.as_str(), "TARGET_IDENTITY_FAILED");

        let nonfinite_draw = VarianceComponents {
            worker: f64::NAN,
            ..valid
        };
        let reduction_error =
            mean_components(&[nonfinite_draw]).expect_err("nonfinite correction reduction");
        assert_eq!(reduction_error.code, ErrorCode::CorrectionNonFinite);

        let large = VarianceComponents {
            worker: f64::MAX,
            firm: 0.0,
            covariance: 0.0,
            total: f64::MAX,
        };
        let negative_large = VarianceComponents {
            worker: -f64::MAX,
            firm: 0.0,
            covariance: 0.0,
            total: -f64::MAX,
        };
        let subtraction_error = subtract_components(large, negative_large)
            .expect_err("overflowing corrected target subtraction");
        assert_eq!(subtraction_error.code, ErrorCode::NonfiniteCorrectedTarget);
        assert_eq!(
            subtraction_error.code.as_str(),
            "NONFINITE_CORRECTED_TARGET"
        );
    }

    #[test]
    fn forced_exact_and_diagonal_routes_agree() {
        let problem = audit_problem(&(0..8).collect::<Vec<_>>());
        let exact = run_jla_no_controls(&problem, audit_options(LinearSolverRoute::Exact))
            .expect("exact engine");
        let diagonal = run_jla_no_controls(&problem, audit_options(LinearSolverRoute::DiagonalPcg))
            .expect("diagonal engine");
        assert_close(&values(exact.plugin), &values(diagonal.plugin), 2.0e-12);
        assert_close(
            &values(exact.correction),
            &values(diagonal.correction),
            2.0e-11,
        );
    }

    #[test]
    fn unsupported_modes_and_invalid_tuning_fail_before_estimation() {
        let problem = audit_problem(&(0..8).collect::<Vec<_>>());
        let mut options = audit_options(LinearSolverRoute::Exact);
        options.deletion = DeletionMode::Observation;
        assert_eq!(
            run_jla_no_controls(&problem, options)
                .expect_err("observation deletion")
                .code,
            ErrorCode::UnsupportedFeature
        );
        options = audit_options(LinearSolverRoute::Exact);
        options.rng = RngContract::StataCompatibility;
        assert_eq!(
            run_jla_no_controls(&problem, options)
                .expect_err("Stata RNG")
                .code,
            ErrorCode::UnsupportedFeature
        );
        options = audit_options(LinearSolverRoute::Exact);
        options.rank_tolerance = f64::NAN;
        assert_eq!(
            run_jla_no_controls(&problem, options)
                .expect_err("rank tolerance")
                .code,
            ErrorCode::InvalidInput
        );
    }

    #[test]
    fn solve_entry_and_dominant_helper_passes_break_after_bounded_work() {
        let problem = wide_work_problem(INTERRUPT_CHECK_CHUNK / 2 + 1);
        assert!(problem.cells() > INTERRUPT_CHECK_CHUNK);
        let plan = JlaPlan::build_no_controls(&problem).expect("plan");

        let mut setup = BreakOnPhase::new("operator_setup_cells", 2);
        let error = TwoWayOperator::new_with_interrupt(&problem, &mut setup)
            .expect_err("operator setup must poll beyond one chunk");
        assert_eq!(error.code, ErrorCode::UserBreak);
        assert_eq!(setup.calls, 2);

        let atoms = vec![1_i64; plan.deletion_units()];
        let mut rhs = BreakOnPhase::new("jla_leverage_rhs", 2);
        let error = leverage_rhs_with_interrupt(&problem, &plan, &atoms, 1, &mut rhs)
            .expect_err("RHS construction must poll beyond one chunk");
        assert_eq!(error.code, ErrorCode::UserBreak);
        assert_eq!(rhs.calls, 2);

        let worker = vec![0.0; problem.workers()];
        let firm = vec![0.0; problem.firms()];
        let mut prediction = BreakOnPhase::new("jla_prediction", 2);
        let error = cell_predictions_with_interrupt(&problem, &worker, &firm, &mut prediction)
            .expect_err("prediction must poll beyond one chunk");
        assert_eq!(error.code, ErrorCode::UserBreak);
        assert_eq!(prediction.calls, 2);

        let fitted = vec![0.0; problem.cells()];
        let mut rss = BreakOnPhase::new("jla_full_fit_rss", 2);
        let error = full_fit_weighted_rss_with_interrupt(&problem, &fitted, &mut rss)
            .expect_err("RSS must poll beyond one chunk");
        assert_eq!(error.code, ErrorCode::UserBreak);
        assert_eq!(rss.calls, 2);

        let draws = vec![VarianceComponents::default(); INTERRUPT_CHECK_CHUNK + 1];
        let mut finalize = BreakOnPhase::new("jla_finalize_mean", 2);
        let error = mean_components_with_interrupt(&draws, &mut finalize)
            .expect_err("final reduction must poll beyond one chunk");
        assert_eq!(error.code, ErrorCode::UserBreak);
        assert_eq!(finalize.calls, 2);

        let invalid = JlaEngineOptions {
            probes: 1,
            ..JlaEngineOptions::default()
        };
        let mut entry = BreakOnPhase::new("jla_solve_entry", 1);
        let error = run_jla_no_controls_with_interrupt(&problem, invalid, &mut entry)
            .expect_err("entry poll precedes option validation");
        assert_eq!(error.code, ErrorCode::UserBreak);
        assert_eq!(entry.calls, 1);
    }

    #[test]
    fn planned_adapter_freezes_independent_batches_counter_memory_and_wall() {
        let problem = audit_problem(&(0..8).collect::<Vec<_>>());
        let mut estimator = audit_options(LinearSolverRoute::Exact);
        estimator.probes = 40;
        let automatic = run_jla_no_controls_planned(
            &problem,
            PlannedJlaEngineOptions {
                estimator,
                leverage_batch: BatchRequest::Auto,
                target_batch: BatchRequest::Explicit(99),
                wallseconds: Some(0.25),
                full_cmg: None,
            },
        )
        .expect("planned compressed JLA");
        assert_eq!(
            automatic.execution.selected_engine,
            SelectedEngine::Compressed
        );
        assert_eq!(
            automatic.execution.planned_rhs,
            1 + 3 * u64::from(estimator.probes)
        );
        assert_eq!(automatic.execution.batch.leverage_active_width, 32);
        assert_eq!(automatic.execution.batch.target_active_width, 40);
        assert_eq!(
            automatic.execution.batch.target_requested,
            BatchRequest::Explicit(99)
        );
        assert_eq!(
            automatic.execution.batch.plan.target.requested,
            BatchRequest::Explicit(99)
        );
        assert_eq!(automatic.execution.batch.automatic_ladder_cap, 32);
        assert_eq!(
            automatic.execution.memory.solve_peak_forecast_bytes,
            automatic.execution.batch.plan.selected_command_peak_bytes
        );
        assert!(automatic.execution.plan_frozen_before_rng);
        assert_eq!(automatic.execution.logical_atoms_before_plan_freeze, 0);
        assert_eq!(
            automatic.execution.unique_packed_words_before_plan_freeze,
            0
        );
        assert_eq!(automatic.execution.physical_trials_before_plan_freeze, 0);
        assert_eq!(automatic.execution.threads.used, 1);
        assert_eq!(automatic.execution.threads.parallel_regions, 0);
        assert_eq!(
            automatic.execution.wall.status,
            crate::wall_plan::WallAdvisoryStatus::Uncalibrated
        );

        let plan = JlaPlan::build_no_controls(&problem).expect("counter plan");
        let probes = u64::from(estimator.probes);
        let leverage_words = plan
            .deletion
            .physical_count
            .iter()
            .map(|count| count.div_ceil(64))
            .sum::<u64>();
        let leverage_trials = plan.deletion.physical_count.iter().sum::<u64>();
        let target_words = plan
            .target
            .physical_count
            .iter()
            .map(|count| count.div_ceil(64))
            .sum::<u64>();
        let target_trials = plan.target.physical_count.iter().sum::<u64>();
        let counter = automatic.execution.counter;
        assert_eq!(
            counter.leverage.planned_logical_atoms,
            probes * plan.deletion_units() as u64
        );
        assert_eq!(
            counter.leverage.planned_unique_packed_words,
            probes * leverage_words
        );
        assert_eq!(
            counter.leverage.planned_physical_bernoulli_trials,
            probes * leverage_trials
        );
        assert_eq!(
            counter.target.planned_logical_atoms,
            probes * plan.target_strata() as u64
        );
        assert_eq!(
            counter.target.planned_unique_packed_words,
            probes * target_words
        );
        assert_eq!(
            counter.target.planned_physical_bernoulli_trials,
            probes * target_trials
        );
        assert_eq!(
            counter.total.actual_logical_atoms,
            counter.total.planned_logical_atoms
        );
        assert_eq!(
            counter.total.actual_unique_packed_words,
            counter.total.planned_unique_packed_words
        );
        assert_eq!(
            counter.total.actual_physical_bernoulli_trials,
            counter.total.planned_physical_bernoulli_trials
        );
    }

    #[test]
    fn planned_adapter_preserves_legacy_bits_and_has_an_exact_memory_boundary() {
        let problem = audit_problem(&(0..8).collect::<Vec<_>>());
        let mut estimator = audit_options(LinearSolverRoute::Exact);
        estimator.probes = 5;
        estimator.leverage_batch_width = 3;
        estimator.target_batch_width = 4;
        let request = PlannedJlaEngineOptions {
            estimator,
            leverage_batch: BatchRequest::Explicit(3),
            target_batch: BatchRequest::Explicit(4),
            wallseconds: None,
            full_cmg: None,
        };
        let planned = run_jla_no_controls_planned(&problem, request).expect("planned result");
        let legacy = run_jla_no_controls(&problem, estimator).expect("legacy result");
        assert_close(
            &values(planned.estimator.plugin),
            &values(legacy.plugin),
            0.0,
        );
        assert_close(
            &values(planned.estimator.correction),
            &values(legacy.correction),
            0.0,
        );
        for (left, right) in planned
            .estimator
            .target_draws
            .iter()
            .zip(&legacy.target_draws)
        {
            assert_close(&values(*left), &values(*right), 0.0);
        }

        let peak = planned.execution.memory.solve_peak_forecast_bytes;
        let admitted = PlannedJlaEngineOptions {
            estimator: JlaEngineOptions {
                memory_limit_bytes: peak,
                ..estimator
            },
            ..request
        };
        run_jla_no_controls_planned(&problem, admitted).expect("exact boundary admits");
        let rejected = PlannedJlaEngineOptions {
            estimator: JlaEngineOptions {
                memory_limit_bytes: peak - 1,
                ..estimator
            },
            ..request
        };
        assert_eq!(
            run_jla_no_controls_planned(&problem, rejected)
                .expect_err("one byte below explicit plan rejects")
                .code,
            ErrorCode::ResourceLimit
        );

        let with_wall = run_jla_no_controls_planned(
            &problem,
            PlannedJlaEngineOptions {
                wallseconds: Some(1.0),
                ..request
            },
        )
        .expect("advisory wall result");
        assert_close(
            &values(planned.estimator.corrected),
            &values(with_wall.estimator.corrected),
            0.0,
        );
    }

    #[test]
    fn planned_auto_cmg_setup_fallback_is_frozen_before_forecast_and_rng() {
        let problem = three_firm_cmg_problem();
        let mut estimator = audit_options(LinearSolverRoute::Auto);
        estimator.probes = 16;
        estimator.leverage_batch_width = 3;
        estimator.target_batch_width = 4;
        estimator.solver.exact_dimension_limit = 1;
        estimator.solver.cmg_minimum_dimension = 2;
        estimator.solver.allow_automatic_cmg_setup_fallback = true;
        estimator.solver.cmg.memory_limit_bytes = 1;
        let request = PlannedJlaEngineOptions {
            estimator,
            leverage_batch: BatchRequest::Explicit(3),
            target_batch: BatchRequest::Explicit(4),
            wallseconds: None,
            full_cmg: None,
        };
        let fallback =
            run_jla_no_controls_planned(&problem, request).expect("automatic setup fallback");
        assert_eq!(
            fallback.execution.requested_solver_route,
            LinearSolverRoute::Auto
        );
        assert_eq!(
            fallback.execution.selected_solver_route,
            LinearSolverRoute::DiagonalPcg
        );
        assert_eq!(
            fallback.execution.solver_setup.requested,
            LinearSolverRoute::Auto
        );
        assert_eq!(
            fallback.execution.solver_setup.selected,
            LinearSolverRoute::DiagonalPcg
        );
        let fallback_receipt = fallback
            .execution
            .solver_setup
            .fallback
            .as_ref()
            .expect("setup fallback receipt");
        assert_eq!(fallback_receipt.from, LinearSolverRoute::CmgPcg);
        assert_eq!(fallback_receipt.to, LinearSolverRoute::DiagonalPcg);
        assert_eq!(fallback_receipt.code, ErrorCode::ResourceLimit);
        assert!(fallback.execution.plan_frozen_before_rng);
        assert_eq!(fallback.execution.logical_atoms_before_plan_freeze, 0);
        assert_eq!(fallback.execution.unique_packed_words_before_plan_freeze, 0);
        assert_eq!(fallback.execution.physical_trials_before_plan_freeze, 0);

        let mut diagonal_estimator = estimator;
        diagonal_estimator.solver.route = LinearSolverRoute::DiagonalPcg;
        diagonal_estimator.solver.cmg.memory_limit_bytes = estimator.solver.cmg.memory_limit_bytes;
        let diagonal = run_jla_no_controls_planned(
            &problem,
            PlannedJlaEngineOptions {
                estimator: diagonal_estimator,
                ..request
            },
        )
        .expect("forced diagonal reference");
        assert_eq!(fallback.execution.memory, diagonal.execution.memory);
        assert_eq!(fallback.execution.wall.work, diagonal.execution.wall.work);
        assert_close(
            &values(fallback.estimator.plugin),
            &values(diagonal.estimator.plugin),
            0.0,
        );
        assert_close(
            &values(fallback.estimator.correction),
            &values(diagonal.estimator.correction),
            0.0,
        );

        let peak = fallback.execution.memory.solve_peak_forecast_bytes;
        let admitted = PlannedJlaEngineOptions {
            estimator: JlaEngineOptions {
                memory_limit_bytes: peak,
                ..estimator
            },
            ..request
        };
        run_jla_no_controls_planned(&problem, admitted).expect("fallback exact boundary admits");
        let rejected = PlannedJlaEngineOptions {
            estimator: JlaEngineOptions {
                memory_limit_bytes: peak - 1,
                ..estimator
            },
            ..request
        };
        assert_eq!(
            run_jla_no_controls_planned(&problem, rejected)
                .expect_err("fallback limit minus one rejects")
                .code,
            ErrorCode::ResourceLimit
        );

        let mut forced_cmg = estimator;
        forced_cmg.solver.route = LinearSolverRoute::CmgPcg;
        assert_eq!(
            run_jla_no_controls_planned(
                &problem,
                PlannedJlaEngineOptions {
                    estimator: forced_cmg,
                    ..request
                },
            )
            .expect_err("forced CMG setup failure is terminal")
            .code,
            ErrorCode::ResourceLimit
        );
    }
}
