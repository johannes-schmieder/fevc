//! Observe complete parallel-process allocation peaks without labelling Rayon
//! storage as CMG-owned. Failure injection is only at the two caller-side fixed
//! mapping banks, before contraction dispatch, never in an infallible runtime
//! allocation. Forest dispatch can already have run before those banks.
#![cfg(feature = "parallel")]
use cmg::*;
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicUsize, Ordering::SeqCst};

static LIVE: AtomicIsize = AtomicIsize::new(0);
static PEAK: AtomicIsize = AtomicIsize::new(0);
thread_local! {
    static CAPTURE_CALLER: Cell<bool> = const { Cell::new(false) };
    static CALLS: Cell<usize> = const { Cell::new(0) };
    static FAIL_SIZE: Cell<Option<usize>> = const { Cell::new(None) };
}
// Allocation origins, not allocating/freeing-thread net flows. This fixed
// ledger adds no allocations or headers to the observed process. Only the
// initiating caller opts in; any thread can free or reallocate its pointers.
// No assertions, formatting or allocating callbacks run under the guard.
static COHORT_ACTIVE: AtomicBool = AtomicBool::new(false);
static COHORT_LOCK: AtomicBool = AtomicBool::new(false);
static COHORT_OVERFLOW: AtomicBool = AtomicBool::new(false);
static COHORT_POINTERS: [AtomicUsize; 4096] = [const { AtomicUsize::new(0) }; 4096];

struct CohortGuard;
impl CohortGuard {
    fn lock() -> Self {
        while COHORT_LOCK
            .compare_exchange(false, true, SeqCst, SeqCst)
            .is_err()
        {
            std::hint::spin_loop();
        }
        Self
    }
}
impl Drop for CohortGuard {
    fn drop(&mut self) {
        COHORT_LOCK.store(false, SeqCst);
    }
}
fn cohort_insert(ptr: *mut u8) {
    if !CAPTURE_CALLER.try_with(Cell::get).unwrap_or(false) {
        return;
    }
    let _guard = CohortGuard::lock();
    if let Some(slot) = COHORT_POINTERS.iter().find(|slot| slot.load(SeqCst) == 0) {
        slot.store(ptr as usize, SeqCst);
    } else {
        COHORT_OVERFLOW.store(true, SeqCst);
    }
}
fn cohort_remove(ptr: *mut u8) {
    if COHORT_ACTIVE.load(SeqCst) {
        let _guard = CohortGuard::lock();
        if let Some(slot) = COHORT_POINTERS
            .iter()
            .find(|slot| slot.load(SeqCst) == ptr as usize)
        {
            slot.store(0, SeqCst);
        }
    }
}
fn cohort_count() -> usize {
    let _guard = CohortGuard::lock();
    COHORT_POINTERS
        .iter()
        .filter(|slot| slot.load(SeqCst) != 0)
        .count()
}
fn cohort_clean() -> bool {
    cohort_count() == 0 && !COHORT_OVERFLOW.load(SeqCst)
}
fn cohort_begin() {
    assert!(!COHORT_ACTIVE.load(SeqCst));
    assert!(cohort_clean(), "previous allocation cohort was not cleaned");
    COHORT_ACTIVE.store(true, SeqCst);
    CAPTURE_CALLER.with(|capture| capture.set(true));
}
fn cohort_pause() {
    CAPTURE_CALLER.with(|capture| capture.set(false));
}
fn cohort_finish() {
    cohort_pause();
    assert!(
        cohort_clean(),
        "partial build leaked or allocation ledger overflowed"
    );
    COHORT_ACTIVE.store(false, SeqCst);
}

// These controls share the test's serial lifetime. A held allocation must fail
// the same exact-clean predicate used by construction, then become clean after
// its actual release. Capture is paused around thread runtime creation.
fn check_allocation_origins() {
    cohort_begin();
    let owned = std::hint::black_box(Box::new([0_u8; 64]));
    cohort_pause();
    assert_eq!(cohort_count(), 1);
    assert!(
        !cohort_clean(),
        "a deliberately held allocation escaped tracking"
    );
    std::thread::spawn(move || drop(owned)).join().unwrap();
    cohort_finish();

    // A worker-origin allocation made outside capture must never be charged
    // when the initiating caller receives and frees it.
    cohort_begin();
    cohort_pause();
    let mut foreign = std::thread::spawn(|| Vec::<u8>::with_capacity(64))
        .join()
        .unwrap();
    assert_eq!(cohort_count(), 0);
    CAPTURE_CALLER.with(|capture| capture.set(true));
    foreign.try_reserve_exact(128).unwrap();
    cohort_pause();
    assert_eq!(
        cohort_count(),
        0,
        "realloc changed an untracked allocation's origin"
    );
    drop(foreign);
    cohort_finish();

    // Capture on a worker, reallocate on the caller, then free on another
    // worker. Both failed and successful realloc preserve the original cohort.
    let mut transferred = std::thread::spawn(|| {
        cohort_begin();
        let value = std::hint::black_box(Vec::<u8>::with_capacity(64));
        cohort_pause();
        value
    })
    .join()
    .unwrap();
    assert_eq!(cohort_count(), 1);
    FAIL_SIZE.with(|slot| slot.set(Some(128)));
    assert!(transferred.try_reserve_exact(128).is_err());
    assert_eq!(FAIL_SIZE.with(|slot| slot.replace(None)), None);
    assert_eq!(cohort_count(), 1);
    transferred.try_reserve_exact(128).unwrap();
    assert_eq!(cohort_count(), 1);
    std::thread::spawn(move || drop(transferred))
        .join()
        .unwrap();
    cohort_finish();
}

struct Observed;
fn fail(size: usize) -> bool {
    let _ = CALLS.try_with(|calls| calls.set(calls.get() + 1));
    FAIL_SIZE
        .try_with(|slot| {
            if slot.get() == Some(size) {
                slot.set(None);
                true
            } else {
                false
            }
        })
        .unwrap_or(false)
}
fn allocated(size: usize) {
    let live = LIVE.fetch_add(size as isize, SeqCst) + size as isize;
    PEAK.fetch_max(live, SeqCst);
}
unsafe impl GlobalAlloc for Observed {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if fail(layout.size()) {
            return std::ptr::null_mut();
        }
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            allocated(layout.size());
            cohort_insert(ptr);
        }
        ptr
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        LIVE.fetch_sub(layout.size() as isize, SeqCst);
        // Remove before the address can be reused by another allocation.
        cohort_remove(ptr);
        unsafe {
            System.dealloc(ptr, layout);
        }
    }
    unsafe fn realloc(&self, ptr: *mut u8, old: Layout, size: usize) -> *mut u8 {
        if fail(size) {
            return std::ptr::null_mut();
        }
        // Keep the old address protected through realloc: it may be freed
        // and reused before System returns. An untracked pointer stays untracked.
        let guard = COHORT_ACTIVE.load(SeqCst).then(CohortGuard::lock);
        let slot = guard.as_ref().and_then(|_| {
            COHORT_POINTERS
                .iter()
                .find(|slot| slot.load(SeqCst) == ptr as usize)
        });
        let result = unsafe { System.realloc(ptr, old, size) };
        if !result.is_null() {
            if let Some(slot) = slot {
                slot.store(result as usize, SeqCst);
            }
        }
        drop(guard);
        if !result.is_null() {
            // Conservative observation of old/new overlap, as in serial fixtures.
            allocated(size);
            LIVE.fetch_sub(old.size() as isize, SeqCst);
        }
        result
    }
}
#[global_allocator]
static ALLOCATOR: Observed = Observed;

fn report(roots: &[&dyn CmgStorage]) -> usize {
    let calls = CALLS.with(Cell::get);
    let bytes = CmgStorageReport::measure(roots)
        .unwrap()
        .total_bytes()
        .unwrap();
    assert_eq!(CALLS.with(Cell::get), calls);
    bytes
}

fn check(graph: &Laplacian, executor: &ParallelExecutor, options: CmgOptions) {
    let bound = CmgPreconditioner::construction_allocation_bound(
        graph.vertex_count(),
        graph.edge_count(),
        cmg::BuildOptions::from(options),
        true,
    )
    .unwrap();
    let calls = CALLS.with(Cell::get);
    assert!(matches!(
        CmgPreconditioner::build_with(
            graph,
            cmg::BuildOptions::from(options),
            cmg::BuildContext {
                team: Some(executor.team()),
                budget_bytes: Some(bound - 1),
                ..cmg::BuildContext::default()
            }
        ),
        Err(CmgError::MemoryBudgetExceeded { .. })
    ));
    assert_eq!(
        CALLS.with(Cell::get),
        calls,
        "short budget allocated before rejection"
    );
    let reference = CmgPreconditioner::build_with(
        graph,
        cmg::BuildOptions::from(options),
        cmg::BuildContext {
            team: Some(executor.team()),
            ..cmg::BuildContext::default()
        },
    )
    .unwrap();
    let parents = report(&[graph, &reference]);
    let before = LIVE.load(SeqCst);
    PEAK.store(before, SeqCst);
    let pre = CmgPreconditioner::build_with(
        graph,
        cmg::BuildOptions::from(options),
        cmg::BuildContext {
            team: Some(executor.team()),
            budget_bytes: Some(bound),
            ..cmg::BuildContext::default()
        },
    )
    .unwrap();
    let peak = (PEAK.load(SeqCst) - before).max(0);
    assert_eq!(pre, reference);
    let retained = report(&[graph, &reference, &pre]) - parents;
    assert!(retained <= bound);
    let hierarchy_bound = CmgHierarchy::construction_allocation_bound(
        graph.vertex_count(),
        graph.edge_count(),
        options,
        true,
    )
    .unwrap();
    let calls = CALLS.with(Cell::get);
    assert!(
        CmgHierarchy::build_with(
            graph,
            cmg::BuildOptions::from(options),
            cmg::BuildContext {
                team: Some(executor.team()),
                budget_bytes: Some(hierarchy_bound - 1),
                ..cmg::BuildContext::default()
            }
        )
        .is_err()
    );
    assert_eq!(CALLS.with(Cell::get), calls);
    // Compare the same construction stage. Preconditioner finalization replaces
    // the penultimate repeat using terminal-factor nonzeros; a standalone
    // hierarchy still uses coarse-graph nonzeros. The exact preconditioner
    // comparison above separately checks that finalization is preserved.
    assert_eq!(
        CmgHierarchy::build_with(
            graph,
            cmg::BuildOptions::from(options),
            cmg::BuildContext {
                team: Some(executor.team()),
                budget_bytes: Some(hierarchy_bound),
                ..cmg::BuildContext::default()
            }
        )
        .unwrap(),
        CmgHierarchy::build_with(
            graph,
            cmg::BuildOptions::from(options),
            cmg::BuildContext {
                team: Some(executor.team()),
                ..cmg::BuildContext::default()
            }
        )
        .unwrap()
    );
    println!(
        "PARALLEL_CMG_CONSTRUCTION_SUCCESS vertices={} edges={} workers={} cmg_bound={bound} retained_new={retained} observed_process_peak_increment={peak} runtime_included_in_observation=true runtime_admitted=false",
        graph.vertex_count(),
        graph.edge_count(),
        executor.thread_count()
    );

    // For these dense fixtures the first level necessarily contracts the 50
    // disconnected pairs. Forest dispatch precedes the mapping banks; the
    // caller-origin cohort follows their scratch across threads through cleanup.
    if executor.thread_count() > 1 && graph.edge_count() >= 65_536 {
        for size in [
            std::mem::size_of::<Option<Edge>>(),
            std::mem::size_of::<Edge>(),
        ] {
            cohort_begin();
            FAIL_SIZE.with(|slot| slot.set(Some(size * graph.edge_count())));
            let result = CmgPreconditioner::build_with(
                graph,
                cmg::BuildOptions::from(options),
                cmg::BuildContext {
                    team: Some(executor.team()),
                    budget_bytes: Some(bound),
                    ..cmg::BuildContext::default()
                },
            );
            cohort_pause();
            let unconsumed = FAIL_SIZE.with(|slot| slot.replace(None));
            assert!(
                unconsumed.is_none(),
                "injection did not reach fixed mapping bank"
            );
            assert!(matches!(result, Err(CmgError::AllocationFailed { .. })));
            drop(result);
            cohort_finish();
        }
    }
    #[cfg(feature = "experimental-components")]
    for prune in [false, true] {
        for preserve in [false, true] {
            for factor in [false, true] {
                let component = ComponentBuildOptions {
                    prune_coarse_isolates: prune,
                    preserve_unpruned_stopping: preserve,
                    factor_terminal_components: factor,
                };
                let bound = CmgPreconditioner::construction_allocation_bound(
                    graph.vertex_count(),
                    graph.edge_count(),
                    cmg::BuildOptions {
                        hierarchy: options,
                        components: component,
                    },
                    true,
                )
                .unwrap();
                let calls = CALLS.with(Cell::get);
                assert!(
                    CmgPreconditioner::build_with(
                        graph,
                        cmg::BuildOptions {
                            hierarchy: options,
                            components: component
                        },
                        cmg::BuildContext {
                            team: Some(executor.team()),
                            budget_bytes: Some(bound - 1),
                            ..cmg::BuildContext::default()
                        }
                    )
                    .is_err()
                );
                assert_eq!(CALLS.with(Cell::get), calls);
                let admitted = CmgPreconditioner::build_with(
                    graph,
                    cmg::BuildOptions {
                        hierarchy: options,
                        components: component,
                    },
                    cmg::BuildContext {
                        team: Some(executor.team()),
                        budget_bytes: Some(bound),
                        ..cmg::BuildContext::default()
                    },
                )
                .unwrap();
                let legacy = CmgPreconditioner::build_with(
                    graph,
                    cmg::BuildOptions {
                        hierarchy: options,
                        components: component,
                    },
                    cmg::BuildContext {
                        team: Some(executor.team()),
                        ..cmg::BuildContext::default()
                    },
                )
                .unwrap();
                assert_eq!(admitted, legacy);
                let known: Vec<_> = (0..graph.vertex_count()).map(|v| (v % 13) as f64).collect();
                let rhs = graph.matvec(&known).unwrap();
                assert_eq!(admitted.apply(&rhs).unwrap(), legacy.apply(&rhs).unwrap());
                assert!(report(&[graph, &admitted]) - report(&[graph]) <= bound);
            }
        }
    }
}

#[test]
fn fixed_parallel_banks_preserve_thresholds_policies_and_cleanup() {
    check_allocation_origins();
    let options = CmgOptions {
        direct_threshold: 64,
        max_levels: 8,
        ..Default::default()
    };
    for threads in [1, 4, 1] {
        let executor = ParallelExecutor::new(ParallelOptions {
            threads,
            min_parallel_len: 1,
            ..Default::default()
        })
        .unwrap();
        for width in [255, 256, 257] {
            let core = 2 * width;
            let graph = Laplacian::from_edges(
                core + 100,
                (0..width)
                    .flat_map(|u| (width..core).map(move |v| (u, v, 1.0)))
                    .chain((core..core + 100).step_by(2).map(|u| (u, u + 1, 0.5))),
            )
            .unwrap();
            check(&graph, &executor, options);
        }
        let endpoints: Vec<_> = (1..73).map(|v| (v - 1, v)).collect();
        let topology = PreparedLaplacianTopology::prepare(79, endpoints).unwrap();
        let mut workspace = topology.workspace().unwrap();
        for weight in [1.0, 0.125, 2.0] {
            let graph = topology
                .assemble_with_workspace(&vec![weight; 72], &mut workspace)
                .unwrap();
            check(&graph, &executor, options);
        }
        assert!(
            CmgStorageReport::measure(&[&executor])
                .unwrap()
                .runtime_heap_unbounded()
        );
        drop(executor); // Existing TLS lifetime tests verify the actual joins.
    }
    for (n, m) in [(usize::MAX, 0), (1, usize::MAX)] {
        assert!(CmgHierarchy::construction_allocation_bound(n, m, options, true).is_err());
        assert!(
            CmgPreconditioner::construction_allocation_bound(
                n,
                m,
                cmg::BuildOptions::from(options),
                true
            )
            .is_err()
        );
    }
    assert_eq!(ParallelExecutor::runtime_allocation_bound(), None);
    assert!(
        ParallelExecutor::new_with_memory_budget(ParallelOptions::default(), usize::MAX).is_err()
    );
}
