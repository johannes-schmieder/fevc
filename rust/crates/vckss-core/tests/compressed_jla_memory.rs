// SPDX-License-Identifier: GPL-3.0-only
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;
use vckss_core::batch_plan::BatchRequest;
use vckss_core::engine::{
    run_jla_no_controls_planned_with_interrupt, JlaEngineOptions, PlannedJlaEngineOptions,
};
use vckss_core::full_cmg::FullCmgPlanOptions;
use vckss_core::interrupt::NeverInterrupt;
use vckss_core::memory::MemoryBudget;
use vckss_core::problem::{CanonicalInput, CompressedProblem};
use vckss_core::solver::{LinearSolverOptions, LinearSolverRoute};
use vckss_core::types::InputColumns;
struct TrackingAllocator;

static TRACKING: AtomicBool = AtomicBool::new(false);
static CURRENT: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);
static TEST_LOCK: Mutex<()> = Mutex::new(());

fn record_allocation(bytes: usize) {
    let current = CURRENT.fetch_add(bytes, Ordering::Relaxed) + bytes;
    if TRACKING.load(Ordering::Relaxed) {
        PEAK.fetch_max(current, Ordering::Relaxed);
    }
}

fn record_deallocation(bytes: usize) {
    CURRENT.fetch_sub(bytes, Ordering::Relaxed);
}

// SAFETY: System receives every request unchanged; the atomics only observe
// requested payload bytes and never affect allocation or deallocation.
unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: forwarded unchanged to the process system allocator.
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() {
            record_allocation(layout.size());
        }
        pointer
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // SAFETY: forwarded unchanged to the process system allocator.
        let pointer = unsafe { System.alloc_zeroed(layout) };
        if !pointer.is_null() {
            record_allocation(layout.size());
        }
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        record_deallocation(layout.size());
        // SAFETY: caller supplies the pointer/layout pair issued by this
        // forwarding allocator.
        unsafe { System.dealloc(pointer, layout) };
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // SAFETY: forwarded unchanged to the process system allocator.
        let replacement = unsafe { System.realloc(pointer, layout, new_size) };
        if !replacement.is_null() {
            if new_size >= layout.size() {
                record_allocation(new_size - layout.size());
            } else {
                record_deallocation(layout.size() - new_size);
            }
        }
        replacement
    }
}

#[global_allocator]
static ALLOCATOR: TrackingAllocator = TrackingAllocator;

fn fixture(n: usize, degree: usize) -> CompressedProblem {
    let firms = n / 120;
    let mut input = InputColumns {
        worker: vec![],
        firm: vec![],
        deletion: vec![],
        outcome: vec![],
        frequency: vec![],
        target_weight: vec![],
        controls: vec![],
    };
    for row in 0..n {
        let worker = row / degree;
        let period = row % degree;
        let layer = worker / firms;
        let offset = match period {
            0 => 0,
            1 => 1 + layer % (firms / 4 - 1),
            2 => firms.div_ceil(3) + (97 * layer) % (firms / 4),
            _ => 2 * firms / 3 + (53 * layer) % (firms / 4),
        };
        let firm = (worker % firms + offset) % firms;
        input.worker.push(worker as u64 + 1);
        input.firm.push(firm as u64 + 1);
        input.deletion.push(row as u64 + 1);
        input.outcome.push(
            (worker % 257) as f64 / 16.0
                + (firm % 127) as f64 / 32.0
                + period as f64 / 64.0
                + (row % 13) as f64 / 128.0,
        );
        input.frequency.push(1);
        input.target_weight.push(1.0);
    }
    CanonicalInput::from_validated(input.validate().unwrap())
        .unwrap()
        .compress(&vec![true; n])
        .unwrap()
}

#[test]
fn compressed_forecast_allocation_measurement() {
    let _lock = TEST_LOCK.lock().unwrap();
    for (n, threads, degree) in [
        (30_720, 1, 3),
        (61_440, 4, 3),
        (245_760, 4, 3),
        (61_440, 4, 4),
    ] {
        assert!(!TRACKING.swap(true, Ordering::SeqCst));
        // Pool thread-local destructors may outlive a previous solve. Count
        // all allocations continuously so a late free never debits a reset
        // counter for the next fixture.
        let baseline = CURRENT.load(Ordering::SeqCst);
        PEAK.store(baseline, Ordering::SeqCst);
        let problem = fixture(n, degree);
        let current = CURRENT.load(Ordering::SeqCst);
        let live = current.saturating_sub(baseline);
        PEAK.store(current, Ordering::SeqCst);
        let result = run_jla_no_controls_planned_with_interrupt(
            &problem,
            PlannedJlaEngineOptions {
                estimator: JlaEngineOptions {
                    memory_budget: MemoryBudget::Unspecified,
                    memory_limit_bytes: 0,
                    solver: LinearSolverOptions {
                        route: LinearSolverRoute::CmgPcg,
                        ..Default::default()
                    },
                    ..Default::default()
                },
                leverage_batch: BatchRequest::Auto,
                target_batch: BatchRequest::Auto,
                wallseconds: None,
                full_cmg: Some(FullCmgPlanOptions::production(threads, 1.0e-10, None)),
            },
            &mut NeverInterrupt,
        )
        .unwrap();
        let peak = PEAK.load(Ordering::SeqCst).saturating_sub(baseline);
        TRACKING.store(false, Ordering::SeqCst);
        let forecast = result.estimator.receipt.memory.expected_peak_forecast_bytes;
        eprintln!("MEMORY n={n} threads={threads} degree={degree} prepared={live} observed={peak} forecast={forecast}");
        assert!(
            forecast >= peak as u64,
            "forecast must cover independently observed allocation peak"
        );
        assert!(
            forecast - peak as u64 <= (peak as u64 / 20).max(1_048_576),
            "forecast exceeds registered accuracy tolerance"
        );
    }
}
