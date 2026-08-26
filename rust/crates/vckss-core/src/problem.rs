// SPDX-License-Identifier: GPL-3.0-only

use core::cmp::Ordering;
use std::collections::BTreeMap;

use crate::error::{BackendError, ErrorCode, Result};
use crate::interrupt::{
    checkpoint_chunk, stable_sort_by_with_interrupt, unstable_sort_by_with_interrupt,
    InterruptCheck, NeverInterrupt,
};
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
        self.validate_with_interrupt(groups, upper_item, &mut NeverInterrupt)
    }

    pub fn validate_with_interrupt(
        &self,
        groups: usize,
        upper_item: usize,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<()> {
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
        let mut invalid = self.ptr.last() != Some(&terminal);
        for (group, pair) in self.ptr.windows(2).enumerate() {
            checkpoint_chunk(interrupt, group, "compression_validate_group_ptr")?;
            invalid |= pair[0] > pair[1];
        }
        for (index, &item) in self.items.iter().enumerate() {
            checkpoint_chunk(interrupt, index, "compression_validate_group_items")?;
            invalid |= usize::try_from(item).map_or(true, |value| value >= upper_item);
        }
        if invalid {
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
        Self::from_validated_with_interrupt(input, &mut NeverInterrupt)
    }

    pub fn from_validated_with_interrupt(
        input: ValidatedInput,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<Self> {
        Self::from_validated_with_implicit_match_and_interrupt(input, false, interrupt)
    }

    pub fn from_validated_with_implicit_match_and_interrupt(
        input: ValidatedInput,
        implicit_match: bool,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<Self> {
        interrupt.checkpoint("canonicalize_entry")?;
        let (worker, worker_levels) = redense(&input.columns.worker, "worker", interrupt)?;
        let (firm, firm_levels) = redense(&input.columns.firm, "firm", interrupt)?;
        let implicit_match_keys = if implicit_match {
            Some(implicit_match_keys(
                &worker,
                &firm,
                firm_levels.len(),
                interrupt,
            )?)
        } else {
            None
        };
        let deletion_source = implicit_match_keys
            .as_deref()
            .unwrap_or(input.columns.deletion.as_slice());
        let (deletion, deletion_levels) = redense(deletion_source, "deletion", interrupt)?;

        let mut coordinate = vec![None; deletion_levels.len()];
        for row in 0..worker.len() {
            checkpoint_chunk(interrupt, row, "canonicalize_coordinates")?;
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

        let topology_checksum = topology_checksum(
            &worker,
            &firm,
            &deletion,
            &input.columns.frequency,
            interrupt,
        )?;
        interrupt.checkpoint("canonicalize_final")?;
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
        self.compress_with_interrupt(active, &mut NeverInterrupt)
    }

    pub fn compress_with_interrupt(
        &self,
        active: &[bool],
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<CompressedProblem> {
        self.compress_with_implicit_match_and_interrupt(active, false, interrupt)
    }

    pub fn compress_with_implicit_match_and_interrupt(
        &self,
        active: &[bool],
        implicit_match: bool,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<CompressedProblem> {
        interrupt.checkpoint("compression_entry")?;
        if active.len() != self.rows() {
            return Err(BackendError::invalid(
                "compression",
                "active mask has the wrong length",
            ));
        }
        if implicit_match && active.iter().any(|&keep| !keep) {
            return Err(BackendError::invariant(
                "compression",
                "the certified implicit-match route must retain every row",
            ));
        }
        let mut retained_rows = Vec::new();
        for (row, &keep) in active.iter().enumerate() {
            checkpoint_chunk(interrupt, row, "compression_retained_rows")?;
            if keep {
                retained_rows.push(row);
            }
        }
        if retained_rows.is_empty() {
            return Err(BackendError::new(
                ErrorCode::GraphEmpty,
                "compression",
                "no rows remain after graph selection",
            ));
        }

        let worker_map = if implicit_match {
            identity_map(self.workers(), "worker", interrupt)?
        } else {
            redense_selected(
                &self.worker,
                &retained_rows,
                self.workers(),
                "worker",
                interrupt,
            )?
        };
        let firm_map = if implicit_match {
            identity_map(self.firms(), "firm", interrupt)?
        } else {
            redense_selected(&self.firm, &retained_rows, self.firms(), "firm", interrupt)?
        };
        let deletion_map = if implicit_match {
            identity_map(self.deletion_units(), "deletion", interrupt)?
        } else {
            redense_selected(
                &self.deletion,
                &retained_rows,
                self.deletion_units(),
                "deletion",
                interrupt,
            )?
        };

        let mut row_worker = Vec::with_capacity(retained_rows.len());
        let mut row_firm = Vec::with_capacity(retained_rows.len());
        let mut row_deletion = Vec::with_capacity(retained_rows.len());
        for (local, &row) in retained_rows.iter().enumerate() {
            checkpoint_chunk(interrupt, local, "compression_row_maps")?;
            row_worker.push(worker_map[usize::try_from(self.worker[row]).expect("dense worker")]);
            row_firm.push(firm_map[usize::try_from(self.firm[row]).expect("dense firm")]);
            row_deletion
                .push(deletion_map[usize::try_from(self.deletion[row]).expect("dense deletion")]);
        }

        let workers = count_levels(&worker_map, interrupt)?;
        let firms = count_levels(&firm_map, interrupt)?;
        let deletion_units = count_levels(&deletion_map, interrupt)?;
        if firms < 2 {
            return Err(BackendError::new(
                ErrorCode::GraphUnidentified,
                "compression",
                "retained graph must contain at least two firms",
            ));
        }

        // The implicit-match certificate proves that each dense deletion
        // unit is exactly one worker-firm coordinate. Stable linear bucketing
        // by that already-dense key therefore yields the same cell order as
        // the ordinary worker/firm comparison sort, including source-row
        // order within a cell.
        let raw_deletion_index = if implicit_match {
            Some(grouped_rows(deletion_units, &row_deletion, interrupt)?)
        } else {
            None
        };
        let mut cell_order = Vec::with_capacity(retained_rows.len());
        if let Some(index) = raw_deletion_index.as_ref() {
            for (position, &row) in index.items.iter().enumerate() {
                checkpoint_chunk(interrupt, position, "compression_cell_order")?;
                cell_order.push(usize::try_from(row).expect("validated retained row"));
            }
        } else {
            for row in 0..retained_rows.len() {
                checkpoint_chunk(interrupt, row, "compression_cell_order")?;
                cell_order.push(row);
            }
            stable_sort_by_with_interrupt(
                &mut cell_order,
                |&left, &right| {
                    row_worker[left]
                        .cmp(&row_worker[right])
                        .then_with(|| row_firm[left].cmp(&row_firm[right]))
                        .then_with(|| retained_rows[left].cmp(&retained_rows[right]))
                },
                interrupt,
                "compression_cell_sort",
            )?;
        }

        let mut cell_worker = Vec::new();
        let mut cell_firm = Vec::new();
        let mut cell_weight = Vec::new();
        let mut cell_outcome_sum = Vec::new();
        let mut cell_target_sum = Vec::new();
        let mut row_cell = vec![MISSING_ID; retained_rows.len()];

        let mut cursor = 0;
        while cursor < cell_order.len() {
            checkpoint_chunk(interrupt, cursor, "compression_cells")?;
            let first = cell_order[cursor];
            let worker = row_worker[first];
            let firm = row_firm[first];
            let begin = cursor;
            cursor += 1;
            while cursor < cell_order.len()
                && row_worker[cell_order[cursor]] == worker
                && row_firm[cell_order[cursor]] == firm
            {
                checkpoint_chunk(interrupt, cursor - begin, "compression_cell_group")?;
                cursor += 1;
            }
            let cell = u32::try_from(cell_worker.len()).map_err(|_| {
                BackendError::new(
                    ErrorCode::ResourceLimit,
                    "compression",
                    "coefficient-cell count exceeds the u32 implementation limit",
                )
            })?;
            if implicit_match && row_deletion[first] != cell {
                return Err(BackendError::invariant(
                    "compression",
                    "implicit-match deletion order does not equal coefficient-cell order",
                ));
            }
            let mut weights = Vec::with_capacity(cursor - begin);
            let mut outcomes = Vec::with_capacity(cursor - begin);
            let mut targets = Vec::with_capacity(cursor - begin);
            for (cell_row, &local_row) in cell_order[begin..cursor].iter().enumerate() {
                checkpoint_chunk(interrupt, cell_row, "compression_cell_rows")?;
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
            cell_weight.push(compensated_sum_interrupt(
                &weights,
                interrupt,
                "compression_cell_weight",
            )?);
            cell_outcome_sum.push(compensated_sum_interrupt(
                &outcomes,
                interrupt,
                "compression_cell_outcome",
            )?);
            cell_target_sum.push(compensated_sum_interrupt(
                &targets,
                interrupt,
                "compression_cell_target",
            )?);
        }
        let mut missing_cell = false;
        for (row, &cell) in row_cell.iter().enumerate() {
            checkpoint_chunk(interrupt, row, "compression_cell_reconcile")?;
            missing_cell |= cell == MISSING_ID;
        }
        if missing_cell {
            return Err(BackendError::invariant(
                "compression",
                "not every retained row was assigned to a coefficient cell",
            ));
        }

        let worker_index = grouped_items(workers, &cell_worker, interrupt)?;
        let mut firm_order = Vec::with_capacity(cell_worker.len());
        for cell in 0..cell_worker.len() {
            checkpoint_chunk(interrupt, cell, "compression_firm_order")?;
            firm_order.push(u32::try_from(cell).map_err(|_| {
                BackendError::new(
                    ErrorCode::ResourceLimit,
                    "compression",
                    "cell index exceeds the u32 implementation limit",
                )
            })?);
        }
        stable_sort_by_with_interrupt(
            &mut firm_order,
            |&left, &right| {
                let left_index = usize::try_from(left).expect("u32 cell index");
                let right_index = usize::try_from(right).expect("u32 cell index");
                (cell_firm[left_index], cell_worker[left_index], left).cmp(&(
                    cell_firm[right_index],
                    cell_worker[right_index],
                    right,
                ))
            },
            interrupt,
            "compression_firm_sort",
        )?;
        let firm_index = grouped_items_from_order(firms, &cell_firm, firm_order, interrupt)?;

        let (target_id, target_index) = match raw_deletion_index.as_ref() {
            Some(index) => exact_target_strata_by_certified_match(
                self,
                &retained_rows,
                &row_worker,
                &row_firm,
                &row_deletion,
                index,
                interrupt,
            )?,
            None => exact_target_strata(
                self,
                &retained_rows,
                &row_worker,
                &row_firm,
                &row_deletion,
                interrupt,
            )?,
        };
        let deletion_index = match raw_deletion_index {
            Some(index) => index,
            None => grouped_rows(deletion_units, &row_deletion, interrupt)?,
        };
        let target_strata = target_index.ptr.len() - 1;

        let mut physical_total = 0_u64;
        let mut retained_targets = Vec::with_capacity(retained_rows.len());
        for (local, &row) in retained_rows.iter().enumerate() {
            checkpoint_chunk(interrupt, local, "compression_retained_totals")?;
            physical_total = physical_total
                .checked_add(self.frequency[row])
                .ok_or_else(|| {
                    BackendError::new(
                        ErrorCode::ResourceLimit,
                        "compression",
                        "retained physical-frequency total overflow",
                    )
                })?;
            retained_targets.push(self.target_weight[row]);
        }
        let target_total =
            compensated_sum_interrupt(&retained_targets, interrupt, "compression_target_total")?;
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

        worker_index.validate_with_interrupt(workers, cell_worker.len(), interrupt)?;
        firm_index.validate_with_interrupt(firms, cell_worker.len(), interrupt)?;
        deletion_index.validate_with_interrupt(deletion_units, retained_rows.len(), interrupt)?;
        target_index.validate_with_interrupt(target_strata, retained_rows.len(), interrupt)?;

        let mut outcome = Vec::with_capacity(retained_rows.len());
        let mut frequency = Vec::with_capacity(retained_rows.len());
        let mut target_weight = Vec::with_capacity(retained_rows.len());
        for (local, &row) in retained_rows.iter().enumerate() {
            checkpoint_chunk(interrupt, local, "compression_output_rows")?;
            outcome.push(self.outcome[row]);
            frequency.push(self.frequency[row]);
            target_weight.push(self.target_weight[row]);
        }
        let mut controls = Vec::with_capacity(self.controls.len());
        for (control_index, column) in self.controls.iter().enumerate() {
            let mut retained = Vec::with_capacity(retained_rows.len());
            for (local, &row) in retained_rows.iter().enumerate() {
                let flat = control_index
                    .checked_mul(retained_rows.len())
                    .and_then(|value| value.checked_add(local))
                    .ok_or_else(|| {
                        BackendError::new(
                            ErrorCode::ResourceLimit,
                            "compression",
                            "control-copy work counter overflow",
                        )
                    })?;
                checkpoint_chunk(interrupt, flat, "compression_output_controls")?;
                retained.push(column[row]);
            }
            controls.push(retained);
        }
        let topology_checksum =
            topology_checksum(&row_worker, &row_firm, &row_deletion, &frequency, interrupt)?;

        interrupt.checkpoint("compression_final")?;

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
            probe_order: None,
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

fn implicit_match_keys(
    worker: &[u32],
    firm: &[u32],
    firm_count: usize,
    interrupt: &mut dyn InterruptCheck,
) -> Result<Vec<u64>> {
    if worker.len() != firm.len() {
        return Err(BackendError::invariant(
            "canonicalize",
            "raw match worker and firm columns have inconsistent lengths",
        ));
    }
    let firm_count = u64::try_from(firm_count).map_err(|_| {
        BackendError::new(
            ErrorCode::ResourceLimit,
            "canonicalize",
            "raw match firm cardinality is not representable",
        )
    })?;
    let mut keys = Vec::with_capacity(worker.len());
    for row in 0..worker.len() {
        checkpoint_chunk(interrupt, row, "canonicalize_raw_match_keys")?;
        let key = u64::from(worker[row])
            .checked_mul(firm_count)
            .and_then(|value| value.checked_add(u64::from(firm[row])))
            .and_then(|value| value.checked_add(1))
            .ok_or_else(|| {
                BackendError::new(
                    ErrorCode::ResourceLimit,
                    "canonicalize",
                    "raw match coordinate identifier overflow",
                )
            })?;
        keys.push(key);
    }
    Ok(keys)
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
    pub probe_order: Option<Vec<f64>>,
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
    pub fn deletion_units(&self) -> usize {
        usize::try_from(self.dimensions.deletion_units).expect("validated deletion-unit dimension")
    }

    #[must_use]
    pub fn cells(&self) -> usize {
        self.cell_worker.len()
    }
}

fn redense(
    values: &[u64],
    label: &'static str,
    interrupt: &mut dyn InterruptCheck,
) -> Result<(Vec<u32>, Vec<u64>)> {
    let mut levels = Vec::with_capacity(values.len());
    for (index, &value) in values.iter().enumerate() {
        checkpoint_chunk(interrupt, index, "canonicalize_redense_copy")?;
        levels.push(value);
    }
    unstable_sort_by_with_interrupt(
        &mut levels,
        Ord::cmp,
        interrupt,
        "canonicalize_redense_sort",
    )?;
    if !levels.is_empty() {
        let mut write = 1_usize;
        for read in 1..levels.len() {
            checkpoint_chunk(interrupt, read, "canonicalize_redense_dedup")?;
            if levels[read] != levels[write - 1] {
                levels[write] = levels[read];
                write += 1;
            }
        }
        levels.truncate(write);
    }
    if levels.len() > u32::MAX as usize {
        return Err(BackendError::new(
            ErrorCode::ResourceLimit,
            "canonicalize",
            format!("{label} cardinality exceeds the u32 implementation limit"),
        ));
    }
    let mut map = BTreeMap::new();
    for (index, &value) in levels.iter().enumerate() {
        checkpoint_chunk(interrupt, index, "canonicalize_redense_levels")?;
        map.insert(
            value,
            u32::try_from(index).expect("cardinality checked against u32"),
        );
    }
    let mut dense = Vec::with_capacity(values.len());
    for (index, value) in values.iter().enumerate() {
        checkpoint_chunk(interrupt, index, "canonicalize_redense_map")?;
        dense.push(map.get(value).copied().ok_or_else(|| {
            BackendError::invariant("canonicalize", format!("{label} map is incomplete"))
        })?);
    }
    Ok((dense, levels))
}

fn redense_selected(
    values: &[u32],
    selected: &[usize],
    old_levels: usize,
    label: &'static str,
    interrupt: &mut dyn InterruptCheck,
) -> Result<Vec<u32>> {
    let mut present = vec![false; old_levels];
    for (index, &row) in selected.iter().enumerate() {
        checkpoint_chunk(interrupt, index, "compression_redense_selected")?;
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
        checkpoint_chunk(interrupt, level, "compression_redense_levels")?;
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

fn identity_map(
    levels: usize,
    label: &'static str,
    interrupt: &mut dyn InterruptCheck,
) -> Result<Vec<u32>> {
    let mut map = Vec::with_capacity(levels);
    for level in 0..levels {
        checkpoint_chunk(interrupt, level, "compression_identity_map")?;
        map.push(u32::try_from(level).map_err(|_| {
            BackendError::new(
                ErrorCode::ResourceLimit,
                "compression",
                format!("{label} cardinality exceeds u32"),
            )
        })?);
    }
    Ok(map)
}

fn count_levels(map: &[u32], interrupt: &mut dyn InterruptCheck) -> Result<usize> {
    let mut maximum = None;
    for (index, &value) in map.iter().enumerate() {
        checkpoint_chunk(interrupt, index, "compression_level_count")?;
        if value != MISSING_ID {
            maximum = Some(maximum.map_or(value, |current: u32| current.max(value)));
        }
    }
    Ok(maximum.map_or(0, |value| usize::try_from(value).expect("u32") + 1))
}

fn grouped_items(
    groups: usize,
    group_of_item: &[u32],
    interrupt: &mut dyn InterruptCheck,
) -> Result<GroupIndex> {
    let mut order = Vec::with_capacity(group_of_item.len());
    for item in 0..group_of_item.len() {
        checkpoint_chunk(interrupt, item, "compression_group_order")?;
        order.push(u32::try_from(item).map_err(|_| {
            BackendError::new(
                ErrorCode::ResourceLimit,
                "compression",
                "group item index exceeds u32",
            )
        })?);
    }
    grouped_items_from_order(groups, group_of_item, order, interrupt)
}

fn grouped_items_from_order(
    groups: usize,
    group_of_item: &[u32],
    order: Vec<u32>,
    interrupt: &mut dyn InterruptCheck,
) -> Result<GroupIndex> {
    let mut ptr = vec![0_u64; groups + 1];
    for (index, &item) in order.iter().enumerate() {
        checkpoint_chunk(interrupt, index, "compression_group_counts")?;
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
        checkpoint_chunk(interrupt, group, "compression_group_prefix")?;
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

fn grouped_rows(
    groups: usize,
    row_group: &[u32],
    interrupt: &mut dyn InterruptCheck,
) -> Result<GroupIndex> {
    let mut ptr = vec![0_u64; groups + 1];
    for (row, &group) in row_group.iter().enumerate() {
        checkpoint_chunk(interrupt, row, "compression_grouped_row_counts")?;
        let group = usize::try_from(group).map_err(|_| {
            BackendError::new(
                ErrorCode::ResourceLimit,
                "compression",
                "row group is not addressable",
            )
        })?;
        if group >= groups {
            return Err(BackendError::invariant(
                "compression",
                "row group exceeds group count",
            ));
        }
        ptr[group + 1] = ptr[group + 1].checked_add(1).ok_or_else(|| {
            BackendError::new(
                ErrorCode::ResourceLimit,
                "compression",
                "row-group count overflow",
            )
        })?;
    }
    for group in 0..groups {
        checkpoint_chunk(interrupt, group, "compression_grouped_row_prefix")?;
        ptr[group + 1] = ptr[group + 1].checked_add(ptr[group]).ok_or_else(|| {
            BackendError::new(
                ErrorCode::ResourceLimit,
                "compression",
                "row-group pointer overflow",
            )
        })?;
    }
    let mut cursor = ptr[..groups].to_vec();
    let mut items = vec![MISSING_ID; row_group.len()];
    for (row, &group) in row_group.iter().enumerate() {
        checkpoint_chunk(interrupt, row, "compression_grouped_row_scatter")?;
        let group = usize::try_from(group).expect("validated row group");
        let position = usize::try_from(cursor[group]).map_err(|_| {
            BackendError::new(
                ErrorCode::ResourceLimit,
                "compression",
                "row-group position is not addressable",
            )
        })?;
        items[position] = u32::try_from(row).map_err(|_| {
            BackendError::new(
                ErrorCode::ResourceLimit,
                "compression",
                "row index exceeds u32",
            )
        })?;
        cursor[group] = cursor[group].checked_add(1).ok_or_else(|| {
            BackendError::new(
                ErrorCode::ResourceLimit,
                "compression",
                "row-group cursor overflow",
            )
        })?;
    }
    Ok(GroupIndex { ptr, items })
}

fn exact_target_strata(
    input: &CanonicalInput,
    retained_rows: &[usize],
    worker: &[u32],
    firm: &[u32],
    deletion: &[u32],
    interrupt: &mut dyn InterruptCheck,
) -> Result<(Vec<u32>, GroupIndex)> {
    let mut order = Vec::with_capacity(retained_rows.len());
    for row in 0..retained_rows.len() {
        checkpoint_chunk(interrupt, row, "compression_target_order")?;
        order.push(row);
    }
    stable_sort_by_with_interrupt(
        &mut order,
        |&left, &right| {
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
        },
        interrupt,
        "compression_target_sort",
    )?;

    let mut target_id = vec![MISSING_ID; retained_rows.len()];
    let mut current = 0_u32;
    for (position, &local_row) in order.iter().enumerate() {
        checkpoint_chunk(interrupt, position, "compression_target_groups")?;
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
    let mut order_u32 = Vec::with_capacity(order.len());
    for (position, row) in order.into_iter().enumerate() {
        checkpoint_chunk(interrupt, position, "compression_target_index")?;
        order_u32.push(u32::try_from(row).map_err(|_| {
            BackendError::new(
                ErrorCode::ResourceLimit,
                "compression",
                "target row index exceeds u32",
            )
        })?);
    }
    let index = grouped_items_from_order(groups, &target_id, order_u32, interrupt)?;
    Ok((target_id, index))
}

#[allow(clippy::too_many_arguments)]
fn exact_target_strata_by_certified_match(
    input: &CanonicalInput,
    retained_rows: &[usize],
    worker: &[u32],
    firm: &[u32],
    deletion: &[u32],
    deletion_index: &GroupIndex,
    interrupt: &mut dyn InterruptCheck,
) -> Result<(Vec<u32>, GroupIndex)> {
    let rows = retained_rows.len();
    if worker.len() != rows
        || firm.len() != rows
        || deletion.len() != rows
        || deletion_index.items.len() != rows
        || deletion_index.ptr.len() < 2
    {
        return Err(BackendError::invariant(
            "compression",
            "certified raw-match target inputs have inconsistent dimensions",
        ));
    }

    // The raw-match certificate orders deletion units by the same dense
    // worker-firm coordinate that leads the ordinary global comparator. Sort
    // only within each coordinate, retaining the exact stable global order.
    let groups = deletion_index.ptr.len() - 1;
    let mut order = deletion_index.items.clone();
    let mut previous_coordinate = None;
    for group in 0..groups {
        checkpoint_chunk(interrupt, group, "compression_raw_target_cells")?;
        let range = deletion_index.range(group);
        if range.is_empty() {
            return Err(BackendError::invariant(
                "compression",
                "certified raw-match target cell is empty",
            ));
        }
        let first = usize::try_from(order[range.start]).map_err(|_| {
            BackendError::new(
                ErrorCode::ResourceLimit,
                "compression",
                "certified raw-match target row is not addressable",
            )
        })?;
        let coordinate = (worker[first], firm[first]);
        if previous_coordinate.is_some_and(|previous| previous >= coordinate) {
            return Err(BackendError::invariant(
                "compression",
                "certified raw-match target cells are not in worker-firm order",
            ));
        }
        previous_coordinate = Some(coordinate);
        let expected = u32::try_from(group).map_err(|_| {
            BackendError::new(
                ErrorCode::ResourceLimit,
                "compression",
                "certified raw-match target cell exceeds u32",
            )
        })?;
        for (local, &row) in order[range.clone()].iter().enumerate() {
            checkpoint_chunk(
                interrupt,
                range.start + local,
                "compression_raw_target_reconcile",
            )?;
            let row = usize::try_from(row).map_err(|_| {
                BackendError::new(
                    ErrorCode::ResourceLimit,
                    "compression",
                    "certified raw-match target row is not addressable",
                )
            })?;
            if deletion[row] != expected || (worker[row], firm[row]) != coordinate {
                return Err(BackendError::invariant(
                    "compression",
                    "certified raw-match target row does not reconcile with its cell",
                ));
            }
        }
        if range.len() > 1 {
            stable_sort_by_with_interrupt(
                &mut order[range],
                |&left, &right| {
                    let left = usize::try_from(left).expect("validated retained row");
                    let right = usize::try_from(right).expect("validated retained row");
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
                },
                interrupt,
                "compression_raw_target_sort",
            )?;
        }
    }

    let mut target_id = vec![MISSING_ID; rows];
    let mut ptr = vec![0_u64];
    let mut current = 0_u32;
    for (position, &row) in order.iter().enumerate() {
        checkpoint_chunk(interrupt, position, "compression_raw_target_groups")?;
        let row = usize::try_from(row).expect("validated retained row");
        if position > 0 {
            let previous = usize::try_from(order[position - 1]).expect("validated retained row");
            if compare_target_rows(
                input,
                retained_rows[previous],
                retained_rows[row],
                worker[previous],
                worker[row],
                firm[previous],
                firm[row],
                deletion[previous],
                deletion[row],
            ) != Ordering::Equal
            {
                ptr.push(u64::try_from(position).map_err(|_| {
                    BackendError::new(
                        ErrorCode::ResourceLimit,
                        "compression",
                        "certified raw-match target offset is not representable",
                    )
                })?);
                current = current.checked_add(1).ok_or_else(|| {
                    BackendError::new(
                        ErrorCode::ResourceLimit,
                        "compression",
                        "target-stratum count exceeds u32",
                    )
                })?;
            }
        }
        target_id[row] = current;
    }
    ptr.push(u64::try_from(rows).map_err(|_| {
        BackendError::new(
            ErrorCode::ResourceLimit,
            "compression",
            "certified raw-match target row count is not representable",
        )
    })?);
    Ok((target_id, GroupIndex { ptr, items: order }))
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

fn topology_checksum(
    worker: &[u32],
    firm: &[u32],
    deletion: &[u32],
    frequency: &[u64],
    interrupt: &mut dyn InterruptCheck,
) -> Result<u64> {
    let mut records = Vec::with_capacity(worker.len());
    for row in 0..worker.len() {
        checkpoint_chunk(interrupt, row, "canonicalize_topology_records")?;
        records.push((worker[row], firm[row], deletion[row], frequency[row]));
    }
    unstable_sort_by_with_interrupt(
        &mut records,
        Ord::cmp,
        interrupt,
        "canonicalize_topology_sort",
    )?;
    let mut hash = FNV_OFFSET;
    for (index, (worker_id, firm_id, deletion_id, row_frequency)) in records.into_iter().enumerate()
    {
        checkpoint_chunk(interrupt, index, "canonicalize_topology_hash")?;
        hash_word(&mut hash, u64::from(worker_id));
        hash_word(&mut hash, u64::from(firm_id));
        hash_word(&mut hash, u64::from(deletion_id));
        hash_word(&mut hash, row_frequency);
    }
    Ok(hash)
}

fn hash_word(hash: &mut u64, value: u64) {
    for byte in value.to_le_bytes() {
        *hash ^= u64::from(byte);
        *hash = hash.wrapping_mul(FNV_PRIME);
    }
}

fn compensated_sum_interrupt(
    values: &[f64],
    interrupt: &mut dyn InterruptCheck,
    phase: &'static str,
) -> Result<f64> {
    let mut sum = 0.0_f64;
    let mut correction = 0.0_f64;
    for (index, &value) in values.iter().enumerate() {
        checkpoint_chunk(interrupt, index, phase)?;
        let adjusted = value - correction;
        let next = sum + adjusted;
        correction = (next - sum) - adjusted;
        sum = next;
    }
    Ok(sum)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::BackendError;
    use crate::types::InputColumns;

    #[derive(Debug)]
    struct AuditCellCopy {
        cell_row_checkpoints: usize,
        first_cell_reduced: bool,
    }

    impl InterruptCheck for AuditCellCopy {
        fn checkpoint(&mut self, phase: &'static str) -> Result<()> {
            if phase == "compression_cell_rows" {
                self.cell_row_checkpoints += 1;
            } else if phase == "compression_cell_weight" && !self.first_cell_reduced {
                self.first_cell_reduced = true;
                if self.cell_row_checkpoints < 3 {
                    return Err(BackendError::new(
                        ErrorCode::UserBreak,
                        phase,
                        "large coefficient-cell copy was not checkpointed by bounded work",
                    ));
                }
            }
            Ok(())
        }
    }

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
        assert_eq!(error.code, ErrorCode::InvalidIdentifier);
    }

    #[test]
    fn grouped_rows_is_stable_with_dense_linear_bucketing() {
        let grouped =
            grouped_rows(3, &[2, 0, 2, 1, 0], &mut NeverInterrupt).expect("stable grouped rows");
        assert_eq!(grouped.ptr, vec![0, 2, 3, 5]);
        assert_eq!(grouped.items, vec![1, 4, 3, 0, 2]);
        grouped.validate(3, 5).expect("valid grouped rows");
    }

    #[test]
    fn implicit_match_keys_identify_dense_worker_firm_coordinates() {
        let keys = implicit_match_keys(&[1, 0, 1, 0, 1], &[1, 0, 0, 1, 1], 2, &mut NeverInterrupt)
            .expect("implicit pair keys");
        assert_eq!(keys, vec![4, 1, 3, 2, 4]);
    }

    #[test]
    fn certified_match_target_strata_reproduce_global_order() {
        let canonical = CanonicalInput::from_validated(
            InputColumns {
                worker: vec![20, 10, 20, 10, 20, 10, 20],
                firm: vec![200, 100, 100, 200, 200, 100, 100],
                deletion: vec![7, 1, 5, 3, 6, 2, 4],
                outcome: vec![4.0, 1.0, -2.0, 3.0, 4.0, -1.0, -2.0],
                frequency: vec![1, 2, 1, 1, 1, 1, 3],
                target_weight: vec![1.0, 2.0, 1.0, 1.0, 1.0, 1.0, 3.0],
                controls: Vec::new(),
            }
            .validate()
            .expect("validated certified-match fixture"),
        )
        .expect("canonical certified-match fixture");
        let mut problem = canonical
            .compress(&vec![true; canonical.rows()])
            .expect("compressed certified-match fixture");
        problem.row_deletion.clone_from(&problem.row_cell);
        let deletion_index =
            grouped_rows(problem.cells(), &problem.row_deletion, &mut NeverInterrupt)
                .expect("certified cell index");

        let global = exact_target_strata(
            &canonical,
            &problem.retained_rows,
            &problem.row_worker,
            &problem.row_firm,
            &problem.row_deletion,
            &mut NeverInterrupt,
        )
        .expect("global exact target strata");
        let grouped = exact_target_strata_by_certified_match(
            &canonical,
            &problem.retained_rows,
            &problem.row_worker,
            &problem.row_firm,
            &problem.row_deletion,
            &deletion_index,
            &mut NeverInterrupt,
        )
        .expect("cell-local exact target strata");
        assert_eq!(grouped, global);
    }

    #[test]
    fn large_odd_index_coefficient_cell_copy_uses_work_offsets() {
        const CELL_ROWS: usize = 9_001;
        let rows = CELL_ROWS * 2;
        let mut columns = InputColumns {
            worker: Vec::with_capacity(rows),
            firm: Vec::with_capacity(rows),
            deletion: Vec::with_capacity(rows),
            outcome: Vec::with_capacity(rows),
            frequency: Vec::with_capacity(rows),
            target_weight: Vec::with_capacity(rows),
            controls: Vec::new(),
        };
        for row in 0..rows {
            // Stable coefficient-cell ordering puts the odd source indices in
            // the first 9,001-row cell. A source-row-based checkpoint never
            // fires in that first copy because every identifier is odd.
            let first_cell = row % 2 == 1;
            columns.worker.push(if first_cell { 1 } else { 2 });
            columns.firm.push(if first_cell { 1 } else { 2 });
            columns.deletion.push(row as u64 + 1);
            columns.outcome.push(row as f64 / 17.0);
            columns.frequency.push((row % 3 + 1) as u64);
            columns.target_weight.push((row % 5 + 1) as f64);
        }
        let canonical = CanonicalInput::from_validated(columns.validate().expect("large fixture"))
            .expect("canonical fixture");
        let mut audit = AuditCellCopy {
            cell_row_checkpoints: 0,
            first_cell_reduced: false,
        };
        let problem = canonical
            .compress_with_interrupt(&vec![true; rows], &mut audit)
            .expect("interruptible compression");
        assert_eq!(problem.dimensions.cells, 2);
        assert!(audit.first_cell_reduced);
        assert_eq!(audit.cell_row_checkpoints, 6);
    }
}
