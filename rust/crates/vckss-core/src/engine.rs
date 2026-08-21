// SPDX-License-Identifier: GPL-3.0-only

//! End-to-end no-control, match-deletion improved-JLA engine.

use crate::error::{BackendError, ErrorCode, Result};
use crate::jla::{plugin_components, JlaPlan, VarianceComponents};
use crate::problem::CompressedProblem;
use crate::rng::{CounterRng, ProbeDomain, MAX_PHYSICAL_WORDS_PER_ATOM};
use crate::solver::{
    LinearSolverOptions, LinearSolverRoute, PreparedSolverReceipt, PreparedTwoWaySolver,
    RoutedSolveReceipt,
};
use crate::types::{DeletionMode, RngContract};

const ROUNDOFF_GATE: f64 = 4096.0 * f64::EPSILON;

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
}

#[derive(Clone, Debug)]
pub struct JlaEngineResult {
    pub plugin: VarianceComponents,
    pub correction: VarianceComponents,
    pub corrected: VarianceComponents,
    pub numerical_mcse: NumericalMcse,
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
    // All unsupported features, tuning, semantic plans, scatter identities,
    // target centering geometry, and solver setup are settled before the
    // counter generator is instantiated or any estimator atom is addressed.
    let options = options.validate()?;
    if !problem.controls.is_empty() {
        return Err(BackendError::new(
            ErrorCode::UnsupportedFeature,
            "jla_validate",
            "the Rust JLA engine supports no-control problems only",
        ));
    }
    let plan = JlaPlan::build_no_controls(problem)?;
    plan.validate_against_problem(problem)?;
    validate_target_geometry(problem, &plan)?;
    preflight_trial_words("leverage", &plan.deletion.physical_count)?;
    preflight_trial_words("target", &plan.target.physical_count)?;
    let solver = PreparedTwoWaySolver::prepare(problem, options.solver)?;

    let (outcome_worker_rhs, outcome_firm_rhs) = solver.operator().outcome_rhs()?;
    let full_fit = solver
        .solve(&outcome_worker_rhs, &outcome_firm_rhs)
        .map_err(|error| rhs_error(error, JlaSolvePhase::FullFit, None, JlaRhsSide::Joint))?;
    let full_fit_receipt = rhs_receipt(
        &full_fit.receipt,
        full_fit.solution.residual.relative_norm,
        full_fit.solution.residual.rhs_norm,
        JlaSolvePhase::FullFit,
        None,
        JlaRhsSide::Joint,
    )?;
    let fitted_cell =
        cell_predictions(problem, &full_fit.solution.worker, &full_fit.solution.firm)?;
    let plugin = plugin_components(problem, &full_fit.solution.worker, &full_fit.solution.firm)?;

    let rng = CounterRng::new(options.seed);
    let groups = plan.deletion_units();
    let mut moments = vec![FiveMoments::default(); groups];
    let mut leverage_receipts = Vec::with_capacity(options.probes as usize);
    for first in (0..options.probes as usize).step_by(options.leverage_batch_width) {
        let width = options
            .leverage_batch_width
            .min(options.probes as usize - first);
        let atoms = rademacher_atoms(
            rng,
            ProbeDomain::Leverage,
            first,
            width,
            &plan.deletion.semantic_rank,
            &plan.deletion.physical_count,
        )?;
        let (worker_rhs, firm_rhs) = leverage_rhs(problem, &plan, &atoms, width)?;
        let solved = solver
            .solve_batch(&worker_rhs, &firm_rhs, width)
            .map_err(|error| {
                contextual_batch_error(error, JlaSolvePhase::Leverage, first, false)
            })?;
        for column in 0..width {
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
            let prediction = cell_predictions(problem, &solution.worker, &solution.firm)?;
            for group in 0..groups {
                let cell =
                    usize::try_from(plan.deletion.cell[group]).expect("validated deletion cell");
                let frequency = plan.deletion.physical_count[group] as f64;
                let projection = frequency.sqrt() * prediction[cell];
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
                moments[group].add(projection, residual);
            }
        }
    }

    let adjustment = leverage_adjustment(problem, &plan, &fitted_cell, &moments, options)?;
    let mut target_draws = vec![VarianceComponents::default(); options.probes as usize];
    let mut target_receipts = Vec::with_capacity(2 * options.probes as usize);
    for first in (0..options.probes as usize).step_by(options.target_batch_width) {
        let width = options
            .target_batch_width
            .min(options.probes as usize - first);
        let atoms = rademacher_atoms(
            rng,
            ProbeDomain::Target,
            first,
            width,
            &plan.target.semantic_rank,
            &plan.target.physical_count,
        )?;
        let (directions, reference_scale) =
            target_directions(problem, &plan, &atoms, width, first)?;
        let (worker_rhs, firm_rhs) =
            target_rhs(problem, &directions, &reference_scale, width, first)?;
        let solved = solver
            .solve_batch(&worker_rhs, &firm_rhs, 2 * width)
            .map_err(|error| contextual_batch_error(error, JlaSolvePhase::Target, first, true))?;
        for column in 0..width {
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
            let worker_prediction =
                cell_predictions(problem, &worker_solution.worker, &worker_solution.firm)?;
            let firm_prediction =
                cell_predictions(problem, &firm_solution.worker, &firm_solution.firm)?;
            target_draws[probe] = contract_target_draw(
                &adjustment.cell_correction_weight,
                &worker_prediction,
                &firm_prediction,
                probe,
            )?;
        }
    }

    let correction = mean_components(&target_draws)?;
    let corrected = subtract_components(plugin, correction)?;
    let numerical_mcse = component_mcse(&target_draws)?;
    let accounting_residual = accounting_residuals(plugin, correction, corrected, &target_draws)?;
    let mut all_receipts = Vec::with_capacity(1 + leverage_receipts.len() + target_receipts.len());
    all_receipts.push(&full_fit_receipt);
    all_receipts.extend(leverage_receipts.iter());
    all_receipts.extend(target_receipts.iter());
    let max_reduced_residual = all_receipts
        .iter()
        .map(|receipt| receipt.reduced_residual)
        .fold(0.0_f64, f64::max);
    let max_complete_residual = all_receipts
        .iter()
        .map(|receipt| receipt.complete_residual)
        .fold(0.0_f64, f64::max);
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
        full_residual_tolerance: options.solver.full_residual_tolerance,
        solver: solver.receipt().clone(),
        full_fit: full_fit_receipt,
        leverage_rhs: leverage_receipts,
        target_rhs: target_receipts,
        max_reduced_residual,
        max_complete_residual,
        max_leverage: adjustment
            .projection_share
            .iter()
            .copied()
            .fold(0.0_f64, f64::max),
        max_reciprocal_residual: adjustment.max_reciprocal_residual,
        accounting_residual,
        topology_checksum: problem.topology_checksum,
    };
    Ok(JlaEngineResult {
        plugin,
        correction,
        corrected,
        numerical_mcse,
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

fn leverage_adjustment(
    problem: &CompressedProblem,
    plan: &JlaPlan,
    fitted_cell: &[f64],
    moments: &[FiveMoments],
    options: JlaEngineOptions,
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

fn rademacher_atoms(
    rng: CounterRng,
    domain: ProbeDomain,
    first_probe: usize,
    width: usize,
    entity: &[u64],
    trials: &[u64],
) -> Result<Vec<i64>> {
    let length = entity.len().checked_mul(width).ok_or_else(|| {
        BackendError::new(
            ErrorCode::ResourceLimit,
            "jla_rng",
            "atom matrix length overflow",
        )
    })?;
    let mut atoms = vec![0_i64; length];
    rng.fill_rademacher_sums(
        domain,
        u64::try_from(first_probe).map_err(|_| {
            BackendError::new(ErrorCode::ResourceLimit, "jla_rng", "probe index overflow")
        })?,
        width,
        entity,
        trials,
        &mut atoms,
    )?;
    Ok(atoms)
}

fn leverage_rhs(
    problem: &CompressedProblem,
    plan: &JlaPlan,
    atoms: &[i64],
    width: usize,
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
    let mut cell = vec![0.0; problem.cells() * width];
    for column in 0..width {
        for group in 0..groups {
            let target = usize::try_from(plan.deletion.cell[group]).expect("validated cell");
            cell[column * problem.cells() + target] += atoms[column * groups + group] as f64;
        }
    }
    transpose_cell_rhs(problem, &cell, width)
}

fn target_directions(
    problem: &CompressedProblem,
    plan: &JlaPlan,
    atoms: &[i64],
    width: usize,
    first_probe: usize,
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
            let cell = usize::try_from(plan.target.cell[stratum]).expect("validated target cell");
            let scale = (plan.target.per_copy_mass[stratum] / problem.target_total).sqrt();
            first[cell].add(scale * atoms[column * strata + stratum] as f64);
        }
        let first = first.into_iter().map(StableSum::finish).collect::<Vec<_>>();
        let mut total = StableSum::default();
        let mut absolute = StableSum::default();
        for &value in &first {
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
            let value = first[cell] - problem.cell_target_sum[cell] / problem.target_total * total;
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

fn target_rhs(
    problem: &CompressedProblem,
    direction: &[f64],
    reference_scale: &[f64],
    width: usize,
    first_probe: usize,
) -> Result<(Vec<f64>, Vec<f64>)> {
    if reference_scale.len() != width
        || reference_scale
            .iter()
            .any(|value| !value.is_finite() || *value < 0.0)
    {
        return Err(BackendError::new(
            ErrorCode::TargetCenteringFailed,
            "jla_target",
            "target reference-scale vector is invalid",
        ));
    }
    let (score_worker, score_firm) = transpose_cell_rhs(problem, direction, width)?;
    let workers = problem.workers();
    let firms = problem.firms();
    let mut worker_rhs = vec![0.0; workers * 2 * width];
    let mut firm_rhs = vec![0.0; firms * 2 * width];
    for column in 0..width {
        let mut worker = score_worker[column * workers..(column + 1) * workers].to_vec();
        let mut firm = score_firm[column * firms..(column + 1) * firms].to_vec();
        let probe = first_probe + column;
        balance_score(
            &mut worker,
            reference_scale[column],
            probe,
            JlaRhsSide::Worker,
        )?;
        balance_score(&mut firm, reference_scale[column], probe, JlaRhsSide::Firm)?;
        worker_rhs[2 * column * workers..(2 * column + 1) * workers].copy_from_slice(&worker);
        firm_rhs[(2 * column + 1) * firms..(2 * column + 2) * firms].copy_from_slice(&firm);
    }
    Ok((worker_rhs, firm_rhs))
}

fn balance_score(score: &mut [f64], reference: f64, probe: usize, side: JlaRhsSide) -> Result<()> {
    if score.is_empty() || !reference.is_finite() || reference < 0.0 {
        return Err(BackendError::new(
            ErrorCode::TargetCenteringFailed,
            "jla_target",
            format!("invalid compatibility score at probe {probe}, side {side:?}"),
        ));
    }
    let original = *score.last().expect("nonempty score");
    let mut preceding = StableSum::default();
    for &value in &score[..score.len() - 1] {
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

fn transpose_cell_rhs(
    problem: &CompressedProblem,
    cell_value: &[f64],
    width: usize,
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
            let value = cell_value[column * problem.cells() + cell];
            let worker_index = usize::try_from(problem.cell_worker[cell]).expect("worker");
            let firm_index = usize::try_from(problem.cell_firm[cell]).expect("firm");
            worker[column * problem.workers() + worker_index] += value;
            firm[column * problem.firms() + firm_index] += value;
        }
    }
    Ok((worker, firm))
}

fn cell_predictions(problem: &CompressedProblem, worker: &[f64], firm: &[f64]) -> Result<Vec<f64>> {
    if worker.len() != problem.workers() || firm.len() != problem.firms() {
        return Err(BackendError::invalid(
            "jla_prediction",
            "coefficient dimensions differ",
        ));
    }
    let prediction = (0..problem.cells())
        .map(|cell| {
            worker[usize::try_from(problem.cell_worker[cell]).expect("worker")]
                + firm[usize::try_from(problem.cell_firm[cell]).expect("firm")]
        })
        .collect::<Vec<_>>();
    if prediction.iter().any(|value| !value.is_finite()) {
        return Err(BackendError::new(
            ErrorCode::CorrectionNonFinite,
            "jla_prediction",
            "cell prediction is nonfinite",
        ));
    }
    Ok(prediction)
}

fn contract_target_draw(
    weight: &[f64],
    worker: &[f64],
    firm: &[f64],
    probe: usize,
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
        worker_second.add(weight[cell] * worker[cell] * worker[cell]);
        firm_second.add(weight[cell] * firm[cell] * firm[cell]);
        covariance.add(weight[cell] * worker[cell] * firm[cell]);
    }
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

fn mean_components(draws: &[VarianceComponents]) -> Result<VarianceComponents> {
    if draws.is_empty() {
        return Err(BackendError::invariant(
            "jla_target",
            "target draw set is empty",
        ));
    }
    if draws.iter().any(|draw| !components_finite(*draw)) {
        return Err(BackendError::new(
            ErrorCode::CorrectionNonFinite,
            "jla_target",
            "target draw reduction received a nonfinite component",
        ));
    }
    let mut sums = [StableSum::default(); 4];
    for draw in draws {
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

fn component_mcse(draws: &[VarianceComponents]) -> Result<NumericalMcse> {
    if draws.len() < 2 {
        return Err(BackendError::invalid(
            "jla_target",
            "MCSE requires at least two draws",
        ));
    }
    let mean = mean_components(draws)?;
    let mut sums = [StableSum::default(); 4];
    for draw in draws {
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

fn accounting_residuals(
    plugin: VarianceComponents,
    correction: VarianceComponents,
    corrected: VarianceComponents,
    draws: &[VarianceComponents],
) -> Result<f64> {
    if !components_finite(plugin)
        || !components_finite(correction)
        || !components_finite(corrected)
        || draws.iter().any(|draw| !components_finite(*draw))
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
    for draw in draws {
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
    use crate::krylov::PcgOptions;
    use crate::problem::CanonicalInput;
    use crate::types::InputColumns;

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
        let leverage = rademacher_atoms(rng, ProbeDomain::Leverage, 0, 5, &entity, &trials)
            .expect("leverage atoms");
        let target = rademacher_atoms(rng, ProbeDomain::Target, 0, 5, &entity, &trials)
            .expect("target atoms");
        assert_ne!(leverage, target);
        assert_ne!(ProbeDomain::Leverage.tag(), ProbeDomain::Target.tag());
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
            0.0,
            0.0,
            JlaSolvePhase::FullFit,
            None,
            JlaRhsSide::Joint,
        )
        .expect("zero exact RHS receipt");
        assert!(!nonzero.zero_rhs);
        assert!(zero.zero_rhs);
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
}
