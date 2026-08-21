// SPDX-License-Identifier: GPL-3.0-only

use vckss_core::types::InputColumns;
use vckss_plugin::session_retained::RetainedNativeSession;

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
    let mut session = RetainedNativeSession::<Vec<bool>>::new();
    let handle = session.prepare(input).expect("prepare");
    session
        .solve(handle, |prepared| {
            assert_eq!(prepared.receipt.input_rows, 18);
            assert_eq!(prepared.receipt.retained_rows, 12);
            assert_eq!(prepared.receipt.workers, 4);
            assert_eq!(prepared.receipt.firms, 3);
            assert_eq!(prepared.problem.outcome.len(), 12);
            assert_eq!(prepared.plan.deletion_units(), 12);
            Ok(prepared.retained)
        })
        .expect("solve");
    assert_eq!(session.result(handle).expect("mask"), &expected);
}

#[test]
fn mask_remains_aligned_after_input_row_reversal() {
    let (input, expected) = disconnected_fixture(true);
    let mut session = RetainedNativeSession::<Vec<bool>>::new();
    let handle = session.prepare(input).expect("prepare");
    session
        .solve(handle, |prepared| Ok(prepared.retained))
        .expect("solve");
    assert_eq!(session.result(handle).expect("mask"), &expected);
}
