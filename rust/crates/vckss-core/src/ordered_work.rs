// SPDX-License-Identifier: GPL-3.0-only

//! Bounded statistical jobs on an already-owned solver pool. Numerical work
//! and error precedence follow logical input order, not completion order.

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use crate::error::{BackendError, ErrorCode, Result};
use crate::full_cmg::FullCmgDirectSolver;
use crate::interrupt::{CancellationInterrupt, InterruptCheck};
use crate::model_operator::reserve_exact;

// Includes input/error headers, scoped task storage and runtime bookkeeping.
// Payload buffers are admitted by the statistical phase, not by this helper.
const METADATA_PER_JOB: usize = 1024;
const MAX_INPUT_HEADER: usize = 128;

pub(crate) fn metadata_bytes(maximum_jobs: usize) -> Result<u64> {
    maximum_jobs
        .checked_mul(METADATA_PER_JOB)
        .filter(|&bytes| bytes <= isize::MAX as usize)
        .map(|bytes| bytes as u64)
        .ok_or_else(|| resource("statistical queue metadata overflow"))
}

pub(crate) enum Pool<'a> {
    Cmg(&'a FullCmgDirectSolver),
    Rayon(&'a rayon::ThreadPool),
}

fn resource(message: &str) -> BackendError {
    BackendError::new(
        ErrorCode::ResourceLimit,
        "ordered_statistical_work",
        message,
    )
}

/// `maximum_jobs` is charged with `metadata_bytes` before the estimator starts.
/// Each input contains only borrowed buffers/indices; owned payloads have their
/// own phase forecast. The operation must not submit nested solves or retain a
/// pool lock. CMG's native coordinator supplies its caller-polled token; legacy
/// core callbacks without such a token retain synchronous caller execution.
/// Only an explicitly inert checker can also use the CMG pool without a token.
pub(crate) fn run<I, Iter, F>(
    pool: Option<Pool<'_>>,
    maximum_jobs: usize,
    input: Iter,
    phase: &'static str,
    operation: F,
    interrupt: &mut dyn InterruptCheck,
) -> Result<()>
where
    I: Send,
    Iter: ExactSizeIterator<Item = I>,
    F: Fn(usize, I, &mut dyn InterruptCheck) -> Result<()> + Sync,
{
    let count = input.len();
    if count > maximum_jobs || size_of::<I>() > MAX_INPUT_HEADER {
        return Err(resource("statistical jobs exceed their admitted capacity"));
    }
    metadata_bytes(maximum_jobs)?;
    interrupt.checkpoint(phase)?;
    let parent = interrupt.cancellation_token();
    if count < 2
        || pool.is_none()
        || matches!(pool, Some(Pool::Cmg(_))) && parent.is_none() && !interrupt.is_inert()
    {
        for (index, item) in input.enumerate() {
            interrupt.checkpoint(phase)?;
            operation(index, item, interrupt)?;
        }
        return Ok(());
    }
    let token = parent.unwrap_or_default();
    let mut errors = Vec::new();
    reserve_exact(&mut errors, count, "statistical queue errors")?;
    errors.resize_with(count, || None);
    let mut jobs = Vec::new();
    reserve_exact(&mut jobs, count, "statistical queue inputs")?;
    for ((index, item), error) in input.enumerate().zip(&mut errors) {
        jobs.push((index, item, error));
    }
    let first_error = AtomicUsize::new(count);
    let remaining = AtomicUsize::new(count);
    let caller = std::thread::current();
    let perform = |(index, item, error): (usize, I, &mut Option<BackendError>)| {
        let result = if index > first_error.load(Ordering::Acquire) {
            Ok(())
        } else {
            let mut check = CancellationInterrupt::new(token.clone());
            catch_unwind(AssertUnwindSafe(|| {
                check.checkpoint(phase)?;
                operation(index, item, &mut check)
            }))
            .unwrap_or_else(|_| {
                Err(BackendError::new(
                    ErrorCode::Panic,
                    phase,
                    "statistical worker panicked",
                ))
            })
        };
        if let Err(failure) = result {
            first_error.fetch_min(index, Ordering::AcqRel);
            *error = Some(failure);
        }
        remaining.fetch_sub(1, Ordering::AcqRel);
        caller.unpark();
    };
    let mut caller_error = None;
    match pool.expect("parallel pool") {
        Pool::Cmg(solver) => {
            // Vec<()> has no payload allocation. All actual job/error storage
            // above was fallibly reserved within the pre-admitted bound.
            solver.map_independent_ordered(jobs, perform);
        }
        Pool::Rayon(pool) => pool.in_place_scope(|scope| {
            for job in jobs {
                let perform = &perform;
                scope.spawn(move |_| perform(job));
            }
            while remaining.load(Ordering::Acquire) != 0 {
                if caller_error.is_none() {
                    if let Err(error) = interrupt.checkpoint(phase) {
                        token.cancel();
                        caller_error = Some(error);
                    }
                }
                if remaining.load(Ordering::Acquire) != 0 {
                    std::thread::park_timeout(Duration::from_millis(1));
                }
            }
        }),
    }
    if let Some(error) = caller_error {
        return Err(error);
    }
    if let Some(error) = errors.into_iter().flatten().next() {
        return Err(error);
    }
    interrupt.checkpoint(phase)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interrupt::NeverInterrupt;

    #[test]
    fn order_partial_work_threads_failures_and_reuse() {
        for threads in [1, 2, 3, 4, 7, 14, 28, 64] {
            let pool = rayon::ThreadPoolBuilder::new()
                .num_threads(threads)
                .build()
                .unwrap();
            for count in [0, 1, 3, 17, 65] {
                let mut values = vec![0; count];
                run(
                    Some(Pool::Rayon(&pool)),
                    65,
                    values.iter_mut(),
                    "ordered_test",
                    |index, value, _| {
                        *value = index;
                        Ok(())
                    },
                    &mut NeverInterrupt,
                )
                .unwrap();
                assert_eq!(values, (0..count).collect::<Vec<_>>());
            }
            let error = run(
                Some(Pool::Rayon(&pool)),
                65,
                0..17,
                "ordered_test",
                |index, _, _| {
                    if index == 2 || index == 9 {
                        Err(BackendError::invalid("ordered_test", index.to_string()))
                    } else {
                        Ok(())
                    }
                },
                &mut NeverInterrupt,
            )
            .unwrap_err();
            assert!(error.message.contains('2'));
            let error = run(
                Some(Pool::Rayon(&pool)),
                65,
                0..17,
                "ordered_test",
                |index, _, _| {
                    assert_ne!(index, 4, "injected panic");
                    Ok(())
                },
                &mut NeverInterrupt,
            )
            .unwrap_err();
            assert_eq!(error.code, ErrorCode::Panic);
            run(
                Some(Pool::Rayon(&pool)),
                65,
                0..17,
                "ordered_test",
                |_, _, _| Ok(()),
                &mut NeverInterrupt,
            )
            .unwrap();
        }
    }

    #[test]
    fn cancellation_stays_on_caller_and_reuses_pool() {
        struct Break {
            owner: std::thread::ThreadId,
            calls: usize,
        }
        impl InterruptCheck for Break {
            fn checkpoint(&mut self, phase: &'static str) -> Result<()> {
                assert_eq!(std::thread::current().id(), self.owner);
                self.calls += 1;
                if self.calls > 1 {
                    Err(BackendError::new(ErrorCode::UserBreak, phase, "test break"))
                } else {
                    Ok(())
                }
            }
        }
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(4)
            .build()
            .unwrap();
        let mut interrupt = Break {
            owner: std::thread::current().id(),
            calls: 0,
        };
        let error = run(
            Some(Pool::Rayon(&pool)),
            4,
            0..4,
            "cancel_test",
            |_, _, check| loop {
                check.checkpoint("cancel_worker")?;
                std::thread::yield_now();
            },
            &mut interrupt,
        )
        .unwrap_err();
        assert_eq!(error.code, ErrorCode::UserBreak);
        run(
            Some(Pool::Rayon(&pool)),
            4,
            0..4,
            "reuse_test",
            |_, _, _| Ok(()),
            &mut NeverInterrupt,
        )
        .unwrap();
        assert!(metadata_bytes(usize::MAX).is_err());
        assert!(run(
            None,
            1,
            0..2,
            "overflow_test",
            |_, _, _| Ok(()),
            &mut NeverInterrupt
        )
        .is_err());
    }
}
