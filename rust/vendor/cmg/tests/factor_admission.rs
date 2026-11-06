//! Independent requested-layout observations for admitted terminal construction.
//! Failure injection is restricted to the fallible constructor, never a process cap.
use cmg::*;
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

thread_local! {
    static LIVE: Cell<isize> = const { Cell::new(0) };
    static PEAK: Cell<isize> = const { Cell::new(0) };
    static CALLS: Cell<usize> = const { Cell::new(0) };
    static FAIL: Cell<Option<usize>> = const { Cell::new(None) };
}
struct Observed;
fn allocated(bytes: usize) {
    let _ = LIVE.try_with(|live| {
        live.set(live.get() + bytes as isize);
        let _ = PEAK.try_with(|peak| peak.set(peak.get().max(live.get())));
    });
}
fn fail() -> bool {
    let _ = CALLS.try_with(|calls| calls.set(calls.get() + 1));
    FAIL.try_with(|fail| match fail.get() {
        Some(0) => {
            fail.set(None);
            true
        }
        Some(n) => {
            fail.set(Some(n - 1));
            false
        }
        None => false,
    })
    .unwrap_or(false)
}
unsafe impl GlobalAlloc for Observed {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if fail() {
            return std::ptr::null_mut();
        }
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            allocated(layout.size());
        }
        ptr
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let _ = LIVE.try_with(|live| live.set(live.get() - layout.size() as isize));
        unsafe { System.dealloc(ptr, layout) };
    }
    unsafe fn realloc(&self, ptr: *mut u8, old: Layout, size: usize) -> *mut u8 {
        if fail() {
            return std::ptr::null_mut();
        }
        let ptr = unsafe { System.realloc(ptr, old, size) };
        if !ptr.is_null() {
            allocated(size);
            let _ = LIVE.try_with(|live| live.set(live.get() - old.size() as isize));
        }
        ptr
    }
}
#[global_allocator]
static ALLOCATOR: Observed = Observed;
fn live() -> isize {
    LIVE.with(Cell::get)
}

fn check(graph: &Laplacian, component: bool) {
    let (bound, reference) = if component {
        #[cfg(feature = "experimental-components")]
        {
            (
                GroundedLdl::component_construction_allocation_bound(
                    graph.vertex_count(),
                    graph.edge_count(),
                )
                .unwrap(),
                GroundedLdl::factor_by_component(graph).unwrap(),
            )
        }
        #[cfg(not(feature = "experimental-components"))]
        unreachable!()
    } else {
        (
            GroundedLdl::construction_allocation_bound(graph.vertex_count()).unwrap(),
            GroundedLdl::factor(graph).unwrap(),
        )
    };
    let construct = |budget| {
        if component {
            #[cfg(feature = "experimental-components")]
            {
                GroundedLdl::factor_with(
                    graph,
                    true,
                    cmg::BuildContext {
                        budget_bytes: Some(budget),
                        ..cmg::BuildContext::default()
                    },
                )
            }
            #[cfg(not(feature = "experimental-components"))]
            unreachable!()
        } else {
            GroundedLdl::factor_with(
                graph,
                false,
                cmg::BuildContext {
                    budget_bytes: Some(budget),
                    ..cmg::BuildContext::default()
                },
            )
        }
    };
    let before = live();
    let calls = CALLS.with(Cell::get);
    assert!(bound > 0);
    assert!(construct(bound - 1).is_err());
    assert_eq!(CALLS.with(Cell::get), calls, "short budget allocated");
    assert_eq!(live(), before);
    PEAK.with(|peak| peak.set(before));
    let factor = construct(bound).unwrap();
    let allocations = CALLS.with(Cell::get) - calls;
    let peak = (PEAK.with(Cell::get) - before) as usize;
    assert!(peak <= bound, "component={component}: {peak} > {bound}");
    assert_eq!(
        factor, reference,
        "provisioning changed factor coefficients"
    );
    let calls_before_report = CALLS.with(Cell::get);
    assert_eq!(
        CmgStorageReport::measure(&[&factor])
            .unwrap()
            .total_bytes()
            .unwrap(),
        (live() - before) as usize,
    );
    assert_eq!(CALLS.with(Cell::get), calls_before_report);
    // Replacing a retained factor requires another entire construction budget;
    // an existing owner is not included in the incremental admission query.
    let old_bytes = (live() - before) as usize;
    let replacement = construct(bound).unwrap();
    assert_eq!(
        CmgStorageReport::measure(&[&factor, &replacement])
            .unwrap()
            .total_bytes()
            .unwrap(),
        (live() - before) as usize,
    );
    assert_eq!((live() - before) as usize, old_bytes * 2);
    drop((factor, replacement));
    assert_eq!(live(), before);
    for index in 0..allocations {
        FAIL.with(|fail| fail.set(Some(index)));
        let result = construct(bound);
        FAIL.with(|fail| fail.set(None));
        assert!(
            matches!(result, Err(CmgError::AllocationFailed { .. })),
            "allocation {index} did not propagate failure"
        );
        assert_eq!(live(), before, "allocation {index} leaked partial owners");
    }
    println!(
        "FACTOR_ALLOCATION_SUCCESS component={component} vertices={} edges={} bound={bound} peak={peak} fallible_failures={allocations}",
        graph.vertex_count(),
        graph.edge_count()
    );
}

#[test]
fn terminal_factors_admit_before_allocation_and_clean_up_every_failure() {
    let fixtures = [
        Laplacian::from_edges(0, []).unwrap(),
        Laplacian::from_edges(1, []).unwrap(),
        Laplacian::from_edges(16, []).unwrap(),
        Laplacian::from_edges(2, [(0, 1, 1.0)]).unwrap(),
        Laplacian::from_edges(37, (1..37).map(|v| (v - 1, v, 1.0))).unwrap(),
        Laplacian::from_edges(24, (0..24).map(|v| (v, (v + 1) % 24, 1.0))).unwrap(),
        Laplacian::from_edges(15, (0..15).flat_map(|v| (0..v).map(move |u| (u, v, 1.0)))).unwrap(),
        Laplacian::from_edges(
            93,
            (1..33)
                .map(|v| (v - 1, v, if v == 16 { 1.0 / 65536.0 } else { 1.0 }))
                .chain((33..73).step_by(2).map(|v| (v, v + 1, 2.0))),
        )
        .unwrap(),
    ];
    for graph in &fixtures {
        check(graph, false);
        #[cfg(feature = "experimental-components")]
        check(graph, true);
    }
    assert!(GroundedLdl::construction_allocation_bound(usize::MAX).is_err());
    #[cfg(feature = "experimental-components")]
    {
        assert!(GroundedLdl::component_construction_allocation_bound(usize::MAX, 0).is_err());
        assert!(GroundedLdl::component_construction_allocation_bound(1, usize::MAX).is_err());
    }
}
