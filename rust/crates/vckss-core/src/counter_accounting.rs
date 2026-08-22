// SPDX-License-Identifier: GPL-3.0-only

//! Checked receipts for the stateless Counter-V1 address space and work.
//!
//! These counts describe logical addresses, packed 64-bit words, Bernoulli
//! trials, and optional generator evaluation work. They are deliberately not
//! described as mutable RNG state being "consumed".

use crate::error::{BackendError, ErrorCode, Result};

pub const COUNTER_ACCOUNTING_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GeneratorEvaluationModel {
    PackedWords,
    PhysicalTrials { passes: u64 },
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CounterPhaseExecutionReceipt {
    pub planned_logical_atoms: u64,
    pub actual_logical_atoms: u64,
    pub planned_unique_packed_words: u64,
    pub actual_unique_packed_words: u64,
    pub planned_physical_bernoulli_trials: u64,
    pub actual_physical_bernoulli_trials: u64,
    pub planned_generator_word_evaluations: Option<u64>,
    pub actual_generator_word_evaluations: Option<u64>,
}

impl CounterPhaseExecutionReceipt {
    #[must_use]
    pub fn completed(mut self) -> Self {
        self.actual_logical_atoms = self.planned_logical_atoms;
        self.actual_unique_packed_words = self.planned_unique_packed_words;
        self.actual_physical_bernoulli_trials = self.planned_physical_bernoulli_trials;
        self.actual_generator_word_evaluations = self.planned_generator_word_evaluations;
        self
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CounterExecutionReceipt {
    pub schema_version: u32,
    pub leverage: CounterPhaseExecutionReceipt,
    pub target: CounterPhaseExecutionReceipt,
    pub total: CounterPhaseExecutionReceipt,
}

pub fn plan_counter_phase(
    probes: u32,
    physical_count: &[u64],
    evaluations: GeneratorEvaluationModel,
) -> Result<CounterPhaseExecutionReceipt> {
    let logical_atoms_per_probe = u64::try_from(physical_count.len())
        .map_err(|_| counter_resource("semantic entity count is not representable"))?;
    plan_counter_phase_with_logical_atoms(
        probes,
        logical_atoms_per_probe,
        physical_count,
        evaluations,
    )
}

/// Plan a phase whose logical Counter address count differs from the number
/// of independently packed physical-count groups. Observation deletion is the
/// motivating case: one logical atom is addressed for every retained physical
/// observation, while packed words are grouped by equal-frequency classes.
pub fn plan_counter_phase_with_logical_atoms(
    probes: u32,
    logical_atoms_per_probe: u64,
    physical_count: &[u64],
    evaluations: GeneratorEvaluationModel,
) -> Result<CounterPhaseExecutionReceipt> {
    let probes = u64::from(probes);
    let mut words_per_probe = 0_u64;
    let mut trials_per_probe = 0_u64;
    for &trials in physical_count {
        if trials == 0 {
            return Err(BackendError::invalid(
                "counter_accounting",
                "physical Bernoulli trial counts must be positive",
            ));
        }
        words_per_probe = words_per_probe
            .checked_add(trials.div_ceil(64))
            .ok_or_else(|| counter_resource("packed Counter word count overflow"))?;
        trials_per_probe = trials_per_probe
            .checked_add(trials)
            .ok_or_else(|| counter_resource("physical Bernoulli trial count overflow"))?;
    }
    let logical = probes
        .checked_mul(logical_atoms_per_probe)
        .ok_or_else(|| counter_resource("logical Counter atom count overflow"))?;
    let words = probes
        .checked_mul(words_per_probe)
        .ok_or_else(|| counter_resource("unique packed Counter word count overflow"))?;
    let trials = probes
        .checked_mul(trials_per_probe)
        .ok_or_else(|| counter_resource("physical Bernoulli trial count overflow"))?;
    let generator = match evaluations {
        GeneratorEvaluationModel::PackedWords => words,
        GeneratorEvaluationModel::PhysicalTrials { passes } => trials
            .checked_mul(passes)
            .ok_or_else(|| counter_resource("generator evaluation work overflow"))?,
    };
    Ok(CounterPhaseExecutionReceipt {
        planned_logical_atoms: logical,
        planned_unique_packed_words: words,
        planned_physical_bernoulli_trials: trials,
        planned_generator_word_evaluations: Some(generator),
        actual_generator_word_evaluations: Some(0),
        ..CounterPhaseExecutionReceipt::default()
    })
}

pub fn combine_counter_phases(
    leverage: CounterPhaseExecutionReceipt,
    target: CounterPhaseExecutionReceipt,
) -> Result<CounterExecutionReceipt> {
    let total = CounterPhaseExecutionReceipt {
        planned_logical_atoms: checked_add(
            leverage.planned_logical_atoms,
            target.planned_logical_atoms,
            "planned logical Counter atoms",
        )?,
        actual_logical_atoms: checked_add(
            leverage.actual_logical_atoms,
            target.actual_logical_atoms,
            "actual logical Counter atoms",
        )?,
        planned_unique_packed_words: checked_add(
            leverage.planned_unique_packed_words,
            target.planned_unique_packed_words,
            "planned unique packed Counter words",
        )?,
        actual_unique_packed_words: checked_add(
            leverage.actual_unique_packed_words,
            target.actual_unique_packed_words,
            "actual unique packed Counter words",
        )?,
        planned_physical_bernoulli_trials: checked_add(
            leverage.planned_physical_bernoulli_trials,
            target.planned_physical_bernoulli_trials,
            "planned physical Bernoulli trials",
        )?,
        actual_physical_bernoulli_trials: checked_add(
            leverage.actual_physical_bernoulli_trials,
            target.actual_physical_bernoulli_trials,
            "actual physical Bernoulli trials",
        )?,
        planned_generator_word_evaluations: checked_optional_add(
            leverage.planned_generator_word_evaluations,
            target.planned_generator_word_evaluations,
            "planned generator evaluations",
        )?,
        actual_generator_word_evaluations: checked_optional_add(
            leverage.actual_generator_word_evaluations,
            target.actual_generator_word_evaluations,
            "actual generator evaluations",
        )?,
    };
    Ok(CounterExecutionReceipt {
        schema_version: COUNTER_ACCOUNTING_SCHEMA_VERSION,
        leverage,
        target,
        total,
    })
}

fn checked_optional_add(left: Option<u64>, right: Option<u64>, label: &str) -> Result<Option<u64>> {
    match (left, right) {
        (Some(left), Some(right)) => checked_add(left, right, label).map(Some),
        _ => Ok(None),
    }
}

fn checked_add(left: u64, right: u64, label: &str) -> Result<u64> {
    left.checked_add(right)
        .ok_or_else(|| counter_resource(&format!("{label} overflow")))
}

fn counter_resource(message: &str) -> BackendError {
    BackendError::new(ErrorCode::ResourceLimit, "counter_accounting", message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packed_and_recomputed_models_have_distinct_checked_counts() {
        let packed = plan_counter_phase(3, &[1, 64, 65], GeneratorEvaluationModel::PackedWords)
            .expect("packed plan");
        assert_eq!(packed.planned_logical_atoms, 9);
        assert_eq!(packed.planned_unique_packed_words, 12);
        assert_eq!(packed.planned_physical_bernoulli_trials, 390);
        assert_eq!(packed.planned_generator_word_evaluations, Some(12));
        let repeated = plan_counter_phase(
            3,
            &[1, 64, 65],
            GeneratorEvaluationModel::PhysicalTrials { passes: 2 },
        )
        .expect("recomputed plan");
        assert_eq!(repeated.planned_generator_word_evaluations, Some(780));

        let observation = plan_counter_phase_with_logical_atoms(
            3,
            130,
            &[1, 64, 65],
            GeneratorEvaluationModel::PhysicalTrials { passes: 2 },
        )
        .expect("observation plan");
        assert_eq!(observation.planned_logical_atoms, 390);
        assert_eq!(observation.planned_unique_packed_words, 12);
        assert_eq!(observation.planned_physical_bernoulli_trials, 390);
    }

    #[test]
    fn zero_counts_and_overflow_are_typed() {
        let zero = plan_counter_phase(2, &[0], GeneratorEvaluationModel::PackedWords)
            .expect_err("zero trial count rejects");
        assert_eq!(zero.code, ErrorCode::InvalidInput);
        let overflow = plan_counter_phase(
            u32::MAX,
            &[u64::MAX, 1],
            GeneratorEvaluationModel::PackedWords,
        )
        .expect_err("trial sum overflow rejects");
        assert_eq!(overflow.code, ErrorCode::ResourceLimit);
    }
}
