// SPDX-License-Identifier: GPL-3.0-only

//! Rust numerical backend for `varcomp_kss`.
//!
//! The crate is deliberately independent of Stata's ABI. The plugin crate owns
//! the caller-thread-only Stata bridge; this crate owns validated native data,
//! deterministic parallelism, graph and numerical algorithms, and receipts.

pub mod error;
pub mod parallel;
pub mod receipt;
pub mod types;

use error::Result;
use parallel::{compensated_sum, DeterministicExecutor};
use types::{BackendOptions, InputColumns};

pub const BACKEND_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const ABI_VERSION: u32 = 1;
pub const NUMERICAL_CONTRACT: &str = "VCKSS-RUST-NUMERICAL-V1";
pub const RECEIPT_SCHEMA: &str = "VCKSS-RUST-RECEIPT-V1";
pub const COUNTER_RNG_CONTRACT: &str = "VCKSS-COUNTER-V1";
pub const CMG_BASELINE: &str = "VCKSS-CMG-API7-IMPROVED-BASELINE";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Capabilities {
    pub abi_version: u32,
    pub supports_exact: bool,
    pub supports_jla: bool,
    pub supports_match_deletion: bool,
    pub supports_observation_deletion: bool,
    pub supports_controls: bool,
    pub supports_diagonal: bool,
    pub supports_cmg: bool,
    pub deterministic_parallelism: bool,
}

impl Capabilities {
    #[must_use]
    pub const fn current() -> Self {
        Self {
            abi_version: ABI_VERSION,
            supports_exact: false,
            supports_jla: false,
            supports_match_deletion: false,
            supports_observation_deletion: false,
            supports_controls: false,
            supports_diagonal: false,
            supports_cmg: false,
            deterministic_parallelism: true,
        }
    }

    #[must_use]
    pub fn to_json(self) -> String {
        format!(
            concat!(
                "{{\"abi_version\":{},",
                "\"backend_version\":\"{}\",",
                "\"numerical_contract\":\"{}\",",
                "\"receipt_schema\":\"{}\",",
                "\"cmg_baseline\":\"{}\",",
                "\"supports_exact\":{},",
                "\"supports_jla\":{},",
                "\"supports_match_deletion\":{},",
                "\"supports_observation_deletion\":{},",
                "\"supports_controls\":{},",
                "\"supports_diagonal\":{},",
                "\"supports_cmg\":{},",
                "\"deterministic_parallelism\":{}}}"
            ),
            self.abi_version,
            BACKEND_VERSION,
            NUMERICAL_CONTRACT,
            RECEIPT_SCHEMA,
            CMG_BASELINE,
            self.supports_exact,
            self.supports_jla,
            self.supports_match_deletion,
            self.supports_observation_deletion,
            self.supports_controls,
            self.supports_diagonal,
            self.supports_cmg,
            self.deterministic_parallelism,
        )
    }
}

pub fn selftest() -> Result<()> {
    BackendOptions::default().validate()?;
    let input = InputColumns {
        worker: vec![1, 1, 2, 2],
        firm: vec![1, 2, 1, 2],
        deletion: vec![1, 2, 3, 4],
        outcome: vec![1.0, 2.0, 3.0, 4.0],
        frequency: vec![1, 1, 1, 1],
        target_weight: vec![1.0, 1.0, 1.0, 1.0],
        controls: Vec::new(),
    }
    .validate()?;
    let executor = DeterministicExecutor::new(2)?;
    let partial = executor.map_partitions(input.rows(), |range| {
        Ok(compensated_sum(&input.columns.outcome[range]))
    })?;
    let total = compensated_sum(&partial);
    if total != 10.0 {
        return Err(error::BackendError::invariant(
            "selftest",
            "deterministic parallel sum failed",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_json_is_stable_and_valid_shape() {
        let json = Capabilities::current().to_json();
        assert!(json.starts_with('{'));
        assert!(json.ends_with('}'));
        assert!(json.contains("\"abi_version\":1"));
        assert!(json.contains(CMG_BASELINE));
    }

    #[test]
    fn backend_selftest_passes() {
        selftest().expect("selftest");
    }
}
