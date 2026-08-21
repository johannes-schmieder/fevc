// SPDX-License-Identifier: GPL-3.0-only

#[path = "../src/context.rs"]
mod context;

use context::{ContextRegistry, ContextStateTag};
use vckss_core::error::{BackendError, ErrorCode};

#[test]
fn prepare_solve_export_and_idempotent_release() {
    let mut registry = ContextRegistry::<Vec<u64>, u64>::new();
    let handle = registry.prepare(vec![2, 3, 5]).expect("prepare");
    assert_eq!(registry.snapshot().state, ContextStateTag::Prepared);

    registry
        .solve(handle, |values| Ok(values.into_iter().sum()))
        .expect("solve");
    assert_eq!(registry.snapshot().state, ContextStateTag::Solved);
    assert_eq!(*registry.result(handle).expect("result"), 10);

    assert!(registry.release(handle).expect("first release"));
    assert!(!registry.release(handle).expect("idempotent release"));
    assert_eq!(registry.snapshot().state, ContextStateTag::Empty);
}

#[test]
fn a_second_prepare_is_rejected_until_release() {
    let mut registry = ContextRegistry::<u64, u64>::new();
    let handle = registry.prepare(7).expect("first prepare");
    let error = registry.prepare(8).expect_err("second prepare must fail");
    assert_eq!(error.code, ErrorCode::ContextPoisoned);
    assert!(registry.release(handle).expect("release"));
    let second = registry.prepare(8).expect("prepare after release");
    assert!(second.generation() > handle.generation());
}

#[test]
fn stale_handles_cannot_access_a_new_generation() {
    let mut registry = ContextRegistry::<u64, u64>::new();
    let first = registry.prepare(3).expect("first prepare");
    assert!(registry.release(first).expect("first release"));
    let second = registry.prepare(4).expect("second prepare");

    assert_eq!(
        registry.result(first).expect_err("stale export").code,
        ErrorCode::StaleContext
    );
    assert_eq!(
        registry.release(first).expect_err("stale release").code,
        ErrorCode::StaleContext
    );
    registry.solve(second, |value| Ok(value * 2)).expect("solve");
    assert_eq!(*registry.result(second).expect("result"), 8);
}

#[test]
fn solver_errors_are_terminal_and_reexported() {
    let mut registry = ContextRegistry::<u64, u64>::new();
    let handle = registry.prepare(3).expect("prepare");
    let expected = BackendError::new(
        ErrorCode::FullResidualFailed,
        "test_solve",
        "certificate failed",
    );
    let error = registry
        .solve(handle, |_| Err(expected.clone()))
        .expect_err("solve must fail");
    assert_eq!(error, expected);
    assert_eq!(registry.snapshot().state, ContextStateTag::Failed);
    assert_eq!(registry.result(handle).expect_err("failed export"), expected);
    assert!(registry.release(handle).expect("release failed context"));
}

#[test]
fn panics_are_contained_and_poison_the_generation() {
    let mut registry = ContextRegistry::<u64, u64>::new();
    let handle = registry.prepare(3).expect("prepare");
    let error = registry
        .solve(handle, |_| -> vckss_core::error::Result<u64> {
            panic!("deliberate test panic")
        })
        .expect_err("panic must be contained");
    assert_eq!(error.code, ErrorCode::Panic);
    assert_eq!(registry.snapshot().state, ContextStateTag::Poisoned);
    assert_eq!(
        registry.result(handle).expect_err("poisoned export").code,
        ErrorCode::ContextPoisoned
    );
    assert!(registry.release(handle).expect("release poisoned context"));
}

#[test]
fn abandoned_context_cleanup_is_generation_safe() {
    let mut registry = ContextRegistry::<u64, u64>::new();
    let first = registry.prepare(10).expect("prepare");
    assert_eq!(registry.clear_abandoned(), Some(first));
    assert_eq!(registry.snapshot().state, ContextStateTag::Empty);
    assert!(!registry.release(first).expect("old release is idempotent"));

    let second = registry.prepare(11).expect("new prepare");
    assert!(second.generation() > first.generation());
    assert_eq!(registry.clear_abandoned(), Some(second));
    assert_eq!(registry.clear_abandoned(), None);
}
