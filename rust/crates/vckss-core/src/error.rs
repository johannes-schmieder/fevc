// SPDX-License-Identifier: GPL-3.0-only

use core::fmt;

/// Stable error classes crossing the Rust/Stata boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i32)]
pub enum ErrorCode {
    Ok = 0,
    UserBreak = 1,
    AbiMismatch = 10,
    UnsupportedFeature = 11,
    InvalidInput = 20,
    InvalidIdentifier = 21,
    InvalidWeight = 22,
    InvalidTargetWeight = 23,
    GraphEmpty = 30,
    GraphUnidentified = 31,
    GraphCertificateFailed = 32,
    ResourceLimit = 40,
    AllocationFailed = 41,
    CmgSetupFailed = 50,
    CmgApplyFailed = 51,
    PcgCurvatureBreakdown = 60,
    PcgPreconditionerBreakdown = 61,
    PcgStagnation = 62,
    PcgMaxIterations = 63,
    FullResidualFailed = 64,
    RngContractFailed = 70,
    CorrectionNonFinite = 71,
    AccountingIdentityFailed = 72,
    JlaMomentFailed = 73,
    JlaConstraintFailed = 74,
    NonestimableDeletion = 75,
    BlockInverseFailed = 76,
    TargetCenteringFailed = 77,
    TargetIdentityFailed = 78,
    NonfiniteCorrectedTarget = 79,
    StaleContext = 80,
    ContextPoisoned = 81,
    InternalInvariantFailed = 90,
    Panic = 99,
}

impl ErrorCode {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "OK",
            Self::UserBreak => "USER_BREAK",
            Self::AbiMismatch => "ABI_MISMATCH",
            Self::UnsupportedFeature => "UNSUPPORTED_FEATURE",
            Self::InvalidInput => "INVALID_INPUT",
            Self::InvalidIdentifier => "INVALID_IDENTIFIER",
            Self::InvalidWeight => "INVALID_WEIGHT",
            Self::InvalidTargetWeight => "INVALID_TARGET_WEIGHT",
            Self::GraphEmpty => "GRAPH_EMPTY",
            Self::GraphUnidentified => "GRAPH_UNIDENTIFIED",
            Self::GraphCertificateFailed => "GRAPH_CERTIFICATE_FAILED",
            Self::ResourceLimit => "RESOURCE_LIMIT",
            Self::AllocationFailed => "ALLOCATION_FAILED",
            Self::CmgSetupFailed => "CMG_SETUP_FAILED",
            Self::CmgApplyFailed => "CMG_APPLY_FAILED",
            Self::PcgCurvatureBreakdown => "PCG_BREAKDOWN_CURVATURE",
            Self::PcgPreconditionerBreakdown => "PCG_BREAKDOWN_PRECONDITIONER",
            Self::PcgStagnation => "PCG_STAGNATION",
            Self::PcgMaxIterations => "PCG_MAXITER",
            Self::FullResidualFailed => "FULL_RESIDUAL_FAILED",
            Self::RngContractFailed => "RNG_CONTRACT_FAILED",
            Self::CorrectionNonFinite => "CORRECTION_NONFINITE",
            Self::AccountingIdentityFailed => "ACCOUNTING_IDENTITY_FAILED",
            Self::JlaMomentFailed => "JLA_MOMENT_FAILED",
            Self::JlaConstraintFailed => "JLA_CONSTRAINT_FAILED",
            Self::NonestimableDeletion => "NONESTIMABLE_DELETION",
            Self::BlockInverseFailed => "BLOCK_INVERSE_FAILED",
            Self::TargetCenteringFailed => "TARGET_CENTERING_FAILED",
            Self::TargetIdentityFailed => "TARGET_IDENTITY_FAILED",
            Self::NonfiniteCorrectedTarget => "NONFINITE_CORRECTED_TARGET",
            Self::StaleContext => "STALE_CONTEXT",
            Self::ContextPoisoned => "CONTEXT_POISONED",
            Self::InternalInvariantFailed => "INTERNAL_INVARIANT_FAILED",
            Self::Panic => "PANIC",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackendError {
    pub code: ErrorCode,
    pub phase: &'static str,
    pub message: String,
}

impl BackendError {
    #[must_use]
    pub fn new(code: ErrorCode, phase: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            phase,
            message: message.into(),
        }
    }

    #[must_use]
    pub fn invalid(phase: &'static str, message: impl Into<String>) -> Self {
        Self::new(ErrorCode::InvalidInput, phase, message)
    }

    #[must_use]
    pub fn invariant(phase: &'static str, message: impl Into<String>) -> Self {
        Self::new(ErrorCode::InternalInvariantFailed, phase, message)
    }
}

impl fmt::Display for BackendError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} [{}]: {}",
            self.code.as_str(),
            self.phase,
            self.message
        )
    }
}

impl std::error::Error for BackendError {}

pub type Result<T> = core::result::Result<T, BackendError>;
