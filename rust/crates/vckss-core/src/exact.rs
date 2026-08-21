// SPDX-License-Identifier: GPL-3.0-only

use crate::error::{BackendError, ErrorCode, Result};
use crate::operator::{stable_norm, SymmetricOperator, TwoWayOperator, TwoWaySolution};

#[derive(Clone, Debug)]
pub struct ExactSolveReceipt {
    pub dimension: usize,
    pub reduced_residual: f64,
    pub full_residual: f64,
}

#[derive(Clone, Debug)]
pub struct ExactSolve {
    pub solution: TwoWaySolution,
    pub receipt: ExactSolveReceipt,
}

pub fn assemble_symmetric(operator: &impl SymmetricOperator) -> Result<Vec<f64>> {
    let dimension = operator.dimension();
    if dimension == 0 {
        return Err(BackendError::invalid(
            "exact",
            "exact operator dimension must be positive",
        ));
    }
    let entries = dimension.checked_mul(dimension).ok_or_else(|| {
        BackendError::new(
            ErrorCode::ResourceLimit,
            "exact",
            "dense exact matrix size overflow",
        )
    })?;
    let mut matrix = vec![0.0; entries];
    let mut basis = vec![0.0; dimension];
    let mut column = vec![0.0; dimension];
    for index in 0..dimension {
        basis.fill(0.0);
        basis[index] = 1.0;
        operator.apply(&basis, &mut column)?;
        for row in 0..dimension {
            matrix[row * dimension + index] = column[row];
        }
    }
    for row in 0..dimension {
        for column_index in 0..row {
            let average = 0.5
                * (matrix[row * dimension + column_index] + matrix[column_index * dimension + row]);
            matrix[row * dimension + column_index] = average;
            matrix[column_index * dimension + row] = average;
        }
    }
    if matrix.iter().any(|value| !value.is_finite()) {
        return Err(BackendError::new(
            ErrorCode::InternalInvariantFailed,
            "exact",
            "assembled exact matrix is nonfinite",
        ));
    }
    Ok(matrix)
}

pub fn cholesky_solve(matrix: &[f64], right_hand_side: &[f64]) -> Result<Vec<f64>> {
    let dimension = right_hand_side.len();
    if dimension == 0
        || matrix.len()
            != dimension.checked_mul(dimension).ok_or_else(|| {
                BackendError::new(
                    ErrorCode::ResourceLimit,
                    "exact",
                    "dense matrix dimension overflow",
                )
            })?
    {
        return Err(BackendError::invalid(
            "exact",
            "dense matrix and right-hand side dimensions are incompatible",
        ));
    }
    if matrix
        .iter()
        .chain(right_hand_side)
        .any(|value| !value.is_finite())
    {
        return Err(BackendError::invalid(
            "exact",
            "dense exact solve input is nonfinite",
        ));
    }

    let mut factor = vec![0.0; matrix.len()];
    for row in 0..dimension {
        for column in 0..=row {
            let mut value = matrix[row * dimension + column];
            for inner in 0..column {
                value -= factor[row * dimension + inner] * factor[column * dimension + inner];
            }
            if row == column {
                if !value.is_finite() || value <= 0.0 {
                    return Err(BackendError::new(
                        ErrorCode::GraphUnidentified,
                        "exact",
                        "identified exact matrix is not positive definite",
                    ));
                }
                factor[row * dimension + column] = value.sqrt();
            } else {
                factor[row * dimension + column] = value / factor[column * dimension + column];
            }
        }
    }

    let mut intermediate = vec![0.0; dimension];
    for row in 0..dimension {
        let mut value = right_hand_side[row];
        for column in 0..row {
            value -= factor[row * dimension + column] * intermediate[column];
        }
        intermediate[row] = value / factor[row * dimension + row];
    }

    let mut solution = vec![0.0; dimension];
    for row in (0..dimension).rev() {
        let mut value = intermediate[row];
        for column in (row + 1)..dimension {
            value -= factor[column * dimension + row] * solution[column];
        }
        solution[row] = value / factor[row * dimension + row];
    }
    if solution.iter().any(|value| !value.is_finite()) {
        return Err(BackendError::new(
            ErrorCode::InternalInvariantFailed,
            "exact",
            "dense exact solution is nonfinite",
        ));
    }
    Ok(solution)
}

pub fn solve_two_way_exact(
    operator: &TwoWayOperator<'_>,
    worker_rhs: &[f64],
    firm_rhs: &[f64],
    full_residual_tolerance: f64,
) -> Result<ExactSolve> {
    if !full_residual_tolerance.is_finite() || full_residual_tolerance <= 0.0 {
        return Err(BackendError::invalid(
            "exact",
            "full residual tolerance must be positive and finite",
        ));
    }
    let reduced_rhs = operator.schur_rhs(worker_rhs, firm_rhs)?;
    let mut matrix = assemble_symmetric(operator)?;
    let dimension = f64::from(u32::try_from(operator.dimension()).map_err(|_| {
        BackendError::new(
            ErrorCode::ResourceLimit,
            "exact",
            "firm quotient dimension exceeds the exact f64/u32 limit",
        )
    })?);
    // The Schur matrix is singular only on the constant vector.  Adding the
    // exact nullspace projector 11'/F makes the full-firm embedding positive
    // definite without changing its action or solution on the zero-sum
    // quotient.
    for value in &mut matrix {
        *value += dimension.recip();
    }
    let mut reduced_firm = cholesky_solve(&matrix, &reduced_rhs)?;
    operator.project(&mut reduced_firm)?;

    let mut reduced_action = vec![0.0; reduced_rhs.len()];
    operator.apply(&reduced_firm, &mut reduced_action)?;
    for (value, rhs) in reduced_action.iter_mut().zip(&reduced_rhs) {
        *value -= rhs;
    }
    let rhs_norm = stable_norm(&reduced_rhs);
    let reduced_residual = if rhs_norm == 0.0 {
        stable_norm(&reduced_action)
    } else {
        stable_norm(&reduced_action) / rhs_norm
    };

    let firm = operator.expand_firm(&reduced_firm)?;
    let worker = operator.reconstruct_worker(worker_rhs, &firm)?;
    let residual = operator.full_residual(&worker, &firm, worker_rhs, firm_rhs)?;
    if residual.relative_norm > full_residual_tolerance {
        return Err(BackendError::new(
            ErrorCode::FullResidualFailed,
            "exact",
            format!(
                "complete normal-equation residual {} exceeds tolerance {}",
                residual.relative_norm, full_residual_tolerance
            ),
        ));
    }

    Ok(ExactSolve {
        solution: TwoWaySolution {
            worker,
            firm,
            reduced_firm,
            residual: residual.clone(),
        },
        receipt: ExactSolveReceipt {
            dimension: operator.firm_quotient_parameter_count(),
            reduced_residual,
            full_residual: residual.relative_norm,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::operator::TwoWayOperator;
    use crate::problem::CanonicalInput;
    use crate::types::InputColumns;

    fn fixture() -> crate::problem::CompressedProblem {
        CanonicalInput::from_validated(
            InputColumns {
                worker: vec![1, 1, 2, 2],
                firm: vec![1, 2, 1, 2],
                deletion: vec![1, 2, 3, 4],
                outcome: vec![1.5, 0.5, -0.5, -1.5],
                frequency: vec![1; 4],
                target_weight: vec![1.0; 4],
                controls: Vec::new(),
            }
            .validate()
            .expect("fixture"),
        )
        .expect("canonical")
        .compress(&[true; 4])
        .expect("compressed")
    }

    #[test]
    fn exact_two_way_solve_recovers_known_effects() {
        let problem = fixture();
        let operator = TwoWayOperator::new(&problem).expect("operator");
        let (worker_rhs, firm_rhs) = operator.outcome_rhs().expect("rhs");
        let result =
            solve_two_way_exact(&operator, &worker_rhs, &firm_rhs, 1.0e-12).expect("solve");
        assert!((result.solution.worker[0] - 1.0).abs() < 1.0e-12);
        assert!((result.solution.worker[1] + 1.0).abs() < 1.0e-12);
        assert!((result.solution.firm[0] - 0.5).abs() < 1.0e-12);
        assert!((result.solution.firm[1] + 0.5).abs() < 1.0e-12);
        assert!(result.receipt.full_residual < 1.0e-12);
    }

    #[test]
    fn cholesky_rejects_non_spd_input() {
        let error = cholesky_solve(&[1.0, 2.0, 2.0, 1.0], &[1.0, 1.0])
            .expect_err("indefinite matrix must fail");
        assert_eq!(error.code, ErrorCode::GraphUnidentified);
    }
}
