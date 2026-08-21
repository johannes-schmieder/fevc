// SPDX-License-Identifier: GPL-3.0-only

//! Staged preparation independent of the Stata SPI.
//!
//! The caller-thread SPI adapter copies numeric columns into `InputColumns`.
//! This module then validates, canonicalizes, applies the complete current
//! match-deletion graph fixed point, compresses the retained sample, and builds
//! exact JLA semantic plans. No estimator random atom is consumed here.

use vckss_core::error::Result;
use vckss_core::graph::select_match_deletion_graph;
use vckss_core::jla::JlaPlan;
use vckss_core::problem::{CanonicalInput, CompressedProblem};
use vckss_core::types::InputColumns;

use crate::context::{ContextHandle, ContextRegistry, ContextSnapshot};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PreparationReceipt {
    pub input_rows: u64,
    pub retained_rows: u64,
    pub workers: u64,
    pub firms: u64,
    pub cells: u64,
    pub deletion_units: u64,
    pub target_strata: u64,
}

#[derive(Clone, Debug)]
pub struct PreparedProblem {
    pub problem: CompressedProblem,
    pub plan: JlaPlan,
    pub receipt: PreparationReceipt,
}

impl PreparedProblem {
    pub fn from_columns(columns: InputColumns) -> Result<Self> {
        let input_rows = u64::try_from(columns.worker.len()).map_err(|_| {
            vckss_core::error::BackendError::new(
                vckss_core::error::ErrorCode::ResourceLimit,
                "session_prepare",
                "input row count is not representable as u64",
            )
        })?;
        let validated = columns.validate()?;
        let canonical = CanonicalInput::from_validated(validated)?;
        let selection = select_match_deletion_graph(&canonical)?;
        let problem = canonical.compress(&selection.active)?;
        let plan = JlaPlan::build_no_controls(&problem)?;
        let receipt = PreparationReceipt {
            input_rows,
            retained_rows: to_u64(problem.outcome.len(), "retained row count")?,
            workers: to_u64(problem.workers(), "worker count")?,
            firms: to_u64(problem.firms(), "firm count")?,
            cells: to_u64(problem.cells(), "cell count")?,
            deletion_units: to_u64(problem.deletion_units(), "deletion-unit count")?,
            target_strata: to_u64(plan.target_strata(), "target-stratum count")?,
        };
        Ok(Self {
            problem,
            plan,
            receipt,
        })
    }
}

/// Typed owner for one staged command. The output type is supplied by the
/// numerical executor, allowing the lifecycle to be tested independently.
#[derive(Debug)]
pub struct NativeSession<Output> {
    registry: ContextRegistry<PreparedProblem, Output>,
}

impl<Output> Default for NativeSession<Output> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Output> NativeSession<Output> {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            registry: ContextRegistry::new(),
        }
    }

    pub fn prepare(&mut self, columns: InputColumns) -> Result<ContextHandle> {
        let prepared = PreparedProblem::from_columns(columns)?;
        self.registry.prepare(prepared)
    }

    pub fn solve<F>(&mut self, handle: ContextHandle, solve: F) -> Result<()>
    where
        F: FnOnce(PreparedProblem) -> Result<Output>,
    {
        self.registry.solve(handle, solve)
    }

    pub fn result(&self, handle: ContextHandle) -> Result<&Output> {
        self.registry.result(handle)
    }

    pub fn release(&mut self, handle: ContextHandle) -> Result<bool> {
        self.registry.release(handle)
    }

    pub fn clear_abandoned(&mut self) -> Option<ContextHandle> {
        self.registry.clear_abandoned()
    }

    #[must_use]
    pub fn snapshot(&self) -> ContextSnapshot {
        self.registry.snapshot()
    }
}

fn to_u64(value: usize, label: &str) -> Result<u64> {
    u64::try_from(value).map_err(|_| {
        vckss_core::error::BackendError::new(
            vckss_core::error::ErrorCode::ResourceLimit,
            "session_prepare",
            format!("{label} is not representable as u64"),
        )
    })
}
