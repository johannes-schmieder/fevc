// SPDX-License-Identifier: GPL-3.0-only

use core::cmp::Ordering;
use std::collections::BTreeMap;

use crate::error::{BackendError, ErrorCode, Result};
use crate::parallel::compensated_sum;
use crate::types::{Dimensions, ValidatedInput};

const MISSING_ID: u32 = u32::MAX;
const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GroupIndex {
    pub ptr: Vec<u64>,
    pub items: Vec<u32>,
}

impl GroupIndex {
    pub fn validate(&self, groups: usize, upper_item: usize) -> Result<()> {
        if self.ptr.len() != groups + 1 || self.ptr.first() != Some(&0) {
            return Err(BackendError::invariant(
                "compression",
                "group pointer has invalid dimensions",
            ));
        }
        let terminal = u64::try_from(self.items.len()).map_err(|_| {
            BackendError::new(
                ErrorCode::ResourceLimit,
                "compression",
                "group item count is not representable",
            )
        })?;
        if self.ptr.last() != Some(&terminal)
            || self.ptr.windows(2).any(|pair| pair[0] > pair[1])
            || self
                .items
                .iter()
                .any(|&item| usize::try_from(item).map_or(true, |value| value >= upper_item))
        {
            return Err(BackendError::invariant(
                "compression",
                "group index is not a valid CSR mapping",
            ));
        }
        Ok(())
    }

    #[must_use]
    pub fn range(&self, group: usize) -> core::ops::Range<usize> {
        let begin = usize::try_from(self.ptr[group]).expect("validated CSR offset");
        let end = usize::try_from(self.ptr[group + 1]).expect("validated CSR offset");
        begin..end
    }
}

#[derive(Clone, Debug)]
pub struct CanonicalInput {
    pub worker: Vec<u32>,
    pub firm: Vec<u32>,
    pub deletion: Vec<u32>,
    pub worker_levels: Vec<u64>,
    pub firm_levels: Vec<u64>,
    pub deletion_levels: Vec<u64>,
    pub outcome: Vec<f64>,
    pub frequency: Vec<u64>,
    pub target_weight: Vec<f64>,
    pub controls: Vec<Vec<f64>>,
    pub physical_total: u64,
    pub target_total: f64,
    pub topology_checksum: u64,
}

impl CanonicalInput {
    pub fn from_validated(input: ValidatedInput) -> Result<Self> {
        let (worker, worker_levels) = redense(&input.columns.worker, "worker")?;
        let (firm, firm_levels) = redense(&input.columns.firm, "firm")?;
        let (deletion, deletion_levels) = redense(&input.columns.deletion, "deletion")?;

        let mut coordinate = vec![None; deletion_levels.len()];
        for row in 0..worker.len() {
            let deletion_index = usize::try_from(deletion[row]).map_err(|_| {
                BackendError::new(
                    ErrorCode::InvalidIdentifier,
                    "canonicalize",
                    "deletion identifier is not addressable",
                )
            })?;
            let pair = (worker[row], firm[row]);
            match coordinate[deletion_index] {
                None => coordinate[deletion_index] = Some(pair),
                Some(previous) if previous == pair => {}
                Some(_) => {
                    return Err(BackendError::new(
                        ErrorCode::InvalidIdentifier,
                        "canonicalize",
                        "each deletion identifier must stay within one worker-firm coordinate",
                    ));
                }
            }
        }

        let topology_checksum =
            topology_checksum(&worker, &firm, &deletion, &input.columns.frequency);
        Ok(Self {
            worker,
            firm,
            deletion,
            worker_levels,
            firm_levels,
            deletion_levels,
            outcome: input.columns.outcome,
            frequency: input.columns.frequency,
            target_weight: input.columns.target_weight,
            controls: input.columns.controls,
            physical_total: input.physical_total,
            target_total: input.target_total,
            topology_checksum,
        })
    }

    #[must_use]
    pub fn rows(&self) -> usize {
        self.worker.len()
    }

    #[must_use]
    pub fn workers(&self) -> usize {
        self.worker_levels.len()
    }

    #[must_use]
    pub fn firms(&self) -> usize {
        self.firm_levels.len()
    }

    #[must_use]
    pub fn deletion_units(&self) -> usize {
        self.deletion_levels.len()
    }

    pub fn compress(&self, active: &[bool]) -> Result<CompressedProblem> {
        if active.len() != self.rows() {
            return Err(BackendError::invalid(
                "compression",
                "active mask has the wrong length",
            ));
        }
        let retained_rows: Vec<usize> = active
            .iter()
            .enumerate()
            .filter_map(|(row, &keep)| keep.then_some(row))
            .collect();
        if retained_rows.is_empty() {
            return Err(BackendError::new(
                ErrorCode::GraphEmpty,
                "compression",
                "no rows remain after graph selection",
            ));
        }

        let worker_map = redense_selected(&self.worker, &retained_rows, self.workers(), "worker")?;
        let firm_map = redense_selected(&self.firm, &retained_rows, self.firms(), "firm")?;
        let deletion_map = redense_selected(
            &self.deletion,
            &retained_rows,
            self.deletion_units(),
            "deletion",
        )?;

        let row_worker: Vec<u32> = retained_rows
            .iter()
            .map(|&row| worker_map[usize::try_from(self.worker[row]).expect("dense worker")])
            .collect();
        let row_firm: Vec<u32> = retained_rows
            .iter()
            .map(|&row| firm_map[usize::try_from(self.firm[row]).expect("dense firm")])
            .collect();
        let row_deletion: Vec<u32> = retained_rows
            .iter()
            .map(|&row| deletion_map[usize::try_from(self.deletion[row]).expect("dense deletion")])
            .collect();

        let workers = count_levels(&worker_map);
        let firms = count_levels(&firm_map);
        let deletion_units = count_levels(&deletion_map);
        if firms < 2 {
            return Err(BackendError::new(
                ErrorCode::GraphUnidentified,
                "compression",
                "retained graph must contain at least two firms",
            ));
        }

        let mut cell_order: Vec<usize> = (0..retained_rows.len()).collect();
        cell_order.sort_by(|&left, &right| {
            row_worker[left]
                .cmp(&row_worker[right])
                .then_with(|| row_firm[left].cmp(&row_firm[right]))
                .then_with(|| retained_rows[left].cmp(&retained_rows[right]))
        });

        let mut cell_worker = Vec::new();
        let mut cell_firm = Vec::new();
        let mut cell_weight = Vec::new();
        let mut cell_outcome_sum = Vec::new();
        let mut cell_target_sum = Vec::new();
        let mut row_cell = vec![MISSING_ID; retained_rows.len()];

        let mut cursor = 0;
        while cursor < cell_order.len() {
            let first = cell_order[cursor];
            let worker = row_worker[first];
            let firm = row_firm[first];
            let begin = cursor;
            cursor += 1;
            while cursor < cell_order.len()
                && row_worker[cell_order[cursor]] == worker
                && row_firm[cell_order[cursor]] == firm
            {
                cursor += 1;
            }
            let cell = u32::try_from(cell_worker.len()).map_err(|_| {
                BackendError::new(
                    ErrorCode::ResourceLimit,
                    "compression",
                    "coefficient-cell count exceeds the u32 implementation limit",
                )
            })?;
            let mut weights = Vec::with_capacity(cursor - begin);
            let mut outcomes = Vec::with_capacity(cursor - begin);
            let mut targets = Vec::with_capacity(cursor - begin);
            for &local_row in &cell_order[begin..cursor] {
                let source_row = retained_rows[local_row];
                let weight = self.frequency[source_row] as f64;
                let outcome = weight * self.outcome[source_row];
                if !weight.is_finite() || !outcome.is_finite() {
                    return Err(BackendError::new(
                        ErrorCode::InvalidWeight,
                        "compression",
                        "weighted cell moment is nonfinite",
                    ));
                }
                weights.push(weight);
                outcomes.push(outcome);
                targets.push(self.target_weight[source_row]);
                row_cell[local_row] = cell;
            }
            cell_worker.push(worker);
            cell_firm.push(firm);
            cell_weight.push(compensated_sum(&weights));
            cell_outcome_sum.push(compensated_sum(&outcomes));
            cell_target_sum.push(compensated_sum(&targets));
        }
        if row_cell.contains(&MISSING_ID) {
            return Err(BackendError::invariant(
                "compression",
                "not every retained row was assigned to a coefficient cell",
            ));
        }

        let worker_index = grouped_items(workers, &cell_worker)?;
        let mut firm_order: Vec<u32> = (0..cell_worker.len())
            .map(|cell| {
                u32::try_from(cell).map_err(|_| {
                    BackendError::new(
                        ErrorCode::ResourceLimit,
                        "compression",
                        "cell index exceeds the u32 implementation limit",
                     )
                })
            })
            .collect::<Result<_>>()?;
        firm_order.sort_by_key(|&cell| {
            let cell_index = usize::try_from(cell).expect("u32 cell index");
            (cell_firm[cell_index], cell_worker[cell_index], cell)
        });
        let firm_index = grouped_items_from_order(firms, &cell_firm, firm_order)?;

        let deletion_index = grouped_rows(deletion_units, &row_deletion)?;
        let (target_id, target_index) =
            exact_target_strata(self, &retained_rows, &row_worker, &row_firm, &row_deletion)?;
        let target_strata = target_index.ptr.len() - 1;

        let physical_total = retained_rows.iter().try_fold(0_u64, |total, &row| {
            total.checked_add(self.frequency[row]).ok_or_else(|| {
                BackendError::new(
                    ErrorCode::ResourceLimit,
                    "compression",
                    "retained physical-frequency total overflow",
                )
            })
        })?;
        let retained_targets: Vec<f64> = retained_rows
            .iter()
            .map(|&row| self.target_weight[row])
            .collect();
        let target_total = compensated_sum(&retained_targets);
        if !target_total.is_finite() || target_total <= 0.0 {
            return Err(BackendError::new(
                ErrorCode::InvalidTargetWeight,
                "compression",
                "retained target mass is not positive and finite",
            ));
        }

        let rows_stored = u64::try_from(retained_rows.len()).map_err(|_| {
            BackendError::new(
                ErrorCode::ResourceLimit,
                "compression",
                "retained row count is not representable",
            )
        })?;
        let dimensions = Dimensions {
            rows_stored,
            rows_physical: physical_total,
            workers: u64::try_from(workers).map_err(|_| {
                BackendError::new(
                    ErrorCode::ResourceLimit,
                    "compression",
                    "worker count overflow",
                )
            })?,
            firms: u64::try_from(firms).map_err(|_| {
                BackendError::new(
                    ErrorCode::ResourceLimit,
                    "compression",
                    "firm count overflow",
                )
            })?,
            cells: u64::try_from(cell_worker.len()).map_err(|_| {
                BackendError::new(
                    ErrorCode::ResourceLimit,
                    "compression",
                    "cell count overflow",
                )
            })?,
            deletion_units: u64::try_from(deletion_units).map_err(|_| {
                BackendError::new(
                    ErrorCode::ResourceLimit,
                    "compression",
                    "deletion-unit count overflow",
                )
            })?,
            target_strata: u64::try_from(target_strata).map_err(|_| {
                BackendError::new(
                    ErrorCode::ResourceLimit,
                    "compression",
                    "target-stratum count overflow",
                )
            })?,
            controls: u32::try_from(self.controls.len()).map_err(|_| {
                BackendError::new(
                    ErrorCode::ResourceLimit,
                    "compression",
                    "control count exceeds u32",
                )
            })?,
        };

        worker_index.validate(workers, cell_worker.len())?;
        firm_index.validate(firms, cell_worker.len())?;
        deletion_index.validate(deletion_units, retained_rows.len())?;
        target_index.validate(target_strata, retained_rows.len())?;

        let outcome = retained_rows.iter().map(|&row| self.outcome[row]).collect();
        let frequency: Vec<u64> = retained_rows
            .iter()
            .map(|&row| self.frequency[row])
            .collect();
        let target_weight = retained_rows
            .iter()
            .map(|&row| self.target_weight[row])
            .collect();
        let controls = self
            .controls
            .iter()
            .map(|column| retained_rows.iter().map(|&row| column[row]).collect())
            .collect();
        let topology_checksum =
            topology_checksum(&row_worker, &row_firm, &row_deletion, &frequency);

        Ok(CompressedProblem {
            dimensions,
            retained_rows,
            row_worker,
            row_firm,
            row_deletion,
            row_cell,
            row_target: target_id,
            outcome,
            frequency,
            target_weight,
            controls,
            cell_worker,
            cell_firm,
            cell_weight,
            cell_outcome_sum,
            cell_target_sum,
            worker_index,
            firm_index,
            deletion_index,
            target_index,
            physical_total,
            target_total,
            topology_checksum,
        })
    }
}

#[derive(Clone, Debug)]
pub struct CompressedProblem {
    pub dimensions: Dimensions,
    pub retained_rows: Vec<usize>,
    pub row_worker: Vec<u32>,
    pub row_firm: Vec<u32>,
    pub row_deletion: Vec<u32>,
    pub row_cell: Vec<u32>,
    pub row_target: Vec<u32>,
    pub outcome: Vec<f64>,
    pub frequency: Vec<u64>,
    pub target_weight: Vec<f64>,
    pub controls: Vec<Vec<f64>>,
    pub cell_worker: Vec<u32>,
    pub cell_firm: Vec<u32>,
    pub cell_weight: Vec<f64>,
    pub cell_outcome_sum: Vec<f64>,
    pub cell_target_sum: Vec<f64>,
    pub worker_index: GroupIndex,
    pub firm_index: GroupIndex,
    pub deletion_index: GroupIndex,
    pub target_index: GroupIndex,
    pub physical_total: u64,
    pub target_total: f64,
    pub topology_checksum: u64,
}

impl CompressedProblem {
    #[must_use]
    pub fn workers(&self) -> usize {
        usize::try_from(self.dimensions.workers).expect("validated worker dimension")
    }

    #[must_use]
    pub fn firms(&self) -> usize {
        usize::try_from(self.dimensions.firms).expect("validated firm dimension")
    }

    #[must_use]
    pub fn cells(&self) -> usize {
        self.cell_worker.len()
    }
}

fn redense(values: &[u64], label: 'static str) -> Result<(Vec<u32>, Vec<u64>)> {
    let mut levels = values.to_vec();
    levels.sort_unstable();
    levels.dedup();
    if levels.len() > u32::MAX as usize {
        return Err(BackendError::new(
            ErrorCode::ResourceLimit,
            "canonicalize",
            format!("{label} cardinality exceeds the u32 implementation limit"),
        ));
    }
    let map: BTreeMap<u64, u32> = levels
        .iter()
        .enumerate()
        .map(|(index, &value)| {
            (
                value,
                u32::try_from(index).expect("cardinality checked against u32"),
            )
        })
        .collect();
    let dense = values
        .iter()
        .map(|value| {
            map.get(value).copied().ok_or_else(|| {
                BackendError::invariant("canonicalize", format!("{label} map is incomplete"))
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok((dense, levels))
}

fn redense_selected(
    values: &[u32],
    selected: &[usize],
    old_levels: usize,
    label: 'static str,
) -> Result<Vec<u32>> {
    let mut present = vec![false; old_levels];
    for &row in selected {
        let level = usize::try_from(values[row]).map_err(|_| {
            BackendError::new(
                ErrorCode::InvalidIdentifier,
                "compression",
                format!("{label} identifier is not addressable"),
            )
        })?;
        if level >= old_levels {
            return Err(BackendError::invariant(
                "compression",
                format!("{label} identifier exceeds its level count"),
            ));
        }
        present[level] = true;
    }
    let mut map = vec![MISSING_ID; old_levels];
    let mut next = 0_u32;
    for (level, &keep) in present.iter().enumerate() {
        if keep {
            map[level] = next;
            next = next.checked_add(1).ok_or_else(|| {
                BackendError::new(
                    ErrorCode::ResourceLimit,
                    "compression",
                    format!("{label} cardinality exceeds u32"),
                )
            })?;
        }
    }
    Ok(map)
}

fn count_levels(map: &[u32]) -> usize {
    map.iter()
        .copied()
        .filter(|&&dlue| value != MISSING_ID)
        .max()
        .map_or(0, |maximum| usize::try_from(maximum).expect("u32") + 1)
}

fn grouped_items(groups: usize, group_of_item: &[u32]) -> Result<GroupIndex> {
    let order = (0..group_of_item.len())
        .map(|item| {
            u32::try_from(item).map_err(|_| {
                BackendError::new(
                    ErrorCode::ResourceLimit,
                    "compression",
                    "group item index exceeds u32",
                )
            })
        })
        .collect::<Result<Vec<_>>>()?;
    grouped_items_from_order(groups, group_of_item, order)
}

fn grouped_items_from_order(
    groups: usize,
    group_of_item: &[u32],
    order: Vec<u32>,
} -> Result<GroupIndex> {
    let mut ptr = vec[0_u64; groups + 1];
    for &item in &order {
        let item_index = usize::try_from(item).map_err(|_| {
            BackendError::new(
                ErrorCode::ResourceLimit,
                "compression",
                "group item is not addressable",
            )
        })?;
        let group = usize::try_from(group_of_item[item_index]).map_err(|_| {
            BackendError::new(
                ErrorCode::ResourceLimit,
                "compression",
                "group identifier is not addressable",
            )
        })?;
        if group >= groups {
            return Err(BackendError::invariant(
                "compression",
                "group identifier exceeds group count",
            ));
        }
        ptr[group + 1] = ptr[group + 1].checked_add(1).ok_or_else(|| {
            BackendError::new(
                ErrorCode::ResourceLimit,
                "compression",
                "group count overflow",
            )
        })?;
    }
    for group in 0..groups {
        ptr[group + 1] = ptr[group + 1].checked_add(ptr[group]).ok_or_else(|| {
            BackendError::new(
                ErrorCode::ResourceLimit,
                "compression",
                "group pointer overflow",
            )
        })?;
    }
    Ok(GroupIndex { ptr, items: order })
}

fn grouped_rows(groups: usize, row_group: &[u32]) -> Result<GroupIndex> {
    let mut order = (0..row_group.len())
        .map(|row| {
            u32::try_from(row).map_err(|_| {
                BackendError::new(
                    ErrorCode::ResourceLimit,
                    "compression",
                    "row index exceeds u32",
                )
            })
        })
        .collect::<Result<Vec<_>>>()?;
    order.sort_by_key(|&row| {
        let index = usize::try_from(row).expect("u32 row");
        (row_group[index], row)
    });
    grouped_items_from_order(groups, row_group, order)
}

fn exact_target_strata(
    input: &CanonicalInput,
    retained_rows: &[usize],
    worker: &[u32],
    firm: &[u32],
    deletion: &[u32],
) -> Result<(Vec<u32>, GroupIndex)> {
    let mut order: Vec<usize> = (0..retained_rows.len()).collect();
    order.sort_by(|&left, &right| {
        compare_target_rows(
            input,
            retained_rows[left],
            retained_rows[right],
            worker[left],
            worker[right],
            firm[left],
            firm[right],
            deletion[left],
            deletion[right],
        )
    });

    let mut target_id = vec![MISSING_ID; retained_rows.len()];
    let mut current = 0_u32;
    for (position, &local_row) in order.iter().enumerate() {
        if position > 0 {
            let previous = order[position - 1];
            if compare_target_rows(
                input,
                retained_rows[previous],
                retained_rows[local_row],
                worker[previous],
                worker[local_row],
                firm[previous],
                firm[local_row],
                deletion[previous],
                deletion[local_row],
            ) != Ordering::Equal
            {
                current = current.checked_add(1).ok_or_else(|| {
                    BackendError::new(
                        ErrorCode::ResourceLimit,
                        "compression",
                        "target-stratum count exceeds u32",
                    )
                })?;
            }
        }
        target_id[local_row] = current;
    }
    let groups = usize::try_from(current).expect("u32") + 1;
    let order_u32 = order
        .into_iter()
        .map(|row| {
            u32::try_from(row).map_err(|_| {
                BackendError::new(
                    ErrorCode::ResourceLimit,
                    "compression",
                    "target row index exceeds u32",
                )
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let index = grouped_items_from_order(groups, &target_id, order_u32)?;
    Ok((target_id, index))
}

#[allow(clippy::too_many_arguments)]
fn compare_target_rows(
    input: &CanonicalInput,
    left: usize,
    right: usize,
    left_worker: u32,
    right_worker: u32,
    left_firm: u32,
    right_firm: u32,
    left_deletion: u32,
    right_deletion: u32,
) -> Ordering {
    let mut ordering = left_worker
        .cmp(&right_worker)
        .then_with(|| left_firm.cmp(&right_firm))
        .then_with(|| left_deletion.cmp(&right_deletion))
        .then_with(|| input.outcome[left].total_cmp(&input.outcome[right]))
        .then_with(|| input.frequency[left].cmp(&input.frequency[right]))
        .then_with(|| input.target_weight[left].total_cmp(&input.target_weight[right]));
    for control in &input.controls {
        ordering = ordering.then_with(|| control[left].total_cmp(&control[right]));
    }
    ordering
}

fn topology_checksum(worker: &[u32], firm: &[u32], deletion: &[u32], frequency: &[u64]) -> u64 {
    let mut records: Vec<(u32, u32, u32, u64)> = (0..worker.len())
        .map(|row| (worker[row], firm[row], deletion[row], frequency[row]))
        .collect();
    records.sort_unstable();
    let mut hash = FNV_OFFSET;
    for (worker_id, firm_id, deletion_id, row_frequency) in records {
        hash_word(&mut hash, u64::from(worker_id));
        hash_word(&mut hash, u64::from(firm_id));
        hash_word(&mut hash, u64::from(deletion_id));
        hash_word(&mut hash, row_frequency);
    }
    hash
}

fn hash_word(hash: &mut u64, value: u64) {
    for byte in value.to_le_bytes() {
        *hash ^= u64::from(byte);
        *hash = hash.wrapping_mul(FNV_PRIME);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::InputColumns;

    fn input(order: &[usize]) -> ValidatedInput {
        let worker = [20_u64, 10, 20, 10];
        let firm = [200_u64, 100, 100, 200];
        let deletion = [4_u64, 1, 3, 2];
        let outcome = [4.0, 1.0, 3.0, 2.0];
        let frequency = [1_u64, 2, 3, 4];
        let target_weight = [1.0, 2.0, 3.0, 4.0];
        InputColumns {
            worker: order.iter().map(|&row| worker[row]).collect(),
            firm: order.iter().map(|&row| firm[row]).collect(),
            deletion: order.iter().map(|&row| deletion[row]).collect(),
            outcome: order.iter().map(|&row| outcome[row]).collect(),
            frequency: order.iter().map(|&row| frequency[row]).collect(),
            target_weight: order.iter().map(|&row| target_weight[row]).collect(),
            controls: Vec::new(),
        }
        .validate()
        .expect("fixture")
    }

    #[test]
    fn canonical_levels_follow_raw_identifier_order() {
        let canonical = CanonicalInput::from_validated(input(&[0, 1, 2, 3])).expect("canonical");
        assert_eq!(canonical.worker_levels, vec![10, 20]);
        assert_eq!(canonical.firm_levels, vec![100, 200]);
        assert_eq!(canonical.worker, vec![1, 0, 1, 0]);
        assert_eq!(canonical.firm, vec![1, 0, 0, 1]);
    }

    #[test]
    fn topology_checksum_and_compression_are_row_order_invariant() {
        let first = CanonicalInput::from_validated(input(&[0, 1, 2, 3])).expect("first");
        let second = CanonicalInput::from_validated(input(&[3, 2, 1, 0])).expect("second");
        assert_eq!(first.topology_checksum, second.topology_checksum);
        let first_problem = first.compress(&[true; 4]).expect("first compression");
        let second_problem = second.compress(&[true; 4]).expect("second compression");
        assert_eq!(first_problem.cell_worker, second_problem.cell_worker);
        assert_eq!(first_problem.cell_firm, second_problem.cell_firm);
        assert_eq!(first_problem.cell_weight, second_problem.cell_weight);
    }

    #[test]
    fn deletion_identifier_cannot_cross_coordinates() {
        let error = CanonicalInput::from_validated(
            InputColumns {
                worker: vec![1, 2],
                firm: vec![1, 2],
                deletion: vec![9, 9],
                outcome: vec![0.0, 0.0],
                frequency: vec![1, 1],
                target_weight: vec![1.0, 1.0],
                controls: Vec::new(),
            }
            .validate()
            .expect("validated"),
        )
        .expect_err("cross-coordinate deletion must fail");
        assert_eq(error.code, ErrorCode::InvalidIdentifier);
    }
}
