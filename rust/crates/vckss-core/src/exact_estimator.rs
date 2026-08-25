// SPDX-License-Identifier: GPL-3.0-only

//! Deterministic dense KSS estimator.
//!
//! This is the estimator-level exact path.  It is distinct from the exact
//! two-way linear solver in [`crate::exact`]: the routines here construct the
//! complete identified design, calculate the leave-out correction for either
//! physical observations or declared match blocks, and evaluate the same
//! worker/firm target quadratic forms as the Mata implementation.

use crate::control_basis::{
    canonicalize_controls_in_order_with_interrupt, enforce_downstream_bound,
};
use crate::counter_accounting::{combine_counter_phases, CounterExecutionReceipt};
use crate::dense::{
    cholesky_factor, frobenius_norm, inverse_forward_error, invert_scaled_spd,
    invert_scaled_zero_sum_quotient, symmetric_eigen_extremes, DenseInverse,
};
use crate::engine_plan::SelectedEngine;
use crate::error::{BackendError, ErrorCode, Result};
use crate::interrupt::{
    checkpoint_chunk, stable_sort_by_with_interrupt, InterruptCheck, NeverInterrupt,
};
use crate::jla::{plugin_components_with_interrupt, VarianceComponents};
use crate::problem::CompressedProblem;
use crate::types::{DeletionMode, NuisanceMode};
use crate::wall_plan::{wall_work_receipt, WallCalibration, WallWork, WallWorkReceipt};

pub const EXACT_EXECUTION_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug)]
pub struct ExactEstimatorOptions {
    pub deletion: DeletionMode,
    pub nuisance: NuisanceMode,
    pub rank_tolerance: f64,
    pub block_tolerance: f64,
    /// Solver tolerance from which the mandatory complete-fit residual gate
    /// `max(1e-11, 10*tolerance)` is derived.
    pub solver_tolerance: f64,
    pub exact_limit: usize,
    pub blocksize_limit: usize,
    pub memory_limit_bytes: u64,
    pub prepared_persistent_bytes: u64,
}

impl Default for ExactEstimatorOptions {
    fn default() -> Self {
        Self {
            deletion: DeletionMode::Match,
            nuisance: NuisanceMode::Joint,
            rank_tolerance: 1.0e-10,
            block_tolerance: 1.0e-10,
            solver_tolerance: 1.0e-12,
            exact_limit: 500,
            blocksize_limit: 5_000,
            memory_limit_bytes: u64::MAX,
            prepared_persistent_bytes: 0,
        }
    }
}

impl ExactEstimatorOptions {
    fn validate(self) -> Result<Self> {
        if !self.rank_tolerance.is_finite()
            || self.rank_tolerance < 1.0e-14
            || self.rank_tolerance >= 0.1
        {
            return Err(BackendError::invalid(
                "exact_estimator",
                "rank tolerance must be finite and lie in [1e-14, 0.1)",
            ));
        }
        if !self.block_tolerance.is_finite()
            || self.block_tolerance < 1.0e-14
            || self.block_tolerance >= 1.0
        {
            return Err(BackendError::invalid(
                "exact_estimator",
                "block tolerance must be finite and lie in [1e-14, 1)",
            ));
        }
        if !self.solver_tolerance.is_finite()
            || self.solver_tolerance <= 0.0
            || self.solver_tolerance >= 0.1
        {
            return Err(BackendError::invalid(
                "exact_estimator",
                "solver tolerance must be finite and lie in (0, 0.1)",
            ));
        }
        if self.exact_limit < 2 || self.exact_limit > 2_000 {
            return Err(BackendError::invalid(
                "exact_estimator",
                "exact limit must lie in [2, 2000]",
            ));
        }
        if self.blocksize_limit == 0 || self.blocksize_limit > 1_000_000 {
            return Err(BackendError::invalid(
                "exact_estimator",
                "block-size limit must lie in [1, 1000000]",
            ));
        }
        if self.memory_limit_bytes == 0 {
            return Err(BackendError::invalid(
                "exact_estimator",
                "whole-command memory limit must be positive",
            ));
        }
        Ok(self)
    }
}

#[derive(Clone, Debug)]
pub struct ExactEstimatorReceipt {
    pub parameters: usize,
    pub full_parameters: usize,
    pub correction_parameters: usize,
    pub deletion_units: u64,
    pub max_leverage: f64,
    pub information_rcond: f64,
    /// Maximum residual of the information inverses only.
    pub inverse_relres: f64,
    pub inverse_original_relres: f64,
    /// Residual of the inverse square root used by match deletion.
    pub inverse_sqrt_relres: f64,
    /// Maximum reciprocal-maker solve residual, kept separate from inverses.
    pub maker_relres: f64,
    pub full_fit_relres: f64,
    pub working_fit_relres: f64,
    pub fit_residual_tolerance: f64,
    pub control_basis_relres: f64,
    pub control_basis_forward_error: f64,
    pub deletion_rank_gap: f64,
    pub firm_zero_sum_residual: f64,
    pub peak_forecast_bytes: u64,
    pub fit_peak_forecast_bytes: u64,
    pub correction_peak_forecast_bytes: u64,
    pub topology_checksum: u64,
}

#[derive(Clone, Debug)]
pub struct ExactEstimatorResult {
    pub plugin: VarianceComponents,
    pub correction: VarianceComponents,
    pub corrected: VarianceComponents,
    pub weighted_rss: f64,
    pub receipt: ExactEstimatorReceipt,
}

/// Frozen mixed-deletion partition for the exact mover-plus-stayer result.
///
/// The combined problem stores mover match groups first, followed by one
/// unique stored-row group for every eligible stayer row.  Frequencies on
/// those stayer rows count exchangeable physical observation deletions.
#[derive(Clone, Debug)]
pub struct ExactStayerHybridPlan {
    pub stayer_rows: Vec<bool>,
    pub mover_deletion_units: usize,
}

#[derive(Clone, Debug)]
pub struct ExactStayerHybridResult {
    pub estimator: ExactEstimatorResult,
    pub mover_correction: VarianceComponents,
    pub stayer_correction: VarianceComponents,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct PlannedExactEstimatorOptions {
    pub estimator: ExactEstimatorOptions,
    /// Advisory only. It cannot change exact admission or arithmetic.
    pub wallseconds: Option<f64>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExactPlanApplicability {
    NotApplicable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExactThreadReceipt {
    pub requested: usize,
    pub used: usize,
    pub parallel_regions: usize,
}

#[derive(Clone, Debug)]
pub struct ExactExecutionReceipt {
    pub schema_version: u32,
    pub selected_engine: SelectedEngine,
    pub solver_route: ExactPlanApplicability,
    pub batches: ExactPlanApplicability,
    pub peak_forecast_bytes: u64,
    pub fit_peak_forecast_bytes: u64,
    pub correction_peak_forecast_bytes: u64,
    pub wall: WallWorkReceipt,
    pub counter: CounterExecutionReceipt,
    pub plan_frozen_before_execution: bool,
    pub logical_atoms_before_plan_freeze: u64,
    pub unique_packed_words_before_plan_freeze: u64,
    pub physical_trials_before_plan_freeze: u64,
    pub threads: ExactThreadReceipt,
}

#[derive(Clone, Debug)]
pub struct PlannedExactEstimatorResult {
    pub estimator: ExactEstimatorResult,
    pub execution: ExactExecutionReceipt,
}

#[derive(Clone, Copy, Debug)]
struct MemoryForecast {
    peak: u64,
    fit_peak: u64,
    correction_peak: u64,
}

#[derive(Clone, Copy, Debug)]
struct FitResidualReceipt {
    relative_norm: f64,
    absolute_norm: f64,
    rhs_norm: f64,
}

pub fn run_exact_estimator(
    problem: &CompressedProblem,
    options: ExactEstimatorOptions,
) -> Result<ExactEstimatorResult> {
    run_exact_estimator_with_interrupt(problem, options, &mut NeverInterrupt)
}

pub fn run_exact_estimator_planned(
    problem: &CompressedProblem,
    options: PlannedExactEstimatorOptions,
) -> Result<PlannedExactEstimatorResult> {
    run_exact_estimator_planned_with_interrupt(problem, options, &mut NeverInterrupt)
}

pub fn run_exact_estimator_planned_with_interrupt(
    problem: &CompressedProblem,
    options: PlannedExactEstimatorOptions,
    interrupt: &mut dyn InterruptCheck,
) -> Result<PlannedExactEstimatorResult> {
    interrupt.checkpoint("exact_planned_entry")?;
    let estimator_options = options.estimator.validate()?;
    let wall = wall_work_receipt(
        exact_wall_work(problem, estimator_options)?,
        options.wallseconds,
        WallCalibration::Uncalibrated,
    )?;
    let estimator = run_exact_estimator_with_interrupt(problem, estimator_options, interrupt)?;
    let counter = combine_counter_phases(Default::default(), Default::default())?;
    Ok(PlannedExactEstimatorResult {
        execution: ExactExecutionReceipt {
            schema_version: EXACT_EXECUTION_SCHEMA_VERSION,
            selected_engine: SelectedEngine::NotApplicable,
            solver_route: ExactPlanApplicability::NotApplicable,
            batches: ExactPlanApplicability::NotApplicable,
            peak_forecast_bytes: estimator.receipt.peak_forecast_bytes,
            fit_peak_forecast_bytes: estimator.receipt.fit_peak_forecast_bytes,
            correction_peak_forecast_bytes: estimator.receipt.correction_peak_forecast_bytes,
            wall,
            counter,
            plan_frozen_before_execution: true,
            logical_atoms_before_plan_freeze: 0,
            unique_packed_words_before_plan_freeze: 0,
            physical_trials_before_plan_freeze: 0,
            threads: ExactThreadReceipt {
                requested: 1,
                used: 1,
                parallel_regions: 0,
            },
        },
        estimator,
    })
}

fn exact_wall_work(
    problem: &CompressedProblem,
    options: ExactEstimatorOptions,
) -> Result<WallWork> {
    let rows = exact_wall_u64(problem.outcome.len(), "row count")?;
    let workers = exact_wall_u64(problem.workers(), "worker count")?;
    let firms = exact_wall_u64(problem.firms(), "firm count")?;
    let controls = exact_wall_u64(problem.controls.len(), "control count")?;
    let cells = exact_wall_u64(problem.cells(), "cell count")?;
    let deletion_units = exact_wall_u64(problem.deletion_units(), "deletion-unit count")?;
    let firm_quotient = firms
        .checked_sub(1)
        .ok_or_else(|| exact_wall_overflow("firm quotient"))?;
    let fe_parameters = exact_wall_sum(&[workers, firm_quotient], "FE parameters")?;
    let full_parameters = exact_wall_sum(&[fe_parameters, controls], "full parameters")?;
    let full_embedding = full_parameters
        .checked_add(1)
        .ok_or_else(|| exact_wall_overflow("full embedding"))?;
    let working_embedding = if options.nuisance == NuisanceMode::FixedOffset && controls > 0 {
        fe_parameters
            .checked_add(1)
            .ok_or_else(|| exact_wall_overflow("working embedding"))?
    } else {
        full_embedding
    };
    let preparation_terms = controls
        .checked_add(4)
        .ok_or_else(|| exact_wall_overflow("preparation terms"))?;
    let setup = exact_wall_sum(
        &[
            exact_wall_product(
                &[full_embedding, full_embedding, full_embedding],
                "full factor",
            )?,
            if working_embedding == full_embedding {
                0
            } else {
                exact_wall_product(
                    &[working_embedding, working_embedding, working_embedding],
                    "working factor",
                )?
            },
        ],
        "factor work",
    )?;
    let fit_terms = exact_wall_sum(&[full_embedding, working_embedding], "fit terms")?;
    let correction_scale = match options.deletion {
        DeletionMode::Observation => problem.physical_total,
        DeletionMode::Match => deletion_units,
    };
    Ok(WallWork {
        preparation: exact_wall_product(&[rows, preparation_terms], "preparation")?,
        engine_setup: setup,
        full_fit: exact_wall_product(&[rows, fit_terms], "fit")?,
        leverage: 0,
        target: exact_wall_product(
            &[correction_scale, working_embedding, working_embedding],
            "deletion correction",
        )?,
        result_export: exact_wall_sum(&[cells, deletion_units, full_parameters], "result export")?,
    })
}

fn exact_wall_u64(value: usize, label: &str) -> Result<u64> {
    u64::try_from(value).map_err(|_| exact_wall_overflow(label))
}

fn exact_wall_product(values: &[u64], label: &str) -> Result<u64> {
    values.iter().try_fold(1_u64, |total, &value| {
        total
            .checked_mul(value)
            .ok_or_else(|| exact_wall_overflow(label))
    })
}

fn exact_wall_sum(values: &[u64], label: &str) -> Result<u64> {
    values.iter().try_fold(0_u64, |total, &value| {
        total
            .checked_add(value)
            .ok_or_else(|| exact_wall_overflow(label))
    })
}

fn exact_wall_overflow(label: &str) -> BackendError {
    BackendError::new(
        ErrorCode::ResourceLimit,
        "exact_wall",
        format!("{label} wall-work overflow"),
    )
}

pub fn run_exact_estimator_with_interrupt(
    problem: &CompressedProblem,
    options: ExactEstimatorOptions,
    interrupt: &mut dyn InterruptCheck,
) -> Result<ExactEstimatorResult> {
    let (result, sources) = run_exact_estimator_internal(problem, options, None, interrupt)?;
    debug_assert!(sources.is_none());
    Ok(result)
}

pub fn run_exact_stayer_hybrid(
    problem: &CompressedProblem,
    plan: &ExactStayerHybridPlan,
    options: ExactEstimatorOptions,
) -> Result<ExactStayerHybridResult> {
    run_exact_stayer_hybrid_with_interrupt(problem, plan, options, &mut NeverInterrupt)
}

pub fn run_exact_stayer_hybrid_with_interrupt(
    problem: &CompressedProblem,
    plan: &ExactStayerHybridPlan,
    options: ExactEstimatorOptions,
    interrupt: &mut dyn InterruptCheck,
) -> Result<ExactStayerHybridResult> {
    validate_stayer_hybrid_plan(problem, plan, options, interrupt)?;
    let (estimator, sources) =
        run_exact_estimator_internal(problem, options, Some(plan), interrupt)?;
    let [mover_correction, stayer_correction] = sources.ok_or_else(|| {
        BackendError::invariant(
            "exact_stayer_hybrid",
            "mixed-deletion correction sources were not returned",
        )
    })?;
    Ok(ExactStayerHybridResult {
        estimator,
        mover_correction,
        stayer_correction,
    })
}

fn validate_stayer_hybrid_plan(
    problem: &CompressedProblem,
    plan: &ExactStayerHybridPlan,
    options: ExactEstimatorOptions,
    interrupt: &mut dyn InterruptCheck,
) -> Result<()> {
    if options.deletion != DeletionMode::Match {
        return Err(BackendError::new(
            ErrorCode::UnsupportedFeature,
            "exact_stayer_hybrid",
            "the mixed-deletion stayer hybrid requires a mover-match headline",
        ));
    }
    if plan.stayer_rows.len() != problem.outcome.len()
        || plan.mover_deletion_units == 0
        || plan.mover_deletion_units > problem.deletion_units()
    {
        return Err(BackendError::invalid(
            "exact_stayer_hybrid",
            "the mixed-deletion partition has invalid dimensions",
        ));
    }
    let mut mover_rows = 0_usize;
    let mut stayer_rows = 0_usize;
    for (row, &stayer) in plan.stayer_rows.iter().enumerate() {
        checkpoint_chunk(interrupt, row, "exact_stayer_partition_rows")?;
        if stayer {
            stayer_rows += 1;
        } else {
            mover_rows += 1;
        }
    }
    if mover_rows == 0 {
        return Err(BackendError::new(
            ErrorCode::GraphEmpty,
            "exact_stayer_hybrid",
            "the mixed-deletion partition has no retained mover rows",
        ));
    }
    if problem.deletion_units() != plan.mover_deletion_units + stayer_rows {
        return Err(BackendError::invalid(
            "exact_stayer_hybrid",
            "every stayer stored row must own one unique trailing deletion group",
        ));
    }
    for group in 0..problem.deletion_units() {
        interrupt.checkpoint("exact_stayer_partition_group")?;
        let range = problem.deletion_index.range(group);
        let expect_stayer = group >= plan.mover_deletion_units;
        if expect_stayer && range.len() != 1 {
            return Err(BackendError::invalid(
                "exact_stayer_hybrid",
                "a stayer deletion group does not contain exactly one stored row",
            ));
        }
        for &item in &problem.deletion_index.items[range] {
            let row = usize::try_from(item).expect("validated deletion row");
            if plan.stayer_rows[row] != expect_stayer {
                return Err(BackendError::invalid(
                    "exact_stayer_hybrid",
                    "mover and stayer deletion groups overlap",
                ));
            }
        }
    }
    Ok(())
}

fn run_exact_estimator_internal(
    problem: &CompressedProblem,
    options: ExactEstimatorOptions,
    hybrid: Option<&ExactStayerHybridPlan>,
    interrupt: &mut dyn InterruptCheck,
) -> Result<(ExactEstimatorResult, Option<[VarianceComponents; 2]>)> {
    interrupt.checkpoint("exact_estimator_entry")?;
    let options = options.validate()?;
    let rows = problem.outcome.len();
    let workers = problem.workers();
    let firms = problem.firms();
    let controls = problem.controls.len();
    if rows == 0 || workers == 0 || firms < 2 {
        return Err(BackendError::invalid(
            "exact_estimator",
            "the retained exact problem has invalid dimensions",
        ));
    }
    // Public parameter counts are quotient dimensions.  Dense arithmetic is
    // performed in a permutation-equivariant W + F (+ Q) embedding whose
    // firm block is constrained to zero sum.
    let fe_parameters = workers
        .checked_add(firms - 1)
        .ok_or_else(|| resource_error("FE parameter count overflow"))?;
    let full_parameters = fe_parameters
        .checked_add(controls)
        .ok_or_else(|| resource_error("full parameter count overflow"))?;
    if full_parameters > options.exact_limit {
        return Err(BackendError::new(
            ErrorCode::ResourceLimit,
            "exact_estimator",
            "identified coefficient dimension exceeds exact_limit()",
        ));
    }
    let full_embedding = full_parameters
        .checked_add(1)
        .ok_or_else(|| resource_error("full embedding dimension overflow"))?;
    let working_embedding = if options.nuisance == NuisanceMode::FixedOffset && controls > 0 {
        fe_parameters + 1
    } else {
        full_embedding
    };
    let memory = exact_peak_forecast(problem, full_embedding, working_embedding, options)?;
    if memory.peak > options.memory_limit_bytes {
        return Err(BackendError::new(
            ErrorCode::ResourceLimit,
            "exact_estimator",
            format!(
                "exact-estimator peak forecast {} bytes exceeds the declared limit {} bytes",
                memory.peak, options.memory_limit_bytes
            ),
        ));
    }

    let semantic_order = control_semantic_order(problem, interrupt)?;
    let canonical = canonicalize_controls_in_order_with_interrupt(
        &problem.controls,
        &problem.frequency,
        &semantic_order,
        options.rank_tolerance,
        interrupt,
    )?;
    drop(semantic_order);
    let control_receipt = canonical.receipt.clone();
    let full_design = build_design(problem, Some(&canonical.columns), interrupt)?;
    let full_information = crossproduct(
        &full_design,
        rows,
        full_embedding,
        &problem.frequency,
        interrupt,
        "exact_full_information",
    )?;
    let full_inverse = invert_scaled_zero_sum_quotient(
        &full_information,
        full_embedding,
        workers..(workers + firms),
        options.rank_tolerance,
        interrupt,
        "exact_full_inverse",
    )?;
    if controls > 0 {
        enforce_downstream_bound(
            control_receipt.forward_error,
            full_inverse.rcond,
            "full-design conditioning cannot certify canonical control-basis invariance",
        )?;
    }
    let full_rhs = weighted_transpose_vector(
        &full_design,
        rows,
        full_embedding,
        &problem.frequency,
        &problem.outcome,
        interrupt,
        "exact_full_rhs",
    )?;
    let mut full_beta = matvec(
        &full_inverse.inverse,
        full_embedding,
        &full_rhs,
        interrupt,
        "exact_full_beta_matvec",
    )?;
    center_firm_coordinates(&mut full_beta, workers, firms)?;
    let full_fit = complete_fit_residual(
        problem,
        &canonical.columns,
        &problem.outcome,
        &full_beta[..workers],
        &full_beta[workers..workers + firms],
        &full_beta[workers + firms..],
        interrupt,
        "exact_full_fit_residual",
    )?;
    let fit_tolerance = (1.0e-11_f64).max(10.0 * options.solver_tolerance);
    enforce_fit_residual(full_fit, fit_tolerance, "full weighted fit")?;

    let full_rcond = full_inverse.rcond;
    let full_inverse_relres = full_inverse.relres;
    let full_inverse_original_relres = full_inverse.original_relres;
    let fixedoffset = options.nuisance == NuisanceMode::FixedOffset && controls > 0;
    let (design, information, working_outcome, working_inverse, mut beta, parameters, embedding) =
        if fixedoffset {
            let mut working = problem.outcome.clone();
            for row in 0..rows {
                checkpoint_chunk(interrupt, row, "exact_fixedoffset_outcome")?;
                let mut offset = 0.0;
                for control in 0..controls {
                    offset +=
                        canonical.columns[control][row] * full_beta[workers + firms + control];
                }
                working[row] -= offset;
            }
            drop(full_rhs);
            drop(full_beta);
            drop(full_inverse);
            drop(full_information);
            drop(full_design);
            let design = build_design(problem, None, interrupt)?;
            let information = crossproduct(
                &design,
                rows,
                working_embedding,
                &problem.frequency,
                interrupt,
                "exact_working_information",
            )?;
            let inverse = invert_scaled_zero_sum_quotient(
                &information,
                working_embedding,
                workers..(workers + firms),
                options.rank_tolerance,
                interrupt,
                "exact_working_inverse",
            )?;
            let rhs = weighted_transpose_vector(
                &design,
                rows,
                working_embedding,
                &problem.frequency,
                &working,
                interrupt,
                "exact_working_rhs",
            )?;
            let beta = matvec(
                &inverse.inverse,
                working_embedding,
                &rhs,
                interrupt,
                "exact_working_beta_matvec",
            )?;
            (
                design,
                information,
                working,
                inverse,
                beta,
                fe_parameters,
                working_embedding,
            )
        } else {
            drop(full_rhs);
            (
                full_design,
                full_information,
                problem.outcome.clone(),
                full_inverse,
                full_beta,
                full_parameters,
                full_embedding,
            )
        };
    center_firm_coordinates(&mut beta, workers, firms)?;
    let mut residual = Vec::with_capacity(rows);
    for row in 0..rows {
        checkpoint_chunk(interrupt, row, "exact_working_residual")?;
        let fit = row_dot(
            &design,
            row,
            embedding,
            &beta,
            interrupt,
            "exact_working_fit_matvec",
        )?;
        let value = working_outcome[row] - fit;
        if !value.is_finite() {
            return Err(BackendError::new(
                ErrorCode::CorrectionNonFinite,
                "exact_estimator",
                "working least-squares residual is nonfinite",
            ));
        }
        residual.push(value);
    }
    let working_fit = if fixedoffset {
        complete_fit_residual(
            problem,
            &[],
            &working_outcome,
            &beta[..workers],
            &beta[workers..workers + firms],
            &[],
            interrupt,
            "exact_working_fit_residual",
        )?
    } else {
        complete_fit_residual(
            problem,
            &canonical.columns,
            &working_outcome,
            &beta[..workers],
            &beta[workers..workers + firms],
            &beta[workers + firms..],
            interrupt,
            "exact_working_joint_fit_residual",
        )?
    };
    enforce_fit_residual(working_fit, fit_tolerance, "working weighted fit")?;
    drop(canonical);

    let firm_effect = &beta[workers..workers + firms];
    let plugin =
        plugin_components_with_interrupt(problem, &beta[..workers], firm_effect, interrupt)?;

    let design_inverse = multiply(
        &design,
        rows,
        embedding,
        &working_inverse.inverse,
        embedding,
        interrupt,
        "exact_design_inverse",
    )?;
    let target = TargetMoments::build(problem, interrupt)?;
    let mut correction = VarianceComponents::default();
    let mut max_leverage = 0.0_f64;
    let mut maker_relres = 0.0_f64;
    let mut deletion_rank_gap = f64::INFINITY;
    let inverse_forward =
        inverse_forward_error(working_inverse.relres, working_inverse.rcond, embedding)
            .ok_or_else(|| {
                BackendError::new(
                    ErrorCode::InverseResidualFailed,
                    "exact_estimator",
                    "working inverse has no fail-closed forward-error bound",
                )
            })?;
    if inverse_forward >= 0.01 {
        return Err(BackendError::new(
            ErrorCode::InverseResidualFailed,
            "exact_estimator",
            "working inverse is too ill-conditioned for deletion-rank certification",
        ));
    }
    let rank_verification_margin = options.block_tolerance.max(10.0 * inverse_forward);
    let (inverse_factor, inverse_sqrt_relres) = if options.deletion == DeletionMode::Match {
        let factor = cholesky_factor(
            &working_inverse.inverse,
            embedding,
            interrupt,
            "exact_inverse_factor",
        )?;
        let relres =
            factor_product_residual(&factor, &working_inverse.inverse, embedding, interrupt)?;
        if !relres.is_finite()
            || relres
                > 100.0 * options.rank_tolerance * (1.0 + frobenius_norm(&working_inverse.inverse))
        {
            return Err(BackendError::new(
                ErrorCode::InverseResidualFailed,
                "exact_estimator",
                "working inverse square root failed its residual gate",
            ));
        }
        (Some(factor), relres)
    } else {
        (None, 0.0)
    };

    let deletion_units = match options.deletion {
        DeletionMode::Observation => {
            for row in 0..rows {
                checkpoint_chunk(interrupt, row, "exact_observation_correction")?;
                let z = &design_inverse[row * embedding..(row + 1) * embedding];
                let leverage = row_dot(
                    &design,
                    row,
                    embedding,
                    z,
                    interrupt,
                    "exact_observation_leverage_matvec",
                )?;
                if !leverage.is_finite() || leverage < -100.0 * options.rank_tolerance {
                    return Err(nonestimable("physical observation leverage is invalid"));
                }
                let maker = 1.0 - leverage;
                if maker <= options.block_tolerance {
                    return Err(nonestimable(
                        "physical observation deletion has leverage at or above one",
                    ));
                }
                deletion_rank_gap = deletion_rank_gap.min(maker);
                if controls > 0 && options.nuisance == NuisanceMode::Joint {
                    enforce_downstream_bound(
                        control_receipt.forward_error,
                        maker,
                        "observation-deletion conditioning cannot certify control-basis invariance",
                    )?;
                }
                if maker <= rank_verification_margin
                    || (controls > 0 && options.nuisance == NuisanceMode::Joint)
                {
                    certify_deleted_information(
                        &information,
                        &design[row * embedding..(row + 1) * embedding],
                        1,
                        embedding,
                        workers..(workers + firms),
                        options.rank_tolerance,
                        interrupt,
                        "exact_observation_deleted_information",
                    )?;
                }
                max_leverage = max_leverage.max(leverage);
                let scale =
                    u64_to_f64(problem.frequency[row])? * working_outcome[row] * residual[row]
                        / maker;
                add_scaled(
                    &mut correction,
                    target.bilinear(
                        z,
                        z,
                        interrupt,
                        "exact_observation_target_bilinear",
                        "exact_observation_target_bilinear_cells",
                    )?,
                    scale,
                );
            }
            problem.physical_total
        }
        DeletionMode::Match => {
            let groups = hybrid.map_or(problem.deletion_units(), |plan| plan.mover_deletion_units);
            for group in 0..groups {
                interrupt.checkpoint("exact_match_block")?;
                let range = problem.deletion_index.range(group);
                if range.len() > options.blocksize_limit {
                    return Err(BackendError::new(
                        ErrorCode::ResourceLimit,
                        "exact_estimator",
                        "a deletion block exceeds blocksize_limit()",
                    ));
                }
                let indices = problem.deletion_index.items[range]
                    .iter()
                    .map(|&row| usize::try_from(row).expect("validated deletion row"))
                    .collect::<Vec<_>>();
                let block = match_block_action(
                    &design,
                    &working_inverse,
                    inverse_factor.as_ref().expect("match inverse factor"),
                    &information,
                    &working_outcome,
                    &residual,
                    &problem.frequency,
                    embedding,
                    workers..(workers + firms),
                    &indices,
                    options,
                    rank_verification_margin,
                    controls > 0 && options.nuisance == NuisanceMode::Joint,
                    control_receipt.forward_error,
                    interrupt,
                )?;
                max_leverage = max_leverage.max(block.max_leverage);
                maker_relres = maker_relres.max(block.relres);
                deletion_rank_gap = deletion_rank_gap.min(1.0 - block.max_leverage);
                let left = transpose_matvec(
                    &block.block_inverse,
                    indices.len(),
                    embedding,
                    &block.transformed_outcome,
                    interrupt,
                    "exact_match_target_left_transpose",
                )?;
                let right = transpose_matvec(
                    &block.block_inverse,
                    indices.len(),
                    embedding,
                    &block.deleted_residual,
                    interrupt,
                    "exact_match_target_right_transpose",
                )?;
                add_scaled(
                    &mut correction,
                    target.bilinear(
                        &left,
                        &right,
                        interrupt,
                        "exact_match_target_bilinear",
                        "exact_match_target_bilinear_cells",
                    )?,
                    1.0,
                );
            }
            u64::try_from(groups).map_err(|_| resource_error("deletion-unit count overflow"))?
        }
    };
    let correction_sources = if let Some(plan) = hybrid {
        let mut mover_correction = correction;
        mover_correction.total =
            mover_correction.worker + mover_correction.firm + 2.0 * mover_correction.covariance;
        mover_correction.verify_accounting(1.0e-9)?;
        let mut stayer_correction = VarianceComponents::default();
        let mut stayer_physical_units = 0_u64;
        for row in 0..rows {
            checkpoint_chunk(interrupt, row, "exact_stayer_observation_correction")?;
            if !plan.stayer_rows[row] {
                continue;
            }
            let z = &design_inverse[row * embedding..(row + 1) * embedding];
            let leverage = row_dot(
                &design,
                row,
                embedding,
                z,
                interrupt,
                "exact_stayer_leverage_matvec",
            )?;
            if !leverage.is_finite() || leverage < -100.0 * options.rank_tolerance {
                return Err(nonestimable(
                    "stayer physical-observation leverage is invalid",
                ));
            }
            let maker = 1.0 - leverage;
            if maker <= options.block_tolerance {
                return Err(nonestimable(
                    "a stayer physical-observation deletion has leverage at or above one",
                ));
            }
            deletion_rank_gap = deletion_rank_gap.min(maker);
            if controls > 0 && options.nuisance == NuisanceMode::Joint {
                enforce_downstream_bound(
                    control_receipt.forward_error,
                    maker,
                    "stayer-observation conditioning cannot certify control-basis invariance",
                )?;
            }
            if maker <= rank_verification_margin
                || (controls > 0 && options.nuisance == NuisanceMode::Joint)
            {
                certify_deleted_information(
                    &information,
                    &design[row * embedding..(row + 1) * embedding],
                    1,
                    embedding,
                    workers..(workers + firms),
                    options.rank_tolerance,
                    interrupt,
                    "exact_stayer_deleted_information",
                )?;
            }
            max_leverage = max_leverage.max(leverage);
            let frequency = problem.frequency[row];
            stayer_physical_units = stayer_physical_units
                .checked_add(frequency)
                .ok_or_else(|| resource_error("stayer physical deletion count overflow"))?;
            let scale = u64_to_f64(frequency)? * working_outcome[row] * residual[row] / maker;
            add_scaled(
                &mut stayer_correction,
                target.bilinear(
                    z,
                    z,
                    interrupt,
                    "exact_stayer_target_bilinear",
                    "exact_stayer_target_bilinear_cells",
                )?,
                scale,
            );
        }
        stayer_correction.total =
            stayer_correction.worker + stayer_correction.firm + 2.0 * stayer_correction.covariance;
        stayer_correction.verify_accounting(1.0e-9)?;
        correction = add_components(mover_correction, stayer_correction)?;
        let mover_units = u64::try_from(plan.mover_deletion_units)
            .map_err(|_| resource_error("mover deletion-unit count overflow"))?;
        let combined_units = mover_units
            .checked_add(stayer_physical_units)
            .ok_or_else(|| resource_error("hybrid deletion-unit count overflow"))?;
        Some(([mover_correction, stayer_correction], combined_units))
    } else {
        None
    };
    let deletion_units = correction_sources
        .as_ref()
        .map_or(deletion_units, |(_, combined_units)| *combined_units);
    correction.total = correction.worker + correction.firm + 2.0 * correction.covariance;
    correction.verify_accounting(1.0e-9)?;
    let corrected = subtract(plugin, correction)?;
    let mut weighted_rss = 0.0;
    for (row, &value) in residual.iter().enumerate() {
        checkpoint_chunk(interrupt, row, "exact_weighted_rss")?;
        weighted_rss += u64_to_f64(problem.frequency[row])? * value * value;
    }
    if !weighted_rss.is_finite() {
        return Err(BackendError::new(
            ErrorCode::CorrectionNonFinite,
            "exact_estimator",
            "weighted residual sum of squares is nonfinite",
        ));
    }

    interrupt.checkpoint("exact_estimator_final")?;
    let firm_zero_sum_residual = beta[workers..workers + firms].iter().sum::<f64>().abs();
    let result = ExactEstimatorResult {
        plugin,
        correction,
        corrected,
        weighted_rss,
        receipt: ExactEstimatorReceipt {
            parameters,
            full_parameters,
            correction_parameters: parameters,
            deletion_units,
            max_leverage,
            information_rcond: full_rcond.min(working_inverse.rcond),
            inverse_relres: full_inverse_relres.max(working_inverse.relres),
            inverse_original_relres: full_inverse_original_relres
                .max(working_inverse.original_relres),
            inverse_sqrt_relres,
            maker_relres,
            full_fit_relres: full_fit.relative_norm,
            working_fit_relres: working_fit.relative_norm,
            fit_residual_tolerance: fit_tolerance,
            control_basis_relres: control_receipt.relres,
            control_basis_forward_error: control_receipt.forward_error,
            deletion_rank_gap,
            firm_zero_sum_residual,
            peak_forecast_bytes: memory.peak,
            fit_peak_forecast_bytes: memory.fit_peak,
            correction_peak_forecast_bytes: memory.correction_peak,
            topology_checksum: problem.topology_checksum,
        },
    };
    Ok((result, correction_sources.map(|(sources, _)| sources)))
}

fn exact_peak_forecast(
    problem: &CompressedProblem,
    full_embedding: usize,
    working_embedding: usize,
    options: ExactEstimatorOptions,
) -> Result<MemoryForecast> {
    let rows = u64::try_from(problem.outcome.len())
        .map_err(|_| resource_error("stored-row count is not representable"))?;
    let controls = u64::try_from(problem.controls.len())
        .map_err(|_| resource_error("control count is not representable"))?;
    let full = u64::try_from(full_embedding)
        .map_err(|_| resource_error("full embedding is not representable"))?;
    let working = u64::try_from(working_embedding)
        .map_err(|_| resource_error("working embedding is not representable"))?;
    let f64_bytes = |count: u64, message: &'static str| {
        count.checked_mul(8).ok_or_else(|| resource_error(message))
    };
    let rq = f64_bytes(
        rows.checked_mul(controls)
            .ok_or_else(|| resource_error("control row-product forecast overflow"))?,
        "control row-product byte forecast overflow",
    )?;
    let q_square = f64_bytes(
        controls
            .checked_mul(controls)
            .ok_or_else(|| resource_error("control-square forecast overflow"))?,
        "control-square byte forecast overflow",
    )?;
    let row_usize_bytes = rows
        .checked_mul(core::mem::size_of::<usize>() as u64)
        .ok_or_else(|| resource_error("semantic-order usize forecast overflow"))?;
    let row_u64_bytes = rows
        .checked_mul(core::mem::size_of::<u64>() as u64)
        .ok_or_else(|| resource_error("semantic-order u64 forecast overflow"))?;
    let row_bool_bytes = rows
        .checked_mul(core::mem::size_of::<bool>() as u64)
        .ok_or_else(|| resource_error("semantic-order bool forecast overflow"))?;
    // Semantic reordering and canonical_controls can simultaneously hold the
    // caller-aligned and ordered column sets plus row-major span maps. Stable
    // sorting owns a second usize row buffer; permutation validation owns its
    // bool mask and ordered frequency copy.
    let control_phase = checked_add_many(&[
        options.prepared_persistent_bytes,
        checked_scale(rq, 8, "canonical-control row-work overflow")?,
        checked_scale(q_square, 10, "canonical-control square-work overflow")?,
        checked_scale(row_usize_bytes, 2, "semantic-order sort forecast overflow")?,
        row_u64_bytes,
        row_bool_bytes,
        64 * 1024,
    ])?;
    let fit_for = |dimension: u64| -> Result<u64> {
        let row_parameter = f64_bytes(
            rows.checked_mul(dimension)
                .ok_or_else(|| resource_error("fit row-parameter forecast overflow"))?,
            "fit row-parameter byte forecast overflow",
        )?;
        let square = f64_bytes(
            dimension
                .checked_mul(dimension)
                .ok_or_else(|| resource_error("fit square forecast overflow"))?,
            "fit square byte forecast overflow",
        )?;
        checked_add_many(&[
            options.prepared_persistent_bytes,
            rq,
            row_parameter,
            // Unshifted information plus quotient augmentation, scaled
            // factor, and inverse coexist at the inverse peak.
            checked_scale(square, 4, "fit inverse phase overflow")?,
            f64_bytes(rows, "fit outcome forecast overflow")?,
            f64_bytes(
                dimension
                    .checked_mul(5)
                    .ok_or_else(|| resource_error("fit vector forecast overflow"))?,
                "fit vector byte forecast overflow",
            )?,
            64 * 1024,
        ])
    };
    let fit_peak = fit_for(full)?.max(fit_for(working)?).max(control_phase);
    let maximum_block = match options.deletion {
        DeletionMode::Observation => 1_u64,
        DeletionMode::Match => {
            let mut maximum = 0_usize;
            for group in 0..problem.deletion_units() {
                maximum = maximum.max(problem.deletion_index.range(group).len());
            }
            u64::try_from(maximum)
                .map_err(|_| resource_error("maximum block size is not representable"))?
        }
    };
    let row_parameter = rows
        .checked_mul(working)
        .and_then(|value| value.checked_mul(8))
        .ok_or_else(|| resource_error("correction row-parameter forecast overflow"))?;
    let square = working
        .checked_mul(working)
        .and_then(|value| value.checked_mul(8))
        .ok_or_else(|| resource_error("correction square forecast overflow"))?;
    let block_parameter = maximum_block
        .checked_mul(working)
        .and_then(|value| value.checked_mul(8))
        .ok_or_else(|| resource_error("exact block forecast overflow"))?;
    let block_square = maximum_block
        .min(working)
        .checked_mul(maximum_block.min(working))
        .and_then(|value| value.checked_mul(8))
        .ok_or_else(|| resource_error("exact block-square forecast overflow"))?;
    let row_work = checked_scale(row_parameter, 2, "exact row-work forecast overflow")?;
    // Working information/inverse/inverse-square-root coexist with a direct
    // deleted copy and that copy's augmented/scaled/inverse certification.
    let square_work = checked_scale(square, 7, "exact square-work forecast overflow")?;
    let block_work = checked_scale(block_parameter, 3, "exact block-work forecast overflow")?;
    let block_square_work =
        checked_scale(block_square, 3, "exact block-square-work forecast overflow")?;
    // Both maker branches retain transformed outcome/residual/action vectors;
    // B>P additionally holds the Woodbury addition/residual copies, while
    // B<=P holds dense-inverse basis/solve vectors. Eight B-vectors and four
    // P-vectors cover both lifetimes, including the two post-action P targets.
    let block_vector_work = f64_bytes(
        maximum_block
            .checked_mul(8)
            .and_then(|value| working.checked_mul(4).and_then(|p| value.checked_add(p)))
            .ok_or_else(|| resource_error("maker vector-work forecast overflow"))?,
        "maker vector-work byte forecast overflow",
    )?;
    let block_index_work = maximum_block
        .checked_mul(core::mem::size_of::<usize>() as u64)
        .ok_or_else(|| resource_error("maker index forecast overflow"))?;
    let correction_peak = checked_add_many(&[
        options.prepared_persistent_bytes,
        row_work,
        square_work,
        block_work,
        block_square_work,
        block_vector_work,
        block_index_work,
        f64_bytes(
            rows.checked_mul(4)
                .ok_or_else(|| resource_error("correction row-vector forecast overflow"))?,
            "correction row-vector byte forecast overflow",
        )?,
        f64_bytes(
            u64::try_from(problem.workers() + problem.firms() + problem.cells())
                .map_err(|_| resource_error("target-moment shape overflow"))?,
            "target-moment byte forecast overflow",
        )?,
        64 * 1024,
    ])?;
    Ok(MemoryForecast {
        peak: fit_peak.max(correction_peak),
        fit_peak,
        correction_peak,
    })
}

fn checked_scale(value: u64, factor: u64, message: &'static str) -> Result<u64> {
    value
        .checked_mul(factor)
        .ok_or_else(|| resource_error(message))
}

fn checked_add_many(values: &[u64]) -> Result<u64> {
    values.iter().try_fold(0_u64, |total, &value| {
        total
            .checked_add(value)
            .ok_or_else(|| resource_error("exact phase-lifetime forecast overflow"))
    })
}

#[derive(Clone, Debug)]
struct MatchBlockResult {
    block_inverse: Vec<f64>,
    transformed_outcome: Vec<f64>,
    deleted_residual: Vec<f64>,
    max_leverage: f64,
    relres: f64,
}

fn match_block_action(
    design: &[f64],
    inverse: &DenseInverse,
    inverse_factor: &[f64],
    information: &[f64],
    outcome: &[f64],
    residual: &[f64],
    frequency: &[u64],
    parameters: usize,
    firm_range: core::ops::Range<usize>,
    indices: &[usize],
    options: ExactEstimatorOptions,
    rank_verification_margin: f64,
    controlled_joint: bool,
    control_forward_error: f64,
    interrupt: &mut dyn InterruptCheck,
) -> Result<MatchBlockResult> {
    let width = indices.len();
    let mut block_design = vec![0.0; width * parameters];
    let mut transformed_outcome = vec![0.0; width];
    let mut transformed_residual = vec![0.0; width];
    for (local, &row) in indices.iter().enumerate() {
        checkpoint_chunk(interrupt, local, "exact_match_copy")?;
        let root = u64_to_f64(frequency[row])?.sqrt();
        transformed_outcome[local] = root * outcome[row];
        transformed_residual[local] = root * residual[row];
        for column in 0..parameters {
            block_design[local * parameters + column] = root * design[row * parameters + column];
        }
    }
    let block_inverse = multiply(
        &block_design,
        width,
        parameters,
        &inverse.inverse,
        parameters,
        interrupt,
        "exact_match_inverse",
    )?;
    let low_rank = multiply(
        &block_design,
        width,
        parameters,
        inverse_factor,
        parameters,
        interrupt,
        "exact_match_factor",
    )?;
    let (deleted_residual, relres, minimum_maker) = if width <= parameters {
        let mut maker = vec![0.0; width * width];
        let mut assembly_work = 0_usize;
        for row in 0..width {
            for column in 0..width {
                let mut value = usize::from(row == column) as f64;
                for inner in 0..parameters {
                    checkpoint_chunk(interrupt, assembly_work, "exact_match_maker_assembly")?;
                    assembly_work = assembly_work.saturating_add(1);
                    value -=
                        low_rank[row * parameters + inner] * low_rank[column * parameters + inner];
                }
                maker[row * width + column] = value;
            }
        }
        let extremes =
            symmetric_eigen_extremes(&maker, width, interrupt, "exact_match_maker_spectrum")?;
        if !extremes.largest_upper.is_finite() || extremes.smallest_lower <= options.block_tolerance
        {
            return Err(nonestimable("a match residual block is singular"));
        }
        let maker_inverse = invert_scaled_spd(
            &maker,
            width,
            options.rank_tolerance,
            interrupt,
            "exact_match_maker_inverse",
        )
        .map_err(|error| {
            preserve_user_break_or(
                error,
                ErrorCode::BlockInverseFailed,
                "a match residual solve failed",
            )
        })?;
        let solved = matvec(
            &maker_inverse.inverse,
            width,
            &transformed_residual,
            interrupt,
            "exact_match_maker_matvec",
        )?;
        let relres = relative_residual(
            &maker,
            width,
            &solved,
            &transformed_residual,
            interrupt,
            "exact_match_maker_residual",
        )?
        .max(maker_inverse.relres);
        (solved, relres, extremes.smallest_lower)
    } else {
        let mut reduced = vec![0.0; parameters * parameters];
        let mut assembly_work = 0_usize;
        for row in 0..parameters {
            for column in 0..parameters {
                let mut value = usize::from(row == column) as f64;
                for inner in 0..width {
                    checkpoint_chunk(interrupt, assembly_work, "exact_match_reduced_assembly")?;
                    assembly_work = assembly_work.saturating_add(1);
                    value -=
                        low_rank[inner * parameters + row] * low_rank[inner * parameters + column];
                }
                reduced[row * parameters + column] = value;
            }
        }
        let extremes = symmetric_eigen_extremes(
            &reduced,
            parameters,
            interrupt,
            "exact_match_reduced_spectrum",
        )?;
        if !extremes.largest_upper.is_finite() || extremes.smallest_lower <= options.block_tolerance
        {
            return Err(nonestimable("a reduced match residual block is singular"));
        }
        let reduced_inverse = invert_scaled_spd(
            &reduced,
            parameters,
            options.rank_tolerance,
            interrupt,
            "exact_match_reduced_inverse",
        )
        .map_err(|error| {
            preserve_user_break_or(
                error,
                ErrorCode::BlockInverseFailed,
                "a reduced match residual solve failed",
            )
        })?;
        let projected = transpose_matvec(
            &low_rank,
            width,
            parameters,
            &transformed_residual,
            interrupt,
            "exact_match_reduced_transpose",
        )?;
        let solved = matvec(
            &reduced_inverse.inverse,
            parameters,
            &projected,
            interrupt,
            "exact_match_reduced_matvec",
        )?;
        let addition = matvec_rect(
            &low_rank,
            width,
            parameters,
            &solved,
            interrupt,
            "exact_match_addition_matvec",
        )?;
        let mut deleted = Vec::with_capacity(width);
        for (row, (&left, right)) in transformed_residual.iter().zip(addition).enumerate() {
            checkpoint_chunk(interrupt, row, "exact_match_deleted_residual")?;
            deleted.push(left + right);
        }
        let action = low_rank_action(
            &low_rank,
            width,
            parameters,
            &deleted,
            interrupt,
            "exact_match_low_rank_action",
        )?;
        let mut maker_residual = Vec::with_capacity(width);
        for row in 0..width {
            checkpoint_chunk(interrupt, row, "exact_match_complete_residual")?;
            maker_residual.push(deleted[row] - action[row] - transformed_residual[row]);
        }
        let relres = vector_relative_norm(
            &maker_residual,
            &transformed_residual,
            interrupt,
            "exact_match_complete_residual_norm",
        )?
        .max(reduced_inverse.relres);
        (deleted, relres, extremes.smallest_lower.min(1.0))
    };
    if !relres.is_finite() || relres > (100.0 * options.rank_tolerance).max(1.0e-10) {
        return Err(BackendError::new(
            ErrorCode::BlockInverseFailed,
            "exact_estimator",
            "a match residual solve failed its complete residual gate",
        ));
    }
    let max_leverage = 1.0 - minimum_maker;
    if !max_leverage.is_finite() || minimum_maker <= options.block_tolerance {
        return Err(nonestimable(
            "a match deletion loses identified design rank",
        ));
    }
    if controlled_joint {
        enforce_downstream_bound(
            control_forward_error,
            minimum_maker,
            "match-deletion conditioning cannot certify control-basis invariance",
        )?;
    }
    if minimum_maker <= rank_verification_margin || controlled_joint {
        certify_deleted_information(
            information,
            &block_design,
            width,
            parameters,
            firm_range,
            options.rank_tolerance,
            interrupt,
            "exact_match_deleted_information",
        )?;
    }
    Ok(MatchBlockResult {
        block_inverse,
        transformed_outcome,
        deleted_residual,
        max_leverage,
        relres,
    })
}

#[derive(Clone, Debug)]
struct TargetMoments<'a> {
    problem: &'a CompressedProblem,
    worker_share: Vec<f64>,
    firm_share: Vec<f64>,
}

impl<'a> TargetMoments<'a> {
    fn build(problem: &'a CompressedProblem, interrupt: &mut dyn InterruptCheck) -> Result<Self> {
        let workers = problem.workers();
        let mut worker_share = vec![0.0; workers];
        let mut firm_share = vec![0.0; problem.firms()];
        for row in 0..problem.outcome.len() {
            checkpoint_chunk(interrupt, row, "exact_target_moments")?;
            let mass = problem.target_weight[row] / problem.target_total;
            let worker = usize::try_from(problem.row_worker[row]).expect("dense worker");
            let firm = usize::try_from(problem.row_firm[row]).expect("dense firm");
            worker_share[worker] += mass;
            firm_share[firm] += mass;
        }
        Ok(Self {
            problem,
            worker_share,
            firm_share,
        })
    }

    fn bilinear(
        &self,
        left: &[f64],
        right: &[f64],
        interrupt: &mut dyn InterruptCheck,
        phase: &'static str,
        cell_phase: &'static str,
    ) -> Result<VarianceComponents> {
        let workers = self.problem.workers();
        let firms = self.problem.firms();
        let left_worker = &left[..workers];
        let right_worker = &right[..workers];
        let left_firm = &left[workers..workers + firms];
        let right_firm = &right[workers..workers + firms];
        let left_worker_mean = dot(&self.worker_share, left_worker, interrupt, phase)?;
        let right_worker_mean = dot(&self.worker_share, right_worker, interrupt, phase)?;
        let left_firm_mean = dot(&self.firm_share, left_firm, interrupt, phase)?;
        let right_firm_mean = dot(&self.firm_share, right_firm, interrupt, phase)?;
        let worker = weighted_pair(
            &self.worker_share,
            left_worker,
            right_worker,
            interrupt,
            phase,
        )? - left_worker_mean * right_worker_mean;
        let firm = weighted_pair(&self.firm_share, left_firm, right_firm, interrupt, phase)?
            - left_firm_mean * right_firm_mean;
        let mut left_worker_right_firm = 0.0;
        let mut right_worker_left_firm = 0.0;
        let mut cell_work = 0_usize;
        for cell in 0..self.problem.cells() {
            let worker_index =
                usize::try_from(self.problem.cell_worker[cell]).expect("dense worker");
            let firm_index = usize::try_from(self.problem.cell_firm[cell]).expect("dense firm");
            let mass = self.problem.cell_target_sum[cell] / self.problem.target_total;
            checkpoint_chunk(interrupt, cell_work, cell_phase)?;
            cell_work = cell_work.saturating_add(1);
            left_worker_right_firm += mass * left_worker[worker_index] * right_firm[firm_index];
            checkpoint_chunk(interrupt, cell_work, cell_phase)?;
            cell_work = cell_work.saturating_add(1);
            right_worker_left_firm += mass * right_worker[worker_index] * left_firm[firm_index];
        }
        let covariance = 0.5
            * (left_worker_right_firm + right_worker_left_firm
                - left_worker_mean * right_firm_mean
                - right_worker_mean * left_firm_mean);
        Ok(VarianceComponents {
            worker,
            firm,
            covariance,
            total: worker + firm + 2.0 * covariance,
        })
    }
}

fn control_semantic_order(
    problem: &CompressedProblem,
    interrupt: &mut dyn InterruptCheck,
) -> Result<Vec<usize>> {
    let mut order = (0..problem.outcome.len()).collect::<Vec<_>>();
    stable_sort_by_with_interrupt(
        &mut order,
        |&left, &right| {
            problem.row_worker[left]
                .cmp(&problem.row_worker[right])
                .then_with(|| problem.row_firm[left].cmp(&problem.row_firm[right]))
                .then_with(|| problem.row_deletion[left].cmp(&problem.row_deletion[right]))
                .then_with(|| problem.row_target[left].cmp(&problem.row_target[right]))
                .then_with(|| problem.frequency[left].cmp(&problem.frequency[right]))
                .then_with(|| problem.target_weight[left].total_cmp(&problem.target_weight[right]))
                .then_with(|| problem.outcome[left].total_cmp(&problem.outcome[right]))
        },
        interrupt,
        "exact_control_semantic_order",
    )?;
    Ok(order)
}

fn build_design(
    problem: &CompressedProblem,
    controls: Option<&[Vec<f64>]>,
    interrupt: &mut dyn InterruptCheck,
) -> Result<Vec<f64>> {
    let rows = problem.outcome.len();
    let workers = problem.workers();
    let firms = problem.firms();
    let control_count = controls.map_or(0, <[Vec<f64>]>::len);
    let parameters = workers + firms + control_count;
    let entries = rows
        .checked_mul(parameters)
        .ok_or_else(|| resource_error("dense design size overflow"))?;
    let mut design = vec![0.0; entries];
    for row in 0..rows {
        checkpoint_chunk(interrupt, row, "exact_design")?;
        let worker = usize::try_from(problem.row_worker[row]).expect("dense worker");
        let firm = usize::try_from(problem.row_firm[row]).expect("dense firm");
        design[row * parameters + worker] = 1.0;
        let common = -(firms as f64).recip();
        for firm_column in 0..firms {
            design[row * parameters + workers + firm_column] =
                common + f64::from(firm_column == firm);
        }
        if let Some(controls) = controls {
            for (control, column) in controls.iter().enumerate() {
                design[row * parameters + workers + firms + control] = column[row];
            }
        }
    }
    Ok(design)
}

fn center_firm_coordinates(beta: &mut [f64], workers: usize, firms: usize) -> Result<()> {
    if beta.len() < workers + firms || firms == 0 {
        return Err(BackendError::invalid(
            "exact_estimator",
            "firm quotient coefficient shape is invalid",
        ));
    }
    let mean = beta[workers..workers + firms].iter().sum::<f64>() / firms as f64;
    for value in &mut beta[workers..workers + firms] {
        *value -= mean;
    }
    if beta.iter().any(|value| !value.is_finite()) {
        return Err(BackendError::new(
            ErrorCode::CorrectionNonFinite,
            "exact_estimator",
            "full-firm quotient reconstruction is nonfinite",
        ));
    }
    Ok(())
}

fn complete_fit_residual(
    problem: &CompressedProblem,
    controls: &[Vec<f64>],
    outcome: &[f64],
    worker_effect: &[f64],
    firm_effect: &[f64],
    control_effect: &[f64],
    interrupt: &mut dyn InterruptCheck,
    phase: &'static str,
) -> Result<FitResidualReceipt> {
    let rows = outcome.len();
    let workers = problem.workers();
    let firms = problem.firms();
    if rows != problem.outcome.len()
        || worker_effect.len() != workers
        || firm_effect.len() != firms
        || controls.len() != control_effect.len()
        || controls.iter().any(|column| column.len() != rows)
    {
        return Err(BackendError::invalid(
            "exact_estimator",
            "complete fit-residual dimensions disagree",
        ));
    }
    let dimension = workers + firms + controls.len();
    let mut rhs = vec![0.0; dimension];
    let mut normal_residual = vec![0.0; dimension];
    for row in 0..rows {
        checkpoint_chunk(interrupt, row, phase)?;
        let worker = usize::try_from(problem.row_worker[row]).expect("dense worker");
        let firm = usize::try_from(problem.row_firm[row]).expect("dense firm");
        let weight = u64_to_f64(problem.frequency[row])?;
        let mut fit = worker_effect[worker] + firm_effect[firm];
        for control in 0..controls.len() {
            fit += controls[control][row] * control_effect[control];
        }
        let weighted_outcome = weight * outcome[row];
        let weighted_residual = weight * (outcome[row] - fit);
        rhs[worker] += weighted_outcome;
        rhs[workers + firm] += weighted_outcome;
        normal_residual[worker] += weighted_residual;
        normal_residual[workers + firm] += weighted_residual;
        for control in 0..controls.len() {
            let column = workers + firms + control;
            rhs[column] += controls[control][row] * weighted_outcome;
            normal_residual[column] += controls[control][row] * weighted_residual;
        }
    }
    let absolute_norm = vector_norm(&normal_residual, interrupt, phase)?;
    let rhs_norm = vector_norm(&rhs, interrupt, phase)?;
    let relative_norm = if rhs_norm == 0.0 {
        absolute_norm
    } else {
        absolute_norm / rhs_norm
    };
    if !absolute_norm.is_finite() || !rhs_norm.is_finite() || !relative_norm.is_finite() {
        return Err(BackendError::new(
            ErrorCode::FullResidualFailed,
            "exact_estimator",
            "complete original fit residual is nonfinite",
        ));
    }
    Ok(FitResidualReceipt {
        relative_norm,
        absolute_norm,
        rhs_norm,
    })
}

fn enforce_fit_residual(
    residual: FitResidualReceipt,
    tolerance: f64,
    label: &'static str,
) -> Result<()> {
    if residual.relative_norm > tolerance {
        return Err(BackendError::new(
            ErrorCode::FullResidualFailed,
            "exact_estimator",
            format!(
                "complete original {label} normal-equation residual {} exceeds {tolerance} (absolute {}, RHS norm {})",
                residual.relative_norm, residual.absolute_norm, residual.rhs_norm
            ),
        ));
    }
    Ok(())
}

fn factor_product_residual(
    factor: &[f64],
    reference: &[f64],
    dimension: usize,
    interrupt: &mut dyn InterruptCheck,
) -> Result<f64> {
    let mut sum = 0.0;
    let mut work = 0_usize;
    for row in 0..dimension {
        for column in 0..dimension {
            let mut value = -reference[row * dimension + column];
            for inner in 0..dimension {
                work = work.saturating_add(1);
                checkpoint_chunk(interrupt, work, "exact_inverse_factor_residual")?;
                value += factor[row * dimension + inner] * factor[column * dimension + inner];
            }
            sum += value * value;
        }
    }
    Ok(sum.sqrt())
}

fn certify_deleted_information(
    information: &[f64],
    deleted_design: &[f64],
    deleted_rows: usize,
    dimension: usize,
    firm_range: core::ops::Range<usize>,
    rank_tolerance: f64,
    interrupt: &mut dyn InterruptCheck,
    phase: &'static str,
) -> Result<()> {
    if information.len() != dimension.saturating_mul(dimension)
        || deleted_design.len() != deleted_rows.saturating_mul(dimension)
    {
        return Err(BackendError::invalid(
            "exact_estimator",
            "deleted-information dimensions disagree",
        ));
    }
    let mut deleted = information.to_vec();
    for deleted_row in 0..deleted_rows {
        for row in 0..dimension {
            let left = deleted_design[deleted_row * dimension + row];
            for column in 0..dimension {
                checkpoint_chunk(
                    interrupt,
                    (deleted_row * dimension + row) * dimension + column,
                    phase,
                )?;
                deleted[row * dimension + column] -=
                    left * deleted_design[deleted_row * dimension + column];
            }
        }
    }
    invert_scaled_zero_sum_quotient(
        &deleted,
        dimension,
        firm_range,
        rank_tolerance,
        interrupt,
        phase,
    )
    .map(|_| ())
    .map_err(|error| {
        preserve_user_break_or(
            error,
            ErrorCode::NonestimableDeletion,
            "direct deleted-information factorization rejects deletion rank",
        )
    })
}

fn crossproduct(
    design: &[f64],
    rows: usize,
    parameters: usize,
    frequency: &[u64],
    interrupt: &mut dyn InterruptCheck,
    phase: &'static str,
) -> Result<Vec<f64>> {
    let mut output = vec![0.0; parameters * parameters];
    for row in 0..rows {
        let weight = u64_to_f64(frequency[row])?;
        for left in 0..parameters {
            let scaled = weight * design[row * parameters + left];
            if scaled == 0.0 {
                continue;
            }
            for right in 0..=left {
                checkpoint_chunk(
                    interrupt,
                    (row * parameters + left) * parameters + right,
                    phase,
                )?;
                output[left * parameters + right] += scaled * design[row * parameters + right];
            }
        }
    }
    for left in 0..parameters {
        for right in 0..left {
            checkpoint_chunk(interrupt, left * parameters + right, phase)?;
            output[right * parameters + left] = output[left * parameters + right];
        }
    }
    Ok(output)
}

fn weighted_transpose_vector(
    design: &[f64],
    rows: usize,
    parameters: usize,
    frequency: &[u64],
    value: &[f64],
    interrupt: &mut dyn InterruptCheck,
    phase: &'static str,
) -> Result<Vec<f64>> {
    let mut output = vec![0.0; parameters];
    for row in 0..rows {
        checkpoint_chunk(interrupt, row, phase)?;
        let scale = u64_to_f64(frequency[row])? * value[row];
        for column in 0..parameters {
            output[column] += design[row * parameters + column] * scale;
        }
    }
    Ok(output)
}

fn multiply(
    left: &[f64],
    left_rows: usize,
    inner: usize,
    right: &[f64],
    right_columns: usize,
    interrupt: &mut dyn InterruptCheck,
    phase: &'static str,
) -> Result<Vec<f64>> {
    if left.len() != left_rows * inner || right.len() != inner * right_columns {
        return Err(BackendError::invalid(
            "exact_estimator",
            "dense multiplication dimensions disagree",
        ));
    }
    let mut output = vec![0.0; left_rows * right_columns];
    for row in 0..left_rows {
        for column in 0..right_columns {
            let mut value = 0.0;
            for index in 0..inner {
                checkpoint_chunk(
                    interrupt,
                    (row * right_columns + column) * inner + index,
                    phase,
                )?;
                value += left[row * inner + index] * right[index * right_columns + column];
            }
            output[row * right_columns + column] = value;
        }
    }
    Ok(output)
}

fn matvec(
    matrix: &[f64],
    dimension: usize,
    value: &[f64],
    interrupt: &mut dyn InterruptCheck,
    phase: &'static str,
) -> Result<Vec<f64>> {
    matvec_rect(matrix, dimension, dimension, value, interrupt, phase)
}

fn matvec_rect(
    matrix: &[f64],
    rows: usize,
    columns: usize,
    value: &[f64],
    interrupt: &mut dyn InterruptCheck,
    phase: &'static str,
) -> Result<Vec<f64>> {
    if matrix.len() != rows * columns || value.len() != columns {
        return Err(BackendError::invalid(
            "exact_estimator",
            "matrix-vector dimensions disagree",
        ));
    }
    let mut output = vec![0.0; rows];
    let mut work = 0_usize;
    for row in 0..rows {
        let mut sum = 0.0;
        for column in 0..columns {
            checkpoint_chunk(interrupt, work, phase)?;
            work = work.saturating_add(1);
            sum += matrix[row * columns + column] * value[column];
        }
        output[row] = sum;
    }
    Ok(output)
}

fn transpose_matvec(
    matrix: &[f64],
    rows: usize,
    columns: usize,
    value: &[f64],
    interrupt: &mut dyn InterruptCheck,
    phase: &'static str,
) -> Result<Vec<f64>> {
    if matrix.len() != rows * columns || value.len() != rows {
        return Err(BackendError::invalid(
            "exact_estimator",
            "transposed matrix-vector dimensions disagree",
        ));
    }
    let mut output = vec![0.0; columns];
    let mut work = 0_usize;
    for row in 0..rows {
        for column in 0..columns {
            checkpoint_chunk(interrupt, work, phase)?;
            work = work.saturating_add(1);
            output[column] += matrix[row * columns + column] * value[row];
        }
    }
    Ok(output)
}

fn low_rank_action(
    factor: &[f64],
    rows: usize,
    columns: usize,
    value: &[f64],
    interrupt: &mut dyn InterruptCheck,
    phase: &'static str,
) -> Result<Vec<f64>> {
    let projected = transpose_matvec(factor, rows, columns, value, interrupt, phase)?;
    matvec_rect(factor, rows, columns, &projected, interrupt, phase)
}

fn relative_residual(
    matrix: &[f64],
    dimension: usize,
    value: &[f64],
    rhs: &[f64],
    interrupt: &mut dyn InterruptCheck,
    phase: &'static str,
) -> Result<f64> {
    let mut residual = matvec(matrix, dimension, value, interrupt, phase)?;
    for (index, (item, reference)) in residual.iter_mut().zip(rhs).enumerate() {
        checkpoint_chunk(interrupt, index, phase)?;
        *item -= reference;
    }
    vector_relative_norm(&residual, rhs, interrupt, phase)
}

fn vector_relative_norm(
    residual: &[f64],
    rhs: &[f64],
    interrupt: &mut dyn InterruptCheck,
    phase: &'static str,
) -> Result<f64> {
    let denominator = vector_norm(rhs, interrupt, phase)?;
    if denominator == 0.0 {
        vector_norm(residual, interrupt, phase)
    } else {
        Ok(vector_norm(residual, interrupt, phase)? / denominator)
    }
}

fn vector_norm(
    value: &[f64],
    interrupt: &mut dyn InterruptCheck,
    phase: &'static str,
) -> Result<f64> {
    Ok(dot(value, value, interrupt, phase)?.sqrt())
}

fn row_dot(
    matrix: &[f64],
    row: usize,
    columns: usize,
    value: &[f64],
    interrupt: &mut dyn InterruptCheck,
    phase: &'static str,
) -> Result<f64> {
    let mut sum = 0.0;
    for column in 0..columns {
        let work = row.saturating_mul(columns).saturating_add(column);
        checkpoint_chunk(interrupt, work, phase)?;
        sum += matrix[row * columns + column] * value[column];
    }
    Ok(sum)
}

fn dot(
    left: &[f64],
    right: &[f64],
    interrupt: &mut dyn InterruptCheck,
    phase: &'static str,
) -> Result<f64> {
    let mut sum = 0.0;
    for (index, (&a, &b)) in left.iter().zip(right).enumerate() {
        checkpoint_chunk(interrupt, index, phase)?;
        sum += a * b;
    }
    Ok(sum)
}

fn weighted_pair(
    weight: &[f64],
    left: &[f64],
    right: &[f64],
    interrupt: &mut dyn InterruptCheck,
    phase: &'static str,
) -> Result<f64> {
    let mut sum = 0.0;
    for (index, ((&mass, &a), &b)) in weight.iter().zip(left).zip(right).enumerate() {
        checkpoint_chunk(interrupt, index, phase)?;
        sum += mass * a * b;
    }
    Ok(sum)
}

fn add_scaled(target: &mut VarianceComponents, value: VarianceComponents, scale: f64) {
    target.worker += scale * value.worker;
    target.firm += scale * value.firm;
    target.covariance += scale * value.covariance;
}

fn add_components(
    left: VarianceComponents,
    right: VarianceComponents,
) -> Result<VarianceComponents> {
    let combined = VarianceComponents {
        worker: left.worker + right.worker,
        firm: left.firm + right.firm,
        covariance: left.covariance + right.covariance,
        total: left.total + right.total,
    };
    combined.verify_accounting(1.0e-9).map_err(|_| {
        BackendError::new(
            ErrorCode::CorrectionNonFinite,
            "exact_stayer_hybrid",
            "mixed-deletion correction is nonfinite or violates its accounting identity",
        )
    })?;
    Ok(combined)
}

fn subtract(
    plugin: VarianceComponents,
    correction: VarianceComponents,
) -> Result<VarianceComponents> {
    let corrected = VarianceComponents {
        worker: plugin.worker - correction.worker,
        firm: plugin.firm - correction.firm,
        covariance: plugin.covariance - correction.covariance,
        total: plugin.total - correction.total,
    };
    corrected.verify_accounting(1.0e-9).map_err(|_| {
        BackendError::new(
            ErrorCode::NonfiniteCorrectedTarget,
            "exact_estimator",
            "exact corrected target is nonfinite or violates its accounting identity",
        )
    })?;
    Ok(corrected)
}

fn u64_to_f64(value: u64) -> Result<f64> {
    let output = value as f64;
    if output as u64 != value {
        return Err(resource_error(
            "integer frequency is not exactly representable in binary64",
        ));
    }
    Ok(output)
}

fn resource_error(message: &str) -> BackendError {
    BackendError::new(ErrorCode::ResourceLimit, "exact_estimator", message)
}

fn nonestimable(message: &str) -> BackendError {
    BackendError::new(ErrorCode::NonestimableDeletion, "exact_estimator", message)
}

fn preserve_user_break_or(
    error: BackendError,
    code: ErrorCode,
    message: &'static str,
) -> BackendError {
    if error.code == ErrorCode::UserBreak {
        error
    } else {
        BackendError::new(code, "exact_estimator", message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::problem::CanonicalInput;
    use crate::types::InputColumns;

    struct BreakOnPhase {
        phase: &'static str,
    }

    impl InterruptCheck for BreakOnPhase {
        fn checkpoint(&mut self, phase: &'static str) -> Result<()> {
            if phase == self.phase {
                Err(BackendError::new(
                    ErrorCode::UserBreak,
                    phase,
                    "injected exact-estimator break",
                ))
            } else {
                Ok(())
            }
        }
    }

    struct BreakAfter {
        phase: &'static str,
        callbacks_before_break: usize,
        matching_callbacks: usize,
    }

    impl InterruptCheck for BreakAfter {
        fn checkpoint(&mut self, phase: &'static str) -> Result<()> {
            if phase != self.phase {
                return Ok(());
            }
            self.matching_callbacks += 1;
            if self.matching_callbacks > self.callbacks_before_break {
                Err(BackendError::new(
                    ErrorCode::UserBreak,
                    phase,
                    "injected exact-estimator delayed break",
                ))
            } else {
                Ok(())
            }
        }
    }

    fn fixture(controls: Vec<Vec<f64>>, frequency: Vec<u64>) -> CompressedProblem {
        let worker = vec![1, 1, 1, 1, 2, 2, 2, 2];
        let firm = vec![1, 1, 2, 2, 1, 1, 2, 2];
        CanonicalInput::from_validated(
            InputColumns {
                worker,
                firm,
                deletion: (1_u64..=8).collect(),
                outcome: vec![1.0, 2.0, 0.0, 2.5, -1.0, 1.5, 2.0, -2.0],
                frequency,
                target_weight: vec![1.0, 2.0, 2.0, 1.0, 3.0, 1.0, 2.0, 4.0],
                controls,
            }
            .validate()
            .expect("fixture validates"),
        )
        .expect("fixture canonicalizes")
        .compress(&[true; 8])
        .expect("fixture compresses")
    }

    fn hybrid_fixture() -> (CompressedProblem, ExactStayerHybridPlan) {
        let rows = 10_usize;
        let problem = CanonicalInput::from_validated(
            InputColumns {
                worker: vec![1, 1, 1, 1, 2, 2, 2, 2, 3, 3],
                firm: vec![1, 1, 2, 2, 1, 1, 2, 2, 1, 1],
                deletion: (1_u64..=rows as u64).collect(),
                outcome: vec![1.0, 2.0, 0.0, 2.5, -1.0, 1.5, 2.0, -2.0, 0.5, 1.25],
                frequency: vec![1, 1, 1, 1, 1, 1, 1, 1, 2, 1],
                target_weight: vec![1.0, 2.0, 2.0, 1.0, 3.0, 1.0, 2.0, 4.0, 1.5, 0.5],
                controls: Vec::new(),
            }
            .validate()
            .expect("hybrid fixture validates"),
        )
        .expect("hybrid fixture canonicalizes")
        .compress(&vec![true; rows])
        .expect("hybrid fixture compresses");
        (
            problem,
            ExactStayerHybridPlan {
                stayer_rows: vec![
                    false, false, false, false, false, false, false, false, true, true,
                ],
                mover_deletion_units: 8,
            },
        )
    }

    #[test]
    fn mixed_deletion_matches_observation_when_mover_blocks_are_singletons() {
        let (problem, plan) = hybrid_fixture();
        let hybrid = run_exact_stayer_hybrid(&problem, &plan, ExactEstimatorOptions::default())
            .expect("mixed-deletion exact result");
        let observation = run_exact_estimator(
            &problem,
            ExactEstimatorOptions {
                deletion: DeletionMode::Observation,
                ..ExactEstimatorOptions::default()
            },
        )
        .expect("observation exact result");
        for (left, right) in component_array(hybrid.estimator.plugin)
            .into_iter()
            .zip(component_array(observation.plugin))
            .chain(
                component_array(hybrid.estimator.correction)
                    .into_iter()
                    .zip(component_array(observation.correction)),
            )
        {
            assert!((left - right).abs() < 1.0e-10, "{left} versus {right}");
        }
        let mut source_sum = hybrid.mover_correction;
        add_scaled(&mut source_sum, hybrid.stayer_correction, 1.0);
        source_sum.total = source_sum.worker + source_sum.firm + 2.0 * source_sum.covariance;
        for (left, right) in component_array(source_sum)
            .into_iter()
            .zip(component_array(hybrid.estimator.correction))
        {
            assert!((left - right).abs() < 1.0e-12, "{left} versus {right}");
        }
        assert_eq!(hybrid.estimator.receipt.deletion_units, 11);
    }

    #[test]
    fn zero_stayer_partition_reproduces_the_mover_exact_result() {
        let problem = fixture(Vec::new(), vec![1; 8]);
        let mover =
            run_exact_estimator(&problem, ExactEstimatorOptions::default()).expect("mover result");
        let hybrid = run_exact_stayer_hybrid(
            &problem,
            &ExactStayerHybridPlan {
                stayer_rows: vec![false; 8],
                mover_deletion_units: 8,
            },
            ExactEstimatorOptions::default(),
        )
        .expect("zero-stayer hybrid result");
        for (left, right) in component_array(mover.plugin)
            .into_iter()
            .zip(component_array(hybrid.estimator.plugin))
            .chain(
                component_array(mover.correction)
                    .into_iter()
                    .zip(component_array(hybrid.estimator.correction)),
            )
        {
            assert!((left - right).abs() < 1.0e-12, "{left} versus {right}");
        }
        assert_eq!(hybrid.stayer_correction, VarianceComponents::default());
    }

    #[test]
    fn mixed_deletion_partition_fails_closed_on_overlap() {
        let (problem, mut plan) = hybrid_fixture();
        plan.stayer_rows[0] = true;
        let error = run_exact_stayer_hybrid(&problem, &plan, ExactEstimatorOptions::default())
            .expect_err("overlapping mover/stayer groups must fail");
        assert_eq!(error.code, ErrorCode::InvalidInput);
    }

    #[test]
    fn match_and_observation_exact_agree_for_single_copy_singleton_units() {
        let problem = fixture(Vec::new(), vec![1; 8]);
        let match_result = run_exact_estimator(&problem, ExactEstimatorOptions::default())
            .expect("match exact result");
        let observation_result = run_exact_estimator(
            &problem,
            ExactEstimatorOptions {
                deletion: DeletionMode::Observation,
                ..ExactEstimatorOptions::default()
            },
        )
        .expect("observation exact result");
        for (left, right) in [
            (match_result.plugin.worker, observation_result.plugin.worker),
            (match_result.plugin.firm, observation_result.plugin.firm),
            (
                match_result.plugin.covariance,
                observation_result.plugin.covariance,
            ),
            (
                match_result.correction.worker,
                observation_result.correction.worker,
            ),
            (
                match_result.correction.firm,
                observation_result.correction.firm,
            ),
            (
                match_result.correction.covariance,
                observation_result.correction.covariance,
            ),
        ] {
            assert!((left - right).abs() < 1.0e-10, "{left} versus {right}");
        }
        assert_eq!(match_result.receipt.deletion_units, 8);
        assert_eq!(observation_result.receipt.deletion_units, 8);
    }

    #[test]
    fn exact_controls_and_fixedoffset_produce_finite_accounting_results() {
        let problem = fixture(
            vec![vec![-1.5, -0.5, 0.5, 1.5, -1.0, 0.0, 1.0, 2.0]],
            vec![1; 8],
        );
        let expected = [
            (
                NuisanceMode::Joint,
                [
                    0.527_343_750_000_000_6,
                    0.384_521_484_374_999_9,
                    0.043_945_312_500_000_014,
                    0.999_755_859_375_000_4,
                ],
                [
                    0.602_416_992_187_498_8,
                    2.037_963_867_187_495,
                    0.068_481_445_312_499_88,
                    2.777_343_749_999_993_3,
                ],
                0.5,
            ),
            (
                NuisanceMode::FixedOffset,
                [
                    0.527_343_749_999_999_8,
                    0.384_521_484_374_999_33,
                    0.043_945_312_499_999_944,
                    0.999_755_859_374_999,
                ],
                [
                    0.333_984_375_000_000_06,
                    0.350_683_593_75,
                    0.014_648_437_499_999_896,
                    0.713_964_843_750_000_1,
                ],
                0.375,
            ),
        ];
        for (nuisance, expected_plugin, expected_correction, expected_leverage) in expected {
            let result = run_exact_estimator(
                &problem,
                ExactEstimatorOptions {
                    nuisance,
                    ..ExactEstimatorOptions::default()
                },
            )
            .expect("controlled exact result");
            result.plugin.verify_accounting(1.0e-10).expect("plugin");
            result
                .correction
                .verify_accounting(1.0e-10)
                .expect("correction");
            result
                .corrected
                .verify_accounting(1.0e-10)
                .expect("corrected");
            assert!(result.weighted_rss.is_finite());
            assert!((result.weighted_rss - 14.25).abs() < 1.0e-10);
            let plugin = [
                result.plugin.worker,
                result.plugin.firm,
                result.plugin.covariance,
                result.plugin.total,
            ];
            let correction = [
                result.correction.worker,
                result.correction.firm,
                result.correction.covariance,
                result.correction.total,
            ];
            for (actual, expected) in plugin.into_iter().zip(expected_plugin) {
                assert!(
                    (actual - expected).abs() < 1.0e-10,
                    "{actual} versus {expected}"
                );
            }
            for (actual, expected) in correction.into_iter().zip(expected_correction) {
                assert!(
                    (actual - expected).abs() < 1.0e-10,
                    "{actual} versus {expected}"
                );
            }
            assert!((result.receipt.max_leverage - expected_leverage).abs() < 1.0e-10);
            assert_eq!(result.receipt.full_parameters, 4);
            assert_eq!(
                result.receipt.correction_parameters,
                4 - usize::from(nuisance == NuisanceMode::FixedOffset)
            );
        }
    }

    #[test]
    fn exact_frequency_changes_observation_units_but_not_match_groups() {
        let problem = fixture(Vec::new(), vec![2, 1, 1, 1, 1, 1, 1, 1]);
        let match_result = run_exact_estimator(&problem, ExactEstimatorOptions::default())
            .expect("match exact result");
        let observation_result = run_exact_estimator(
            &problem,
            ExactEstimatorOptions {
                deletion: DeletionMode::Observation,
                ..ExactEstimatorOptions::default()
            },
        )
        .expect("observation exact result");
        assert_eq!(match_result.receipt.deletion_units, 8);
        assert_eq!(observation_result.receipt.deletion_units, 9);
        assert!(
            (match_result.correction.total - observation_result.correction.total).abs() > 1.0e-8
        );
    }

    fn three_firm_fixture(firm_labels: [u64; 3]) -> CompressedProblem {
        let mut worker = Vec::new();
        let mut firm = Vec::new();
        let mut outcome = Vec::new();
        let mut target_weight = Vec::new();
        let worker_effect = [1.2, -0.7, 0.4];
        let firm_effect = [0.8, -0.3, -0.5];
        for worker_index in 0..3 {
            for firm_index in 0..3 {
                for copy in 0..2 {
                    worker.push([101_u64, 303, 707][worker_index]);
                    firm.push(firm_labels[firm_index]);
                    outcome.push(
                        worker_effect[worker_index]
                            + firm_effect[firm_index]
                            + [0.25, -0.15][copy]
                            + 0.03 * (worker_index * firm_index) as f64,
                    );
                    target_weight.push(1.0 + ((worker_index + firm_index + copy) % 3) as f64);
                }
            }
        }
        let rows = outcome.len();
        CanonicalInput::from_validated(
            InputColumns {
                worker,
                firm,
                deletion: (1_u64..=rows as u64).collect(),
                outcome,
                frequency: vec![1; rows],
                target_weight,
                controls: Vec::new(),
            }
            .validate()
            .expect("three-firm fixture validates"),
        )
        .expect("three-firm fixture canonicalizes")
        .compress(&vec![true; rows])
        .expect("three-firm fixture compresses")
    }

    fn component_array(value: VarianceComponents) -> [f64; 4] {
        [value.worker, value.firm, value.covariance, value.total]
    }

    #[test]
    fn exact_full_firm_quotient_is_last_firm_and_relabel_invariant() {
        let original_problem = three_firm_fixture([10, 20, 30]);
        let relabeled_problem = three_firm_fixture([30, 10, 20]);
        let original = run_exact_estimator(&original_problem, ExactEstimatorOptions::default())
            .expect("original result");
        let relabeled = run_exact_estimator(&relabeled_problem, ExactEstimatorOptions::default())
            .expect("relabeled result");
        for (left, right) in component_array(original.plugin)
            .into_iter()
            .zip(component_array(relabeled.plugin))
            .chain(
                component_array(original.correction)
                    .into_iter()
                    .zip(component_array(relabeled.correction)),
            )
        {
            assert!((left - right).abs() < 1.0e-10, "{left} versus {right}");
        }
        assert!(original.receipt.firm_zero_sum_residual < 1.0e-12);
        assert!(relabeled.receipt.firm_zero_sum_residual < 1.0e-12);
        assert!(
            (original.receipt.information_rcond - relabeled.receipt.information_rcond).abs()
                < 1.0e-12
        );

        let design =
            build_design(&original_problem, None, &mut NeverInterrupt).expect("quotient design");
        let columns = original_problem.workers() + original_problem.firms();
        for row in 0..original_problem.outcome.len() {
            let firm_sum: f64 = design
                [row * columns + original_problem.workers()..row * columns + columns]
                .iter()
                .sum();
            assert!(firm_sum.abs() < 1.0e-15);
        }
    }

    #[test]
    fn complete_fit_residual_includes_every_firm_and_control_equation() {
        let control = vec![-1.5, -0.5, 0.5, 1.5, -1.0, 0.0, 1.0, 2.0];
        let problem = fixture(vec![control.clone()], vec![1; 8]);
        let worker_effect = [0.6, -0.4];
        let firm_effect = [0.25, -0.25];
        let control_effect = [0.7];
        let outcome = (0..8)
            .map(|row| {
                let worker = usize::try_from(problem.row_worker[row]).expect("worker");
                let firm = usize::try_from(problem.row_firm[row]).expect("firm");
                worker_effect[worker] + firm_effect[firm] + control[row] * control_effect[0]
            })
            .collect::<Vec<_>>();
        let exact = complete_fit_residual(
            &problem,
            std::slice::from_ref(&control),
            &outcome,
            &worker_effect,
            &firm_effect,
            &control_effect,
            &mut NeverInterrupt,
            "test_fit",
        )
        .expect("exact fit receipt");
        assert!(exact.relative_norm < 1.0e-14);

        let omitted_firm = complete_fit_residual(
            &problem,
            std::slice::from_ref(&control),
            &outcome,
            &worker_effect,
            &[firm_effect[0], 0.0],
            &control_effect,
            &mut NeverInterrupt,
            "test_fit",
        )
        .expect("omitted-firm receipt");
        let omitted_control = complete_fit_residual(
            &problem,
            &[control],
            &outcome,
            &worker_effect,
            &firm_effect,
            &[0.0],
            &mut NeverInterrupt,
            "test_fit",
        )
        .expect("omitted-control receipt");
        assert!(omitted_firm.relative_norm > 1.0e-3);
        assert!(omitted_control.relative_norm > 1.0e-3);
    }

    #[test]
    fn zero_rhs_fit_receipt_uses_absolute_normalization() {
        let problem = fixture(Vec::new(), vec![1; 8]);
        let zero = vec![0.0; 8];
        let exact = complete_fit_residual(
            &problem,
            &[],
            &zero,
            &[0.0, 0.0],
            &[0.0, 0.0],
            &[],
            &mut NeverInterrupt,
            "test_zero_rhs",
        )
        .expect("zero fit receipt");
        assert_eq!(exact.rhs_norm, 0.0);
        assert_eq!(exact.relative_norm, exact.absolute_norm);
        let perturbed = complete_fit_residual(
            &problem,
            &[],
            &zero,
            &[0.1, 0.0],
            &[0.0, 0.0],
            &[],
            &mut NeverInterrupt,
            "test_zero_rhs",
        )
        .expect("perturbed zero-RHS receipt");
        assert_eq!(perturbed.rhs_norm, 0.0);
        assert_eq!(perturbed.relative_norm, perturbed.absolute_norm);
        assert!(perturbed.absolute_norm > 0.0);
    }

    #[test]
    fn canonical_control_transform_preserves_estimator_and_receipts() {
        let first = vec![
            vec![-1.5, -0.5, 0.5, 1.5, -1.0, 0.0, 1.0, 2.0],
            vec![0.5, -1.0, 2.0, -0.5, 1.5, -2.0, 0.25, 3.0],
        ];
        let transformed = vec![
            first[0]
                .iter()
                .zip(&first[1])
                .map(|(&a, &b)| 1.0e3 * (2.0 * a - b))
                .collect(),
            first[0]
                .iter()
                .zip(&first[1])
                .map(|(&a, &b)| 1.0e-2 * (a + 3.0 * b))
                .collect(),
        ];
        let left = run_exact_estimator(
            &fixture(first, vec![1; 8]),
            ExactEstimatorOptions::default(),
        )
        .expect("original control coordinates");
        let right = run_exact_estimator(
            &fixture(transformed, vec![1; 8]),
            ExactEstimatorOptions::default(),
        )
        .expect("transformed control coordinates");
        for (a, b) in component_array(left.plugin)
            .into_iter()
            .zip(component_array(right.plugin))
            .chain(
                component_array(left.correction)
                    .into_iter()
                    .zip(component_array(right.correction)),
            )
        {
            assert!((a - b).abs() < 1.0e-9, "{a} versus {b}");
        }
        assert!((left.receipt.information_rcond - right.receipt.information_rcond).abs() < 1.0e-9);
    }

    #[test]
    fn controlled_rank_loss_is_typed_for_both_deletion_modes() {
        let spike = vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        let problem = fixture(vec![spike], vec![1; 8]);
        for deletion in [DeletionMode::Observation, DeletionMode::Match] {
            let error = run_exact_estimator(
                &problem,
                ExactEstimatorOptions {
                    deletion,
                    ..ExactEstimatorOptions::default()
                },
            )
            .expect_err("deleting the unique control-support row loses rank");
            assert_eq!(error.code, ErrorCode::NonestimableDeletion);
        }
    }

    #[test]
    fn fit_inverse_and_maker_residual_receipts_are_independent() {
        let result = run_exact_estimator(
            &three_firm_fixture([10, 20, 30]),
            ExactEstimatorOptions::default(),
        )
        .expect("exact result");
        assert!(result.receipt.inverse_relres.is_finite());
        assert!(result.receipt.maker_relres.is_finite());
        assert!(result.receipt.full_fit_relres <= result.receipt.fit_residual_tolerance);
        assert!(result.receipt.working_fit_relres <= result.receipt.fit_residual_tolerance);
        assert!(result.receipt.inverse_relres <= 1.0e-8);
        assert!(result.receipt.maker_relres <= 1.0e-8);
        assert!(
            result.receipt.inverse_relres != result.receipt.full_fit_relres
                || result.receipt.inverse_relres != result.receipt.maker_relres
        );
    }

    #[test]
    fn phase_memory_forecast_has_an_exact_admission_boundary() {
        let problem = fixture(Vec::new(), vec![1; 8]);
        let baseline = run_exact_estimator(&problem, ExactEstimatorOptions::default())
            .expect("baseline forecast");
        let forecast = baseline.receipt.peak_forecast_bytes;
        assert_eq!(
            forecast,
            baseline
                .receipt
                .fit_peak_forecast_bytes
                .max(baseline.receipt.correction_peak_forecast_bytes)
        );
        run_exact_estimator(
            &problem,
            ExactEstimatorOptions {
                memory_limit_bytes: forecast,
                ..ExactEstimatorOptions::default()
            },
        )
        .expect("exact boundary admits");
        let error = run_exact_estimator(
            &problem,
            ExactEstimatorOptions {
                memory_limit_bytes: forecast - 1,
                ..ExactEstimatorOptions::default()
            },
        )
        .expect_err("one byte below forecast rejects");
        assert_eq!(error.code, ErrorCode::ResourceLimit);
    }

    fn block_fixture(block: usize) -> CompressedProblem {
        let mut worker = Vec::new();
        let mut firm = Vec::new();
        let mut deletion = Vec::new();
        let mut outcome = Vec::new();
        let mut next_deletion = 2_u64;
        for copy in 0..block {
            worker.push(1);
            firm.push(1);
            deletion.push(1);
            outcome.push(0.5 + 0.03 * copy as f64);
        }
        for (worker_id, firm_id) in [(1_u64, 2_u64), (2, 1), (2, 2)] {
            for copy in 0..5 {
                worker.push(worker_id);
                firm.push(firm_id);
                deletion.push(next_deletion);
                next_deletion += 1;
                outcome.push(worker_id as f64 - 0.4 * firm_id as f64 + 0.07 * copy as f64);
            }
        }
        let rows = outcome.len();
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
            .expect("block fixture validates"),
        )
        .expect("block fixture canonicalizes")
        .compress(&vec![true; rows])
        .expect("block fixture compresses")
    }

    fn interrupt_grid_fixture(
        side: usize,
        first_block: usize,
        with_control: bool,
    ) -> CompressedProblem {
        let mut worker = Vec::new();
        let mut firm = Vec::new();
        let mut deletion = Vec::new();
        let mut outcome = Vec::new();
        let mut target_weight = Vec::new();
        let mut control = Vec::new();
        let mut next_deletion = 2_u64;
        for worker_index in 0..side {
            for firm_index in 0..side {
                let copies = if worker_index == 0 && firm_index == 0 {
                    first_block
                } else {
                    1
                };
                let deletion_id = if worker_index == 0 && firm_index == 0 {
                    1
                } else {
                    let value = next_deletion;
                    next_deletion += 1;
                    value
                };
                for copy in 0..copies {
                    worker.push(worker_index as u64 + 1);
                    firm.push(firm_index as u64 + 1);
                    deletion.push(deletion_id);
                    let interaction = (worker_index + 1) as f64 * (firm_index + 1) as f64;
                    outcome.push(
                        0.2 * worker_index as f64 - 0.1 * firm_index as f64
                            + 0.003 * interaction
                            + 0.0001 * copy as f64,
                    );
                    target_weight.push(1.0);
                    if with_control {
                        control.push(interaction);
                    }
                }
            }
        }
        let rows = outcome.len();
        CanonicalInput::from_validated(
            InputColumns {
                worker,
                firm,
                deletion,
                outcome,
                frequency: vec![1; rows],
                target_weight,
                controls: if with_control {
                    vec![control]
                } else {
                    Vec::new()
                },
            }
            .validate()
            .expect("interrupt grid validates"),
        )
        .expect("interrupt grid canonicalizes")
        .compress(&vec![true; rows])
        .expect("interrupt grid compresses")
    }

    fn assert_delayed_break(
        problem: &CompressedProblem,
        options: ExactEstimatorOptions,
        phase: &'static str,
    ) {
        let mut interrupt = BreakAfter {
            phase,
            callbacks_before_break: 1,
            matching_callbacks: 0,
        };
        let error = run_exact_estimator_with_interrupt(problem, options, &mut interrupt)
            .expect_err("delayed phase break must escape exact estimator");
        assert_eq!(error.code, ErrorCode::UserBreak);
        assert_eq!(interrupt.matching_callbacks, 2);
    }

    #[test]
    fn maker_assemblies_poll_after_multiple_flattened_chunks() {
        // P=20.  B=16 exercises B<=P with 5,120 inner products; B=32
        // exercises B>P with 12,800 inner products.
        assert_delayed_break(
            &interrupt_grid_fixture(10, 16, false),
            ExactEstimatorOptions::default(),
            "exact_match_maker_assembly",
        );
        assert_delayed_break(
            &interrupt_grid_fixture(10, 32, false),
            ExactEstimatorOptions::default(),
            "exact_match_reduced_assembly",
        );
    }

    #[test]
    fn observation_target_bilinear_polls_inside_cell_work() {
        // Two scalar cross-moments per cell make 46^2 cells cross the 4,096
        // work boundary during the first observation correction.
        assert_delayed_break(
            &interrupt_grid_fixture(46, 1, false),
            ExactEstimatorOptions {
                deletion: DeletionMode::Observation,
                ..ExactEstimatorOptions::default()
            },
            "exact_observation_target_bilinear_cells",
        );
    }

    #[test]
    fn full_and_working_beta_matvecs_poll_after_multiple_flattened_chunks() {
        // The W+F embedding has P=66, hence 4,356 matvec products.
        let plain = interrupt_grid_fixture(33, 1, false);
        assert_delayed_break(
            &plain,
            ExactEstimatorOptions::default(),
            "exact_full_beta_matvec",
        );
        let controlled = interrupt_grid_fixture(33, 1, true);
        assert_delayed_break(
            &controlled,
            ExactEstimatorOptions {
                nuisance: NuisanceMode::FixedOffset,
                ..ExactEstimatorOptions::default()
            },
            "exact_working_beta_matvec",
        );
    }

    fn assert_memory_boundary(problem: &CompressedProblem, options: ExactEstimatorOptions) {
        let result = run_exact_estimator(problem, options).expect("forecast baseline");
        let forecast = result.receipt.peak_forecast_bytes;
        run_exact_estimator(
            problem,
            ExactEstimatorOptions {
                memory_limit_bytes: forecast,
                ..options
            },
        )
        .expect("forecast boundary admits");
        let error = run_exact_estimator(
            problem,
            ExactEstimatorOptions {
                memory_limit_bytes: forecast - 1,
                ..options
            },
        )
        .expect_err("one byte below forecast rejects");
        assert_eq!(error.code, ErrorCode::ResourceLimit);
    }

    #[test]
    fn memory_boundaries_cover_both_maker_shapes_and_fixedoffset_observation() {
        // Embedding P=4: these exercise B<=P and B>P maker implementations.
        assert_memory_boundary(&block_fixture(4), ExactEstimatorOptions::default());
        assert_memory_boundary(&block_fixture(12), ExactEstimatorOptions::default());
        let controlled = fixture(
            vec![vec![-1.5, -0.5, 0.5, 1.5, -1.0, 0.0, 1.0, 2.0]],
            vec![1; 8],
        );
        assert_memory_boundary(
            &controlled,
            ExactEstimatorOptions {
                deletion: DeletionMode::Observation,
                nuisance: NuisanceMode::FixedOffset,
                ..ExactEstimatorOptions::default()
            },
        );
    }

    #[test]
    fn maker_and_deleted_factorization_preserve_user_break() {
        let plain = fixture(Vec::new(), vec![1; 8]);
        let maker_error = run_exact_estimator_with_interrupt(
            &plain,
            ExactEstimatorOptions::default(),
            &mut BreakOnPhase {
                phase: "exact_match_maker_inverse",
            },
        )
        .expect_err("maker inverse break");
        assert_eq!(maker_error.code, ErrorCode::UserBreak);

        let controlled = fixture(
            vec![vec![-1.5, -0.5, 0.5, 1.5, -1.0, 0.0, 1.0, 2.0]],
            vec![1; 8],
        );
        let delete_error = run_exact_estimator_with_interrupt(
            &controlled,
            ExactEstimatorOptions::default(),
            &mut BreakOnPhase {
                phase: "exact_match_deleted_information",
            },
        )
        .expect_err("deleted-information break");
        assert_eq!(delete_error.code, ErrorCode::UserBreak);
    }

    fn permuted_control_fixture(order: &[usize]) -> CompressedProblem {
        let worker = [1_u64, 1, 1, 1, 2, 2, 2, 2];
        let firm = [1_u64, 1, 2, 2, 1, 1, 2, 2];
        let outcome = [1.0, 2.0, 0.0, 2.5, -1.0, 1.5, 2.0, -2.0];
        let target = [1.0, 2.0, 2.0, 1.0, 3.0, 1.0, 2.0, 4.0];
        let control = [-1.5, -0.5, 0.5, 1.5, -1.0, 0.0, 1.0, 2.0];
        CanonicalInput::from_validated(
            InputColumns {
                worker: order.iter().map(|&row| worker[row]).collect(),
                firm: order.iter().map(|&row| firm[row]).collect(),
                deletion: order.iter().map(|&row| row as u64 + 1).collect(),
                outcome: order.iter().map(|&row| outcome[row]).collect(),
                frequency: vec![1; order.len()],
                target_weight: order.iter().map(|&row| target[row]).collect(),
                controls: vec![order.iter().map(|&row| control[row]).collect()],
            }
            .validate()
            .expect("permuted control fixture validates"),
        )
        .expect("permuted control fixture canonicalizes")
        .compress(&vec![true; order.len()])
        .expect("permuted control fixture compresses")
    }

    #[test]
    fn canonical_control_semantic_order_is_row_permutation_invariant() {
        let left = run_exact_estimator(
            &permuted_control_fixture(&[0, 1, 2, 3, 4, 5, 6, 7]),
            ExactEstimatorOptions::default(),
        )
        .expect("source order");
        let right = run_exact_estimator(
            &permuted_control_fixture(&[7, 2, 5, 0, 6, 1, 4, 3]),
            ExactEstimatorOptions::default(),
        )
        .expect("permuted order");
        for (a, b) in component_array(left.plugin)
            .into_iter()
            .zip(component_array(right.plugin))
            .chain(
                component_array(left.correction)
                    .into_iter()
                    .zip(component_array(right.correction)),
            )
        {
            assert!((a - b).abs() < 1.0e-10, "{a} versus {b}");
        }
        assert!(
            (left.receipt.control_basis_forward_error - right.receipt.control_basis_forward_error)
                .abs()
                < 1.0e-12
        );
    }

    #[test]
    fn planned_exact_adapter_is_rng_free_not_applicable_and_wall_advisory() {
        let problem = fixture(Vec::new(), vec![1; 8]);
        let legacy = run_exact_estimator(&problem, ExactEstimatorOptions::default())
            .expect("legacy exact result");
        let planned = run_exact_estimator_planned(
            &problem,
            PlannedExactEstimatorOptions {
                estimator: ExactEstimatorOptions::default(),
                wallseconds: Some(0.25),
            },
        )
        .expect("planned exact result");
        for (left, right) in component_array(legacy.plugin)
            .into_iter()
            .zip(component_array(planned.estimator.plugin))
            .chain(
                component_array(legacy.correction)
                    .into_iter()
                    .zip(component_array(planned.estimator.correction)),
            )
        {
            assert_eq!(left.to_bits(), right.to_bits());
        }
        assert_eq!(
            planned.execution.selected_engine,
            SelectedEngine::NotApplicable
        );
        assert_eq!(
            planned.execution.solver_route,
            ExactPlanApplicability::NotApplicable
        );
        assert_eq!(
            planned.execution.batches,
            ExactPlanApplicability::NotApplicable
        );
        assert_eq!(planned.execution.counter.total.planned_logical_atoms, 0);
        assert_eq!(planned.execution.counter.total.actual_logical_atoms, 0);
        assert_eq!(
            planned.execution.counter.total.planned_unique_packed_words,
            0
        );
        assert_eq!(
            planned
                .execution
                .counter
                .total
                .planned_physical_bernoulli_trials,
            0
        );
        assert_eq!(planned.execution.logical_atoms_before_plan_freeze, 0);
        assert_eq!(planned.execution.unique_packed_words_before_plan_freeze, 0);
        assert_eq!(planned.execution.physical_trials_before_plan_freeze, 0);
        assert!(planned.execution.plan_frozen_before_execution);
        assert_eq!(planned.execution.threads.used, 1);
        assert_eq!(planned.execution.threads.parallel_regions, 0);
        assert_eq!(
            planned.execution.wall.status,
            crate::wall_plan::WallAdvisoryStatus::Uncalibrated
        );
        assert!(planned.execution.wall.total_work > 0);
        assert_eq!(
            planned.execution.peak_forecast_bytes,
            planned.estimator.receipt.peak_forecast_bytes
        );
    }

    #[test]
    fn planned_exact_wallseconds_does_not_change_output_or_memory_admission() {
        let problem = fixture(Vec::new(), vec![1; 8]);
        let baseline =
            run_exact_estimator_planned(&problem, PlannedExactEstimatorOptions::default())
                .expect("baseline planned exact");
        let peak = baseline.execution.peak_forecast_bytes;
        let with_wall = run_exact_estimator_planned(
            &problem,
            PlannedExactEstimatorOptions {
                estimator: ExactEstimatorOptions {
                    memory_limit_bytes: peak,
                    ..ExactEstimatorOptions::default()
                },
                wallseconds: Some(100.0),
            },
        )
        .expect("exact boundary with wall");
        for (left, right) in component_array(baseline.estimator.corrected)
            .into_iter()
            .zip(component_array(with_wall.estimator.corrected))
        {
            assert_eq!(left.to_bits(), right.to_bits());
        }
        let error = run_exact_estimator_planned(
            &problem,
            PlannedExactEstimatorOptions {
                estimator: ExactEstimatorOptions {
                    memory_limit_bytes: peak - 1,
                    ..ExactEstimatorOptions::default()
                },
                wallseconds: Some(100.0),
            },
        )
        .expect_err("one byte below exact plan rejects");
        assert_eq!(error.code, ErrorCode::ResourceLimit);
    }
}
