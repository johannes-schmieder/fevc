// SPDX-License-Identifier: GPL-3.0-only

//! Staged preparation independent of the Stata SPI.
//!
//! The caller-thread SPI adapter copies numeric columns into `InputColumns`.
//! This module then validates, canonicalizes, applies the complete current
//! match-deletion graph fixed point, compresses the retained sample, and builds
//! exact JLA semantic plans. No estimator random atom is consumed here.

use vckss_core::error::Result;
use vckss_core::graph::{select_match_deletion_graph, GraphSelectionReceipt};
use vckss_core::jla::JlaPlan;
use vckss_core::problem::{CanonicalInput, CompressedProblem};
use vckss_core::types::InputColumns;

use crate::context::{ContextHandle, ContextRegistry, ContextSnapshot};

pub const ENGINE_NUMERIC_COLUMNS: u64 = 6;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PreparationMemoryReceipt {
    pub hard_limit_bytes: u64,
    pub caller_copy_bytes: u64,
    pub preparation_peak_forecast_bytes: u64,
    pub prepared_resident_bytes: u64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PreparationReceipt {
    pub input_rows: u64,
    pub retained_rows: u64,
    pub workers: u64,
    pub firms: u64,
    pub cells: u64,
    pub deletion_units: u64,
    pub target_strata: u64,
    pub target_weight_sum: f64,
    pub graph: GraphSelectionReceipt,
    pub memory: PreparationMemoryReceipt,
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
        let graph = selection.receipt;
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
            target_weight_sum: problem.target_total,
            graph,
            memory: PreparationMemoryReceipt::default(),
        };
        Ok(Self {
            problem,
            plan,
            receipt,
        })
    }
}

pub fn expected_caller_copy_bytes(rows: u64) -> Result<u64> {
    rows.checked_mul(ENGINE_NUMERIC_COLUMNS)
        .and_then(|value| value.checked_mul(8))
        .ok_or_else(|| memory_error("six-column caller copy byte count overflow"))
}

pub fn admit_prepare_memory(
    rows: u64,
    hard_limit_bytes: u64,
    caller_copy_bytes: u64,
) -> Result<PreparationMemoryReceipt> {
    if rows == 0 || hard_limit_bytes == 0 {
        return Err(vckss_core::error::BackendError::invalid(
            "engine_memory",
            "row count and whole-command memory limit must be positive",
        ));
    }
    let expected = expected_caller_copy_bytes(rows)?;
    if caller_copy_bytes != expected {
        return Err(vckss_core::error::BackendError::invalid(
            "engine_memory",
            format!(
                "declared caller copy is {caller_copy_bytes} bytes; six columns require {expected} bytes"
            ),
        ));
    }
    // The preparation path retains the six typed Rust columns while building
    // canonical maps, graph adjacency/certificates, compressed scatter maps,
    // and the semantic plan. The 768-byte per-row direct model charges all
    // logical arrays at doubling-growth capacity plus B-tree/adjacency node
    // storage; the fixed charge covers their top-level containers.
    let rust_prepare_bytes = rows
        .checked_mul(768)
        .and_then(|value| value.checked_add(4096))
        .ok_or_else(|| memory_error("Rust preparation byte forecast overflow"))?;
    let preparation_peak_forecast_bytes = caller_copy_bytes
        .checked_add(rust_prepare_bytes)
        .ok_or_else(|| memory_error("simultaneous C/Rust preparation peak overflow"))?;
    if preparation_peak_forecast_bytes > hard_limit_bytes {
        return Err(vckss_core::error::BackendError::new(
            vckss_core::error::ErrorCode::ResourceLimit,
            "engine_memory",
            format!(
                "simultaneous C/Rust preparation forecast {preparation_peak_forecast_bytes} bytes exceeds the declared limit {hard_limit_bytes} bytes"
            ),
        ));
    }
    Ok(PreparationMemoryReceipt {
        hard_limit_bytes,
        caller_copy_bytes,
        preparation_peak_forecast_bytes,
        prepared_resident_bytes: 0,
    })
}

pub fn admit_prepare_memory_with_controls(
    rows: u64,
    controls: u32,
    hard_limit_bytes: u64,
    caller_copy_bytes: u64,
) -> Result<PreparationMemoryReceipt> {
    admit_prepare_memory_with_controls_and_probe_order(
        rows,
        controls,
        false,
        hard_limit_bytes,
        caller_copy_bytes,
    )
}

pub fn admit_prepare_memory_with_controls_and_probe_order(
    rows: u64,
    controls: u32,
    probeorder_supplied: bool,
    hard_limit_bytes: u64,
    caller_copy_bytes: u64,
) -> Result<PreparationMemoryReceipt> {
    if controls == 0 && !probeorder_supplied {
        return admit_prepare_memory(rows, hard_limit_bytes, caller_copy_bytes);
    }
    if rows == 0 || hard_limit_bytes == 0 {
        return Err(vckss_core::error::BackendError::invalid(
            "engine_memory",
            "row count and whole-command memory limit must be positive",
        ));
    }
    let numeric_columns = ENGINE_NUMERIC_COLUMNS
        .checked_add(u64::from(controls))
        .and_then(|value| value.checked_add(u64::from(probeorder_supplied)))
        .ok_or_else(|| memory_error("dynamic input-column count overflow"))?;
    let expected = rows
        .checked_mul(numeric_columns)
        .and_then(|value| value.checked_mul(8))
        .ok_or_else(|| memory_error("dynamic caller copy byte count overflow"))?;
    if caller_copy_bytes != expected {
        return Err(vckss_core::error::BackendError::invalid(
            "engine_memory",
            format!(
                "declared caller copy is {caller_copy_bytes} bytes; {numeric_columns} columns require {expected} bytes"
            ),
        ));
    }
    // Retained control columns add their owned values plus canonicalization
    // and dense-exact working space.  The conservative 32-byte per
    // row/control charge includes vector growth slack and temporary copies.
    let control_bytes = rows
        .checked_mul(u64::from(controls))
        .and_then(|value| value.checked_mul(32))
        .ok_or_else(|| memory_error("control preparation byte forecast overflow"))?;
    // The source key and its retained-row copy coexist while graph selection
    // is reconciled. Both vectors are charged at their exact row capacity.
    let probe_order_bytes = rows
        .checked_mul(u64::from(probeorder_supplied))
        .and_then(|value| value.checked_mul(16))
        .ok_or_else(|| memory_error("probe-order preparation byte forecast overflow"))?;
    let rust_prepare_bytes = rows
        .checked_mul(768)
        .and_then(|value| value.checked_add(control_bytes))
        .and_then(|value| value.checked_add(probe_order_bytes))
        .and_then(|value| value.checked_add(4096))
        .ok_or_else(|| memory_error("Rust preparation byte forecast overflow"))?;
    let preparation_peak_forecast_bytes = caller_copy_bytes
        .checked_add(rust_prepare_bytes)
        .ok_or_else(|| memory_error("simultaneous C/Rust preparation peak overflow"))?;
    if preparation_peak_forecast_bytes > hard_limit_bytes {
        return Err(vckss_core::error::BackendError::new(
            vckss_core::error::ErrorCode::ResourceLimit,
            "engine_memory",
            format!(
                "simultaneous C/Rust preparation forecast {preparation_peak_forecast_bytes} bytes exceeds the declared limit {hard_limit_bytes} bytes"
            ),
        ));
    }
    Ok(PreparationMemoryReceipt {
        hard_limit_bytes,
        caller_copy_bytes,
        preparation_peak_forecast_bytes,
        prepared_resident_bytes: 0,
    })
}

fn memory_error(message: &str) -> vckss_core::error::BackendError {
    vckss_core::error::BackendError::new(
        vckss_core::error::ErrorCode::ResourceLimit,
        "engine_memory",
        message,
    )
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
