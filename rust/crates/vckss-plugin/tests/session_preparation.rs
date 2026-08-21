// SPDX-License-Identifier: GPL-3.0-only

#[path = "../src/context.rs"]
mod context;
#[path = "../src/session.rs"]
mod session;

use context::ContextStateTag;
use session::{NativeSession, PreparationReceipt};
use vckss_core::error::ErrorCode;
use vckss_core::types::InputColumns;

fn dense_fixture(reverse_rows: bool, controls: bool) -> InputColumns {
    let workers = 12_usize;
    let firms = 4_usize;
    let rows = workers * firms;
    let mut worker = Vec::with_capacity(rows);
    let mut firm = Vec::with_capacity(rows);
    let mut deletion = Vec::with_capacity(rows);
    let mut outcome = Vec::with_capacity(rows);
    let mut frequency = Vec::with_capacity(rows);
    let mut target_weight = Vec::with_capacity(rows);
    for worker_index in 0..workers {
        for firm_index in 0..firms {
            worker.push(u64::try_from(10_000 + worker_index).expect("worker"));
            firm.push(u64::try_from(20_000 + firm_index).expect("firm"));
            deletion.push(
                u64::try_from(30_000 + worker_index * firms + firm_index).expect("deletion"),
            );
            let sign = if (worker_index + firm_index) % 2 == 0 {
                1.0
            } else {
                -1.0
            };
            outcome.push(
                sign * f64::from(u32::try_from(worker_index + firm_index + 1).expect("value")),
            );
            frequency.push(u64::try_from(firm_index + 1).expect("frequency"));
            target_weight.push(f64::from(
                u32::try_from((worker_index % 3) + firm_index + 1).expect("target"),
            ));
        }
    }
    if reverse_rows {
        worker.reverse();
        firm.reverse();
        deletion.reverse();
        outcome.reverse();
        frequency.reverse();
        target_weight.reverse();
    }
    InputColumns {
        worker,
        firm,
        deletion,
        outcome,
        frequency,
        target_weight,
        controls: if controls {
            vec![vec![1.0; rows]]
        } else {
            Vec::new()
        },
    }
}

#[test]
fn preparation_retains_dense_bridge_free_fixture() {
    let mut session = NativeSession::<PreparationReceipt>::new();
    let handle = session
        .prepare(dense_fixture(false, false))
        .expect("prepare");
    assert_eq!(session.snapshot().state, ContextStateTag::Prepared);

    session
        .solve(handle, |prepared| {
            assert_eq!(prepared.receipt.input_rows, 48);
            assert_eq!(prepared.receipt.retained_rows, 48);
            assert_eq!(prepared.receipt.workers, 12);
            assert_eq!(prepared.receipt.firms, 4);
            assert_eq!(prepared.receipt.cells, 48);
            assert_eq!(prepared.receipt.deletion_units, 48);
            assert_eq!(prepared.receipt.target_strata, 48);
            assert_eq!(prepared.plan.deletion_units(), 48);
            assert_eq!(prepared.problem.outcome.len(), 48);
            Ok(prepared.receipt)
        })
        .expect("solve preparation receipt");
    assert_eq!(session.snapshot().state, ContextStateTag::Solved);
    assert_eq!(session.result(handle).expect("receipt").retained_rows, 48);
    assert!(session.release(handle).expect("release"));
}

#[test]
fn preparation_receipt_is_row_order_invariant() {
    fn prepare(columns: InputColumns) -> PreparationReceipt {
        let mut session = NativeSession::<PreparationReceipt>::new();
        let handle = session.prepare(columns).expect("prepare");
        session
            .solve(handle, |prepared| Ok(prepared.receipt))
            .expect("solve");
        *session.result(handle).expect("result")
    }
    assert_eq!(
        prepare(dense_fixture(false, false)),
        prepare(dense_fixture(true, false))
    );
}

#[test]
fn unsupported_controls_fail_before_context_creation() {
    let mut session = NativeSession::<PreparationReceipt>::new();
    let error = session
        .prepare(dense_fixture(false, true))
        .expect_err("controls are not yet admitted on this route");
    assert_eq!(error.code, ErrorCode::UnsupportedFeature);
    assert_eq!(session.snapshot().state, ContextStateTag::Empty);
}

#[test]
fn invalid_columns_fail_without_poisoning_the_session() {
    let mut session = NativeSession::<PreparationReceipt>::new();
    let mut input = dense_fixture(false, false);
    input.frequency[7] = 0;
    let error = session.prepare(input).expect_err("zero frequency must fail");
    assert_eq!(error.code, ErrorCode::InvalidWeight);
    assert_eq!(session.snapshot().state, ContextStateTag::Empty);

    let handle = session
        .prepare(dense_fixture(false, false))
        .expect("valid prepare after failure");
    assert_eq!(session.snapshot().generation, Some(handle.generation()));
}

#[test]
fn abandoned_preparation_releases_all_owned_state() {
    let mut session = NativeSession::<PreparationReceipt>::new();
    let handle = session
        .prepare(dense_fixture(false, false))
        .expect("prepare");
    assert_eq!(session.clear_abandoned(), Some(handle));
    assert_eq!(session.snapshot().state, ContextStateTag::Empty);
    assert!(!session.release(handle).expect("idempotent old release"));
}
