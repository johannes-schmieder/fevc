// SPDX-License-Identifier: GPL-3.0-only

//! Frozen counter-based random-number contract for Rust probe generation.
//!
//! `VCKSS-COUNTER-V1` uses Philox4x64-10. Logical randomness is addressed by
//! `(seed, domain, probe, canonical entity, physical-word index)`. Four
//! consecutive probes occupy the four lanes of one Philox block, so batching
//! and thread scheduling cannot alter any atom.

use crate::error::{BackendError, ErrorCode, Result};
use crate::types::MAX_EXACT_BINARY64_INTEGER;

const PHILOX_MULTIPLIER_0: u64 = 0xd2b7_4407_b1ce_6e93;
const PHILOX_MULTIPLIER_1: u64 = 0xca5a_8263_9512_1157;
const PHILOX_WEYL_0: u64 = 0x9e37_79b9_7f4a_7c15;
const PHILOX_WEYL_1: u64 = 0xbb67_ae85_84ca_a73b;
const KEY_DERIVATION_XOR: u64 = 0xd1b5_4a32_d192_ed03;
const PHILOX_ROUNDS: usize = 10;
pub const MAX_PHYSICAL_WORDS_PER_ATOM: u64 = 1_u64 << 20;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProbeDomain {
    Leverage,
    Target,
    Diagnostic,
    Retry,
    SelfTest,
}

impl ProbeDomain {
    #[must_use]
    pub const fn tag(self) -> u64 {
        match self {
            Self::Leverage => 0x4c45_5645_5241_4745,
            Self::Target => 0x0054_4152_4745_5401,
            Self::Diagnostic => 0x4449_4147_4e4f_5354,
            Self::Retry => 0x0052_4554_5259_0001,
            Self::SelfTest => 0x5345_4c46_5445_5354,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CounterRng {
    seed: u64,
}

impl CounterRng {
    #[must_use]
    pub const fn new(seed: u64) -> Self {
        Self { seed }
    }

    #[must_use]
    pub const fn seed(self) -> u64 {
        self.seed
    }

    #[must_use]
    pub fn raw_block(
        self,
        domain: ProbeDomain,
        probe_block: u64,
        entity: u64,
        word_index: u64,
    ) -> [u64; 4] {
        let mut counter = [entity, probe_block, word_index, domain.tag()];
        let mut key = [
            splitmix64(self.seed),
            splitmix64(self.seed ^ KEY_DERIVATION_XOR),
        ];
        for round in 0..PHILOX_ROUNDS {
            counter = philox_round(counter, key);
            if round + 1 != PHILOX_ROUNDS {
                key[0] = key[0].wrapping_add(PHILOX_WEYL_0);
                key[1] = key[1].wrapping_add(PHILOX_WEYL_1);
            }
        }
        counter
    }

    #[must_use]
    pub fn word(
        self,
        domain: ProbeDomain,
        probe: u64,
        entity: u64,
        word_index: u64,
    ) -> u64 {
        let lane = usize::try_from(probe % 4).expect("Philox lane is in 0..4");
        self.raw_block(domain, probe / 4, entity, word_index)[lane]
    }

    #[must_use]
    pub fn rademacher(
        self,
        domain: ProbeDomain,
        probe: u64,
        entity: u64,
        subdraw: u64,
    ) -> i8 {
        if self.word(domain, probe, entity, subdraw) & 1 == 0 {
            -1
        } else {
            1
        }
    }

    pub fn rademacher_sum(
        self,
        domain: ProbeDomain,
        probe: u64,
        entity: u64,
        trials: u64,
    ) -> Result<i64> {
        if trials == 0 || trials > MAX_EXACT_BINARY64_INTEGER {
            return Err(BackendError::new(
                ErrorCode::RngContractFailed,
                "counter_rng",
                "Rademacher trial count must lie in [1, 2^53]",
            ));
        }
        let words = trials.div_ceil(64);
        if words > MAX_PHYSICAL_WORDS_PER_ATOM {
            return Err(BackendError::new(
                ErrorCode::ResourceLimit,
                "counter_rng",
                format!(
                    "one semantic atom requires {words} physical random words, above the registered limit {MAX_PHYSICAL_WORDS_PER_ATOM}"
                ),
            ));
        }

        let complete_words = trials / 64;
        let remainder = trials % 64;
        let mut positive = 0_u64;
        for word_index in 0..complete_words {
            positive = positive
                .checked_add(u64::from(
                    self.word(domain, probe, entity, word_index).count_ones(),
                ))
                .ok_or_else(|| rng_resource_error("Rademacher positive-count overflow"))?;
        }
        if remainder > 0 {
            let shift = u32::try_from(remainder).expect("remainder is below 64");
            let mask = (1_u64 << shift) - 1;
            positive = positive
                .checked_add(u64::from(
                    (self.word(domain, probe, entity, complete_words) & mask).count_ones(),
                ))
                .ok_or_else(|| rng_resource_error("Rademacher positive-count overflow"))?;
        }
        let doubled = positive
            .checked_mul(2)
            .ok_or_else(|| rng_resource_error("Rademacher signed-count overflow"))?;
        let doubled = i64::try_from(doubled)
            .map_err(|_| rng_resource_error("Rademacher signed count exceeds i64"))?;
        let trials = i64::try_from(trials)
            .map_err(|_| rng_resource_error("Rademacher trial count exceeds i64"))?;
        Ok(doubled - trials)
    }

    pub fn fill_rademacher_sums(
        self,
        domain: ProbeDomain,
        first_probe: u64,
        columns: usize,
        entity: &[u64],
        trials: &[u64],
        output: &mut [i64],
    ) -> Result<()> {
        if columns == 0 || entity.is_empty() || entity.len() != trials.len() {
            return Err(BackendError::invalid(
                "counter_rng",
                "semantic atom request has incompatible dimensions",
            ));
        }
        let expected = entity
            .len()
            .checked_mul(columns)
            .ok_or_else(|| rng_resource_error("semantic atom matrix length overflow"))?;
        if output.len() != expected {
            return Err(BackendError::invalid(
                "counter_rng",
                "semantic atom output has incompatible dimensions",
            ));
        }
        for (row, (&entity_key, &trial_count)) in entity.iter().zip(trials).enumerate() {
            for column in 0..columns {
                let probe = first_probe
                    .checked_add(u64::try_from(column).map_err(|_| {
                        rng_resource_error("probe column is not representable as u64")
                    })?)
                    .ok_or_else(|| rng_resource_error("logical probe index overflow"))?;
                output[column * entity.len() + row] =
                    self.rademacher_sum(domain, probe, entity_key, trial_count)?;
            }
        }
        Ok(())
    }
}

#[must_use]
fn splitmix64(input: u64) -> u64 {
    let mut value = input.wrapping_add(0x9e37_79b9_7f4a_7c15);
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

#[must_use]
fn philox_round(counter: [u64; 4], key: [u64; 2]) -> [u64; 4] {
    let (high_0, low_0) = multiply_high_low(PHILOX_MULTIPLIER_0, counter[0]);
    let (high_1, low_1) = multiply_high_low(PHILOX_MULTIPLIER_1, counter[2]);
    [
        high_1 ^ counter[1] ^ key[0],
        low_1,
        high_0 ^ counter[3] ^ key[1],
        low_0,
    ]
}

#[must_use]
fn multiply_high_low(left: u64, right: u64) -> (u64, u64) {
    let product = u128::from(left) * u128::from(right);
    let high = u64::try_from(product >> 64).expect("upper product half fits u64");
    let low = u64::try_from(product & u128::from(u64::MAX))
        .expect("lower product half fits u64");
    (high, low)
}

fn rng_resource_error(message: &str) -> BackendError {
    BackendError::new(ErrorCode::ResourceLimit, "counter_rng", message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn philox_block_matches_permanent_contract_vector() {
        assert_eq!(
            CounterRng::new(123_456_789).raw_block(
                ProbeDomain::Leverage,
                17,
                99,
                3,
            ),
            [
                0x9f4b_6ea9_0b26_b57d,
                0x529b_58b9_6148_9b78,
                0x9d84_fab1_1898_a6a0,
                0x4694_5983_0d83_6d27,
            ]
        );
    }

    #[test]
    fn logical_word_and_domain_vectors_are_frozen() {
        let rng = CounterRng::new(123);
        assert_eq!(
            rng.word(ProbeDomain::Leverage, 5, 7, 0),
            0x40ce_71bf_068d_1e52
        );
        assert_eq!(
            rng.word(ProbeDomain::Target, 5, 7, 0),
            0x46e2_bf28_fab1_ea2c
        );
    }

    #[test]
    fn compressed_sums_match_permanent_vectors() {
        let rng = CounterRng::new(123);
        let expected = [(1, -1), (5, -1), (20, 0), (64, -4), (65, -5), (100, 4)];
        for (trials, sum) in expected {
            assert_eq!(
                rng.rademacher_sum(ProbeDomain::Leverage, 5, 7, trials)
                    .expect("sum"),
                sum
            );
        }
    }

    #[test]
    fn batches_do_not_change_logical_atoms() {
        let rng = CounterRng::new(987_654_321);
        let entity = [3, 11, 29];
        let trials = [1, 20, 65];
        let mut complete = vec![0_i64; entity.len() * 11];
        rng.fill_rademacher_sums(
            ProbeDomain::Target,
            0,
            11,
            &entity,
            &trials,
            &mut complete,
        )
        .expect("complete batch");

        let first_columns = 4;
        let mut split = vec![0_i64; complete.len()];
        rng.fill_rademacher_sums(
            ProbeDomain::Target,
            0,
            first_columns,
            &entity,
            &trials,
            &mut split[..entity.len() * first_columns],
        )
        .expect("first batch");
        rng.fill_rademacher_sums(
            ProbeDomain::Target,
            u64::try_from(first_columns).expect("column count"),
            11 - first_columns,
            &entity,
            &trials,
            &mut split[entity.len() * first_columns..],
        )
        .expect("second batch");
        assert_eq!(complete, split);
    }

    #[test]
    fn zero_trials_are_rejected() {
        let error = CounterRng::new(1)
            .rademacher_sum(ProbeDomain::Leverage, 0, 0, 0)
            .expect_err("zero trials must fail");
        assert_eq!(error.code, ErrorCode::RngContractFailed);
    }
}
