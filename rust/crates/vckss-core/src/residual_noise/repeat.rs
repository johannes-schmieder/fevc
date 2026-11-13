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
    invalid, nonfinite, positive_part, solve_reduced, NoiseMoments, ResidualCovariance,
    StableAccumulator, MAXIMUM_WORKING_TERMS, SQUARE, UNITS_PER_WORKING_TERM,
};
use crate::component_inference::{PRIMITIVE_TARGETS, REPORTED_TARGETS};
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
        }
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
pub fn finish_repeat_covariance(
    influence: &[Vec<f64>; PRIMITIVE_TARGETS],
    variance: &[f64],
    adjustment: &[f64; SQUARE],
    moments: &NoiseMoments,
    interrupt: &mut dyn InterruptCheck,
) -> Result<ResidualCovariance> {
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
    Ok(ResidualCovariance {
        primitive,
        model_primitive: primitive,
        signal,
        trace_term,
        trace_mcse,
        signal_truncation: 0.0,
        smallest_signal_eigenvalue: smallest,
        largest_signal_eigenvalue: largest_eigenvalue,
        influence_concentration,
    })
}

#[cfg(test)]
mod tests;
