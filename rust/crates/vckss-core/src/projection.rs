// SPDX-License-Identifier: GPL-3.0-only

//! Sparse fixed-effect projection preparation and streamed covariance work.
//!
//! Projection covariates are reduced to coefficient-space loadings before
//! estimator RNG.  The live generic-JLA runtime later applies the already
//! prepared model solver to those loadings and streams the observation scores;
//! it never constructs an observation-by-parameter design, a full inverse, or
//! an observation-by-projection score matrix.

use crate::dense::{invert_scaled_spd, symmetric_eigen_extremes};
use crate::error::{BackendError, ErrorCode, Result};
use crate::interrupt::{checkpoint_chunk, InterruptCheck, NeverInterrupt};
use crate::model_solver::{ModelCoefficients, ModelSolve};
use crate::problem::CompressedProblem;

pub const PROJECTION_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProjectionEffect {
    Worker,
    Firm,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProjectionWeight {
    Frequency,
    Target,
}

#[derive(Clone, Debug)]
pub struct PreparedProjection {
    pub schema_version: u32,
    pub effect: ProjectionEffect,
    pub weight: ProjectionWeight,
    pub columns: usize,
    /// Compatible full-system worker RHSs, one column at a time.
    pub worker_rhs: Vec<f64>,
    /// Compatible full-firm RHSs, one column at a time.
    pub firm_rhs: Vec<f64>,
    pub gram_rcond: f64,
    pub gram_relres: f64,
    pub gram_original_relres: f64,
    pub persistent_bytes: u64,
}

#[derive(Clone, Debug)]
pub struct ProjectionResult {
    pub schema_version: u32,
    pub effect: ProjectionEffect,
    pub weight: ProjectionWeight,
    pub columns: usize,
    pub coefficients: Vec<f64>,
    pub covariance: Vec<f64>,
    pub naive_covariance: Vec<f64>,
    pub gram_rcond: f64,
    pub gram_relres: f64,
    pub gram_original_relres: f64,
    pub covariance_smallest_eigenvalue: f64,
    pub covariance_largest_eigenvalue: f64,
    pub psd_cleanup: f64,
    pub proxy_minimum: f64,
    pub proxy_maximum: f64,
    pub maximum_iterations: u32,
    pub maximum_reduced_residual: f64,
    pub maximum_complete_residual: f64,
    pub full_residual_tolerance: f64,
    pub projection_peak_forecast_bytes: u64,
    pub persistent_bytes: u64,
    pub result_bytes: u64,
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

pub fn prepare_projection(
    problem: &CompressedProblem,
    project: &[Vec<f64>],
    effect: ProjectionEffect,
    weight: ProjectionWeight,
    rank_tolerance: f64,
) -> Result<PreparedProjection> {
    prepare_projection_with_interrupt(
        problem,
        project,
        effect,
        weight,
        rank_tolerance,
        &mut NeverInterrupt,
    )
}

pub fn prepare_projection_with_interrupt(
    problem: &CompressedProblem,
    project: &[Vec<f64>],
    effect: ProjectionEffect,
    weight: ProjectionWeight,
    rank_tolerance: f64,
    interrupt: &mut dyn InterruptCheck,
) -> Result<PreparedProjection> {
    interrupt.checkpoint("projection_prepare_entry")?;
    let rows = problem.outcome.len();
    if project.is_empty() {
        return Err(BackendError::invalid(
            "projection_prepare",
            "project() must supply at least one covariate",
        ));
    }
    if project
        .iter()
        .any(|column| column.len() != rows || column.iter().any(|value| !value.is_finite()))
    {
        return Err(BackendError::invalid(
            "projection_prepare",
            "projection columns have inconsistent dimensions or nonfinite values",
        ));
    }
    if problem.frequency.iter().any(|&frequency| frequency != 1) {
        return Err(BackendError::new(
            ErrorCode::UnsupportedFeature,
            "projection_prepare",
            "the first sparse projection route requires unit frequency weights",
        ));
    }
    let columns = project
        .len()
        .checked_add(1)
        .ok_or_else(|| resource("projection column count overflow"))?;
    let gram_entries = checked_product(columns, columns, "projection Gram")?;
    let effect_levels = match effect {
        ProjectionEffect::Worker => problem.workers(),
        ProjectionEffect::Firm => problem
            .firms()
            .checked_sub(1)
            .ok_or_else(|| resource("grounded firm dimension underflow"))?,
    };
    let loading_entries = checked_product(effect_levels, columns, "projection loading")?;
    let mut gram = vec![StableAccumulator::default(); gram_entries];
    let mut cross = vec![StableAccumulator::default(); loading_entries];
    let mut values = vec![0.0; columns];
    for row in 0..rows {
        checkpoint_chunk(interrupt, row, "projection_prepare_accumulate")?;
        values[0] = 1.0;
        for (column, source) in project.iter().enumerate() {
            values[column + 1] = source[row];
        }
        let mass = match weight {
            ProjectionWeight::Frequency => 1.0,
            ProjectionWeight::Target => problem.target_weight[row],
        };
        if !mass.is_finite() || mass < 0.0 {
            return Err(BackendError::new(
                ErrorCode::InvalidTargetWeight,
                "projection_prepare",
                "projection mass is negative or nonfinite",
            ));
        }
        for left in 0..columns {
            for right in 0..columns {
                gram[left * columns + right].add(mass * values[left] * values[right]);
            }
        }
        let level = match effect {
            ProjectionEffect::Worker => usize::try_from(problem.row_worker[row])
                .map_err(|_| resource("worker index is not addressable"))?,
            ProjectionEffect::Firm => {
                let firm = usize::try_from(problem.row_firm[row])
                    .map_err(|_| resource("firm index is not addressable"))?;
                if firm == problem.firms() - 1 {
                    continue;
                }
                firm
            }
        };
        for column in 0..columns {
            cross[level * columns + column].add(mass * values[column]);
        }
    }
    let gram = gram
        .into_iter()
        .map(StableAccumulator::finish)
        .collect::<Vec<_>>();
    let cross = cross
        .into_iter()
        .map(StableAccumulator::finish)
        .collect::<Vec<_>>();
    let inverse = invert_scaled_spd(&gram, columns, rank_tolerance, interrupt, "projection_gram")?;
    let workers = problem.workers();
    let firms = problem.firms();
    let worker_entries = checked_product(workers, columns, "projection worker RHS")?;
    let firm_entries = checked_product(firms, columns, "projection firm RHS")?;
    let mut worker_rhs = vec![0.0; worker_entries];
    let mut firm_rhs = vec![0.0; firm_entries];
    for column in 0..columns {
        let mut grounded_sum = StableAccumulator::default();
        for level in 0..effect_levels {
            let mut value = StableAccumulator::default();
            for inner in 0..columns {
                value.add(
                    cross[level * columns + inner] * inverse.inverse[inner * columns + column],
                );
            }
            let value = value.finish();
            grounded_sum.add(value);
            match effect {
                ProjectionEffect::Worker => worker_rhs[column * workers + level] = value,
                ProjectionEffect::Firm => firm_rhs[column * firms + level] = value,
            }
        }
        // Convert the last-firm-grounded functional to a compatible RHS for
        // the full-firm zero-sum quotient: sum(worker RHS) == sum(firm RHS).
        firm_rhs[column * firms + firms - 1] = match effect {
            ProjectionEffect::Worker => grounded_sum.finish(),
            ProjectionEffect::Firm => -grounded_sum.finish(),
        };
    }
    let persistent_bytes = byte_count(
        worker_rhs
            .len()
            .checked_add(firm_rhs.len())
            .ok_or_else(|| resource("projection persistent count overflow"))?,
        "projection persistent bytes",
    )?;
    interrupt.checkpoint("projection_prepare_final")?;
    Ok(PreparedProjection {
        schema_version: PROJECTION_SCHEMA_VERSION,
        effect,
        weight,
        columns,
        worker_rhs,
        firm_rhs,
        gram_rcond: inverse.rcond,
        gram_relres: inverse.relres,
        gram_original_relres: inverse.original_relres,
        persistent_bytes,
    })
}

pub fn projection_coefficients(
    prepared: &PreparedProjection,
    fit: &ModelCoefficients,
) -> Result<Vec<f64>> {
    let workers = fit.worker.len();
    let firms = fit.firm.len();
    if prepared.worker_rhs.len() != workers.saturating_mul(prepared.columns)
        || prepared.firm_rhs.len() != firms.saturating_mul(prepared.columns)
    {
        return Err(BackendError::invariant(
            "projection_coefficients",
            "projection RHS and fitted coefficients disagree",
        ));
    }
    let mut output = Vec::with_capacity(prepared.columns);
    for column in 0..prepared.columns {
        let mut value = StableAccumulator::default();
        for worker in 0..workers {
            value.add(prepared.worker_rhs[column * workers + worker] * fit.worker[worker]);
        }
        for firm in 0..firms {
            value.add(prepared.firm_rhs[column * firms + firm] * fit.firm[firm]);
        }
        let value = value.finish();
        if !value.is_finite() {
            return Err(nonfinite("projection coefficient is nonfinite"));
        }
        output.push(value);
    }
    Ok(output)
}

#[allow(clippy::too_many_arguments)]
pub fn accumulate_projection_covariance(
    problem: &CompressedProblem,
    prepared: &PreparedProjection,
    coefficients: Vec<f64>,
    solutions: &[ModelSolve],
    controls: &[Vec<f64>],
    working_y: &[f64],
    deleted_adjusted: &[f64],
    residual: &[f64],
    row_order: &[usize],
    interrupt: &mut dyn InterruptCheck,
) -> Result<ProjectionResult> {
    let rows = problem.outcome.len();
    let columns = prepared.columns;
    if solutions.len() != columns
        || coefficients.len() != columns
        || working_y.len() != rows
        || deleted_adjusted.len() != rows
        || residual.len() != rows
        || row_order.len() != rows
        || controls.iter().any(|column| column.len() != rows)
    {
        return Err(BackendError::invariant(
            "projection_covariance",
            "projection covariance inputs have inconsistent dimensions",
        ));
    }
    let mut mean = StableAccumulator::default();
    for (position, &row) in row_order.iter().enumerate() {
        checkpoint_chunk(interrupt, position, "projection_mean")?;
        mean.add(working_y[row]);
    }
    let mean = mean.finish() / rows as f64;
    if !mean.is_finite() {
        return Err(nonfinite("projection working-outcome mean is nonfinite"));
    }
    let entries = checked_product(columns, columns, "projection covariance")?;
    let mut covariance = vec![StableAccumulator::default(); entries];
    let mut naive = vec![StableAccumulator::default(); entries];
    let mut score = vec![0.0; columns];
    let mut proxy_minimum = f64::INFINITY;
    let mut proxy_maximum = f64::NEG_INFINITY;
    for (position, &row) in row_order.iter().enumerate() {
        checkpoint_chunk(interrupt, position, "projection_covariance_stream")?;
        let worker = usize::try_from(problem.row_worker[row])
            .map_err(|_| resource("projection worker index is not addressable"))?;
        let firm = usize::try_from(problem.row_firm[row])
            .map_err(|_| resource("projection firm index is not addressable"))?;
        let proxy = (working_y[row] - mean) * deleted_adjusted[row];
        let naive_mass = residual[row] * residual[row];
        if !proxy.is_finite() || !naive_mass.is_finite() {
            return Err(nonfinite("projection variance proxy is nonfinite"));
        }
        proxy_minimum = proxy_minimum.min(proxy);
        proxy_maximum = proxy_maximum.max(proxy);
        for (column, solution) in solutions.iter().enumerate() {
            let mut value = StableAccumulator::default();
            value.add(solution.coefficients.worker[worker]);
            value.add(solution.coefficients.firm[firm]);
            for (control, source) in controls.iter().enumerate() {
                value.add(solution.coefficients.control[control] * source[row]);
            }
            score[column] = value.finish();
            if !score[column].is_finite() {
                return Err(nonfinite("projection score is nonfinite"));
            }
        }
        for left in 0..columns {
            for right in left..columns {
                let product = score[left] * score[right];
                covariance[left * columns + right].add(proxy * product);
                naive[left * columns + right].add(naive_mass * product);
            }
        }
    }
    let mut covariance = finish_symmetric(covariance, columns)?;
    let mut naive_covariance = finish_symmetric(naive, columns)?;
    let spectrum =
        symmetric_eigen_extremes(&covariance, columns, interrupt, "projection_covariance_psd")?;
    let scale = (0..columns)
        .map(|index| covariance[index * columns + index].abs())
        .fold(1.0e-30_f64, f64::max);
    if (0..columns).any(|index| covariance[index * columns + index] <= 0.0)
        || spectrum.smallest_lower < -1.0e-8 * scale
    {
        return Err(BackendError::new(
            ErrorCode::JlaConstraintFailed,
            "projection_covariance_psd",
            "the KSS projection covariance is not positive semidefinite",
        ));
    }
    // The certified lower enclosure can be slightly negative even when the
    // matrix is positive semidefinite. A diagonal shift bounded by the same
    // public 1e-8*scale cleanup contract keeps the returned matrix usable
    // without constructing eigenvectors.
    let psd_cleanup = (-spectrum.smallest_lower).max(0.0);
    if psd_cleanup > 0.0 {
        for index in 0..columns {
            covariance[index * columns + index] += psd_cleanup;
        }
    }
    for index in 0..columns {
        let diagonal = naive_covariance[index * columns + index];
        if !diagonal.is_finite() || diagonal < -1.0e-12 * scale {
            return Err(nonfinite(
                "naive projection covariance has an invalid diagonal",
            ));
        }
        if diagonal < 0.0 {
            naive_covariance[index * columns + index] = 0.0;
        }
    }
    let mut maximum_iterations = 0_u32;
    let mut maximum_reduced_residual = 0.0_f64;
    let mut maximum_complete_residual = 0.0_f64;
    let mut full_residual_tolerance = 0.0_f64;
    for solution in solutions {
        maximum_iterations = maximum_iterations.max(solution.receipt.pcg.iterations);
        maximum_reduced_residual =
            maximum_reduced_residual.max(solution.receipt.pcg.relative_residual);
        maximum_complete_residual = maximum_complete_residual.max(solution.receipt.full_residual);
        full_residual_tolerance = solution.receipt.full_residual_tolerance;
    }
    let result_bytes = byte_count(
        coefficients
            .len()
            .checked_add(covariance.len())
            .and_then(|value| value.checked_add(naive_covariance.len()))
            .ok_or_else(|| resource("projection result count overflow"))?,
        "projection result bytes",
    )?;
    Ok(ProjectionResult {
        schema_version: PROJECTION_SCHEMA_VERSION,
        effect: prepared.effect,
        weight: prepared.weight,
        columns,
        coefficients,
        covariance,
        naive_covariance,
        gram_rcond: prepared.gram_rcond,
        gram_relres: prepared.gram_relres,
        gram_original_relres: prepared.gram_original_relres,
        covariance_smallest_eigenvalue: spectrum.smallest_lower,
        covariance_largest_eigenvalue: spectrum.largest_upper,
        psd_cleanup,
        proxy_minimum,
        proxy_maximum,
        maximum_iterations,
        maximum_reduced_residual,
        maximum_complete_residual,
        full_residual_tolerance,
        projection_peak_forecast_bytes: 0,
        persistent_bytes: prepared.persistent_bytes,
        result_bytes,
    })
}

fn finish_symmetric(values: Vec<StableAccumulator>, dimension: usize) -> Result<Vec<f64>> {
    let mut output = vec![0.0; values.len()];
    for left in 0..dimension {
        for right in left..dimension {
            let value = values[left * dimension + right].finish();
            if !value.is_finite() {
                return Err(nonfinite("projection covariance accumulation is nonfinite"));
            }
            output[left * dimension + right] = value;
            output[right * dimension + left] = value;
        }
    }
    Ok(output)
}

fn checked_product(left: usize, right: usize, label: &str) -> Result<usize> {
    left.checked_mul(right)
        .ok_or_else(|| resource(format!("{label} dimension overflow")))
}

fn byte_count(values: usize, label: &str) -> Result<u64> {
    u64::try_from(values)
        .ok()
        .and_then(|value| value.checked_mul(core::mem::size_of::<f64>() as u64))
        .ok_or_else(|| resource(format!("{label} overflow")))
}

fn resource(message: impl Into<String>) -> BackendError {
    BackendError::new(ErrorCode::ResourceLimit, "projection", message)
}

fn nonfinite(message: impl Into<String>) -> BackendError {
    BackendError::new(ErrorCode::CorrectionNonFinite, "projection", message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::problem::CanonicalInput;
    use crate::types::InputColumns;

    fn fixture() -> CompressedProblem {
        let input = InputColumns {
            worker: vec![10, 10, 20, 20],
            firm: vec![100, 200, 100, 200],
            deletion: vec![1, 2, 3, 4],
            outcome: vec![1.0, 2.0, 3.0, 4.0],
            frequency: vec![1, 1, 1, 1],
            target_weight: vec![1.0, 1.0, 1.0, 1.0],
            controls: Vec::new(),
        }
        .validate()
        .expect("valid projection fixture");
        CanonicalInput::from_validated(input)
            .expect("canonical projection fixture")
            .compress(&[true; 4])
            .expect("compressed projection fixture")
    }

    #[test]
    fn worker_loading_reproduces_grounded_ols_functional() {
        let problem = fixture();
        let prepared = prepare_projection(
            &problem,
            &[vec![0.0, 1.0, 2.0, 3.0]],
            ProjectionEffect::Worker,
            ProjectionWeight::Frequency,
            1.0e-10,
        )
        .expect("identified worker projection");
        let coefficients = projection_coefficients(
            &prepared,
            &ModelCoefficients {
                worker: vec![10.0, 20.0],
                firm: vec![-1.0, 1.0],
                control: Vec::new(),
            },
        )
        .expect("worker projection coefficients");
        // Quotient worker coefficients become 11 and 21 when the displayed
        // last firm (quotient value 1) is grounded at zero.
        assert!((coefficients[0] - 10.0).abs() < 1.0e-12);
        assert!((coefficients[1] - 4.0).abs() < 1.0e-12);
        for column in 0..prepared.columns {
            let worker_sum: f64 = prepared.worker_rhs
                [column * problem.workers()..(column + 1) * problem.workers()]
                .iter()
                .sum();
            let firm_sum: f64 = prepared.firm_rhs
                [column * problem.firms()..(column + 1) * problem.firms()]
                .iter()
                .sum();
            assert!((worker_sum - firm_sum).abs() < 1.0e-12);
        }
    }

    #[test]
    fn firm_loading_preserves_last_firm_grounding() {
        let problem = fixture();
        let prepared = prepare_projection(
            &problem,
            &[vec![0.0, 1.0, 2.0, 3.0]],
            ProjectionEffect::Firm,
            ProjectionWeight::Frequency,
            1.0e-10,
        )
        .expect("identified firm projection");
        let coefficients = projection_coefficients(
            &prepared,
            &ModelCoefficients {
                worker: vec![0.0, 0.0],
                firm: vec![-1.0, 1.0],
                control: Vec::new(),
            },
        )
        .expect("firm projection coefficients");
        assert!((coefficients[0] + 1.6).abs() < 1.0e-12);
        assert!((coefficients[1] - 0.4).abs() < 1.0e-12);
    }

    #[test]
    fn singular_projection_gram_fails_closed() {
        let problem = fixture();
        let error = prepare_projection(
            &problem,
            &[vec![1.0; 4]],
            ProjectionEffect::Worker,
            ProjectionWeight::Frequency,
            1.0e-10,
        )
        .expect_err("constant project column duplicates the automatic constant");
        assert_eq!(error.code, ErrorCode::SingularInformation);
    }

    #[test]
    fn nonunit_frequency_is_explicitly_deferred() {
        let mut problem = fixture();
        problem.frequency[0] = 2;
        let error = prepare_projection(
            &problem,
            &[vec![0.0, 1.0, 2.0, 3.0]],
            ProjectionEffect::Worker,
            ProjectionWeight::Frequency,
            1.0e-10,
        )
        .expect_err("weighted route is outside the first qualification surface");
        assert_eq!(error.code, ErrorCode::UnsupportedFeature);
    }
}
