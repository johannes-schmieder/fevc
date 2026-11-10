// SPDX-License-Identifier: MIT

//! Leave-out residual standard errors for the variance components.
//!
//! For a leave-out estimator `theta_t = y' K_t y` with independent inferential
//! units, `Cov(theta_t, theta_s) = 4 (K_t mu)' V (K_s mu) + 2 tr(K_t V K_s V)`.
//! This module estimates the first, signal, term from squared regression
//! residuals and removes from it what noise alone contributes. That noise
//! contribution and the trace term are both expectations under pure noise, and
//! both are estimated by pushing Gaussian pseudo-noise through the same kernel
//! as the outcome. The working variance of the pseudo-noise is an ordinary
//! least-squares fit of the leave-out products on a small outcome-free basis.
//!
//! Squared residuals smooth each unit's variance over the units linked to it
//! through a worker or firm, so the signal term assumes similar variances among
//! such neighbours. It is not robust to arbitrary heteroskedasticity. The
//! derivation and its limits are in `fevc/docs/standard_errors/`.

use crate::component_inference::{PRIMITIVE_TARGETS, REPORTED_TARGETS};
use crate::error::{BackendError, ErrorCode, Result};
use crate::interrupt::{checkpoint_chunk, InterruptCheck};
use crate::structured_variance::normalized_midranks;

/// Units of one type needed for each working-variance coefficient. Below this
/// the basis is shortened in a fixed order; the rule uses no outcome.
pub const UNITS_PER_WORKING_TERM: usize = 100;
/// Intercept, leverage rank and its square, mass rank and its square.
pub const MAXIMUM_WORKING_TERMS: usize = 5;
const TOTAL_WEIGHT: [f64; PRIMITIVE_TARGETS] = [1.0, 1.0, 2.0];
const SQUARE: usize = PRIMITIVE_TARGETS * PRIMITIVE_TARGETS;

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

fn invalid(message: impl Into<String>) -> BackendError {
    BackendError::invalid("residual_noise", message)
}

fn nonfinite(message: impl Into<String>) -> BackendError {
    BackendError::new(ErrorCode::CorrectionNonFinite, "residual_noise", message)
}

/// Positive working variances and the outcome-free shape of their fit.
#[derive(Clone, Debug)]
pub struct WorkingVariance {
    pub variance: Vec<f64>,
    /// Active coefficients for mover units and for stayer observations.
    pub terms: [usize; 2],
    pub floor: f64,
    pub floored: u64,
}

/// Fit `E[z_a e_{a,-a}] = sigma_a^2` by least squares within each unit type.
///
/// `proxy` holds the leave-out products the point estimator already forms.
/// Units `[0, mover_units)` are collapsed matches or observations; any later
/// units are physical stayer observations. `mass` is the regression mass of a
/// collapsed match and is absent for observation units.
pub fn fit_working_variance(
    proxy: &[f64],
    residual: &[f64],
    maker_inverse: &[f64],
    leverage: &[f64],
    mass: Option<&[f64]>,
    mover_units: usize,
    positivity_multiplier: f64,
    interrupt: &mut dyn InterruptCheck,
) -> Result<WorkingVariance> {
    let rows = proxy.len();
    if rows == 0
        || residual.len() != rows
        || maker_inverse.len() != rows
        || leverage.len() != rows
        || mass.is_some_and(|value| value.len() != rows)
        || mover_units > rows
        || !positivity_multiplier.is_finite()
        || positivity_multiplier <= 0.0
    {
        return Err(invalid("working-variance inputs are inconsistent"));
    }
    let mut variance = vec![0.0; rows];
    let mut terms = [0_usize; 2];
    for (kind, range) in [(0, 0..mover_units), (1, mover_units..rows)] {
        if range.is_empty() {
            continue;
        }
        let leverage_rank = normalized_midranks(&leverage[range.clone()], interrupt)?;
        let mass_rank = match mass {
            Some(mass) if kind == 0 => Some(normalized_midranks(&mass[range.clone()], interrupt)?),
            _ => None,
        };
        let units = range.len();
        let available = if mass_rank.is_some() { MAXIMUM_WORKING_TERMS } else { 3 };
        let permitted = (units / UNITS_PER_WORKING_TERM).clamp(1, available);
        let basis = |local: usize| -> [f64; MAXIMUM_WORKING_TERMS] {
            let lever = leverage_rank[local];
            // Linear terms precede squares so a shortened basis stays monotone.
            match &mass_rank {
                Some(rank) => [1.0, lever, rank[local], lever * lever, rank[local] * rank[local]],
                None => [1.0, lever, lever * lever, 0.0, 0.0],
            }
        };
        let mut gram = [[StableAccumulator::default(); MAXIMUM_WORKING_TERMS]; MAXIMUM_WORKING_TERMS];
        let mut moment = [StableAccumulator::default(); MAXIMUM_WORKING_TERMS];
        for local in 0..units {
            checkpoint_chunk(interrupt, local, "residual_noise_working_variance")?;
            let value = proxy[range.start + local];
            if !value.is_finite() {
                return Err(nonfinite("a leave-out variance product is nonfinite"));
            }
            let row = basis(local);
            for left in 0..permitted {
                moment[left].add(row[left] * value);
                for right in left..permitted {
                    gram[left][right].add(row[left] * row[right]);
                }
            }
        }
        let mut matrix = [[0.0; MAXIMUM_WORKING_TERMS]; MAXIMUM_WORKING_TERMS];
        let mut rhs = [0.0; MAXIMUM_WORKING_TERMS];
        for left in 0..permitted {
            rhs[left] = moment[left].finish();
            for right in left..permitted {
                matrix[left][right] = gram[left][right].finish();
                matrix[right][left] = matrix[left][right];
            }
        }
        let (coefficient, active) = solve_reduced(&matrix, &rhs, permitted)?;
        terms[kind] = active;
        for local in 0..units {
            checkpoint_chunk(interrupt, local, "residual_noise_working_variance")?;
            let row = basis(local);
            let mut value = StableAccumulator::default();
            for term in 0..permitted {
                value.add(row[term] * coefficient[term]);
            }
            variance[range.start + local] = value.finish();
        }
    }
    let mut scale = Vec::with_capacity(rows);
    for row in 0..rows {
        checkpoint_chunk(interrupt, row, "residual_noise_working_variance")?;
        scale.push(residual[row] * residual[row] * maker_inverse[row]);
    }
    let middle = rows / 2;
    scale.select_nth_unstable_by(middle, f64::total_cmp);
    let floor = positivity_multiplier * scale[middle];
    if !floor.is_finite() || floor <= 0.0 {
        return Err(BackendError::new(
            ErrorCode::JlaConstraintFailed,
            "residual_noise_working_variance",
            "the residual scale for the working-variance floor is not positive",
        ));
    }
    let mut floored = 0_u64;
    for value in &mut variance {
        if !value.is_finite() {
            return Err(nonfinite("a fitted working variance is nonfinite"));
        }
        if *value < floor {
            *value = floor;
            floored += 1;
        }
    }
    if floored == rows as u64 {
        return Err(BackendError::new(
            ErrorCode::JlaConstraintFailed,
            "residual_noise_working_variance",
            "every fitted working variance was floored",
        ));
    }
    Ok(WorkingVariance {
        variance,
        terms,
        floor,
        floored,
    })
}

/// Solve a small normal system, dropping columns that the retained ones span.
/// Pivot order is the fixed term order, so the choice uses no outcome.
fn solve_reduced(
    matrix: &[[f64; MAXIMUM_WORKING_TERMS]; MAXIMUM_WORKING_TERMS],
    rhs: &[f64; MAXIMUM_WORKING_TERMS],
    terms: usize,
) -> Result<([f64; MAXIMUM_WORKING_TERMS], usize)> {
    const SPAN_TOLERANCE: f64 = 1.0e-10;
    let mut factor = [[0.0; MAXIMUM_WORKING_TERMS]; MAXIMUM_WORKING_TERMS];
    let mut kept = [false; MAXIMUM_WORKING_TERMS];
    for column in 0..terms {
        let mut pivot = matrix[column][column];
        for prior in (0..column).filter(|&prior| kept[prior]) {
            pivot -= factor[column][prior] * factor[column][prior];
        }
        if !pivot.is_finite() {
            return Err(nonfinite("the working-variance Gram matrix is nonfinite"));
        }
        if pivot <= SPAN_TOLERANCE * matrix[column][column].max(f64::MIN_POSITIVE) {
            continue;
        }
        kept[column] = true;
        factor[column][column] = pivot.sqrt();
        for row in column + 1..terms {
            let mut value = matrix[row][column];
            for prior in (0..column).filter(|&prior| kept[prior]) {
                value -= factor[row][prior] * factor[column][prior];
            }
            factor[row][column] = value / factor[column][column];
        }
    }
    if !kept[0] {
        return Err(invalid("the working-variance intercept is not identified"));
    }
    let mut forward = [0.0; MAXIMUM_WORKING_TERMS];
    for row in (0..terms).filter(|&row| kept[row]) {
        let mut value = rhs[row];
        for prior in (0..row).filter(|&prior| kept[prior]) {
            value -= factor[row][prior] * forward[prior];
        }
        forward[row] = value / factor[row][row];
    }
    let mut coefficient = [0.0; MAXIMUM_WORKING_TERMS];
    for row in (0..terms).rev().filter(|&row| kept[row]) {
        let mut value = forward[row];
        for later in (row + 1..terms).filter(|&later| kept[later]) {
            value -= factor[later][row] * coefficient[later];
        }
        coefficient[row] = value / factor[row][row];
    }
    if coefficient.iter().any(|value| !value.is_finite()) {
        return Err(nonfinite("the working-variance coefficients are nonfinite"));
    }
    Ok((coefficient, kept.iter().filter(|&&value| value).count()))
}

/// Remove the retained-mean direction from a unit-space vector. The direction
/// is `sqrt(mass)` for collapsed matches and one for observations, which is
/// how a common shift of every physical outcome appears after collapsing.
pub fn center_units(
    values: &mut [f64],
    mass: Option<&[f64]>,
    interrupt: &mut dyn InterruptCheck,
) -> Result<()> {
    if mass.is_some_and(|mass| mass.len() != values.len()) {
        return Err(invalid("centering mass has the wrong length"));
    }
    let mut numerator = StableAccumulator::default();
    let mut denominator = StableAccumulator::default();
    for row in 0..values.len() {
        checkpoint_chunk(interrupt, row, "residual_noise_center")?;
        let weight = mass.map_or(1.0, |mass| mass[row]);
        numerator.add(weight.sqrt() * values[row]);
        denominator.add(weight);
    }
    let denominator = denominator.finish();
    if !denominator.is_finite() || denominator <= 0.0 {
        return Err(invalid("centering mass is not positive"));
    }
    let mean = numerator.finish() / denominator;
    if !mean.is_finite() {
        return Err(nonfinite("a unit-space mean is nonfinite"));
    }
    for row in 0..values.len() {
        checkpoint_chunk(interrupt, row, "residual_noise_center")?;
        values[row] -= mean * mass.map_or(1.0, |mass| mass[row].sqrt());
    }
    Ok(())
}

/// Pseudo-noise expectations of the trace and of the residual statistic.
#[derive(Clone, Debug, PartialEq)]
pub struct NoiseMoments {
    /// `tr(K_t V K_s V)`, row-major.
    pub trace: [f64; SQUARE],
    pub trace_mcse: [f64; SQUARE],
    /// `E sum_a w_a (M zeta)_a^2 (K_t zeta)_a (K_s zeta)_a`, row-major.
    pub residual_noise: [f64; SQUARE],
    pub residual_noise_mcse: [f64; SQUARE],
    pub draws: u32,
    /// `tr(W^2)^2 / tr(W^4)` for `W = V^{1/2} K V^{1/2}` and the four reported
    /// targets. Its inverse and inverse square root bracket the leading
    /// squared-eigenvalue share. Missing when fewer than two draws are made.
    pub effective_rank: [f64; REPORTED_TARGETS],
}

/// Streams the per-draw statistics. Only the previous draw's kernel vectors
/// are retained, for the consecutive pairs behind the effective rank.
#[derive(Debug, Default)]
pub struct NoiseAccumulator {
    draws: u32,
    trace_sum: [StableAccumulator; SQUARE],
    trace_square: [StableAccumulator; SQUARE],
    noise_sum: [StableAccumulator; SQUARE],
    noise_square: [StableAccumulator; SQUARE],
    pair_square: [StableAccumulator; REPORTED_TARGETS],
    pairs: u32,
    previous: Option<[Vec<f64>; PRIMITIVE_TARGETS]>,
}

impl NoiseAccumulator {
    /// `kernel[t]` is `K_t zeta` and `pseudo_residual` is `M zeta` for one draw
    /// `zeta ~ N(0, diag(variance))`; `weight` is the residual-maker inverse.
    pub fn push(
        &mut self,
        variance: &[f64],
        weight: &[f64],
        pseudo_residual: &[f64],
        kernel: [Vec<f64>; PRIMITIVE_TARGETS],
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<()> {
        let rows = variance.len();
        if weight.len() != rows
            || pseudo_residual.len() != rows
            || kernel.iter().any(|value| value.len() != rows)
        {
            return Err(invalid("pseudo-noise vectors have inconsistent lengths"));
        }
        let mut trace = [StableAccumulator::default(); SQUARE];
        let mut noise = [StableAccumulator::default(); SQUARE];
        let mut pair = [StableAccumulator::default(); SQUARE];
        for row in 0..rows {
            checkpoint_chunk(interrupt, row, "residual_noise_draw")?;
            let scale = weight[row] * pseudo_residual[row] * pseudo_residual[row];
            for left in 0..PRIMITIVE_TARGETS {
                for right in left..PRIMITIVE_TARGETS {
                    let product = kernel[left][row] * kernel[right][row];
                    trace[left * PRIMITIVE_TARGETS + right].add(variance[row] * product);
                    noise[left * PRIMITIVE_TARGETS + right].add(scale * product);
                }
                if let Some(previous) = &self.previous {
                    for right in 0..PRIMITIVE_TARGETS {
                        pair[left * PRIMITIVE_TARGETS + right]
                            .add(variance[row] * previous[left][row] * kernel[right][row]);
                    }
                }
            }
        }
        for left in 0..PRIMITIVE_TARGETS {
            for right in left..PRIMITIVE_TARGETS {
                let index = left * PRIMITIVE_TARGETS + right;
                let mirror = right * PRIMITIVE_TARGETS + left;
                let trace_value = trace[index].finish();
                let noise_value = noise[index].finish();
                if !trace_value.is_finite() || !noise_value.is_finite() {
                    return Err(nonfinite("a pseudo-noise statistic is nonfinite"));
                }
                for position in [index, mirror] {
                    self.trace_sum[position].add(trace_value);
                    self.trace_square[position].add(trace_value * trace_value);
                    self.noise_sum[position].add(noise_value);
                    self.noise_square[position].add(noise_value * noise_value);
                    if left == right {
                        break;
                    }
                }
            }
        }
        if self.previous.is_some() {
            let mut total = 0.0;
            for left in 0..PRIMITIVE_TARGETS {
                let diagonal = pair[left * PRIMITIVE_TARGETS + left].finish();
                self.pair_square[left].add(diagonal * diagonal);
                for right in 0..PRIMITIVE_TARGETS {
                    total += TOTAL_WEIGHT[left]
                        * TOTAL_WEIGHT[right]
                        * pair[left * PRIMITIVE_TARGETS + right].finish();
                }
            }
            if !total.is_finite() {
                return Err(nonfinite("a pseudo-noise pair statistic is nonfinite"));
            }
            self.pair_square[PRIMITIVE_TARGETS].add(total * total);
            self.pairs += 1;
        }
        self.previous = Some(kernel);
        self.draws = self
            .draws
            .checked_add(1)
            .ok_or_else(|| invalid("pseudo-noise draw count overflow"))?;
        Ok(())
    }

    pub fn finish(self) -> Result<NoiseMoments> {
        if self.draws == 0 {
            return Err(invalid("no pseudo-noise draw was accumulated"));
        }
        let draws = f64::from(self.draws);
        let summarize = |sum: &[StableAccumulator; SQUARE], square: &[StableAccumulator; SQUARE]| {
            let mut mean = [0.0; SQUARE];
            let mut mcse = [0.0; SQUARE];
            for index in 0..SQUARE {
                mean[index] = sum[index].finish() / draws;
                if self.draws > 1 {
                    let spread = (square[index].finish() - draws * mean[index] * mean[index])
                        / (draws - 1.0);
                    mcse[index] = (spread.max(0.0) / draws).sqrt();
                }
            }
            (mean, mcse)
        };
        let (trace, trace_mcse) = summarize(&self.trace_sum, &self.trace_square);
        let (residual_noise, residual_noise_mcse) = summarize(&self.noise_sum, &self.noise_square);
        let mut effective_rank = [f64::NAN; REPORTED_TARGETS];
        if self.pairs > 0 {
            for target in 0..REPORTED_TARGETS {
                let trace_square = if target < PRIMITIVE_TARGETS {
                    trace[target * PRIMITIVE_TARGETS + target]
                } else {
                    let mut value = 0.0;
                    for left in 0..PRIMITIVE_TARGETS {
                        for right in 0..PRIMITIVE_TARGETS {
                            value += TOTAL_WEIGHT[left]
                                * TOTAL_WEIGHT[right]
                                * trace[left * PRIMITIVE_TARGETS + right];
                        }
                    }
                    value
                };
                let fourth = self.pair_square[target].finish() / f64::from(self.pairs);
                if fourth > 0.0 && trace_square > 0.0 {
                    // tr(W^4) <= tr(W^2)^2, so the ratio is at least one.
                    effective_rank[target] = (trace_square * trace_square / fourth).max(1.0);
                }
            }
        }
        if trace
            .iter()
            .chain(&trace_mcse)
            .chain(&residual_noise)
            .chain(&residual_noise_mcse)
            .any(|value| !value.is_finite())
        {
            return Err(nonfinite("pseudo-noise moments are nonfinite"));
        }
        Ok(NoiseMoments {
            trace,
            trace_mcse,
            residual_noise,
            residual_noise_mcse,
            draws: self.draws,
            effective_rank,
        })
    }
}

/// First-order contribution of estimating a fixed control offset.
///
/// Vectors are control-major over units. `collapsed` holds the unit sums of
/// the controls divided by `sqrt(mass)`, `residualized` the same aggregation of
/// the controls after removing the two-way effects, and `within` the unit sums
/// of products of within-unit deviations of controls and of the offset
/// outcome. `precision` is the inverse cross-product of the residualized
/// controls over physical observations, row-major.
#[derive(Clone, Debug)]
pub struct ControlAdjustment {
    pub controls: usize,
    pub collapsed: Vec<f64>,
    pub residualized: Vec<f64>,
    pub within: Vec<f64>,
    pub precision: Vec<f64>,
}

/// Covariance pieces for the three primitive targets, all row-major.
#[derive(Clone, Debug, PartialEq)]
pub struct ResidualCovariance {
    /// Reported covariance: truncated residual-based signal plus the trace term.
    pub primitive: [f64; SQUARE],
    /// The same with fitted working variances in place of squared residuals.
    pub model_primitive: [f64; SQUARE],
    pub signal: [f64; SQUARE],
    pub trace_term: [f64; SQUARE],
    pub trace_mcse: [f64; SQUARE],
    /// Most negative signal eigenvalue removed by the truncation, or zero.
    pub signal_truncation: f64,
    pub smallest_signal_eigenvalue: f64,
    pub largest_signal_eigenvalue: f64,
    /// Largest unit share of each reported target's squared linear influence.
    pub influence_concentration: [f64; REPORTED_TARGETS],
}

/// Assemble the covariance from the realized influences and the noise moments.
///
/// `influence[t]` is `K_t z`, `residual` is `M z`, and `maker_inverse` scales a
/// squared residual to a variance. The signal is truncated to be positive
/// semidefinite because `4 (K mu)' V (K mu)` is; the reported covariance then
/// dominates the trace term and is positive definite whenever that term is.
pub fn finish_covariance(
    influence: &[Vec<f64>; PRIMITIVE_TARGETS],
    residual: &[f64],
    maker_inverse: &[f64],
    variance: &[f64],
    control: Option<&ControlAdjustment>,
    moments: &NoiseMoments,
    interrupt: &mut dyn InterruptCheck,
) -> Result<ResidualCovariance> {
    let rows = residual.len();
    if influence.iter().any(|value| value.len() != rows)
        || maker_inverse.len() != rows
        || variance.len() != rows
        || maker_inverse
            .iter()
            .any(|value| !value.is_finite() || *value <= 0.0)
        || variance.iter().any(|value| !value.is_finite() || *value <= 0.0)
    {
        return Err(invalid("residual covariance inputs are invalid"));
    }
    let mut load = [const { Vec::new() }; PRIMITIVE_TARGETS];
    if let Some(control) = control {
        let q = control.controls;
        if q == 0
            || control.collapsed.len() != q * rows
            || control.residualized.len() != q * rows
            || control.within.len() != q * rows
            || control.precision.len() != q * q
        {
            return Err(invalid("control adjustment has inconsistent dimensions"));
        }
        for target in 0..PRIMITIVE_TARGETS {
            // Derivative of the fixed-offset estimate in the control coefficients.
            let mut slope = vec![StableAccumulator::default(); q];
            for row in 0..rows {
                checkpoint_chunk(interrupt, row, "residual_noise_control")?;
                for (index, value) in slope.iter_mut().enumerate() {
                    value.add(-2.0 * control.collapsed[index * rows + row] * influence[target][row]);
                }
            }
            let slope = slope.into_iter().map(StableAccumulator::finish).collect::<Vec<_>>();
            let mut weight = vec![0.0; q];
            for left in 0..q {
                let mut value = StableAccumulator::default();
                for right in 0..q {
                    value.add(control.precision[left * q + right] * slope[right]);
                }
                weight[left] = value.finish();
            }
            if weight.iter().any(|value| !value.is_finite()) {
                return Err(nonfinite("a control-estimation weight is nonfinite"));
            }
            load[target] = weight;
        }
    }
    let mut residual_signal = [StableAccumulator::default(); SQUARE];
    let mut influence_square = [StableAccumulator::default(); SQUARE];
    let mut model_square = [StableAccumulator::default(); SQUARE];
    let mut total = [StableAccumulator::default(); REPORTED_TARGETS];
    let mut largest = [0.0_f64; REPORTED_TARGETS];
    for row in 0..rows {
        checkpoint_chunk(interrupt, row, "residual_noise_signal")?;
        let scaled = residual[row] * maker_inverse[row].sqrt();
        let mut score = [0.0; PRIMITIVE_TARGETS];
        for target in 0..PRIMITIVE_TARGETS {
            let mut slope = 2.0 * influence[target][row];
            let mut shift = 0.0;
            if let Some(control) = control {
                for index in 0..control.controls {
                    slope += control.residualized[index * rows + row] * load[target][index];
                    shift += control.within[index * rows + row] * load[target][index];
                }
            }
            score[target] = slope * scaled + shift;
        }
        let squared_residual = scaled * scaled;
        for left in 0..PRIMITIVE_TARGETS {
            for right in left..PRIMITIVE_TARGETS {
                let index = left * PRIMITIVE_TARGETS + right;
                let product = influence[left][row] * influence[right][row];
                residual_signal[index].add(score[left] * score[right]);
                influence_square[index].add(4.0 * product * squared_residual);
                model_square[index].add(4.0 * product * variance[row]);
            }
        }
        for target in 0..REPORTED_TARGETS {
            let value = if target < PRIMITIVE_TARGETS {
                score[target]
            } else {
                score[0] + score[1] + 2.0 * score[2]
            };
            total[target].add(value * value);
            largest[target] = largest[target].max(value * value);
        }
    }
    let mut signal = [0.0; SQUARE];
    let mut model_signal = [0.0; SQUARE];
    let mut trace_term = [0.0; SQUARE];
    let mut trace_mcse = [0.0; SQUARE];
    for left in 0..PRIMITIVE_TARGETS {
        for right in left..PRIMITIVE_TARGETS {
            let index = left * PRIMITIVE_TARGETS + right;
            let mirror = right * PRIMITIVE_TARGETS + left;
            let value = residual_signal[index].finish() - 4.0 * moments.residual_noise[index];
            // Swap only the squared two-way influence for its working-variance
            // counterpart; control-estimation pieces stay residual-based.
            let model = value - (influence_square[index].finish() - 4.0 * moments.residual_noise[index])
                + (model_square[index].finish() - 4.0 * moments.trace[index]);
            if !value.is_finite() || !model.is_finite() {
                return Err(nonfinite("the component signal covariance is nonfinite"));
            }
            for position in [index, mirror] {
                signal[position] = value;
                model_signal[position] = model;
                trace_term[position] = 2.0 * moments.trace[index];
                trace_mcse[position] = 2.0 * moments.trace_mcse[index];
            }
        }
    }
    let (truncated, smallest, largest_eigenvalue) = positive_part(&signal)?;
    let (model_truncated, _, _) = positive_part(&model_signal)?;
    let mut primitive = [0.0; SQUARE];
    let mut model_primitive = [0.0; SQUARE];
    for index in 0..SQUARE {
        primitive[index] = truncated[index] + trace_term[index];
        model_primitive[index] = model_truncated[index] + trace_term[index];
    }
    let mut influence_concentration = [0.0; REPORTED_TARGETS];
    for target in 0..REPORTED_TARGETS {
        let value = total[target].finish();
        if !value.is_finite() {
            return Err(nonfinite("a component influence total is nonfinite"));
        }
        if value > 0.0 {
            influence_concentration[target] = largest[target] / value;
        }
    }
    Ok(ResidualCovariance {
        primitive,
        model_primitive,
        signal,
        trace_term,
        trace_mcse,
        signal_truncation: (-smallest).max(0.0),
        smallest_signal_eigenvalue: smallest,
        largest_signal_eigenvalue: largest_eigenvalue,
        influence_concentration,
    })
}

/// What a caller needs to judge the pass beyond the covariance itself.
#[derive(Clone, Debug, PartialEq)]
pub struct Diagnostic {
    pub draws: u32,
    pub working_terms: [usize; 2],
    pub working_floor: f64,
    pub working_floored: u64,
    /// Controls whose estimation enters through [`ControlAdjustment`].
    pub controls: usize,
    pub effective_rank: [f64; REPORTED_TARGETS],
    pub signal_truncation: f64,
    pub residual_noise: [f64; SQUARE],
    pub residual_noise_mcse: [f64; SQUARE],
    pub model_primitive: [f64; SQUARE],
    /// Four-target map of `model_primitive`, row-major.
    pub model_covariance: [f64; REPORTED_TARGETS * REPORTED_TARGETS],
}

impl Diagnostic {
    pub fn new(
        fit: &WorkingVariance,
        controls: usize,
        moments: &NoiseMoments,
        covariance: &ResidualCovariance,
    ) -> Result<Self> {
        Ok(Self {
            draws: moments.draws,
            working_terms: fit.terms,
            working_floor: fit.floor,
            working_floored: fit.floored,
            controls,
            effective_rank: moments.effective_rank,
            signal_truncation: covariance.signal_truncation,
            residual_noise: moments.residual_noise,
            residual_noise_mcse: moments.residual_noise_mcse,
            model_primitive: covariance.model_primitive,
            model_covariance: crate::component_inference::map_primitive_covariance(
                &covariance.model_primitive,
            )?,
        })
    }
}

/// Project a symmetric 3x3 matrix on the positive semidefinite cone and return
/// its extreme eigenvalues. Cyclic Jacobi rotations are exact enough here and
/// avoid any dependence on the rest of the dense module.
fn positive_part(matrix: &[f64; SQUARE]) -> Result<([f64; SQUARE], f64, f64)> {
    const N: usize = PRIMITIVE_TARGETS;
    let mut a = [[0.0; N]; N];
    let mut v = [[0.0; N]; N];
    for row in 0..N {
        v[row][row] = 1.0;
        for column in 0..N {
            a[row][column] = 0.5 * (matrix[row * N + column] + matrix[column * N + row]);
        }
    }
    let scale = a
        .iter()
        .flatten()
        .fold(0.0_f64, |left, right| left.max(right.abs()));
    if !scale.is_finite() {
        return Err(nonfinite("the component signal covariance is nonfinite"));
    }
    for _ in 0..64 {
        let off = a[0][1].abs().max(a[0][2].abs()).max(a[1][2].abs());
        if off <= f64::EPSILON * scale {
            break;
        }
        for (p, q) in [(0, 1), (0, 2), (1, 2)] {
            if a[p][q] == 0.0 {
                continue;
            }
            let theta = (a[q][q] - a[p][p]) / (2.0 * a[p][q]);
            let t = theta.signum() / (theta.abs() + (theta * theta + 1.0).sqrt());
            let t = if theta == 0.0 { 1.0 } else { t };
            let c = 1.0 / (t * t + 1.0).sqrt();
            let s = t * c;
            for k in 0..N {
                let (akp, akq) = (a[k][p], a[k][q]);
                a[k][p] = c * akp - s * akq;
                a[k][q] = s * akp + c * akq;
            }
            for k in 0..N {
                let (apk, aqk) = (a[p][k], a[q][k]);
                a[p][k] = c * apk - s * aqk;
                a[q][k] = s * apk + c * aqk;
            }
            for k in 0..N {
                let (vkp, vkq) = (v[k][p], v[k][q]);
                v[k][p] = c * vkp - s * vkq;
                v[k][q] = s * vkp + c * vkq;
            }
        }
    }
    let eigenvalue = [a[0][0], a[1][1], a[2][2]];
    let smallest = eigenvalue.iter().copied().fold(f64::INFINITY, f64::min);
    let largest = eigenvalue.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let mut output = [0.0; SQUARE];
    for row in 0..N {
        for column in 0..N {
            let mut value = 0.0;
            for k in 0..N {
                value += v[row][k] * eigenvalue[k].max(0.0) * v[column][k];
            }
            output[row * N + column] = value;
        }
    }
    if output.iter().any(|value| !value.is_finite()) {
        return Err(nonfinite("the truncated signal covariance is nonfinite"));
    }
    Ok((output, smallest, largest))
}

#[cfg(test)]
mod tests;
