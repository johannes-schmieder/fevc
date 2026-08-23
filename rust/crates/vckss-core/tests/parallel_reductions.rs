// SPDX-License-Identifier: GPL-3.0-only

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use vckss_core::parallel::deterministic_sum;

struct CountingAllocator;

static COUNT_ENABLED: AtomicBool = AtomicBool::new(false);
static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);

#[global_allocator]
static GLOBAL_ALLOCATOR: CountingAllocator = CountingAllocator;

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if COUNT_ENABLED.load(Ordering::Relaxed) {
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        }
        // SAFETY: Delegates the unchanged allocation request to the system allocator.
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        if COUNT_ENABLED.load(Ordering::Relaxed) {
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        }
        // SAFETY: Delegates the unchanged allocation request to the system allocator.
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        // SAFETY: The pointer and layout were supplied by the allocation caller.
        unsafe { System.dealloc(pointer, layout) }
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        if COUNT_ENABLED.load(Ordering::Relaxed) {
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        }
        // SAFETY: Delegates the unchanged reallocation request to the system allocator.
        unsafe { System.realloc(pointer, layout, new_size) }
    }
}

struct AllocationWindow;

impl AllocationWindow {
    fn start() -> Self {
        ALLOCATIONS.store(0, Ordering::Relaxed);
        COUNT_ENABLED.store(true, Ordering::SeqCst);
        Self
    }
}

impl Drop for AllocationWindow {
    fn drop(&mut self) {
        COUNT_ENABLED.store(false, Ordering::SeqCst);
    }
}

fn reference_tree(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mut current = values.to_vec();
    while current.len() > 1 {
        let mut next = Vec::with_capacity(current.len().div_ceil(2));
        for pair in current.chunks(2) {
            next.push(if pair.len() == 2 {
                pair[0] + pair[1]
            } else {
                pair[0]
            });
        }
        current = next;
    }
    current[0]
}

fn patterned_values(length: usize) -> Vec<f64> {
    const PATTERN: [f64; 17] = [
        1.0e16, 1.0, -1.0e16, 3.0, -7.25, 0.125, 11.0, -2.0, 5.5, -13.0, 0.0, 9.75, -0.5, 4.0,
        2.25, -6.0, 8.0,
    ];
    (0..length)
        .map(|index| PATTERN[index % PATTERN.len()])
        .collect()
}

#[test]
fn deterministic_sum_preserves_the_tree_with_one_workspace_allocation() {
    for length in [0, 1, 2, 3, 7, 16, 31, 64, 127, 1_025] {
        let values = patterned_values(length);
        assert_eq!(
            deterministic_sum(&values).to_bits(),
            reference_tree(&values).to_bits(),
            "fixed reduction tree changed at length {length}"
        );
    }

    let values = patterned_values(65_537);
    let expected = reference_tree(&values).to_bits();
    const CALLS: usize = 16;

    let window = AllocationWindow::start();
    let mut observed = 0_u64;
    for _ in 0..CALLS {
        observed = std::hint::black_box(deterministic_sum(std::hint::black_box(&values))).to_bits();
    }
    drop(window);

    assert_eq!(observed, expected);
    let allocations = ALLOCATIONS.load(Ordering::Relaxed);
    assert!(
        allocations >= CALLS,
        "expected at least one workspace allocation per call, observed {allocations}"
    );
    assert!(
        allocations <= CALLS + 4,
        "deterministic reduction allocated {allocations} times for {CALLS} calls"
    );
}
