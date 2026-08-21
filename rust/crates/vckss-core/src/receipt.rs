// SPDX-License-Identifier: GPL-3.0-only

use std::time::{Duration, Instant};

use crate::error::{BackendError, Result};
use crate::types::Dimensions;

#[derive(Clone, Debug, Default)]
pub struct PhaseTimings {
    pub ingest: Duration,
    pub canonicalize: Duration,
    pub graph: Duration,
    pub compress: Duration,
    pub preconditioner_setup: Duration,
    pub solve: Duration,
    pub correction: Duration,
    pub export: Duration,
}

#[derive(Clone, Debug, Default)]
pub struct MemoryReceipt {
    pub hard_limit_bytes: u64,
    pub persistent_bytes: u64,
    pub phase_bytes: u64,
    pub batch_bytes: u64,
    pub thread_bytes: u64,
    pub reserve_bytes: u64,
}

impl MemoryReceipt {
    pub fn checked_peak(&self) -> Result<u64> {
        [
            self.persistent_bytes,
            self.phase_bytes,
            self.batch_bytes,
            self.thread_bytes,
            self.reserve_bytes,
        ]
        .into_iter()
        .try_fold(0_u64, |total, value| {
            total
                .checked_add(value)
                .ok_or_else(|| BackendError::invariant("receipt", "memory receipt overflow"))
        })
    }
}

#[derive(Clone, Debug, Default)]
pub struct RunReceipt {
    pub backend_version: String,
    pub build_commit: String,
    pub dimensions: Dimensions,
    pub threads_requested: u32,
    pub threads_used: u32,
    pub timings: PhaseTimings,
    pub memory: MemoryReceipt,
    pub topology_checksum: u64,
}

#[derive(Debug)]
pub struct PhaseTimer {
    start: Instant,
}

impl PhaseTimer {
    #[must_use]
    pub fn start() -> Self {
        Self {
            start: Instant::now(),
        }
    }

    #[must_use]
    pub fn elapsed(self) -> Duration {
        self.start.elapsed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn peak_is_checked_sum() {
        let receipt = MemoryReceipt {
            hard_limit_bytes: 100,
            persistent_bytes: 10,
            phase_bytes: 20,
            batch_bytes: 30,
            thread_bytes: 4,
            reserve_bytes: 5,
        };
        assert_eq!(receipt.checked_peak().expect("peak"), 69);
    }
}
