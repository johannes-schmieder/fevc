// SPDX-License-Identifier: GPL-3.0-only

//! Cooperative interruption for long-running native calculations.
//!
//! The numerical core never calls a host SDK.  Its caller supplies a
//! synchronous checker, and every legacy entry point installs the inert
//! checker so the existing public surface and numerical contract are
//! unchanged.

use core::cmp::Ordering;
use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};
use std::sync::Arc;

use crate::error::{BackendError, ErrorCode, Result};

/// Maximum amount of ordinary scalar loop work between bounded checkpoints in
/// explicitly instrumented loops. Standard-library ordering routines cannot
/// host a fallible callback in their comparator; deterministic sort phases are
/// instead checkpointed immediately before and after the sort.
pub const INTERRUPT_CHECK_CHUNK: usize = 4_096;
/// Maximum uninterrupted standard-library sort run. Each run is followed by
/// checked deterministic merge passes, so UserBreak latency stays bounded
/// without replacing Rust's optimized sort kernels with comparison-by-
/// comparison host polling.
pub const INTERRUPTIBLE_SORT_RUN: usize = 262_144;

/// Caller-owned synchronous interruption check.
///
/// Implementations need not be `Send` or `Sync`; the production Stata bridge
/// deliberately supplies a caller-thread-only implementation.
pub trait InterruptCheck {
    fn checkpoint(&mut self, phase: &'static str) -> Result<()>;

    fn cancellation_token(&self) -> Option<CancellationToken> {
        None
    }
}

/// Inert checker used by every pre-existing core entry point.
#[derive(Debug, Default)]
pub struct NeverInterrupt;

impl InterruptCheck for NeverInterrupt {
    #[inline]
    fn checkpoint(&mut self, _phase: &'static str) -> Result<()> {
        Ok(())
    }
}

/// Thread-safe cancellation state shared by one caller-thread coordinator and
/// every native worker participating in the owned solve.
#[derive(Clone, Debug, Default)]
pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl CancellationToken {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, AtomicOrdering::Release);
    }

    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(AtomicOrdering::Acquire)
    }

    #[must_use]
    pub fn atomic_flag(&self) -> &AtomicBool {
        &self.cancelled
    }
}

/// Worker-safe checker backed only by [`CancellationToken`]. It never invokes
/// a host callback and is therefore safe to use from coordinator and Rayon
/// workers alike.
#[derive(Clone, Debug)]
pub struct CancellationInterrupt {
    token: CancellationToken,
}

impl CancellationInterrupt {
    #[must_use]
    pub const fn new(token: CancellationToken) -> Self {
        Self { token }
    }

    #[must_use]
    pub const fn token(&self) -> &CancellationToken {
        &self.token
    }
}

impl InterruptCheck for CancellationInterrupt {
    #[inline]
    fn checkpoint(&mut self, phase: &'static str) -> Result<()> {
        if self.token.is_cancelled() {
            Err(BackendError::new(
                ErrorCode::UserBreak,
                phase,
                "user requested interruption",
            ))
        } else {
            Ok(())
        }
    }

    fn cancellation_token(&self) -> Option<CancellationToken> {
        Some(self.token.clone())
    }
}

/// Check at the beginning of each bounded chunk without changing arithmetic
/// order or random-number addressing.
#[inline]
pub fn checkpoint_chunk(
    interrupt: &mut dyn InterruptCheck,
    index: usize,
    phase: &'static str,
) -> Result<()> {
    if index % INTERRUPT_CHECK_CHUNK == 0 {
        interrupt.checkpoint(phase)?;
    }
    Ok(())
}

/// Deterministic stable chunked sort whose run boundaries and merge copies are
/// cooperatively interruptible. Equal elements retain input order, matching
/// the semantic guarantee of `slice::sort_by`.
pub fn stable_sort_by_with_interrupt<T, F>(
    values: &mut [T],
    mut compare: F,
    interrupt: &mut dyn InterruptCheck,
    phase: &'static str,
) -> Result<()>
where
    T: Clone,
    F: FnMut(&T, &T) -> Ordering,
{
    let len = values.len();
    if len < 2 {
        interrupt.checkpoint(phase)?;
        return Ok(());
    }
    for run in values.chunks_mut(INTERRUPTIBLE_SORT_RUN) {
        interrupt.checkpoint(phase)?;
        run.sort_by(|left, right| compare(left, right));
        interrupt.checkpoint(phase)?;
    }
    if len <= INTERRUPTIBLE_SORT_RUN {
        return Ok(());
    }
    let mut buffer = fallible_sort_buffer(len, phase)?;
    for (index, value) in values.iter().enumerate() {
        checkpoint_chunk(interrupt, index, phase)?;
        buffer.push(value.clone());
    }
    let mut source_is_values = true;
    let mut width = INTERRUPTIBLE_SORT_RUN;
    while width < len {
        if source_is_values {
            merge_pass(values, &mut buffer, width, &mut compare, interrupt, phase)?;
        } else {
            merge_pass(&buffer, values, width, &mut compare, interrupt, phase)?;
        }
        source_is_values = !source_is_values;
        width = width.saturating_mul(2);
    }
    if !source_is_values {
        for (index, (destination, source)) in values.iter_mut().zip(&buffer).enumerate() {
            checkpoint_chunk(interrupt, index, phase)?;
            destination.clone_from(source);
        }
    }
    interrupt.checkpoint(phase)?;
    Ok(())
}

fn fallible_sort_buffer<T>(len: usize, phase: &'static str) -> Result<Vec<T>> {
    len.checked_mul(core::mem::size_of::<T>()).ok_or_else(|| {
        BackendError::new(
            ErrorCode::ResourceLimit,
            phase,
            "sort buffer byte-size overflow",
        )
    })?;
    let mut buffer = Vec::new();
    buffer.try_reserve_exact(len).map_err(|_| {
        BackendError::new(
            ErrorCode::AllocationFailed,
            phase,
            format!("could not allocate sort buffer with {len} elements"),
        )
    })?;
    Ok(buffer)
}

fn merge_pass<T, F>(
    source: &[T],
    destination: &mut [T],
    width: usize,
    compare: &mut F,
    interrupt: &mut dyn InterruptCheck,
    phase: &'static str,
) -> Result<()>
where
    T: Clone,
    F: FnMut(&T, &T) -> Ordering,
{
    let len = source.len();
    let mut begin = 0_usize;
    while begin < len {
        let middle = begin.saturating_add(width).min(len);
        let end = begin.saturating_add(width.saturating_mul(2)).min(len);
        let (mut left, mut right) = (begin, middle);
        for output in begin..end {
            checkpoint_chunk(interrupt, output - begin, phase)?;
            let take_left = right == end
                || (left < middle && compare(&source[left], &source[right]) != Ordering::Greater);
            if take_left {
                destination[output].clone_from(&source[left]);
                left += 1;
            } else {
                destination[output].clone_from(&source[right]);
                right += 1;
            }
        }
        begin = end;
    }
    Ok(())
}

/// Deterministic chunked unstable sort with checked deterministic merge passes.
/// This preserves standard-library sort performance while bounding the work
/// between cancellation observations.
pub fn unstable_sort_by_with_interrupt<T, F>(
    values: &mut [T],
    mut compare: F,
    interrupt: &mut dyn InterruptCheck,
    phase: &'static str,
) -> Result<()>
where
    T: Clone,
    F: FnMut(&T, &T) -> Ordering,
{
    let len = values.len();
    if len < 2 {
        interrupt.checkpoint(phase)?;
        return Ok(());
    }
    for run in values.chunks_mut(INTERRUPTIBLE_SORT_RUN) {
        interrupt.checkpoint(phase)?;
        run.sort_unstable_by(|left, right| compare(left, right));
        interrupt.checkpoint(phase)?;
    }
    if len <= INTERRUPTIBLE_SORT_RUN {
        return Ok(());
    }
    let mut buffer = fallible_sort_buffer(len, phase)?;
    for (index, value) in values.iter().enumerate() {
        checkpoint_chunk(interrupt, index, phase)?;
        buffer.push(value.clone());
    }
    let mut source_is_values = true;
    let mut width = INTERRUPTIBLE_SORT_RUN;
    while width < len {
        if source_is_values {
            merge_pass(values, &mut buffer, width, &mut compare, interrupt, phase)?;
        } else {
            merge_pass(&buffer, values, width, &mut compare, interrupt, phase)?;
        }
        source_is_values = !source_is_values;
        width = width.saturating_mul(2);
    }
    if !source_is_values {
        for (index, (destination, source)) in values.iter_mut().zip(&buffer).enumerate() {
            checkpoint_chunk(interrupt, index, phase)?;
            destination.clone_from(source);
        }
    }
    interrupt.checkpoint(phase)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::{BackendError, ErrorCode};

    struct BreakAfter {
        calls: usize,
        stop: usize,
    }

    impl InterruptCheck for BreakAfter {
        fn checkpoint(&mut self, phase: &'static str) -> Result<()> {
            self.calls += 1;
            if self.calls == self.stop {
                Err(BackendError::new(
                    ErrorCode::UserBreak,
                    phase,
                    "injected sort break",
                ))
            } else {
                Ok(())
            }
        }
    }

    #[test]
    fn cancellable_large_sorts_match_legacy_order_and_break_inside_work() {
        let length = INTERRUPTIBLE_SORT_RUN * 2 + 9_003;
        let mut stable = (0..length)
            .rev()
            .map(|index| (index % 17, index))
            .collect::<Vec<_>>();
        let mut expected_stable = stable.clone();
        expected_stable.sort_by_key(|value| value.0);
        stable_sort_by_with_interrupt(
            &mut stable,
            |left, right| left.0.cmp(&right.0),
            &mut NeverInterrupt,
            "stable_sort_test",
        )
        .expect("stable sort");
        assert_eq!(stable, expected_stable);

        let mut unstable = (0..u64::try_from(length).unwrap())
            .rev()
            .collect::<Vec<_>>();
        let mut expected_unstable = unstable.clone();
        expected_unstable.sort_unstable();
        unstable_sort_by_with_interrupt(
            &mut unstable,
            Ord::cmp,
            &mut NeverInterrupt,
            "unstable_sort_test",
        )
        .expect("unstable sort");
        assert_eq!(unstable, expected_unstable);

        let mut interrupted = (0..u64::try_from(length).unwrap())
            .rev()
            .collect::<Vec<_>>();
        let mut breaker = BreakAfter { calls: 0, stop: 4 };
        let error = unstable_sort_by_with_interrupt(
            &mut interrupted,
            Ord::cmp,
            &mut breaker,
            "unstable_sort_test",
        )
        .expect_err("sort must be interruptible between optimized runs");
        assert_eq!(error.code, ErrorCode::UserBreak);
        assert_eq!(breaker.calls, 4);
    }

    #[test]
    fn stable_sort_buffer_overflow_is_typed_before_allocation() {
        let error = fallible_sort_buffer::<u64>(usize::MAX, "sort_overflow")
            .expect_err("byte-size overflow");
        assert_eq!(error.code, ErrorCode::ResourceLimit);
        assert_eq!(error.phase, "sort_overflow");
    }
}
