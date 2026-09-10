// SPDX-License-Identifier: GPL-3.0-only

//! Optional command budgets, separate from forecasts and allocation failures.

use crate::error::{BackendError, ErrorCode, Result};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum MemoryCheck {
    #[default]
    Error,
    Warn,
    Off,
}

/// Legacy entrypoints retain their explicitly supplied numeric limit. New
/// callers represent an omitted budget with `Unspecified`, never a large cap.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum MemoryBudget {
    #[default]
    Legacy,
    Unspecified,
    Explicit {
        bytes: u64,
        check: MemoryCheck,
    },
}

impl MemoryBudget {
    pub fn validate(self, legacy_limit: u64) -> Result<()> {
        if self.limit(legacy_limit) == Some(0) {
            return Err(BackendError::invalid(
                "memory_budget",
                "an explicit memory budget must be positive",
            ));
        }
        Ok(())
    }

    #[must_use]
    pub const fn limit(self, legacy_limit: u64) -> Option<u64> {
        match self {
            Self::Legacy => Some(legacy_limit),
            Self::Unspecified => None,
            Self::Explicit { bytes, .. } => Some(bytes),
        }
    }

    #[must_use]
    pub const fn check(self) -> MemoryCheck {
        match self {
            Self::Legacy => MemoryCheck::Error,
            Self::Unspecified => MemoryCheck::Off,
            Self::Explicit { check, .. } => check,
        }
    }

    #[must_use]
    pub fn fits(self, forecast: u64, legacy_limit: u64) -> bool {
        self.limit(legacy_limit)
            .is_none_or(|limit| forecast <= limit)
    }

    #[must_use]
    pub fn rejects(self, forecast: u64, legacy_limit: u64) -> bool {
        self.check() == MemoryCheck::Error && !self.fits(forecast, legacy_limit)
    }

    pub fn admit(self, forecast: u64, legacy_limit: u64, phase: &'static str) -> Result<()> {
        self.validate(legacy_limit)?;
        if self.rejects(forecast, legacy_limit) {
            return Err(BackendError::new(
                ErrorCode::ResourceLimit,
                phase,
                format!(
                    "allocation forecast {forecast} bytes exceeds the declared budget {} bytes",
                    self.limit(legacy_limit).expect("rejecting budget exists")
                ),
            ));
        }
        Ok(())
    }

    /// Zero is only the unused legacy wire field; the versioned policy
    /// receipt independently represents budget presence.
    #[must_use]
    pub fn hard_limit(self, legacy_limit: u64) -> Option<u64> {
        if self.check() == MemoryCheck::Error {
            self.limit(legacy_limit)
        } else {
            None
        }
    }

    #[must_use]
    pub fn receipt_limit(self, legacy_limit: u64) -> u64 {
        self.limit(legacy_limit).unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absence_and_advisory_budgets_do_not_reject() {
        let omitted = MemoryBudget::Unspecified;
        assert_eq!(omitted.limit(4 << 30), None);
        assert!(omitted.fits(u64::MAX, 1));
        assert!(omitted.admit(u64::MAX, 0, "test").is_ok());
        for check in [MemoryCheck::Warn, MemoryCheck::Off] {
            let budget = MemoryBudget::Explicit { bytes: 1, check };
            assert!(!budget.fits(2, 100));
            assert!(!budget.rejects(2, 100));
        }
    }

    #[test]
    fn explicit_and_legacy_strict_boundaries_are_preserved() {
        for budget in [
            MemoryBudget::Legacy,
            MemoryBudget::Explicit {
                bytes: 10,
                check: MemoryCheck::Error,
            },
        ] {
            assert!(budget.admit(10, 10, "test").is_ok());
            assert_eq!(
                budget.admit(11, 10, "test").unwrap_err().code,
                ErrorCode::ResourceLimit
            );
        }
        assert!(MemoryBudget::Explicit {
            bytes: 0,
            check: MemoryCheck::Off
        }
        .validate(10)
        .is_err());
    }
}
