// SPDX-License-Identifier: GPL-3.0-only

use crate::error::{BackendError, ErrorCode, Result};

pub const MAX_EXACT_BINARY64_INTEGER: u64 = 1_u64 << 53;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Algorithm {
    Auto,
    Exact,
    Jla,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Preconditioner {
    Auto,
    Diagonal,
    Cmg,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeletionMode {
    Match,
    Observation,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NuisanceMode {
    Joint,
    FixedOffset,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RngContract {
    StataCompatibility,
    CounterV1,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Determinism {
    Strict,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Dimensions {
    pub rows_stored: u64,
    pub rows_physical: u64,
    pub workers: u64,
    pub firms: u64,
    pub cells: u64,
    pub deletion_units: u64,
    pub target_strata: u64,
    pub controls: u32,
}

impl Dimensions {
    pub fn checked_parameter_count(self) -> Result<u64> {
        self.workers
            .checked_add(self.firms)
            .and_then(|value| value.checked_sub(1))
            .and_then(|value| value.checked_add(u64::from(self.controls)))
            .ok_or_else(|| {
                BackendError::new(
                    ErrorCode::ResourceLimit,
                    "dimensions",
                    "parameter count overflow",
                )
            })
    }
}

#[derive(Clone, Copy, Debug)]
pub struct BackendOptions {
    pub algorithm: Algorithm,
    pub preconditioner: Preconditioner,
    pub deletion: DeletionMode,
    pub nuisance: NuisanceMode,
    pub rng: RngContract,
    pub determinism: Determinism,
    pub probes: u32,
    pub batch_width: u32,
    pub threads: u32,
    pub tolerance: f64,
    pub max_iterations: u32,
    pub exact_limit: u32,
    pub memory_limit_bytes: u64,
}

impl Default for BackendOptions {
    fn default() -> Self {
        Self {
            algorithm: Algorithm::Auto,
            preconditioner: Preconditioner::Auto,
            deletion: DeletionMode::Match,
            nuisance: NuisanceMode::Joint,
            rng: RngContract::CounterV1,
            determinism: Determinism::Strict,
            probes: 200,
            batch_width: 8,
            threads: 1,
            tolerance: 1.0e-10,
            max_iterations: 10_000,
            exact_limit: 500,
            memory_limit_bytes: 4_u64 << 30,
        }
    }
}

impl BackendOptions {
    pub fn validate(self) -> Result<Self> {
        if self.probes < 2 {
            return Err(BackendError::invalid(
                "options",
                "probes must be at least two",
            ));
        }
        if self.batch_width == 0 {
            return Err(BackendError::invalid(
                "options",
                "batch width must be positive",
            ));
        }
        if self.threads == 0 {
            return Err(BackendError::invalid(
                "options",
                "thread count must be positive",
            ));
        }
        if !(1.0e-15..=1.0e-4).contains(&self.tolerance) || !self.tolerance.is_finite() {
            return Err(BackendError::invalid(
                "options",
                "tolerance must be finite and lie in [1e-15, 1e-4]",
            ));
        }
        if self.max_iterations == 0 {
            return Err(BackendError::invalid(
                "options",
                "maximum iterations must be positive",
            ));
        }
        if !(2..=2_000).contains(&self.exact_limit) {
            return Err(BackendError::invalid(
                "options",
                "exact limit must lie in [2, 2000]",
            ));
        }
        if self.memory_limit_bytes == 0 {
            return Err(BackendError::invalid(
                "options",
                "memory limit must be positive",
            ));
        }
        Ok(self)
    }

    #[must_use]
    pub fn full_residual_tolerance(self) -> f64 {
        (10.0 * self.tolerance).max(1.0e-11)
    }
}

#[derive(Clone, Debug)]
pub struct InputColumns {
    pub worker: Vec<u64>,
    pub firm: Vec<u64>,
    pub deletion: Vec<u64>,
    pub outcome: Vec<f64>,
    pub frequency: Vec<u64>,
    pub target_weight: Vec<f64>,
    pub controls: Vec<Vec<f64>>,
}

impl InputColumns {
    pub fn validate(self) -> Result<ValidatedInput> {
        let n = self.worker.len();
        if n == 0 {
            return Err(BackendError::invalid("ingest", "input sample is empty"));
        }
        let equal_lengths = self.firm.len() == n
            && self.deletion.len() == n
            && self.outcome.len() == n
            && self.frequency.len() == n
            && self.target_weight.len() == n
            && self.controls.iter().all(|column| column.len() == n);
        if !equal_lengths {
            return Err(BackendError::invalid(
                "ingest",
                "input columns have inconsistent lengths",
            ));
        }

        let mut physical_total = 0_u64;
        let mut target_total = 0.0_f64;
        for row in 0..n {
            let outcome = self.outcome[row];
            let target = self.target_weight[row];
            let frequency = self.frequency[row];
            if !outcome.is_finite() {
                return Err(BackendError::invalid(
                    "ingest",
                    format!("outcome is nonfinite at zero-based row {row}"),
                ));
            }
            if frequency == 0 {
                return Err(BackendError::new(
                    ErrorCode::InvalidWeight,
                    "ingest",
                    format!("frequency is zero at zero-based row {row}"),
                ));
            }
            if !target.is_finite() || target < 0.0 {
                return Err(BackendError::new(
                    ErrorCode::InvalidTargetWeight,
                    "ingest",
                    format!("target weight is invalid at zero-based row {row}"),
                ));
            }
            if self.controls.iter().any(|column| !column[row].is_finite()) {
                return Err(BackendError::invalid(
                    "ingest",
                    format!("control is nonfinite at zero-based row {row}"),
                ));
            }
            physical_total = physical_total.checked_add(frequency).ok_or_else(|| {
                BackendError::new(
                    ErrorCode::ResourceLimit,
                    "ingest",
                    "physical-frequency total overflow",
                )
            })?;
            if physical_total > MAX_EXACT_BINARY64_INTEGER {
                return Err(BackendError::new(
                    ErrorCode::ResourceLimit,
                    "ingest",
                    "literal physical-frequency total exceeds the exact binary64 integer range",
                ));
            }
            target_total += target;
        }
        if !target_total.is_finite() || target_total <= 0.0 {
            return Err(BackendError::new(
                ErrorCode::InvalidTargetWeight,
                "ingest",
                "target weights must have positive finite total mass",
            ));
        }

        Ok(ValidatedInput {
            columns: self,
            physical_total,
            target_total,
        })
    }
}

#[derive(Clone, Debug)]
pub struct ValidatedInput {
    pub columns: InputColumns,
    pub physical_total: u64,
    pub target_total: f64,
}

impl ValidatedInput {
    #[must_use]
    pub fn rows(&self) -> usize {
        self.columns.worker.len()
    }

    #[must_use]
    pub fn controls(&self) -> usize {
        self.columns.controls.len()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MemoryBudget {
    limit: u64,
    committed: u64,
}

impl MemoryBudget {
    pub fn new(limit: u64) -> Result<Self> {
        if limit == 0 {
            return Err(BackendError::invalid(
                "resource",
                "memory budget must be positive",
            ));
        }
        Ok(Self {
            limit,
            committed: 0,
        })
    }

    pub fn reserve(&mut self, bytes: u64, label: &str) -> Result<()> {
        let next = self.committed.checked_add(bytes).ok_or_else(|| {
            BackendError::new(
                ErrorCode::ResourceLimit,
                "resource",
                format!("memory accounting overflow while reserving {label}"),
            )
        })?;
        if next > self.limit {
            return Err(BackendError::new(
                ErrorCode::ResourceLimit,
                "resource",
                format!(
                    "reservation for {label} would require {next} bytes, above the {}-byte limit",
                    self.limit
                ),
            ));
        }
        self.committed = next;
        Ok(())
    }

    pub fn release(&mut self, bytes: u64) -> Result<()> {
        self.committed = self
            .committed
            .checked_sub(bytes)
            .ok_or_else(|| BackendError::invariant("resource", "memory receipt underflow"))?;
        Ok(())
    }

    #[must_use]
    pub const fn committed(self) -> u64 {
        self.committed
    }

    #[must_use]
    pub const fn available(self) -> u64 {
        self.limit - self.committed
    }
}

pub fn bytes_for<T>(length: u64) -> Result<u64> {
    length
        .checked_mul(u64::try_from(core::mem::size_of::<T>()).map_err(|_| {
            BackendError::new(
                ErrorCode::ResourceLimit,
                "resource",
                "element size is not representable as u64",
            )
        })?)
        .ok_or_else(|| {
            BackendError::new(
                ErrorCode::ResourceLimit,
                "resource",
                "allocation byte count overflow",
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn options_reject_zero_threads() {
        let options = BackendOptions {
            threads: 0,
            ..BackendOptions::default()
        };
        assert_eq!(
            options.validate().expect_err("zero threads must fail").code,
            ErrorCode::InvalidInput
        );
    }

    #[test]
    fn input_validation_counts_physical_mass() {
        let input = InputColumns {
            worker: vec![10, 10],
            firm: vec![20, 21],
            deletion: vec![1, 2],
            outcome: vec![1.0, 2.0],
            frequency: vec![3, 4],
            target_weight: vec![3.0, 4.0],
            controls: Vec::new(),
        }
        .validate()
        .expect("valid input");
        assert_eq!(input.physical_total, 7);
        assert_eq!(input.rows(), 2);
    }

    #[test]
    fn physical_total_above_binary64_limit_is_rejected() {
        let error = InputColumns {
            worker: vec![1, 2],
            firm: vec![1, 2],
            deletion: vec![1, 2],
            outcome: vec![0.0, 0.0],
            frequency: vec![MAX_EXACT_BINARY64_INTEGER, 1],
            target_weight: vec![1.0, 1.0],
            controls: Vec::new(),
        }
        .validate()
        .expect_err("over-limit physical mass must fail");
        assert_eq!(error.code, ErrorCode::ResourceLimit);
    }

    #[test]
    fn memory_budget_fails_before_overcommit() {
        let mut budget = MemoryBudget::new(100).expect("budget");
        budget.reserve(60, "a").expect("first reservation");
        assert_eq!(budget.available(), 40);
        assert_eq!(
            budget.reserve(41, "b").expect_err("must fail").code,
            ErrorCode::ResourceLimit
        );
        assert_eq!(budget.committed(), 60);
    }
}
