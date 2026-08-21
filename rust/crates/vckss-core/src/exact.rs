// SPDX-License-Identifier: GPL-3.0-only

use crate::error::{BackendError, ErrorCode, Result};
use crate::interrupt::{checkpoint_chunk, InterruptCheck, NeverInterrupt};
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
    let mut interrupt = NeverInterrupt;
    assemble_symmetric_with_interrupt(operator, &mut interrupt)
}

pub fn assemble_symmetric_with_interrupt(
    operator: &impl SymmetricOperator,
    interrupt: &mut dyn InterruptCheck,
) -> Result<Vec<f64>> {
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
        interrupt.checkpoint("exact_assemble")?;
        basis.fill(0.0);
        basis[index] = 1.0;
        operator.apply_with_interrupt(&basis, &mut column, interrupt)?;
        for row in 0..dimension {
            checkpoint_chunk(interrupt, row, "exact_assemble_column")?;
            matrix[row * dimension + index] = column[row];
        }
    }
    for row in 0..dimension {
        interrupt.checkpoint("exact_symmetry")?;
        for column_index in 0..row {
            checkpoint_chunk(interrupt, column_index, "exact_symmetry")?;
            let average = 0.5
                * (matrix[row * dimension + column_index] + matrix[column_index * dimension + row]);
            matrix[row * dimension + column_index] = average;
            matrix[column_index * dimension + row] = average;
        }
    }
    for (index, value) in matrix.iter().enumerate() {
        checkpoint_chunk(interrupt, index, "exact_assemble_validate")?;
        if !value.is_finite() {
            return Err(BackendError::new(
                ErrorCode::InternalInvariantFailed,
                "exact",
                "assembled exact matrix is nonfinite",
            ));
        }
    }
    Ok(matrix)
}

pub fn cholesky_solve(matrix: &[f64], right_hand_side: &[f64]) -> Result<Vec<f64>> {
    let mut interrupt = NeverInterrupt;
    cholesky_solve_with_interrupt(matrix, right_hand_side, &mut interrupt)
}

pub fn cholesky_solve_with_interrupt(
    matrix: &[f64],
    right_hand_side: &[f64],
    interrupt: &mut dyn InterruptCheck,
) -> Result<Vec<f64>> {
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
    for (index, value) in matrix.iter().chain(right_hand_side).enumerate() {
        checkpoint_chunk(interrupt, index, "exact_validate")?;
        if !value.is_finite() {
            return Err(BackendError::invalid(
                "exact",
                "dense exact solve input is nonfinite",
            ));
        }
    }

    let mut factor = vec![0.0; matrix.len()];
    for row in 0..dimension {
        interrupt.checkpoint("exact_factor")?;
        for column in 0..=row {
            let mut value = matrix[row * dimension + column];
            for inner in 0..column {
                checkpoint_chunk(interrupt, inner, "exact_factor")?;
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
        interrupt.checkpoint("exact_forward")?;
        let mut value = right_hand_side[row];
        for column in 0..row {
            checkpoint_chunk(interrupt, column, "exact_forward")?;
            value -= factor[row * dimension + column] * intermediate[column];
        }
        intermediate[row] = value / factor[row * dimension + row];
    }

    let mut solution = vec![0.0; dimension];
    for row in (0..dimension).rev() {
        interrupt.checkpoint("exact_backward")?;
        let mut value = intermediate[row];
        for (work, column) in ((row + 1)..dimension).enumerate() {
            checkpoint_chunk(interrupt, work, "exact_backward")?;
            value -= factor[column * dimension + row] * solution[column];
        }
        solution[row] = value / factor[row * dimension + row];
    }
    for (index, value) in solution.iter().enumerate() {
        checkpoint_chunk(interrupt, index, "exact_solution_validate")?;
        if !value.is_finite() {
            return Err(BackendError::new(
                ErrorCode::InternalInvariantFailed,
                "exact",
                "dense exact solution is nonfinite",
            ));
        }
    }
    Ok(solution)
}

pub fn solve_two_way_exact(
    operator: &TwoWayOperator<'_>,
    worker_rhs: &[f64],
    firm_rhs: &[f64],
    full_residual_tolerance: f64,
) -> Result<ExactSolve> {
    let mut interrupt = NeverInterrupt;
    solve_two_way_exact_with_interrupt(
        operator,
        worker_rhs,
        firm_rhs,
        full_residual_tolerance,
        &mut interrupt,
    )
}

pub fn solve_two_way_exact_with_interrupt(
    operator: &TwoWayOperator<'_>,
    worker_rhs: &[f64],
    firm_rhs: &[f64],
    full_residual_tolerance: f64,
    interrupt: &mut dyn InterruptCheck,
) -> Result<ExactSolve> {
    if !full_residual_tolerance.is_finite() || full_residual_tolerance <= 0.0 {
        return Err(BackendError::invalid(
            "exact",
            "full residual tolerance must be positive and finite",
        ));
    }
    let reduced_rhs = operator.schur_rhs_with_interrupt(worker_rhs, firm_rhs, interrupt)?;
    let mut matrix = assemble_symmetric_with_interrupt(operator, interrupt)?;
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
    for (index, value) in matrix.iter_mut().enumerate() {
        checkpoint_chunk(interrupt, index, "exact_nullspace_shift")?;
        *value += dimension.recip();
    }
    let mut reduced_firm = cholesky_solve_with_interrupt(&matrix, &reduced_rhs, interrupt)?;
    operator.project(&mut reduced_firm)?;

    let mut reduced_action = vec![0.0; reduced_rhs.len()];
    operator.apply_with_interrupt(&reduced_firm, &mut reduced_action, interrupt)?;
    for (index, (value, rhs)) in reduced_action.iter_mut().zip(&reduced_rhs).enumerate() {
        checkpoint_chunk(interrupt, index, "exact_reduced_residual")?;
        *value -= rhs;
    }
    let rhs_norm = stable_norm(&reduced_rhs);
    let reduced_residual = if rhs_norm == 0.0 {
        stable_norm(&reduced_action)
    } else {
        stable_norm(&reduced_action) / rhs_norm
    };

    let firm = operator.expand_firm(&reduced_firm)?;
    let worker = operator.reconstruct_worker_with_interrupt(worker_rhs, &firm, interrupt)?;
    let residual =
        operator.full_residual_with_interrupt(&worker, &firm, worker_rhs, firm_rhs, interrupt)?;
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
