// SPDX-License-Identifier: GPL-3.0-only

#[path = "../src/context.rs"]
mod context;
#[path = "../src/session.rs"]
mod session;

use context::ContextStateTag;
use session::NativeSession;
use vckss_core::engine::{run_jla_no_controls, JlaEngineOptions};
use vckss_core::types::InputColumns;

fn dense_fixture() -> InputColumns {
    let workers = 10_usize;
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
            worker.push(u64::try_from(worker_index + 1).expect("worker"));
            firm.push(u64::try_from(firm_index + 1).expect("firm"));
            deletion.push(
                u64::try_from(worker_index * firms + firm_index + 1).expect("deletion"),
            );
            let sign = if (worker_index + firm_index) % 2 == 0 {
                1.0
            } else {
                -1.0
            };
            outcome.push(
                sign * f64::from(
                    u32::try_from(2 * worker_index + firm_index + 1).expect("outcome"),
                ),
            );
            frequency.push(u64::try_from((firm_index % 2) + 1).expect("frequency"));
            target_weight.push(f64::from(
                u32::try_from((worker_index % 3) + firm_index + 1).expect("target"),
            ));
        }
    }
    InputColumns {
        worker,
        firm,
        deletion,
        outcome,
        frequency,
        target_weight,
        controls: Vec::new(),
    }
}

#[test]
fn staged_session_runs_the_complete_default_jla_engine() {
    let mut session = NativeSession::new();
    let handle = session.prepare(dense_fixture()).expect("prepare");
    session
        .solve(handle, |prepared| {
            run_jla_no_controls(&prepared.problem, JlaEngineOptions::default())
        })
        .expect("end-to-end JLA");
    assert_eq!(session.snapshot().state, ContextStateTag::Solved);
    let _validated_result = session.result(handle).expect("JLA result");
    assert!(session.release(handle).expect("release"));
}
