// SPDX-License-Identifier: GPL-3.0-only

//! Internal component-inference primitives.
//!
//! Its independent core contract takes an oracle-provided, strictly positive
//! variance vector for observation rows or collapsed match rows. The plugin
//! and Stata layers expose only the separately named observation-level
//! structured fits; the fixed-offset match q=0 and q=1 layers remain internal.
//! The variance model affects only this covariance attachment, so the
//! generic-JLA point estimator is unchanged. The `q=0` path supplies a Gaussian
//! approximation with spectral diagnostics. The observation and internal
//! grouped `q=1` paths remove one estimated generalized eigenmode and construct
//! the corresponding Andrews--Mikusheva ellipse-image interval.

use crate::dense::symmetric_eigen_extremes;
use crate::error::{BackendError, ErrorCode, Result};
use crate::interrupt::{checkpoint_chunk, InterruptCheck, NeverInterrupt};
use crate::model_operator::zeroed_f64_with_interrupt;
use crate::model_solver::ModelCoefficients;
use crate::problem::CompressedProblem;
use crate::rng::{CounterRng, ProbeDomain};
use crate::structured_variance::{
    StructuredVarianceModel, StructuredVarianceOptions, StructuredVarianceResult,
};

pub const COMPONENT_INFERENCE_SCHEMA_VERSION: u32 = 6;
pub const PRIMITIVE_TARGETS: usize = 3;
pub const REPORTED_TARGETS: usize = 4;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ComponentInferenceUnit {
    #[default]
    Observation,
    Match,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(u32)]
pub enum ComponentVarianceSource {
    #[default]
    Oracle = 0,
    StructuredCommon = 1,
    StructuredLeverage = 2,
}

impl ComponentVarianceSource {
    #[must_use]
    pub fn structured_model(self) -> Option<StructuredVarianceModel> {
        match self {
            Self::Oracle => None,
            Self::StructuredCommon => Some(StructuredVarianceModel::Common),
            Self::StructuredLeverage => Some(StructuredVarianceModel::LeverageOnly),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ComponentReferenceDistribution {
    #[default]
    Q0,
    Q1,
}

#[derive(Clone, Copy, Debug)]
pub struct ComponentInferenceOptions {
    pub seed: u64,
    pub probes: u32,
    pub batch_width: usize,
    pub psd_tolerance: f64,
    /// Gaussian trace-square probes used only for target-spectrum diagnostics.
    pub spectrum_probes: u32,
    /// Fixed block-power iterations.  A fixed count preserves scheduling and
    /// batch invariance; the residual receipt, not an early exit, certifies it.
    pub spectrum_iterations: u32,
    pub spectrum_tolerance: f64,
    pub reference_distribution: ComponentReferenceDistribution,
    pub confidence_level: f64,
    pub critical_simulations: u32,
}

impl Default for ComponentInferenceOptions {
    fn default() -> Self {
        Self {
            seed: 1,
            probes: 1_000,
            batch_width: 8,
            psd_tolerance: 1.0e-8,
            spectrum_probes: 128,
            spectrum_iterations: 128,
            spectrum_tolerance: 2.0e-3,
            reference_distribution: ComponentReferenceDistribution::Q0,
            confidence_level: 0.95,
            critical_simulations: 100_000,
        }
    }
}

impl ComponentInferenceOptions {
    pub fn validate(self) -> Result<Self> {
        if self.probes < 2 {
            return Err(invalid("component-inference probes must be at least two"));
        }
        if self.batch_width == 0 {
            return Err(invalid("component-inference batch width must be positive"));
        }
        if !self.psd_tolerance.is_finite() || self.psd_tolerance < 0.0 || self.psd_tolerance >= 0.1
        {
            return Err(invalid(
                "component-inference PSD tolerance must lie in [0, 0.1)",
            ));
        }
        if self.spectrum_probes < 2 {
            return Err(invalid(
                "component-inference spectrum probes must be at least two",
            ));
        }
        if self.spectrum_iterations < 2 || self.spectrum_iterations > 10_000 {
            return Err(invalid(
                "component-inference spectrum iterations must lie in [2, 10000]",
            ));
        }
        if !self.spectrum_tolerance.is_finite()
            || self.spectrum_tolerance <= 0.0
            || self.spectrum_tolerance >= 0.1
        {
            return Err(invalid(
                "component-inference spectrum tolerance must lie in (0, 0.1)",
            ));
        }
        if !self.confidence_level.is_finite()
            || self.confidence_level <= 0.0
            || self.confidence_level >= 1.0
        {
            return Err(invalid(
                "component-inference confidence level must lie in (0, 1)",
            ));
        }
        if self.critical_simulations < 1_000 {
            return Err(invalid(
                "component-inference q=1 critical simulations must be at least 1000",
            ));
        }
        Ok(self)
    }
}

/// Numerical evidence about the generalized spectrum of one reported target.
/// Concentration statistics are diagnostics, not automatic routing cutoffs.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ComponentSpectrumDiagnostics {
    pub certified: bool,
    pub leading_eigenvalue: f64,
    pub second_eigenvalue: f64,
    pub trace_square_raw: f64,
    pub trace_square: f64,
    pub trace_square_mcse: f64,
    pub trace_reconciliation: f64,
    pub leading_share: f64,
    pub leading_share_mcse_trace_only: f64,
    pub remainder_leading_share: f64,
    pub maximum_mode_weight_squared: f64,
    pub leading_residual: f64,
    pub second_residual: f64,
    pub probes: u32,
    pub iterations: u32,
}

/// Target-local failure codes. Shared model, solve and identity failures still
/// return an error for the complete attachment. No status selects a fallback.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(u32)]
pub enum ComponentQ1Status {
    #[default]
    Computed = 0,
    NonpositiveVariance = 1,
    SingularCovariance = 2,
    IntervalFailure = 3,
    ModeNotCertified = 6,
}

/// Numerical availability, not a claim that a target's asymptotic regime holds.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(u32)]
pub enum ComponentQ0Status {
    #[default]
    Computed = 0,
    NonpositiveVariance = 1,
    NoLinearInfluence = 4,
    ModeNotCertified = 6,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(u32)]
pub enum ComponentJointStatus {
    #[default]
    Computed = 0,
    NonpositiveDiagonal = 1,
    Indefinite = 2,
}

/// Target-specific `q=1` decomposition and Andrews--Mikusheva confidence set.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ComponentQ1TargetResult {
    pub status: ComponentQ1Status,
    pub point_estimate: f64,
    pub leading_score: f64,
    /// Leave-one-inferential-unit estimate used to recenter the leading square.
    /// This is deliberately distinct from `leading_variance`, which comes
    /// from the positive covariance model used for studentization.
    pub leading_variance_correction: f64,
    pub leading_variance: f64,
    pub leading_recentered_component: f64,
    pub remainder_estimate: f64,
    /// Numerical discrepancy between the algebraic q=1 decomposition and the
    /// direct rank-one-subtracted leave-out kernel action.
    pub remainder_identity_error: f64,
    pub leading_remainder_covariance: f64,
    pub remainder_variance: f64,
    pub remainder_trace_mcse: f64,
    pub standardized_determinant: f64,
    pub remainder_influence_variance: f64,
    pub remainder_trace_variance: f64,
    pub curvature: f64,
    pub critical_value: f64,
    pub critical_draws: u32,
    pub confidence_lower: f64,
    pub confidence_upper: f64,
    pub leading_f_statistic: f64,
    /// Largest observation contribution to the linear-influence variance,
    /// divided by the complete linear-influence variance.
    pub remainder_influence_concentration: f64,
}

#[derive(Clone, Debug)]
pub struct PreparedComponentInference {
    pub schema_version: u32,
    pub inference_unit: ComponentInferenceUnit,
    pub variance_source: ComponentVarianceSource,
    pub variance: Vec<f64>,
    pub options: ComponentInferenceOptions,
    pub structured_options: StructuredVarianceOptions,
    pub persistent_bytes: u64,
    /// Legacy constructors remain strict; the additive public boundary opts in.
    pub individual_intervals: bool,
    pub(crate) residual_moments: Option<crate::residual_moment_inference::Options>,
    pub(crate) design_only_order: bool,
    /// V3 policy: common residual-moment fitter and span-preserving basis reduction.
    pub(crate) unified_variance_fit: bool,
}

#[derive(Clone, Debug)]
pub struct ComponentInferenceResult {
    pub schema_version: u32,
    pub joint_status: ComponentJointStatus,
    pub q0_status: [ComponentQ0Status; REPORTED_TARGETS],
    pub inference_unit: ComponentInferenceUnit,
    pub independent_units: u64,
    /// True only for grouped match inference that conditions on the realized
    /// full-sample control offset and omits uncertainty from estimating it.
    pub nuisance_uncertainty_conditioned_away: bool,
    pub effective_match_count: f64,
    pub largest_match_mass_share: f64,
    pub largest_match_leverage: f64,
    pub smallest_maker_denominator: f64,
    pub variance_source: ComponentVarianceSource,
    /// Present only for an internally fitted structured variance model.  Both
    /// registered fits are retained so the leverage-only sensitivity is an
    /// auditable result rather than a second model fit.
    pub structured_variance: Option<StructuredVarianceResult>,
    /// Separate internal fitting-method identity; never a public oracle fit.
    pub residual_moments: Option<crate::residual_moment_inference::Diagnostic>,
    /// Row-major covariance; diagnostic raw values only when joint_status fails.
    pub primitive_covariance: [f64; PRIMITIVE_TARGETS * PRIMITIVE_TARGETS],
    /// Exact structural map of `primitive_covariance` to the four public
    /// targets.  The fourth row and column are not independently estimated.
    pub covariance: [f64; REPORTED_TARGETS * REPORTED_TARGETS],
    pub influence_term: [f64; PRIMITIVE_TARGETS * PRIMITIVE_TARGETS],
    /// Largest independent inferential-unit contribution to each target's
    /// linear-influence variance. The unit is an observation or a declared
    /// match according to `inference_unit`; the fourth value uses the exact
    /// three-to-four influence map.
    pub influence_concentration: [f64; REPORTED_TARGETS],
    pub trace_term: [f64; PRIMITIVE_TARGETS * PRIMITIVE_TARGETS],
    /// Numerical standard errors for the finite-probe trace-covariance terms.
    pub trace_mcse: [f64; PRIMITIVE_TARGETS * PRIMITIVE_TARGETS],
    pub probe_mean: [f64; PRIMITIVE_TARGETS],
    pub probes: u32,
    pub psd_cleanup: f64,
    pub smallest_eigenvalue_before_cleanup: f64,
    pub largest_eigenvalue_before_cleanup: f64,
    pub solve_receipts: Vec<ComponentInferenceSolveReceipt>,
    pub maximum_iterations: u32,
    pub maximum_reduced_residual: f64,
    pub maximum_complete_residual: f64,
    pub full_residual_tolerance: f64,
    pub peak_forecast_bytes: u64,
    pub leverage: Vec<f64>,
    pub maker_inverse: Vec<f64>,
    pub target_diagonal: [Vec<f64>; PRIMITIVE_TARGETS],
    pub influence: [Vec<f64>; PRIMITIVE_TARGETS],
    pub point_correction_identity_error: f64,
    pub counter_atoms: u64,
    pub counter_words: u64,
    /// Counter-V1 simulations used for the q=1 curvature critical value; zero
    /// for q=0 results.
    pub critical_simulations: u32,
    pub spectrum: [ComponentSpectrumDiagnostics; REPORTED_TARGETS],
    pub q1: Option<[ComponentQ1TargetResult; REPORTED_TARGETS]>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ComponentInferenceSolvePhase {
    Influence,
    CovarianceProbe,
    SpectrumStart,
    SpectrumIteration,
    SpectrumTrace,
}

#[derive(Clone, Copy, Debug)]
pub struct ComponentInferenceSolveReceipt {
    pub phase: ComponentInferenceSolvePhase,
    pub target_or_probe: u32,
    pub iterations: u32,
    pub reduced_residual: f64,
    pub complete_residual: f64,
    pub full_residual_tolerance: f64,
}

/// Prepare the private oracle-variance attachment.  Unit frequency is an
/// intentional scientific boundary of the first implementation, not a
/// storage shortcut for general literal-copy weights.
pub fn prepare_oracle_component_inference(
    problem: &CompressedProblem,
    variance: &[f64],
    options: ComponentInferenceOptions,
) -> Result<PreparedComponentInference> {
    prepare_oracle_component_inference_with_interrupt(
        problem,
        variance,
        options,
        &mut NeverInterrupt,
    )
}

/// Interruptible preparation used by a future atomic generation boundary.
/// Validation and copying complete before the prepared attachment can be
/// published to a session.
pub fn prepare_oracle_component_inference_with_interrupt(
    problem: &CompressedProblem,
    variance: &[f64],
    options: ComponentInferenceOptions,
    interrupt: &mut dyn InterruptCheck,
) -> Result<PreparedComponentInference> {
    let options = options.validate()?;
    if variance.len() != problem.outcome.len() {
        return Err(invalid(
            "oracle variance vector does not match the retained observation count",
        ));
    }
    let persistent_bytes = byte_count(variance.len(), "oracle variance vector bytes")?;
    let mut owned_variance = Vec::with_capacity(variance.len());
    for row in 0..variance.len() {
        checkpoint_chunk(interrupt, row, "component_inference_prepare")?;
        if problem.frequency[row] != 1 {
            return Err(BackendError::new(
                ErrorCode::UnsupportedFeature,
                "component_inference_prepare",
                "the internal component-inference MVP requires unit frequency weights",
            ));
        }
        if !variance[row].is_finite() || variance[row] <= 0.0 {
            return Err(invalid(
                "oracle observation variances must be strictly positive and finite",
            ));
        }
        owned_variance.push(variance[row]);
    }
    Ok(PreparedComponentInference {
        schema_version: COMPONENT_INFERENCE_SCHEMA_VERSION,
        inference_unit: ComponentInferenceUnit::Observation,
        variance_source: ComponentVarianceSource::Oracle,
        variance: owned_variance,
        options,
        structured_options: StructuredVarianceOptions::default(),
        persistent_bytes,
        individual_intervals: false,
        residual_moments: None,
        design_only_order: false,
        unified_variance_fit: false,
    })
}

/// Prepare an internally fitted, common observation-level variance model.
/// The fit is deferred until the attached JLA generation has produced its
/// leave-one-observation proxy and target diagonals.  This changes only the
/// component covariance attachment, never the component point estimator.
pub fn prepare_structured_component_inference(
    problem: &CompressedProblem,
    variance_source: ComponentVarianceSource,
    options: ComponentInferenceOptions,
    structured_options: StructuredVarianceOptions,
) -> Result<PreparedComponentInference> {
    prepare_structured_component_inference_with_interrupt(
        problem,
        variance_source,
        options,
        structured_options,
        &mut NeverInterrupt,
    )
}

pub fn prepare_structured_component_inference_with_interrupt(
    problem: &CompressedProblem,
    variance_source: ComponentVarianceSource,
    options: ComponentInferenceOptions,
    structured_options: StructuredVarianceOptions,
    interrupt: &mut dyn InterruptCheck,
) -> Result<PreparedComponentInference> {
    let options = options.validate()?;
    let structured_options = structured_options.validate()?;
    if variance_source.structured_model().is_none() {
        return Err(invalid(
            "structured component inference requires a structured variance source",
        ));
    }
    for row in 0..problem.outcome.len() {
        checkpoint_chunk(interrupt, row, "component_inference_prepare")?;
        if problem.frequency[row] != 1 {
            return Err(BackendError::new(
                ErrorCode::UnsupportedFeature,
                "component_inference_prepare",
                "structured component inference requires unit frequency weights",
            ));
        }
    }
    Ok(PreparedComponentInference {
        schema_version: COMPONENT_INFERENCE_SCHEMA_VERSION,
        inference_unit: ComponentInferenceUnit::Observation,
        variance_source,
        variance: Vec::new(),
        options,
        structured_options,
        persistent_bytes: 0,
        individual_intervals: false,
        residual_moments: None,
        design_only_order: false,
        unified_variance_fit: false,
    })
}

/// Prepare an internal fixed-offset match attachment. The public Stata
/// capability registry does not expose this constructor. Positive integer
/// frequency weights remain algebraic regression mass; the attachment will
/// create exactly one inferential row per declared match.
pub fn prepare_grouped_structured_component_inference(
    problem: &CompressedProblem,
    variance_source: ComponentVarianceSource,
    options: ComponentInferenceOptions,
    structured_options: StructuredVarianceOptions,
) -> Result<PreparedComponentInference> {
    let options = options.validate()?;
    let structured_options = structured_options.validate()?;
    if variance_source.structured_model().is_none() {
        return Err(invalid(
            "grouped structured component inference requires a structured variance source",
        ));
    }
    if problem.deletion_units() == 0 {
        return Err(invalid(
            "grouped component inference requires at least one declared match",
        ));
    }
    Ok(PreparedComponentInference {
        schema_version: COMPONENT_INFERENCE_SCHEMA_VERSION,
        inference_unit: ComponentInferenceUnit::Match,
        variance_source,
        variance: Vec::new(),
        options,
        structured_options,
        persistent_bytes: 0,
        individual_intervals: false,
        residual_moments: None,
        design_only_order: false,
        unified_variance_fit: false,
    })
}

/// Prepare an internal aggregate-match oracle variance vector. This is a test
/// and development boundary, not a public variance-model option.
pub fn prepare_grouped_oracle_component_inference(
    problem: &CompressedProblem,
    variance: &[f64],
    options: ComponentInferenceOptions,
) -> Result<PreparedComponentInference> {
    let options = options.validate()?;
    if variance.len() != problem.deletion_units()
        || variance
            .iter()
            .any(|value| !value.is_finite() || *value <= 0.0)
    {
        return Err(invalid(
            "grouped oracle variances must contain one positive finite value per declared match",
        ));
    }
    Ok(PreparedComponentInference {
        schema_version: COMPONENT_INFERENCE_SCHEMA_VERSION,
        inference_unit: ComponentInferenceUnit::Match,
        variance_source: ComponentVarianceSource::Oracle,
        variance: variance.to_vec(),
        options,
        structured_options: StructuredVarianceOptions::default(),
        persistent_bytes: byte_count(variance.len(), "grouped oracle variance bytes")?,
        individual_intervals: false,
        residual_moments: None,
        design_only_order: false,
        unified_variance_fit: false,
    })
}

/// The row-level ratio used in the zero-diagonal kernel,
/// `r[t,i] = B[t,i,i] * maker_inverse[i]`.  `maker_inverse` is the same
/// finite-projection inverse multiplier used by the attached JLA point
/// correction, which preserves the point-estimator identity exactly for the
/// realized sketches.
pub fn target_ratios(
    target_diagonal: &[Vec<f64>; PRIMITIVE_TARGETS],
    maker_inverse: &[f64],
) -> Result<[Vec<f64>; PRIMITIVE_TARGETS]> {
    let rows = maker_inverse.len();
    if target_diagonal.iter().any(|value| value.len() != rows)
        || maker_inverse
            .iter()
            .any(|value| !value.is_finite() || *value <= 0.0)
    {
        return Err(invalid(
            "target diagonals and maker inverses have incompatible or invalid values",
        ));
    }
    let mut output = core::array::from_fn(|_| vec![0.0; rows]);
    for target in 0..PRIMITIVE_TARGETS {
        for row in 0..rows {
            let value = target_diagonal[target][row] * maker_inverse[row];
            if !value.is_finite() {
                return Err(nonfinite("component target ratio is nonfinite"));
            }
            output[target][row] = value;
        }
    }
    Ok(output)
}

/// Apply one primitive target to a fitted coefficient vector without
/// materializing `Q_t`.  The returned arrays are compatible full-system
/// right-hand sides and controls are identically zero.
pub fn primitive_target_rhs(
    problem: &CompressedProblem,
    coefficients: &ModelCoefficients,
    target: usize,
    interrupt: &mut dyn InterruptCheck,
) -> Result<(Vec<f64>, Vec<f64>, Vec<f64>)> {
    if target >= PRIMITIVE_TARGETS
        || coefficients.worker.len() != problem.workers()
        || coefficients.firm.len() != problem.firms()
        || (!coefficients.control.is_empty()
            && coefficients.control.len() != problem.controls.len())
    {
        return Err(invalid("primitive target RHS dimensions are invalid"));
    }
    let (worker_mean, firm_mean) = coefficient_means(problem, coefficients, interrupt)?;
    let mut worker = zeroed_f64_with_interrupt(
        problem.workers(),
        "primitive worker RHS",
        interrupt,
        "component_inference_target_rhs",
    )?;
    let mut firm = zeroed_f64_with_interrupt(
        problem.firms(),
        "primitive firm RHS",
        interrupt,
        "component_inference_target_rhs",
    )?;
    for cell in 0..problem.cells() {
        checkpoint_chunk(interrupt, cell, "component_inference_target_rhs")?;
        let worker_index = problem.cell_worker[cell] as usize;
        let firm_index = problem.cell_firm[cell] as usize;
        let mass = problem.cell_target_sum[cell] / problem.target_total;
        let worker_value = coefficients.worker[worker_index] - worker_mean;
        let firm_value = coefficients.firm[firm_index] - firm_mean;
        match target {
            0 => worker[worker_index] += mass * worker_value,
            1 => firm[firm_index] += mass * firm_value,
            2 => {
                worker[worker_index] += 0.5 * mass * firm_value;
                firm[firm_index] += 0.5 * mass * worker_value;
            }
            _ => unreachable!("primitive target index checked above"),
        }
    }
    if worker.iter().chain(&firm).any(|value| !value.is_finite()) {
        return Err(nonfinite("primitive target RHS is nonfinite"));
    }
    Ok((
        worker,
        firm,
        zeroed_f64_with_interrupt(
            problem.controls.len(),
            "primitive control RHS",
            interrupt,
            "component_inference_target_rhs",
        )?,
    ))
}

/// Apply one of the four reported targets. The fourth action is assembled
/// only through the exact worker + firm + 2 covariance map.
pub fn reported_target_rhs(
    problem: &CompressedProblem,
    coefficients: &ModelCoefficients,
    target: usize,
    interrupt: &mut dyn InterruptCheck,
) -> Result<(Vec<f64>, Vec<f64>, Vec<f64>)> {
    if target < PRIMITIVE_TARGETS {
        return primitive_target_rhs(problem, coefficients, target, interrupt);
    }
    if target != REPORTED_TARGETS - 1 {
        return Err(invalid("reported component target index is invalid"));
    }
    let mut worker = primitive_target_rhs(problem, coefficients, 0, interrupt)?;
    let firm = primitive_target_rhs(problem, coefficients, 1, interrupt)?;
    let covariance = primitive_target_rhs(problem, coefficients, 2, interrupt)?;
    let mut combine = |left: &mut [f64], middle: &[f64], right: &[f64]| -> Result<()> {
        for (index, ((left, &middle), &right)) in left.iter_mut().zip(middle).zip(right).enumerate()
        {
            checkpoint_chunk(interrupt, index, "component_inference_target_rhs")?;
            *left = *left + middle + 2.0 * right;
        }
        Ok(())
    };
    combine(&mut worker.0, &firm.0, &covariance.0)?;
    combine(&mut worker.1, &firm.1, &covariance.1)?;
    combine(&mut worker.2, &firm.2, &covariance.2)?;
    Ok(worker)
}

/// Evaluate all three primitive quadratic targets for a coefficient vector.
pub fn primitive_plugins(
    problem: &CompressedProblem,
    coefficients: &ModelCoefficients,
    interrupt: &mut dyn InterruptCheck,
) -> Result<[f64; PRIMITIVE_TARGETS]> {
    if coefficients.worker.len() != problem.workers()
        || coefficients.firm.len() != problem.firms()
        || (!coefficients.control.is_empty()
            && coefficients.control.len() != problem.controls.len())
    {
        return Err(invalid(
            "primitive plugin coefficient dimensions are invalid",
        ));
    }
    let (worker_mean, firm_mean) = coefficient_means(problem, coefficients, interrupt)?;
    let mut worker = StableAccumulator::default();
    let mut firm = StableAccumulator::default();
    let mut covariance = StableAccumulator::default();
    for cell in 0..problem.cells() {
        checkpoint_chunk(interrupt, cell, "component_inference_plugin")?;
        let mass = problem.cell_target_sum[cell] / problem.target_total;
        let worker_value = coefficients.worker[problem.cell_worker[cell] as usize] - worker_mean;
        let firm_value = coefficients.firm[problem.cell_firm[cell] as usize] - firm_mean;
        worker.add(mass * worker_value * worker_value);
        firm.add(mass * firm_value * firm_value);
        covariance.add(mass * worker_value * firm_value);
    }
    let output = [worker.finish(), firm.finish(), covariance.finish()];
    if output.iter().any(|value| !value.is_finite()) {
        return Err(nonfinite("primitive plugin target is nonfinite"));
    }
    Ok(output)
}

fn coefficient_means(
    problem: &CompressedProblem,
    coefficients: &ModelCoefficients,
    interrupt: &mut dyn InterruptCheck,
) -> Result<(f64, f64)> {
    let mut worker = StableAccumulator::default();
    let mut firm = StableAccumulator::default();
    for cell in 0..problem.cells() {
        checkpoint_chunk(interrupt, cell, "component_inference_target_mean")?;
        let mass = problem.cell_target_sum[cell];
        worker.add(mass * coefficients.worker[problem.cell_worker[cell] as usize]);
        firm.add(mass * coefficients.firm[problem.cell_firm[cell] as usize]);
    }
    let worker = worker.finish() / problem.target_total;
    let firm = firm.finish() / problem.target_total;
    if !worker.is_finite() || !firm.is_finite() {
        return Err(nonfinite("component coefficient mean is nonfinite"));
    }
    Ok((worker, firm))
}

/// Finish `g_t=C_t y` after the one combined target-specific inverse action.
/// The supplied prediction is
/// `X H^{-1} {Q_t beta + .5 X' R_t y}`.
pub fn finish_influence(
    combined_prediction: &[f64],
    ratio: &[f64],
    outcome: &[f64],
    residual: &[f64],
) -> Result<Vec<f64>> {
    let rows = outcome.len();
    if combined_prediction.len() != rows || ratio.len() != rows || residual.len() != rows {
        return Err(invalid(
            "component influence inputs have incompatible dimensions",
        ));
    }
    let mut output = vec![0.0; rows];
    for row in 0..rows {
        let value = combined_prediction[row] - 0.5 * ratio[row] * (outcome[row] + residual[row]);
        if !value.is_finite() {
            return Err(nonfinite("component influence value is nonfinite"));
        }
        output[row] = value;
    }
    Ok(output)
}

/// Scalar identity used after one common pseudo-outcome solve:
/// `q_t = u'Q_tu - e'R_tz = z'C_tz`.
pub fn probe_scalar(plugin: f64, pseudo_residual: &[f64], ratio: &[f64], z: &[f64]) -> Result<f64> {
    if pseudo_residual.len() != ratio.len() || ratio.len() != z.len() {
        return Err(invalid(
            "component covariance-probe inputs have incompatible dimensions",
        ));
    }
    let mut correction = StableAccumulator::default();
    for row in 0..z.len() {
        correction.add(pseudo_residual[row] * ratio[row] * z[row]);
    }
    let output = plugin - correction.finish();
    if !output.is_finite() {
        return Err(nonfinite("component covariance-probe scalar is nonfinite"));
    }
    Ok(output)
}

pub fn q1_remainder_ratio(
    target_diagonal: &[f64],
    maker_inverse: &[f64],
    mode: &[f64],
    eigenvalue: f64,
) -> Result<Vec<f64>> {
    if target_diagonal.len() != maker_inverse.len()
        || target_diagonal.len() != mode.len()
        || !eigenvalue.is_finite()
    {
        return Err(invalid("q=1 remainder-ratio inputs are invalid"));
    }
    target_diagonal
        .iter()
        .zip(maker_inverse)
        .zip(mode)
        .map(|((&diagonal, &maker_inverse), &mode)| {
            let value = (diagonal - eigenvalue * mode * mode) * maker_inverse;
            if value.is_finite() && maker_inverse > 0.0 {
                Ok(value)
            } else {
                Err(nonfinite("q=1 remainder ratio is nonfinite"))
            }
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
pub fn finish_q1_influence(
    combined_prediction: &[f64],
    ratio: &[f64],
    outcome: &[f64],
    residual: &[f64],
    mode: &[f64],
    eigenvalue: f64,
    leading_score: f64,
) -> Result<Vec<f64>> {
    if mode.len() != outcome.len() || !eigenvalue.is_finite() || !leading_score.is_finite() {
        return Err(invalid("q=1 influence inputs are invalid"));
    }
    let mut output = finish_influence(combined_prediction, ratio, outcome, residual)?;
    for row in 0..output.len() {
        output[row] -= eigenvalue * mode[row] * leading_score;
        if !output[row].is_finite() {
            return Err(nonfinite("q=1 remainder influence is nonfinite"));
        }
    }
    Ok(output)
}

#[allow(clippy::too_many_arguments)]
pub fn q1_probe_scalar(
    plugin: f64,
    pseudo_residual: &[f64],
    ratio: &[f64],
    z: &[f64],
    mode: &[f64],
    eigenvalue: f64,
) -> Result<f64> {
    if mode.len() != z.len() || !eigenvalue.is_finite() {
        return Err(invalid("q=1 remainder-probe inputs are invalid"));
    }
    let mut score = StableAccumulator::default();
    for row in 0..z.len() {
        score.add(mode[row] * z[row]);
    }
    let score = score.finish();
    probe_scalar(
        plugin - eigenvalue * score * score,
        pseudo_residual,
        ratio,
        z,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn finish_q1_target(
    point_estimate: f64,
    leading_score: f64,
    leading_variance_correction: f64,
    direct_remainder_estimate: f64,
    remainder_identity_tolerance: f64,
    eigenvalue: f64,
    mode: &[f64],
    influence: &[f64],
    variance: &[f64],
    remainder_trace_variance: f64,
    remainder_trace_mcse: f64,
    psd_tolerance: f64,
) -> Result<ComponentQ1TargetResult> {
    if mode.len() != variance.len()
        || influence.len() != variance.len()
        || [
            point_estimate,
            leading_score,
            leading_variance_correction,
            direct_remainder_estimate,
            remainder_identity_tolerance,
            eigenvalue,
            remainder_trace_variance,
            remainder_trace_mcse,
            psd_tolerance,
        ]
        .iter()
        .any(|value| !value.is_finite())
        || remainder_trace_variance < 0.0
        || remainder_trace_mcse < 0.0
        || psd_tolerance < 0.0
        || remainder_identity_tolerance < 0.0
    {
        return Err(invalid("q=1 target covariance inputs are invalid"));
    }
    let mut leading_variance = StableAccumulator::default();
    let mut influence_variance = StableAccumulator::default();
    let mut covariance = StableAccumulator::default();
    let mut maximum_influence_contribution = 0.0_f64;
    for row in 0..variance.len() {
        if !variance[row].is_finite() || variance[row] <= 0.0 {
            return Err(invalid("q=1 target variance vector is invalid"));
        }
        leading_variance.add(mode[row] * mode[row] * variance[row]);
        let contribution = 4.0 * influence[row] * influence[row] * variance[row];
        influence_variance.add(contribution);
        maximum_influence_contribution = maximum_influence_contribution.max(contribution);
        covariance.add(2.0 * mode[row] * variance[row] * influence[row]);
    }
    let leading_variance = leading_variance.finish();
    let linear_influence_variance = influence_variance.finish();
    let remainder_variance = linear_influence_variance - remainder_trace_variance;
    let covariance = covariance.finish();
    // The two coordinates have different units (outcome and outcome squared).
    // Certify their correlation matrix rather than compare unscaled entries.
    let correlation = covariance / leading_variance.sqrt() / remainder_variance.sqrt();
    let standardized_determinant = (1.0 - correlation.abs()) * (1.0 + correlation.abs());
    let status = if leading_variance <= 0.0 || remainder_variance <= 0.0 {
        ComponentQ1Status::NonpositiveVariance
    } else if !standardized_determinant.is_finite() || standardized_determinant <= psd_tolerance {
        ComponentQ1Status::SingularCovariance
    } else {
        ComponentQ1Status::Computed
    };
    let conditional_remainder_variance = remainder_variance * standardized_determinant;
    let curvature = if status == ComponentQ1Status::Computed {
        2.0 * eigenvalue.abs() * leading_variance / conditional_remainder_variance.sqrt()
    } else {
        f64::NAN
    };
    let leading_recentered_component =
        eigenvalue * (leading_score * leading_score - leading_variance_correction);
    let remainder_estimate = point_estimate - leading_recentered_component;
    let remainder_identity_error = (remainder_estimate - direct_remainder_estimate).abs();
    let identity_scale = point_estimate
        .abs()
        .max(remainder_estimate.abs())
        .max(direct_remainder_estimate.abs())
        .max(1.0);
    if remainder_identity_error > remainder_identity_tolerance * identity_scale {
        return Err(BackendError::new(
            ErrorCode::TargetIdentityFailed,
            "component_inference_q1",
            "the q=1 leave-out recentering does not reproduce the rank-one-subtracted target",
        ));
    }
    let output = ComponentQ1TargetResult {
        status,
        point_estimate,
        leading_score,
        leading_variance_correction,
        leading_variance,
        leading_recentered_component,
        remainder_estimate,
        remainder_identity_error,
        leading_remainder_covariance: covariance,
        remainder_variance,
        remainder_trace_mcse,
        standardized_determinant,
        remainder_influence_variance: linear_influence_variance,
        remainder_trace_variance,
        curvature,
        critical_value: f64::NAN,
        critical_draws: 0,
        confidence_lower: f64::NAN,
        confidence_upper: f64::NAN,
        leading_f_statistic: if leading_variance > 0.0 {
            leading_score * leading_score / leading_variance
        } else {
            f64::NAN
        },
        remainder_influence_concentration: if linear_influence_variance > 0.0 {
            maximum_influence_contribution / linear_influence_variance
        } else {
            0.0
        },
    };
    if [
        output.leading_recentered_component,
        output.remainder_estimate,
        output.remainder_identity_error,
        output.leading_variance,
        output.remainder_variance,
        output.leading_remainder_covariance,
        output.remainder_influence_concentration,
    ]
    .iter()
    .any(|value| !value.is_finite())
    {
        return Err(nonfinite("q=1 target result is nonfinite"));
    }
    Ok(output)
}

fn component_standard_normal(
    rng: CounterRng,
    domain: ProbeDomain,
    probe: u64,
    entity: u64,
    first_word: u64,
) -> f64 {
    let first = rng.word(domain, probe, entity, first_word);
    let second = rng.word(domain, probe, entity, first_word + 1);
    let u1 = (((first >> 11) as f64) + 0.5) * (1.0 / 9_007_199_254_740_992.0);
    let u2 = (((second >> 11) as f64) + 0.5) * (1.0 / 9_007_199_254_740_992.0);
    (-2.0 * u1.ln()).sqrt() * (core::f64::consts::TAU * u2).cos()
}

/// Simulated Andrews--Mikusheva radius for the q=1 curvature. Draws have a
/// separate Counter-V1 domain and are addressed by target and simulation.
pub fn q1_critical_value(
    seed: u64,
    target: usize,
    curvature: f64,
    confidence_level: f64,
    simulations: u32,
) -> Result<f64> {
    if target >= REPORTED_TARGETS
        || !curvature.is_finite()
        || curvature < 0.0
        || !confidence_level.is_finite()
        || confidence_level <= 0.0
        || confidence_level >= 1.0
        || simulations < 1_000
    {
        return Err(invalid("q=1 critical-value inputs are invalid"));
    }
    let rng = CounterRng::new(seed);
    let mut distance = Vec::with_capacity(simulations as usize);
    for simulation in 0..u64::from(simulations) {
        let first = component_standard_normal(
            rng,
            ProbeDomain::ComponentInferenceCritical,
            simulation,
            target as u64,
            0,
        )
        .abs();
        let second = component_standard_normal(
            rng,
            ProbeDomain::ComponentInferenceCritical,
            simulation,
            target as u64,
            2,
        )
        .abs();
        let value = if curvature <= 1.0e-10 {
            first
        } else {
            let inverse = curvature.recip();
            (first * first + second * second + 2.0 * first * inverse)
                / ((second * second + (first + inverse) * (first + inverse)).sqrt() + inverse)
        };
        if !value.is_finite() || value < 0.0 {
            return Err(nonfinite("q=1 critical-value draw is nonfinite"));
        }
        distance.push(value);
    }
    distance.sort_by(f64::total_cmp);
    let index = ((confidence_level * f64::from(simulations)).ceil() as usize)
        .clamp(1, simulations as usize)
        - 1;
    let critical = distance[index];
    if !critical.is_finite() || critical <= 0.0 {
        return Err(nonfinite("q=1 critical value is invalid"));
    }
    Ok(critical)
}

fn q1_objective(angle: f64, center: [f64; 2], root: [f64; 4], radius: f64, eigenvalue: f64) -> f64 {
    let cosine = angle.cos();
    let sine = angle.sin();
    let leading = center[0] + radius * root[0] * cosine;
    let remainder = center[1] + radius * (root[2] * cosine + root[3] * sine);
    eigenvalue * leading * leading + remainder
}

fn q1_refine(
    mut left: f64,
    mut right: f64,
    center: [f64; 2],
    root: [f64; 4],
    radius: f64,
    eigenvalue: f64,
    maximize: bool,
) -> f64 {
    let golden = (5.0_f64.sqrt() - 1.0) / 2.0;
    let mut first = right - golden * (right - left);
    let mut second = left + golden * (right - left);
    for _ in 0..80 {
        let first_value = q1_objective(first, center, root, radius, eigenvalue);
        let second_value = q1_objective(second, center, root, radius, eigenvalue);
        if (maximize && first_value < second_value) || (!maximize && first_value > second_value) {
            left = first;
            first = second;
            second = left + golden * (right - left);
        } else {
            right = second;
            second = first;
            first = right - golden * (right - left);
        }
    }
    0.5 * (left + right)
}

pub fn q1_am_interval(
    center: [f64; 2],
    covariance: [f64; 4],
    radius: f64,
    eigenvalue: f64,
) -> Result<[f64; 2]> {
    if center
        .iter()
        .chain(&covariance)
        .chain([radius, eigenvalue].iter())
        .any(|value| !value.is_finite())
        || radius <= 0.0
        || covariance[0] <= 0.0
        || covariance[3] <= 0.0
        || (covariance[1] - covariance[2]).abs()
            > 1.0e-12 * covariance[1].abs().max(covariance[2].abs()).max(1.0)
    {
        return Err(invalid("q=1 confidence-ellipse inputs are invalid"));
    }
    let first = covariance[0].sqrt();
    let cross = covariance[2] / first;
    let conditional = covariance[3] - cross * cross;
    if conditional <= 0.0 || !conditional.is_finite() {
        return Err(BackendError::new(
            ErrorCode::JlaConstraintFailed,
            "component_inference_q1",
            "the q=1 confidence ellipse is singular or indefinite",
        ));
    }
    let root = [first, 0.0, cross, conditional.sqrt()];
    const GRID: usize = 8_192;
    let step = core::f64::consts::TAU / GRID as f64;
    let mut minimum = (f64::INFINITY, 0_usize);
    let mut maximum = (f64::NEG_INFINITY, 0_usize);
    for index in 0..GRID {
        let value = q1_objective(index as f64 * step, center, root, radius, eigenvalue);
        if value < minimum.0 {
            minimum = (value, index);
        }
        if value > maximum.0 {
            maximum = (value, index);
        }
    }
    let lower_angle = q1_refine(
        (minimum.1 as f64 - 1.0) * step,
        (minimum.1 as f64 + 1.0) * step,
        center,
        root,
        radius,
        eigenvalue,
        false,
    );
    let upper_angle = q1_refine(
        (maximum.1 as f64 - 1.0) * step,
        (maximum.1 as f64 + 1.0) * step,
        center,
        root,
        radius,
        eigenvalue,
        true,
    );
    let output = [
        q1_objective(lower_angle, center, root, radius, eigenvalue),
        q1_objective(upper_angle, center, root, radius, eigenvalue),
    ];
    if output.iter().any(|value| !value.is_finite()) || output[0] > output[1] {
        return Err(nonfinite("q=1 confidence interval is invalid"));
    }
    Ok(output)
}

pub fn finish_q1_interval(
    mut target_result: ComponentQ1TargetResult,
    seed: u64,
    target: usize,
    eigenvalue: f64,
    confidence_level: f64,
    simulations: u32,
) -> Result<ComponentQ1TargetResult> {
    if target_result.status != ComponentQ1Status::Computed {
        return Ok(target_result);
    }
    let critical = q1_critical_value(
        seed,
        target,
        target_result.curvature,
        confidence_level,
        simulations,
    )?;
    target_result.critical_draws = simulations;
    let interval = q1_am_interval(
        [
            target_result.leading_score,
            target_result.remainder_estimate,
        ],
        [
            target_result.leading_variance,
            target_result.leading_remainder_covariance,
            target_result.leading_remainder_covariance,
            target_result.remainder_variance,
        ],
        critical,
        eigenvalue,
    );
    let interval = match interval {
        Ok(interval) => interval,
        Err(error)
            if matches!(
                error.code,
                ErrorCode::CorrectionNonFinite | ErrorCode::JlaConstraintFailed
            ) =>
        {
            target_result.status = ComponentQ1Status::IntervalFailure;
            return Ok(target_result);
        }
        Err(error) => return Err(error),
    };
    target_result.critical_value = critical;
    target_result.confidence_lower = interval[0];
    target_result.confidence_upper = interval[1];
    Ok(target_result)
}

/// Generate one schedule- and batch-addressed Gaussian pseudo-outcome.  The
/// Box--Muller transform belongs only to the new component-inference domain;
/// existing Counter-V1 atoms and their permanent vectors are unchanged.
pub fn fill_gaussian_pseudo_outcome(
    rng: CounterRng,
    probe: u64,
    entity: &[u64],
    subdraw: &[u64],
    variance: &[f64],
    output: &mut [f64],
    interrupt: &mut dyn InterruptCheck,
) -> Result<()> {
    if entity.len() != variance.len()
        || subdraw.len() != variance.len()
        || output.len() != variance.len()
    {
        return Err(invalid("component Gaussian address dimensions disagree"));
    }
    for row in 0..output.len() {
        checkpoint_chunk(interrupt, row, "component_inference_gaussian")?;
        let first_word = subdraw[row]
            .checked_mul(2)
            .ok_or_else(|| resource("component Gaussian word index overflow"))?;
        let second_word = first_word
            .checked_add(1)
            .ok_or_else(|| resource("component Gaussian word index overflow"))?;
        let first = rng.word(
            ProbeDomain::ComponentInference,
            probe,
            entity[row],
            first_word,
        );
        let second = rng.word(
            ProbeDomain::ComponentInference,
            probe,
            entity[row],
            second_word,
        );
        // Midpoints of the 53-bit cells keep both uniforms strictly inside
        // (0,1), so log(0) cannot enter the numerical trace calculation.
        let u1 = (((first >> 11) as f64) + 0.5) * (1.0 / 9_007_199_254_740_992.0);
        let u2 = (((second >> 11) as f64) + 0.5) * (1.0 / 9_007_199_254_740_992.0);
        let standard = (-2.0 * u1.ln()).sqrt() * (core::f64::consts::TAU * u2).cos();
        let value = variance[row].sqrt() * standard;
        if !value.is_finite() || !variance[row].is_finite() || variance[row] <= 0.0 {
            return Err(nonfinite("component Gaussian pseudo-outcome is nonfinite"));
        }
        output[row] = value;
    }
    Ok(())
}

/// Reconcile a randomized trace-square estimate with two explicitly estimated
/// modes and form concentration diagnostics. The six-MCSE check is a
/// numerical consistency gate; it does not accept or reject `q=0` or `q=1`.
#[allow(clippy::too_many_arguments)]
pub fn finish_spectrum_diagnostics(
    leading_eigenvalue: f64,
    second_eigenvalue: f64,
    trace_square_raw: f64,
    trace_square_mcse: f64,
    maximum_mode_weight_squared: f64,
    leading_residual: f64,
    second_residual: f64,
    probes: u32,
    iterations: u32,
    residual_tolerance: f64,
) -> Result<ComponentSpectrumDiagnostics> {
    let values = [
        leading_eigenvalue,
        second_eigenvalue,
        trace_square_raw,
        trace_square_mcse,
        maximum_mode_weight_squared,
        leading_residual,
        second_residual,
        residual_tolerance,
    ];
    if values.iter().any(|value| !value.is_finite())
        || trace_square_raw <= 0.0
        || trace_square_mcse < 0.0
        || !(0.0..=1.0 + 1.0e-12).contains(&maximum_mode_weight_squared)
        || leading_residual < 0.0
        || second_residual < 0.0
        || residual_tolerance <= 0.0
        || probes < 2
        || iterations < 2
    {
        return Err(invalid("component target-spectrum inputs are invalid"));
    }
    if leading_eigenvalue.abs() <= f64::EPSILON.sqrt()
        || second_eigenvalue.abs() > leading_eigenvalue.abs() * (1.0 + residual_tolerance)
        || leading_residual > residual_tolerance
        || second_residual > residual_tolerance
    {
        return Err(BackendError::new(
            ErrorCode::JlaConstraintFailed,
            "component_inference_spectrum",
            format!(
                "the first two generalized target modes did not satisfy the spectral certificate: lambda1={leading_eigenvalue:.6e}, lambda2={second_eigenvalue:.6e}, residual1={leading_residual:.3e}, residual2={second_residual:.3e}, tolerance={residual_tolerance:.3e}"
            ),
        ));
    }
    let two_mode_square =
        leading_eigenvalue.mul_add(leading_eigenvalue, second_eigenvalue * second_eigenvalue);
    if trace_square_raw + 6.0 * trace_square_mcse < two_mode_square {
        return Err(BackendError::new(
            ErrorCode::JlaConstraintFailed,
            "component_inference_spectrum",
            "the randomized trace square is materially below the certified two-mode contribution",
        ));
    }
    let trace_square = trace_square_raw.max(two_mode_square);
    let trace_reconciliation = trace_square - trace_square_raw;
    let leading_square = leading_eigenvalue * leading_eigenvalue;
    let second_square = second_eigenvalue * second_eigenvalue;
    let leading_share = leading_square / trace_square;
    let leading_share_mcse_trace_only =
        leading_square * trace_square_mcse / (trace_square * trace_square);
    let remainder_square = (trace_square - leading_square).max(0.0);
    let remainder_leading_share = if remainder_square > 0.0 {
        (second_square / remainder_square).min(1.0)
    } else if second_square == 0.0 {
        0.0
    } else {
        return Err(BackendError::new(
            ErrorCode::JlaConstraintFailed,
            "component_inference_spectrum",
            "the target spectrum has no numerically valid remainder",
        ));
    };
    let output = ComponentSpectrumDiagnostics {
        certified: true,
        leading_eigenvalue,
        second_eigenvalue,
        trace_square_raw,
        trace_square,
        trace_square_mcse,
        trace_reconciliation,
        leading_share,
        leading_share_mcse_trace_only,
        remainder_leading_share,
        maximum_mode_weight_squared: maximum_mode_weight_squared.min(1.0),
        leading_residual,
        second_residual,
        probes,
        iterations,
    };
    if [
        output.leading_share,
        output.leading_share_mcse_trace_only,
        output.remainder_leading_share,
    ]
    .iter()
    .any(|value| !value.is_finite())
    {
        return Err(nonfinite(
            "component target-spectrum diagnostic is nonfinite",
        ));
    }
    Ok(output)
}

#[derive(Clone, Copy, Debug, Default)]
pub struct JointProbeMoments {
    count: u64,
    marginal: [StableAccumulator; PRIMITIVE_TARGETS],
    pair: [PairRawMoments; PRIMITIVE_TARGETS * PRIMITIVE_TARGETS],
}

impl JointProbeMoments {
    pub fn push(&mut self, value: [f64; PRIMITIVE_TARGETS]) -> Result<()> {
        if value.iter().any(|entry| !entry.is_finite()) {
            return Err(nonfinite(
                "component joint probe contains a nonfinite scalar",
            ));
        }
        self.count = self
            .count
            .checked_add(1)
            .ok_or_else(|| resource("component probe count overflow"))?;
        for target in 0..PRIMITIVE_TARGETS {
            self.marginal[target].add(value[target]);
        }
        for left in 0..PRIMITIVE_TARGETS {
            for right in 0..PRIMITIVE_TARGETS {
                self.pair[left * PRIMITIVE_TARGETS + right].add(value[left], value[right]);
            }
        }
        Ok(())
    }

    pub fn finish(self) -> Result<ProbeMomentResult> {
        if self.count < 2 {
            return Err(invalid(
                "at least two component covariance probes are required",
            ));
        }
        let n = self.count as f64;
        let mean = core::array::from_fn(|target| self.marginal[target].finish() / n);
        let mut covariance = [0.0; PRIMITIVE_TARGETS * PRIMITIVE_TARGETS];
        let mut mcse = [0.0; PRIMITIVE_TARGETS * PRIMITIVE_TARGETS];
        for left in 0..PRIMITIVE_TARGETS {
            for right in left..PRIMITIVE_TARGETS {
                let index = left * PRIMITIVE_TARGETS + right;
                let reverse = right * PRIMITIVE_TARGETS + left;
                let pair = self.pair[index].finish(n, mean[left], mean[right])?;
                covariance[index] = pair.covariance;
                covariance[reverse] = pair.covariance;
                mcse[index] = pair.mcse;
                mcse[reverse] = pair.mcse;
            }
        }
        Ok(ProbeMomentResult {
            mean,
            covariance,
            mcse,
            probes: u32::try_from(self.count)
                .map_err(|_| resource("component probe count is not representable as u32"))?,
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ProbeMomentResult {
    pub mean: [f64; PRIMITIVE_TARGETS],
    pub covariance: [f64; PRIMITIVE_TARGETS * PRIMITIVE_TARGETS],
    pub mcse: [f64; PRIMITIVE_TARGETS * PRIMITIVE_TARGETS],
    pub probes: u32,
}

/// Combine the deterministic influence term and online Gaussian trace term,
/// enforce the registered material-PSD boundary, and derive the fourth target
/// only through its exact linear map.
pub fn finish_component_covariance(
    influence: &[Vec<f64>; PRIMITIVE_TARGETS],
    variance: &[f64],
    probes: ProbeMomentResult,
    psd_tolerance: f64,
    interrupt: &mut dyn InterruptCheck,
) -> Result<ComponentInferenceResult> {
    finish_component_covariance_with_reporting(
        influence,
        variance,
        probes,
        psd_tolerance,
        false,
        interrupt,
    )
}

/// Individual intervals do not require the entire estimated covariance to be
/// PSD. Material indefiniteness is retained as a separate status, never clipped.
#[allow(clippy::too_many_lines)]
pub fn finish_component_covariance_with_reporting(
    influence: &[Vec<f64>; PRIMITIVE_TARGETS],
    variance: &[f64],
    probes: ProbeMomentResult,
    psd_tolerance: f64,
    individual_intervals: bool,
    interrupt: &mut dyn InterruptCheck,
) -> Result<ComponentInferenceResult> {
    let rows = variance.len();
    if influence.iter().any(|value| value.len() != rows)
        || variance
            .iter()
            .any(|value| !value.is_finite() || *value <= 0.0)
        || !psd_tolerance.is_finite()
        || psd_tolerance < 0.0
    {
        return Err(invalid("component covariance inputs are invalid"));
    }
    let mut influence_term = [0.0; PRIMITIVE_TARGETS * PRIMITIVE_TARGETS];
    for left in 0..PRIMITIVE_TARGETS {
        for right in left..PRIMITIVE_TARGETS {
            let mut value = StableAccumulator::default();
            for row in 0..rows {
                checkpoint_chunk(interrupt, row, "component_inference_influence_covariance")?;
                value.add(4.0 * influence[left][row] * variance[row] * influence[right][row]);
            }
            let value = value.finish();
            if !value.is_finite() {
                return Err(nonfinite("component influence covariance is nonfinite"));
            }
            influence_term[left * PRIMITIVE_TARGETS + right] = value;
            influence_term[right * PRIMITIVE_TARGETS + left] = value;
        }
    }
    let mut primitive = [0.0; PRIMITIVE_TARGETS * PRIMITIVE_TARGETS];
    for index in 0..primitive.len() {
        primitive[index] = influence_term[index] - probes.covariance[index];
        if !primitive[index].is_finite() {
            return Err(nonfinite("component primitive covariance is nonfinite"));
        }
    }
    let spectrum = symmetric_eigen_extremes(
        &primitive,
        PRIMITIVE_TARGETS,
        interrupt,
        "component_inference_psd",
    )?;
    let scale = (0..PRIMITIVE_TARGETS)
        .map(|index| primitive[index * PRIMITIVE_TARGETS + index].abs())
        .fold(1.0e-30_f64, f64::max);
    let joint_status = if (0..PRIMITIVE_TARGETS).any(|index| {
        !primitive[index * PRIMITIVE_TARGETS + index].is_finite()
            || primitive[index * PRIMITIVE_TARGETS + index] <= 0.0
    }) {
        ComponentJointStatus::NonpositiveDiagonal
    } else if spectrum.smallest_lower < -psd_tolerance * scale {
        ComponentJointStatus::Indefinite
    } else {
        ComponentJointStatus::Computed
    };
    if joint_status != ComponentJointStatus::Computed && !individual_intervals {
        return Err(BackendError::new(
            ErrorCode::JlaConstraintFailed,
            "component_inference_psd",
            "the primitive component covariance is materially indefinite or singular",
        ));
    }
    let psd_cleanup = if joint_status == ComponentJointStatus::Computed {
        (-spectrum.smallest_lower).max(0.0)
    } else {
        0.0
    };
    if psd_cleanup > 0.0 {
        for target in 0..PRIMITIVE_TARGETS {
            primitive[target * PRIMITIVE_TARGETS + target] += psd_cleanup;
        }
    }
    let covariance = map_primitive_covariance(&primitive)?;
    let mut q0_status = core::array::from_fn(|target| {
        if covariance[target * REPORTED_TARGETS + target] > 0.0 {
            ComponentQ0Status::Computed
        } else {
            ComponentQ0Status::NonpositiveVariance
        }
    });
    let mut influence_concentration = [0.0; REPORTED_TARGETS];
    for target in 0..REPORTED_TARGETS {
        let mut total = StableAccumulator::default();
        let mut maximum = 0.0_f64;
        for row in 0..rows {
            let value = if target < PRIMITIVE_TARGETS {
                influence[target][row]
            } else {
                influence[0][row] + influence[1][row] + 2.0 * influence[2][row]
            };
            let contribution = 4.0 * value * value * variance[row];
            total.add(contribution);
            maximum = maximum.max(contribution);
        }
        let total = total.finish();
        if !total.is_finite() {
            return Err(nonfinite(
                "component linear-influence variance is nonfinite",
            ));
        }
        if total <= 0.0 {
            if individual_intervals {
                q0_status[target] = ComponentQ0Status::NoLinearInfluence;
                continue;
            }
            return Err(BackendError::new(
                ErrorCode::JlaConstraintFailed,
                "component_inference_influence_concentration",
                "a component target has no positive linear-influence variance",
            ));
        }
        influence_concentration[target] = maximum / total;
    }
    Ok(ComponentInferenceResult {
        schema_version: COMPONENT_INFERENCE_SCHEMA_VERSION,
        joint_status,
        q0_status,
        inference_unit: ComponentInferenceUnit::Observation,
        independent_units: u64::try_from(rows)
            .map_err(|_| resource("component inference unit count"))?,
        nuisance_uncertainty_conditioned_away: false,
        effective_match_count: 0.0,
        largest_match_mass_share: 0.0,
        largest_match_leverage: 0.0,
        smallest_maker_denominator: 0.0,
        variance_source: ComponentVarianceSource::Oracle,
        structured_variance: None,
        residual_moments: None,
        primitive_covariance: primitive,
        covariance,
        influence_term,
        influence_concentration,
        trace_term: probes.covariance,
        trace_mcse: probes.mcse,
        probe_mean: probes.mean,
        probes: probes.probes,
        psd_cleanup,
        smallest_eigenvalue_before_cleanup: spectrum.smallest_lower,
        largest_eigenvalue_before_cleanup: spectrum.largest_upper,
        solve_receipts: Vec::new(),
        maximum_iterations: 0,
        maximum_reduced_residual: 0.0,
        maximum_complete_residual: 0.0,
        full_residual_tolerance: 0.0,
        peak_forecast_bytes: 0,
        leverage: Vec::new(),
        maker_inverse: Vec::new(),
        target_diagonal: core::array::from_fn(|_| Vec::new()),
        influence: core::array::from_fn(|_| Vec::new()),
        point_correction_identity_error: 0.0,
        counter_atoms: 0,
        counter_words: 0,
        critical_simulations: 0,
        spectrum: [ComponentSpectrumDiagnostics::default(); REPORTED_TARGETS],
        q1: None,
    })
}

pub fn map_primitive_covariance(
    primitive: &[f64; PRIMITIVE_TARGETS * PRIMITIVE_TARGETS],
) -> Result<[f64; REPORTED_TARGETS * REPORTED_TARGETS]> {
    if primitive.iter().any(|value| !value.is_finite()) {
        return Err(nonfinite("primitive component covariance is nonfinite"));
    }
    let mut output = [0.0; REPORTED_TARGETS * REPORTED_TARGETS];
    for left in 0..PRIMITIVE_TARGETS {
        for right in 0..PRIMITIVE_TARGETS {
            output[left * REPORTED_TARGETS + right] = primitive[left * PRIMITIVE_TARGETS + right];
        }
    }
    for left in 0..PRIMITIVE_TARGETS {
        let value = output[left * REPORTED_TARGETS]
            + output[left * REPORTED_TARGETS + 1]
            + 2.0 * output[left * REPORTED_TARGETS + 2];
        output[left * REPORTED_TARGETS + 3] = value;
        output[3 * REPORTED_TARGETS + left] = value;
    }
    output[3 * REPORTED_TARGETS + 3] =
        output[3] + output[REPORTED_TARGETS + 3] + 2.0 * output[2 * REPORTED_TARGETS + 3];
    if output.iter().any(|value| !value.is_finite()) {
        return Err(nonfinite("mapped component covariance is nonfinite"));
    }
    Ok(output)
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

#[derive(Clone, Copy, Debug, Default)]
struct PairRawMoments {
    xy: StableAccumulator,
    x2: StableAccumulator,
    y2: StableAccumulator,
    x2y: StableAccumulator,
    xy2: StableAccumulator,
    x2y2: StableAccumulator,
}

impl PairRawMoments {
    fn add(&mut self, x: f64, y: f64) {
        let x2 = x * x;
        let y2 = y * y;
        self.xy.add(x * y);
        self.x2.add(x2);
        self.y2.add(y2);
        self.x2y.add(x2 * y);
        self.xy2.add(x * y2);
        self.x2y2.add(x2 * y2);
    }

    fn finish(self, n: f64, mean_x: f64, mean_y: f64) -> Result<PairMomentResult> {
        let exy = self.xy.finish() / n;
        let covariance_population = exy - mean_x * mean_y;
        let covariance = covariance_population * n / (n - 1.0);
        let central_22 = self.x2y2.finish() / n
            - 2.0 * mean_y * self.x2y.finish() / n
            - 2.0 * mean_x * self.xy2.finish() / n
            + mean_y * mean_y * self.x2.finish() / n
            + mean_x * mean_x * self.y2.finish() / n
            + 4.0 * mean_x * mean_y * exy
            - 3.0 * mean_x * mean_x * mean_y * mean_y;
        let influence_variance =
            (central_22 - covariance_population * covariance_population).max(0.0);
        let mcse = (influence_variance / n).sqrt() * n / (n - 1.0);
        if !covariance.is_finite() || !mcse.is_finite() {
            return Err(nonfinite("component online covariance moment is nonfinite"));
        }
        Ok(PairMomentResult { covariance, mcse })
    }
}

#[derive(Clone, Copy, Debug)]
struct PairMomentResult {
    covariance: f64,
    mcse: f64,
}

fn byte_count(values: usize, label: &str) -> Result<u64> {
    u64::try_from(values)
        .ok()
        .and_then(|value| value.checked_mul(core::mem::size_of::<f64>() as u64))
        .ok_or_else(|| resource(format!("{label} overflow")))
}

fn invalid(message: impl Into<String>) -> BackendError {
    BackendError::new(ErrorCode::InvalidInput, "component_inference", message)
}

fn resource(message: impl Into<String>) -> BackendError {
    BackendError::new(ErrorCode::ResourceLimit, "component_inference", message)
}

fn nonfinite(message: impl Into<String>) -> BackendError {
    BackendError::new(
        ErrorCode::CorrectionNonFinite,
        "component_inference",
        message,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dense::invert_scaled_spd;
    use crate::interrupt::NeverInterrupt;

    fn normal_cdf(value: f64) -> f64 {
        // Independent test-only approximation (Abramowitz--Stegun 7.1.26).
        // Its absolute error is below 7.5e-8, which is negligible relative
        // to the registered 100,000-draw critical-value Monte Carlo error.
        let absolute = value.abs();
        let t = 1.0 / (1.0 + 0.231_641_9 * absolute);
        let polynomial = t
            * (0.319_381_530
                + t * (-0.356_563_782
                    + t * (1.781_477_937 + t * (-1.821_255_978 + t * 1.330_274_429))));
        let upper =
            (-0.5 * absolute * absolute).exp() * polynomial / (2.0 * core::f64::consts::PI).sqrt();
        if value >= 0.0 {
            1.0 - upper
        } else {
            upper
        }
    }

    fn integrated_q1_cdf(distance: f64, curvature: f64) -> f64 {
        if distance <= 0.0 {
            return 0.0;
        }
        if curvature <= 1.0e-10 {
            return 2.0 * normal_cdf(distance) - 1.0;
        }
        let inverse = curvature.recip();
        const PANELS: usize = 32_768;
        let width = distance / PANELS as f64;
        let integrand = |x: f64| {
            let y_squared = ((distance - x) * (2.0 * inverse + distance + x)).max(0.0);
            let half_normal_density = (2.0 / core::f64::consts::PI).sqrt() * (-0.5 * x * x).exp();
            let half_normal_cdf = (2.0 * normal_cdf(y_squared.sqrt()) - 1.0).max(0.0);
            half_normal_density * half_normal_cdf
        };
        let mut sum = integrand(0.0) + integrand(distance);
        for panel in 1..PANELS {
            sum += if panel % 2 == 0 { 2.0 } else { 4.0 } * integrand(panel as f64 * width);
        }
        sum * width / 3.0
    }

    fn multiply(
        left: &[f64],
        left_rows: usize,
        inner: usize,
        right: &[f64],
        right_cols: usize,
    ) -> Vec<f64> {
        let mut output = vec![0.0; left_rows * right_cols];
        for row in 0..left_rows {
            for column in 0..right_cols {
                for k in 0..inner {
                    output[row * right_cols + column] +=
                        left[row * inner + k] * right[k * right_cols + column];
                }
            }
        }
        output
    }

    fn transpose(value: &[f64], rows: usize, columns: usize) -> Vec<f64> {
        let mut output = vec![0.0; value.len()];
        for row in 0..rows {
            for column in 0..columns {
                output[column * rows + row] = value[row * columns + column];
            }
        }
        output
    }

    fn matrix_vector(matrix: &[f64], rows: usize, columns: usize, vector: &[f64]) -> Vec<f64> {
        (0..rows)
            .map(|row| {
                (0..columns)
                    .map(|column| matrix[row * columns + column] * vector[column])
                    .sum()
            })
            .collect()
    }

    fn dense_fixture() -> (
        usize,
        usize,
        Vec<f64>,
        Vec<f64>,
        [Vec<f64>; PRIMITIVE_TARGETS],
        Vec<f64>,
    ) {
        let rows = 8;
        let parameters = 4;
        let x = vec![
            1.0, 0.0, 0.0, -1.1, 1.0, 0.0, 1.0, -0.4, 1.0, 1.0, 0.0, 0.2, 1.0, 1.0, 1.0, 0.8, 1.0,
            2.0, 0.0, 1.3, 1.0, 2.0, 1.0, 1.9, 1.0, 3.0, 0.0, 2.6, 1.0, 3.0, 1.0, 3.4,
        ];
        let y = vec![0.7, 1.6, -0.2, 2.7, 1.1, 3.8, 2.4, 5.2];
        let targets = [
            vec![
                0.0, 0.0, 0.0, 0.0, 0.0, 0.20, -0.08, 0.0, 0.0, -0.08, 0.16, 0.0, 0.0, 0.0, 0.0,
                0.0,
            ],
            vec![
                0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.18, 0.0, 0.0, 0.0, 0.0, 0.0,
            ],
            vec![
                0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.06, 0.0, 0.0, 0.06, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
            ],
        ];
        let variance = vec![0.5, 0.8, 1.1, 0.7, 1.4, 0.9, 1.2, 0.6];
        (rows, parameters, x, y, targets, variance)
    }

    #[test]
    fn dense_kernel_combined_influence_and_scalar_identity_hold() {
        let (n, p, x, y, targets, variance) = dense_fixture();
        let xt = transpose(&x, n, p);
        let h = multiply(&xt, p, n, &x, p);
        let inverse = invert_scaled_spd(&h, p, 1.0e-12, &mut NeverInterrupt, "component_test_h")
            .expect("invertible dense information")
            .inverse;
        let sxt = multiply(&inverse, p, p, &xt, n);
        let projection = multiply(&x, n, p, &sxt, n);
        let beta = matrix_vector(&sxt, p, n, &y);
        let fitted = matrix_vector(&projection, n, n, &y);
        let residual = y
            .iter()
            .zip(&fitted)
            .map(|(left, right)| left - right)
            .collect::<Vec<_>>();

        for target in &targets {
            let sq = multiply(&inverse, p, p, target, p);
            let sqs = multiply(&sq, p, p, &inverse, p);
            let b = multiply(&multiply(&x, n, p, &sqs, p), n, p, &xt, n);
            let diagonal = (0..n).map(|row| b[row * n + row]).collect::<Vec<_>>();
            let maker_inverse = (0..n)
                .map(|row| (1.0 - projection[row * n + row]).recip())
                .collect::<Vec<_>>();
            let ratios = target_ratios(
                &[diagonal.clone(), diagonal.clone(), diagonal.clone()],
                &maker_inverse,
            )
            .expect("finite ratios");
            let ratio = &ratios[0];

            let q_beta = matrix_vector(target, p, p, &beta);
            let ry = ratio
                .iter()
                .zip(&y)
                .map(|(left, right)| left * right)
                .collect::<Vec<_>>();
            let xtry = matrix_vector(&xt, p, n, &ry);
            let combined_rhs = q_beta
                .iter()
                .zip(&xtry)
                .map(|(left, right)| left + 0.5 * right)
                .collect::<Vec<_>>();
            let combined_prediction =
                matrix_vector(&multiply(&x, n, p, &inverse, p), n, p, &combined_rhs);
            let influence =
                finish_influence(&combined_prediction, ratio, &y, &residual).expect("influence");

            let mut c = b.clone();
            for row in 0..n {
                for column in 0..n {
                    let maker = f64::from(row == column) - projection[row * n + column];
                    c[row * n + column] -= 0.5 * (ratio[row] * maker + maker * ratio[column]);
                }
            }
            let dense_influence = matrix_vector(&c, n, n, &y);
            for (left, right) in influence.iter().zip(dense_influence) {
                assert!((left - right).abs() < 2.0e-11);
            }
            for row in 0..n {
                assert!(c[row * n + row].abs() < 2.0e-12);
            }
            let plugin: f64 = beta
                .iter()
                .zip(matrix_vector(target, p, p, &beta))
                .map(|(left, right)| left * right)
                .sum();
            let correction: f64 = (0..n)
                .map(|row| diagonal[row] * y[row] * residual[row] * maker_inverse[row])
                .sum();
            let kernel_value: f64 = y
                .iter()
                .zip(matrix_vector(&c, n, n, &y))
                .map(|(left, right)| left * right)
                .sum();
            assert!((kernel_value - (plugin - correction)).abs() < 3.0e-11);

            let mut z = vec![0.0; n];
            fill_gaussian_pseudo_outcome(
                CounterRng::new(79),
                3,
                &(1..=n as u64).collect::<Vec<_>>(),
                &vec![0; n],
                &variance,
                &mut z,
                &mut NeverInterrupt,
            )
            .expect("Gaussian draw");
            let u = matrix_vector(&sxt, p, n, &z);
            let pseudo_fit = matrix_vector(&projection, n, n, &z);
            let pseudo_residual = z
                .iter()
                .zip(pseudo_fit)
                .map(|(left, right)| left - right)
                .collect::<Vec<_>>();
            let plugin = u
                .iter()
                .zip(matrix_vector(target, p, p, &u))
                .map(|(left, right)| left * right)
                .sum();
            let scalar = probe_scalar(plugin, &pseudo_residual, ratio, &z).expect("probe scalar");
            let dense_scalar: f64 = z
                .iter()
                .zip(matrix_vector(&c, n, n, &z))
                .map(|(left, right)| left * right)
                .sum();
            assert!((scalar - dense_scalar).abs() < 3.0e-11);
        }
    }

    #[test]
    fn dense_q1_remainder_influence_probe_and_covariance_identities_hold() {
        let (n, p, x, y, targets, variance) = dense_fixture();
        let target = &targets[0];
        let xt = transpose(&x, n, p);
        let h = multiply(&xt, p, n, &x, p);
        let inverse = invert_scaled_spd(&h, p, 1.0e-12, &mut NeverInterrupt, "component_q1_test_h")
            .expect("invertible dense information")
            .inverse;
        let sxt = multiply(&inverse, p, p, &xt, n);
        let projection = multiply(&x, n, p, &sxt, n);
        let sq = multiply(&inverse, p, p, target, p);
        let b = multiply(
            &multiply(&x, n, p, &multiply(&sq, p, p, &inverse, p), p),
            n,
            p,
            &xt,
            n,
        );

        // Independent dense power iteration on B^2 obtains the dominant
        // observation-space generalized mode without production mode code.
        let mut mode = matrix_vector(&x, n, p, &[0.3, -0.7, 0.2, 1.1]);
        for _ in 0..256 {
            mode = matrix_vector(&b, n, n, &matrix_vector(&b, n, n, &mode));
            let norm = mode.iter().map(|value| value * value).sum::<f64>().sqrt();
            for value in &mut mode {
                *value /= norm;
            }
        }
        let b_mode = matrix_vector(&b, n, n, &mode);
        let eigenvalue = mode
            .iter()
            .zip(&b_mode)
            .map(|(left, right)| left * right)
            .sum::<f64>();
        let eigen_residual = b_mode
            .iter()
            .zip(&mode)
            .map(|(left, right)| (left - eigenvalue * right).powi(2))
            .sum::<f64>()
            .sqrt();
        assert!(eigen_residual < 1.0e-11);

        let target_diagonal = (0..n).map(|row| b[row * n + row]).collect::<Vec<_>>();
        let maker_inverse = (0..n)
            .map(|row| (1.0 - projection[row * n + row]).recip())
            .collect::<Vec<_>>();
        let ratio = q1_remainder_ratio(&target_diagonal, &maker_inverse, &mode, eigenvalue)
            .expect("q=1 remainder ratio");
        let mut c1 = b.clone();
        for row in 0..n {
            for column in 0..n {
                let maker = f64::from(row == column) - projection[row * n + column];
                c1[row * n + column] -= eigenvalue * mode[row] * mode[column]
                    + 0.5 * (ratio[row] * maker + maker * ratio[column]);
            }
            assert!(c1[row * n + row].abs() < 3.0e-12);
        }

        let beta = matrix_vector(&sxt, p, n, &y);
        let fitted = matrix_vector(&projection, n, n, &y);
        let residual = y
            .iter()
            .zip(fitted)
            .map(|(left, right)| left - right)
            .collect::<Vec<_>>();
        let q_beta = matrix_vector(target, p, p, &beta);
        let xtry = matrix_vector(
            &xt,
            p,
            n,
            &ratio
                .iter()
                .zip(&y)
                .map(|(left, right)| 0.5 * left * right)
                .collect::<Vec<_>>(),
        );
        let combined_rhs = q_beta
            .iter()
            .zip(xtry)
            .map(|(left, right)| left + right)
            .collect::<Vec<_>>();
        let combined_prediction =
            matrix_vector(&multiply(&x, n, p, &inverse, p), n, p, &combined_rhs);
        let leading_score = mode
            .iter()
            .zip(&y)
            .map(|(left, right)| left * right)
            .sum::<f64>();
        let influence = finish_q1_influence(
            &combined_prediction,
            &ratio,
            &y,
            &residual,
            &mode,
            eigenvalue,
            leading_score,
        )
        .expect("q=1 remainder influence");
        let dense_influence = matrix_vector(&c1, n, n, &y);
        for (left, right) in influence.iter().zip(&dense_influence) {
            assert!((left - right).abs() < 4.0e-11);
        }

        let mut z = vec![0.0; n];
        fill_gaussian_pseudo_outcome(
            CounterRng::new(0xa21c_709e),
            7,
            &(1..=n as u64).collect::<Vec<_>>(),
            &vec![0; n],
            &variance,
            &mut z,
            &mut NeverInterrupt,
        )
        .expect("q=1 Gaussian draw");
        let u = matrix_vector(&sxt, p, n, &z);
        let plugin = u
            .iter()
            .zip(matrix_vector(target, p, p, &u))
            .map(|(left, right)| left * right)
            .sum::<f64>();
        let pseudo_fit = matrix_vector(&projection, n, n, &z);
        let pseudo_residual = z
            .iter()
            .zip(pseudo_fit)
            .map(|(left, right)| left - right)
            .collect::<Vec<_>>();
        let scalar = q1_probe_scalar(plugin, &pseudo_residual, &ratio, &z, &mode, eigenvalue)
            .expect("q=1 remainder probe scalar");
        let dense_scalar = z
            .iter()
            .zip(matrix_vector(&c1, n, n, &z))
            .map(|(left, right)| left * right)
            .sum::<f64>();
        assert!((scalar - dense_scalar).abs() < 4.0e-11);

        let trace_variance = (0..n)
            .flat_map(|row| {
                let variance_ref = &variance;
                let c1_ref = &c1;
                (0..n).map(move |column| {
                    2.0 * variance_ref[row]
                        * c1_ref[row * n + column]
                        * variance_ref[column]
                        * c1_ref[column * n + row]
                })
            })
            .sum::<f64>();
        let point_estimate = y
            .iter()
            .zip(matrix_vector(&b, n, n, &y))
            .map(|(left, right)| left * right)
            .sum::<f64>()
            - (0..n)
                .map(|row| target_diagonal[row] * y[row] * residual[row] * maker_inverse[row])
                .sum::<f64>();
        let leading_variance_correction = (0..n)
            .map(|row| mode[row] * mode[row] * y[row] * residual[row] * maker_inverse[row])
            .sum::<f64>();
        let direct_remainder_estimate = y
            .iter()
            .zip(&influence)
            .map(|(outcome, influence)| outcome * influence)
            .sum::<f64>();
        let result = finish_q1_target(
            point_estimate,
            leading_score,
            leading_variance_correction,
            direct_remainder_estimate,
            1.0e-9,
            eigenvalue,
            &mode,
            &influence,
            &variance,
            trace_variance,
            0.0,
            1.0e-10,
        )
        .expect("dense q=1 covariance");
        let expected_leading = mode
            .iter()
            .zip(&variance)
            .map(|(mode, variance)| mode * mode * variance)
            .sum::<f64>();
        let expected_covariance = mode
            .iter()
            .zip(&variance)
            .zip(&influence)
            .map(|((mode, variance), influence)| 2.0 * mode * variance * influence)
            .sum::<f64>();
        let expected_remainder = 4.0
            * influence
                .iter()
                .zip(&variance)
                .map(|(influence, variance)| influence * influence * variance)
                .sum::<f64>()
            - trace_variance;
        let expected_influence_concentration = influence
            .iter()
            .zip(&variance)
            .map(|(influence, variance)| 4.0 * influence * influence * variance)
            .fold(0.0_f64, f64::max)
            / (expected_remainder + trace_variance);
        assert!((result.leading_variance - expected_leading).abs() < 2.0e-13);
        assert_eq!(
            result.leading_variance_correction.to_bits(),
            leading_variance_correction.to_bits()
        );
        assert!(result.remainder_identity_error < 4.0e-11);
        let modeled_recenter_remainder =
            point_estimate - eigenvalue * (leading_score * leading_score - expected_leading);
        assert!((leading_variance_correction - expected_leading).abs() > 1.0e-3);
        assert!((modeled_recenter_remainder - direct_remainder_estimate).abs() > 1.0e-3);
        assert!(
            (result.leading_remainder_covariance - expected_covariance).abs() < 2.0e-13,
            "{} versus {}",
            result.leading_remainder_covariance,
            expected_covariance
        );
        assert!((result.remainder_variance - expected_remainder).abs() < 2.0e-13);
        assert!(
            (result.remainder_influence_concentration - expected_influence_concentration).abs()
                < 2.0e-13
        );
        let dense_covariance = mode
            .iter()
            .zip(&variance)
            .zip(&dense_influence)
            .map(|((mode, variance), influence)| 2.0 * mode * variance * influence)
            .sum::<f64>();
        assert!((result.leading_remainder_covariance - dense_covariance).abs() < 2.0e-11);
        assert!(result.curvature.is_finite() && result.curvature > 0.0);
    }

    #[test]
    fn q1_curvature_matches_standardized_parabola_geometry_and_outcome_units() {
        // Independent geometric oracle: after whitening, the target level set
        // is the parabola (t, a*t^2). Its vertex curvature is |y''(0)|.
        let mut reference_interval = None;
        for scale in [0.01_f64, 0.1, 1.0, 10.0, 100.0] {
            let result = finish_q1_target(
                2.0 * scale * scale,
                0.0,
                0.0,
                2.0 * scale * scale,
                1.0e-9,
                0.7,
                &[1.0, 0.0],
                &[0.0, scale],
                &[0.25 * scale * scale, scale * scale],
                0.0,
                0.0,
                1.0e-10,
            )
            .expect("scaled positive q1 covariance");
            let horizontal = result.leading_variance.sqrt();
            let vertical = result.remainder_variance.sqrt();
            let parabola = |t: f64| 0.7 * (horizontal * t).powi(2) / vertical;
            let step = 0.125;
            let geometric_curvature =
                (parabola(step) - 2.0 * parabola(0.0) + parabola(-step)) / (step * step);
            assert!((result.curvature - geometric_curvature).abs() < 1.0e-12);
            let interval =
                finish_q1_interval(result, 781, 0, 0.7, 0.95, 2_000).expect("scaled q1 interval");
            let normalized = [
                interval.confidence_lower / (scale * scale),
                interval.confidence_upper / (scale * scale),
            ];
            if let Some(expected) = reference_interval {
                let expected: [f64; 2] = expected;
                for i in 0..2 {
                    assert!((normalized[i] - expected[i]).abs() < 1.0e-10);
                }
            } else {
                reference_interval = Some(normalized);
            }
        }
    }

    #[test]
    fn q1_covariance_gate_is_invariant_to_coordinate_units() {
        for scale in [1.0e-6_f64, 1.0, 1.0e6] {
            let result = finish_q1_target(
                2.0 * scale * scale,
                0.0,
                0.0,
                2.0 * scale * scale,
                1.0e-9,
                0.7,
                &[1.0, 0.0],
                &[0.0, scale],
                &[0.25 * scale * scale, scale * scale],
                0.0,
                0.0,
                1.0e-8,
            )
            .expect("positive diagonal covariance is valid regardless of units");
            assert!((result.curvature - 0.175).abs() < 1.0e-12);
        }
        for coordinate in [0.1_f64, 1.0, 10.0] {
            let result = finish_q1_target(
                2.0,
                0.0,
                0.0,
                2.0,
                1.0e-9,
                0.7 / (coordinate * coordinate),
                &[coordinate, 0.0],
                &[0.0, 1.0],
                &[0.25, 1.0],
                0.0,
                0.0,
                1.0e-8,
            )
            .expect("reparameterized score covariance");
            assert!((result.curvature - 0.175).abs() < 1.0e-12);
        }
        let result = finish_q1_target(
            2.0,
            0.0,
            0.0,
            2.0,
            1.0e-9,
            0.7,
            &[1.0, 0.0],
            &[1.0, 0.0],
            &[0.25, 1.0],
            0.0,
            0.0,
            1.0e-8,
        )
        .expect("singularity is target-local");
        assert_eq!(result.status, ComponentQ1Status::SingularCovariance);
        let unavailable = finish_q1_interval(result, 1, 0, 0.7, 0.95, 1000).unwrap();
        assert!(unavailable.confidence_lower.is_nan());
        assert!(unavailable.confidence_upper.is_nan());
        assert_eq!(unavailable.critical_draws, 0);
    }

    #[test]
    fn q1_unavailable_target_preserves_other_streams_and_fatal_identity_checks() {
        let make = |trace, direct| {
            finish_q1_target(
                2.0,
                0.0,
                0.0,
                direct,
                1.0e-9,
                0.7,
                &[1.0, 0.0],
                &[0.0, 1.0],
                &[0.25, 1.0],
                trace,
                0.01,
                1.0e-8,
            )
        };
        let output: Vec<_> = (0..4)
            .map(|target| {
                finish_q1_interval(
                    make(if target == 2 { 5.0 } else { 0.0 }, 2.0).unwrap(),
                    719,
                    target,
                    0.7,
                    0.95,
                    2000,
                )
                .unwrap()
            })
            .collect();
        assert_eq!(output[2].status, ComponentQ1Status::NonpositiveVariance);
        assert_eq!(output[2].remainder_variance, -1.0);
        assert_eq!(output[2].remainder_trace_variance, 5.0);
        assert!(output[2].confidence_lower.is_nan());
        assert_eq!(
            output
                .iter()
                .map(|target| target.critical_draws)
                .sum::<u32>(),
            6000
        );
        for target in [0, 1, 3] {
            let alone =
                finish_q1_interval(make(0.0, 2.0).unwrap(), 719, target, 0.7, 0.95, 2000).unwrap();
            assert_eq!(output[target], alone);
        }
        assert_eq!(
            make(5.0, 3.0).unwrap_err().code,
            ErrorCode::TargetIdentityFailed
        );
    }

    #[test]
    fn dense_grouped_match_kernel_uses_maker_blocks_not_observation_diagonals() {
        let (n, p, x, y, targets, _) = dense_fixture();
        let xt = transpose(&x, n, p);
        let h = multiply(&xt, p, n, &x, p);
        let inverse = invert_scaled_spd(
            &h,
            p,
            1.0e-12,
            &mut NeverInterrupt,
            "component_group_test_h",
        )
        .expect("invertible dense information")
        .inverse;
        let projection = multiply(&x, n, p, &multiply(&inverse, p, p, &xt, n), n);
        let mut maker = vec![0.0; n * n];
        for row in 0..n {
            for column in 0..n {
                maker[row * n + column] = f64::from(row == column) - projection[row * n + column];
            }
        }
        let sq = multiply(&inverse, p, p, &targets[0], p);
        let b = multiply(
            &multiply(&x, n, p, &multiply(&sq, p, p, &inverse, p), p),
            n,
            p,
            &xt,
            n,
        );
        let grouped_kernel = |groups: &[Vec<usize>]| {
            let mut r = vec![0.0; n * n];
            for (group_index, group) in groups.iter().enumerate() {
                let width = group.len();
                let mut maker_block = vec![0.0; width * width];
                let mut target_block = vec![0.0; width * width];
                for (local_row, &row) in group.iter().enumerate() {
                    for (local_column, &column) in group.iter().enumerate() {
                        maker_block[local_row * width + local_column] = maker[row * n + column];
                        target_block[local_row * width + local_column] = b[row * n + column];
                    }
                }
                let maker_inverse = invert_scaled_spd(
                    &maker_block,
                    width,
                    1.0e-12,
                    &mut NeverInterrupt,
                    "component_group_test_maker",
                )
                .unwrap_or_else(|_| panic!("match {group_index} maker block is invertible"))
                .inverse;
                let ratio = multiply(&target_block, width, width, &maker_inverse, width);
                for (local_row, &row) in group.iter().enumerate() {
                    for (local_column, &column) in group.iter().enumerate() {
                        r[row * n + column] = ratio[local_row * width + local_column];
                    }
                }
            }
            let rm = multiply(&r, n, n, &maker, n);
            let mr_transpose = multiply(&maker, n, n, &transpose(&r, n, n), n);
            (0..n * n)
                .map(|index| b[index] - 0.5 * (rm[index] + mr_transpose[index]))
                .collect::<Vec<_>>()
        };
        let singleton = (0..n).map(|row| vec![row]).collect::<Vec<_>>();
        let matches = vec![vec![0, 1], vec![2, 3], vec![4, 5], vec![6, 7]];
        let observation_kernel = grouped_kernel(&singleton);
        let match_kernel = grouped_kernel(&matches);
        assert!(observation_kernel
            .iter()
            .zip(&match_kernel)
            .any(|(left, right)| (left - right).abs() > 1.0e-8));
        for group in &matches {
            for &row in group {
                for &column in group {
                    assert!(match_kernel[row * n + column].abs() < 3.0e-12);
                }
            }
        }
        let match_value = y
            .iter()
            .zip(matrix_vector(&match_kernel, n, n, &y))
            .map(|(left, right)| left * right)
            .sum::<f64>();
        let plugin = y
            .iter()
            .zip(matrix_vector(&b, n, n, &y))
            .map(|(left, right)| left * right)
            .sum::<f64>();
        let residual = matrix_vector(&maker, n, n, &y);
        let mut correction = 0.0;
        for group in &matches {
            let width = group.len();
            let mut maker_block = vec![0.0; width * width];
            let mut target_block = vec![0.0; width * width];
            for (local_row, &row) in group.iter().enumerate() {
                for (local_column, &column) in group.iter().enumerate() {
                    maker_block[local_row * width + local_column] = maker[row * n + column];
                    target_block[local_row * width + local_column] = b[row * n + column];
                }
            }
            let maker_inverse = invert_scaled_spd(
                &maker_block,
                width,
                1.0e-12,
                &mut NeverInterrupt,
                "component_group_test_point",
            )
            .expect("match maker block")
            .inverse;
            let leaveout_residual = matrix_vector(
                &maker_inverse,
                width,
                width,
                &group.iter().map(|&row| residual[row]).collect::<Vec<_>>(),
            );
            let target_leaveout = matrix_vector(&target_block, width, width, &leaveout_residual);
            correction += group
                .iter()
                .enumerate()
                .map(|(local, &row)| y[row] * target_leaveout[local])
                .sum::<f64>();
        }
        assert!((match_value - (plugin - correction)).abs() < 5.0e-11);
    }

    #[test]
    fn spectral_concentration_reports_diffuse_and_dominant_cases_without_routing() {
        let diffuse = finish_spectrum_diagnostics(
            1.0, 0.9, 100.0, 0.5, 0.03, 1.0e-9, 2.0e-8, 512, 32, 1.0e-6,
        )
        .expect("diffuse spectrum");
        assert!((diffuse.leading_share - 0.01).abs() < 1.0e-15);
        assert!(diffuse.remainder_leading_share < 0.01);

        let dominant = finish_spectrum_diagnostics(
            10.0, 1.0, 102.0, 0.2, 0.04, 1.0e-9, 2.0e-8, 512, 32, 1.0e-6,
        )
        .expect("one-mode spectrum");
        assert!(dominant.leading_share > 0.98);
        assert!((dominant.remainder_leading_share - 0.5).abs() < 1.0e-14);

        let error = finish_spectrum_diagnostics(
            10.0, 2.0, 10.0, 0.1, 0.04, 1.0e-9, 2.0e-8, 512, 32, 1.0e-6,
        )
        .expect_err("material trace/mode inconsistency rejects");
        assert_eq!(error.code, ErrorCode::JlaConstraintFailed);
        assert_eq!(error.phase, "component_inference_spectrum");
    }

    #[test]
    fn spectrum_trace_mcse_tracks_probe_doubling_deterministically() {
        let eigenvalues = [3.0_f64, 2.0, 1.0, 0.5];
        let exact = eigenvalues.iter().map(|value| value * value).sum::<f64>();
        let trace = |probes: u64| {
            let mut first = StableAccumulator::default();
            let mut second = StableAccumulator::default();
            let rng = CounterRng::new(0x9a73_41c5_ee20_6d18);
            let entity = [11_u64, 12, 13, 14];
            let subdraw = [0_u64; 4];
            let variance = [1.0_f64; 4];
            let mut draw = [0.0_f64; 4];
            for probe in 0..probes {
                fill_gaussian_pseudo_outcome(
                    rng,
                    probe,
                    &entity,
                    &subdraw,
                    &variance,
                    &mut draw,
                    &mut NeverInterrupt,
                )
                .expect("deterministic trace draw");
                let value = eigenvalues
                    .iter()
                    .zip(draw)
                    .map(|(eigenvalue, draw)| eigenvalue * eigenvalue * draw * draw)
                    .sum::<f64>();
                first.add(value);
                second.add(value * value);
            }
            let count = probes as f64;
            let mean = first.finish() / count;
            let variance = ((second.finish() - count * mean * mean) / (count - 1.0)).max(0.0);
            (mean, (variance / count).sqrt())
        };
        let first = trace(4_096);
        let doubled = trace(8_192);
        assert!((first.0 - exact).abs() <= 4.0 * first.1);
        assert!((doubled.0 - exact).abs() <= 4.0 * doubled.1);
        assert!(doubled.1 < first.1);
        let first_diagnostic = finish_spectrum_diagnostics(
            3.0, 2.0, first.0, first.1, 0.1, 0.0, 0.0, 4_096, 8, 1.0e-6,
        )
        .expect("first spectrum diagnostic");
        let doubled_diagnostic = finish_spectrum_diagnostics(
            3.0, 2.0, doubled.0, doubled.1, 0.1, 0.0, 0.0, 8_192, 8, 1.0e-6,
        )
        .expect("doubled spectrum diagnostic");
        assert!(
            doubled_diagnostic.leading_share_mcse_trace_only
                < first_diagnostic.leading_share_mcse_trace_only
        );
    }

    #[test]
    fn oracle_q0_conditional_coverage_is_correct_for_a_diffuse_heteroskedastic_target() {
        // Forty independent 2-by-2 off-diagonal blocks give eigenvalues
        // +/-1/sqrt(n), hence leading concentration 1/n.  This is a bounded
        // conditional experiment for the oracle-variance q=0 layer, not a
        // validation of any estimated variance regression.
        let rows = 80_usize;
        let pairs = rows / 2;
        let coefficient = 1.0 / (rows as f64).sqrt();
        let mean = (0..rows)
            .map(|row| -0.25 + 0.01 * row as f64)
            .collect::<Vec<_>>();
        let variance = (0..rows)
            .map(|row| 0.35 + 0.11 * (row % 7) as f64)
            .collect::<Vec<_>>();
        let estimand = (0..pairs)
            .map(|pair| 2.0 * coefficient * mean[2 * pair] * mean[2 * pair + 1])
            .sum::<f64>();
        let exact_variance = (0..pairs)
            .map(|pair| {
                let left = 2 * pair;
                let right = left + 1;
                4.0 * coefficient
                    * coefficient
                    * (variance[left] * variance[right]
                        + variance[left] * mean[right] * mean[right]
                        + variance[right] * mean[left] * mean[left])
            })
            .sum::<f64>();
        let standard_error = exact_variance.sqrt();
        let entity = (1..=rows as u64).collect::<Vec<_>>();
        let subdraw = vec![0_u64; rows];
        let mut errors = vec![0.0; rows];
        let simulations = 10_000_u64;
        let mut covered = 0_u64;
        for simulation in 0..simulations {
            fill_gaussian_pseudo_outcome(
                CounterRng::new(0x9a18_18d4_b803_a04c),
                simulation,
                &entity,
                &subdraw,
                &variance,
                &mut errors,
                &mut NeverInterrupt,
            )
            .expect("oracle conditional draw");
            let estimate = (0..pairs)
                .map(|pair| {
                    let left = 2 * pair;
                    let right = left + 1;
                    2.0 * coefficient * (mean[left] + errors[left]) * (mean[right] + errors[right])
                })
                .sum::<f64>();
            let radius = 1.959_963_984_540_054 * standard_error;
            covered += u64::from((estimate - estimand).abs() <= radius);
        }
        let coverage = covered as f64 / simulations as f64;
        println!("oracle q=0 conditional coverage={coverage:.4}");
        assert!((0.938..=0.962).contains(&coverage), "coverage={coverage}");
    }

    #[test]
    fn q1_critical_value_and_confidence_ellipse_are_deterministic() {
        let first = q1_critical_value(71, 2, 0.0, 0.95, 50_000).expect("zero-curvature radius");
        let repeated = q1_critical_value(71, 2, 0.0, 0.95, 50_000).expect("repeated radius");
        let higher = q1_critical_value(71, 2, 0.0, 0.99, 50_000).expect("higher-confidence radius");
        assert_eq!(first.to_bits(), repeated.to_bits());
        assert!((1.90..2.02).contains(&first));
        assert!(higher > first);

        let center = [0.7, -0.3];
        let covariance = [1.0, 0.2, 0.2, 2.0];
        let interval = q1_am_interval(center, covariance, first, 0.0)
            .expect("linear zero-curvature ellipse image");
        assert!((interval[0] - (center[1] - first * 2.0_f64.sqrt())).abs() < 2.0e-11);
        assert!((interval[1] - (center[1] + first * 2.0_f64.sqrt())).abs() < 2.0e-11);
    }

    #[test]
    fn q1_counter_critical_values_match_independent_numerical_integration() {
        let simulations = 100_000;
        let confidence = 0.95;
        let monte_carlo_tolerance =
            4.0 * (confidence * (1.0 - confidence) / f64::from(simulations)).sqrt() + 2.0e-5;
        for (target, curvature) in [0.0, 0.05, 0.25, 1.0, 4.0, 20.0].into_iter().enumerate() {
            let critical = q1_critical_value(
                0x7e2b_94ad_160c_c7f1,
                target % REPORTED_TARGETS,
                curvature,
                confidence,
                simulations,
            )
            .expect("q=1 Counter-V1 critical value");
            let integrated = integrated_q1_cdf(critical, curvature);
            assert!(
                (integrated - confidence).abs() <= monte_carlo_tolerance,
                "curvature={curvature}, critical={critical}, integrated CDF={integrated}, tolerance={monte_carlo_tolerance}"
            );
        }
    }

    #[test]
    fn q1_ellipse_image_matches_independent_dense_angular_oracle() {
        const GRID: usize = 1_000_000;
        let radius = 2.137;
        for (center, covariance, eigenvalue) in [
            ([0.7, -0.3], [1.0, -0.8, -0.8, 1.7], 0.65),
            ([-1.1, 0.9], [0.8, 0.0, 0.0, 2.3], -0.75),
            ([0.2, -0.4], [1.4, 0.75, 0.75, 0.9], 1.25),
        ] {
            let observed = q1_am_interval(center, covariance, radius, eigenvalue)
                .expect("production q=1 ellipse image");
            let first = covariance[0].sqrt();
            let cross = covariance[2] / first;
            let conditional = (covariance[3] - cross * cross).sqrt();
            let mut oracle = [f64::INFINITY, f64::NEG_INFINITY];
            for index in 0..GRID {
                let angle = core::f64::consts::TAU * index as f64 / GRID as f64;
                let leading = center[0] + radius * first * angle.cos();
                let remainder =
                    center[1] + radius * (cross * angle.cos() + conditional * angle.sin());
                let value = eigenvalue * leading * leading + remainder;
                oracle[0] = oracle[0].min(value);
                oracle[1] = oracle[1].max(value);
            }
            assert!(
                (observed[0] - oracle[0]).abs() < 2.0e-9,
                "lower endpoint {:?} versus {:?}",
                observed,
                oracle
            );
            assert!(
                (observed[1] - oracle[1]).abs() < 2.0e-9,
                "upper endpoint {:?} versus {:?}",
                observed,
                oracle
            );
        }
    }

    #[test]
    fn online_probe_covariance_matches_retained_dense_calculation() {
        let draws = [
            [0.2, -0.7, 1.1],
            [1.4, 0.1, -0.2],
            [-0.3, 0.9, 0.5],
            [2.1, -1.2, 0.8],
            [0.6, 0.4, -1.0],
        ];
        let mut online = JointProbeMoments::default();
        for draw in draws {
            online.push(draw).expect("finite draw");
        }
        let result = online.finish().expect("online covariance");
        for left in 0..PRIMITIVE_TARGETS {
            for right in 0..PRIMITIVE_TARGETS {
                let expected: f64 = draws
                    .iter()
                    .map(|draw| {
                        (draw[left] - result.mean[left]) * (draw[right] - result.mean[right])
                    })
                    .sum::<f64>()
                    / (draws.len() - 1) as f64;
                assert!(
                    (result.covariance[left * PRIMITIVE_TARGETS + right] - expected).abs()
                        < 1.0e-14
                );
                assert!(result.mcse[left * PRIMITIVE_TARGETS + right].is_finite());
            }
        }
    }

    #[test]
    fn gaussian_trace_covariance_matches_explicit_dense_trace_within_reported_mcse() {
        let (n, p, x, y, targets, variance) = dense_fixture();
        let xt = transpose(&x, n, p);
        let h = multiply(&xt, p, n, &x, p);
        let inverse = invert_scaled_spd(
            &h,
            p,
            1.0e-12,
            &mut NeverInterrupt,
            "component_trace_test_h",
        )
        .expect("invertible dense information")
        .inverse;
        let sxt = multiply(&inverse, p, p, &xt, n);
        let projection = multiply(&x, n, p, &sxt, n);
        let maker_inverse = (0..n)
            .map(|row| (1.0 - projection[row * n + row]).recip())
            .collect::<Vec<_>>();
        let mut kernels: [Vec<f64>; PRIMITIVE_TARGETS] = core::array::from_fn(|_| Vec::new());
        for (target_index, target) in targets.iter().enumerate() {
            let sq = multiply(&inverse, p, p, target, p);
            let sqs = multiply(&sq, p, p, &inverse, p);
            let b = multiply(&multiply(&x, n, p, &sqs, p), n, p, &xt, n);
            let diagonal = (0..n).map(|row| b[row * n + row]).collect::<Vec<_>>();
            let ratio = diagonal
                .iter()
                .zip(&maker_inverse)
                .map(|(left, right)| left * right)
                .collect::<Vec<_>>();
            let mut c = b;
            for row in 0..n {
                for column in 0..n {
                    let maker = f64::from(row == column) - projection[row * n + column];
                    c[row * n + column] -= 0.5 * (ratio[row] * maker + maker * ratio[column]);
                }
            }
            kernels[target_index] = c;
        }
        let entity = (1..=n as u64).collect::<Vec<_>>();
        let subdraw = vec![0; n];
        let rng = CounterRng::new(0x3141_5926_5358_9793);
        let probes = 50_000_u64;
        let mut z = vec![0.0; n];
        let mut moments = JointProbeMoments::default();
        for probe in 0..probes {
            fill_gaussian_pseudo_outcome(
                rng,
                probe,
                &entity,
                &subdraw,
                &variance,
                &mut z,
                &mut NeverInterrupt,
            )
            .expect("Gaussian trace draw");
            let scalar = core::array::from_fn(|target| {
                z.iter()
                    .zip(matrix_vector(&kernels[target], n, n, &z))
                    .map(|(left, right)| left * right)
                    .sum()
            });
            moments.push(scalar).expect("finite trace scalar");
        }
        let observed = moments.finish().expect("trace covariance moments");
        let mut maximum_standardized_error = 0.0_f64;
        for left in 0..PRIMITIVE_TARGETS {
            for right in 0..PRIMITIVE_TARGETS {
                let mut expected = 0.0;
                for row in 0..n {
                    for column in 0..n {
                        expected += 2.0
                            * variance[row]
                            * kernels[left][row * n + column]
                            * variance[column]
                            * kernels[right][column * n + row];
                    }
                }
                let index = left * PRIMITIVE_TARGETS + right;
                maximum_standardized_error = maximum_standardized_error.max(
                    (observed.covariance[index] - expected).abs()
                        / observed.mcse[index].max(1.0e-30),
                );
                assert!(
                    (observed.covariance[index] - expected).abs()
                        <= 6.0 * observed.mcse[index] + 1.0e-10,
                    "observed {}, exact {}, MCSE {} for ({left},{right})",
                    observed.covariance[index],
                    expected,
                    observed.mcse[index]
                );
            }
        }
        assert!(maximum_standardized_error <= 6.0 + 1.0e-10);
        // Every dense C_t has an exact zero diagonal, so tr(V C_t)=0.
        for target in 0..PRIMITIVE_TARGETS {
            let standard_error =
                (observed.covariance[target * PRIMITIVE_TARGETS + target] / probes as f64).sqrt();
            assert!(observed.mean[target].abs() <= 6.0 * standard_error + 1.0e-10);
        }
        let _ = y;
    }

    #[test]
    fn fourth_target_is_an_exact_singular_linear_map() {
        let primitive = [2.0, 0.3, -0.1, 0.3, 1.5, 0.2, -0.1, 0.2, 0.8];
        let mapped = map_primitive_covariance(&primitive).expect("finite map");
        for index in 0..REPORTED_TARGETS {
            assert_eq!(
                mapped[3 * REPORTED_TARGETS + index],
                mapped[index]
                    + mapped[REPORTED_TARGETS + index]
                    + 2.0 * mapped[2 * REPORTED_TARGETS + index]
            );
        }
        let null = [-1.0, -1.0, -2.0, 1.0];
        for row in 0..REPORTED_TARGETS {
            let value: f64 = (0..REPORTED_TARGETS)
                .map(|column| mapped[row * REPORTED_TARGETS + column] * null[column])
                .sum();
            assert!(value.abs() < 1.0e-14);
        }
    }

    #[test]
    fn gaussian_domain_is_deterministic_and_batch_addressed() {
        let variance = [0.5, 1.0, 2.0, 4.0];
        let entity = [3, 7, 7, 11];
        let subdraw = [0, 0, 1, 0];
        let mut first = [0.0; 4];
        let mut second = [0.0; 4];
        fill_gaussian_pseudo_outcome(
            CounterRng::new(1234),
            17,
            &entity,
            &subdraw,
            &variance,
            &mut first,
            &mut NeverInterrupt,
        )
        .expect("first draw");
        fill_gaussian_pseudo_outcome(
            CounterRng::new(1234),
            17,
            &entity,
            &subdraw,
            &variance,
            &mut second,
            &mut NeverInterrupt,
        )
        .expect("second draw");
        assert_eq!(first, second);
        assert_ne!(first[1], first[2]);
    }
}
