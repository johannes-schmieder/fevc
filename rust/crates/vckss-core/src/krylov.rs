// SPDX-License-Identifier: GPL-3.0-only

use crate::error::{BackendError, ErrorCode, Result};
use crate::interrupt::{checkpoint_chunk, InterruptCheck, NeverInterrupt};
use crate::operator::{stable_dot, stable_norm, SymmetricOperator, TwoWayOperator, TwoWaySolution};

pub trait Preconditioner {
    fn dimension(&self) -> usize;
    fn apply(&self, residual: &[f64], output: &mut [f64]) -> Result<()>;

    fn apply_with_interrupt(
        &self,
        residual: &[f64],
        output: &mut [f64],
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<()> {
        interrupt.checkpoint("pcg_preconditioner")?;
        self.apply(residual, output)
    }
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
        let mut interrupt = NeverInterrupt;
        self.apply_with_interrupt(residual, output, &mut interrupt)
    }

    fn apply_with_interrupt(
        &self,
        residual: &[f64],
        output: &mut [f64],
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<()> {
        if residual.len() != self.dimension() || output.len() != self.dimension() {
            return Err(BackendError::invalid(
                "pcg",
                "preconditioner application has incompatible dimensions",
            ));
        }
        for (index, ((value, &residual_value), &inverse)) in output
            .iter_mut()
            .zip(residual)
            .zip(&self.inverse)
            .enumerate()
        {
            checkpoint_chunk(interrupt, index, "pcg_preconditioner")?;
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
    let mut interrupt = NeverInterrupt;
    pcg_with_interrupt(
        operator,
        preconditioner,
        right_hand_side,
        options,
        &mut interrupt,
    )
}

pub fn pcg_with_interrupt(
    operator: &impl SymmetricOperator,
    preconditioner: &impl Preconditioner,
    right_hand_side: &[f64],
    options: PcgOptions,
    interrupt: &mut dyn InterruptCheck,
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

    let mut projected_rhs = right_hand_side.to_vec();
    operator.project(&mut projected_rhs)?;
    let rhs_norm = stable_norm(&projected_rhs);
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
    let mut residual = projected_rhs.clone();
    let mut preconditioned = vec![0.0; dimension];
    let mut direction = vec![0.0; dimension];
    let mut action = vec![0.0; dimension];
    let mut operator_applications = 0_u32;
    let mut preconditioner_applications = 0_u32;
    let mut residual_replacements = 0_u32;

    interrupt.checkpoint("pcg_initial")?;
    preconditioner.apply_with_interrupt(&residual, &mut preconditioned, interrupt)?;
    operator.project(&mut preconditioned)?;
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
        interrupt.checkpoint("pcg_iteration")?;
        operator.apply_with_interrupt(&direction, &mut action, interrupt)?;
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
            checkpoint_chunk(interrupt, index, "pcg_recurrence")?;
            solution[index] += alpha * direction[index];
            residual[index] -= alpha * action[index];
        }
        operator.project(&mut residual)?;
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
        let recomputed = iteration % options.residual_replacement_interval == 0;
        if recomputed {
            interrupt.checkpoint("pcg_residual_replacement")?;
            operator.apply_with_interrupt(&solution, &mut action, interrupt)?;
            operator_applications += 1;
            for index in 0..dimension {
                action[index] = projected_rhs[index] - action[index];
            }
            operator.project(&mut action)?;
            residual_replacements += 1;
            let explicit_norm = stable_norm(&action);
            let drift_norm = stable_difference_norm(&action, &residual);
            if !explicit_norm.is_finite() || !drift_norm.is_finite() {
                return Err(BackendError::new(
                    ErrorCode::PcgStagnation,
                    "pcg",
                    "explicit PCG residual or recurrence drift is nonfinite",
                ));
            }
            let drift_gate = (1.0e-14 * rhs_norm).max(0.1 * options.tolerance * rhs_norm);
            if explicit_norm <= options.tolerance * rhs_norm || drift_norm > drift_gate {
                residual.copy_from_slice(&action);
                restarted = true;
            }
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
            if !recomputed {
                interrupt.checkpoint("pcg_verify")?;
                operator.apply_with_interrupt(&solution, &mut action, interrupt)?;
                operator_applications += 1;
                for index in 0..dimension {
                    action[index] = projected_rhs[index] - action[index];
                }
                operator.project(&mut action)?;
                residual_replacements += 1;
            }
            let verified = stable_norm(&action) / rhs_norm;
            if !verified.is_finite() {
                return Err(BackendError::new(
                    ErrorCode::PcgStagnation,
                    "pcg",
                    "verified PCG residual is nonfinite",
                ));
            }
            if verified <= options.tolerance {
                operator.project(&mut solution)?;
                return Ok(PcgSolve {
                    solution,
                    receipt: PcgReceipt {
                        iterations: iteration,
                        relative_residual: verified,
                        residual_replacements,
                        operator_applications,
                        preconditioner_applications,
                        zero_rhs: false,
                    },
                });
            }
            relative_residual = verified;
            residual.copy_from_slice(&action);
            restarted = true;
        }

        preconditioner.apply_with_interrupt(&residual, &mut preconditioned, interrupt)?;
        operator.project(&mut preconditioned)?;
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
            checkpoint_chunk(interrupt, index, "pcg_direction")?;
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

fn stable_difference_norm(left: &[f64], right: &[f64]) -> f64 {
    let mut sum = 0.0;
    let mut correction = 0.0;
    for (&left_value, &right_value) in left.iter().zip(right) {
        let difference = left_value - right_value;
        let square = difference * difference;
        let adjusted = square - correction;
        let next = sum + adjusted;
        correction = (next - sum) - adjusted;
        sum = next;
    }
    sum.max(0.0).sqrt()
}

pub fn solve_two_way_pcg(
    operator: &TwoWayOperator<'_>,
    worker_rhs: &[f64],
    firm_rhs: &[f64],
    options: PcgOptions,
    full_residual_tolerance: f64,
) -> Result<TwoWayPcgSolve> {
    let mut interrupt = NeverInterrupt;
    solve_two_way_pcg_with_interrupt(
        operator,
        worker_rhs,
        firm_rhs,
        options,
        full_residual_tolerance,
        &mut interrupt,
    )
}

pub fn solve_two_way_pcg_with_interrupt(
    operator: &TwoWayOperator<'_>,
    worker_rhs: &[f64],
    firm_rhs: &[f64],
    options: PcgOptions,
    full_residual_tolerance: f64,
    interrupt: &mut dyn InterruptCheck,
) -> Result<TwoWayPcgSolve> {
    if !full_residual_tolerance.is_finite() || full_residual_tolerance <= 0.0 {
        return Err(BackendError::invalid(
            "pcg",
            "full residual tolerance must be positive and finite",
        ));
    }
    let reduced_rhs = operator.schur_rhs_with_interrupt(worker_rhs, firm_rhs, interrupt)?;
    let preconditioner = DiagonalPreconditioner::new(operator.reduced_diagonal())?;
    let reduced = pcg_with_interrupt(operator, &preconditioner, &reduced_rhs, options, interrupt)?;
    let firm = operator.expand_firm(&reduced.solution)?;
    let worker = operator.reconstruct_worker_with_interrupt(worker_rhs, &firm, interrupt)?;
    let residual =
        operator.full_residual_with_interrupt(&worker, &firm, worker_rhs, firm_rhs, interrupt)?;
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
