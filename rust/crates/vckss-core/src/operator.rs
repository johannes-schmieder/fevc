// SPDX-License-Identifier: GPL-3.0-only

use crate::error::{BackendError, ErrorCode, Result};
use crate::parallel::compensated_sum;
use crate::problem::CompressedProblem;

pub trait SymmetricOperator {
    fn dimension(&self) -> usize;
    fn apply(&self, input: &[f64], output: &mut [f64]) -> Result<()>;
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
        if worker_diagonal
            .iter()
            .chain(&firm_diagonal)
            .any(|&value| !value.is_finite() || value <= 0.0)
        {
            return Err(BackendError::new(
                ErrorCode::GraphUnidentified,
                "operator",
                "worker or firm information diagonal is not positive and finite",
            ));
        }

        let reduced_diagonal =
            reduced_diagonal(problem, &worker_diagonal, &firm_diagonal)?;
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

    pub fn expand_firm(&self, reduced: &[f64]) -> Result<Vec<f64>> {
        if reduced.len() != self.dimension() {
            return Err(BackendError::invalid(
                "operator",
                "reduced firm vector has the wrong dimension",
            ));
        }
        if reduced.iter().any(|value| !value.is_finite()) {
            return Err(BackendError::invalid(
                "operator",
                "reduced firm vector is nonfinite",
            ));
        }
        let mut firm = Vec::with_capacity(self.problem.firms());
        firm.extend_from_slice(reduced);
        firm.push(-compensated_sum(reduced));
        Ok(firm)
    }

    pub fn reduce_full_firm(&self, full: &[f64]) -> Result<Vec<f64>> {
        if full.len() != self.problem.firms() {
            return Err(BackendError::invalid(
                "operator",
                "full firm vector has the wrong dimension",
            ));
        }
        let last = full[full.len() - 1];
        if !last.is_finite() || full.iter().any(|value| !value.is_finite()) {
            return Err(BackendError::invalid(
                "operator",
                "full firm vector is nonfinite",
            ));
        }
        Ok(full[..full.len() - 1]
            .iter()
            .map(|&value| value - last)
            .collect())
    }

    pub fn apply_full_schur(&self, firm: &[f64], output: &mut [f64]) -> Result<()> {
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
            let worker = usize::try_from(self.problem.cell_worker[cell]).expect("dense worker");
            let firm_index = usize::try_from(self.problem.cell_firm[cell]).expect("dense firm");
            worker_sum[worker] += self.problem.cell_weight[cell] * firm[firm_index];
        }
        for (value, diagonal) in worker_sum.iter_mut().zip(&self.worker_diagonal) {
            *value /= diagonal;
        }
        for (value, (&diagonal, &coefficient)) in output
            .iter_mut()
            .zip(self.firm_diagonal.iter().zip(firm))
        {
            *value = diagonal * coefficient;
        }
        for cell in 0..self.problem.cells() {
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
            let worker = usize::try_from(self.problem.cell_worker[cell]).expect("dense worker");
            let firm = usize::try_from(self.problem.cell_firm[cell]).expect("dense firm");
            full[firm] -= self.problem.cell_weight[cell] * worker_scaled[worker];
        }
        self.reduce_full_firm(&full)
    }

    pub fn reconstruct_worker(&self, worker_rhs: &[f64], firm: &[f64]) -> Result<Vec<f64>> {
        if worker_rhs.len() != self.problem.workers() || firm.len() != self.problem.firms() {
            return Err(BackendError::invalid(
                "operator",
                "worker reconstruction has incompatible dimensions",
            ));
        }
        let mut worker = worker_rhs.to_vec();
        for cell in 0..self.problem.cells() {
            let worker_index =
                usize::try_from(self.problem.cell_worker[cell]).expect("dense worker");
            let firm_index = usize::try_from(self.problem.cell_firm[cell]).expect("dense firm");
            worker[worker_index] -= self.problem.cell_weight[cell] * firm[firm_index];
        }
        for (value, diagonal) in worker.iter_mut().zip(&self.worker_diagonal) {
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
        let mut worker = vec![0.0; self.problem.workers()];
        let mut firm = vec![0.0; self.problem.firms()];
        for cell in 0..self.problem.cells() {
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
        self.problem.firms() - 1
    }

    fn apply(&self, input: &[f64], output: &mut [f64]) -> Result<()> {
        if input.len() != self.dimension() || output.len() != self.dimension() {
            return Err(BackendError::invalid(
                "operator",
                "reduced Schur action has incompatible dimensions",
            ));
        }
        let firm = self.expand_firm(input)?;
        let mut full_output = vec![0.0; self.problem.firms()];
        self.apply_full_schur(&firm, &mut full_output)?;
        let reduced = self.reduce_full_firm(&full_output)?;
        output.copy_from_slice(&reduced);
        Ok(())
    }
}

fn reduced_diagonal(
    problem: &CompressedProblem,
    worker_diagonal: &[f64],
    firm_diagonal: &[f64],
) -> Result<Vec<f64>> {
    let firms = problem.firms();
    let last = firms - 1;
    let mut full_diagonal = firm_diagonal.to_vec();
    for cell in 0..problem.cells() {
        let worker = usize::try_from(problem.cell_worker[cell]).expect("dense worker");
        let firm = usize::try_from(problem.cell_firm[cell]).expect("dense firm");
        let weight = problem.cell_weight[cell];
        full_diagonal[firm] -= weight * weight / worker_diagonal[worker];
    }

    let mut cross_last = vec![0.0; last];
    for worker in 0..problem.workers() {
        let range = problem.worker_index.range(worker);
        let mut last_weight = 0.0;
        for position in range.clone() {
            let cell = usize::try_from(problem.worker_index.items[position]).expect("cell");
            if usize::try_from(problem.cell_firm[cell]).expect("firm") == last {
                last_weight = problem.cell_weight[cell];
                break;
            }
        }
        if last_weight == 0.0 {
            continue;
        }
        for position in range {
            let cell = usize::try_from(problem.worker_index.items[position]).expect("cell");
            let firm = usize::try_from(problem.cell_firm[cell]).expect("firm");
            if firm != last {
                cross_last[firm] -=
                    problem.cell_weight[cell] * last_weight / worker_diagonal[worker];
            }
        }
    }

    let reduced: Vec<f64> = (0..last)
        .map(|firm| full_diagonal[firm] + full_diagonal[last] - 2.0 * cross_last[firm])
        .collect();
    if reduced.iter().any(|&value| !value.is_finite() || value <= 0.0) {
        return Err(BackendError::new(
            ErrorCode::GraphUnidentified,
            "operator",
            "reduced Schur diagonal is not positive and finite",
        ));
    }
    Ok(reduced)
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
    fn reduced_operator_is_symmetric() {
        let problem = problem();
        let operator = TwoWayOperator::new(&problem).expect("operator");
        let left = [0.3];
        let right = [-1.7];
        let mut a_right = [0.0];
        let mut a_left = [0.0];
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
