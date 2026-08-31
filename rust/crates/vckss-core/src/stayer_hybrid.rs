// SPDX-License-Identifier: GPL-3.0-only

//! Source-bound augmentation blocks shared by exact and generic-JLA
//! mover-plus-stayer estimation.
//!
//! This module owns validated augmentation of an already graph-certified mover
//! problem.  The exact kernel then uses the ordinary certified match-block and
//! physical-observation paths under one combined fit and pooled target.  The
//! independent finite point-target oracle remains deliberately separate.

use core::fmt;

use crate::error::{BackendError, ErrorCode, Result as BackendResult};
use crate::exact_estimator::ExactStayerHybridPlan;
use crate::interrupt::{checkpoint_chunk, InterruptCheck, NeverInterrupt};
use crate::problem::{CanonicalInput, CompressedProblem};
use crate::types::InputColumns;

/// Largest integer that is represented exactly by an IEEE-754 `f64`.
const MAX_EXACT_INTEGER: f64 = 9_007_199_254_740_992.0;

#[derive(Clone, Debug)]
pub struct StayerAugmentationInput {
    /// One-based dense retained-mover-firm index.
    pub firm: Vec<u64>,
    /// One-based dense eligible-stayer-worker index.
    pub worker: Vec<u64>,
    pub outcome: Vec<f64>,
    pub frequency: Vec<u64>,
    pub target_weight: Vec<f64>,
    /// Column-major controls in the same order as the mover problem.
    pub controls: Vec<Vec<f64>>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StayerAugmentationReceipt {
    pub mover_stored_rows: u64,
    pub stayer_stored_rows: u64,
    pub combined_stored_rows: u64,
    pub mover_physical_mass: u64,
    pub stayer_physical_mass: u64,
    pub combined_physical_mass: u64,
    pub mover_workers: u64,
    pub stayer_workers: u64,
    pub combined_workers: u64,
    pub firms: u64,
    pub mover_deletion_units: u64,
    pub stayer_deletion_units: u64,
    pub combined_deletion_units: u64,
    pub mover_target_mass: f64,
    pub stayer_target_mass: f64,
    pub combined_target_mass: f64,
    pub topology_checksum: u64,
}

#[derive(Clone, Debug)]
pub struct PreparedExactStayerHybrid {
    pub problem: CompressedProblem,
    pub plan: ExactStayerHybridPlan,
    pub receipt: StayerAugmentationReceipt,
}

pub fn prepare_exact_stayer_hybrid(
    mover: &CompressedProblem,
    stayers: StayerAugmentationInput,
) -> BackendResult<PreparedExactStayerHybrid> {
    prepare_exact_stayer_hybrid_with_interrupt(mover, stayers, &mut NeverInterrupt)
}

#[allow(clippy::too_many_lines)]
pub fn prepare_exact_stayer_hybrid_with_interrupt(
    mover: &CompressedProblem,
    stayers: StayerAugmentationInput,
    interrupt: &mut dyn InterruptCheck,
) -> BackendResult<PreparedExactStayerHybrid> {
    interrupt.checkpoint("stayer_augmentation_entry")?;
    if mover.outcome.is_empty() || mover.workers() == 0 || mover.firms() < 2 {
        return Err(BackendError::invalid(
            "stayer_augmentation",
            "the retained mover problem has invalid dimensions",
        ));
    }
    if mover.probe_order.is_some() {
        return Err(BackendError::new(
            ErrorCode::UnsupportedFeature,
            "stayer_augmentation",
            "probe-order input is not applicable to the mixed stayer augmentation",
        ));
    }
    let stayer_rows = stayers.outcome.len();
    let equal_lengths = stayers.firm.len() == stayer_rows
        && stayers.worker.len() == stayer_rows
        && stayers.frequency.len() == stayer_rows
        && stayers.target_weight.len() == stayer_rows
        && stayers.controls.len() == mover.controls.len()
        && stayers
            .controls
            .iter()
            .all(|column| column.len() == stayer_rows);
    if !equal_lengths {
        return Err(BackendError::invalid(
            "stayer_augmentation",
            "stayer columns have inconsistent dimensions",
        ));
    }

    let mut stayer_workers = 0_usize;
    for (row, &worker) in stayers.worker.iter().enumerate() {
        checkpoint_chunk(interrupt, row, "stayer_augmentation_worker_max")?;
        let worker = usize::try_from(worker).map_err(|_| {
            BackendError::new(
                ErrorCode::InvalidIdentifier,
                "stayer_augmentation",
                "a stayer worker identifier is not representable",
            )
        })?;
        stayer_workers = stayer_workers.max(worker);
    }
    if (stayer_rows == 0) != (stayer_workers == 0) {
        return Err(BackendError::new(
            ErrorCode::InvalidIdentifier,
            "stayer_augmentation",
            "eligible stayer workers must be a nonempty dense map when rows are supplied",
        ));
    }
    let mut worker_firm = vec![None; stayer_workers];
    let mut worker_physical = vec![0_u64; stayer_workers];
    let mut worker_seen = vec![false; stayer_workers];
    let mut stayer_physical_mass = 0_u64;
    let mut stayer_target_sum = CompensatedSum::default();
    for row in 0..stayer_rows {
        checkpoint_chunk(interrupt, row, "stayer_augmentation_validate")?;
        let firm = usize::try_from(stayers.firm[row]).map_err(|_| {
            BackendError::new(
                ErrorCode::InvalidIdentifier,
                "stayer_augmentation",
                "a stayer firm identifier is not representable",
            )
        })?;
        if firm == 0 || firm > mover.firms() {
            return Err(BackendError::new(
                ErrorCode::InvalidIdentifier,
                "stayer_augmentation",
                format!("stayer row {row} is not attached to a retained mover firm"),
            ));
        }
        let worker = usize::try_from(stayers.worker[row]).map_err(|_| {
            BackendError::new(
                ErrorCode::InvalidIdentifier,
                "stayer_augmentation",
                "a stayer worker identifier is not representable",
            )
        })?;
        if worker == 0 || worker > stayer_workers {
            return Err(BackendError::new(
                ErrorCode::InvalidIdentifier,
                "stayer_augmentation",
                format!("stayer row {row} has a non-dense worker identifier"),
            ));
        }
        let worker_index = worker - 1;
        match worker_firm[worker_index] {
            Some(previous) if previous != firm => {
                return Err(BackendError::new(
                    ErrorCode::InvalidIdentifier,
                    "stayer_augmentation",
                    format!("eligible stayer worker {worker} spans more than one firm"),
                ));
            }
            None => worker_firm[worker_index] = Some(firm),
            Some(_) => {}
        }
        worker_seen[worker_index] = true;
        worker_physical[worker_index] = worker_physical[worker_index]
            .checked_add(stayers.frequency[row])
            .ok_or_else(|| augmentation_resource("stayer worker physical mass overflow"))?;
        stayer_physical_mass = stayer_physical_mass
            .checked_add(stayers.frequency[row])
            .ok_or_else(|| augmentation_resource("stayer physical mass overflow"))?;
        stayer_target_sum.add(stayers.target_weight[row]);
    }
    if worker_seen.iter().any(|&seen| !seen) {
        return Err(BackendError::new(
            ErrorCode::InvalidIdentifier,
            "stayer_augmentation",
            "eligible stayer worker identifiers are not dense",
        ));
    }
    if worker_physical.iter().any(|&mass| mass < 2) {
        return Err(BackendError::invalid(
            "stayer_augmentation",
            "every eligible stayer worker must represent at least two physical observations",
        ));
    }

    let mover_rows = mover.outcome.len();
    let combined_rows = mover_rows
        .checked_add(stayer_rows)
        .ok_or_else(|| augmentation_resource("combined stored-row count overflow"))?;
    let mover_workers = mover.workers();
    let mover_deletion_units = mover.deletion_units();
    let mut columns = InputColumns {
        worker: Vec::with_capacity(combined_rows),
        firm: Vec::with_capacity(combined_rows),
        deletion: Vec::with_capacity(combined_rows),
        outcome: Vec::with_capacity(combined_rows),
        frequency: Vec::with_capacity(combined_rows),
        target_weight: Vec::with_capacity(combined_rows),
        controls: mover
            .controls
            .iter()
            .map(|_| Vec::with_capacity(combined_rows))
            .collect(),
    };
    for row in 0..mover_rows {
        checkpoint_chunk(interrupt, row, "stayer_augmentation_copy_movers")?;
        columns.worker.push(u64::from(mover.row_worker[row]) + 1);
        columns.firm.push(u64::from(mover.row_firm[row]) + 1);
        columns
            .deletion
            .push(u64::from(mover.row_deletion[row]) + 1);
        columns.outcome.push(mover.outcome[row]);
        columns.frequency.push(mover.frequency[row]);
        columns.target_weight.push(mover.target_weight[row]);
        for (column, values) in mover.controls.iter().enumerate() {
            columns.controls[column].push(values[row]);
        }
    }
    for row in 0..stayer_rows {
        checkpoint_chunk(interrupt, row, "stayer_augmentation_copy_stayers")?;
        let worker = u64::try_from(mover_workers)
            .ok()
            .and_then(|value| value.checked_add(stayers.worker[row]))
            .ok_or_else(|| augmentation_resource("combined worker identifier overflow"))?;
        let deletion = u64::try_from(mover_deletion_units)
            .ok()
            .and_then(|value| value.checked_add(row as u64 + 1))
            .ok_or_else(|| augmentation_resource("combined deletion identifier overflow"))?;
        columns.worker.push(worker);
        columns.firm.push(stayers.firm[row]);
        columns.deletion.push(deletion);
        columns.outcome.push(stayers.outcome[row]);
        columns.frequency.push(stayers.frequency[row]);
        columns.target_weight.push(stayers.target_weight[row]);
        for (column, values) in stayers.controls.iter().enumerate() {
            columns.controls[column].push(values[row]);
        }
    }
    let canonical = CanonicalInput::from_validated_with_interrupt(
        columns.validate_with_interrupt(interrupt)?,
        interrupt,
    )?;
    let active = vec![true; combined_rows];
    let problem = canonical.compress_with_interrupt(&active, interrupt)?;
    let expected_workers = mover_workers
        .checked_add(stayer_workers)
        .ok_or_else(|| augmentation_resource("combined worker count overflow"))?;
    if problem.outcome.len() != combined_rows
        || problem.workers() != expected_workers
        || problem.firms() != mover.firms()
        || problem.deletion_units() != mover_deletion_units + stayer_rows
    {
        return Err(BackendError::invariant(
            "stayer_augmentation",
            "combined exact problem dimensions do not reconcile",
        ));
    }
    for row in 0..mover_rows {
        checkpoint_chunk(interrupt, row, "stayer_augmentation_reconcile_movers")?;
        if problem.row_worker[row] != mover.row_worker[row]
            || problem.row_firm[row] != mover.row_firm[row]
            || problem.row_deletion[row] != mover.row_deletion[row]
            || problem.outcome[row].to_bits() != mover.outcome[row].to_bits()
            || problem.frequency[row] != mover.frequency[row]
            || problem.target_weight[row].to_bits() != mover.target_weight[row].to_bits()
            || problem
                .controls
                .iter()
                .zip(&mover.controls)
                .any(|(left, right)| left[row].to_bits() != right[row].to_bits())
        {
            return Err(BackendError::invariant(
                "stayer_augmentation",
                "combined preparation changed a retained mover row",
            ));
        }
    }
    let combined_deletion_units = u64::try_from(mover_deletion_units)
        .map_err(|_| augmentation_resource("mover deletion-unit count overflow"))?
        .checked_add(stayer_physical_mass)
        .ok_or_else(|| augmentation_resource("combined physical deletion count overflow"))?;
    let stayer_target_mass = stayer_target_sum.value();
    if !stayer_target_mass.is_finite() || stayer_target_mass < 0.0 {
        return Err(BackendError::new(
            ErrorCode::InvalidTargetWeight,
            "stayer_augmentation",
            "stayer target mass is not finite and nonnegative",
        ));
    }
    let receipt = StayerAugmentationReceipt {
        mover_stored_rows: augmentation_u64(mover_rows, "mover stored rows")?,
        stayer_stored_rows: augmentation_u64(stayer_rows, "stayer stored rows")?,
        combined_stored_rows: augmentation_u64(combined_rows, "combined stored rows")?,
        mover_physical_mass: mover.physical_total,
        stayer_physical_mass,
        combined_physical_mass: problem.physical_total,
        mover_workers: augmentation_u64(mover_workers, "mover workers")?,
        stayer_workers: augmentation_u64(stayer_workers, "stayer workers")?,
        combined_workers: augmentation_u64(expected_workers, "combined workers")?,
        firms: augmentation_u64(mover.firms(), "retained firms")?,
        mover_deletion_units: augmentation_u64(mover_deletion_units, "mover deletion units")?,
        stayer_deletion_units: stayer_physical_mass,
        combined_deletion_units,
        mover_target_mass: mover.target_total,
        stayer_target_mass,
        combined_target_mass: problem.target_total,
        topology_checksum: problem.topology_checksum,
    };
    if receipt.combined_physical_mass
        != receipt
            .mover_physical_mass
            .checked_add(receipt.stayer_physical_mass)
            .ok_or_else(|| augmentation_resource("combined physical receipt overflow"))?
        || (receipt.combined_target_mass - (receipt.mover_target_mass + receipt.stayer_target_mass))
            .abs()
            > 1.0e-12
                * receipt
                    .combined_target_mass
                    .abs()
                    .max(receipt.mover_target_mass.abs())
                    .max(receipt.stayer_target_mass.abs())
                    .max(1.0)
    {
        return Err(BackendError::invariant(
            "stayer_augmentation",
            "combined sample accounting does not reconcile",
        ));
    }
    interrupt.checkpoint("stayer_augmentation_final")?;
    Ok(PreparedExactStayerHybrid {
        problem,
        plan: ExactStayerHybridPlan {
            stayer_rows: (0..combined_rows).map(|row| row >= mover_rows).collect(),
            mover_deletion_units,
        },
        receipt,
    })
}

fn augmentation_u64(value: usize, label: &'static str) -> BackendResult<u64> {
    u64::try_from(value).map_err(|_| augmentation_resource(label))
}

fn augmentation_resource(message: &'static str) -> BackendError {
    BackendError::new(ErrorCode::ResourceLimit, "stayer_augmentation", message)
}

/// Borrowed, dense columnar rows for eligible one-firm stayers.
#[derive(Clone, Copy, Debug)]
pub struct StayerRows<'a> {
    /// Dense retained-mover-graph firm index for every stored row.
    pub firm: &'a [usize],
    /// Dense stayer-worker index for every stored row.
    pub worker: &'a [usize],
    /// Dense stayer observation-deletion-unit index for every stored row.
    pub deletion_unit: &'a [usize],
    /// Outcome column.
    pub outcome: &'a [f64],
    /// Positive integer physical frequency for every stored row.
    pub frequency: &'a [f64],
    /// Nonnegative target weight for every stored row.
    pub target_weight: &'a [f64],
    /// Optional fixed offset column.
    pub offset: Option<&'a [f64]>,
    /// Row-major controls with `rows * controls_count` entries.
    pub controls: &'a [f64],
    /// Number of control columns.
    pub controls_count: usize,
    /// Number of retained mover firms available to the stayer rows.
    pub retained_firms: usize,
    /// Number of densely indexed eligible stayer workers.
    pub stayer_workers: usize,
    /// Number of densely indexed stayer observation deletion units.
    pub deletion_units: usize,
}

/// Receipt emitted after validating a [`StayerRows`] view.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StayerValidationReceipt {
    /// Number of stored rows.
    pub stored_rows: usize,
    /// Number of observed dense stayer workers.
    pub observed_workers: usize,
    /// Number of retained mover firms touched by stayers.
    pub firms_touched: usize,
    /// Number of observed dense deletion units.
    pub observed_deletion_units: usize,
    /// Sum of physical frequencies.
    pub physical_mass: f64,
    /// Sum of target weights.
    pub target_mass: f64,
    /// Largest stored-row physical frequency.
    pub maximum_frequency: f64,
}

/// Independent finite-oracle input for stayer point targets.
#[derive(Clone, Copy, Debug)]
pub struct StayerPointOracleInput<'a> {
    /// Validated stayer rows.
    pub rows: StayerRows<'a>,
    /// Firm effects indexed by the retained mover-firm index.
    pub firm_effects: &'a [f64],
    /// Control coefficients in the same order as the row-major controls.
    pub control_coefficients: &'a [f64],
}

/// Centered target-weighted point moments for eligible stayers.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StayerPointMoments {
    /// Pooled stayer target mass used by this oracle block.
    pub target_mass: f64,
    /// Target-weighted mean worker effect.
    pub worker_mean: f64,
    /// Target-weighted mean firm effect.
    pub firm_mean: f64,
    /// Centered target-weighted worker-effect variance.
    pub worker_variance: f64,
    /// Centered target-weighted firm-effect variance.
    pub firm_variance: f64,
    /// Centered target-weighted worker-firm covariance.
    pub covariance: f64,
    /// Centered target-weighted variance of worker plus firm effects.
    pub total_variance: f64,
    /// `total - worker - firm - 2 * covariance`.
    pub accounting_error: f64,
}

/// Typed validation or finite-oracle failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StayerHybridError {
    /// Input field or validation phase that failed.
    pub field: &'static str,
    /// Optional zero-based stored-row index.
    pub row: Option<usize>,
    /// Human-readable failure detail.
    pub detail: String,
}

impl StayerHybridError {
    fn global(field: &'static str, detail: impl Into<String>) -> Self {
        Self {
            field,
            row: None,
            detail: detail.into(),
        }
    }

    fn at_row(field: &'static str, row: usize, detail: impl Into<String>) -> Self {
        Self {
            field,
            row: Some(row),
            detail: detail.into(),
        }
    }
}

impl fmt::Display for StayerHybridError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.row {
            Some(row) => write!(
                formatter,
                "stayer hybrid {} failure at row {}: {}",
                self.field, row, self.detail
            ),
            None => write!(
                formatter,
                "stayer hybrid {} failure: {}",
                self.field, self.detail
            ),
        }
    }
}

impl std::error::Error for StayerHybridError {}

#[derive(Clone, Copy, Debug, Default)]
struct CompensatedSum {
    sum: f64,
    correction: f64,
}

impl CompensatedSum {
    fn add(&mut self, value: f64) {
        let adjusted = value - self.correction;
        let next = self.sum + adjusted;
        self.correction = (next - self.sum) - adjusted;
        self.sum = next;
    }

    fn value(self) -> f64 {
        self.sum
    }
}

fn require_length(
    field: &'static str,
    actual: usize,
    expected: usize,
) -> Result<(), StayerHybridError> {
    if actual == expected {
        Ok(())
    } else {
        Err(StayerHybridError::global(
            field,
            format!("length {actual} does not equal expected length {expected}"),
        ))
    }
}

fn checked_controls_length(rows: usize, controls: usize) -> Result<usize, StayerHybridError> {
    rows.checked_mul(controls).ok_or_else(|| {
        StayerHybridError::global("controls", "row-by-control length overflows usize")
    })
}

/// Validate dense stayer rows and return their deterministic accounting receipt.
pub fn validate_stayer_rows(
    rows: StayerRows<'_>,
) -> Result<StayerValidationReceipt, StayerHybridError> {
    let stored_rows = rows.outcome.len();
    if stored_rows == 0 {
        return Err(StayerHybridError::global(
            "rows",
            "at least one eligible stayer row is required",
        ));
    }
    if rows.retained_firms == 0 {
        return Err(StayerHybridError::global(
            "retained_firms",
            "at least one retained mover firm is required",
        ));
    }
    if rows.stayer_workers == 0 {
        return Err(StayerHybridError::global(
            "stayer_workers",
            "at least one eligible stayer worker is required",
        ));
    }
    if rows.deletion_units == 0 {
        return Err(StayerHybridError::global(
            "deletion_units",
            "at least one stayer observation deletion unit is required",
        ));
    }

    for (field, length) in [
        ("firm", rows.firm.len()),
        ("worker", rows.worker.len()),
        ("deletion_unit", rows.deletion_unit.len()),
        ("frequency", rows.frequency.len()),
        ("target_weight", rows.target_weight.len()),
    ] {
        require_length(field, length, stored_rows)?;
    }
    if let Some(offset) = rows.offset {
        require_length("offset", offset.len(), stored_rows)?;
    }
    require_length(
        "controls",
        rows.controls.len(),
        checked_controls_length(stored_rows, rows.controls_count)?,
    )?;

    let mut worker_firm = vec![None; rows.stayer_workers];
    let mut worker_seen = vec![false; rows.stayer_workers];
    let mut firm_seen = vec![false; rows.retained_firms];
    let mut deletion_seen = vec![false; rows.deletion_units];
    let mut physical_mass = CompensatedSum::default();
    let mut target_mass = CompensatedSum::default();
    let mut maximum_frequency = 0.0_f64;

    for row in 0..stored_rows {
        let firm = rows.firm[row];
        if firm >= rows.retained_firms {
            return Err(StayerHybridError::at_row(
                "firm",
                row,
                format!("index {firm} is outside 0..{}", rows.retained_firms),
            ));
        }
        let worker = rows.worker[row];
        if worker >= rows.stayer_workers {
            return Err(StayerHybridError::at_row(
                "worker",
                row,
                format!("index {worker} is outside 0..{}", rows.stayer_workers),
            ));
        }
        let deletion_unit = rows.deletion_unit[row];
        if deletion_unit >= rows.deletion_units {
            return Err(StayerHybridError::at_row(
                "deletion_unit",
                row,
                format!(
                    "index {deletion_unit} is outside 0..{}",
                    rows.deletion_units
                ),
            ));
        }
        let outcome = rows.outcome[row];
        if !outcome.is_finite() {
            return Err(StayerHybridError::at_row(
                "outcome",
                row,
                "value must be finite",
            ));
        }
        let frequency = rows.frequency[row];
        if !frequency.is_finite()
            || frequency <= 0.0
            || frequency > MAX_EXACT_INTEGER
            || frequency != frequency.trunc()
        {
            return Err(StayerHybridError::at_row(
                "frequency",
                row,
                "value must be a positive exactly represented integer",
            ));
        }
        let target_weight = rows.target_weight[row];
        if !target_weight.is_finite() || target_weight < 0.0 {
            return Err(StayerHybridError::at_row(
                "target_weight",
                row,
                "value must be finite and nonnegative",
            ));
        }
        if let Some(offset) = rows.offset {
            if !offset[row].is_finite() {
                return Err(StayerHybridError::at_row(
                    "offset",
                    row,
                    "value must be finite",
                ));
            }
        }
        let control_start = row * rows.controls_count;
        for column in 0..rows.controls_count {
            if !rows.controls[control_start + column].is_finite() {
                return Err(StayerHybridError::at_row(
                    "controls",
                    row,
                    format!("control column {column} must be finite"),
                ));
            }
        }

        match worker_firm[worker] {
            Some(previous) if previous != firm => {
                return Err(StayerHybridError::at_row(
                    "worker",
                    row,
                    format!(
                        "worker {worker} spans firms {previous} and {firm}; only one-firm stayers are eligible"
                    ),
                ));
            }
            None => worker_firm[worker] = Some(firm),
            Some(_) => {}
        }
        worker_seen[worker] = true;
        firm_seen[firm] = true;
        deletion_seen[deletion_unit] = true;
        physical_mass.add(frequency);
        target_mass.add(target_weight);
        maximum_frequency = maximum_frequency.max(frequency);
    }

    let observed_workers = worker_seen.iter().filter(|&&seen| seen).count();
    if observed_workers != rows.stayer_workers {
        return Err(StayerHybridError::global(
            "stayer_workers",
            format!(
                "dense count declares {} workers but {} are observed",
                rows.stayer_workers, observed_workers
            ),
        ));
    }
    let observed_deletion_units = deletion_seen.iter().filter(|&&seen| seen).count();
    if observed_deletion_units != rows.deletion_units {
        return Err(StayerHybridError::global(
            "deletion_units",
            format!(
                "dense count declares {} units but {} are observed",
                rows.deletion_units, observed_deletion_units
            ),
        ));
    }
    let target_mass = target_mass.value();
    if !(target_mass.is_finite() && target_mass > 0.0) {
        return Err(StayerHybridError::global(
            "target_weight",
            "pooled stayer target mass must be finite and positive",
        ));
    }
    let physical_mass = physical_mass.value();
    if !(physical_mass.is_finite() && physical_mass > 0.0) {
        return Err(StayerHybridError::global(
            "frequency",
            "physical mass must be finite and positive",
        ));
    }

    Ok(StayerValidationReceipt {
        stored_rows,
        observed_workers,
        firms_touched: firm_seen.iter().filter(|&&seen| seen).count(),
        observed_deletion_units,
        physical_mass,
        target_mass,
        maximum_frequency,
    })
}

/// Compute an independent dense finite oracle for centered stayer point moments.
///
/// Worker effects are physical-frequency-weighted means of the outcome after
/// removing the prepared mover-firm effect, controls, and optional fixed
/// offset. Reported moments use the supplied target weights and are centered
/// over the pooled stayer target mass represented by this input block.
pub fn dense_stayer_point_oracle(
    input: StayerPointOracleInput<'_>,
) -> Result<StayerPointMoments, StayerHybridError> {
    let receipt = validate_stayer_rows(input.rows)?;
    require_length(
        "firm_effects",
        input.firm_effects.len(),
        input.rows.retained_firms,
    )?;
    require_length(
        "control_coefficients",
        input.control_coefficients.len(),
        input.rows.controls_count,
    )?;
    for (firm, &effect) in input.firm_effects.iter().enumerate() {
        if !effect.is_finite() {
            return Err(StayerHybridError::global(
                "firm_effects",
                format!("firm effect {firm} must be finite"),
            ));
        }
    }
    for (column, &coefficient) in input.control_coefficients.iter().enumerate() {
        if !coefficient.is_finite() {
            return Err(StayerHybridError::global(
                "control_coefficients",
                format!("coefficient {column} must be finite"),
            ));
        }
    }

    let mut worker_weight = vec![CompensatedSum::default(); input.rows.stayer_workers];
    let mut worker_adjusted = vec![CompensatedSum::default(); input.rows.stayer_workers];
    for row in 0..receipt.stored_rows {
        let control_start = row * input.rows.controls_count;
        let mut control_fit = CompensatedSum::default();
        for column in 0..input.rows.controls_count {
            control_fit.add(
                input.rows.controls[control_start + column] * input.control_coefficients[column],
            );
        }
        let offset = input.rows.offset.map_or(0.0, |values| values[row]);
        let adjusted = input.rows.outcome[row]
            - offset
            - input.firm_effects[input.rows.firm[row]]
            - control_fit.value();
        if !adjusted.is_finite() {
            return Err(StayerHybridError::at_row(
                "adjusted_outcome",
                row,
                "value is non-finite after removing firm, controls, and offset",
            ));
        }
        let worker = input.rows.worker[row];
        let frequency = input.rows.frequency[row];
        worker_weight[worker].add(frequency);
        worker_adjusted[worker].add(frequency * adjusted);
    }

    let mut worker_effect = vec![0.0_f64; input.rows.stayer_workers];
    for worker in 0..input.rows.stayer_workers {
        let weight = worker_weight[worker].value();
        if !(weight.is_finite() && weight > 0.0) {
            return Err(StayerHybridError::global(
                "worker_frequency",
                format!("worker {worker} has invalid physical mass {weight}"),
            ));
        }
        worker_effect[worker] = worker_adjusted[worker].value() / weight;
        if !worker_effect[worker].is_finite() {
            return Err(StayerHybridError::global(
                "worker_effect",
                format!("worker {worker} effect is non-finite"),
            ));
        }
    }

    let mut worker_mean_numerator = CompensatedSum::default();
    let mut firm_mean_numerator = CompensatedSum::default();
    for row in 0..receipt.stored_rows {
        let target = input.rows.target_weight[row];
        worker_mean_numerator.add(target * worker_effect[input.rows.worker[row]]);
        firm_mean_numerator.add(target * input.firm_effects[input.rows.firm[row]]);
    }
    let worker_mean = worker_mean_numerator.value() / receipt.target_mass;
    let firm_mean = firm_mean_numerator.value() / receipt.target_mass;

    let mut worker_variance = CompensatedSum::default();
    let mut firm_variance = CompensatedSum::default();
    let mut covariance = CompensatedSum::default();
    let mut total_variance = CompensatedSum::default();
    let total_mean = worker_mean + firm_mean;
    for row in 0..receipt.stored_rows {
        let target = input.rows.target_weight[row];
        let worker_centered = worker_effect[input.rows.worker[row]] - worker_mean;
        let firm_centered = input.firm_effects[input.rows.firm[row]] - firm_mean;
        worker_variance.add(target * worker_centered * worker_centered);
        firm_variance.add(target * firm_centered * firm_centered);
        covariance.add(target * worker_centered * firm_centered);
        let total_centered = worker_effect[input.rows.worker[row]]
            + input.firm_effects[input.rows.firm[row]]
            - total_mean;
        total_variance.add(target * total_centered * total_centered);
    }

    let worker_variance = worker_variance.value() / receipt.target_mass;
    let firm_variance = firm_variance.value() / receipt.target_mass;
    let covariance = covariance.value() / receipt.target_mass;
    let total_variance = total_variance.value() / receipt.target_mass;
    let accounting_error = total_variance - worker_variance - firm_variance - 2.0 * covariance;

    Ok(StayerPointMoments {
        target_mass: receipt.target_mass,
        worker_mean,
        firm_mean,
        worker_variance,
        firm_variance,
        covariance,
        total_variance,
        accounting_error,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exact_estimator::{run_exact_stayer_hybrid, ExactEstimatorOptions};

    fn mover_problem() -> CompressedProblem {
        CanonicalInput::from_validated(
            InputColumns {
                worker: vec![1, 1, 1, 1, 2, 2, 2, 2],
                firm: vec![1, 1, 2, 2, 1, 1, 2, 2],
                deletion: (1_u64..=8).collect(),
                outcome: vec![1.0, 2.0, 0.0, 2.5, -1.0, 1.5, 2.0, -2.0],
                frequency: vec![1; 8],
                target_weight: vec![1.0; 8],
                controls: Vec::new(),
            }
            .validate()
            .expect("mover input validates"),
        )
        .expect("mover input canonicalizes")
        .compress(&[true; 8])
        .expect("mover input compresses")
    }

    #[test]
    fn augmentation_builds_a_reconciled_mixed_deletion_problem() {
        let mover = mover_problem();
        let prepared = prepare_exact_stayer_hybrid(
            &mover,
            StayerAugmentationInput {
                firm: vec![1, 1],
                worker: vec![1, 1],
                outcome: vec![0.5, 1.25],
                frequency: vec![1, 1],
                target_weight: vec![0.0, 0.0],
                controls: Vec::new(),
            },
        )
        .expect("stayer augmentation");
        assert_eq!(prepared.receipt.mover_stored_rows, 8);
        assert_eq!(prepared.receipt.stayer_stored_rows, 2);
        assert_eq!(prepared.receipt.combined_stored_rows, 10);
        assert_eq!(prepared.receipt.mover_workers, 2);
        assert_eq!(prepared.receipt.stayer_workers, 1);
        assert_eq!(prepared.receipt.combined_workers, 3);
        assert_eq!(prepared.receipt.firms, 2);
        assert_eq!(prepared.receipt.mover_deletion_units, 8);
        assert_eq!(prepared.receipt.stayer_deletion_units, 2);
        assert_eq!(prepared.receipt.combined_deletion_units, 10);
        assert_eq!(prepared.receipt.stayer_target_mass, 0.0);
        assert_eq!(
            prepared.plan.stayer_rows,
            [vec![false; 8], vec![true; 2]].concat()
        );
        let result = run_exact_stayer_hybrid(
            &prepared.problem,
            &prepared.plan,
            ExactEstimatorOptions::default(),
        )
        .expect("mixed exact result");
        result
            .estimator
            .correction
            .verify_accounting(1.0e-10)
            .expect("combined correction accounting");
    }

    #[test]
    fn augmentation_accepts_an_explicit_zero_stayer_certificate() {
        let mover = mover_problem();
        let prepared = prepare_exact_stayer_hybrid(
            &mover,
            StayerAugmentationInput {
                firm: Vec::new(),
                worker: Vec::new(),
                outcome: Vec::new(),
                frequency: Vec::new(),
                target_weight: Vec::new(),
                controls: Vec::new(),
            },
        )
        .expect("zero-stayer augmentation");
        assert_eq!(prepared.receipt.stayer_stored_rows, 0);
        assert_eq!(prepared.receipt.combined_stored_rows, 8);
        assert_eq!(prepared.receipt.combined_deletion_units, 8);
        assert!(prepared.plan.stayer_rows.iter().all(|&value| !value));
    }

    #[test]
    fn augmentation_rejects_single_copy_and_unattached_stayers() {
        let mover = mover_problem();
        let singleton = prepare_exact_stayer_hybrid(
            &mover,
            StayerAugmentationInput {
                firm: vec![1],
                worker: vec![1],
                outcome: vec![0.5],
                frequency: vec![1],
                target_weight: vec![1.0],
                controls: Vec::new(),
            },
        )
        .expect_err("one-copy stayer must fail");
        assert_eq!(singleton.code, ErrorCode::InvalidInput);

        let unattached = prepare_exact_stayer_hybrid(
            &mover,
            StayerAugmentationInput {
                firm: vec![3],
                worker: vec![1],
                outcome: vec![0.5],
                frequency: vec![2],
                target_weight: vec![1.0],
                controls: Vec::new(),
            },
        )
        .expect_err("unattached stayer must fail");
        assert_eq!(unattached.code, ErrorCode::InvalidIdentifier);
    }

    fn rows<'a>(
        firm: &'a [usize],
        worker: &'a [usize],
        deletion_unit: &'a [usize],
        outcome: &'a [f64],
        frequency: &'a [f64],
        target_weight: &'a [f64],
        retained_firms: usize,
        stayer_workers: usize,
        deletion_units: usize,
    ) -> StayerRows<'a> {
        StayerRows {
            firm,
            worker,
            deletion_unit,
            outcome,
            frequency,
            target_weight,
            offset: None,
            controls: &[],
            controls_count: 0,
            retained_firms,
            stayer_workers,
            deletion_units,
        }
    }

    fn assert_close(actual: f64, expected: f64) {
        let scale = 1.0_f64.max(actual.abs()).max(expected.abs());
        assert!(
            (actual - expected).abs() <= 1.0e-12 * scale,
            "actual={actual:.17e}, expected={expected:.17e}"
        );
    }

    #[test]
    fn validation_rejects_a_worker_spanning_two_firms() {
        let input = rows(
            &[0, 1],
            &[0, 0],
            &[0, 1],
            &[1.0, 2.0],
            &[1.0, 1.0],
            &[1.0, 1.0],
            2,
            1,
            2,
        );
        let error = validate_stayer_rows(input).expect_err("worker must be rejected");
        assert_eq!(error.field, "worker");
        assert_eq!(error.row, Some(1));
    }

    #[test]
    fn validation_rejects_noninteger_frequency_and_zero_target_mass() {
        let noninteger = rows(&[0], &[0], &[0], &[1.0], &[1.5], &[1.0], 1, 1, 1);
        assert_eq!(
            validate_stayer_rows(noninteger)
                .expect_err("noninteger frequency must fail")
                .field,
            "frequency"
        );

        let zero_target = rows(&[0], &[0], &[0], &[1.0], &[1.0], &[0.0], 1, 1, 1);
        assert_eq!(
            validate_stayer_rows(zero_target)
                .expect_err("zero target mass must fail")
                .field,
            "target_weight"
        );
    }

    #[test]
    fn dense_oracle_preserves_centered_accounting_identity() {
        let input_rows = rows(
            &[0, 0, 1],
            &[0, 0, 1],
            &[0, 1, 2],
            &[1.0, 3.0, 6.0],
            &[1.0, 1.0, 2.0],
            &[1.0, 1.0, 2.0],
            2,
            2,
            3,
        );
        let moments = dense_stayer_point_oracle(StayerPointOracleInput {
            rows: input_rows,
            firm_effects: &[0.0, 2.0],
            control_coefficients: &[],
        })
        .expect("finite oracle");

        assert_close(moments.target_mass, 4.0);
        assert_close(moments.worker_mean, 3.0);
        assert_close(moments.firm_mean, 1.0);
        assert_close(moments.worker_variance, 1.0);
        assert_close(moments.firm_variance, 1.0);
        assert_close(moments.covariance, 1.0);
        assert_close(moments.total_variance, 4.0);
        assert_close(moments.accounting_error, 0.0);
    }

    #[test]
    fn dense_oracle_respects_frequency_controls_and_offset() {
        let input_rows = StayerRows {
            firm: &[0, 0],
            worker: &[0, 0],
            deletion_unit: &[0, 1],
            outcome: &[10.0, 20.0],
            frequency: &[3.0, 1.0],
            target_weight: &[1.0, 1.0],
            offset: Some(&[1.0, 2.0]),
            controls: &[2.0, 4.0],
            controls_count: 1,
            retained_firms: 1,
            stayer_workers: 1,
            deletion_units: 2,
        };
        let moments = dense_stayer_point_oracle(StayerPointOracleInput {
            rows: input_rows,
            firm_effects: &[3.0],
            control_coefficients: &[0.5],
        })
        .expect("finite oracle");

        // Adjusted outcomes are 5 and 13; physical-frequency mean is 7.
        assert_close(moments.worker_mean, 7.0);
        assert_close(moments.firm_mean, 3.0);
        assert_close(moments.worker_variance, 0.0);
        assert_close(moments.firm_variance, 0.0);
        assert_close(moments.covariance, 0.0);
        assert_close(moments.total_variance, 0.0);
        assert_close(moments.accounting_error, 0.0);
    }
}
