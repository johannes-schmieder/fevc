// SPDX-License-Identifier: GPL-3.0-only

//! Outcome-free change of basis, not a variance-model selection procedure.
use super::{dot, invalid, Sum};
use crate::error::Result;
use crate::interrupt::{checkpoint_chunk, InterruptCheck};

const SPAN_TOLERANCE: f64 = 1.0e-12;

/// Keep original columns in deterministic order, including the intercept.
/// Twice-reorthogonalized residuals certify that each omitted column is in
/// the retained span to roundoff. Small but nonredundant directions survive
/// here and must pass the fitter's stricter conditioning test; no ridge or
/// outcome-dependent rank decision is made.
pub(crate) fn reduce(
    basis: &mut Vec<f64>,
    terms: usize,
    interrupt: &mut dyn InterruptCheck,
) -> Result<(Vec<usize>, f64)> {
    if terms == 0 || basis.is_empty() || basis.len() % terms != 0 {
        return Err(invalid("invalid residual-moment basis dimensions"));
    }
    let rows = basis.len() / terms;
    let mut orthogonal = Vec::<Vec<f64>>::with_capacity(terms);
    let mut kept = Vec::with_capacity(terms);
    let mut maximum_error = 0.0_f64;
    for column in 0..terms {
        interrupt.checkpoint("residual_moment_basis_span")?;
        let mut residual = Vec::with_capacity(rows);
        let mut norm = Sum::default();
        let mut scale = 0.0_f64;
        for row in 0..rows {
            checkpoint_chunk(interrupt, row, "residual_moment_basis_span")?;
            let value = basis[row * terms + column];
            if !value.is_finite() || (column == 0 && value != 1.0) {
                return Err(invalid("variance basis must be finite with an intercept"));
            }
            residual.push(value);
            norm.add(value * value);
            scale = scale.max(value.abs());
        }
        let norm = norm.value().sqrt();
        for _ in 0..2 {
            for direction in &orthogonal {
                let coefficient = dot(&residual, direction);
                for row in 0..rows {
                    checkpoint_chunk(interrupt, row, "residual_moment_basis_span")?;
                    residual[row] -= coefficient * direction[row];
                }
            }
        }
        let remaining = dot(&residual, &residual).sqrt();
        let max_remaining = residual.iter().fold(0.0_f64, |a, x| a.max(x.abs()));
        let error = if norm == 0.0 {
            0.0
        } else {
            (remaining / norm).max(max_remaining / scale)
        };
        if column != 0 && error <= SPAN_TOLERANCE {
            maximum_error = maximum_error.max(error);
        } else {
            if !remaining.is_finite() || remaining <= 0.0 {
                return Err(invalid("invalid residual-moment basis span"));
            }
            for value in &mut residual {
                *value /= remaining;
            }
            orthogonal.push(residual);
            kept.push(column);
        }
    }
    drop(orthogonal);
    // Kept columns retain their original scale and order; a full-rank input
    // stays byte-for-byte unchanged. Compact in place to bound peak storage.
    if kept.len() != terms {
        for row in 0..rows {
            checkpoint_chunk(interrupt, row, "residual_moment_basis_compact")?;
            for (active, &column) in kept.iter().enumerate() {
                basis[row * kept.len() + active] = basis[row * terms + column];
            }
        }
        basis.truncate(rows * kept.len());
    }
    Ok((kept, maximum_error))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interrupt::NeverInterrupt;

    #[test]
    fn zero_alias_and_interaction_columns_preserve_original_span() {
        let mut basis = Vec::new();
        for i in 0..100 {
            let x = (i as f64 - 50.0) / 100.0;
            basis.extend_from_slice(&[1.0, x, 0.0, x * x, 2.0 * x, 1.0 + x]);
        }
        let original = basis.clone();
        let (kept, error) = reduce(&mut basis, 6, &mut NeverInterrupt).unwrap();
        assert_eq!(kept, vec![0, 1, 3]);
        assert!(error < SPAN_TOLERANCE);
        for row in 0..100 {
            let [one, x, square] = basis[row * 3..row * 3 + 3].try_into().unwrap();
            let reconstructed = [one, x, 0.0, square, 2.0 * x, one + x];
            assert_eq!(&original[row * 6..row * 6 + 6], &reconstructed);
        }
    }

    #[test]
    fn near_alias_is_not_dropped_to_escape_conditioning_gate() {
        let mut basis = Vec::new();
        for i in 0..100 {
            let x = (i as f64 - 50.0) / 100.0;
            basis.extend_from_slice(&[1.0, x, x + 1.0e-7 * x * x]);
        }
        let original = basis.clone();
        let (kept, _) = reduce(&mut basis, 3, &mut NeverInterrupt).unwrap();
        assert_eq!(kept.len(), 3);
        assert_eq!(basis, original);
        let mut gram = vec![0.0; 9];
        for row in 0..100 {
            for i in 0..3 {
                for j in 0..3 {
                    gram[i * 3 + j] += basis[row * 3 + i] * basis[row * 3 + j];
                }
            }
        }
        assert!(crate::dense::invert_scaled_spd(
            &gram,
            3,
            1.0e-10,
            &mut NeverInterrupt,
            "basis_test"
        )
        .is_err());
    }
}
