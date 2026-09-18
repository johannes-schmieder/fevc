// SPDX-License-Identifier: GPL-3.0-only

//! Small, deterministic dense symmetric kernels used by certified oracles.

use crate::error::{BackendError, ErrorCode, Result};
use crate::interrupt::{checkpoint_chunk, InterruptCheck};
use crate::model_operator::{
    checked_matrix_length, copy_f64_with_interrupt, zeroed_f64_with_interrupt,
};

#[derive(Clone, Debug)]
pub(crate) struct DenseInverse {
    pub(crate) inverse: Vec<f64>,
    /// Certified lower bound on the diagonally scaled spectral rcond.
    pub(crate) rcond: f64,
    /// Maximum column residual in diagonally scaled coordinates.
    pub(crate) relres: f64,
    /// Maximum column residual in the original coordinates.
    pub(crate) original_relres: f64,
}

#[derive(Clone, Debug)]
struct Spectrum {
    eigenvalues: Vec<f64>,
    /// Normwise eigenvalue enclosure radius: terminal Jacobi off-diagonal
    /// norm plus an explicit accumulated floating-point backward-error term.
    error_bound: f64,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct EigenExtremes {
    pub(crate) smallest_lower: f64,
    pub(crate) largest_upper: f64,
}

pub(crate) fn symmetric_eigen_extremes(
    matrix: &[f64],
    dimension: usize,
    interrupt: &mut dyn InterruptCheck,
    phase: &'static str,
) -> Result<EigenExtremes> {
    if dimension == 0 || matrix.len() != dimension.saturating_mul(dimension) {
        return Err(BackendError::invalid(
            phase,
            "symmetric eigenvalue dimensions disagree",
        ));
    }
    let mut symmetric = copy_f64_with_interrupt(matrix, phase, interrupt, phase)?;
    symmetrize_with_interrupt(&mut symmetric, dimension, interrupt, phase)?;
    let spectrum = symmetric_spectrum(&symmetric, dimension, interrupt, phase)?;
    Ok(EigenExtremes {
        smallest_lower: spectrum.eigenvalues[0] - spectrum.error_bound,
        largest_upper: spectrum.eigenvalues[dimension - 1] + spectrum.error_bound,
    })
}

pub(crate) fn invert_scaled_spd(
    matrix: &[f64],
    dimension: usize,
    tolerance: f64,
    interrupt: &mut dyn InterruptCheck,
    phase: &'static str,
) -> Result<DenseInverse> {
    let (scaled, scale) = diagonally_scale(matrix, dimension, interrupt, phase)?;
    let spectrum = symmetric_spectrum(&scaled, dimension, interrupt, phase)?;
    let largest = spectrum.eigenvalues[dimension - 1];
    let smallest = spectrum.eigenvalues[0];
    if !largest.is_finite()
        || largest <= 0.0
        || smallest - spectrum.error_bound <= tolerance * (largest + spectrum.error_bound)
    {
        return Err(BackendError::new(
            ErrorCode::SingularInformation,
            phase,
            "diagonally scaled symmetric information is singular or numerically unidentified",
        ));
    }
    let rcond = (smallest - spectrum.error_bound) / (largest + spectrum.error_bound);
    invert_from_scaled(
        matrix, scaled, &scale, dimension, rcond, tolerance, interrupt, phase,
    )
}

/// Invert a symmetric information matrix whose only exact null direction is
/// the constant vector on `null_range`.  Rank and the reported reciprocal
/// condition number are certified from the diagonally scaled *unshifted*
/// matrix.  A unit projector is added only for the numerical inverse; its
/// inverse action agrees with the quotient inverse on every compatible RHS.
pub(crate) fn invert_scaled_zero_sum_quotient(
    matrix: &[f64],
    dimension: usize,
    null_range: core::ops::Range<usize>,
    tolerance: f64,
    interrupt: &mut dyn InterruptCheck,
    phase: &'static str,
) -> Result<DenseInverse> {
    if null_range.is_empty() || null_range.end > dimension {
        return Err(BackendError::invalid(
            phase,
            "zero-sum quotient range is invalid",
        ));
    }
    let (scaled, quotient_scale) = diagonally_scale(matrix, dimension, interrupt, phase)?;
    let spectrum = symmetric_spectrum(&scaled, dimension, interrupt, phase)?;
    let largest = spectrum.eigenvalues[dimension - 1];
    let null_eigenvalue = spectrum.eigenvalues[0];
    let smallest = spectrum.eigenvalues[1];
    let null_margin = spectrum.error_bound;
    if !largest.is_finite()
        || largest <= 0.0
        || null_eigenvalue.abs() > null_margin
        || smallest - spectrum.error_bound <= tolerance * (largest + spectrum.error_bound)
    {
        return Err(BackendError::new(
            ErrorCode::SingularInformation,
            phase,
            "full-firm zero-sum quotient is singular or numerically unidentified",
        ));
    }
    let rcond = (smallest - spectrum.error_bound) / (largest + spectrum.error_bound);
    drop(spectrum);
    drop(scaled);

    let mut augmented = copy_f64_with_interrupt(matrix, phase, interrupt, phase)?;
    let width = null_range.len() as f64;
    let projector = width.recip();
    for row in null_range.clone() {
        for column in null_range.clone() {
            augmented[row * dimension + column] += projector;
        }
    }
    let (augmented_scaled, scale) = diagonally_scale(&augmented, dimension, interrupt, phase)?;
    let mut inverse = invert_from_scaled(
        &augmented,
        augmented_scaled,
        &scale,
        dimension,
        rcond,
        tolerance,
        interrupt,
        phase,
    )?;
    inverse.relres = quotient_scaled_inverse_residual(
        matrix,
        &inverse.inverse,
        &quotient_scale,
        dimension,
        null_range,
        interrupt,
        phase,
    )?;
    if inverse.relres > (100.0 * tolerance).max(1.0e-10) {
        return Err(BackendError::new(
            ErrorCode::InverseResidualFailed,
            phase,
            "zero-sum quotient inverse failed its compatible scaled residual gate",
        ));
    }
    Ok(inverse)
}

fn diagonally_scale(
    matrix: &[f64],
    dimension: usize,
    interrupt: &mut dyn InterruptCheck,
    phase: &'static str,
) -> Result<(Vec<f64>, Vec<f64>)> {
    let entries = dimension.checked_mul(dimension).ok_or_else(|| {
        BackendError::new(
            ErrorCode::ResourceLimit,
            "dense_symmetric",
            "dense symmetric dimension overflow",
        )
    })?;
    if dimension == 0 || matrix.len() != entries || matrix.iter().any(|value| !value.is_finite()) {
        return Err(BackendError::invalid(
            "dense_symmetric",
            "dense symmetric matrix has invalid dimensions or values",
        ));
    }
    let mut scale = zeroed_f64_with_interrupt(dimension, phase, interrupt, phase)?;
    for index in 0..dimension {
        checkpoint_chunk(interrupt, index, phase)?;
        let diagonal = matrix[index * dimension + index];
        if !diagonal.is_finite() || diagonal <= 0.0 {
            return Err(BackendError::new(
                ErrorCode::SingularInformation,
                "dense_symmetric",
                "symmetric information has a nonpositive diagonal",
            ));
        }
        scale[index] = diagonal.sqrt().recip();
    }
    let mut scaled = zeroed_f64_with_interrupt(entries, phase, interrupt, phase)?;
    for row in 0..dimension {
        for column in 0..dimension {
            checkpoint_chunk(interrupt, row * dimension + column, phase)?;
            scaled[row * dimension + column] = 0.5
                * (matrix[row * dimension + column] + matrix[column * dimension + row])
                * scale[row]
                * scale[column];
        }
    }
    Ok((scaled, scale))
}

fn symmetric_spectrum(
    matrix: &[f64],
    dimension: usize,
    interrupt: &mut dyn InterruptCheck,
    phase: &'static str,
) -> Result<Spectrum> {
    let mut work = copy_f64_with_interrupt(matrix, phase, interrupt, phase)?;
    let mut norm_square = 0.0;
    for (index, value) in work.iter().enumerate() {
        checkpoint_chunk(interrupt, index, phase)?;
        norm_square += value * value;
    }
    let matrix_norm = norm_square.sqrt();
    let convergence = 64.0 * f64::EPSILON * (dimension as f64).max(1.0) * matrix_norm.max(1.0);
    let mut terminal_off_diagonal = off_diagonal_norm(&work, dimension, interrupt, phase)?;
    let mut completed_sweeps = 0_usize;
    let mut flattened_work = 0_usize;
    for sweep in 0..128_usize {
        checkpoint_chunk(interrupt, sweep, phase)?;
        if terminal_off_diagonal <= convergence {
            break;
        }
        for left in 0..dimension {
            for right in (left + 1)..dimension {
                flattened_work = flattened_work.saturating_add(1);
                checkpoint_chunk(interrupt, flattened_work, phase)?;
                let cross = work[left * dimension + right];
                if cross == 0.0 {
                    continue;
                }
                let diagonal_left = work[left * dimension + left];
                let diagonal_right = work[right * dimension + right];
                let tau = (diagonal_right - diagonal_left) / (2.0 * cross);
                let tangent = if tau >= 0.0 {
                    1.0 / (tau + (1.0 + tau * tau).sqrt())
                } else {
                    -1.0 / (-tau + (1.0 + tau * tau).sqrt())
                };
                let cosine = (1.0 + tangent * tangent).sqrt().recip();
                let sine = tangent * cosine;
                for index in 0..dimension {
                    flattened_work = flattened_work.saturating_add(1);
                    checkpoint_chunk(interrupt, flattened_work, phase)?;
                    if index == left || index == right {
                        continue;
                    }
                    let index_left = work[index * dimension + left];
                    let index_right = work[index * dimension + right];
                    let rotated_left = cosine * index_left - sine * index_right;
                    let rotated_right = sine * index_left + cosine * index_right;
                    work[index * dimension + left] = rotated_left;
                    work[left * dimension + index] = rotated_left;
                    work[index * dimension + right] = rotated_right;
                    work[right * dimension + index] = rotated_right;
                }
                work[left * dimension + left] = diagonal_left - tangent * cross;
                work[right * dimension + right] = diagonal_right + tangent * cross;
                work[left * dimension + right] = 0.0;
                work[right * dimension + left] = 0.0;
            }
        }
        completed_sweeps = sweep + 1;
        terminal_off_diagonal = off_diagonal_norm(&work, dimension, interrupt, phase)?;
    }
    if !terminal_off_diagonal.is_finite() || terminal_off_diagonal > convergence {
        return Err(BackendError::new(
            ErrorCode::SymmetricEigensolverFailed,
            phase,
            format!(
                "deterministic Jacobi eigensolver residual {terminal_off_diagonal} exceeds {convergence}"
            ),
        ));
    }
    // A cyclic Jacobi sweep updates each matrix entry O(n) times.  The bound
    // below charges 64 rounded operations per entry-path and sweep, plus one
    // input scaling pass.  This is deliberately conservative but remains
    // substantially sharper than charging every rotation as a serial error.
    let roundoff_operations = 64.0 * (completed_sweeps + 1) as f64 * dimension.max(1) as f64;
    let roundoff_bound = rounding_gamma(roundoff_operations).ok_or_else(|| {
        BackendError::new(
            ErrorCode::SymmetricEigensolverFailed,
            phase,
            "Jacobi accumulated-roundoff bound is not finite",
        )
    })? * matrix_norm.max(1.0);
    let error_bound = terminal_off_diagonal + roundoff_bound;
    let mut eigenvalues = zeroed_f64_with_interrupt(dimension, phase, interrupt, phase)?;
    for (index, value) in eigenvalues.iter_mut().enumerate() {
        checkpoint_chunk(interrupt, index, phase)?;
        *value = work[index * dimension + index];
    }
    // No stable-order contract exists between equal scalar eigenvalues. The
    // unstable sort avoids a hidden temporary allocation in this small kernel.
    eigenvalues.sort_unstable_by(f64::total_cmp);
    if eigenvalues.iter().any(|value| !value.is_finite()) {
        return Err(BackendError::new(
            ErrorCode::SymmetricEigensolverFailed,
            phase,
            "deterministic symmetric eigensolver returned a nonfinite eigenvalue",
        ));
    }
    Ok(Spectrum {
        eigenvalues,
        error_bound,
    })
}

fn invert_from_scaled(
    original: &[f64],
    mut scaled: Vec<f64>,
    scale: &[f64],
    dimension: usize,
    rcond: f64,
    tolerance: f64,
    interrupt: &mut dyn InterruptCheck,
    phase: &'static str,
) -> Result<DenseInverse> {
    cholesky_in_place(&mut scaled, dimension, interrupt, phase)?;
    let mut inverse = zeroed_f64_with_interrupt(
        checked_matrix_length(dimension, dimension, phase)?,
        phase,
        interrupt,
        phase,
    )?;
    let mut basis = zeroed_f64_with_interrupt(dimension, phase, interrupt, phase)?;
    // Every inverse column uses the same triangular scratch after its previous
    // solve has completed. Keep one fallibly allocated buffer for the loop.
    let mut intermediate = zeroed_f64_with_interrupt(dimension, phase, interrupt, phase)?;
    for column in 0..dimension {
        checkpoint_chunk(interrupt, column, phase)?;
        basis.fill(0.0);
        basis[column] = 1.0;
        solve_cholesky_into(
            &scaled,
            dimension,
            &basis,
            &mut intermediate,
            &mut inverse[column..],
            dimension,
            interrupt,
            phase,
        )?;
    }
    drop(intermediate);
    drop(basis);
    let scaled_relres =
        scaled_inverse_residual(original, &inverse, scale, dimension, interrupt, phase)?;
    for row in 0..dimension {
        for column in 0..dimension {
            checkpoint_chunk(interrupt, row * dimension + column, phase)?;
            inverse[row * dimension + column] *= scale[row] * scale[column];
        }
    }
    symmetrize_with_interrupt(&mut inverse, dimension, interrupt, phase)?;
    let original_relres = inverse_residual(original, &inverse, dimension, interrupt, phase)?;
    let residual_gate = (100.0 * tolerance).max(1.0e-10);
    if !scaled_relres.is_finite()
        || !original_relres.is_finite()
        || scaled_relres > residual_gate
        || original_relres > residual_gate
    {
        return Err(BackendError::new(
            ErrorCode::InverseResidualFailed,
            phase,
            format!("INVERSE_RESIDUAL_FAILED: dense symmetric inverse residuals scaled={scaled_relres:.17e}, original={original_relres:.17e}, gate={residual_gate:.17e}"),
        ));
    }
    Ok(DenseInverse {
        inverse,
        rcond,
        relres: scaled_relres,
        original_relres,
    })
}

pub(crate) fn cholesky_factor(
    matrix: &[f64],
    dimension: usize,
    interrupt: &mut dyn InterruptCheck,
    phase: &'static str,
) -> Result<Vec<f64>> {
    if matrix.len() != dimension.saturating_mul(dimension) {
        return Err(BackendError::invalid(
            phase,
            "Cholesky input dimensions disagree",
        ));
    }
    let mut factor = copy_f64_with_interrupt(matrix, phase, interrupt, phase)?;
    cholesky_in_place(&mut factor, dimension, interrupt, phase)?;
    Ok(factor)
}

fn cholesky_in_place(
    matrix: &mut [f64],
    dimension: usize,
    interrupt: &mut dyn InterruptCheck,
    phase: &'static str,
) -> Result<()> {
    let mut work = 0_usize;
    for row in 0..dimension {
        for column in 0..=row {
            work = work.saturating_add(1);
            checkpoint_chunk(interrupt, work, phase)?;
            let mut value = matrix[row * dimension + column];
            for inner in 0..column {
                work = work.saturating_add(1);
                checkpoint_chunk(interrupt, work, phase)?;
                value -= matrix[row * dimension + inner] * matrix[column * dimension + inner];
            }
            if row == column {
                if !value.is_finite() || value <= 0.0 {
                    return Err(BackendError::new(
                        ErrorCode::SingularInformation,
                        phase,
                        "certified symmetric matrix could not be directly factorized",
                    ));
                }
                matrix[row * dimension + column] = value.sqrt();
            } else {
                matrix[row * dimension + column] = value / matrix[column * dimension + column];
            }
        }
        for column in (row + 1)..dimension {
            work = work.saturating_add(1);
            checkpoint_chunk(interrupt, work, phase)?;
            matrix[row * dimension + column] = 0.0;
        }
    }
    Ok(())
}

fn solve_cholesky_into(
    factor: &[f64],
    dimension: usize,
    rhs: &[f64],
    intermediate: &mut [f64],
    output: &mut [f64],
    stride: usize,
    interrupt: &mut dyn InterruptCheck,
    phase: &'static str,
) -> Result<()> {
    if dimension == 0
        || stride == 0
        || rhs.len() != dimension
        || intermediate.len() != dimension
        || factor.len() != dimension.saturating_mul(dimension)
        || output.len() < (dimension - 1).saturating_mul(stride) + 1
    {
        return Err(BackendError::invalid(
            "dense_symmetric",
            "dense factor solve dimensions disagree",
        ));
    }
    let mut work = 0_usize;
    for row in 0..dimension {
        let mut value = rhs[row];
        for column in 0..row {
            work = work.saturating_add(1);
            checkpoint_chunk(interrupt, work, phase)?;
            value -= factor[row * dimension + column] * intermediate[column];
        }
        intermediate[row] = value / factor[row * dimension + row];
    }
    for row in (0..dimension).rev() {
        let mut value = intermediate[row];
        for column in (row + 1)..dimension {
            work = work.saturating_add(1);
            checkpoint_chunk(interrupt, work, phase)?;
            value -= factor[column * dimension + row] * output[column * stride];
        }
        output[row * stride] = value / factor[row * dimension + row];
    }
    if output
        .iter()
        .step_by(stride)
        .any(|value| !value.is_finite())
    {
        return Err(BackendError::new(
            ErrorCode::InverseResidualFailed,
            "dense_symmetric",
            "dense factor solve returned a nonfinite value",
        ));
    }
    Ok(())
}

pub(crate) fn inverse_forward_error(relres: f64, rcond: f64, dimension: usize) -> Option<f64> {
    if !relres.is_finite() || !rcond.is_finite() || rcond <= 0.0 || dimension == 0 {
        return None;
    }
    let operator_residual = (dimension as f64).sqrt() * relres;
    let denominator = rcond - operator_residual;
    (denominator > 0.0).then_some(operator_residual / denominator)
}

pub(crate) fn frobenius_norm(matrix: &[f64]) -> f64 {
    matrix.iter().map(|value| value * value).sum::<f64>().sqrt()
}

fn off_diagonal_norm(
    matrix: &[f64],
    dimension: usize,
    interrupt: &mut dyn InterruptCheck,
    phase: &'static str,
) -> Result<f64> {
    let mut sum = 0.0;
    for row in 0..dimension {
        for column in 0..dimension {
            checkpoint_chunk(interrupt, row * dimension + column, phase)?;
            if row != column {
                let value = matrix[row * dimension + column];
                sum += value * value;
            }
        }
    }
    Ok(sum.sqrt())
}

fn rounding_gamma(operations: f64) -> Option<f64> {
    let product = operations * f64::EPSILON;
    (product.is_finite() && product < 0.5).then_some(product / (1.0 - product))
}

fn symmetrize_with_interrupt(
    matrix: &mut [f64],
    dimension: usize,
    interrupt: &mut dyn InterruptCheck,
    phase: &'static str,
) -> Result<()> {
    for row in 0..dimension {
        for column in 0..row {
            checkpoint_chunk(interrupt, row * dimension + column, phase)?;
            let average =
                0.5 * (matrix[row * dimension + column] + matrix[column * dimension + row]);
            matrix[row * dimension + column] = average;
            matrix[column * dimension + row] = average;
        }
    }
    Ok(())
}

fn scaled_inverse_residual(
    matrix: &[f64],
    scaled_inverse: &[f64],
    scale: &[f64],
    dimension: usize,
    interrupt: &mut dyn InterruptCheck,
    phase: &'static str,
) -> Result<f64> {
    let mut maximum = 0.0_f64;
    let mut work = 0_usize;
    for column in 0..dimension {
        let mut norm_square = 0.0;
        for row in 0..dimension {
            let mut value = -f64::from(row == column);
            for inner in 0..dimension {
                work = work.saturating_add(1);
                checkpoint_chunk(interrupt, work, phase)?;
                value += matrix[row * dimension + inner]
                    * scale[row]
                    * scale[inner]
                    * scaled_inverse[inner * dimension + column];
            }
            norm_square += value * value;
        }
        maximum = maximum.max(norm_square.sqrt());
    }
    Ok(maximum)
}

/// Return the maximum compatible-column residual for a zero-sum quotient in
/// the same diagonal coordinates used by the quotient spectral certificate.
/// If `D = diag(scale)`, the certified matrix is `S = D A D`, its normalized
/// null vector is proportional to `D^{-1} n`, and the supplied original-scale
/// quotient inverse `K` acts in scaled coordinates as `D^{-1} K D^{-1}`.
fn quotient_scaled_inverse_residual(
    matrix: &[f64],
    inverse: &[f64],
    scale: &[f64],
    dimension: usize,
    null_range: core::ops::Range<usize>,
    interrupt: &mut dyn InterruptCheck,
    phase: &'static str,
) -> Result<f64> {
    let entries = dimension.checked_mul(dimension).ok_or_else(|| {
        BackendError::new(
            ErrorCode::ResourceLimit,
            phase,
            "quotient residual dimension overflow",
        )
    })?;
    if dimension == 0
        || matrix.len() != entries
        || inverse.len() != entries
        || scale.len() != dimension
        || null_range.is_empty()
        || null_range.end > dimension
    {
        return Err(BackendError::invalid(
            phase,
            "quotient residual dimensions disagree",
        ));
    }

    let mut null = zeroed_f64_with_interrupt(dimension, phase, interrupt, phase)?;
    let mut null_norm_square = 0.0;
    for index in null_range {
        checkpoint_chunk(interrupt, index, phase)?;
        let value = scale[index].recip();
        null[index] = value;
        null_norm_square += value * value;
    }
    if !null_norm_square.is_finite() || null_norm_square <= 0.0 {
        return Err(BackendError::new(
            ErrorCode::InverseResidualFailed,
            phase,
            "scaled quotient null direction is not finite",
        ));
    }
    let null_norm_inverse = null_norm_square.sqrt().recip();
    for (index, value) in null.iter_mut().enumerate() {
        checkpoint_chunk(interrupt, index, phase)?;
        *value *= null_norm_inverse;
    }

    let mut rhs = zeroed_f64_with_interrupt(dimension, phase, interrupt, phase)?;
    let mut solution = zeroed_f64_with_interrupt(dimension, phase, interrupt, phase)?;
    let mut maximum = 0.0_f64;
    let mut work = 0_usize;
    for column in 0..dimension {
        let null_column = null[column];
        let mut rhs_norm_square = 0.0;
        for row in 0..dimension {
            work = work.saturating_add(1);
            checkpoint_chunk(interrupt, work, phase)?;
            rhs[row] = f64::from(row == column) - null[row] * null_column;
            rhs_norm_square += rhs[row] * rhs[row];
        }
        for row in 0..dimension {
            let mut value = 0.0;
            for inner in 0..dimension {
                work = work.saturating_add(1);
                checkpoint_chunk(interrupt, work, phase)?;
                value +=
                    inverse[row * dimension + inner] * rhs[inner] / (scale[row] * scale[inner]);
            }
            solution[row] = value;
        }
        let mut residual_norm_square = 0.0;
        for row in 0..dimension {
            let mut value = -rhs[row];
            for inner in 0..dimension {
                work = work.saturating_add(1);
                checkpoint_chunk(interrupt, work, phase)?;
                value +=
                    matrix[row * dimension + inner] * scale[row] * scale[inner] * solution[inner];
            }
            residual_norm_square += value * value;
        }
        let residual_norm = residual_norm_square.sqrt();
        let rhs_norm = rhs_norm_square.sqrt();
        let normalized = if rhs_norm > 0.0 {
            residual_norm / rhs_norm
        } else {
            residual_norm
        };
        if !normalized.is_finite() {
            return Err(BackendError::new(
                ErrorCode::InverseResidualFailed,
                phase,
                "scaled quotient inverse residual is not finite",
            ));
        }
        maximum = maximum.max(normalized);
    }
    Ok(maximum)
}

fn inverse_residual(
    matrix: &[f64],
    inverse: &[f64],
    dimension: usize,
    interrupt: &mut dyn InterruptCheck,
    phase: &'static str,
) -> Result<f64> {
    let mut maximum = 0.0_f64;
    let mut work = 0_usize;
    for column in 0..dimension {
        let mut norm_square = 0.0;
        for row in 0..dimension {
            let mut value = -f64::from(row == column);
            for inner in 0..dimension {
                work = work.saturating_add(1);
                checkpoint_chunk(interrupt, work, phase)?;
                value += matrix[row * dimension + inner] * inverse[inner * dimension + column];
            }
            norm_square += value * value;
        }
        maximum = maximum.max(norm_square.sqrt());
    }
    Ok(maximum)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interrupt::{InterruptCheck, NeverInterrupt};

    struct BreakFirst;

    impl InterruptCheck for BreakFirst {
        fn checkpoint(&mut self, phase: &'static str) -> Result<()> {
            Err(BackendError::new(
                ErrorCode::UserBreak,
                phase,
                "injected dense break",
            ))
        }
    }

    #[test]
    fn scaled_spectral_rcond_matches_analytic_two_by_two() {
        let rho = 0.75;
        let matrix = [4.0, 6.0 * rho, 6.0 * rho, 9.0];
        let inverse = invert_scaled_spd(&matrix, 2, 1.0e-12, &mut NeverInterrupt, "dense_test")
            .expect("SPD inverse");
        let expected = (1.0 - rho) / (1.0 + rho);
        assert!(inverse.rcond <= expected);
        assert!((inverse.rcond - expected).abs() < 1.0e-12);
    }

    #[test]
    fn scaled_spectral_rcond_is_scale_and_permutation_invariant() {
        let correlation = [1.0, 0.2, -0.1, 0.2, 1.0, 0.35, -0.1, 0.35, 1.0];
        let scale = [1.0e-2, 3.0, 1.0e2];
        let mut rescaled = vec![0.0; 9];
        for row in 0..3 {
            for column in 0..3 {
                rescaled[row * 3 + column] =
                    correlation[row * 3 + column] * scale[row] * scale[column];
            }
        }
        let order = [2, 0, 1];
        let mut permuted = vec![0.0; 9];
        for row in 0..3 {
            for column in 0..3 {
                permuted[row * 3 + column] = rescaled[order[row] * 3 + order[column]];
            }
        }
        let base = invert_scaled_spd(&correlation, 3, 1.0e-12, &mut NeverInterrupt, "dense_test")
            .expect("base inverse");
        let scaled = invert_scaled_spd(&rescaled, 3, 1.0e-12, &mut NeverInterrupt, "dense_test")
            .expect("scaled inverse");
        let reordered = invert_scaled_spd(&permuted, 3, 1.0e-12, &mut NeverInterrupt, "dense_test")
            .expect("permuted inverse");
        assert!((base.rcond - scaled.rcond).abs() < 1.0e-13);
        assert!((base.rcond - reordered.rcond).abs() < 1.0e-13);
    }

    #[test]
    fn dense_cholesky_and_inversion_preserve_injected_user_break() {
        let dimension = 96;
        let mut matrix = vec![0.0; dimension * dimension];
        for row in 0..dimension {
            matrix[row * dimension + row] = 2.0;
        }
        let factor_error =
            cholesky_factor(&matrix, dimension, &mut BreakFirst, "dense_cholesky_break")
                .expect_err("Cholesky must poll inside flattened work");
        assert_eq!(factor_error.code, ErrorCode::UserBreak);

        let inverse_error = invert_scaled_spd(
            &matrix,
            dimension,
            1.0e-10,
            &mut BreakFirst,
            "dense_inverse_break",
        )
        .expect_err("inverse must poll inside dense work");
        assert_eq!(inverse_error.code, ErrorCode::UserBreak);
    }
}
