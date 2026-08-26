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
use crate::interrupt::{
    checkpoint_chunk, unstable_sort_by_with_interrupt, InterruptCheck, NeverInterrupt,
};
use crate::problem::{CompressedProblem, GroupIndex};
use crate::types::MAX_EXACT_BINARY64_INTEGER;

#[derive(Clone, Debug)]
pub struct DeletionSemanticPlan {
    pub cell: Vec<u32>,
    pub physical_count: Vec<u64>,
    pub outcome_sum: Vec<f64>,
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
        Self::build_no_controls_with_interrupt(problem, &mut NeverInterrupt)
    }

    pub fn build_no_controls_with_interrupt(
        problem: &CompressedProblem,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<Self> {
        interrupt.checkpoint("jla_plan_entry")?;
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

        let per_copy_mass = per_copy_target_mass(problem, interrupt)?;
        #[cfg(feature = "cmg-full-spike")]
        let raw_match = crate::full_cmg_spike::private_raw_match_requested()?;
        #[cfg(not(feature = "cmg-full-spike"))]
        let raw_match = false;
        let row_semantic_rank = if raw_match {
            semantic_row_ranks_by_certified_match(problem, &per_copy_mass, interrupt)?
        } else {
            semantic_row_ranks(problem, &per_copy_mass, interrupt)?
        };
        let deletion = deletion_plan(problem, &row_semantic_rank, interrupt)?;
        let target = if raw_match {
            target_plan_by_certified_match(problem, &per_copy_mass, &row_semantic_rank, interrupt)?
        } else {
            target_plan(problem, &per_copy_mass, &row_semantic_rank, interrupt)?
        };
        interrupt.checkpoint("jla_plan_final")?;
        Ok(Self {
            row_semantic_rank,
            deletion,
            target,
        })
    }

    /// Recheck the retained unit/stratum scatter identities used by the
    /// compressed no-control match estimator. Frequency and target mass are
    /// deliberately certified as separate measures.
    pub fn validate_against_problem(&self, problem: &CompressedProblem) -> Result<()> {
        let cells = problem.cells();
        let groups = problem.deletion_units();
        if self.deletion.cell.len() != groups
            || self.deletion.physical_count.len() != groups
            || self.deletion.outcome_sum.len() != groups
            || self.deletion.target_mass.len() != groups
            || self.deletion.semantic_rank.len() != groups
            || self.target.cell.len() != self.target.physical_count.len()
            || self.target.cell.len() != self.target.target_mass.len()
            || self.target.cell.len() != self.target.per_copy_mass.len()
            || self.target.cell.len() != self.target.semantic_rank.len()
        {
            return Err(BackendError::invariant(
                "jla_plan",
                "JLA scatter plan arrays have inconsistent dimensions",
            ));
        }

        let mut deletion_frequency = vec![0_u64; cells];
        let mut deletion_outcome = vec![StableSum::default(); cells];
        let mut deletion_outcome_abs = vec![StableSum::default(); cells];
        for group in 0..groups {
            let cell = usize::try_from(self.deletion.cell[group])
                .map_err(|_| resource_error("deletion cell is not addressable"))?;
            if cell >= cells || self.deletion.physical_count[group] == 0 {
                return Err(BackendError::invariant(
                    "jla_plan",
                    "deletion scatter plan contains an invalid cell or physical count",
                ));
            }
            deletion_frequency[cell] = deletion_frequency[cell]
                .checked_add(self.deletion.physical_count[group])
                .ok_or_else(|| resource_error("deletion-to-cell frequency overflow"))?;
            deletion_outcome[cell].add(self.deletion.outcome_sum[group]);
            deletion_outcome_abs[cell].add(self.deletion.outcome_sum[group].abs());
        }

        let mut target_frequency = vec![0_u64; cells];
        let mut target_mass = vec![StableSum::default(); cells];
        let mut target_mass_abs = vec![StableSum::default(); cells];
        for stratum in 0..self.target.cell.len() {
            let cell = usize::try_from(self.target.cell[stratum])
                .map_err(|_| resource_error("target-stratum cell is not addressable"))?;
            if cell >= cells || self.target.physical_count[stratum] == 0 {
                return Err(BackendError::invariant(
                    "jla_plan",
                    "target scatter plan contains an invalid cell or physical count",
                ));
            }
            target_frequency[cell] = target_frequency[cell]
                .checked_add(self.target.physical_count[stratum])
                .ok_or_else(|| resource_error("target-to-cell frequency overflow"))?;
            target_mass[cell].add(self.target.target_mass[stratum]);
            target_mass_abs[cell].add(self.target.target_mass[stratum].abs());
            let implied = self.target.per_copy_mass[stratum]
                * exact_frequency_as_f64(self.target.physical_count[stratum], stratum)?;
            if !aggregate_close(
                implied,
                self.target.target_mass[stratum],
                self.target.target_mass[stratum].abs(),
            ) {
                return Err(BackendError::invariant(
                    "jla_plan",
                    format!("target stratum {stratum} does not preserve per-copy target mass"),
                ));
            }
        }

        for cell in 0..cells {
            let expected_frequency = exact_cell_frequency(problem.cell_weight[cell], cell)?;
            if deletion_frequency[cell] != expected_frequency
                || target_frequency[cell] != expected_frequency
            {
                return Err(BackendError::invariant(
                    "jla_plan",
                    format!("deletion/target frequency scatters do not reproduce cell {cell}"),
                ));
            }
            if !aggregate_close(
                deletion_outcome[cell].finish(),
                problem.cell_outcome_sum[cell],
                deletion_outcome_abs[cell].finish(),
            ) {
                return Err(BackendError::invariant(
                    "jla_plan",
                    format!("deletion outcome scatter does not reproduce cell {cell}"),
                ));
            }
            if !aggregate_close(
                target_mass[cell].finish(),
                problem.cell_target_sum[cell],
                target_mass_abs[cell].finish(),
            ) {
                return Err(BackendError::invariant(
                    "jla_plan",
                    format!("target-mass scatter does not reproduce cell {cell}"),
                ));
            }
        }
        Ok(())
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
    let mut interrupt = NeverInterrupt;
    plugin_components_with_interrupt(
        problem,
        worker_coefficient,
        firm_coefficient,
        &mut interrupt,
    )
}

pub fn plugin_components_with_interrupt(
    problem: &CompressedProblem,
    worker_coefficient: &[f64],
    firm_coefficient: &[f64],
    interrupt: &mut dyn InterruptCheck,
) -> Result<VarianceComponents> {
    if worker_coefficient.len() != problem.workers() || firm_coefficient.len() != problem.firms() {
        return Err(BackendError::invalid(
            "jla_plugin",
            "effect coefficients have incompatible dimensions or nonfinite values",
        ));
    }
    for (index, &value) in worker_coefficient
        .iter()
        .chain(firm_coefficient)
        .enumerate()
    {
        checkpoint_chunk(interrupt, index, "jla_plugin_coefficients")?;
        if !value.is_finite() {
            return Err(BackendError::invalid(
                "jla_plugin",
                "effect coefficients have incompatible dimensions or nonfinite values",
            ));
        }
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
        checkpoint_chunk(interrupt, row, "jla_plugin_mean")?;
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
        checkpoint_chunk(interrupt, row, "jla_plugin_second")?;
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
    if worker_coefficient.len() != problem.workers() || firm_coefficient.len() != problem.firms() {
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

fn semantic_row_ranks(
    problem: &CompressedProblem,
    per_copy_mass: &[f64],
    interrupt: &mut dyn InterruptCheck,
) -> Result<Vec<u64>> {
    let rows = problem.outcome.len();
    let mut order = Vec::with_capacity(rows);
    for row in 0..rows {
        checkpoint_chunk(interrupt, row, "jla_plan_semantic_order")?;
        order.push(row);
    }
    unstable_sort_by_with_interrupt(
        &mut order,
        |&left, &right| {
            compare_semantic_rows(problem, per_copy_mass, left, right)
                .then_with(|| left.cmp(&right))
        },
        interrupt,
        "jla_plan_semantic_sort",
    )?;
    semantic_ranks_from_order(problem, per_copy_mass, order, interrupt)
}

fn semantic_row_ranks_by_certified_match(
    problem: &CompressedProblem,
    per_copy_mass: &[f64],
    interrupt: &mut dyn InterruptCheck,
) -> Result<Vec<u64>> {
    let order = certified_match_order(
        problem,
        |left, right| {
            compare_semantic_rows(problem, per_copy_mass, left, right)
                .then_with(|| left.cmp(&right))
        },
        interrupt,
        "jla_plan_raw_semantic_sort",
    )?;
    semantic_ranks_from_order(problem, per_copy_mass, order, interrupt)
}

fn semantic_ranks_from_order(
    problem: &CompressedProblem,
    per_copy_mass: &[f64],
    order: Vec<usize>,
    interrupt: &mut dyn InterruptCheck,
) -> Result<Vec<u64>> {
    let rows = problem.outcome.len();
    let mut rank = vec![0_u64; rows];
    let mut current = 0_u64;
    let mut previous = None;
    for (position, row) in order.into_iter().enumerate() {
        checkpoint_chunk(interrupt, position, "jla_plan_semantic_rank")?;
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
    let mut incomplete = current == 0;
    for (row, &value) in rank.iter().enumerate() {
        checkpoint_chunk(interrupt, row, "jla_plan_semantic_reconcile")?;
        incomplete |= value == 0;
    }
    if incomplete {
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
        .then_with(|| {
            problem
                .probe_order
                .as_ref()
                .map_or(Ordering::Equal, |key| ordered_f64(key[left], key[right]))
        })
}

fn certified_match_order<F>(
    problem: &CompressedProblem,
    mut compare: F,
    interrupt: &mut dyn InterruptCheck,
    phase: &'static str,
) -> Result<Vec<usize>>
where
    F: FnMut(usize, usize) -> Ordering,
{
    let groups = problem.deletion_units();
    if groups != problem.cells()
        || problem.deletion_index.ptr.len() != groups + 1
        || problem.deletion_index.items.len() != problem.outcome.len()
    {
        return Err(BackendError::invariant(
            "jla_plan",
            "the certified raw-match order requires one deletion unit per cell",
        ));
    }
    let mut order = Vec::with_capacity(problem.outcome.len());
    for (position, &row) in problem.deletion_index.items.iter().enumerate() {
        checkpoint_chunk(interrupt, position, phase)?;
        order.push(
            usize::try_from(row)
                .map_err(|_| resource_error("certified raw-match row is not addressable"))?,
        );
    }
    let mut previous_coordinate = None;
    for group in 0..groups {
        checkpoint_chunk(interrupt, group, phase)?;
        let coordinate = (problem.cell_worker[group], problem.cell_firm[group]);
        if previous_coordinate.is_some_and(|previous| previous >= coordinate) {
            return Err(BackendError::invariant(
                "jla_plan",
                "certified raw-match cells are not in worker-firm order",
            ));
        }
        previous_coordinate = Some(coordinate);
        let range = problem.deletion_index.range(group);
        if range.is_empty() {
            return Err(BackendError::invariant(
                "jla_plan",
                "certified raw-match deletion unit is empty",
            ));
        }
        let expected = u32::try_from(group)
            .map_err(|_| resource_error("certified raw-match group exceeds u32"))?;
        for (local, &row) in order[range.clone()].iter().enumerate() {
            checkpoint_chunk(interrupt, range.start + local, phase)?;
            if problem.row_deletion[row] != expected || problem.row_cell[row] != expected {
                return Err(BackendError::invariant(
                    "jla_plan",
                    "certified raw-match row does not reconcile with its cell",
                ));
            }
        }
        order[range.clone()].sort_unstable_by(|&left, &right| compare(left, right));
    }
    Ok(order)
}

fn deletion_plan(
    problem: &CompressedProblem,
    row_semantic_rank: &[u64],
    interrupt: &mut dyn InterruptCheck,
) -> Result<DeletionSemanticPlan> {
    let groups = problem.deletion_index.ptr.len() - 1;
    let mut cell = Vec::with_capacity(groups);
    let mut physical_count = Vec::with_capacity(groups);
    let mut outcome_sum = Vec::with_capacity(groups);
    let mut target_mass = Vec::with_capacity(groups);
    let mut semantic_rank = Vec::with_capacity(groups);
    for group in 0..groups {
        checkpoint_chunk(interrupt, group, "jla_plan_deletion_groups")?;
        let range = problem.deletion_index.range(group);
        if range.is_empty() {
            return Err(BackendError::invariant(
                "jla_plan",
                "deletion unit has no retained row",
            ));
        }
        let mut physical = 0_u64;
        let mut outcome = StableSum::default();
        let mut target = StableSum::default();
        let mut minimum_rank = u64::MAX;
        let mut group_cell = None;
        for (local, position) in range.enumerate() {
            checkpoint_chunk(interrupt, local, "jla_plan_deletion_rows")?;
            let row = usize::try_from(problem.deletion_index.items[position])
                .expect("validated deletion row");
            physical = physical
                .checked_add(problem.frequency[row])
                .ok_or_else(|| resource_error("deletion physical-count overflow"))?;
            let row_cell = problem.row_cell[row];
            match group_cell {
                None => group_cell = Some(row_cell),
                Some(previous) if previous == row_cell => {}
                Some(_) => {
                    return Err(BackendError::new(
                        ErrorCode::InvalidIdentifier,
                        "jla_plan",
                        "one deletion unit crosses coefficient cells",
                    ));
                }
            }
            outcome
                .add(exact_frequency_as_f64(problem.frequency[row], row)? * problem.outcome[row]);
            target.add(problem.target_weight[row]);
            minimum_rank = minimum_rank.min(row_semantic_rank[row]);
        }
        if physical == 0 || minimum_rank == u64::MAX || group_cell.is_none() {
            return Err(BackendError::invariant(
                "jla_plan",
                "deletion semantic plan is incomplete",
            ));
        }
        cell.push(group_cell.expect("checked deletion cell"));
        physical_count.push(physical);
        outcome_sum.push(outcome.finish());
        target_mass.push(target.finish());
        semantic_rank.push(minimum_rank);
    }
    if !all_unique(&semantic_rank, interrupt)? {
        return Err(BackendError::invariant(
            "jla_plan",
            "deletion-unit semantic ranks are not unique",
        ));
    }
    Ok(DeletionSemanticPlan {
        cell,
        physical_count,
        outcome_sum,
        target_mass,
        semantic_rank,
    })
}

#[allow(clippy::too_many_lines)]
fn target_plan(
    problem: &CompressedProblem,
    per_copy_mass: &[f64],
    row_semantic_rank: &[u64],
    interrupt: &mut dyn InterruptCheck,
) -> Result<TargetSemanticPlan> {
    let rows = problem.outcome.len();
    let mut order = Vec::with_capacity(rows);
    for row in 0..rows {
        checkpoint_chunk(interrupt, row, "jla_plan_target_order")?;
        order.push(row);
    }
    unstable_sort_by_with_interrupt(
        &mut order,
        |&left, &right| {
            problem.row_cell[left]
                .cmp(&problem.row_cell[right])
                .then_with(|| ordered_f64(per_copy_mass[left], per_copy_mass[right]))
                .then_with(|| row_semantic_rank[left].cmp(&row_semantic_rank[right]))
                .then_with(|| left.cmp(&right))
        },
        interrupt,
        "jla_plan_target_sort",
    )?;
    target_plan_from_order(problem, per_copy_mass, row_semantic_rank, order, interrupt)
}

fn target_plan_by_certified_match(
    problem: &CompressedProblem,
    per_copy_mass: &[f64],
    row_semantic_rank: &[u64],
    interrupt: &mut dyn InterruptCheck,
) -> Result<TargetSemanticPlan> {
    let order = certified_match_order(
        problem,
        |left, right| {
            problem.row_cell[left]
                .cmp(&problem.row_cell[right])
                .then_with(|| ordered_f64(per_copy_mass[left], per_copy_mass[right]))
                .then_with(|| row_semantic_rank[left].cmp(&row_semantic_rank[right]))
                .then_with(|| left.cmp(&right))
        },
        interrupt,
        "jla_plan_raw_target_sort",
    )?;
    target_plan_from_order(problem, per_copy_mass, row_semantic_rank, order, interrupt)
}

fn target_plan_from_order(
    problem: &CompressedProblem,
    per_copy_mass: &[f64],
    row_semantic_rank: &[u64],
    order: Vec<usize>,
    interrupt: &mut dyn InterruptCheck,
) -> Result<TargetSemanticPlan> {
    let rows = problem.outcome.len();
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
        checkpoint_chunk(interrupt, cursor, "jla_plan_target_groups")?;
        let first_row = order[cursor];
        let stratum_cell = problem.row_cell[first_row];
        let per_copy = canonical_zero(per_copy_mass[first_row]);
        let per_copy_bits = per_copy.to_bits();
        let begin = cursor;
        cursor += 1;
        while cursor < order.len() {
            checkpoint_chunk(interrupt, cursor - begin, "jla_plan_target_group_scan")?;
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
        for (local, &row) in order[begin..cursor].iter().enumerate() {
            checkpoint_chunk(interrupt, local, "jla_plan_target_rows")?;
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
        ptr.push(
            u64::try_from(items.len()).map_err(|_| {
                resource_error("target-stratum row count is not representable as u64")
            })?,
        );
    }
    let mut incomplete = false;
    for (row, &stratum) in row_to_stratum.iter().enumerate() {
        checkpoint_chunk(interrupt, row, "jla_plan_target_row_reconcile")?;
        incomplete |= stratum == u32::MAX;
    }
    for (stratum, (&physical, &rank)) in physical_count.iter().zip(&semantic_rank).enumerate() {
        checkpoint_chunk(interrupt, stratum, "jla_plan_target_reconcile")?;
        incomplete |= physical == 0 || rank == u64::MAX;
    }
    if incomplete || !all_unique(&semantic_rank, interrupt)? {
        return Err(BackendError::invariant(
            "jla_plan",
            "target semantic plan is incomplete or noncanonical",
        ));
    }
    let row_index = GroupIndex { ptr, items };
    row_index.validate_with_interrupt(cell.len(), rows, interrupt)?;
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

fn per_copy_target_mass(
    problem: &CompressedProblem,
    interrupt: &mut dyn InterruptCheck,
) -> Result<Vec<f64>> {
    let mut output = Vec::with_capacity(problem.target_weight.len());
    for (row, (&target, &frequency)) in problem
        .target_weight
        .iter()
        .zip(&problem.frequency)
        .enumerate()
    {
        checkpoint_chunk(interrupt, row, "jla_plan_per_copy_mass")?;
        let denominator = exact_frequency_as_f64(frequency, row)?;
        let value = canonical_zero(target / denominator);
        if !value.is_finite() || value < 0.0 {
            return Err(BackendError::new(
                ErrorCode::InvalidTargetWeight,
                "jla_plan",
                format!("per-copy target mass is invalid at zero-based row {row}"),
            ));
        } else {
            output.push(value);
        }
    }
    Ok(output)
}

#[allow(clippy::cast_precision_loss)]
fn exact_frequency_as_f64(frequency: u64, row: usize) -> Result<f64> {
    if frequency == 0 || frequency > MAX_EXACT_BINARY64_INTEGER {
        return Err(BackendError::new(
            ErrorCode::InvalidWeight,
            "jla_plan",
            format!("frequency at zero-based row {row} must be an exact positive binary64 integer"),
        ));
    }
    Ok(frequency as f64)
}

fn exact_cell_frequency(value: f64, cell: usize) -> Result<u64> {
    if !value.is_finite()
        || value <= 0.0
        || value.fract() != 0.0
        || value > MAX_EXACT_BINARY64_INTEGER as f64
    {
        return Err(BackendError::new(
            ErrorCode::InvalidWeight,
            "jla_plan",
            format!("cell {cell} frequency is not a positive exact binary64 integer"),
        ));
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    Ok(value as u64)
}

fn aggregate_close(reconstructed: f64, reference: f64, absolute_mass: f64) -> bool {
    if !reconstructed.is_finite() || !reference.is_finite() || !absolute_mass.is_finite() {
        return false;
    }
    let scale = reconstructed
        .abs()
        .max(reference.abs())
        .max(absolute_mass.abs())
        .max(1.0);
    (reconstructed - reference).abs() <= 4096.0 * f64::EPSILON * scale
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

fn all_unique(values: &[u64], interrupt: &mut dyn InterruptCheck) -> Result<bool> {
    let mut sorted = Vec::with_capacity(values.len());
    for (index, &value) in values.iter().enumerate() {
        checkpoint_chunk(interrupt, index, "jla_plan_unique_copy")?;
        sorted.push(value);
    }
    unstable_sort_by_with_interrupt(&mut sorted, Ord::cmp, interrupt, "jla_plan_unique_sort")?;
    for (index, pair) in sorted.windows(2).enumerate() {
        checkpoint_chunk(interrupt, index, "jla_plan_unique_reconcile")?;
        if pair[0] == pair[1] {
            return Ok(false);
        }
    }
    Ok(true)
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
    fn optional_probe_order_only_refines_tied_semantic_rows() {
        let mut problem = problem_from_rows(
            vec![1, 1, 1, 2, 2],
            vec![1, 1, 2, 1, 2],
            vec![1, 1, 2, 3, 4],
            vec![3.0, 3.0, 1.0, -2.0, 5.0],
            vec![1, 1, 1, 1, 1],
            vec![1.0, 1.0, 1.0, 1.0, 1.0],
        );
        let tied = JlaPlan::build_no_controls(&problem).expect("tied plan");
        assert_eq!(tied.row_semantic_rank[0], tied.row_semantic_rank[1]);

        problem.probe_order = Some(vec![20.0, 10.0, 30.0, 40.0, 50.0]);
        let refined = JlaPlan::build_no_controls(&problem).expect("refined plan");
        assert_ne!(refined.row_semantic_rank[0], refined.row_semantic_rank[1]);
        assert!(refined.row_semantic_rank[1] < refined.row_semantic_rank[0]);
        assert_eq!(tied.target.physical_count, refined.target.physical_count);
        assert_eq!(tied.target.target_mass, refined.target.target_mass);
    }

    #[test]
    fn certified_match_orders_reproduce_the_global_plan() {
        let mut problem = problem_from_rows(
            vec![1, 1, 1, 1, 2, 2, 2],
            vec![1, 1, 2, 2, 1, 1, 2],
            vec![1, 1, 2, 2, 3, 3, 4],
            vec![3.0, -4.0, 8.0, 1.0, -2.0, 5.0, 7.0],
            vec![2, 1, 4, 1, 1, 2, 1],
            vec![2.0, 1.0, 8.0, 1.0, 1.0, 2.0, 1.0],
        );
        problem.probe_order = Some(vec![7.0, 1.0, 6.0, 2.0, 5.0, 3.0, 4.0]);
        let per_copy =
            per_copy_target_mass(&problem, &mut NeverInterrupt).expect("per-copy target mass");
        let global_rank = semantic_row_ranks(&problem, &per_copy, &mut NeverInterrupt)
            .expect("global semantic ranks");
        let grouped_rank =
            semantic_row_ranks_by_certified_match(&problem, &per_copy, &mut NeverInterrupt)
                .expect("grouped semantic ranks");
        assert_eq!(grouped_rank, global_rank);

        let global = target_plan(&problem, &per_copy, &global_rank, &mut NeverInterrupt)
            .expect("global target plan");
        let grouped =
            target_plan_by_certified_match(&problem, &per_copy, &grouped_rank, &mut NeverInterrupt)
                .expect("grouped target plan");
        assert_eq!(grouped.cell, global.cell);
        assert_eq!(grouped.per_copy_mass, global.per_copy_mass);
        assert_eq!(grouped.physical_count, global.physical_count);
        assert_eq!(grouped.target_mass, global.target_mass);
        assert_eq!(grouped.semantic_rank, global.semantic_rank);
        assert_eq!(grouped.row_to_stratum, global.row_to_stratum);
        assert_eq!(grouped.row_index, global.row_index);
    }

    #[test]
    fn plugin_components_satisfy_accounting() {
        let problem = fixture();
        let components =
            plugin_components(&problem, &[1.0, -0.5], &[0.25, -0.75]).expect("plugin components");
        components.verify_accounting(1.0e-13).expect("accounting");
        assert!(
            (components.total - components.worker - components.firm - 2.0 * components.covariance)
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
