// SPDX-License-Identifier: GPL-3.0-only

//! Deterministic leverage/target batch planning.
//!
//! Automatic widths come from one frozen candidate ladder and depend only on
//! declared caps plus checked phase-memory forecasts.  They never depend on
//! elapsed time, solver iterations, or scheduler behavior.

use crate::error::{BackendError, ErrorCode, Result};
use crate::memory::MemoryBudget;

pub const BATCH_PLAN_SCHEMA_VERSION: u32 = 1;
pub const BATCH_WIDTH_CANDIDATES: [usize; 7] = [1, 2, 4, 8, 16, 32, 64];
pub const BATCH_ARITHMETIC_CONTRACT: &str = "VCKSS-INDEPENDENT-SCALAR-LOGICAL-ORDER-V1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BatchRequest {
    Auto,
    Explicit(usize),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BatchSelectionReason {
    LargestAdmissibleCandidate,
    ExplicitWidth,
    NoBudgetPerformanceChoice,
    MinimumMemoryOverBudget,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BatchPlannerCaps {
    pub probes: usize,
    pub declared_threads: usize,
    /// Maximum simultaneous logical columns per declared thread.  This is an
    /// explicit registered policy input, not a measurement.
    pub columns_per_thread: usize,
    pub route_width_cap: usize,
    /// Maximum simultaneous live bytes in preparation, full-fit, correction,
    /// or result-export phases where neither planned batch is active.
    pub non_batched_peak_bytes: u64,
    pub hard_memory_bytes: u64,
    pub memory_budget: MemoryBudget,
}

impl BatchPlannerCaps {
    fn validate(self) -> Result<ValidatedCaps> {
        if self.probes == 0
            || self.declared_threads == 0
            || self.columns_per_thread == 0
            || self.route_width_cap == 0
        {
            return Err(BackendError::invalid(
                "batch_plan",
                "probe, thread, route, and memory caps must be positive",
            ));
        }
        self.memory_budget.validate(self.hard_memory_bytes)?;
        if self
            .memory_budget
            .rejects(self.non_batched_peak_bytes, self.hard_memory_bytes)
        {
            return Err(memory_error(
                "non-batched",
                0,
                self.non_batched_peak_bytes,
                self.hard_memory_bytes,
            ));
        }
        let thread_width_cap = self
            .declared_threads
            .checked_mul(self.columns_per_thread)
            .ok_or_else(|| resource_error("thread batch-width cap overflow"))?;
        let effective_width_cap = self.probes.min(thread_width_cap).min(self.route_width_cap);
        Ok(ValidatedCaps {
            input: self,
            thread_width_cap,
            effective_width_cap,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ValidatedCaps {
    input: BatchPlannerCaps,
    thread_width_cap: usize,
    effective_width_cap: usize,
}

/// Affine simultaneous-lifetime model for one batched command phase.
/// `fixed_bytes` includes every persistent, caller-owned, and phase-owned byte
/// live in that phase. `bytes_per_width` may include multiple logical RHS
/// columns per probe, as in the target phase.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PhaseMemoryModel {
    pub fixed_bytes: u64,
    pub bytes_per_width: u64,
}

impl PhaseMemoryModel {
    pub fn forecast(self, width: usize) -> Result<u64> {
        if width == 0 {
            return Err(BackendError::invalid(
                "batch_plan",
                "phase forecast width must be positive",
            ));
        }
        let width = u64::try_from(width)
            .map_err(|_| resource_error("phase forecast width is not representable"))?;
        self.bytes_per_width
            .checked_mul(width)
            .and_then(|bytes| self.fixed_bytes.checked_add(bytes))
            .ok_or_else(|| resource_error("phase memory forecast overflow"))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BatchPhaseReceipt {
    pub requested: BatchRequest,
    pub selected_width: usize,
    pub reason: BatchSelectionReason,
    pub probe_width_cap: usize,
    pub declared_threads: usize,
    pub thread_width_cap: usize,
    pub route_width_cap: usize,
    pub effective_width_cap: usize,
    pub hard_memory_bytes: u64,
    pub width_one_forecast_bytes: u64,
    pub selected_forecast_bytes: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BatchPlanReceipt {
    pub schema_version: u32,
    pub deterministic: bool,
    pub bitwise_estimator_width_invariance_required: bool,
    pub arithmetic_contract: &'static str,
    pub non_batched_peak_bytes: u64,
    pub selected_command_peak_bytes: u64,
    pub whole_command_admitted: bool,
    pub leverage: BatchPhaseReceipt,
    pub target: BatchPhaseReceipt,
}

pub fn plan_batches(
    leverage_request: BatchRequest,
    target_request: BatchRequest,
    caps: BatchPlannerCaps,
    leverage_memory: PhaseMemoryModel,
    target_memory: PhaseMemoryModel,
) -> Result<BatchPlanReceipt> {
    plan_batches_with_forecasts(
        leverage_request,
        target_request,
        caps,
        |width| leverage_memory.forecast(width),
        |width| target_memory.forecast(width),
    )
}

pub fn plan_batches_with_forecasts<L, T>(
    leverage_request: BatchRequest,
    target_request: BatchRequest,
    caps: BatchPlannerCaps,
    leverage_forecast: L,
    target_forecast: T,
) -> Result<BatchPlanReceipt>
where
    L: Fn(usize) -> Result<u64>,
    T: Fn(usize) -> Result<u64>,
{
    plan_batches_with_width_policy(
        leverage_request,
        target_request,
        caps,
        leverage_forecast,
        target_forecast,
        false,
    )
}

/// Full-CMG automatic widths include the desired cap; other routes keep V1.
pub(crate) fn plan_full_cmg_batches_with_forecasts<L, T>(
    leverage_request: BatchRequest,
    target_request: BatchRequest,
    caps: BatchPlannerCaps,
    leverage_forecast: L,
    target_forecast: T,
) -> Result<BatchPlanReceipt>
where
    L: Fn(usize) -> Result<u64>,
    T: Fn(usize) -> Result<u64>,
{
    plan_batches_with_width_policy(
        leverage_request,
        target_request,
        caps,
        leverage_forecast,
        target_forecast,
        true,
    )
}

fn plan_batches_with_width_policy<L, T>(
    leverage_request: BatchRequest,
    target_request: BatchRequest,
    caps: BatchPlannerCaps,
    leverage_forecast: L,
    target_forecast: T,
    full_cmg: bool,
) -> Result<BatchPlanReceipt>
where
    L: Fn(usize) -> Result<u64>,
    T: Fn(usize) -> Result<u64>,
{
    let caps = caps.validate()?;
    let leverage = plan_phase(
        leverage_request,
        caps,
        leverage_forecast,
        "leverage",
        full_cmg,
    )?;
    let target = plan_phase(target_request, caps, target_forecast, "target", full_cmg)?;
    Ok(BatchPlanReceipt {
        schema_version: BATCH_PLAN_SCHEMA_VERSION,
        deterministic: true,
        bitwise_estimator_width_invariance_required: true,
        arithmetic_contract: BATCH_ARITHMETIC_CONTRACT,
        non_batched_peak_bytes: caps.input.non_batched_peak_bytes,
        selected_command_peak_bytes: caps
            .input
            .non_batched_peak_bytes
            .max(leverage.selected_forecast_bytes)
            .max(target.selected_forecast_bytes),
        whole_command_admitted: true,
        leverage,
        target,
    })
}

fn plan_phase<F>(
    requested: BatchRequest,
    caps: ValidatedCaps,
    forecast: F,
    phase: &'static str,
    full_cmg: bool,
) -> Result<BatchPhaseReceipt>
where
    F: Fn(usize) -> Result<u64>,
{
    if let BatchRequest::Explicit(width) = requested {
        if width == 0 {
            return Err(BackendError::invalid(
                "batch_plan",
                format!("explicit {phase} batch width must be positive"),
            ));
        }
        if width > caps.effective_width_cap {
            return Err(BackendError::new(
                ErrorCode::ResourceLimit,
                "batch_plan",
                format!(
                    "explicit {phase} batch width {width} exceeds the probe/thread/route cap {}",
                    caps.effective_width_cap
                ),
            ));
        }
    }
    let command_forecast =
        |width| forecast(width).map(|bytes| bytes.max(caps.input.non_batched_peak_bytes));
    let width_one_forecast_bytes = command_forecast(1)?;
    let (selected_width, selected_forecast_bytes, reason) = match requested {
        BatchRequest::Explicit(width) => {
            let bytes = if width == 1 {
                width_one_forecast_bytes
            } else {
                command_forecast(width)?
            };
            if caps
                .input
                .memory_budget
                .rejects(bytes, caps.input.hard_memory_bytes)
            {
                return Err(memory_error(
                    phase,
                    width,
                    bytes,
                    caps.input.hard_memory_bytes,
                ));
            }
            (width, bytes, BatchSelectionReason::ExplicitWidth)
        }
        BatchRequest::Auto => {
            let mut selected = None;
            let mut previous = None;
            // Fixed-size iteration uses no allocation and cannot overflow at
            // usize::MAX. The final desired width is essential without a budget.
            let widths = (0..=usize::BITS as usize).filter_map(|index| {
                if full_cmg {
                    if index == usize::BITS as usize {
                        Some(caps.effective_width_cap)
                    } else {
                        1_usize
                            .checked_shl(index as u32)
                            .filter(|width| *width <= caps.effective_width_cap)
                    }
                } else {
                    BATCH_WIDTH_CANDIDATES
                        .get(index)
                        .copied()
                        .filter(|width| *width <= caps.effective_width_cap)
                }
            });
            for width in widths {
                let bytes = if width == 1 {
                    width_one_forecast_bytes
                } else {
                    command_forecast(width)?
                };
                if previous.is_some_and(|prior| bytes < prior) {
                    return Err(BackendError::invariant(
                        "batch_plan",
                        format!("{phase} phase memory forecast is not monotone in batch width"),
                    ));
                }
                previous = Some(bytes);
                if caps
                    .input
                    .memory_budget
                    .fits(bytes, caps.input.hard_memory_bytes)
                {
                    selected = Some((width, bytes));
                }
            }
            let (width, bytes, reason) = match selected {
                Some((width, bytes)) => (
                    width,
                    bytes,
                    if caps
                        .input
                        .memory_budget
                        .limit(caps.input.hard_memory_bytes)
                        .is_none()
                    {
                        BatchSelectionReason::NoBudgetPerformanceChoice
                    } else {
                        BatchSelectionReason::LargestAdmissibleCandidate
                    },
                ),
                None if !caps
                    .input
                    .memory_budget
                    .rejects(width_one_forecast_bytes, caps.input.hard_memory_bytes) =>
                {
                    (
                        1,
                        width_one_forecast_bytes,
                        BatchSelectionReason::MinimumMemoryOverBudget,
                    )
                }
                None => {
                    return Err(memory_error(
                        phase,
                        1,
                        width_one_forecast_bytes,
                        caps.input.hard_memory_bytes,
                    ))
                }
            };
            (width, bytes, reason)
        }
    };
    Ok(BatchPhaseReceipt {
        requested,
        selected_width,
        reason,
        probe_width_cap: caps.input.probes,
        declared_threads: caps.input.declared_threads,
        thread_width_cap: caps.thread_width_cap,
        route_width_cap: caps.input.route_width_cap,
        effective_width_cap: caps.effective_width_cap,
        hard_memory_bytes: caps.input.hard_memory_bytes,
        width_one_forecast_bytes,
        selected_forecast_bytes,
    })
}

fn memory_error(phase: &str, width: usize, forecast: u64, limit: u64) -> BackendError {
    BackendError::new(
        ErrorCode::ResourceLimit,
        "batch_plan",
        format!(
            "{phase} batch width {width} forecasts {forecast} bytes above the hard limit {limit}"
        ),
    )
}

fn resource_error(message: &str) -> BackendError {
    BackendError::new(ErrorCode::ResourceLimit, "batch_plan", message)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn caps(memory: u64) -> BatchPlannerCaps {
        BatchPlannerCaps {
            probes: 100,
            declared_threads: 8,
            columns_per_thread: 8,
            route_width_cap: 64,
            non_batched_peak_bytes: 0,
            hard_memory_bytes: memory,
            memory_budget: MemoryBudget::Legacy,
        }
    }

    #[test]
    fn automatic_plans_are_separate_and_choose_largest_admissible_candidates() {
        let receipt = plan_batches(
            BatchRequest::Auto,
            BatchRequest::Auto,
            caps(1_000),
            PhaseMemoryModel {
                fixed_bytes: 100,
                bytes_per_width: 20,
            },
            PhaseMemoryModel {
                fixed_bytes: 200,
                bytes_per_width: 40,
            },
        )
        .expect("batch plan");
        assert_eq!(receipt.leverage.selected_width, 32);
        assert_eq!(receipt.target.selected_width, 16);
        assert_eq!(receipt.schema_version, BATCH_PLAN_SCHEMA_VERSION);
        assert_eq!(receipt.arithmetic_contract, BATCH_ARITHMETIC_CONTRACT);
        assert!(receipt.deterministic);
        assert!(receipt.bitwise_estimator_width_invariance_required);
    }

    #[test]
    fn explicit_widths_fail_instead_of_shrinking() {
        let route_error = plan_batches(
            BatchRequest::Explicit(65),
            BatchRequest::Explicit(1),
            caps(u64::MAX),
            PhaseMemoryModel {
                fixed_bytes: 0,
                bytes_per_width: 1,
            },
            PhaseMemoryModel {
                fixed_bytes: 0,
                bytes_per_width: 1,
            },
        )
        .expect_err("route cap");
        assert_eq!(route_error.code, ErrorCode::ResourceLimit);

        let memory_error = plan_batches(
            BatchRequest::Explicit(8),
            BatchRequest::Explicit(1),
            caps(79),
            PhaseMemoryModel {
                fixed_bytes: 0,
                bytes_per_width: 10,
            },
            PhaseMemoryModel {
                fixed_bytes: 0,
                bytes_per_width: 1,
            },
        )
        .expect_err("explicit memory failure");
        assert_eq!(memory_error.code, ErrorCode::ResourceLimit);
    }

    #[test]
    fn one_byte_admission_boundary_is_exact() {
        let model = PhaseMemoryModel {
            fixed_bytes: 17,
            bytes_per_width: 5,
        };
        let admitted = plan_batches(
            BatchRequest::Auto,
            BatchRequest::Auto,
            caps(22),
            model,
            model,
        )
        .expect("exact one-width boundary");
        assert_eq!(admitted.leverage.selected_width, 1);
        assert_eq!(admitted.leverage.selected_forecast_bytes, 22);

        let error = plan_batches(
            BatchRequest::Auto,
            BatchRequest::Auto,
            caps(21),
            model,
            model,
        )
        .expect_err("one byte below boundary");
        assert_eq!(error.code, ErrorCode::ResourceLimit);
    }

    #[test]
    fn larger_memory_never_reduces_an_automatic_width() {
        let model = PhaseMemoryModel {
            fixed_bytes: 100,
            bytes_per_width: 25,
        };
        let limits = [125, 150, 200, 300, 500, 900, 1_700];
        let mut previous = 0;
        for limit in limits {
            let receipt = plan_batches(
                BatchRequest::Auto,
                BatchRequest::Auto,
                caps(limit),
                model,
                model,
            )
            .expect("monotone plan");
            assert!(receipt.leverage.selected_width >= previous);
            previous = receipt.leverage.selected_width;
        }
    }

    #[test]
    fn probe_thread_and_route_caps_are_all_applied() {
        let model = PhaseMemoryModel {
            fixed_bytes: 0,
            bytes_per_width: 1,
        };
        let receipt = plan_batches(
            BatchRequest::Auto,
            BatchRequest::Auto,
            BatchPlannerCaps {
                probes: 50,
                declared_threads: 3,
                columns_per_thread: 5,
                route_width_cap: 12,
                non_batched_peak_bytes: 0,
                hard_memory_bytes: u64::MAX,
                memory_budget: MemoryBudget::Legacy,
            },
            model,
            model,
        )
        .expect("capped plan");
        assert_eq!(receipt.leverage.thread_width_cap, 15);
        assert_eq!(receipt.leverage.effective_width_cap, 12);
        assert_eq!(receipt.leverage.selected_width, 8);
    }

    #[test]
    fn checked_forecasts_reject_overflow_and_nonmonotone_callbacks() {
        let overflow = PhaseMemoryModel {
            fixed_bytes: u64::MAX,
            bytes_per_width: 1,
        }
        .forecast(1)
        .expect_err("forecast overflow");
        assert_eq!(overflow.code, ErrorCode::ResourceLimit);

        let nonmonotone = plan_batches_with_forecasts(
            BatchRequest::Auto,
            BatchRequest::Auto,
            caps(u64::MAX),
            |width| Ok(if width == 1 { 100 } else { 99 }),
            |width| Ok(width as u64),
        )
        .expect_err("nonmonotone callback");
        assert_eq!(nonmonotone.code, ErrorCode::InternalInvariantFailed);

        let cap_overflow = plan_batches(
            BatchRequest::Auto,
            BatchRequest::Auto,
            BatchPlannerCaps {
                probes: 1,
                declared_threads: usize::MAX,
                columns_per_thread: 2,
                route_width_cap: 1,
                non_batched_peak_bytes: 0,
                hard_memory_bytes: 1,
                memory_budget: MemoryBudget::Legacy,
            },
            PhaseMemoryModel {
                fixed_bytes: 0,
                bytes_per_width: 1,
            },
            PhaseMemoryModel {
                fixed_bytes: 0,
                bytes_per_width: 1,
            },
        )
        .expect_err("thread cap overflow");
        assert_eq!(cap_overflow.code, ErrorCode::ResourceLimit);
    }

    #[test]
    fn identical_inputs_produce_identical_receipts() {
        let model = PhaseMemoryModel {
            fixed_bytes: 123,
            bytes_per_width: 456,
        };
        let left = plan_batches(
            BatchRequest::Auto,
            BatchRequest::Explicit(7),
            caps(10_000),
            model,
            model,
        )
        .expect("left");
        let right = plan_batches(
            BatchRequest::Auto,
            BatchRequest::Explicit(7),
            caps(10_000),
            model,
            model,
        )
        .expect("right");
        assert_eq!(left, right);
    }

    #[test]
    fn whole_command_non_batched_peak_is_admitted_before_batch_selection() {
        let model = PhaseMemoryModel {
            fixed_bytes: 1,
            bytes_per_width: 1,
        };
        let admitted = plan_batches(
            BatchRequest::Auto,
            BatchRequest::Auto,
            BatchPlannerCaps {
                non_batched_peak_bytes: 1_000,
                ..caps(1_000)
            },
            model,
            model,
        )
        .expect("whole-command boundary");
        assert!(admitted.whole_command_admitted);
        assert_eq!(admitted.selected_command_peak_bytes, 1_000);
        assert_eq!(admitted.leverage.width_one_forecast_bytes, 1_000);

        let error = plan_batches(
            BatchRequest::Auto,
            BatchRequest::Auto,
            BatchPlannerCaps {
                non_batched_peak_bytes: 1_001,
                ..caps(1_000)
            },
            model,
            model,
        )
        .expect_err("one byte above whole-command limit");
        assert_eq!(error.code, ErrorCode::ResourceLimit);
    }
    #[test]
    fn absent_budget_does_not_change_width_with_numeric_limit() {
        let model = PhaseMemoryModel {
            fixed_bytes: 100,
            bytes_per_width: 100,
        };
        for numeric_limit in [0, 1, 4 << 30] {
            let receipt = plan_batches(
                BatchRequest::Auto,
                BatchRequest::Auto,
                BatchPlannerCaps {
                    memory_budget: MemoryBudget::Unspecified,
                    ..caps(numeric_limit)
                },
                model,
                model,
            )
            .unwrap();
            assert_eq!(receipt.leverage.selected_width, 64);
            assert_eq!(
                receipt.leverage.reason,
                BatchSelectionReason::NoBudgetPerformanceChoice
            );
        }
    }

    #[test]
    fn advisory_budget_shrinks_only_automatic_batches() {
        use crate::memory::MemoryCheck;
        let model = PhaseMemoryModel {
            fixed_bytes: 100,
            bytes_per_width: 100,
        };
        for check in [MemoryCheck::Warn, MemoryCheck::Off] {
            let budget = MemoryBudget::Explicit { bytes: 1, check };
            let configured = BatchPlannerCaps {
                memory_budget: budget,
                ..caps(1)
            };
            let automatic = plan_batches(
                BatchRequest::Auto,
                BatchRequest::Auto,
                configured,
                model,
                model,
            )
            .unwrap();
            assert_eq!(automatic.leverage.selected_width, 1);
            assert_eq!(
                automatic.leverage.reason,
                BatchSelectionReason::MinimumMemoryOverBudget
            );
            let explicit = plan_batches(
                BatchRequest::Explicit(32),
                BatchRequest::Explicit(16),
                configured,
                model,
                model,
            )
            .unwrap();
            assert_eq!(explicit.leverage.selected_width, 32);
            assert_eq!(explicit.target.selected_width, 16);
        }
    }
}
