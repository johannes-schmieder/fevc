// SPDX-License-Identifier: GPL-3.0-only

//! Shared residual-moment variance fitter for independent scalar inference units.
//!
//! For independent unit-frequency errors with variance Z gamma, e=(I-P)y gives
//! E[Z'e^2] = K gamma, K=Z'((I-P) elementwise-squared)Z. We approximate K using
//! Cov{Z'((I-P)g)^2}/2 for numerical Gaussian g. The older native policies retain
//! Z'diag(1-2h)Z + Cov{Z'(Pg)^2}/2. The caller supplies an outcome-free basis
//! and compatible projection leverages (JLA estimates in public attachments).
//! The direct representation avoids a subtractive estimated-leverage term. Neither
//! approximate diagonals, probe inversion nor prediction flooring inherits the
//! exact-K unbiasedness claim. A match unit is collapsed before squaring, using
//! its fixed-offset FE projection. This fitter does not alter point corrections
//! or q1 recentering; current validation scope lives in the package plan.

use crate::dense::invert_scaled_spd;
use crate::error::{BackendError, ErrorCode, Result};
use crate::interrupt::{checkpoint_chunk, unstable_sort_by_with_interrupt, InterruptCheck};
use crate::rng::{CounterRng, ProbeDomain, MAX_PHYSICAL_WORDS_PER_ATOM};

const PHASE: &str = "observation_residual_moments";
const MAX_TERMS: usize = 21;
const SMALL_RESIDUAL_GATE: f64 = 1.0e-9;

pub(crate) mod basis;

/// Internal numerical representation; native V1–V3 policies remain unchanged.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GramMethod {
    LegacyProjectedCovariance,
    DirectResidualCovariance,
}

#[derive(Clone, Copy, Debug)]
pub struct ResidualMomentOptions {
    pub seed: u64,
    pub probes: usize,
    pub batch_width: usize,
    pub gram_method: GramMethod,
    pub rank_tolerance: f64,
    pub positivity_multiplier: f64,
    pub observations_per_term: usize,
    pub effective_projection_tolerance: f64,
    /// Budget for this candidate plus the callback's additional peak, not the
    /// whole command. The caller must separately admit already-retained inputs
    /// and solver state, and reserve this increment before entering this API.
    pub additional_memory_limit_bytes: usize,
    /// Caller-certified extra peak used inside the projection callback.
    pub projection_workspace_bytes: usize,
}

impl Default for ResidualMomentOptions {
    fn default() -> Self {
        Self {
            seed: 1,
            probes: 512,
            batch_width: 16,
            gram_method: GramMethod::LegacyProjectedCovariance,
            rank_tolerance: 1.0e-10,
            positivity_multiplier: 1.0e-8,
            observations_per_term: 5,
            effective_projection_tolerance: 1.0e-10,
            additional_memory_limit_bytes: 64 * 1024 * 1024,
            projection_workspace_bytes: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResidualMomentMemory {
    /// Conservative envelope for all candidate-owned heaps and fixed work.
    pub candidate_peak_bytes: usize,
    pub projection_workspace_bytes: usize,
    pub additional_peak_bytes: usize,
    /// Borrowed basis, h and addresses; excludes caller outcome/solver state.
    pub borrowed_input_bytes: usize,
}

/// Preflight without allocation or RNG. No solver workspace is inferred from
/// dimensions: its actual route belongs to the caller's existing frozen plan.
pub fn memory_plan(
    rows: usize,
    terms: usize,
    options: ResidualMomentOptions,
) -> Result<ResidualMomentMemory> {
    let memory = memory_envelope(rows, terms, options)?;
    if rows < checked_mul(terms, options.observations_per_term)? {
        return Err(invalid("insufficient observations per variance term"));
    }
    Ok(memory)
}

/// Admission envelope for a not-yet-reduced outcome-free basis. The actual
/// fit still enforces the support requirement on its active column count.
pub(crate) fn memory_envelope(
    rows: usize,
    terms: usize,
    options: ResidualMomentOptions,
) -> Result<ResidualMomentMemory> {
    if rows == 0
        || rows as u128 > (1_u128 << 53)
        || !(1..=MAX_TERMS).contains(&terms)
        || options.probes < 2
        || options.probes as u128 > (1_u128 << 53)
        || options.batch_width == 0
        || options.observations_per_term == 0
        || !options.rank_tolerance.is_finite()
        || !(1.0e-14..0.1).contains(&options.rank_tolerance)
        || !options.positivity_multiplier.is_finite()
        || !(0.0..1.0).contains(&options.positivity_multiplier)
        || options.positivity_multiplier == 0.0
        || !options.effective_projection_tolerance.is_finite()
        || !(0.0..0.1).contains(&options.effective_projection_tolerance)
        || options.effective_projection_tolerance == 0.0
    {
        return Err(invalid("invalid dimensions or residual-moment options"));
    }
    // At fit time: HC2 scale/sort buffer and raw/positive predictions. At
    // preparation: two probe batches, small covariance/inverse work and a
    // residual certificate per column. The sum bounds either lifecycle peak.
    let width = options.batch_width.min(options.probes);
    let row_words = checked_mul(rows, checked_add(checked_mul(2, width)?, 4)?)?;
    let small_words = checked_add(checked_mul(64, checked_mul(terms, terms)?)?, width)?;
    let candidate_peak_bytes = checked_add(
        checked_mul(8, checked_add(row_words, small_words)?)?,
        16 * 1024,
    )?;
    let additional_peak_bytes =
        checked_add(candidate_peak_bytes, options.projection_workspace_bytes)?;
    let borrowed_input_bytes = checked_mul(rows, checked_add(checked_mul(8, terms)?, 24)?)?;
    let atoms = u64::try_from(checked_mul(rows, options.probes)?)
        .map_err(|_| resource("probe atom count overflow"))?;
    atoms
        .checked_mul(2)
        .ok_or_else(|| resource("probe word count overflow"))?;
    if additional_peak_bytes > options.additional_memory_limit_bytes {
        return Err(resource(
            "residual-moment additional memory exceeds admitted budget",
        ));
    }
    Ok(ResidualMomentMemory {
        candidate_peak_bytes,
        projection_workspace_bytes: options.projection_workspace_bytes,
        additional_peak_bytes,
        borrowed_input_bytes,
    })
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResidualMomentPreparationDiagnostic {
    pub probes: usize,
    pub counter_atoms: u64,
    pub counter_words: u64,
    pub maximum_full_residual: f64,
    pub full_residual_gate: f64,
    pub gram_rcond: f64,
    pub gram_inverse_relres: f64,
    pub maximum_leverage: f64,
    pub residual_degrees_of_freedom: f64,
    pub memory: ResidualMomentMemory,
}

#[derive(Debug)]
pub struct PreparedResidualMoments<'a> {
    basis: &'a [f64],
    leverage: &'a [f64],
    terms: usize,
    gram: Vec<f64>,
    inverse: Vec<f64>,
    positivity_multiplier: f64,
    pub diagnostic: ResidualMomentPreparationDiagnostic,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ResidualMomentFit {
    pub coefficients: Vec<f64>,
    pub raw_variance: Vec<f64>,
    pub positive_variance: Vec<f64>,
    pub nonpositive_predictions: usize,
    pub floored_predictions: usize,
    pub positivity_floor: f64,
    pub moment_relative_residual: f64,
}

/// Prepare once per fixed design/numerical seed, before fitting any outcome.
///
/// `basis` is row-major, with a constant first column and at most 21 terms.
/// Intended callers supply the existing leverage/common rank-polynomial basis;
/// this API neither selects a model nor drops collinear columns. `addresses`
/// contains strictly increasing canonical (entity, subdraw) pairs: replicated
/// design rows require distinct subdraws, not duplicate Gaussian atoms.
///
/// The callback writes P times each column of the column-major input and one
/// complete **original-system** relative residual per column. It must use the
/// identical full model, preserve its quotient, honor cancellation and its
/// declared workspace allowance, and fail closed without rerouting. The API
/// validates certificates but cannot independently verify a foreign callback.
pub fn prepare_with_interrupt<'a, F>(
    basis: &'a [f64],
    terms: usize,
    exact_leverage: &'a [f64],
    addresses: &[(u64, u64)],
    options: ResidualMomentOptions,
    mut project: F,
    interrupt: &mut dyn InterruptCheck,
) -> Result<PreparedResidualMoments<'a>>
where
    F: FnMut(&[f64], usize, &mut [f64], &mut [f64], &mut dyn InterruptCheck) -> Result<()>,
{
    let rows = exact_leverage.len();
    let memory = memory_plan(rows, terms, options)?;
    validate_inputs(basis, terms, exact_leverage, addresses, interrupt)?;
    let mut direct = vec![Sum::default(); terms * terms];
    let mut basis_gram = vec![Sum::default(); terms * terms];
    let mut df = Sum::default();
    let mut maximum_leverage: f64 = 0.0;
    for row in 0..rows {
        checkpoint_chunk(interrupt, row, PHASE)?;
        let h = exact_leverage[row];
        df.add(1.0 - h);
        maximum_leverage = maximum_leverage.max(h);
        for left in 0..terms {
            for right in 0..terms {
                let product = basis[row * terms + left] * basis[row * terms + right];
                basis_gram[left * terms + right].add(product);
                if options.gram_method == GramMethod::LegacyProjectedCovariance {
                    direct[left * terms + right].add((1.0 - 2.0 * h) * product);
                }
            }
        }
    }
    let basis_gram = basis_gram.into_iter().map(Sum::value).collect::<Vec<_>>();
    // Structural column deficiency is rejected before any numerical draw.
    invert_scaled_spd(&basis_gram, terms, options.rank_tolerance, interrupt, PHASE)?;
    drop(basis_gram);
    let width = options.batch_width.min(options.probes);
    let mut gaussian = zeroed(checked_mul(rows, width)?, interrupt)?;
    let mut projected = zeroed(gaussian.len(), interrupt)?;
    let mut residuals = zeroed(width, interrupt)?;
    let mut moments = CenteredMoments::new(terms);
    let full_residual_gate = (10.0 * options.effective_projection_tolerance).max(1.0e-11);
    let mut maximum_full_residual: f64 = 0.0;
    let rng = CounterRng::new(options.seed);
    for first in (0..options.probes).step_by(width) {
        interrupt.checkpoint("observation_residual_moment_probes")?;
        let columns = width.min(options.probes - first);
        for column in 0..columns {
            for (row, &(entity, subdraw)) in addresses.iter().enumerate() {
                checkpoint_chunk(interrupt, row, PHASE)?;
                gaussian[column * rows + row] =
                    normal(rng, (first + column) as u64, entity, subdraw);
                // Detect partial callbacks even when the previous batch was valid.
                projected[column * rows + row] = f64::NAN;
            }
            residuals[column] = f64::NAN;
        }
        project(
            &gaussian[..rows * columns],
            columns,
            &mut projected[..rows * columns],
            &mut residuals[..columns],
            interrupt,
        )?;
        for column in 0..columns {
            let residual = residuals[column];
            if !residual.is_finite() || residual < 0.0 || residual > full_residual_gate {
                return Err(BackendError::new(
                    ErrorCode::FullResidualFailed,
                    PHASE,
                    "projected probe lacks an acceptable complete-system residual",
                ));
            }
            maximum_full_residual = maximum_full_residual.max(residual);
            let mut value = [Sum::default(); MAX_TERMS];
            for row in 0..rows {
                checkpoint_chunk(interrupt, row, PHASE)?;
                let index = column * rows + row;
                let square = match options.gram_method {
                    GramMethod::LegacyProjectedCovariance => projected[index].powi(2),
                    GramMethod::DirectResidualCovariance => {
                        (gaussian[index] - projected[index]).powi(2)
                    }
                };
                if !square.is_finite() {
                    return Err(numerical("nonfinite projected probe"));
                }
                for term in 0..terms {
                    value[term].add(basis[row * terms + term] * square);
                }
            }
            moments.push(&value[..terms])?;
        }
    }
    let gram = direct
        .into_iter()
        .zip(moments.cross)
        .map(|(d, c)| d.value() + 0.5 * c.value() / (options.probes - 1) as f64)
        .collect::<Vec<_>>();
    let inverse = invert_scaled_spd(&gram, terms, options.rank_tolerance, interrupt, PHASE)?;
    let atoms = (rows as u64) * (options.probes as u64);
    Ok(PreparedResidualMoments {
        basis,
        leverage: exact_leverage,
        terms,
        gram,
        inverse: inverse.inverse,
        positivity_multiplier: options.positivity_multiplier,
        diagnostic: ResidualMomentPreparationDiagnostic {
            probes: options.probes,
            counter_atoms: atoms,
            counter_words: 2 * atoms,
            maximum_full_residual,
            full_residual_gate,
            gram_rcond: inverse.rcond,
            gram_inverse_relres: inverse.original_relres.max(inverse.relres),
            maximum_leverage,
            residual_degrees_of_freedom: df.value(),
            memory,
        },
    })
}

impl PreparedResidualMoments<'_> {
    #[must_use]
    pub fn gram(&self) -> &[f64] {
        &self.gram
    }

    /// Residuals must be from the identical, certified full-model fit. The
    /// returned positive vector is for covariance only, never point correction.
    pub fn fit_with_interrupt(
        &self,
        residual: &[f64],
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<ResidualMomentFit> {
        if residual.len() != self.leverage.len() {
            return Err(invalid("residual dimensions disagree"));
        }
        let mut rhs = vec![Sum::default(); self.terms];
        let mut scales = zeroed(residual.len(), interrupt)?;
        for row in 0..residual.len() {
            checkpoint_chunk(interrupt, row, PHASE)?;
            let square = residual[row].powi(2);
            scales[row] = square / (1.0 - self.leverage[row]);
            if !scales[row].is_finite() {
                return Err(numerical("nonfinite residual or HC2 scale"));
            }
            for term in 0..self.terms {
                rhs[term].add(self.basis[row * self.terms + term] * square);
            }
        }
        unstable_sort_by_with_interrupt(&mut scales, f64::total_cmp, interrupt, PHASE)?;
        let middle = scales.len() / 2;
        let scale = if scales.len() % 2 == 0 {
            0.5 * scales[middle - 1] + 0.5 * scales[middle]
        } else {
            scales[middle]
        };
        let floor = self.positivity_multiplier * scale;
        if !floor.is_finite() || floor <= 0.0 {
            return Err(numerical("nonpositive or nonfinite residual scale/floor"));
        }
        drop(scales);
        let rhs = rhs.into_iter().map(Sum::value).collect::<Vec<_>>();
        let mut coefficients = vec![0.0; self.terms];
        for left in 0..self.terms {
            coefficients[left] = dot(
                &self.inverse[left * self.terms..(left + 1) * self.terms],
                &rhs,
            );
        }
        let mut residual_norm: f64 = 0.0;
        let mut rhs_norm: f64 = 0.0;
        for row in 0..self.terms {
            let error = dot(
                &self.gram[row * self.terms..(row + 1) * self.terms],
                &coefficients,
            ) - rhs[row];
            residual_norm = residual_norm.hypot(error);
            rhs_norm = rhs_norm.hypot(rhs[row]);
        }
        let moment_relative_residual = if rhs_norm == 0.0 {
            residual_norm
        } else {
            residual_norm / rhs_norm
        };
        if !moment_relative_residual.is_finite() || moment_relative_residual > SMALL_RESIDUAL_GATE {
            return Err(BackendError::new(
                ErrorCode::InverseResidualFailed,
                PHASE,
                "residual-moment fit failed its original small-system residual gate",
            ));
        }
        let mut raw_variance = zeroed(residual.len(), interrupt)?;
        let mut positive_variance = zeroed(residual.len(), interrupt)?;
        let mut nonpositive_predictions = 0;
        let mut floored_predictions = 0;
        for row in 0..residual.len() {
            checkpoint_chunk(interrupt, row, PHASE)?;
            let value = dot(
                &self.basis[row * self.terms..(row + 1) * self.terms],
                &coefficients,
            );
            if !value.is_finite() {
                return Err(numerical("nonfinite variance prediction"));
            }
            raw_variance[row] = value;
            positive_variance[row] = value.max(floor);
            nonpositive_predictions += usize::from(value <= 0.0);
            floored_predictions += usize::from(value < floor);
        }
        if floored_predictions == residual.len() {
            return Err(numerical("entire variance fit was floored"));
        }
        Ok(ResidualMomentFit {
            coefficients,
            raw_variance,
            positive_variance,
            nonpositive_predictions,
            floored_predictions,
            positivity_floor: floor,
            moment_relative_residual,
        })
    }
}

fn validate_inputs(
    basis: &[f64],
    terms: usize,
    h: &[f64],
    addresses: &[(u64, u64)],
    interrupt: &mut dyn InterruptCheck,
) -> Result<()> {
    if basis.len() != checked_mul(h.len(), terms)? || addresses.len() != h.len() {
        return Err(invalid("basis, leverage and address dimensions disagree"));
    }
    for row in 0..h.len() {
        checkpoint_chunk(interrupt, row, PHASE)?;
        if !h[row].is_finite() || !(0.0..1.0).contains(&h[row]) {
            return Err(invalid("observation leverage must be finite in [0,1)"));
        }
        if basis[row * terms] != 1.0
            || basis[row * terms..(row + 1) * terms]
                .iter()
                .any(|x| !x.is_finite())
        {
            return Err(invalid(
                "variance basis must be finite with a constant first column",
            ));
        }
        if addresses[row].1 >= MAX_PHYSICAL_WORDS_PER_ATOM / 2
            || (row > 0 && addresses[row - 1] >= addresses[row])
        {
            return Err(BackendError::new(
                ErrorCode::RngContractFailed,
                PHASE,
                "Gaussian addresses must be canonical, distinct and within the word cap",
            ));
        }
    }
    Ok(())
}

fn normal(rng: CounterRng, probe: u64, entity: u64, subdraw: u64) -> f64 {
    let first = rng.word(
        ProbeDomain::ObservationResidualMoments,
        probe,
        entity,
        2 * subdraw,
    );
    let second = rng.word(
        ProbeDomain::ObservationResidualMoments,
        probe,
        entity,
        2 * subdraw + 1,
    );
    let unit = 1.0 / 9_007_199_254_740_992.0;
    let u1 = ((first >> 11) as f64 + 0.5) * unit;
    let u2 = ((second >> 11) as f64 + 0.5) * unit;
    (-2.0 * u1.ln()).sqrt() * (core::f64::consts::TAU * u2).cos()
}

#[derive(Clone, Copy, Debug, Default)]
struct Sum {
    sum: f64,
    correction: f64,
}
impl Sum {
    fn add(&mut self, value: f64) {
        let next = self.sum + value;
        self.correction += if self.sum.abs() >= value.abs() {
            (self.sum - next) + value
        } else {
            (value - next) + self.sum
        };
        self.sum = next;
    }
    fn value(self) -> f64 {
        self.sum + self.correction
    }
}

#[derive(Debug)]
struct CenteredMoments {
    count: usize,
    mean: Vec<f64>,
    cross: Vec<Sum>,
}
impl CenteredMoments {
    fn new(terms: usize) -> Self {
        Self {
            count: 0,
            mean: vec![0.0; terms],
            cross: vec![Sum::default(); terms * terms],
        }
    }
    fn push(&mut self, value: &[Sum]) -> Result<()> {
        self.count += 1;
        let mut delta = [0.0; MAX_TERMS];
        for term in 0..self.mean.len() {
            delta[term] = value[term].value() - self.mean[term];
            self.mean[term] += delta[term] / self.count as f64;
            if !delta[term].is_finite() || !self.mean[term].is_finite() {
                return Err(numerical("nonfinite projected moment"));
            }
        }
        let weight = (self.count - 1) as f64 / self.count as f64;
        for left in 0..self.mean.len() {
            for right in 0..self.mean.len() {
                self.cross[left * self.mean.len() + right].add(weight * delta[left] * delta[right]);
            }
        }
        Ok(())
    }
}

fn dot(left: &[f64], right: &[f64]) -> f64 {
    let mut sum = Sum::default();
    for (&a, &b) in left.iter().zip(right) {
        sum.add(a * b);
    }
    sum.value()
}
fn zeroed(length: usize, interrupt: &mut dyn InterruptCheck) -> Result<Vec<f64>> {
    let mut result = Vec::new();
    result.try_reserve_exact(length).map_err(|_| {
        BackendError::new(
            ErrorCode::AllocationFailed,
            PHASE,
            "residual-moment allocation failed",
        )
    })?;
    for index in 0..length {
        checkpoint_chunk(interrupt, index, PHASE)?;
        result.push(0.0);
    }
    Ok(result)
}
fn checked_mul(a: usize, b: usize) -> Result<usize> {
    a.checked_mul(b)
        .ok_or_else(|| resource("residual-moment memory/count overflow"))
}
fn checked_add(a: usize, b: usize) -> Result<usize> {
    a.checked_add(b)
        .ok_or_else(|| resource("residual-moment memory/count overflow"))
}
fn invalid(message: &str) -> BackendError {
    BackendError::invalid(PHASE, message)
}
fn resource(message: &str) -> BackendError {
    BackendError::new(ErrorCode::ResourceLimit, PHASE, message)
}
fn numerical(message: &str) -> BackendError {
    BackendError::new(ErrorCode::CorrectionNonFinite, PHASE, message)
}

#[cfg(test)]
mod tests;
