// SPDX-License-Identifier: GPL-3.0-only

//! Requested Rust heap payload, independent of allocator residency and RSS.
//! Only preparation uses a scoped peak. All requests still reach System
//! unchanged; this meter never rejects an allocation or changes its layout.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, MutexGuard};

struct Meter;
static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);
static SCOPE: Mutex<()> = Mutex::new(());

fn add(bytes: usize) {
    let live = LIVE.fetch_add(bytes, Ordering::Relaxed) + bytes;
    PEAK.fetch_max(live, Ordering::Relaxed);
}

// SAFETY: Every pointer, layout and reallocation is forwarded unchanged to
// System. Accounting touches only independent atomics and cannot allocate.
unsafe impl GlobalAlloc for Meter {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: forwarding the caller's allocation request.
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() {
            add(layout.size());
        }
        pointer
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // SAFETY: forwarding the caller's allocation request.
        let pointer = unsafe { System.alloc_zeroed(layout) };
        if !pointer.is_null() {
            add(layout.size());
        }
        pointer
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        LIVE.fetch_sub(layout.size(), Ordering::Relaxed);
        // SAFETY: forwarding the pointer/layout issued by System.
        unsafe { System.dealloc(pointer, layout) };
    }
    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        // SAFETY: forwarding the pointer/layout and new requested size.
        let replacement = unsafe { System.realloc(pointer, layout, size) };
        if !replacement.is_null() {
            if size >= layout.size() {
                add(size - layout.size());
            } else {
                LIVE.fetch_sub(layout.size() - size, Ordering::Relaxed);
            }
        }
        replacement
    }
}

#[global_allocator]
static ALLOCATOR: Meter = Meter;

pub(crate) struct PreparationPeak {
    baseline: usize,
    _scope: MutexGuard<'static, ()>,
}
impl PreparationPeak {
    pub(crate) fn begin() -> Self {
        let scope = SCOPE
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let baseline = LIVE.load(Ordering::Relaxed);
        PEAK.store(baseline, Ordering::Relaxed);
        Self {
            baseline,
            _scope: scope,
        }
    }
    pub(crate) fn bytes(&self) -> u64 {
        PEAK.load(Ordering::Relaxed).saturating_sub(self.baseline) as u64
    }
}
