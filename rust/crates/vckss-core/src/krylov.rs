// SPDX-License-Identifier: GPL-3.0-only

use crate::error::{BackendError, ErrorCode, Result};
use crate::operator::{stable_dot, stable_norm, SymmetricOperator, TwoWayOperator, TwoWaySolution};

pub trait Preconditioner {
    fn dimension(&self) -> usize;
    fn apply(&self, residual: &[f64], output: &mut [f64]) -> Result<()>;
}

#[derive(Clone, Debug)]
pub struct DiagonalPreconditioner {
    inverse: Vec<f64>,
}

impl DiagonalPreconditioner {
    pub fn new(diagonal: &[f64]) -> Result<Self> {
        if diagonal.is_empty()
            || diagonal
                .iter()
                .any(|&value| !value.is_finite() || value <= 0.0)
        {
            return Err(BackendError::new(
                ErrorCode::PcgPreconditionerBreakdown,
                "pcg",
                "preconditioner diagonal must be positive and finite",
            ));
        }
        let inverse: Vec<f64> = diagonal.iter().map(|&value| value.recip()).collect();
        if inverse.iter().any(|value| !value.is_finite()) {
            return Err(BackendError::new(
                ErrorCode::PcgPreconditionerBreakdown,
                "pcg",
                "inverse preconditioner diagonal is nonfinite",
            ));
        }
        Ok(Self { inverse })
    }
}

impl Preconditioner for DiagonalPreconditioner {
    fn dimension(&self) -> usize {
        self.inverse.len()
    }

    fn apply(&self, residual: &[f64], output: &mut [f64]) -> Result<()> {
        if residual.len() != self.dimension() || output.len() != self.dimension() {
            return Err(BackendError::invalid(
                "pcg",
                "preconditioner application has incompatible dimensions",
            ));
        }
        for ((value, &residual_value), &inverse) in
            output.iter_mut().zip(residual).zip(&self.inverse)
        {
            *value = inverse * residual_value;
        }
        if output.iter().any(|value| !value.is_finite()) {
            return Err(BackendError::new(
                ErrorCode::PcgPreconditionerBreakdown,
                "pcg",
                "preconditioner application produced a nonfinite value",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug)]
pub struct PcgOptions {
    pub tolerance: f64,
    pub maximum_iterations: u32,
    pub residual_replacement_interval: u32,
}

impl PcgOptions {
    pub fn validate(self) -> Result<Self> {
        if !self.tolerance.is_finite() || self.tolerance <= 0.0 || self.tolerance > 1.0 {
            return Err(BackendError::invalid(
                "pcg",
                "PCG tolerance must lie in (0, 1]",
            ));
        }
        if self.maximum_iterations == 0 {
            return Err(BackendError::invalid(
                "pcg",
                "PCG maximum iterations must be positive",
            ));
        }
        if self.residual_replacement_interval == 0 {
            return Err(BackendError::invalid(
                "pcg",
                "PCG residual-replacement interval must be positive",
            ));
        }
        Ok(self)
    }
}

#[derive(Clone, Debug)]
pub struct PcgReceipt {
    pub iterations: u32,
    pub relative_residual: f64,
    pub residual_replacements: u32,
    pub operator_applications: u32,
    pub preconditioner_applications: u32,
    pub zero_rhs: bool,
}

#[derive(Clone, Debug)]
pub struct PcgSolve {
    pub solution: Vec<f64>,
    pub receipt: PcgReceipt,
}

#[derive(Clone, Debug)]
pub struct TwoWayPcgSolve {
    pub solution: TwoWaySolution,
    pub pcg: PcgReceipt,
}

pub fn pcg(
    operator: &impl SymmetricOperator,
    preconditioner: &impl Preconditioner,
    right_hand_side: &[f64],
    options: PcgOptions,
) -> Result<PcgSolve> {
    let options = options.validate()?;
    let dimension = operator.dimension();
    if dimension == 0
        || right_hand_side.len() != dimension
        || preconditioner.dimension() != dimension
    {
        return Err(BackendError::invalid(
            "pcg",
            "operator, preconditioner, and right-hand side dimensions are incompatible",
        ));
    }
    if right_hand_side.iter().any(|value| !value.is_finite()) {
        return Err(BackendError::invalid("pcg", "right-hand side is nonfinite"));
    }

    let rhs_norm = stable_norm(right_hand_side);
    if rhs_norm == 0.0 {
        return Ok(PcgSolve {
            solution: vec![0.0; dimension],
            receipt: PcgReceipt {
                iterations: 0,
                relative_residual: 0.0,
                residual_replacements: 0,
                operator_applications: 0,
                preconditioner_applications: 0,
                zero_rhs: true,
            },
        });
    }

    let mut solution = vec![0.0; dimension];
    let mut residual = right_hand_side.to_vec();
    let mut preconditioned = vec![0.0; dimension];
    let mut direction = vec![0.0; dimension];
    let mut action = vec![0.0; dimension];
    let mut operator_applications = 0_u32;
    let mut preconditioner_applications = 0_u32;
    let mut residual_replacements = 0_u32;

    preconditioner.apply(&residual, &mut preconditioned)?;
    preconditioner_applications += 1;
    direction.copy_from_slice(&preconditioned);
    let mut residual_product = stable_dot(&residual, &preconditioned);
    require_positive_finite(
        residual_product,
        ErrorCode::PcgPreconditionerBreakdown,
        "initial preconditioned residual has nonpositive curvature",
    )?;
    let mut relative_residual = stable_norm(&residual) / rhs_norm;

    for iteration in 1..=options.maximum_iterations {
        operator.apply(&direction, &mut action)?;
        operator_applications += 1;
        let curvature = stable_dot(&direction, &action);
        require_positive_finite(
            curvature,
            ErrorCode::PcgCurvatureBreakdown,
            "search direction has nonpositive operator curvature",
        )?;
        let alpha = residual_product / curvature;
        if !alpha.is_finite() {
            return Err(BackendError::new(
                ErrorCode::PcgCurvatureBreakdown,
                "pcg",
                "PCG step length is nonfinite",
            ));
        }
        for index in 0..dimension {
            solution[index] += alpha * direction[index];
            residual[index] -= alpha * action[index];
        }
        if solution
            .iter()
            .chain(&residual)
            .any(|value| !value.is_finite())
        {
            return Err(BackendError::new(
                ErrorCode::PcgStagnation,
                "pcg",
                "PCG recurrence produced a nonfinite value",
            ));
        }

        let mut restarted = false;
        if iteration % options.residual_replacement_interval == 0 {
            operator.apply(&solution, &mut action)?;
            operator_applications += 1;
            for index in 0..dimension {
                residual[index] = right_hand_side[index] - action[index];
            }
            residual_replacements += 1;
            restarted = true;
        }

        relative_residual = stable_norm(&residual) / rhs_norm;
        if !relative_residual.is_finite() {
            return Err(BackendError::new(
                ErrorCode::PcgStagnation,
                "pcg",
                "PCG relative residual is nonfinite",
            ));
        }
        if relative_residual <= options.tolerance {
            operator.apply(&solution, &mut action)?;
            operator_applications += 1;
            for index in 0..dimension {
                residual[index] = right_hand_side[index] - action[index];
            }
            let verified = stable_norm(&residual) / rhs_norm;
            if !verified.is_finite() {
                return Err(BackendError::new(
                    ErrorCode::PcgStagnation,
                    "pcg",
                    "verified PCG residual is nonfinite",
                ));
            }
            if verified <= options.tolerance {
                return Ok(PcgSolve {
                    solution,
                    receipt: PcgReceipt {
                        iterations: iteration,
                        relative_residual: verified,
                        residual_replacements: residual_replacements + 1,
                        operator_applications,
                        preconditioner_applications,
                        zero_rhs: false,
                    },
                });
            }
            relative_residual = verified;
            residual_replacements += 1;
            restarted = true;
        }

        preconditioner.apply(&residual, &mut preconditioned)?;
        preconditioner_applications += 1;
        let next_product = stable_dot(&residual, &preconditioned);
        require_positive_finite(
            next_product,
            ErrorCode::PcgPreconditionerBreakdown,
            "preconditioned residual has nonpositive curvature",
        )?;
        if restarted {
            direction.copy_from_slice(&preconditioned);
            residual_product = next_product;
            continue;
        }
        let beta = next_product / residual_product;
        if !beta.is_finite() || beta < 0.0 {
            return Err(BackendError::new(
                ErrorCode::PcgPreconditionerBreakdown,
                "pcg",
                "PCG direction coefficient is invalid",
            ));
        }
        for index in 0..dimension {
            direction[index] = preconditioned[index] + beta * direction[index];
        }
        residual_product = next_product;
    }

    Err(BackendError::new(
        ErrorCode::PcgMaxIterations,
        "pcg",
        format!(
            "PCG did not converge in {} iterations; final relative residual was {}",
            options.maximum_iterations, relative_residual
        ),
    ))
}

pub fn solve_two_way_pcg(
    operator: &TwoWayOperator<'_>,
    worker_rhs: &[f64],
    firm_rhs: &[f64],
    options: PcgOptions,
    full_residual_tolerance: f64,
) -> Result<TwoWayPcgSolve> {
    if !full_residual_tolerance.is_finite() || full_residual_tolerance <= 0.0 {
        return Err(BackendError::invalid(
            "pcg",
            "full residual tolerance must be positive and finite",
        ));
    }
    let reduced_rhs = operator.schur_rhs(worker_rhs, firm_rhs)?;
    let preconditioner = DiagonalPreconditioner::new(operator.reduced_diagonal())?;
    let reduced = pcg(operator, &preconditioner, &reduced_rhs, options)?;
    let firm = operator.expand_firm(&reduced.solution)?;
    let worker = operator.reconstruct_worker(worker_rhs, &firm)?;
    let residual = operator.full_residual(&worker, &firm, worker_rhs, firm_rhs)?;
    if residual.relative_norm > full_residual_tolerance {
        return Err(BackendError::new(
            ErrorCode::FullResidualFailed,
            "pcg",
            format!(
                "complete normal-equation residual {} exceeds tolerance {}",
                residual.relative_norm, full_residual_tolerance
            ),
        ));
    }
    Ok(TwoWayPcgSolve {
        solution: TwoWaySolution {
            worker,
            firm,
            reduced_firm: reduced.solution,
            residual,
        },
        pcg: reduced.receipt,
    })
}

fn require_positive_finite(value: f64, code: ErrorCode, message: &'static str) -> Result<()> {
    if value.is_finite() && value > 0.0 {
        Ok(())
    } else {
        Err(BackendError::new(code, "pcg", message))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exact::solve_two_way_exact;
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
    fn diagonal_pcg_matches_exact_two_way_solution() {
        let problem = fixture();
        let operator = TwoWayOperator::new(&problem).expect("operator");
        let (worker_rhs, firm_rhs) = operator.outcome_rhs().expect("rhs");
        let exact = solve_two_way_exact(&operator, &worker_rhs, &firm_rhs, 1.0e-12).expect("exact");
        let iterative = solve_two_way_pcg(
            &operator,
            &worker_rhs,
            &firm_rhs,
            PcgOptions {
                tolerance: 1.0e-12,
                maximum_iterations: 100,
                residual_replacement_interval: 10,
            },
            1.0e-11,
        )
        .expect("PCG");
        for (left, right) in iterative.solution.worker.iter().zip(&exact.solution.worker) {
            assert!((left - right).abs() < 1.0e-11);
        }
        for (left, right) in iterative.solution.firm.iter().zip(&exact.solution.firm) {
            assert!((left - right).abs() < 1.0e-11);
        }
        assert!(iterative.solution.residual.relative_norm < 1.0e-11);
    }

    #[test]
    fn zero_rhs_returns_exact_zero_without_operator_work() {
        let problem = fixture();
        let operator = TwoWayOperator::new(&problem).expect("operator");
        let preconditioner =
            DiagonalPreconditioner::new(operator.reduced_diagonal()).expect("preconditioner");
        let result = pcg(
            &operator,
            &preconditioner,
            &vec![0.0; operator.dimension()],
            PcgOptions {
                tolerance: 1.0e-12,
                maximum_iterations: 100,
                residual_replacement_interval: 10,
            },
        )
        .expect("zero solve");
        assert!(result.receipt.zero_rhs);
        assert_eq!(result.receipt.operator_applications, 0);
        assert_eq!(result.solution, vec![0.0; operator.dimension()]);
    }
}
