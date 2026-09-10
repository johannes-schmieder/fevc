// SPDX-License-Identifier: GPL-3.0-only

use std::sync::Arc;

use vckss_core::error::{BackendError, ErrorCode, Result};
use vckss_core::interrupt::{InterruptCheck, NeverInterrupt};
use vckss_core::projection::{ProjectionEffect, ProjectionWeight};
use vckss_core::types::{DeletionMode, InputColumns};
use vckss_plugin::session::PreparationMemoryReceipt;
use vckss_plugin::session_retained::{PreparedProblemWithMask, RetainedNativeSession};

struct BreakOnPhase {
    target: &'static str,
    seen: bool,
}

impl InterruptCheck for BreakOnPhase {
    fn checkpoint(&mut self, phase: &'static str) -> Result<()> {
        if phase == self.target {
            self.seen = true;
            Err(BackendError::new(
                ErrorCode::UserBreak,
                phase,
                "injected preparation break",
            ))
        } else {
            Ok(())
        }
    }
}

fn disconnected_fixture(reverse: bool) -> (InputColumns, Vec<bool>) {
    let mut worker = Vec::<u64>::new();
    let mut firm = Vec::<u64>::new();
    let mut deletion = Vec::<u64>::new();
    let mut outcome = Vec::<f64>::new();
    let mut frequency = Vec::<u64>::new();
    let mut target_weight = Vec::<f64>::new();
    let mut expected = Vec::<bool>::new();

    for worker_id in 1_u64..=4 {
        for firm_id in 1_u64..=3 {
            worker.push(worker_id);
            firm.push(firm_id);
            deletion.push(u64::try_from(deletion.len() + 1).expect("deletion"));
            outcome.push((worker_id as f64) - (firm_id as f64));
            frequency.push(1);
            target_weight.push(1.0);
            expected.push(true);
        }
    }
    for worker_id in 10_u64..=12 {
        for firm_id in 10_u64..=11 {
            worker.push(worker_id);
            firm.push(firm_id);
            deletion.push(u64::try_from(deletion.len() + 1).expect("deletion"));
            outcome.push((worker_id as f64) - (firm_id as f64));
            frequency.push(1);
            target_weight.push(1.0);
            expected.push(false);
        }
    }

    if reverse {
        worker.reverse();
        firm.reverse();
        deletion.reverse();
        outcome.reverse();
        frequency.reverse();
        target_weight.reverse();
        expected.reverse();
    }
    (
        InputColumns {
            worker,
            firm,
            deletion,
            outcome,
            frequency,
            target_weight,
            controls: Vec::new(),
        },
        expected,
    )
}

#[test]
fn mask_selects_the_unique_largest_connected_component() {
    let (input, expected) = disconnected_fixture(false);
    let mut session = RetainedNativeSession::<Arc<Vec<bool>>>::new();
    let handle = session.prepare(input).expect("prepare");
    session
        .solve(handle, |prepared| {
            assert_eq!(prepared.receipt.input_rows, 18);
            assert_eq!(prepared.receipt.retained_rows, 12);
            assert_eq!(prepared.receipt.workers, 4);
            assert_eq!(prepared.receipt.firms, 3);
            assert_eq!(prepared.problem.outcome.len(), 12);
            assert_eq!(
                prepared
                    .plan
                    .as_ref()
                    .expect("match/no-control plan")
                    .deletion_units(),
                12
            );
            Ok(prepared.retained)
        })
        .expect("solve");
    assert_eq!(
        session.result(handle).expect("mask").as_slice(),
        expected.as_slice()
    );
}

#[test]
fn mask_remains_aligned_after_input_row_reversal() {
    let (input, expected) = disconnected_fixture(true);
    let mut session = RetainedNativeSession::<Arc<Vec<bool>>>::new();
    let handle = session.prepare(input).expect("prepare");
    session
        .solve(handle, |prepared| Ok(prepared.retained))
        .expect("solve");
    assert_eq!(
        session.result(handle).expect("mask").as_slice(),
        expected.as_slice()
    );
}

#[test]
fn preparation_breaks_are_reachable_across_every_major_phase() {
    for phase in [
        "ingest_validate_rows",
        "canonicalize_redense_sort",
        "graph_component_bfs",
        "compression_cell_sort",
        "jla_plan_semantic_sort",
        "session_prepare_final",
    ] {
        let (input, _) = disconnected_fixture(false);
        let mut interrupt = BreakOnPhase {
            target: phase,
            seen: false,
        };
        let error = PreparedProblemWithMask::from_columns_and_interrupt(input, &mut interrupt)
            .expect_err("targeted phase must interrupt preparation");
        assert_eq!(error.code, ErrorCode::UserBreak, "phase={phase}");
        assert!(interrupt.seen, "phase={phase}");
    }
}

#[test]
fn legacy_resident_limit_preserves_the_historical_packed_mask_receipt() {
    let (input, _) = disconnected_fixture(false);
    let caller_copy_bytes = u64::try_from(input.worker.len()).expect("rows") * 6 * 8;
    let generous = PreparationMemoryReceipt {
        budget: vckss_core::memory::MemoryBudget::Legacy,
        hard_limit_bytes: 1_u64 << 30,
        caller_copy_bytes,
        preparation_peak_forecast_bytes: 1,
        prepared_resident_bytes: 0,
    };
    let prepared = PreparedProblemWithMask::from_columns_with_memory(input, generous)
        .expect("generous preparation");
    let exact_limit = caller_copy_bytes + prepared.receipt.memory.prepared_resident_bytes;
    assert!(prepared.retained.capacity().div_ceil(8) < prepared.retained.capacity());

    let exact = PreparationMemoryReceipt {
        hard_limit_bytes: exact_limit,
        ..generous
    };
    let (input, _) = disconnected_fixture(false);
    PreparedProblemWithMask::from_columns_with_memory(input, exact)
        .expect("exact caller plus resident limit");

    let one_byte_short = PreparationMemoryReceipt {
        hard_limit_bytes: exact_limit - 1,
        ..generous
    };
    let (input, _) = disconnected_fixture(false);
    let error = PreparedProblemWithMask::from_columns_with_memory(input, one_byte_short)
        .expect_err("one byte below exact retained limit");
    assert_eq!(error.code, ErrorCode::ResourceLimit);
}

#[test]
fn versioned_resident_limit_charges_actual_boolean_storage() {
    use vckss_core::memory::{MemoryBudget, MemoryCheck};

    let (input, _) = disconnected_fixture(false);
    let caller_copy_bytes = u64::try_from(input.worker.len()).expect("rows") * 6 * 8;
    let generous = PreparationMemoryReceipt {
        budget: MemoryBudget::Unspecified,
        hard_limit_bytes: 0,
        caller_copy_bytes,
        preparation_peak_forecast_bytes: 1,
        prepared_resident_bytes: 0,
    };
    let prepared = PreparedProblemWithMask::from_columns_with_memory(input, generous)
        .expect("unbudgeted preparation");
    let problem_bytes = vckss_core::engine::prepared_problem_bytes(
        &prepared.problem,
        prepared.plan.as_ref().expect("compressed plan"),
    )
    .expect("problem storage");
    let mask_bytes = std::alloc::Layout::array::<bool>(prepared.retained.capacity())
        .expect("mask allocation layout")
        .size() as u64;
    assert_eq!(
        prepared.receipt.memory.prepared_resident_bytes,
        problem_bytes + mask_bytes
    );
    let exact_limit = caller_copy_bytes + problem_bytes + mask_bytes;
    for (limit, succeeds) in [(exact_limit, true), (exact_limit - 1, false)] {
        let (input, _) = disconnected_fixture(false);
        let result = PreparedProblemWithMask::from_columns_with_memory(
            input,
            PreparationMemoryReceipt {
                budget: MemoryBudget::Explicit {
                    bytes: limit,
                    check: MemoryCheck::Error,
                },
                hard_limit_bytes: limit,
                ..generous
            },
        );
        if succeeds {
            result.expect("complete caller and resident storage fits exactly");
        } else {
            assert_eq!(
                result.expect_err("one byte short").code,
                ErrorCode::ResourceLimit
            );
        }
    }
}

#[test]
fn projection_augmentation_is_admitted_at_its_complete_synchronous_peak() {
    let (input, _) = disconnected_fixture(false);
    let caller_copy_bytes = u64::try_from(input.worker.len()).expect("rows") * 6 * 8;
    let generous = PreparationMemoryReceipt {
        budget: vckss_core::memory::MemoryBudget::Legacy,
        hard_limit_bytes: 1_u64 << 30,
        caller_copy_bytes,
        preparation_peak_forecast_bytes: 1,
        prepared_resident_bytes: 0,
    };
    let mut prepared = PreparedProblemWithMask::from_columns_with_mode_and_memory_and_interrupt(
        input,
        DeletionMode::Observation,
        generous,
        &mut NeverInterrupt,
    )
    .expect("observation preparation");
    let rows = prepared.problem.outcome.len();
    let project = (0..rows).map(|row| row as f64).collect::<Vec<_>>();
    let projection_copy = u64::try_from(rows).expect("projection rows") * 8;
    let columns = 2_u64;
    let persistent = u64::try_from(prepared.problem.workers() + prepared.problem.firms())
        .expect("projection parameters")
        * columns
        * 8;
    let square = columns * columns * 8;
    let exact_peak = prepared.receipt.memory.prepared_resident_bytes
        + 2 * projection_copy
        + 4 * persistent
        + 8 * square
        + 4096;
    prepared.receipt.memory.hard_limit_bytes = exact_peak - 1;
    let error = prepared
        .augment_projection_with_interrupt(
            vec![project.clone()],
            ProjectionEffect::Firm,
            ProjectionWeight::Frequency,
            1.0e-10,
            projection_copy,
            &mut NeverInterrupt,
        )
        .expect_err("one byte below the projection peak");
    assert_eq!(error.code, ErrorCode::ResourceLimit);
    assert!(prepared.projection.is_none());

    prepared.receipt.memory.hard_limit_bytes = exact_peak;
    prepared
        .augment_projection_with_interrupt(
            vec![project],
            ProjectionEffect::Firm,
            ProjectionWeight::Frequency,
            1.0e-10,
            projection_copy,
            &mut NeverInterrupt,
        )
        .expect("exact projection peak");
    let receipt = &prepared.projection.as_ref().expect("projection").receipt;
    assert_eq!(receipt.augmentation_peak_forecast_bytes, exact_peak);
    assert_eq!(receipt.projection_persistent_bytes, persistent);
}
