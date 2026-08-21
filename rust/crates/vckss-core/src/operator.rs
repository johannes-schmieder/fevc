// SPDX-License-Identifier: GPL-3.0-only

use crate::error::{BackendError, ErrorCode, Result};
use crate::interrupt::{checkpoint_chunk, InterruptCheck, NeverInterrupt};
use crate::parallel::compensated_sum;
use crate::problem::CompressedProblem;

pub trait SymmetricOperator {
    fn dimension(&self) -> usize;
    fn apply(&self, input: &[f64], output: &mut [f64]) -> Result<()>;

    fn apply_with_interrupt(
        &self,
        input: &[f64],
        output: &mut [f64],
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<()> {
        interrupt.checkpoint("operator_apply")?;
        self.apply(input, output)
    }

    fn project(&self, values: &mut [f64]) -> Result<()> {
        if values.len() != self.dimension() || values.iter().any(|value| !value.is_finite()) {
            return Err(BackendError::invalid(
                "operator",
                "operator projection has incompatible dimensions",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct FullResidual {
    pub worker: Vec<f64>,
    pub firm: Vec<f64>,
    pub absolute_norm: f64,
    pub relative_norm: f64,
    pub rhs_norm: f64,
}

#[derive(Clone, Debug)]
pub struct TwoWaySolution {
    pub worker: Vec<f64>,
    pub firm: Vec<f64>,
    pub reduced_firm: Vec<f64>,
    pub residual: FullResidual,
}

#[derive(Clone, Debug)]
pub struct TwoWayOperator<'a> {
    problem: &'a CompressedProblem,
    worker_diagonal: Vec<f64>,
    firm_diagonal: Vec<f64>,
    reduced_diagonal: Vec<f64>,
}

impl<'a> TwoWayOperator<'a> {
    pub fn new(problem: &'a CompressedProblem) -> Result<Self> {
        let mut interrupt = NeverInterrupt;
        Self::new_with_interrupt(problem, &mut interrupt)
    }

    pub fn new_with_interrupt(
        problem: &'a CompressedProblem,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<Self> {
        interrupt.checkpoint("operator_setup")?;
        if !problem.controls.is_empty() {
            return Err(BackendError::new(
                ErrorCode::UnsupportedFeature,
                "operator",
                "the initial two-way operator does not yet include controls",
            ));
        }
        if problem.firms() < 2 || problem.workers() == 0 || problem.cells() == 0 {
            return Err(BackendError::new(
                ErrorCode::GraphUnidentified,
                "operator",
                "two-way operator requires workers, cells, and at least two firms",
            ));
        }

        let mut worker_diagonal = vec![0.0; problem.workers()];
        let mut firm_diagonal = vec![0.0; problem.firms()];
        for cell in 0..problem.cells() {
            checkpoint_chunk(interrupt, cell, "operator_setup_cells")?;
            let weight = problem.cell_weight[cell];
            if !weight.is_finite() || weight <= 0.0 {
                return Err(BackendError::new(
                    ErrorCode::InvalidWeight,
                    "operator",
                    "cell information weight must be positive and finite",
                ));
            }
            let worker = usize::try_from(problem.cell_worker[cell]).expect("dense worker");
            let firm = usize::try_from(problem.cell_firm[cell]).expect("dense firm");
            worker_diagonal[worker] += weight;
            firm_diagonal[firm] += weight;
        }
        for (index, &value) in worker_diagonal.iter().chain(&firm_diagonal).enumerate() {
            checkpoint_chunk(interrupt, index, "operator_setup_diagonal")?;
            if !value.is_finite() || value <= 0.0 {
                return Err(BackendError::new(
                    ErrorCode::GraphUnidentified,
                    "operator",
                    "worker or firm information diagonal is not positive and finite",
                ));
            }
        }

        let reduced_diagonal =
            reduced_diagonal_with_interrupt(problem, &worker_diagonal, &firm_diagonal, interrupt)?;
        Ok(Self {
            problem,
            worker_diagonal,
            firm_diagonal,
            reduced_diagonal,
        })
    }

    #[must_use]
    pub fn problem(&self) -> &'a CompressedProblem {
        self.problem
    }

    #[must_use]
    pub fn worker_diagonal(&self) -> &[f64] {
        &self.worker_diagonal
    }

    #[must_use]
    pub fn firm_diagonal(&self) -> &[f64] {
        &self.firm_diagonal
    }

    #[must_use]
    pub fn reduced_diagonal(&self) -> &[f64] {
        &self.reduced_diagonal
    }

    #[must_use]
    pub fn firm_quotient_parameter_count(&self) -> usize {
        self.problem.firms() - 1
    }

    pub fn expand_firm(&self, reduced: &[f64]) -> Result<Vec<f64>> {
        if reduced.len() != self.dimension() {
            return Err(BackendError::invalid(
                "operator",
                "firm quotient vector has the wrong dimension",
            ));
        }
        if reduced.iter().any(|value| !value.is_finite()) {
            return Err(BackendError::invalid(
                "operator",
                "firm quotient vector is nonfinite",
            ));
        }
        let mut firm = reduced.to_vec();
        center(&mut firm)?;
        Ok(firm)
    }

    pub fn reduce_full_firm(&self, full: &[f64]) -> Result<Vec<f64>> {
        if full.len() != self.problem.firms() {
            return Err(BackendError::invalid(
                "operator",
                "full firm vector has the wrong dimension",
            ));
        }
        if full.iter().any(|value| !value.is_finite()) {
            return Err(BackendError::invalid(
                "operator",
                "full firm vector is nonfinite",
            ));
        }
        let mut quotient = full.to_vec();
        center(&mut quotient)?;
        Ok(quotient)
    }

    pub fn apply_full_schur(&self, firm: &[f64], output: &mut [f64]) -> Result<()> {
        let mut interrupt = NeverInterrupt;
        self.apply_full_schur_with_interrupt(firm, output, &mut interrupt)
    }

    pub fn apply_full_schur_with_interrupt(
        &self,
        firm: &[f64],
        output: &mut [f64],
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<()> {
        if firm.len() != self.problem.firms() || output.len() != self.problem.firms() {
            return Err(BackendError::invalid(
                "operator",
                "full Schur action has incompatible dimensions",
            ));
        }
        if firm.iter().any(|value| !value.is_finite()) {
            return Err(BackendError::invalid(
                "operator",
                "full Schur input is nonfinite",
            ));
        }

        let mut worker_sum = vec![0.0; self.problem.workers()];
        output.fill(0.0);
        for cell in 0..self.problem.cells() {
            checkpoint_chunk(interrupt, cell, "operator_schur")?;
            let worker = usize::try_from(self.problem.cell_worker[cell]).expect("dense worker");
            let firm_index = usize::try_from(self.problem.cell_firm[cell]).expect("dense firm");
            worker_sum[worker] += self.problem.cell_weight[cell] * firm[firm_index];
        }
        for (index, (value, diagonal)) in
            worker_sum.iter_mut().zip(&self.worker_diagonal).enumerate()
        {
            checkpoint_chunk(interrupt, index, "operator_schur")?;
            *value /= diagonal;
        }
        for (value, (&diagonal, &coefficient)) in
            output.iter_mut().zip(self.firm_diagonal.iter().zip(firm))
        {
            *value = diagonal * coefficient;
        }
        for cell in 0..self.problem.cells() {
            checkpoint_chunk(interrupt, cell, "operator_schur")?;
            let worker = usize::try_from(self.problem.cell_worker[cell]).expect("dense worker");
            let firm_index = usize::try_from(self.problem.cell_firm[cell]).expect("dense firm");
            output[firm_index] -= self.problem.cell_weight[cell] * worker_sum[worker];
        }
        if output.iter().any(|value| !value.is_finite()) {
            return Err(BackendError::new(
                ErrorCode::InternalInvariantFailed,
                "operator",
                "full Schur action produced a nonfinite value",
            ));
        }
        Ok(())
    }

    pub fn schur_rhs(&self, worker_rhs: &[f64], firm_rhs: &[f64]) -> Result<Vec<f64>> {
        let mut interrupt = NeverInterrupt;
        self.schur_rhs_with_interrupt(worker_rhs, firm_rhs, &mut interrupt)
    }

    pub fn schur_rhs_with_interrupt(
        &self,
        worker_rhs: &[f64],
        firm_rhs: &[f64],
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<Vec<f64>> {
        if worker_rhs.len() != self.problem.workers() || firm_rhs.len() != self.problem.firms() {
            return Err(BackendError::invalid(
                "operator",
                "two-way right-hand side has incompatible dimensions",
            ));
        }
        if worker_rhs
            .iter()
            .chain(firm_rhs)
            .any(|value| !value.is_finite())
        {
            return Err(BackendError::invalid(
                "operator",
                "two-way right-hand side is nonfinite",
            ));
        }
        let worker_scaled: Vec<f64> = worker_rhs
            .iter()
            .zip(&self.worker_diagonal)
            .map(|(&value, &diagonal)| value / diagonal)
            .collect();
        let mut full = firm_rhs.to_vec();
        for cell in 0..self.problem.cells() {
            checkpoint_chunk(interrupt, cell, "operator_rhs")?;
            let worker = usize::try_from(self.problem.cell_worker[cell]).expect("dense worker");
            let firm = usize::try_from(self.problem.cell_firm[cell]).expect("dense firm");
            full[firm] -= self.problem.cell_weight[cell] * worker_scaled[worker];
        }
        self.reduce_full_firm(&full)
    }

    pub fn reconstruct_worker(&self, worker_rhs: &[f64], firm: &[f64]) -> Result<Vec<f64>> {
        let mut interrupt = NeverInterrupt;
        self.reconstruct_worker_with_interrupt(worker_rhs, firm, &mut interrupt)
    }

    pub fn reconstruct_worker_with_interrupt(
        &self,
        worker_rhs: &[f64],
        firm: &[f64],
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<Vec<f64>> {
        if worker_rhs.len() != self.problem.workers() || firm.len() != self.problem.firms() {
            return Err(BackendError::invalid(
                "operator",
                "worker reconstruction has incompatible dimensions",
            ));
        }
        let mut worker = worker_rhs.to_vec();
        for cell in 0..self.problem.cells() {
            checkpoint_chunk(interrupt, cell, "operator_reconstruct")?;
            let worker_index =
                usize::try_from(self.problem.cell_worker[cell]).expect("dense worker");
            let firm_index = usize::try_from(self.problem.cell_firm[cell]).expect("dense firm");
            worker[worker_index] -= self.problem.cell_weight[cell] * firm[firm_index];
        }
        for (index, (value, diagonal)) in worker.iter_mut().zip(&self.worker_diagonal).enumerate() {
            checkpoint_chunk(interrupt, index, "operator_reconstruct")?;
            *value /= diagonal;
        }
        if worker.iter().any(|value| !value.is_finite()) {
            return Err(BackendError::new(
                ErrorCode::InternalInvariantFailed,
                "operator",
                "worker reconstruction produced a nonfinite value",
            ));
        }
        Ok(worker)
    }

    pub fn outcome_rhs(&self) -> Result<(Vec<f64>, Vec<f64>)> {
        let mut interrupt = NeverInterrupt;
        self.outcome_rhs_with_interrupt(&mut interrupt)
    }

    pub fn outcome_rhs_with_interrupt(
        &self,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<(Vec<f64>, Vec<f64>)> {
        let mut worker = vec![0.0; self.problem.workers()];
        let mut firm = vec![0.0; self.problem.firms()];
        for cell in 0..self.problem.cells() {
            checkpoint_chunk(interrupt, cell, "operator_outcome_rhs")?;
            let worker_index =
                usize::try_from(self.problem.cell_worker[cell]).expect("dense worker");
            let firm_index = usize::try_from(self.problem.cell_firm[cell]).expect("dense firm");
            let value = self.problem.cell_outcome_sum[cell];
            if !value.is_finite() {
                return Err(BackendError::new(
                    ErrorCode::InvalidInput,
                    "operator",
                    "cell outcome moment is nonfinite",
                ));
            }
            worker[worker_index] += value;
            firm[firm_index] += value;
        }
        Ok((worker, firm))
    }

    pub fn full_residual(
        &self,
        worker: &[f64],
        firm: &[f64],
        worker_rhs: &[f64],
        firm_rhs: &[f64],
    ) -> Result<FullResidual> {
        let mut interrupt = NeverInterrupt;
        self.full_residual_with_interrupt(worker, firm, worker_rhs, firm_rhs, &mut interrupt)
    }

    pub fn full_residual_with_interrupt(
        &self,
        worker: &[f64],
        firm: &[f64],
        worker_rhs: &[f64],
        firm_rhs: &[f64],
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<FullResidual> {
        if worker.len() != self.problem.workers()
            || firm.len() != self.problem.firms()
            || worker_rhs.len() != self.problem.workers()
            || firm_rhs.len() != self.problem.firms()
        {
            return Err(BackendError::invalid(
                "operator",
                "full residual has incompatible dimensions",
            ));
        }
        if worker
            .iter()
            .chain(firm)
            .chain(worker_rhs)
            .chain(firm_rhs)
            .any(|value| !value.is_finite())
        {
            return Err(BackendError::invalid(
                "operator",
                "full residual input is nonfinite",
            ));
        }

        let mut worker_residual: Vec<f64> = worker_rhs
            .iter()
            .zip(worker.iter().zip(&self.worker_diagonal))
            .map(|(&rhs, (&coefficient, &diagonal))| rhs - diagonal * coefficient)
            .collect();
        let mut firm_residual: Vec<f64> = firm_rhs
            .iter()
            .zip(firm.iter().zip(&self.firm_diagonal))
            .map(|(&rhs, (&coefficient, &diagonal))| rhs - diagonal * coefficient)
            .collect();
        for cell in 0..self.problem.cells() {
            checkpoint_chunk(interrupt, cell, "operator_full_residual")?;
            let worker_index =
                usize::try_from(self.problem.cell_worker[cell]).expect("dense worker");
            let firm_index = usize::try_from(self.problem.cell_firm[cell]).expect("dense firm");
            let weight = self.problem.cell_weight[cell];
            worker_residual[worker_index] -= weight * firm[firm_index];
            firm_residual[firm_index] -= weight * worker[worker_index];
        }

        let absolute_norm = combined_norm(&worker_residual, &firm_residual);
        let rhs_norm = combined_norm(worker_rhs, firm_rhs);
        let relative_norm = if rhs_norm == 0.0 {
            absolute_norm
        } else {
            absolute_norm / rhs_norm
        };
        if !absolute_norm.is_finite() || !relative_norm.is_finite() {
            return Err(BackendError::new(
                ErrorCode::FullResidualFailed,
                "operator",
                "full residual is nonfinite",
            ));
        }
        Ok(FullResidual {
            worker: worker_residual,
            firm: firm_residual,
            absolute_norm,
            relative_norm,
            rhs_norm,
        })
    }
}

impl SymmetricOperator for TwoWayOperator<'_> {
    fn dimension(&self) -> usize {
        self.problem.firms()
    }

    fn apply(&self, input: &[f64], output: &mut [f64]) -> Result<()> {
        let mut interrupt = NeverInterrupt;
        self.apply_with_interrupt(input, output, &mut interrupt)
    }

    fn apply_with_interrupt(
        &self,
        input: &[f64],
        output: &mut [f64],
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<()> {
        if input.len() != self.dimension() || output.len() != self.dimension() {
            return Err(BackendError::invalid(
                "operator",
                "reduced Schur action has incompatible dimensions",
            ));
        }
        let firm = self.expand_firm(input)?;
        self.apply_full_schur_with_interrupt(&firm, output, interrupt)?;
        center(output)?;
        Ok(())
    }

    fn project(&self, values: &mut [f64]) -> Result<()> {
        if values.len() != self.dimension() {
            return Err(BackendError::invalid(
                "operator",
                "firm quotient projection has the wrong dimension",
            ));
        }
        center(values)
    }
}

fn reduced_diagonal_with_interrupt(
    problem: &CompressedProblem,
    worker_diagonal: &[f64],
    firm_diagonal: &[f64],
    interrupt: &mut dyn InterruptCheck,
) -> Result<Vec<f64>> {
    let mut full_diagonal = firm_diagonal.to_vec();
    for cell in 0..problem.cells() {
        checkpoint_chunk(interrupt, cell, "operator_setup_reduced_diagonal")?;
        let worker = usize::try_from(problem.cell_worker[cell]).expect("dense worker");
        let firm = usize::try_from(problem.cell_firm[cell]).expect("dense firm");
        let weight = problem.cell_weight[cell];
        full_diagonal[firm] -= weight * weight / worker_diagonal[worker];
    }

    for (index, &value) in full_diagonal.iter().enumerate() {
        checkpoint_chunk(interrupt, index, "operator_setup_reduced_diagonal")?;
        if !value.is_finite() || value <= 0.0 {
            return Err(BackendError::new(
                ErrorCode::GraphUnidentified,
                "operator",
                "reduced Schur diagonal is not positive and finite",
            ));
        }
    }
    Ok(full_diagonal)
}

fn center(values: &mut [f64]) -> Result<()> {
    if values.is_empty() || values.iter().any(|value| !value.is_finite()) {
        return Err(BackendError::invalid(
            "operator",
            "cannot center an empty or nonfinite firm quotient vector",
        ));
    }
    let count = f64::from(u32::try_from(values.len()).map_err(|_| {
        BackendError::new(
            ErrorCode::ResourceLimit,
            "operator",
            "firm quotient dimension exceeds the exact f64/u32 limit",
        )
    })?);
    let mean = compensated_sum(values) / count;
    for value in values {
        *value -= mean;
    }
    Ok(())
}

#[must_use]
pub fn stable_dot(left: &[f64], right: &[f64]) -> f64 {
    let mut sum = 0.0;
    let mut correction = 0.0;
    for (&left_value, &right_value) in left.iter().zip(right) {
        let product = left_value * right_value;
        let adjusted = product - correction;
        let next = sum + adjusted;
        correction = (next - sum) - adjusted;
        sum = next;
    }
    sum
}

#[must_use]
pub fn stable_norm(values: &[f64]) -> f64 {
    stable_dot(values, values).max(0.0).sqrt()
}

#[must_use]
fn combined_norm(left: &[f64], right: &[f64]) -> f64 {
    (stable_dot(left, left) + stable_dot(right, right))
        .max(0.0)
        .sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::problem::CanonicalInput;
    use crate::types::InputColumns;

    fn problem() -> CompressedProblem {
        let worker = [1_usize, 1, 2, 2];
        let firm = [1_usize, 2, 1, 2];
        let worker_effect = [1.0, -1.0];
        let firm_effect = [0.5, -0.5];
        let outcome: Vec<f64> = (0..4)
            .map(|row| worker_effect[worker[row] - 1] + firm_effect[firm[row] - 1])
            .collect();
        CanonicalInput::from_validated(
            InputColumns {
                worker: worker.iter().map(|&value| value as u64).collect(),
                firm: firm.iter().map(|&value| value as u64).collect(),
                deletion: vec![1, 2, 3, 4],
                outcome,
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
    fn full_zero_sum_operator_is_symmetric() {
        let problem = problem();
        let operator = TwoWayOperator::new(&problem).expect("operator");
        assert_eq!(operator.dimension(), problem.firms());
        let left = [0.3, -0.3];
        let right = [-1.7, 1.7];
        let mut a_right = [0.0; 2];
        let mut a_left = [0.0; 2];
        operator.apply(&right, &mut a_right).expect("action");
        operator.apply(&left, &mut a_left).expect("action");
        let difference = stable_dot(&left, &a_right) - stable_dot(&a_left, &right);
        assert!(difference.abs() < 1.0e-12);
    }

    #[test]
    fn known_coefficients_satisfy_full_system() {
        let problem = problem();
        let operator = TwoWayOperator::new(&problem).expect("operator");
        let (worker_rhs, firm_rhs) = operator.outcome_rhs().expect("rhs");
        let residual = operator
            .full_residual(&[1.0, -1.0], &[0.5, -0.5], &worker_rhs, &firm_rhs)
            .expect("residual");
        assert!(residual.relative_norm < 1.0e-14);
    }
}
