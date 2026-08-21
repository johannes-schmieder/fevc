// SPDX-License-Identifier: GPL-3.0-only

//! Graph-certified preparation with an original-marked-row retention mask.
//!
//! `retained[row]` is aligned to the caller's compact marked-row sequence. The
//! C shim can therefore scatter it back into a preallocated Stata keep variable
//! without retaining pointers to Stata-managed data inside Rust.

use vckss_core::error::{BackendError, ErrorCode, Result};
use vckss_core::graph::select_match_deletion_graph;
use vckss_core::jla::JlaPlan;
use vckss_core::problem::{CanonicalInput, CompressedProblem};
use vckss_core::types::InputColumns;

use crate::context::{ContextHandle, ContextRegistry, ContextSnapshot};
use crate::session::PreparationReceipt;

#[derive(Clone, Debug)]
pub struct PreparedProblemWithMask {
    pub problem: CompressedProblem,
    pub plan: JlaPlan,
    pub retained: Vec<bool>,
    pub receipt: PreparationReceipt,
}

impl PreparedProblemWithMask {
    pub fn from_columns(columns: InputColumns) -> Result<Self> {
        let input_rows = to_u64(columns.worker.len(), "input row count")?;
        let canonical = CanonicalInput::from_validated(columns.validate()?)?;
        let selection = select_match_deletion_graph(&canonical)?;
        if selection.active.len() != usize::try_from(input_rows).map_err(|_| {
            BackendError::new(
                ErrorCode::ResourceLimit,
                "session_prepare",
                "input row count is not representable as usize",
            )
        })? {
            return Err(BackendError::invariant(
                "session_prepare",
                "graph-selection mask has the wrong row dimension",
            ));
        }
        let retained = selection.active;
        let problem = canonical.compress(&retained)?;
        let plan = JlaPlan::build_no_controls(&problem)?;
        let retained_rows = retained.iter().filter(|&&value| value).count();
        if retained_rows != problem.outcome.len() {
            return Err(BackendError::invariant(
                "session_prepare",
                "retention mask and compressed row count disagree",
            ));
        }
        let receipt = PreparationReceipt {
            input_rows,
            retained_rows: to_u64(retained_rows, "retained row count")?,
            workers: to_u64(problem.workers(), "worker count")?,
            firms: to_u64(problem.firms(), "firm count")?,
            cells: to_u64(problem.cells(), "cell count")?,
            deletion_units: to_u64(problem.deletion_units(), "deletion-unit count")?,
            target_strata: to_u64(plan.target_strata(), "target-stratum count")?,
        };
        Ok(Self {
            problem,
            plan,
            retained,
            receipt,
        })
    }
}

#[derive(Debug)]
pub struct RetainedNativeSession<Output> {
    registry: ContextRegistry<PreparedProblemWithMask, Output>,
}

impl<Output> Default for RetainedNativeSession<Output> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Output> RetainedNativeSession<Output> {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            registry: ContextRegistry::new(),
        }
    }

    pub fn prepare(&mut self, columns: InputColumns) -> Result<ContextHandle> {
        self.registry
            .prepare(PreparedProblemWithMask::from_columns(columns)?)
    }

    pub fn solve<F>(&mut self, handle: ContextHandle, solve: F) -> Result<()>
    where
        F: FnOnce(PreparedProblemWithMask) -> Result<Output>,
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
        BackendError::new(
            ErrorCode::ResourceLimit,
            "session_prepare",
            format!("{label} is not representable as u64"),
        )
    })
}
