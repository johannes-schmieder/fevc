//! Source-bound building blocks for the exact mover-plus-stayer hybrid.
//!
//! This module begins with validation and an independent finite point-target
//! oracle.  It deliberately does not expose a production correction route yet.
//! The correction layer must be qualified against the existing Mata hybrid
//! before it can be wired into the public ABI.

use core::fmt;

/// Largest integer that is represented exactly by an IEEE-754 `f64`.
const MAX_EXACT_INTEGER: f64 = 9_007_199_254_740_992.0;

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
                input.rows.controls[control_start + column]
                    * input.control_coefficients[column],
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
    let accounting_error =
        total_variance - worker_variance - firm_variance - 2.0 * covariance;

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
        let noninteger = rows(
            &[0],
            &[0],
            &[0],
            &[1.0],
            &[1.5],
            &[1.0],
            1,
            1,
            1,
        );
        assert_eq!(
            validate_stayer_rows(noninteger)
                .expect_err("noninteger frequency must fail")
                .field,
            "frequency"
        );

        let zero_target = rows(
            &[0],
            &[0],
            &[0],
            &[1.0],
            &[1.0],
            &[0.0],
            1,
            1,
            1,
        );
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
