// SPDX-License-Identifier: GPL-3.0-only

//! Deterministic no-control JLA preparation and point-estimate components.
//!
//! The semantic contract follows the current compressed Mata route:
//! row ranks are ordered by worker, firm, deletion unit, per-copy target mass,
//! and outcome; target strata are grouped only by coefficient cell and exact
//! per-copy target mass. Deletion identity and outcome deliberately do not
//! split a target stratum.

use core::cmp::Ordering;

use crate::error::{BackendError, ErrorCode, Result};
use crate::problem::{CompressedProblem, GroupIndex};
use crate::types::MAX_EXACT_BINARY64_INTEGER;

#[derive(Clone, Debug)]
pub struct DeletionSemanticPlan {
    pub physical_count: Vec<u64>,
    pub target_mass: Vec<f64>,
    pub semantic_rank: Vec<u64>,
}

#[derive(Clone, Debug)]
pub struct TargetSemanticPlan {
    pub cell: Vec<u32>,
    pub per_copy_mass: Vec<f64>,
    pub physical_count: Vec<u64>,
    pub target_mass: Vec<f64>,
    pub semantic_rank: Vec<u64>,
    pub row_to_stratum: Vec<u32>,
    pub row_index: GroupIndex,
}

#[derive(Clone, Debug)]
pub struct JlaPlan {
    pub row_semantic_rank: Vec<u64>,
    pub deletion: DeletionSemanticPlan,
    pub target: TargetSemanticPlan,
}

impl JlaPlan {
    pub fn build_no_controls(problem: &CompressedProblem) -> Result<Self> {
        if !problem.controls.is_empty() {
            return Err(BackendError::new(
                ErrorCode::UnsupportedFeature,
                "jla_plan",
                "the current Rust JLA plan supports the no-control route only",
            ));
        }
        let rows = problem.outcome.len();
        if rows == 0
            || problem.row_worker.len() != rows
            || problem.row_firm.len() != rows
            || problem.row_deletion.len() != rows
            || problem.row_cell.len() != rows
            || problem.frequency.len() != rows
            || problem.target_weight.len() != rows
        {
            return Err(BackendError::invariant(
                "jla_plan",
                "compressed row arrays have inconsistent dimensions",
            ));
        }

        let per_copy_mass = per_copy_target_mass(problem)?;
        let row_semantic_rank = semantic_row_ranks(problem, &per_copy_mass)?;
        let deletion = deletion_plan(problem, &row_semantic_rank)?;
        let target = target_plan(problem, &per_copy_mass, &row_semantic_rank)?;
        Ok(Self {
            row_semantic_rank,
            deletion,
            target,
        })
    }

    #[must_use]
    pub fn deletion_units(&self) -> usize {
        self.deletion.physical_count.len()
    }

    #[must_use]
    pub fn target_strata(&self) -> usize {
        self.target.cell.len()
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct VarianceComponents {
    pub worker: f64,
    pub firm: f64,
    pub covariance: f64,
    pub total: f64,
}

impl VarianceComponents {
    pub fn verify_accounting(self, tolerance: f64) -> Result<()> {
        if !tolerance.is_finite() || tolerance <= 0.0 {
            return Err(BackendError::invalid(
                "jla_components",
                "accounting tolerance must be positive and finite",
            ));
        }
        if [self.worker, self.firm, self.covariance, self.total]
            .iter()
            .any(|value| !value.is_finite())
        {
            return Err(BackendError::new(
                ErrorCode::CorrectionNonFinite,
                "jla_components",
                "variance component is nonfinite",
            ));
        }
        let implied = self.worker + self.firm + 2.0 * self.covariance;
        let scale = self
            .worker
            .abs()
            .max(self.firm.abs())
            .max((2.0 * self.covariance).abs())
            .max(self.total.abs())
            .max(1.0);
        if (self.total - implied).abs() > tolerance * scale {
            return Err(BackendError::new(
                ErrorCode::AccountingIdentityFailed,
                "jla_components",
                format!(
                    "total component {} differs from worker + firm + 2 covariance {}",
                    self.total, implied
                ),
            ));
        }
        Ok(())
    }
}

pub fn plugin_components(
    problem: &CompressedProblem,
    worker_coefficient: &[f64],
    firm_coefficient: &[f64],
) -> Result<VarianceComponents> {
    if worker_coefficient.len() != problem.workers()
        || firm_coefficient.len() != problem.firms()
        || worker_coefficient.iter().any(|value| !value.is_finite())
        || firm_coefficient.iter().any(|value| !value.is_finite())
    {
        return Err(BackendError::invalid(
            "jla_plugin",
            "effect coefficients have incompatible dimensions or nonfinite values",
        ));
    }
    if !problem.target_total.is_finite() || problem.target_total <= 0.0 {
        return Err(BackendError::new(
            ErrorCode::InvalidTargetWeight,
            "jla_plugin",
            "target mass is not positive and finite",
        ));
    }

    let mut worker_mean = StableSum::default();
    let mut firm_mean = StableSum::default();
    for row in 0..problem.outcome.len() {
        let weight = problem.target_weight[row];
        let worker = usize::try_from(problem.row_worker[row]).expect("validated worker");
        let firm = usize::try_from(problem.row_firm[row]).expect("validated firm");
        worker_mean.add(weight * worker_coefficient[worker]);
        firm_mean.add(weight * firm_coefficient[firm]);
    }
    let worker_mean = worker_mean.finish() / problem.target_total;
    let firm_mean = firm_mean.finish() / problem.target_total;
    if !worker_mean.is_finite() || !firm_mean.is_finite() {
        return Err(BackendError::new(
            ErrorCode::CorrectionNonFinite,
            "jla_plugin",
            "centered effect mean is nonfinite",
        ));
    }

    let mut worker_second = StableSum::default();
    let mut firm_second = StableSum::default();
    let mut covariance = StableSum::default();
    let mut total_second = StableSum::default();
    for row in 0..problem.outcome.len() {
        let weight = problem.target_weight[row];
        let worker = usize::try_from(problem.row_worker[row]).expect("validated worker");
        let firm = usize::try_from(problem.row_firm[row]).expect("validated firm");
        let centered_worker = worker_coefficient[worker] - worker_mean;
        let centered_firm = firm_coefficient[firm] - firm_mean;
        worker_second.add(weight * centered_worker * centered_worker);
        firm_second.add(weight * centered_firm * centered_firm);
        covariance.add(weight * centered_worker * centered_firm);
        let centered_total = centered_worker + centered_firm;
        total_second.add(weight * centered_total * centered_total);
    }
    let result = VarianceComponents {
        worker: worker_second.finish() / problem.target_total,
        firm: firm_second.finish() / problem.target_total,
        covariance: covariance.finish() / problem.target_total,
        total: total_second.finish() / problem.target_total,
    };
    result.verify_accounting(1.0e-11)?;
    Ok(result)
}

pub fn fitted_values(
    problem: &CompressedProblem,
    worker_coefficient: &[f64],
    firm_coefficient: &[f64],
) -> Result<Vec<f64>> {
    if worker_coefficient.len() != problem.workers()
        || firm_coefficient.len() != problem.firms()
    {
        return Err(BackendError::invalid(
            "jla_fit",
            "effect coefficients have incompatible dimensions",
        ));
    }
    let mut fitted = Vec::with_capacity(problem.outcome.len());
    for row in 0..problem.outcome.len() {
        let worker = usize::try_from(problem.row_worker[row]).expect("validated worker");
        let firm = usize::try_from(problem.row_firm[row]).expect("validated firm");
        let value = worker_coefficient[worker] + firm_coefficient[firm];
        if !value.is_finite() {
            return Err(BackendError::new(
                ErrorCode::CorrectionNonFinite,
                "jla_fit",
                "fitted value is nonfinite",
            ));
        }
        fitted.push(value);
    }
    Ok(fitted)
}

pub fn residual_values(
    problem: &CompressedProblem,
    worker_coefficient: &[f64],
    firm_coefficient: &[f64],
) -> Result<Vec<f64>> {
    let mut fitted = fitted_values(problem, worker_coefficient, firm_coefficient)?;
    for (value, &outcome) in fitted.iter_mut().zip(&problem.outcome) {
        *value = outcome - *value;
        if !value.is_finite() {
            return Err(BackendError::new(
                ErrorCode::CorrectionNonFinite,
                "jla_fit",
                "residual is nonfinite",
            ));
        }
    }
    Ok(fitted)
}

fn semantic_row_ranks(problem: &CompressedProblem, per_copy_mass: &[f64]) -> Result<Vec<u64>> {
    let rows = problem.outcome.len();
    let mut order = (0..rows).collect::<Vec<_>>();
    order.sort_unstable_by(|&left, &right| {
        compare_semantic_rows(problem, per_copy_mass, left, right).then_with(|| left.cmp(&right))
    });
    let mut rank = vec![0_u64; rows];
    let mut current = 0_u64;
    let mut previous = None;
    for row in order {
        let starts_group = match previous {
            None => true,
            Some(prior) => {
                compare_semantic_rows(problem, per_copy_mass, prior, row) != Ordering::Equal
            }
        };
        if starts_group {
            current = current
                .checked_add(1)
                .ok_or_else(|| resource_error("semantic row-rank overflow"))?;
        }
        rank[row] = current;
        previous = Some(row);
    }
    if current == 0 || rank.contains(&0) {
        return Err(BackendError::invariant(
            "jla_plan",
            "semantic row ranks are incomplete",
        ));
    }
    Ok(rank)
}

fn compare_semantic_rows(
    problem: &CompressedProblem,
    per_copy_mass: &[f64],
    left: usize,
    right: usize,
) -> Ordering {
    problem.row_worker[left]
        .cmp(&problem.row_worker[right])
        .then_with(|| problem.row_firm[left].cmp(&problem.row_firm[right]))
        .then_with(|| problem.row_deletion[left].cmp(&problem.row_deletion[right]))
        .then_with(|| ordered_f64(per_copy_mass[left], per_copy_mass[right]))
        .then_with(|| ordered_f64(problem.outcome[left], problem.outcome[right]))
}

fn deletion_plan(
    problem: &CompressedProblem,
    row_semantic_rank: &[u64],
) -> Result<DeletionSemanticPlan> {
    let groups = problem.deletion_index.ptr.len() - 1;
    let mut physical_count = Vec::with_capacity(groups);
    let mut target_mass = Vec::with_capacity(groups);
    let mut semantic_rank = Vec::with_capacity(groups);
    for group in 0..groups {
        let range = problem.deletion_index.range(group);
        if range.is_empty() {
            return Err(BackendError::invariant(
                "jla_plan",
                "deletion unit has no retained row",
            ));
        }
        let mut physical = 0_u64;
        let mut target = StableSum::default();
        let mut minimum_rank = u64::MAX;
        for position in range {
            let row = usize::try_from(problem.deletion_index.items[position])
                .expect("validated deletion row");
            physical = physical
                .checked_add(problem.frequency[row])
                .ok_or_else(|| resource_error("deletion physical-count overflow"))?;
            target.add(problem.target_weight[row]);
            minimum_rank = minimum_rank.min(row_semantic_rank[row]);
        }
        if physical == 0 || minimum_rank == u64::MAX {
            return Err(BackendError::invariant(
                "jla_plan",
                "deletion semantic plan is incomplete",
            ));
        }
        physical_count.push(physical);
        target_mass.push(target.finish());
        semantic_rank.push(minimum_rank);
    }
    if !all_unique(&semantic_rank) {
        return Err(BackendError::invariant(
            "jla_plan",
            "deletion-unit semantic ranks are not unique",
        ));
    }
    Ok(DeletionSemanticPlan {
        physical_count,
        target_mass,
        semantic_rank,
    })
}

#[allow(clippy::too_many_lines)]
fn target_plan(
    problem: &CompressedProblem,
    per_copy_mass: &[f64],
    row_semantic_rank: &[u64],
) -> Result<TargetSemanticPlan> {
    let rows = problem.outcome.len();
    let mut order = (0..rows).collect::<Vec<_>>();
    order.sort_unstable_by(|&left, &right| {
        problem.row_cell[left]
            .cmp(&problem.row_cell[right])
            .then_with(|| ordered_f64(per_copy_mass[left], per_copy_mass[right]))
            .then_with(|| row_semantic_rank[left].cmp(&row_semantic_rank[right]))
            .then_with(|| left.cmp(&right))
    });

    let mut cell = Vec::<u32>::new();
    let mut stratum_mass = Vec::<f64>::new();
    let mut physical_count = Vec::<u64>::new();
    let mut target_mass = Vec::<f64>::new();
    let mut semantic_rank = Vec::<u64>::new();
    let mut row_to_stratum = vec![u32::MAX; rows];
    let mut ptr = vec![0_u64];
    let mut items = Vec::<u32>::with_capacity(rows);

    let mut cursor = 0_usize;
    while cursor < order.len() {
        let first_row = order[cursor];
        let stratum_cell = problem.row_cell[first_row];
        let per_copy = canonical_zero(per_copy_mass[first_row]);
        let per_copy_bits = per_copy.to_bits();
        let begin = cursor;
        cursor += 1;
        while cursor < order.len() {
            let row = order[cursor];
            if problem.row_cell[row] != stratum_cell
                || canonical_zero(per_copy_mass[row]).to_bits() != per_copy_bits
            {
                break;
            }
            cursor += 1;
        }
        let stratum = u32::try_from(cell.len()).map_err(|_| {
            resource_error("target-stratum count exceeds the u32 implementation limit")
        })?;
        let mut physical = 0_u64;
        let mut target = StableSum::default();
        let mut minimum_rank = u64::MAX;
        for &row in &order[begin..cursor] {
            physical = physical
                .checked_add(problem.frequency[row])
                .ok_or_else(|| resource_error("target-stratum physical-count overflow"))?;
            target.add(problem.target_weight[row]);
            minimum_rank = minimum_rank.min(row_semantic_rank[row]);
            row_to_stratum[row] = stratum;
            items.push(u32::try_from(row).map_err(|_| {
                resource_error("retained row exceeds the u32 implementation limit")
            })?);
        }
        cell.push(stratum_cell);
        stratum_mass.push(per_copy);
        physical_count.push(physical);
        target_mass.push(target.finish());
        semantic_rank.push(minimum_rank);
        ptr.push(u64::try_from(items.len()).map_err(|_| {
            resource_error("target-stratum row count is not representable as u64")
        })?);
    }
    if row_to_stratum.contains(&u32::MAX)
        || physical_count.contains(&0)
        || semantic_rank.contains(&u64::MAX)
        || !all_unique(&semantic_rank)
    {
        return Err(BackendError::invariant(
            "jla_plan",
            "target semantic plan is incomplete or noncanonical",
        ));
    }
    let row_index = GroupIndex { ptr, items };
    row_index.validate(cell.len(), rows)?;
    Ok(TargetSemanticPlan {
        cell,
        per_copy_mass: stratum_mass,
        physical_count,
        target_mass,
        semantic_rank,
        row_to_stratum,
        row_index,
    })
}

fn per_copy_target_mass(problem: &CompressedProblem) -> Result<Vec<f64>> {
    problem
        .target_weight
        .iter()
        .zip(&problem.frequency)
        .enumerate()
        .map(|(row, (&target, &frequency))| {
            let denominator = exact_frequency_as_f64(frequency, row)?;
            let value = canonical_zero(target / denominator);
            if !value.is_finite() || value < 0.0 {
                Err(BackendError::new(
                    ErrorCode::InvalidTargetWeight,
                    "jla_plan",
                    format!("per-copy target mass is invalid at zero-based row {row}"),
                ))
            } else {
                Ok(value)
            }
        })
        .collect()
}

#[allow(clippy::cast_precision_loss)]
fn exact_frequency_as_f64(frequency: u64, row: usize) -> Result<f64> {
    if frequency == 0 || frequency > MAX_EXACT_BINARY64_INTEGER {
        return Err(BackendError::new(
            ErrorCode::InvalidWeight,
            "jla_plan",
            format!(
                "frequency at zero-based row {row} must be an exact positive binary64 integer"
            ),
        ));
    }
    Ok(frequency as f64)
}

fn ordered_f64(left: f64, right: f64) -> Ordering {
    canonical_zero(left).total_cmp(&canonical_zero(right))
}

fn canonical_zero(value: f64) -> f64 {
    if value == 0.0 {
        0.0
    } else {
        value
    }
}

fn all_unique(values: &[u64]) -> bool {
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    sorted.windows(2).all(|pair| pair[0] != pair[1])
}

#[derive(Clone, Copy, Debug, Default)]
struct StableSum {
    sum: f64,
    correction: f64,
}

impl StableSum {
    fn add(&mut self, value: f64) {
        let updated = self.sum + value;
        self.correction += if self.sum.abs() >= value.abs() {
            (self.sum - updated) + value
        } else {
            (value - updated) + self.sum
        };
        self.sum = updated;
    }

    fn finish(self) -> f64 {
        self.sum + self.correction
    }
}

fn resource_error(message: &str) -> BackendError {
    BackendError::new(ErrorCode::ResourceLimit, "jla_plan", message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::problem::CanonicalInput;
    use crate::types::InputColumns;

    fn problem_from_rows(
        worker: Vec<u64>,
        firm: Vec<u64>,
        deletion: Vec<u64>,
        outcome: Vec<f64>,
        frequency: Vec<u64>,
        target_weight: Vec<f64>,
    ) -> CompressedProblem {
        let rows = worker.len();
        CanonicalInput::from_validated(
            InputColumns {
                worker,
                firm,
                deletion,
                outcome,
                frequency,
                target_weight,
                controls: Vec::new(),
            }
            .validate()
            .expect("fixture"),
        )
        .expect("canonical fixture")
        .compress(&vec![true; rows])
        .expect("compressed fixture")
    }

    fn fixture() -> CompressedProblem {
        problem_from_rows(
            vec![1, 1, 1, 1, 2, 2],
            vec![1, 1, 1, 2, 1, 2],
            vec![1, 2, 3, 4, 5, 6],
            vec![3.0, -4.0, 8.0, 1.0, -2.0, 5.0],
            vec![2, 1, 4, 1, 1, 1],
            vec![2.0, 1.0, 8.0, 1.0, 1.0, 1.0],
        )
    }

    #[test]
    fn target_strata_ignore_deletion_and_outcome() {
        let problem = fixture();
        let plan = JlaPlan::build_no_controls(&problem).expect("JLA plan");
        assert_eq!(plan.target_strata(), 5);
        assert_eq!(plan.target.row_to_stratum[0], plan.target.row_to_stratum[1]);
        assert_ne!(plan.target.row_to_stratum[0], plan.target.row_to_stratum[2]);
        let shared = usize::try_from(plan.target.row_to_stratum[0]).expect("stratum");
        assert_eq!(plan.target.physical_count[shared], 3);
        assert!((plan.target.target_mass[shared] - 3.0).abs() < 1.0e-15);
        assert!((plan.target.per_copy_mass[shared] - 1.0).abs() < 1.0e-15);
    }

    #[test]
    fn semantic_summary_is_row_permutation_invariant() {
        let original = fixture();
        let permutation = [5_usize, 1, 3, 0, 4, 2];
        let worker_values = [1_u64, 1, 1, 1, 2, 2];
        let firm_values = [1_u64, 1, 1, 2, 1, 2];
        let deletion_values = [1_u64, 2, 3, 4, 5, 6];
        let outcome_values = [3.0_f64, -4.0, 8.0, 1.0, -2.0, 5.0];
        let frequency_values = [2_u64, 1, 4, 1, 1, 1];
        let target_values = [2.0_f64, 1.0, 8.0, 1.0, 1.0, 1.0];
        let permuted = problem_from_rows(
            permutation.iter().map(|&row| worker_values[row]).collect(),
            permutation.iter().map(|&row| firm_values[row]).collect(),
            permutation
                .iter()
                .map(|&row| deletion_values[row])
                .collect(),
            permutation.iter().map(|&row| outcome_values[row]).collect(),
            permutation
                .iter()
                .map(|&row| frequency_values[row])
                .collect(),
            permutation.iter().map(|&row| target_values[row]).collect(),
        );
        let left = JlaPlan::build_no_controls(&original).expect("left plan");
        let right = JlaPlan::build_no_controls(&permuted).expect("right plan");
        let mut left_target = (0..left.target_strata())
            .map(|index| {
                (
                    left.target.cell[index],
                    left.target.per_copy_mass[index].to_bits(),
                    left.target.physical_count[index],
                    left.target.target_mass[index].to_bits(),
                    left.target.semantic_rank[index],
                )
            })
            .collect::<Vec<_>>();
        let mut right_target = (0..right.target_strata())
            .map(|index| {
                (
                    right.target.cell[index],
                    right.target.per_copy_mass[index].to_bits(),
                    right.target.physical_count[index],
                    right.target.target_mass[index].to_bits(),
                    right.target.semantic_rank[index],
                )
            })
            .collect::<Vec<_>>();
        left_target.sort_unstable();
        right_target.sort_unstable();
        assert_eq!(left_target, right_target);
        assert_eq!(left.deletion.semantic_rank, right.deletion.semantic_rank);
    }

    #[test]
    fn plugin_components_satisfy_accounting() {
        let problem = fixture();
        let components = plugin_components(&problem, &[1.0, -0.5], &[0.25, -0.75])
            .expect("plugin components");
        components.verify_accounting(1.0e-13).expect("accounting");
        assert!(
            (components.total
                - components.worker
                - components.firm
                - 2.0 * components.covariance)
                .abs()
                < 1.0e-13
        );
    }

    #[test]
    fn controls_are_explicitly_deferred() {
        let mut problem = fixture();
        problem.controls.push(vec![1.0; problem.outcome.len()]);
        let error = JlaPlan::build_no_controls(&problem).expect_err("controls must fail");
        assert_eq!(error.code, ErrorCode::UnsupportedFeature);
    }
}
