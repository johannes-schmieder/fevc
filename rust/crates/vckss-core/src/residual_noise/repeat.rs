// SPDX-License-Identifier: MIT

//! Repeat-measurement standard errors for the variance components.
//!
//! Each error variance is estimated from independent repeat measurements of
//! the same mean wherever they exist, using only the independence the point
//! estimator already assumes, and from a model elsewhere. Deletion units that
//! share a worker-firm cell have identical regressors and independent errors,
//! so a cell with two or more units is a repeat-measurement set and its units
//! receive the sample variance of their outcomes. Under match deletion these
//! are the physical observations of an eligible stayer; a mover's collapsed
//! match is one unit and has no repeat measurement. Under observation deletion
//! every physical observation is a unit, so a mover's match of two or more
//! observations is a set as well. Units alone in their cell receive a scale,
//! fitted on leverage and unit mass, times a quadratic in the standardised
//! estimated worker and firm effects, both steps fitted to leave-out products.
//!
//! With these variances the covariance is the KSS high-rank form
//! `4 sum_a v_a (K_t z)_a (K_s z)_a - 2 tr(K_t V K_s V)` plus a closed-form
//! term for the pairs of units of one set, where the sample variance is
//! correlated with the influence. With true variances for the units outside
//! sets the estimate is exactly unbiased under normal errors in the sets,
//! whatever their variances are. The derivation and its limits are in
//! `fevc/docs/standard_errors/`.

use super::{
    invalid, nonfinite, positive_part, solve_reduced, ControlAdjustment, NoiseMoments,
    ResidualCovariance, StableAccumulator, MAXIMUM_WORKING_TERMS, SQUARE, UNITS_PER_WORKING_TERM,
};
use crate::component_inference::{
    ComponentQ1Status, ComponentQ1TargetResult, PRIMITIVE_TARGETS, REPORTED_TARGETS,
};
use crate::error::{BackendError, ErrorCode, Result};
use crate::interrupt::{checkpoint_chunk, InterruptCheck};
use crate::structured_variance::normalized_midranks;

/// Intercept, worker effect and its square, firm effect and its square, and
/// their product. Linear terms of one effect precede the other's so that a
/// shortened basis stays a function of the worker effect first.
pub const PROFILE_TERMS: usize = 6;
/// Standardised effects are limited to this many standard deviations.
pub const PROFILE_CAP: f64 = 3.0;
/// The fitted scale is not allowed below this share of the mean leave-out
/// product of the fitted units, so that the profile fit cannot divide by a
/// vanishing or negative scale.
pub const SCALE_FLOOR_SHARE: f64 = 0.05;

/// Unit-level inputs. A unit is a deletion unit: a collapsed mover match or a
/// physical stayer observation under match deletion, a physical observation
/// under observation deletion.
#[derive(Clone, Copy, Debug)]
pub struct RepeatInputs<'a> {
    /// Leave-out products `z_a e_{a,-a}`.
    pub proxy: &'a [f64],
    /// Unit outcomes. Only differences within a set are used.
    pub outcome: &'a [f64],
    pub residual: &'a [f64],
    pub maker_inverse: &'a [f64],
    pub leverage: &'a [f64],
    /// Regression mass of every unit: the match size of a collapsed match,
    /// and one for a physical observation.
    pub mass: &'a [f64],
    /// Worker-firm cell of every unit. Units of one cell have identical
    /// regressors, so two or more of them form a repeat-measurement set.
    pub set: &'a [u64],
    /// Whether the unit's worker is observed at more than one firm.
    pub mover: &'a [bool],
    /// Estimated worker and firm effect of every unit.
    pub worker_effect: &'a [f64],
    pub firm_effect: &'a [f64],
    /// Diagonals of the three primitive targets on every unit.
    pub target_diagonal: [&'a [f64]; PRIMITIVE_TARGETS],
    pub positivity_multiplier: f64,
}

/// Membership of the repeat-measurement sets, kept so that the pair term can
/// be formed for another kernel on the same units.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RepeatSets {
    /// Units of all sets, grouped by set.
    pub members: Vec<u32>,
    /// One past the last member of every set.
    pub end: Vec<u32>,
    /// Sample variance of every set, before the positivity floor.
    pub sample: Vec<f64>,
    pub mover: Vec<bool>,
}

/// Unit variances and the outcome-free shape of their construction.
#[derive(Clone, Debug, PartialEq)]
pub struct RepeatVariance {
    pub variance: Vec<f64>,
    /// Active coefficients of the scale and the profile of the fitted model.
    pub scale_terms: usize,
    pub profile_terms: usize,
    /// Profile coefficients in the fixed term order; zero where inactive.
    pub profile: [f64; PROFILE_TERMS],
    pub scale_floor: f64,
    pub scale_floored: u64,
    /// Positivity floor of the fitted variances and of the pseudo-noise
    /// variance of a set whose outcomes coincide.
    pub floor: f64,
    pub floored: u64,
    /// Repeat-measurement sets, their units, and the units of sets whose
    /// sample variance is below the floor. Under match deletion the sets are
    /// the eligible stayers, which the names record.
    pub stayers: u64,
    pub stayer_units: u64,
    pub stayer_floored: u64,
    /// Units alone in their cell, which receive the fitted model, and the
    /// units the model was fitted on.
    pub model_units: u64,
    pub model_fit_units: u64,
    /// Closed-form term for the pairs of units of one set, row-major and
    /// symmetric. A stayer's errors are absorbed by its worker effect, so a
    /// stayer contributes to the worker-variance entry alone; a mover's set
    /// reaches all three kernels.
    pub adjustment: [f64; SQUARE],
    pub sets: RepeatSets,
}

impl RepeatVariance {
    /// Pair term of the remainder of one reported target after the direction
    /// `mode` with eigenvalue `eigenvalue` is split off. Two units of one set
    /// then share the weight `(B_aa - eigenvalue mode_a^2) / (1 - P_aa)`.
    /// `stayer_diagonal` says whether the target's diagonal is nonzero on a
    /// stayer's units: it is for the worker variance and the variance of the
    /// sum, and vanishes for the firm variance and the covariance.
    pub fn remainder_pair_term(
        &self,
        target_diagonal: &[f64],
        maker_inverse: &[f64],
        mode: &[f64],
        eigenvalue: f64,
        stayer_diagonal: bool,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<f64> {
        let rows = self.variance.len();
        if target_diagonal.len() != rows
            || maker_inverse.len() != rows
            || mode.len() != rows
            || !eigenvalue.is_finite()
        {
            return Err(invalid("remainder pair-term inputs are inconsistent"));
        }
        let mut total = StableAccumulator::default();
        let mut begin = 0_usize;
        for set in 0..self.sets.end.len() {
            checkpoint_chunk(interrupt, set, "repeat_measurement_remainder_pairs")?;
            let end = self.sets.end[set] as usize;
            let units = &self.sets.members[begin..end];
            begin = end;
            let count = units.len() as f64;
            let mut weight = StableAccumulator::default();
            for &unit in units {
                let unit = unit as usize;
                let square = eigenvalue * mode[unit] * mode[unit];
                if self.sets.mover[set] {
                    weight.add((target_diagonal[unit] - square) * maker_inverse[unit]);
                } else if stayer_diagonal {
                    weight.add(target_diagonal[unit] - square);
                } else {
                    weight.add(-square);
                }
            }
            let weight = if self.sets.mover[set] {
                weight.finish() / count
            } else {
                weight.finish() / (count - 1.0)
            };
            let sample = self.sets.sample[set];
            total.add(
                4.0 * weight * weight * sample * sample * (count - 2.0) * (count - 1.0)
                    / (count + 1.0),
            );
        }
        let total = total.finish();
        if !total.is_finite() {
            return Err(nonfinite("the remainder pair term is nonfinite"));
        }
        Ok(total)
    }
}

/// First-order term of the estimated control coefficients.
///
/// The repeat-measurement variances belong to the outcome net of the fitted
/// control index. Its coefficients are estimated, so its noise is not the
/// unit errors alone: with `Pi = I - Z~ Lambda Z'` it is `Pi' eps` minus
/// `Z Lambda` times the within-unit scores of the controls. A target
/// `y' K y` then moves with the coefficients through `-2 Z' K y`, a leading
/// score `d'y` through `-Z' d`. For any unit-space influence `h`,
/// `u = Lambda Z' h` gives the starred influence `Pi h = h - Z~ u`, which
/// carries the unit errors, and the within-unit part `W c` with the load
/// `c = -2 u` for a target and `c = -u` for a score; the two meet through the
/// leave-out residual `e / M`. The quadratic term uses pseudo-noise with the
/// covariance of that noise: every draw is transformed before any kernel, and
/// the images enter the bilinear form
/// `sum_a v_a (Pi x)_a (Pi y)_a + (Lambda Z' x)' W'W (Lambda Z' y)`.
#[derive(Clone, Debug)]
pub struct RepeatControl<'a> {
    adjustment: &'a ControlAdjustment,
    rows: usize,
    /// Leave-out residual `e_a / M_aa` of every unit.
    leave_out: Vec<f64>,
    /// `W' W` over the units, row-major.
    omega: Vec<f64>,
    /// Units with a nonzero within-unit product: the only ones whose extra
    /// standard normal enters a draw.
    within_units: Vec<usize>,
}

/// One pseudo-noise image in the bilinear form: `Pi x` and `Lambda Z' x`.
#[derive(Clone, Debug, PartialEq)]
pub struct RepeatControlImage {
    pub projected: Vec<f64>,
    pub coefficient: Vec<f64>,
}

impl<'a> RepeatControl<'a> {
    pub fn new(
        adjustment: &'a ControlAdjustment,
        residual: &[f64],
        maker_inverse: &[f64],
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<Self> {
        let rows = residual.len();
        let q = adjustment.controls;
        if rows == 0
            || q == 0
            || maker_inverse.len() != rows
            || adjustment.collapsed.len() != q * rows
            || adjustment.residualized.len() != q * rows
            || adjustment.within.len() != q * rows
            || adjustment.precision.len() != q * q
        {
            return Err(invalid(
                "repeat-measurement control inputs have inconsistent dimensions",
            ));
        }
        let mut leave_out = Vec::with_capacity(rows);
        let mut within_units = Vec::new();
        for row in 0..rows {
            checkpoint_chunk(interrupt, row, "repeat_measurement_control")?;
            leave_out.push(residual[row] * maker_inverse[row]);
            if (0..q).any(|control| adjustment.within[control * rows + row] != 0.0) {
                within_units.push(row);
            }
        }
        let mut omega = vec![0.0; q * q];
        for left in 0..q {
            for right in left..q {
                let mut value = StableAccumulator::default();
                for (position, &row) in within_units.iter().enumerate() {
                    checkpoint_chunk(interrupt, position, "repeat_measurement_control")?;
                    value.add(
                        adjustment.within[left * rows + row]
                            * adjustment.within[right * rows + row],
                    );
                }
                omega[left * q + right] = value.finish();
                omega[right * q + left] = omega[left * q + right];
            }
        }
        if leave_out
            .iter()
            .chain(&omega)
            .chain(&adjustment.precision)
            .chain(&adjustment.collapsed)
            .chain(&adjustment.residualized)
            .any(|value| !value.is_finite())
        {
            return Err(nonfinite("a repeat-measurement control input is nonfinite"));
        }
        Ok(Self {
            adjustment,
            rows,
            leave_out,
            omega,
            within_units,
        })
    }

    #[must_use]
    pub const fn controls(&self) -> usize {
        self.adjustment.controls
    }

    /// Units whose extra standard normal enters a draw, in increasing order.
    /// Under observation deletion there are none.
    #[must_use]
    pub fn within_units(&self) -> &[usize] {
        &self.within_units
    }

    /// `W'W`, row-major.
    #[must_use]
    pub fn omega(&self) -> &[f64] {
        &self.omega
    }

    /// `Lambda Z' h`.
    fn coefficient(&self, h: &[f64]) -> Vec<f64> {
        let (q, rows) = (self.adjustment.controls, self.rows);
        let slope = (0..q)
            .map(|control| {
                let mut value = StableAccumulator::default();
                for row in 0..rows {
                    value.add(self.adjustment.collapsed[control * rows + row] * h[row]);
                }
                value.finish()
            })
            .collect::<Vec<_>>();
        self.precision_times(&slope)
    }

    fn precision_times(&self, slope: &[f64]) -> Vec<f64> {
        let q = self.adjustment.controls;
        (0..q)
            .map(|left| {
                let mut value = StableAccumulator::default();
                for right in 0..q {
                    value.add(self.adjustment.precision[left * q + right] * slope[right]);
                }
                value.finish()
            })
            .collect()
    }

    /// `h - Z~ u`.
    fn star(&self, h: &[f64], coefficient: &[f64]) -> Vec<f64> {
        let rows = self.rows;
        (0..rows)
            .map(|row| {
                let mut value = h[row];
                for (control, &weight) in coefficient.iter().enumerate() {
                    value -= self.adjustment.residualized[control * rows + row] * weight;
                }
                value
            })
            .collect()
    }

    /// `W (factor u)`.
    fn within(&self, coefficient: &[f64], factor: f64) -> Vec<f64> {
        let rows = self.rows;
        (0..rows)
            .map(|row| {
                let mut value = 0.0;
                for (control, &weight) in coefficient.iter().enumerate() {
                    value += self.adjustment.within[control * rows + row] * factor * weight;
                }
                value
            })
            .collect()
    }

    /// `left' W'W right`.
    #[must_use]
    pub fn quadratic(&self, left: &[f64], right: &[f64]) -> f64 {
        let q = self.adjustment.controls;
        let mut value = StableAccumulator::default();
        for a in 0..q {
            for b in 0..q {
                value.add(left[a] * self.omega[a * q + b] * right[b]);
            }
        }
        value.finish()
    }

    /// Starred influence and within-unit part of a target's influence.
    fn target_parts(&self, influence: &[f64]) -> (Vec<f64>, Vec<f64>) {
        let coefficient = self.coefficient(influence);
        (
            self.star(influence, &coefficient),
            self.within(&coefficient, -2.0),
        )
    }

    /// Transform a pseudo-noise draw `zeta ~ N(0, V)` into one with the
    /// covariance of the control-adjusted noise:
    /// `zeta - Z Lambda (Z~' zeta + W' xi)`, where `xi[i]` is the standard
    /// normal of unit `within_units()[i]`.
    pub fn transform_draw(&self, draw: &mut [f64], xi: &[f64]) -> Result<()> {
        let (q, rows) = (self.adjustment.controls, self.rows);
        if draw.len() != rows || xi.len() != self.within_units.len() {
            return Err(invalid("a pseudo-noise control draw has the wrong length"));
        }
        let slope = (0..q)
            .map(|control| {
                let mut value = StableAccumulator::default();
                for row in 0..rows {
                    value.add(self.adjustment.residualized[control * rows + row] * draw[row]);
                }
                for (position, &row) in self.within_units.iter().enumerate() {
                    value.add(self.adjustment.within[control * rows + row] * xi[position]);
                }
                value.finish()
            })
            .collect::<Vec<_>>();
        let coefficient = self.precision_times(&slope);
        for row in 0..rows {
            let mut value = draw[row];
            for (control, &weight) in coefficient.iter().enumerate() {
                value -= self.adjustment.collapsed[control * rows + row] * weight;
            }
            if !value.is_finite() {
                return Err(nonfinite("a transformed pseudo-noise draw is nonfinite"));
            }
            draw[row] = value;
        }
        Ok(())
    }

    /// `Pi x` and `Lambda Z' x` of one pseudo-noise image.
    pub fn image(&self, image: &[f64]) -> Result<RepeatControlImage> {
        if image.len() != self.rows {
            return Err(invalid("a pseudo-noise image has the wrong length"));
        }
        let coefficient = self.coefficient(image);
        Ok(RepeatControlImage {
            projected: self.star(image, &coefficient),
            coefficient,
        })
    }

    /// The bilinear form of one image with itself:
    /// `sum_a v_a (Pi x)_a^2 + (Lambda Z' x)' W'W (Lambda Z' x)`.
    pub fn image_square(&self, variance: &[f64], image: &[f64]) -> Result<f64> {
        if variance.len() != self.rows {
            return Err(invalid("a pseudo-noise variance has the wrong length"));
        }
        let parts = self.image(image)?;
        let mut value = StableAccumulator::default();
        for row in 0..self.rows {
            value.add(variance[row] * parts.projected[row] * parts.projected[row]);
        }
        let value = value.finish() + self.quadratic(&parts.coefficient, &parts.coefficient);
        if !value.is_finite() {
            return Err(nonfinite("a pseudo-noise control statistic is nonfinite"));
        }
        Ok(value)
    }

    /// Fitted and leave-out variance of the leading score `d'y(g)`, with the
    /// starred direction and its within-unit part.
    fn score_parts(
        &self,
        mode: &[f64],
        variance: &[f64],
        proxy: &[f64],
    ) -> (f64, f64, Vec<f64>, Vec<f64>) {
        let coefficient = self.coefficient(mode);
        let star = self.star(mode, &coefficient);
        let within = self.within(&coefficient, -1.0);
        let mut fitted = StableAccumulator::default();
        let mut leave_out = StableAccumulator::default();
        for row in 0..self.rows {
            let shared =
                2.0 * self.leave_out[row] * star[row] * within[row] + within[row] * within[row];
            fitted.add(variance[row] * star[row] * star[row] + shared);
            leave_out.add(proxy[row] * star[row] * star[row] + shared);
        }
        (fitted.finish(), leave_out.finish(), star, within)
    }

    /// Fitted variance of the leading score, as `finish_repeat_q1_target`
    /// forms it.
    pub fn score_fitted_variance(
        &self,
        mode: &[f64],
        variance: &[f64],
        proxy: &[f64],
    ) -> Result<f64> {
        if mode.len() != self.rows || variance.len() != self.rows || proxy.len() != self.rows {
            return Err(invalid("leading-score inputs have the wrong length"));
        }
        Ok(self.score_parts(mode, variance, proxy).0)
    }
}

/// The linear block of the control term in the high-rank covariance,
/// row-major: the starred influences, their cross term with the within-unit
/// parts, and the within-unit term.
#[derive(Clone, Debug, PartialEq)]
pub struct RepeatControlTerms {
    pub linear_unit: [f64; SQUARE],
    pub linear_cross: [f64; SQUARE],
    pub linear_within: [f64; SQUARE],
}

/// What the `q = 1` pieces of one target need for the control term.
#[derive(Clone, Copy, Debug)]
pub struct RepeatQ1Control<'a> {
    pub control: &'a RepeatControl<'a>,
    /// Leave-out products `y_a e_a / M_aa`.
    pub proxy: &'a [f64],
}

/// The `q = 1` decomposition of one target with repeat-measurement variances.
///
/// `mode` is the direction split off, `influence` the realized influence of
/// the remainder and `remainder_trace` the pseudo-noise estimate of
/// `tr(K_r V K_r V)`. The variance of the leading score is the larger of the
/// leave-out estimate, which also recentres the leading square, and the value
/// from the unit variances; the leave-out estimate is unbiased but rests on
/// the few units that carry the direction and can be far too small or
/// negative. The remainder's variance is the high-rank form with its own pair
/// term. Nothing is truncated: a nonpositive remainder variance or a singular
/// pair is a status of the target.
///
/// With `control`, the score's variances, the covariance and the remainder's
/// linear part carry the first-order term of the estimated control
/// coefficients, and `remainder_trace` is the bilinear form of the remainder
/// images of the transformed draws. The decomposition itself, including the
/// leave-out correction of the leading square, is the same.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
pub fn finish_repeat_q1_target(
    point_estimate: f64,
    leading_score: f64,
    leading_variance_leave_out: f64,
    direct_remainder_estimate: f64,
    remainder_identity_tolerance: f64,
    eigenvalue: f64,
    mode: &[f64],
    influence: &[f64],
    variance: &[f64],
    remainder_trace: f64,
    remainder_trace_mcse: f64,
    remainder_pair_term: f64,
    psd_tolerance: f64,
    control: Option<RepeatQ1Control<'_>>,
) -> Result<ComponentQ1TargetResult> {
    if mode.len() != variance.len()
        || influence.len() != variance.len()
        || control.is_some_and(|control| {
            control.control.rows != variance.len() || control.proxy.len() != variance.len()
        })
        || [
            point_estimate,
            leading_score,
            leading_variance_leave_out,
            direct_remainder_estimate,
            remainder_identity_tolerance,
            eigenvalue,
            remainder_trace,
            remainder_trace_mcse,
            remainder_pair_term,
            psd_tolerance,
        ]
        .iter()
        .any(|value| !value.is_finite())
        || remainder_trace_mcse < 0.0
        || psd_tolerance < 0.0
        || remainder_identity_tolerance < 0.0
    {
        return Err(invalid("repeat-measurement q=1 inputs are invalid"));
    }
    let remainder_trace_variance = 2.0 * remainder_trace;
    let (leading_variance, linear, largest, remainder_variance, covariance) = match control {
        None => {
            let mut fitted = StableAccumulator::default();
            let mut linear = StableAccumulator::default();
            let mut cross = StableAccumulator::default();
            let mut largest = 0.0_f64;
            for row in 0..variance.len() {
                fitted.add(mode[row] * mode[row] * variance[row]);
                let contribution = 4.0 * influence[row] * influence[row] * variance[row];
                linear.add(contribution);
                largest = largest.max(contribution);
                cross.add(2.0 * mode[row] * variance[row] * influence[row]);
            }
            let leading_variance = fitted.finish().max(leading_variance_leave_out);
            let linear = linear.finish();
            let remainder_variance = linear - remainder_trace_variance + remainder_pair_term;
            (
                leading_variance,
                linear,
                largest,
                remainder_variance,
                cross.finish(),
            )
        }
        Some(control) => {
            let parts = control.control;
            let (fitted, leave_out, score_star, score_within) =
                parts.score_parts(mode, variance, control.proxy);
            let (star, within) = parts.target_parts(influence);
            let mut linear = StableAccumulator::default();
            let mut cross = StableAccumulator::default();
            let mut largest = 0.0_f64;
            for row in 0..variance.len() {
                let residual = parts.leave_out[row];
                let contribution = 4.0 * variance[row] * star[row] * star[row]
                    + 4.0 * residual * star[row] * within[row]
                    + within[row] * within[row];
                linear.add(contribution);
                largest = largest.max(contribution);
                cross.add(
                    2.0 * variance[row] * score_star[row] * star[row]
                        + residual
                            * (score_star[row] * within[row] + 2.0 * star[row] * score_within[row])
                        + score_within[row] * within[row],
                );
            }
            let linear = linear.finish();
            // `remainder_trace` is the bilinear form of the remainder images.
            let remainder_variance = linear - remainder_trace_variance + remainder_pair_term;
            (
                fitted.max(leave_out),
                linear,
                largest,
                remainder_variance,
                cross.finish(),
            )
        }
    };
    let correlation = covariance / leading_variance.sqrt() / remainder_variance.sqrt();
    let standardized_determinant = (1.0 - correlation.abs()) * (1.0 + correlation.abs());
    let status = if leading_variance <= 0.0 || remainder_variance <= 0.0 {
        ComponentQ1Status::NonpositiveVariance
    } else if !standardized_determinant.is_finite() || standardized_determinant <= psd_tolerance {
        ComponentQ1Status::SingularCovariance
    } else {
        ComponentQ1Status::Computed
    };
    let curvature = if status == ComponentQ1Status::Computed {
        2.0 * eigenvalue.abs() * leading_variance
            / (remainder_variance * standardized_determinant).sqrt()
    } else {
        f64::NAN
    };
    let leading_recentered_component =
        eigenvalue * (leading_score * leading_score - leading_variance_leave_out);
    let remainder_estimate = point_estimate - leading_recentered_component;
    let remainder_identity_error = (remainder_estimate - direct_remainder_estimate).abs();
    let scale = point_estimate
        .abs()
        .max(remainder_estimate.abs())
        .max(direct_remainder_estimate.abs())
        .max(1.0);
    if remainder_identity_error > remainder_identity_tolerance * scale {
        return Err(BackendError::new(
            ErrorCode::TargetIdentityFailed,
            "repeat_measurement_q1",
            "the q=1 leave-out recentering does not reproduce the rank-one-subtracted target",
        ));
    }
    let output = ComponentQ1TargetResult {
        status,
        point_estimate,
        leading_score,
        leading_variance_correction: leading_variance_leave_out,
        leading_variance,
        leading_recentered_component,
        remainder_estimate,
        remainder_identity_error,
        leading_remainder_covariance: covariance,
        remainder_variance,
        remainder_trace_mcse: 2.0 * remainder_trace_mcse,
        standardized_determinant,
        remainder_influence_variance: linear,
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
        remainder_influence_concentration: if linear > 0.0 { largest / linear } else { 0.0 },
    };
    if [
        output.leading_recentered_component,
        output.remainder_estimate,
        output.leading_variance,
        output.remainder_variance,
        output.leading_remainder_covariance,
    ]
    .iter()
    .any(|value| !value.is_finite())
    {
        return Err(nonfinite("a repeat-measurement q=1 result is nonfinite"));
    }
    Ok(output)
}

fn constraint(message: impl Into<String>) -> BackendError {
    BackendError::new(
        ErrorCode::JlaConstraintFailed,
        "repeat_measurement_variance",
        message,
    )
}

/// Mass-weighted mean and standard deviation over all units, that is, over
/// the physical observations of movers and stayers alike.
fn standardise(
    values: &[f64],
    mass: &[f64],
    interrupt: &mut dyn InterruptCheck,
) -> Result<(f64, f64)> {
    let mut total = StableAccumulator::default();
    let mut sum = StableAccumulator::default();
    for row in 0..values.len() {
        checkpoint_chunk(interrupt, row, "repeat_measurement_standardise")?;
        total.add(mass[row]);
        sum.add(mass[row] * values[row]);
    }
    let total = total.finish();
    if !total.is_finite() || total <= 0.0 {
        return Err(invalid("unit mass is not positive"));
    }
    let mean = sum.finish() / total;
    let mut square = StableAccumulator::default();
    for row in 0..values.len() {
        checkpoint_chunk(interrupt, row, "repeat_measurement_standardise")?;
        let deviation = values[row] - mean;
        square.add(mass[row] * deviation * deviation);
    }
    let deviation = (square.finish() / total).sqrt();
    if !mean.is_finite() || !deviation.is_finite() {
        return Err(nonfinite("an estimated effect is nonfinite"));
    }
    Ok((mean, deviation))
}

/// Sample variances of the repeat-measurement sets, the two-step model of the
/// units outside sets, and the closed-form term for the pairs of units of one
/// set.
#[allow(clippy::too_many_lines)]
pub fn fit_repeat_variance(
    input: RepeatInputs<'_>,
    interrupt: &mut dyn InterruptCheck,
) -> Result<RepeatVariance> {
    let rows = input.proxy.len();
    if rows == 0
        || [
            input.outcome.len(),
            input.residual.len(),
            input.maker_inverse.len(),
            input.leverage.len(),
            input.mass.len(),
            input.set.len(),
            input.mover.len(),
            input.worker_effect.len(),
            input.firm_effect.len(),
            input.target_diagonal[0].len(),
            input.target_diagonal[1].len(),
            input.target_diagonal[2].len(),
        ]
        .iter()
        .any(|&length| length != rows)
        || !input.positivity_multiplier.is_finite()
        || input.positivity_multiplier <= 0.0
    {
        return Err(invalid("repeat-measurement inputs are inconsistent"));
    }

    // The positivity floor of the other fitted variance sources.
    let mut scale = Vec::with_capacity(rows);
    for row in 0..rows {
        checkpoint_chunk(interrupt, row, "repeat_measurement_floor")?;
        scale.push(input.residual[row] * input.residual[row] * input.maker_inverse[row]);
    }
    let middle = rows / 2;
    scale.select_nth_unstable_by(middle, f64::total_cmp);
    let floor = input.positivity_multiplier * scale[middle];
    drop(scale);
    if !floor.is_finite() || floor <= 0.0 {
        return Err(constraint(
            "the residual scale for the variance floor is not positive",
        ));
    }

    // Sets: the sample variance of the outcomes of one cell, and the pair
    // term. Units alone in their cell are left for the model.
    let mut variance = vec![0.0; rows];
    let mut order = (0..rows).collect::<Vec<_>>();
    order.sort_unstable_by_key(|&unit| (input.set[unit], unit));
    let mut single = Vec::new();
    let mut recorded = RepeatSets::default();
    if u32::try_from(rows).is_err() {
        return Err(invalid(
            "repeat-measurement units exceed the supported count",
        ));
    }
    let mut adjustment = [StableAccumulator::default(); SQUARE];
    let mut sets = 0_u64;
    let mut set_units = 0_u64;
    let mut set_floored = 0_u64;
    let mut begin = 0;
    while begin < order.len() {
        checkpoint_chunk(interrupt, begin, "repeat_measurement_sets")?;
        let set = input.set[order[begin]];
        let mut end = begin + 1;
        while end < order.len() && input.set[order[end]] == set {
            end += 1;
        }
        let units = &order[begin..end];
        begin = end;
        let mover = input.mover[units[0]];
        if units.iter().any(|&unit| input.mover[unit] != mover) {
            return Err(BackendError::invariant(
                "repeat_measurement_variance",
                "the units of one cell disagree on their worker's mobility",
            ));
        }
        if units.len() < 2 {
            if !mover {
                return Err(BackendError::invariant(
                    "repeat_measurement_variance",
                    "a stayer has fewer than two observations",
                ));
            }
            single.push(units[0]);
            continue;
        }
        let count = units.len() as f64;
        let mut sum = StableAccumulator::default();
        let mut diagonal = [StableAccumulator::default(); PRIMITIVE_TARGETS];
        for &unit in units {
            sum.add(input.outcome[unit]);
            if mover {
                // Two units a, b of one cell share the weight
                // C_ab = B_aa / (1 - P_aa), with B and P common to the cell.
                for target in 0..PRIMITIVE_TARGETS {
                    diagonal[target]
                        .add(input.target_diagonal[target][unit] * input.maker_inverse[unit]);
                }
            } else {
                diagonal[0].add(input.target_diagonal[0][unit]);
            }
        }
        let mean = sum.finish() / count;
        let mut square = StableAccumulator::default();
        for &unit in units {
            let deviation = input.outcome[unit] - mean;
            square.add(deviation * deviation);
        }
        let sample = square.finish() / (count - 1.0);
        // A stayer's leverage is exactly 1 / T, so its weight is
        // B_aa T / (T - 1), and its firm and covariance diagonals vanish.
        let pair = if mover {
            [
                diagonal[0].finish() / count,
                diagonal[1].finish() / count,
                diagonal[2].finish() / count,
            ]
        } else {
            [diagonal[0].finish() / (count - 1.0), 0.0, 0.0]
        };
        let factor = 4.0 * sample * sample * (count - 2.0) * (count - 1.0) / (count + 1.0);
        if !sample.is_finite() || !factor.is_finite() || pair.iter().any(|value| !value.is_finite())
        {
            return Err(nonfinite(
                "a repeat-measurement sample variance is nonfinite",
            ));
        }
        for left in 0..PRIMITIVE_TARGETS {
            for right in left..PRIMITIVE_TARGETS {
                if pair[left] != 0.0 && pair[right] != 0.0 {
                    adjustment[left * PRIMITIVE_TARGETS + right]
                        .add(factor * pair[left] * pair[right]);
                }
            }
        }
        if sample < floor {
            set_floored += units.len() as u64;
        }
        for &unit in units {
            variance[unit] = sample.max(floor);
            recorded.members.push(unit as u32);
        }
        recorded.end.push(recorded.members.len() as u32);
        recorded.sample.push(sample);
        recorded.mover.push(mover);
        sets += 1;
        set_units += units.len() as u64;
    }
    drop(order);
    single.sort_unstable();

    let mut scale_terms = 0;
    let mut profile_terms = 0;
    let mut profile = [0.0; PROFILE_TERMS];
    let mut scale_floor = 0.0;
    let mut scale_floored = 0_u64;
    let mut floored = 0_u64;
    let mut fitted_units = 0_u64;
    if !single.is_empty() {
        // The model is fitted where it is used. Too few units alone in their
        // cell cannot carry it; all units of movers then do. Under match
        // deletion the two samples coincide.
        let borrowed = single.len() < UNITS_PER_WORKING_TERM;
        let sample = if borrowed {
            (0..rows)
                .filter(|&unit| input.mover[unit])
                .collect::<Vec<_>>()
        } else {
            single.clone()
        };
        let fitted_count = sample.len();
        fitted_units = fitted_count as u64;
        let mut used = vec![!borrowed; fitted_count];
        if borrowed {
            let mut next = 0;
            for (position, &unit) in sample.iter().enumerate() {
                if next < single.len() && single[next] == unit {
                    used[position] = true;
                    next += 1;
                }
            }
        }
        let gather = |values: &[f64]| sample.iter().map(|&unit| values[unit]).collect::<Vec<_>>();

        // Step 1: the scale, on ranks of leverage and of unit mass.
        let leverage_rank = normalized_midranks(&gather(input.leverage), interrupt)?;
        let mass_rank = normalized_midranks(&gather(input.mass), interrupt)?;
        let permitted = (fitted_count / UNITS_PER_WORKING_TERM).clamp(1, MAXIMUM_WORKING_TERMS);
        let scale_basis = |position: usize| -> [f64; MAXIMUM_WORKING_TERMS] {
            let (lever, mass) = (leverage_rank[position], mass_rank[position]);
            [1.0, lever, mass, lever * lever, mass * mass]
        };
        let mut gram =
            [[StableAccumulator::default(); MAXIMUM_WORKING_TERMS]; MAXIMUM_WORKING_TERMS];
        let mut moment = [StableAccumulator::default(); MAXIMUM_WORKING_TERMS];
        let mut mean = StableAccumulator::default();
        for (position, &unit) in sample.iter().enumerate() {
            checkpoint_chunk(interrupt, position, "repeat_measurement_scale")?;
            let value = input.proxy[unit];
            if !value.is_finite() {
                return Err(nonfinite("a leave-out variance product is nonfinite"));
            }
            mean.add(value);
            let row = scale_basis(position);
            for left in 0..permitted {
                moment[left].add(row[left] * value);
                for right in left..permitted {
                    gram[left][right].add(row[left] * row[right]);
                }
            }
        }
        let mean = mean.finish() / fitted_count as f64;
        if !mean.is_finite() || mean <= 0.0 {
            return Err(constraint(
                "the mean leave-out variance product of the fitted units is not positive",
            ));
        }
        scale_floor = SCALE_FLOOR_SHARE * mean;
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
        scale_terms = active;
        let mut fitted_scale = Vec::with_capacity(fitted_count);
        for position in 0..fitted_count {
            checkpoint_chunk(interrupt, position, "repeat_measurement_scale")?;
            let row = scale_basis(position);
            let mut value = StableAccumulator::default();
            for term in 0..permitted {
                value.add(row[term] * coefficient[term]);
            }
            let value = value.finish();
            if !value.is_finite() {
                return Err(nonfinite("a fitted scale is nonfinite"));
            }
            if used[position] && value < scale_floor {
                scale_floored += 1;
            }
            fitted_scale.push(value.max(scale_floor));
        }

        // Step 2: the profile, in the standardised and limited effects.
        let (worker_mean, worker_deviation) =
            standardise(input.worker_effect, input.mass, interrupt)?;
        let (firm_mean, firm_deviation) = standardise(input.firm_effect, input.mass, interrupt)?;
        let limited = |value: f64, mean: f64, deviation: f64| -> f64 {
            if deviation > 0.0 {
                ((value - mean) / deviation).clamp(-PROFILE_CAP, PROFILE_CAP)
            } else {
                0.0
            }
        };
        let profile_basis = |unit: usize| -> [f64; PROFILE_TERMS] {
            let a = limited(input.worker_effect[unit], worker_mean, worker_deviation);
            let p = limited(input.firm_effect[unit], firm_mean, firm_deviation);
            [1.0, a, a * a, p, p * p, a * p]
        };
        let permitted = (fitted_count / UNITS_PER_WORKING_TERM).clamp(1, PROFILE_TERMS);
        let mut gram = [[StableAccumulator::default(); PROFILE_TERMS]; PROFILE_TERMS];
        let mut moment = [StableAccumulator::default(); PROFILE_TERMS];
        for (position, &unit) in sample.iter().enumerate() {
            checkpoint_chunk(interrupt, position, "repeat_measurement_profile")?;
            let row = profile_basis(unit);
            let weight = fitted_scale[position];
            for left in 0..permitted {
                moment[left].add(weight * row[left] * input.proxy[unit]);
                for right in left..permitted {
                    gram[left][right].add(weight * weight * row[left] * row[right]);
                }
            }
        }
        let mut matrix = [[0.0; PROFILE_TERMS]; PROFILE_TERMS];
        let mut rhs = [0.0; PROFILE_TERMS];
        for left in 0..permitted {
            rhs[left] = moment[left].finish();
            for right in left..permitted {
                matrix[left][right] = gram[left][right].finish();
                matrix[right][left] = matrix[left][right];
            }
        }
        let (coefficient, active) = solve_reduced(&matrix, &rhs, permitted)?;
        profile_terms = active;
        profile = coefficient;
        for (position, &unit) in sample.iter().enumerate() {
            checkpoint_chunk(interrupt, position, "repeat_measurement_profile")?;
            if !used[position] {
                continue;
            }
            let row = profile_basis(unit);
            let mut value = StableAccumulator::default();
            for term in 0..permitted {
                value.add(row[term] * coefficient[term]);
            }
            let value = fitted_scale[position] * value.finish();
            if !value.is_finite() {
                return Err(nonfinite("a fitted variance is nonfinite"));
            }
            if value < floor {
                floored += 1;
            }
            variance[unit] = value.max(floor);
        }
        if floored == single.len() as u64 {
            return Err(constraint("every fitted variance was floored"));
        }
    }

    let mut output = [0.0; SQUARE];
    for left in 0..PRIMITIVE_TARGETS {
        for right in left..PRIMITIVE_TARGETS {
            let value = adjustment[left * PRIMITIVE_TARGETS + right].finish();
            if !value.is_finite() {
                return Err(nonfinite("the pair term is nonfinite"));
            }
            output[left * PRIMITIVE_TARGETS + right] = value;
            output[right * PRIMITIVE_TARGETS + left] = value;
        }
    }
    Ok(RepeatVariance {
        variance,
        scale_terms,
        profile_terms,
        profile,
        scale_floor,
        scale_floored,
        floor,
        floored,
        stayers: sets,
        stayer_units: set_units,
        stayer_floored: set_floored,
        model_units: single.len() as u64,
        model_fit_units: fitted_units,
        adjustment: output,
        sets: recorded,
    })
}

/// Assemble the covariance from the realized influences, the unit variances
/// and the pseudo-noise trace.
///
/// `influence[t]` is `K_t z`. The reported covariance is
/// `4 sum_a v_a (K_t z)_a (K_s z)_a - 2 tr(K_t V K_s V) + adjustment`. It is
/// returned as computed: nothing is projected or truncated, so a diagonal
/// entry can be nonpositive in a small sample and is then reported as such.
/// `signal` holds the implied estimate of `4 (K_t mu)' V (K_s mu)`.
///
/// With `control`, the signal is the linear term of the starred influences,
/// their within-unit parts and the cross term between the two, and the trace
/// in `moments` is the bilinear form of the images of the transformed draws,
/// `tr(K_t S K_s S)` for the covariance `S` of the control-adjusted noise.
/// The linear pieces are returned beside the covariance.
pub fn finish_repeat_covariance(
    influence: &[Vec<f64>; PRIMITIVE_TARGETS],
    variance: &[f64],
    adjustment: &[f64; SQUARE],
    moments: &NoiseMoments,
    control: Option<&RepeatControl<'_>>,
    interrupt: &mut dyn InterruptCheck,
) -> Result<(ResidualCovariance, Option<RepeatControlTerms>)> {
    if let Some(control) = control {
        return finish_repeat_control_covariance(
            influence, variance, adjustment, moments, control, interrupt,
        );
    }
    let rows = variance.len();
    if influence.iter().any(|value| value.len() != rows)
        || variance
            .iter()
            .any(|value| !value.is_finite() || *value <= 0.0)
        || adjustment.iter().any(|value| !value.is_finite())
    {
        return Err(invalid("repeat-measurement covariance inputs are invalid"));
    }
    let mut square = [StableAccumulator::default(); SQUARE];
    let mut total = [StableAccumulator::default(); REPORTED_TARGETS];
    let mut largest = [0.0_f64; REPORTED_TARGETS];
    for row in 0..rows {
        checkpoint_chunk(interrupt, row, "repeat_measurement_signal")?;
        for left in 0..PRIMITIVE_TARGETS {
            for right in left..PRIMITIVE_TARGETS {
                square[left * PRIMITIVE_TARGETS + right]
                    .add(4.0 * variance[row] * influence[left][row] * influence[right][row]);
            }
        }
        for target in 0..REPORTED_TARGETS {
            let value = if target < PRIMITIVE_TARGETS {
                influence[target][row]
            } else {
                influence[0][row] + influence[1][row] + 2.0 * influence[2][row]
            };
            let value = 4.0 * variance[row] * value * value;
            total[target].add(value);
            largest[target] = largest[target].max(value);
        }
    }
    let mut signal = [0.0; SQUARE];
    let mut trace_term = [0.0; SQUARE];
    let mut trace_mcse = [0.0; SQUARE];
    let mut primitive = [0.0; SQUARE];
    for left in 0..PRIMITIVE_TARGETS {
        for right in left..PRIMITIVE_TARGETS {
            let index = left * PRIMITIVE_TARGETS + right;
            let mirror = right * PRIMITIVE_TARGETS + left;
            let value = square[index].finish() - 4.0 * moments.trace[index] + adjustment[index];
            if !value.is_finite() {
                return Err(nonfinite("the component signal covariance is nonfinite"));
            }
            for position in [index, mirror] {
                signal[position] = value;
                trace_term[position] = 2.0 * moments.trace[index];
                trace_mcse[position] = 2.0 * moments.trace_mcse[index];
                primitive[position] = value + trace_term[position];
            }
        }
    }
    let (_, smallest, largest_eigenvalue) = positive_part(&signal)?;
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
    Ok((
        ResidualCovariance {
            primitive,
            model_primitive: primitive,
            signal,
            trace_term,
            trace_mcse,
            signal_truncation: 0.0,
            smallest_signal_eigenvalue: smallest,
            largest_signal_eigenvalue: largest_eigenvalue,
            influence_concentration,
        },
        None,
    ))
}

fn finish_repeat_control_covariance(
    influence: &[Vec<f64>; PRIMITIVE_TARGETS],
    variance: &[f64],
    adjustment: &[f64; SQUARE],
    moments: &NoiseMoments,
    control: &RepeatControl<'_>,
    interrupt: &mut dyn InterruptCheck,
) -> Result<(ResidualCovariance, Option<RepeatControlTerms>)> {
    let rows = variance.len();
    if influence.iter().any(|value| value.len() != rows)
        || control.rows != rows
        || variance
            .iter()
            .any(|value| !value.is_finite() || *value <= 0.0)
        || adjustment.iter().any(|value| !value.is_finite())
    {
        return Err(invalid("repeat-measurement covariance inputs are invalid"));
    }
    let parts: [(Vec<f64>, Vec<f64>); PRIMITIVE_TARGETS] =
        core::array::from_fn(|target| control.target_parts(&influence[target]));
    let mut unit = [StableAccumulator::default(); SQUARE];
    let mut cross = [StableAccumulator::default(); SQUARE];
    let mut inside = [StableAccumulator::default(); SQUARE];
    let mut total = [StableAccumulator::default(); REPORTED_TARGETS];
    let mut largest = [0.0_f64; REPORTED_TARGETS];
    for row in 0..rows {
        checkpoint_chunk(interrupt, row, "repeat_measurement_signal")?;
        let residual = control.leave_out[row];
        for left in 0..PRIMITIVE_TARGETS {
            for right in left..PRIMITIVE_TARGETS {
                let index = left * PRIMITIVE_TARGETS + right;
                let (star_left, within_left) = (parts[left].0[row], parts[left].1[row]);
                let (star_right, within_right) = (parts[right].0[row], parts[right].1[row]);
                unit[index].add(4.0 * variance[row] * star_left * star_right);
                cross[index]
                    .add(2.0 * residual * (star_left * within_right + star_right * within_left));
                inside[index].add(within_left * within_right);
            }
        }
        for target in 0..REPORTED_TARGETS {
            let (star, within) = if target < PRIMITIVE_TARGETS {
                (parts[target].0[row], parts[target].1[row])
            } else {
                (
                    parts[0].0[row] + parts[1].0[row] + 2.0 * parts[2].0[row],
                    parts[0].1[row] + parts[1].1[row] + 2.0 * parts[2].1[row],
                )
            };
            let value = 4.0 * variance[row] * star * star
                + 4.0 * residual * star * within
                + within * within;
            total[target].add(value);
            largest[target] = largest[target].max(value);
        }
    }
    let mut signal = [0.0; SQUARE];
    let mut trace_term = [0.0; SQUARE];
    let mut trace_mcse = [0.0; SQUARE];
    let mut primitive = [0.0; SQUARE];
    let mut terms = RepeatControlTerms {
        linear_unit: [0.0; SQUARE],
        linear_cross: [0.0; SQUARE],
        linear_within: [0.0; SQUARE],
    };
    for left in 0..PRIMITIVE_TARGETS {
        for right in left..PRIMITIVE_TARGETS {
            let index = left * PRIMITIVE_TARGETS + right;
            let mirror = right * PRIMITIVE_TARGETS + left;
            let pieces = [
                unit[index].finish(),
                cross[index].finish(),
                inside[index].finish(),
            ];
            let value =
                pieces[0] + pieces[1] + pieces[2] - 4.0 * moments.trace[index] + adjustment[index];
            if !value.is_finite() {
                return Err(nonfinite("the component signal covariance is nonfinite"));
            }
            for position in [index, mirror] {
                signal[position] = value;
                trace_term[position] = 2.0 * moments.trace[index];
                trace_mcse[position] = 2.0 * moments.trace_mcse[index];
                primitive[position] = value + trace_term[position];
                terms.linear_unit[position] = pieces[0];
                terms.linear_cross[position] = pieces[1];
                terms.linear_within[position] = pieces[2];
            }
        }
    }
    let (_, smallest, largest_eigenvalue) = positive_part(&signal)?;
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
    Ok((
        ResidualCovariance {
            primitive,
            model_primitive: primitive,
            signal,
            trace_term,
            trace_mcse,
            signal_truncation: 0.0,
            smallest_signal_eigenvalue: smallest,
            largest_signal_eigenvalue: largest_eigenvalue,
            influence_concentration,
        },
        Some(terms),
    ))
}

#[cfg(test)]
mod tests;
