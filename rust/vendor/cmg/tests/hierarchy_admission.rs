//! Serial construction admission: independent requested-layout observation.
// Context literals keep `..Default::default()` for their feature-gated fields.
#![allow(clippy::needless_update)]
use cmg::*;
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

thread_local! {
    static LIVE: Cell<isize> = const { Cell::new(0) };
    static PEAK: Cell<isize> = const { Cell::new(0) };
    static CALLS: Cell<usize> = const { Cell::new(0) };
    static FALLIBLE: Cell<usize> = const { Cell::new(0) };
    static FAIL: Cell<Option<usize>> = const { Cell::new(None) };
    static TRACE: Cell<Option<AllocationLedger>> = const { Cell::new(None) };
}
struct Observed;
fn allocated(bytes: usize) {
    let _ = LIVE.try_with(|live| {
        live.set(live.get() + bytes as isize);
        let _ = PEAK.try_with(|peak| peak.set(peak.get().max(live.get())));
    });
}
fn fail(size: usize) -> bool {
    let _ = CALLS.try_with(|calls| calls.set(calls.get() + 1));
    // Arc::new is infallible on stable Rust. Do not inject at Arc layout sizes
    // (including vector requests coincidentally equal to those sizes). This
    // intentionally does NOT claim all-allocation failure coverage. Terminal
    // factors, which own no Arcs, have exhaustive injection in a separate test.
    let counts = 2 * std::mem::size_of::<usize>();
    if [
        counts,
        counts + std::mem::size_of::<Vec<f64>>(),
        counts + std::mem::size_of::<Components>(),
    ]
    .contains(&size)
    {
        return false;
    }
    let _ = FALLIBLE.try_with(|calls| calls.set(calls.get() + 1));
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
        trace_allocation(layout.size());
        if fail(layout.size()) {
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
        trace_allocation(size);
        if fail(size) {
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
fn report(roots: &[&dyn CmgStorage]) -> usize {
    let calls = CALLS.with(Cell::get);
    let value = CmgStorageReport::measure(roots)
        .unwrap()
        .total_bytes()
        .unwrap();
    assert_eq!(CALLS.with(Cell::get), calls);
    value
}

fn check_hierarchy(graph: &Laplacian, options: CmgOptions) {
    let reference = CmgHierarchy::build(graph, options).unwrap();
    let bound = CmgHierarchy::construction_allocation_bound(
        graph.vertex_count(),
        graph.edge_count(),
        options,
        false,
    )
    .unwrap();
    let before = live();
    let calls = CALLS.with(Cell::get);
    assert!(
        CmgHierarchy::build_with(
            graph,
            cmg::BuildOptions::from(options),
            cmg::BuildContext {
                budget_bytes: Some(bound - 1),
                ..cmg::BuildContext::default()
            }
        )
        .is_err()
    );
    assert_eq!(CALLS.with(Cell::get), calls);
    PEAK.with(|peak| peak.set(before));
    let hierarchy = CmgHierarchy::build_with(
        graph,
        cmg::BuildOptions::from(options),
        cmg::BuildContext {
            budget_bytes: Some(bound),
            ..cmg::BuildContext::default()
        },
    )
    .unwrap();
    let peak = (PEAK.with(Cell::get) - before) as usize;
    assert!(peak <= bound, "standalone hierarchy {peak} > {bound}");
    assert_eq!(hierarchy, reference);
    assert_eq!(
        report(&[graph, &hierarchy]) - report(&[graph]),
        (live() - before) as usize
    );
    drop(hierarchy);
    assert_eq!(live(), before);
    println!(
        "SERIAL_HIERARCHY_ALLOCATION_SUCCESS vertices={} edges={} bound={bound} peak={peak}",
        graph.vertex_count(),
        graph.edge_count()
    );
}

fn check(
    graph: &Laplacian,
    options: CmgOptions,
    component: Option<(bool, bool, bool)>,
    inject: bool,
) -> TerminalReason {
    #[cfg(feature = "experimental-components")]
    let experiment = component.map(|(prune, preserve, factor)| ComponentBuildOptions {
        prune_coarse_isolates: prune,
        preserve_unpruned_stopping: preserve,
        factor_terminal_components: factor,
    });
    let bound = if component.is_some() {
        #[cfg(feature = "experimental-components")]
        {
            CmgPreconditioner::construction_allocation_bound(
                graph.vertex_count(),
                graph.edge_count(),
                cmg::BuildOptions {
                    hierarchy: options,
                    components: experiment.unwrap(),
                },
                false,
            )
            .unwrap()
        }
        #[cfg(not(feature = "experimental-components"))]
        unreachable!()
    } else {
        CmgPreconditioner::construction_allocation_bound(
            graph.vertex_count(),
            graph.edge_count(),
            cmg::BuildOptions::from(options),
            false,
        )
        .unwrap()
    };
    let reference = if component.is_some() {
        #[cfg(feature = "experimental-components")]
        {
            CmgPreconditioner::build_with(
                graph,
                cmg::BuildOptions {
                    hierarchy: options,
                    components: experiment.unwrap(),
                },
                cmg::BuildContext::default(),
            )
            .unwrap()
        }
        #[cfg(not(feature = "experimental-components"))]
        unreachable!()
    } else {
        CmgPreconditioner::build(graph, options).unwrap()
    };
    let construct = |budget| {
        if component.is_some() {
            #[cfg(feature = "experimental-components")]
            {
                CmgPreconditioner::build_with(
                    graph,
                    cmg::BuildOptions {
                        hierarchy: options,
                        components: experiment.unwrap(),
                    },
                    cmg::BuildContext {
                        budget_bytes: Some(budget),
                        ..cmg::BuildContext::default()
                    },
                )
            }
            #[cfg(not(feature = "experimental-components"))]
            unreachable!()
        } else {
            CmgPreconditioner::build_with(
                graph,
                cmg::BuildOptions::from(options),
                cmg::BuildContext {
                    budget_bytes: Some(budget),
                    ..cmg::BuildContext::default()
                },
            )
        }
    };
    let before = live();
    let calls = CALLS.with(Cell::get);
    let fallible = FALLIBLE.with(Cell::get);
    assert!(construct(bound - 1).is_err());
    assert_eq!(CALLS.with(Cell::get), calls, "short budget allocated");
    PEAK.with(|peak| peak.set(before));
    let pre = construct(bound).unwrap();
    let allocations = FALLIBLE.with(Cell::get) - fallible;
    let peak = (PEAK.with(Cell::get) - before) as usize;
    assert!(peak <= bound, "{peak} > {bound}");
    assert_eq!(
        pre, reference,
        "admission changed numerical state or stopping"
    );
    assert_eq!(
        report(&[graph, &pre]) - report(&[graph]),
        (live() - before) as usize
    );
    let reason = pre.hierarchy().report().terminal_reason();
    let old_and_parent = report(&[graph, &pre]);
    let before_second = live();
    let second = construct(bound).unwrap();
    assert_eq!(
        report(&[graph, &pre, &second]) - old_and_parent,
        (live() - before_second) as usize
    );
    drop((pre, second));
    assert_eq!(live(), before);
    if inject {
        for index in 0..allocations {
            FAIL.with(|fail| fail.set(Some(index)));
            let result = construct(bound);
            FAIL.with(|fail| fail.set(None));
            assert!(
                matches!(result, Err(CmgError::AllocationFailed { .. })),
                "failure index {index}"
            );
            assert_eq!(live(), before, "partial hierarchy leaked at {index}");
        }
    }
    println!(
        "SERIAL_BUILD_ALLOCATION_SUCCESS vertices={} edges={} component={component:?} reason={reason:?} bound={bound} peak={peak} injected={} arc_size_injection_excluded=true",
        graph.vertex_count(),
        graph.edge_count(),
        if inject { allocations } else { 0 }
    );
    reason
}

#[test]
fn serial_hierarchy_bounds_preserve_all_stopping_and_component_policies() {
    let fixtures = [
        Laplacian::from_edges(0, []).unwrap(),
        Laplacian::from_edges(17, []).unwrap(),
        Laplacian::from_edges(2, [(0, 1, 1.0)]).unwrap(),
        Laplacian::from_edges(73, (1..73).map(|v| (v - 1, v, 1.0))).unwrap(),
        Laplacian::from_edges(28, (0..28).map(|v| (v, (v + 1) % 28, 1.0))).unwrap(),
        Laplacian::from_edges(20, (0..20).flat_map(|v| (0..v).map(move |u| (u, v, 1.0)))).unwrap(),
        Laplacian::from_edges(
            133,
            (1..73)
                .map(|v| (v - 1, v, if v == 36 { 1.0 / 65536.0 } else { 1.0 }))
                .chain((73..113).step_by(2).map(|v| (v, v + 1, 2.0))),
        )
        .unwrap(),
    ];
    let mut seen = [false; 5];
    for (index, graph) in fixtures.iter().enumerate() {
        for (threshold, levels, fill) in [
            (1, 8, 5.0),
            (16, 8, 5.0),
            (74, 8, 5.0),
            (1, 1, 5.0),
            (1, 8, 0.01),
            (1, 8, 1.0),
        ] {
            let options = CmgOptions {
                direct_threshold: threshold,
                max_levels: levels,
                max_hierarchy_nnz_factor: fill,
                ..Default::default()
            };
            check_hierarchy(graph, options);
            let reason = check(
                graph,
                options,
                None,
                (index == 3 || index == 5) && threshold == 16,
            );
            seen[match reason {
                TerminalReason::Direct => 0,
                TerminalReason::FullContraction => 1,
                TerminalReason::StagnatedVertexReduction => 2,
                TerminalReason::StagnatedFill => 3,
                TerminalReason::MaximumLevels => 4,
            }] = true;
            #[cfg(feature = "experimental-components")]
            for prune in [false, true] {
                for preserve in [false, true] {
                    for factor in [false, true] {
                        check(
                            graph,
                            options,
                            Some((prune, preserve, factor)),
                            index == 6 && threshold == 16 && preserve && prune,
                        );
                    }
                }
            }
        }
    }
    assert!(seen.into_iter().all(|present| present));
    let options = CmgOptions::default();
    assert!(
        CmgPreconditioner::construction_allocation_bound(
            usize::MAX,
            0,
            cmg::BuildOptions::from(options),
            false
        )
        .is_err()
    );
    assert!(
        CmgPreconditioner::construction_allocation_bound(
            1,
            usize::MAX,
            cmg::BuildOptions::from(options),
            false
        )
        .is_err()
    );
    assert!(
        CmgPreconditioner::construction_allocation_bound(
            1,
            0,
            cmg::BuildOptions::from(CmgOptions {
                max_levels: usize::MAX,
                ..options
            }),
            false
        )
        .is_err()
    );
}

#[test]
fn prepared_numeric_generations_share_parents_but_admit_new_builds() {
    let endpoints: Vec<_> = (1..41).map(|v| (v - 1, v)).collect();
    let topology = PreparedLaplacianTopology::prepare(49, endpoints).unwrap();
    let mut workspace = topology.workspace().unwrap();
    for weight in [1.0, 0.125, 2.0] {
        let graph = topology
            .assemble_with_workspace(&[weight; 40], &mut workspace)
            .unwrap();
        let options = CmgOptions {
            direct_threshold: 12,
            max_levels: 6,
            ..Default::default()
        };
        check(&graph, options, None, false);
        #[cfg(feature = "experimental-components")]
        check(&graph, options, Some((true, true, true)), false);
    }
}

// This fixed-size ledger is entirely TLS/stack storage. Tracing is opt-in, so
// legacy admission/failure injection above retains its existing behavior.
#[derive(Clone, Copy, Default, Debug)]
struct AllocationLedger {
    polled: bool,
    polls: usize,
    allocations: usize,
    ungated: Option<usize>,
}

fn trace_allocation(bytes: usize) {
    let _ = TRACE.try_with(|trace| {
        if let Some(mut ledger) = trace.get() {
            // Controlled construction polls cancellation before every
            // allocation, so each allocation follows a fresh poll.
            if !ledger.polled && ledger.ungated.is_none() {
                ledger.ungated = Some(bytes);
            }
            ledger.polled = false;
            ledger.allocations += 1;
            trace.set(Some(ledger));
        }
    });
}

fn allocation_poll(stop_at: Option<usize>) -> bool {
    TRACE.with(|trace| {
        let mut ledger = trace.get().unwrap();
        let index = ledger.polls;
        ledger.polls += 1;
        ledger.polled = true;
        trace.set(Some(ledger));
        stop_at.is_some_and(|stop| index >= stop)
    })
}

struct AllocationTrace;
impl AllocationTrace {
    fn start() -> Self {
        TRACE.with(|trace| {
            assert!(trace.get().is_none());
            trace.set(Some(AllocationLedger::default()));
        });
        Self
    }
    fn finish(self) -> AllocationLedger {
        TRACE.with(|trace| trace.take().unwrap())
    }
}
impl Drop for AllocationTrace {
    fn drop(&mut self) {
        TRACE.with(|trace| trace.set(None));
    }
}

fn check_controlled_allocations<T>(
    bound: usize,
    mut construct: impl FnMut(Cancellation<'_>) -> Result<T, CmgError>,
) {
    let before = live();
    PEAK.with(|peak| peak.set(before));
    let trace = AllocationTrace::start();
    let result = construct(&|| allocation_poll(None));
    let ledger = trace.finish();
    let peak = (PEAK.with(Cell::get) - before) as usize;
    let result = result.unwrap();
    assert_eq!(ledger.ungated, None, "allocation without a preceding poll");
    assert!(ledger.allocations > 0);
    assert!(
        peak <= bound,
        "controlled peak {peak} exceeds bound {bound}"
    );
    drop(result);
    assert_eq!(live(), before, "successful candidate drop leaked storage");

    // Cancel at each poll, including those before zero-length requests and
    // Arc owners. No allocator failure injection or query allocation is needed.
    for index in 0..ledger.polls {
        PEAK.with(|peak| peak.set(before));
        let trace = AllocationTrace::start();
        let result = construct(&|| allocation_poll(Some(index)));
        let cancelled = trace.finish();
        assert!(
            matches!(result, Err(CmgError::Cancelled { .. })),
            "poll {index} did not cancel construction"
        );
        assert_eq!(cancelled.polls, index + 1);
        assert_eq!(cancelled.ungated, None, "poll {index}: ungated allocation");
        drop(result);
        assert_eq!(
            live(),
            before,
            "poll {index}: cancelled candidate leaked storage"
        );
        assert!((PEAK.with(Cell::get) - before) as usize <= bound);
    }
}

#[test]
fn controlled_graph_allocations_are_gated_bounded_and_released() {
    for (vertices, edges) in [
        (0, vec![]),
        (9, vec![]),
        (
            9,
            vec![(0, 1, 1.0), (1, 0, 2.0), (1, 2, 0.125), (4, 7, 3.0)],
        ),
        (
            12,
            (0..12)
                .flat_map(|v| (0..v).map(move |u| (u, v, 1.0 + (u + v) as f64)))
                .collect(),
        ),
    ] {
        // Independent requested-layout model: two compact edge banks, one
        // diagonal, and Arc<Vec<Edge>>, Arc<Vec<f64>>, Arc<()> owners.
        let arc_counts = Layout::new::<[usize; 2]>();
        let arc_edges = arc_counts
            .extend(Layout::new::<Vec<Edge>>())
            .unwrap()
            .0
            .pad_to_align()
            .size();
        let arc_diagonal = arc_counts
            .extend(Layout::new::<Vec<f64>>())
            .unwrap()
            .0
            .pad_to_align()
            .size();
        let arc_identity = arc_counts
            .extend(Layout::new::<()>())
            .unwrap()
            .0
            .pad_to_align()
            .size();
        let bound = 2 * edges.len() * std::mem::size_of::<Edge>()
            + vertices * std::mem::size_of::<f64>()
            + arc_edges
            + arc_diagonal
            + arc_identity;
        check_controlled_allocations(bound, |cancel| {
            Laplacian::build(
                vertices,
                &edges,
                cmg::BuildContext {
                    cancel: Some(cancel),
                    budget_bytes: Some(bound),
                    ..cmg::BuildContext::default()
                },
            )
        });
    }
}

#[cfg(feature = "experimental-components")]
#[test]
fn controlled_preconditioner_allocations_are_gated_bounded_and_released() {
    let fixtures = [
        Laplacian::from_edges(0, []).unwrap(),
        Laplacian::from_edges(11, []).unwrap(),
        Laplacian::from_edges(23, (1..19).map(|v| (v - 1, v, 1.0))).unwrap(),
        Laplacian::from_edges(16, (0..16).flat_map(|v| (0..v).map(move |u| (u, v, 1.0)))).unwrap(),
        Laplacian::from_edges(
            29,
            (1..17)
                .map(|v| (v - 1, v, 1.0))
                .chain([(20, 22, 2.0), (22, 24, 3.0)]),
        )
        .unwrap(),
    ];
    for graph in &fixtures {
        for direct_threshold in [4, 32] {
            let options = CmgOptions {
                direct_threshold,
                max_levels: 5,
                ..CmgOptions::default()
            };
            for factor_terminal_components in [false, true] {
                let experiment = ComponentBuildOptions {
                    prune_coarse_isolates: true,
                    preserve_unpruned_stopping: true,
                    factor_terminal_components,
                };
                let bound = CmgPreconditioner::construction_allocation_bound(
                    graph.vertex_count(),
                    graph.edge_count(),
                    cmg::BuildOptions {
                        hierarchy: options,
                        components: experiment,
                    },
                    false,
                )
                .unwrap();
                let construct = |cancel: Cancellation<'_>| {
                    CmgPreconditioner::build_with(
                        graph,
                        cmg::BuildOptions {
                            hierarchy: options,
                            components: experiment,
                        },
                        cmg::BuildContext {
                            cancel: Some(cancel),
                            budget_bytes: Some(bound),
                            ..cmg::BuildContext::default()
                        },
                    )
                };
                check_controlled_allocations(bound, construct);
            }
        }
    }
}

#[cfg(feature = "experimental-components")]
#[test]
fn controlled_multilevel_build_reports_a_late_allocation_failure_without_leaking() {
    let graph = Laplacian::from_edges(24, (1..24).map(|v| (v - 1, v, 1.0))).unwrap();
    let options = CmgOptions {
        max_levels: 8,
        direct_threshold: 4,
        ..Default::default()
    };
    let experiment = ComponentBuildOptions {
        prune_coarse_isolates: true,
        preserve_unpruned_stopping: true,
        factor_terminal_components: true,
    };
    let bound = CmgPreconditioner::construction_allocation_bound(
        graph.vertex_count(),
        graph.edge_count(),
        cmg::BuildOptions {
            hierarchy: options,
            components: experiment,
        },
        false,
    )
    .unwrap();
    let never = || false;
    let construct = |budget_bytes| {
        CmgPreconditioner::build_with(
            &graph,
            cmg::BuildOptions {
                hierarchy: options,
                components: experiment,
            },
            cmg::BuildContext {
                cancel: Some(&never),
                budget_bytes: Some(budget_bytes),
                ..cmg::BuildContext::default()
            },
        )
    };
    let candidate = construct(bound).unwrap();
    assert!(candidate.hierarchy().levels().len() > 1);
    drop(candidate);
    let calls = CALLS.with(Cell::get);
    assert!(matches!(
        construct(bound - 1),
        Err(CmgError::MemoryBudgetExceeded { .. })
    ));
    assert_eq!(CALLS.with(Cell::get), calls, "short budget allocated");
    // One late exogenous fallible allocation failure; Arc-size exclusions
    // remain those documented by this allocator above.
    let before = live();
    let start = FALLIBLE.with(Cell::get);
    let result = construct(bound).unwrap();
    let fallible = FALLIBLE.with(Cell::get) - start;
    drop(result);
    assert_eq!(live(), before);
    assert!(fallible > 0);
    FAIL.with(|fail| fail.set(Some(fallible - 1)));
    let result = construct(bound);
    FAIL.with(|fail| fail.set(None));
    assert!(matches!(result, Err(CmgError::AllocationFailed { .. })));
    assert_eq!(live(), before);
}

#[test]
fn controlled_scalar_workspace_allocations_are_gated_bounded_and_released() {
    let fixtures = [
        Laplacian::from_edges(11, []).unwrap(),
        Laplacian::from_edges(23, (1..19).map(|v| (v - 1, v, 1.0))).unwrap(),
        Laplacian::from_edges(
            29,
            (1..17)
                .map(|v| (v - 1, v, 1.0))
                .chain([(20, 22, 2.0), (22, 24, 3.0)]),
        )
        .unwrap(),
    ];
    for graph in &fixtures {
        for direct_threshold in [2, 32] {
            let preconditioner = CmgPreconditioner::build(
                graph,
                CmgOptions {
                    direct_threshold,
                    ..CmgOptions::default()
                },
            )
            .unwrap();
            let bound = preconditioner.workspace_allocation_bytes().unwrap();
            let never = || false;
            assert_eq!(
                preconditioner
                    .workspace_allocation_bytes_controlled(&never)
                    .unwrap(),
                bound
            );
            check_controlled_allocations(bound, |cancel| {
                preconditioner.try_workspace_controlled(cancel)
            });
        }
    }
}

// Two equal-weight 264-cliques. The low-effective-degree correction keeps all
// but one pair of each clique as singletons, so each level removes only two
// vertices and an unrestricted hierarchy needs 34 levels. The direct
// threshold and fill guard are lowered/raised so that neither stops it first.
fn deep_two_clique_graph() -> Laplacian {
    const K: usize = 264;
    Laplacian::from_edges(
        2 * K,
        (0..2).flat_map(|c| {
            (0..K).flat_map(move |v| (0..v).map(move |u| (c * K + u, c * K + v, 1.0)))
        }),
    )
    .unwrap()
}

fn deep_options() -> CmgOptions {
    CmgOptions {
        direct_threshold: 1,
        max_hierarchy_nnz_factor: 64.0,
        ..Default::default()
    }
}

#[test]
fn default_level_cap_stops_a_deep_hierarchy_inside_its_admitted_bound() {
    let graph = deep_two_clique_graph();
    let options = deep_options();
    assert_eq!(options.max_levels, 32);
    let deep = CmgHierarchy::build(
        &graph,
        CmgOptions {
            max_levels: 64,
            ..options
        },
    )
    .unwrap();
    assert_eq!(deep.levels().len(), 34);
    assert_eq!(
        deep.report().terminal_reason(),
        TerminalReason::StagnatedVertexReduction
    );
    let capped = CmgHierarchy::build(&graph, options).unwrap();
    assert_eq!(capped.levels().len(), 32);
    assert_eq!(
        capped.report().terminal_reason(),
        TerminalReason::MaximumLevels
    );
    let terminal = &capped.levels()[31];
    assert_eq!(
        terminal.terminal_reason(),
        Some(TerminalReason::MaximumLevels)
    );
    assert!(terminal.terminal_reason().unwrap().is_iterative());
    // The cap only replaces the 32nd level's contraction by the terminal; every
    // earlier level, and the terminal graph and smoother, are the deep build's.
    assert_eq!(capped.levels()[..31], deep.levels()[..31]);
    assert_eq!(terminal.graph(), deep.levels()[31].graph());
    assert_eq!(
        terminal.inverse_diagonal(),
        deep.levels()[31].inverse_diagonal()
    );
    // Admission at the capped bound: one byte short refuses without allocating,
    // the bound admits a build within it, and the build matches the reference.
    check_hierarchy(&graph, options);
    assert_eq!(
        check(&graph, options, None, false),
        TerminalReason::MaximumLevels
    );
}

#[test]
fn level_cap_admits_a_budget_the_former_level_assumption_refused() {
    let graph = deep_two_clique_graph();
    let capped = deep_options();
    let former = CmgOptions {
        max_levels: 128,
        ..capped
    };
    let bound = |options| {
        CmgPreconditioner::construction_allocation_bound(
            graph.vertex_count(),
            graph.edge_count(),
            cmg::BuildOptions::from(options),
            false,
        )
        .unwrap()
    };
    let budget = bound(capped);
    assert!(budget < bound(former));
    let construct = |options| {
        CmgPreconditioner::build_with(
            &graph,
            cmg::BuildOptions::from(options),
            cmg::BuildContext {
                budget_bytes: Some(budget),
                ..cmg::BuildContext::default()
            },
        )
    };
    // An explicit 128-level request keeps its own assumption and is refused
    // before any allocation under the budget the default cap now admits.
    let calls = CALLS.with(Cell::get);
    assert!(matches!(
        construct(former),
        Err(CmgError::MemoryBudgetExceeded { required_bytes, budget_bytes })
            if required_bytes == bound(former) && budget_bytes == budget
    ));
    assert_eq!(CALLS.with(Cell::get), calls, "refused build allocated");
    let admitted = construct(capped).unwrap();
    assert_eq!(
        admitted.hierarchy().report().terminal_reason(),
        TerminalReason::MaximumLevels
    );
    // Below the cap both assumptions build the identical preconditioner.
    let shallow = Laplacian::from_edges(4096, (1..4096).map(|v| (v - 1, v, 1.0))).unwrap();
    let shallow_options = CmgOptions {
        direct_threshold: 4,
        ..Default::default()
    };
    let reference = CmgPreconditioner::build(&shallow, shallow_options).unwrap();
    assert!(reference.hierarchy().levels().len() < 32);
    assert_eq!(
        CmgPreconditioner::build(
            &shallow,
            CmgOptions {
                max_levels: 128,
                ..shallow_options
            }
        )
        .unwrap(),
        reference
    );
}
