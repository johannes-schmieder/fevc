// SPDX-License-Identifier: GPL-3.0-only

//! Graph-certified preparation with an original-marked-row retention mask.
//!
//! `retained[row]` is aligned to the caller's compact marked-row sequence. The
//! C shim can therefore scatter it back into a preallocated Stata keep variable
//! without retaining pointers to Stata-managed data inside Rust.

use std::sync::Arc;

use vckss_core::error::{BackendError, ErrorCode, Result};
use vckss_core::graph::{
    select_match_deletion_graph_with_interrupt, select_observation_deletion_graph_with_interrupt,
};
use vckss_core::interrupt::{checkpoint_chunk, InterruptCheck, NeverInterrupt};
use vckss_core::jla::JlaPlan;
use vckss_core::problem::{CanonicalInput, CompressedProblem};
use vckss_core::types::{DeletionMode, InputColumns};

use crate::context::{ContextHandle, ContextRegistry, ContextSnapshot};
use crate::session::{PreparationMemoryReceipt, PreparationReceipt};

#[derive(Clone, Debug)]
pub struct PreparedProblemWithMask {
    pub problem: CompressedProblem,
    pub plan: Option<JlaPlan>,
    pub deletion: DeletionMode,
    pub retained: Arc<Vec<bool>>,
    pub receipt: PreparationReceipt,
}

impl PreparedProblemWithMask {
    pub fn from_columns(columns: InputColumns) -> Result<Self> {
        Self::build(
            columns,
            DeletionMode::Match,
            PreparationMemoryReceipt::default(),
            &mut NeverInterrupt,
        )
    }

    pub fn from_columns_and_interrupt(
        columns: InputColumns,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<Self> {
        Self::build(
            columns,
            DeletionMode::Match,
            PreparationMemoryReceipt::default(),
            interrupt,
        )
    }

    pub fn from_columns_with_memory(
        columns: InputColumns,
        memory: PreparationMemoryReceipt,
    ) -> Result<Self> {
        if memory.hard_limit_bytes == 0 || memory.preparation_peak_forecast_bytes == 0 {
            return Err(BackendError::invalid(
                "engine_memory",
                "admitted preparation memory receipt is incomplete",
            ));
        }
        Self::build(columns, DeletionMode::Match, memory, &mut NeverInterrupt)
    }

    pub fn from_columns_with_memory_and_interrupt(
        columns: InputColumns,
        memory: PreparationMemoryReceipt,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<Self> {
        if memory.hard_limit_bytes == 0 || memory.preparation_peak_forecast_bytes == 0 {
            return Err(BackendError::invalid(
                "engine_memory",
                "admitted preparation memory receipt is incomplete",
            ));
        }
        Self::build(columns, DeletionMode::Match, memory, interrupt)
    }

    pub fn from_columns_with_mode_and_memory_and_interrupt(
        columns: InputColumns,
        deletion: DeletionMode,
        memory: PreparationMemoryReceipt,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<Self> {
        if memory.hard_limit_bytes == 0 || memory.preparation_peak_forecast_bytes == 0 {
            return Err(BackendError::invalid(
                "engine_memory",
                "admitted preparation memory receipt is incomplete",
            ));
        }
        Self::build(columns, deletion, memory, interrupt)
    }

    fn build(
        columns: InputColumns,
        deletion: DeletionMode,
        mut memory: PreparationMemoryReceipt,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<Self> {
        interrupt.checkpoint("session_prepare_entry")?;
        let input_rows = to_u64(columns.worker.len(), "input row count")?;
        let canonical = CanonicalInput::from_validated_with_interrupt(
            columns.validate_with_interrupt(interrupt)?,
            interrupt,
        )?;
        let selection = match deletion {
            DeletionMode::Match => {
                select_match_deletion_graph_with_interrupt(&canonical, interrupt)?
            }
            DeletionMode::Observation => {
                select_observation_deletion_graph_with_interrupt(&canonical, interrupt)?
            }
        };
        let graph = selection.receipt;
        if selection.active.len()
            != usize::try_from(input_rows).map_err(|_| {
                BackendError::new(
                    ErrorCode::ResourceLimit,
                    "session_prepare",
                    "input row count is not representable as usize",
                )
            })?
        {
            return Err(BackendError::invariant(
                "session_prepare",
                "graph-selection mask has the wrong row dimension",
            ));
        }
        let retained = Arc::new(selection.active);
        let problem = canonical.compress_with_interrupt(retained.as_slice(), interrupt)?;
        let plan = if deletion == DeletionMode::Match && problem.controls.is_empty() {
            Some(JlaPlan::build_no_controls_with_interrupt(
                &problem, interrupt,
            )?)
        } else {
            None
        };
        let mut retained_rows = 0_usize;
        for (row, &value) in retained.iter().enumerate() {
            checkpoint_chunk(interrupt, row, "session_prepare_retained_reconcile")?;
            retained_rows += usize::from(value);
        }
        if retained_rows != problem.outcome.len() {
            return Err(BackendError::invariant(
                "session_prepare",
                "retention mask and compressed row count disagree",
            ));
        }
        if memory.hard_limit_bytes != 0 {
            interrupt.checkpoint("session_prepare_resident_reconcile")?;
            let retained_bytes = to_u64(
                bit_packed_capacity_bytes(retained.capacity()),
                "bit-packed retained-mask capacity",
            )?;
            let problem_bytes = match plan.as_ref() {
                Some(plan) => vckss_core::engine::prepared_problem_bytes(&problem, plan)?,
                None => vckss_core::engine::compressed_problem_bytes(&problem)?,
            };
            memory.prepared_resident_bytes =
                problem_bytes.checked_add(retained_bytes).ok_or_else(|| {
                    BackendError::new(
                        ErrorCode::ResourceLimit,
                        "engine_memory",
                        "prepared resident byte count overflow",
                    )
                })?;
            let simultaneous = memory
                .caller_copy_bytes
                .checked_add(memory.prepared_resident_bytes)
                .ok_or_else(|| {
                    BackendError::new(
                        ErrorCode::ResourceLimit,
                        "engine_memory",
                        "simultaneous caller/prepared byte count overflow",
                    )
                })?;
            if simultaneous > memory.hard_limit_bytes {
                return Err(BackendError::new(
                    ErrorCode::ResourceLimit,
                    "engine_memory",
                    format!(
                        "simultaneous caller copy and prepared resident allocation {simultaneous} bytes exceeds the declared limit {} bytes",
                        memory.hard_limit_bytes
                    ),
                ));
            }
        }
        let receipt = PreparationReceipt {
            input_rows,
            retained_rows: to_u64(retained_rows, "retained row count")?,
            workers: to_u64(problem.workers(), "worker count")?,
            firms: to_u64(problem.firms(), "firm count")?,
            cells: to_u64(problem.cells(), "cell count")?,
            deletion_units: match deletion {
                DeletionMode::Match => to_u64(problem.deletion_units(), "deletion-unit count")?,
                DeletionMode::Observation => problem.physical_total,
            },
            target_strata: to_u64(
                plan.as_ref().map_or_else(
                    || usize::try_from(problem.dimensions.target_strata).expect("target strata"),
                    JlaPlan::target_strata,
                ),
                "target-stratum count",
            )?,
            target_weight_sum: problem.target_total,
            graph,
            memory,
        };
        interrupt.checkpoint("session_prepare_final")?;
        Ok(Self {
            problem,
            plan,
            deletion,
            retained,
            receipt,
        })
    }
}

pub(crate) const fn bit_packed_capacity_bytes(bit_capacity: usize) -> usize {
    bit_capacity.div_ceil(8)
}

#[cfg(test)]
mod tests {
    use super::bit_packed_capacity_bytes;

    #[test]
    fn bit_packed_capacity_rounds_only_at_byte_boundaries() {
        assert_eq!(bit_packed_capacity_bytes(0), 0);
        assert_eq!(bit_packed_capacity_bytes(1), 1);
        assert_eq!(bit_packed_capacity_bytes(7), 1);
        assert_eq!(bit_packed_capacity_bytes(8), 1);
        assert_eq!(bit_packed_capacity_bytes(9), 2);
        assert_eq!(
            bit_packed_capacity_bytes(usize::MAX),
            usize::MAX / 8 + usize::from(usize::MAX % 8 != 0)
        );
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
