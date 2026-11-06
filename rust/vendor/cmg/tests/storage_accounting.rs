//! Independent requested-allocation checks. One serial test owns the tracer;
//! no production allocator cap is installed and failures target fallible paths.
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
        unsafe {
            System.dealloc(ptr, layout);
        }
    }
    unsafe fn realloc(&self, ptr: *mut u8, old: Layout, size: usize) -> *mut u8 {
        if fail() {
            return std::ptr::null_mut();
        }
        let ptr = unsafe { System.realloc(ptr, old, size) };
        if !ptr.is_null() {
            // A realloc may temporarily retain both buffers; record that upper
            // observation before removing the old requested layout.
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
    let bytes = CmgStorageReport::measure(roots)
        .unwrap()
        .total_bytes()
        .unwrap();
    assert_eq!(CALLS.with(Cell::get), calls, "report must not allocate");
    bytes
}

#[test]
#[cfg(feature = "parallel")]
fn estimated_runtime_budget_is_explicit_checked_and_not_a_strict_bound() {
    let mib = 1024 * 1024;
    let allowance = ParallelRuntimeAllowance::default();
    let options = ParallelOptions {
        threads: 16,
        ..Default::default()
    };
    let estimate = allowance.estimate(options).unwrap();
    assert_eq!(estimate.threads(), 16);
    assert_eq!(estimate.runtime_heap_allowance_bytes(), 64 * mib);
    assert_eq!(estimate.worker_stack_allowance_bytes(), 128 * mib);
    assert_eq!(
        estimate.total_bytes(),
        192 * mib + estimate.worker_handles_bytes()
    );
    let calls = CALLS.with(Cell::get);
    assert!(matches!(
        ParallelExecutor::new_with_estimated_memory_budget(
            options,
            allowance,
            estimate.total_bytes() - 1,
        ),
        Err(CmgError::MemoryBudgetExceeded { .. })
    ));
    assert_eq!(
        CALLS.with(Cell::get),
        calls,
        "short runtime estimate spawned/allocated"
    );
    for threads in [1, 4, 2, 1] {
        let options = ParallelOptions { threads, ..options };
        let expected = allowance.estimate(options).unwrap();
        let (executor, charged) = ParallelExecutor::new_with_estimated_memory_budget(
            options,
            allowance,
            expected.total_bytes(),
        )
        .unwrap();
        assert_eq!(charged, expected);
        assert_eq!(executor.thread_count(), threads);
        let measured = CmgStorageReport::measure(&[&executor]).unwrap();
        assert!(measured.runtime_heap_unbounded());
        assert_eq!(
            measured.total_bytes().unwrap(),
            charged.worker_handles_bytes()
        );
        drop(executor);
    }
    FAIL.with(|fail| fail.set(Some(0)));
    let result = ParallelExecutor::new_with_estimated_memory_budget(
        options,
        allowance,
        estimate.total_bytes(),
    );
    FAIL.with(|fail| fail.set(None));
    assert!(matches!(result, Err(CmgError::AllocationFailed { .. })));
    for allowance in [
        ParallelRuntimeAllowance {
            pool_heap_bytes: usize::MAX,
            ..allowance
        },
        ParallelRuntimeAllowance {
            worker_heap_bytes: usize::MAX,
            ..allowance
        },
        ParallelRuntimeAllowance {
            worker_stack_bytes: usize::MAX,
            ..allowance
        },
        ParallelRuntimeAllowance {
            worker_stack_bytes: 0,
            ..allowance
        },
    ] {
        assert!(allowance.estimate(options).is_err());
    }
    assert_eq!(ParallelExecutor::runtime_allocation_bound(), None);
    assert!(ParallelExecutor::new_with_memory_budget(options, usize::MAX).is_err());
    println!(
        "ESTIMATED_RUNTIME_POLICY_SUCCESS exact_short_overflow_retirement=true strict_coverage=false"
    );
}

#[test]
fn owned_solution_banks_include_outer_spare_capacity() {
    let graph = Laplacian::from_edges(17, (1..17).map(|v| (v - 1, v, 1.0))).unwrap();
    let pre = CmgPreconditioner::build(&graph, CmgOptions::default()).unwrap();
    let mut rhs = [0.0; 17];
    rhs[0] = 1.0;
    rhs[16] = -1.0;
    let mut bank = Vec::with_capacity(7);
    for _ in 0..3 {
        bank.push(solve_pcg(&graph, &pre, &rhs, PcgOptions::default()).unwrap());
    }
    assert_eq!(
        report(&[&bank]),
        bank.capacity() * std::mem::size_of::<PcgResult>() + 3 * 17 * 8
    );
    assert_eq!(report(&[&bank, &bank[0]]), report(&[&bank]));
    #[cfg(feature = "parallel")]
    {
        let executor = ParallelExecutor::new(ParallelOptions {
            threads: 2,
            ..Default::default()
        })
        .unwrap();
        let solver = ParallelPcgSolver::from_preconditioner(
            pre,
            executor.team(),
            ParallelPcgPolicy::default(),
        )
        .unwrap();
        let batch = solver
            .solve_batch(
                &[rhs.to_vec(), rhs.to_vec()],
                PcgOptions::default(),
                executor.team(),
            )
            .unwrap();
        let observed = report(&[&batch]);
        let results = batch.into_results();
        assert_eq!(observed, report(&[&results]));
        assert!(observed >= 2 * (std::mem::size_of::<PcgResult>() + 17 * 8));
    }
}

#[test]
fn csr_and_plan_admit_before_allocating_and_clean_up_partial_owners() {
    let graph = Laplacian::from_edges(64, (1..64).map(|i| (i - 1, i, 1.0))).unwrap();
    let bound = CsrLaplacian::construction_allocation_bound(64, 63).unwrap();
    let calls = CALLS.with(Cell::get);
    assert!(CsrLaplacian::from_laplacian_with_budget(&graph, bound - 1).is_err());
    assert_eq!(CALLS.with(Cell::get), calls);
    let before = live();
    PEAK.with(|p| p.set(before));
    let csr = CsrLaplacian::from_laplacian_with_budget(&graph, bound).unwrap();
    let observed_peak = (PEAK.with(Cell::get) - before) as usize;
    assert_eq!(observed_peak, bound);
    assert_eq!(
        report(&[&graph, &csr]) - report(&[&graph]),
        (live() - before) as usize
    );
    drop(csr);
    assert_eq!(live(), before);
    for index in 0..4 {
        FAIL.with(|f| f.set(Some(index)));
        let result = CsrLaplacian::from_laplacian_with_budget(&graph, bound);
        FAIL.with(|f| f.set(None));
        assert!(matches!(result, Err(CmgError::AllocationFailed { .. })));
        assert_eq!(live(), before);
    }
    assert!(CsrLaplacian::construction_allocation_bound(usize::MAX, 0).is_err());
    assert!(CsrLaplacian::construction_allocation_bound(1, usize::MAX).is_err());
    println!(
        "CSR_ALLOCATION_SUCCESS bound={bound} observed_peak={observed_peak} fallible_failures=4"
    );

    #[cfg(feature = "parallel")]
    {
        // Eligible planned operators, but CSR density below the parallel-fill
        // threshold. No runtime work is dispatched, so this independent caller
        // allocator observation exactly isolates the claimed CMG-owned scope.
        let graph = Laplacian::from_edges(
            400,
            (0..400).flat_map(|v| {
                [
                    (v % 20 != 19).then_some((v, v + 1, 1.0)),
                    (v < 380).then_some((v, v + 20, 1.0)),
                ]
                .into_iter()
                .flatten()
            }),
        )
        .unwrap();
        let pre = CmgPreconditioner::build(
            &graph,
            CmgOptions {
                max_levels: 2,
                direct_threshold: 8,
                ..Default::default()
            },
        )
        .unwrap();
        let executor = ParallelExecutor::new(ParallelOptions {
            threads: 2,
            min_parallel_len: 1,
            ..Default::default()
        })
        .unwrap();
        let bound = ParallelCmgPlan::construction_allocation_bound(&pre, executor.team()).unwrap();
        let before = live();
        let calls = CALLS.with(Cell::get);
        assert!(ParallelCmgPlan::build_with_budget(&pre, executor.team(), bound - 1).is_err());
        assert_eq!(CALLS.with(Cell::get), calls);
        PEAK.with(|p| p.set(before));
        let plan = ParallelCmgPlan::build_with_budget(&pre, executor.team(), bound).unwrap();
        assert_eq!(plan.operator_count(), 1);
        let observed_peak = (PEAK.with(Cell::get) - before) as usize;
        assert_eq!(observed_peak, bound);
        assert_eq!(
            report(&[&pre, &plan]) - report(&[&pre]),
            (live() - before) as usize
        );
        // Holding an old plan charges both descriptor/operator banks, but shared
        // lineage only once. No clone/replacement is hidden by admission.
        let old_and_parent = report(&[&pre, &plan]);
        let before_new = live();
        let new = ParallelCmgPlan::build_with_budget(&pre, executor.team(), bound).unwrap();
        assert_eq!(
            report(&[&pre, &plan, &new]) - old_and_parent,
            (live() - before_new) as usize
        );
        drop((plan, new));
        assert_eq!(live(), before);
        let mut failures = 0;
        for index in 0..20 {
            FAIL.with(|f| f.set(Some(index)));
            let result = ParallelCmgPlan::build_with_budget(&pre, executor.team(), bound);
            FAIL.with(|f| f.set(None));
            let success = result.is_ok();
            drop(result);
            assert_eq!(live(), before, "partial plan leaked at allocation {index}");
            if success {
                break;
            }
            failures += 1;
        }
        assert_eq!(failures, 6); // two descriptor banks and four serial CSR banks
        let solver = ParallelPcgSolver::from_preconditioner(
            pre,
            executor.team(),
            ParallelPcgPolicy::default(),
        )
        .unwrap();
        assert_eq!(solver.plan_construction_allocation_bound().unwrap(), bound);
        assert!(
            solver
                .try_plan_with_budget(executor.team(), bound - 1)
                .is_err()
        );
        assert_eq!(
            solver
                .try_plan_with_budget(executor.team(), bound)
                .unwrap()
                .operator_count(),
            1
        );
        assert_eq!(solver.plan_construction_allocation_bound().unwrap(), 0);
        let calls = CALLS.with(Cell::get);
        assert_eq!(
            solver
                .try_plan_with_budget(executor.team(), 0)
                .unwrap()
                .operator_count(),
            1
        );
        assert_eq!(CALLS.with(Cell::get), calls);
        println!(
            "PLAN_ALLOCATION_SUCCESS bound={bound} observed_peak={observed_peak} fallible_failures={failures}"
        );
    }
}

#[test]
fn independent_capacity_admission_overlap_and_failure_checks() {
    let endpoints: Vec<_> = (1..85)
        .filter(|i| i % 11 != 0)
        .map(|i| (i - 1, i))
        .chain([(0, 1), (0, 1)])
        .collect();
    let weights = vec![1.0; endpoints.len()];
    let base = live();
    let bound =
        PreparedLaplacianTopology::preparation_allocation_bound(91, endpoints.len()).unwrap();
    assert!(PreparedLaplacianTopology::prepare_with_budget(91, &endpoints, bound - 1).is_err());
    assert_eq!(live(), base);
    PEAK.with(|p| p.set(base));
    let topology = PreparedLaplacianTopology::prepare_with_budget(91, &endpoints, bound).unwrap();
    assert!(PEAK.with(Cell::get) - base <= bound as isize);
    assert_eq!(report(&[&topology]), (live() - base) as usize);
    let mut assembly = topology.workspace().unwrap();
    let before = live();
    let numeric = topology.numeric_allocation_bytes().unwrap();
    assert!(
        topology
            .assemble_with_workspace_budgeted(&weights, &mut assembly, numeric - 1)
            .is_err()
    );
    assert_eq!(live(), before);
    let graph = topology
        .assemble_with_workspace_budgeted(&weights, &mut assembly, numeric)
        .unwrap();
    assert_eq!((live() - before) as usize, numeric);
    let pre = CmgPreconditioner::build(
        &graph,
        CmgOptions {
            direct_threshold: 12,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(
        report(&[&topology, &assembly, &graph, &pre]),
        (live() - base) as usize
    );
    let repeated = report(&[&topology, &assembly, &graph, &pre, &graph, &pre]);
    assert_eq!(repeated, (live() - base) as usize);

    let bank_bytes = PcgBatchWorkspace::bank_allocation_bytes(&pre, 3).unwrap();
    let before_bank = live();
    assert!(PcgBatchWorkspace::new_bank_with_budget(&pre, 3, bank_bytes - 1).is_err());
    assert_eq!(live(), before_bank);
    let bank = PcgBatchWorkspace::new_bank_with_budget(&pre, 3, bank_bytes).unwrap();
    assert_eq!(report(&[&bank]), bank_bytes);
    assert_eq!((live() - before_bank) as usize, bank_bytes);
    let replacement = PcgBatchWorkspace::new_bank_with_budget(&pre, 1, bank_bytes).unwrap();
    let next_graph = topology.assemble(&weights).unwrap();
    assert_eq!(
        report(&[
            &topology,
            &assembly,
            &graph,
            &next_graph,
            &pre,
            &bank,
            &replacement
        ]),
        (live() - base) as usize
    );
    drop((replacement, next_graph, bank));
    assert_eq!(live(), before_bank);

    // Inject each fallible allocation failure in a two-workspace bank, including
    // failures after the first workspace is complete. Every partial owner drops.
    let mut failure_count = 0;
    for index in 0..300 {
        let before = live();
        FAIL.with(|fail| fail.set(Some(index)));
        let result = PcgBatchWorkspace::new_bank_with_budget(&pre, 2, usize::MAX);
        FAIL.with(|fail| fail.set(None));
        let success = result.is_ok();
        drop(result);
        assert_eq!(live(), before, "partial bank leaked at allocation {index}");
        if success {
            break;
        }
        failure_count += 1;
    }
    assert!(failure_count > 30);
    assert!(PcgBatchWorkspace::bank_allocation_bytes(&pre, usize::MAX).is_err());
    assert!(PreparedLaplacianTopology::preparation_allocation_bound(usize::MAX, 1).is_err());
    assert!(PreparedLaplacianTopology::preparation_allocation_bound(1, usize::MAX).is_err());

    // Spare capacities, including outer banks and compact label backing memory.
    let mut spare: Vec<Vec<f64>> = Vec::with_capacity(17);
    spare.push(Vec::with_capacity(100));
    assert_eq!(
        report(&[&spare]),
        17 * std::mem::size_of::<Vec<f64>>() + 800
    );
    let mut labels = Vec::with_capacity(96);
    labels.extend([0usize, 1, 1, 0]);
    let before = live();
    let labels_bytes = 96 * std::mem::size_of::<usize>();
    let aggregation = Aggregation::new(labels, 2).unwrap();
    assert_eq!(
        report(&[&aggregation]),
        (live() - before + labels_bytes as isize) as usize
    );
    let old = report(&[&aggregation]);
    let _ = aggregation.labels();
    let _ = aggregation.sizes();
    assert!(report(&[&aggregation]) >= old);
    drop((spare, aggregation));
    drop((pre, graph, assembly, topology));
    assert_eq!(live(), base);
}

#[test]
fn fill_crossing_and_isolates_are_not_a_nonzero_storage_cap() {
    for fill in [0.01, 1.0, 1.25, 5.0] {
        for max_levels in [1, 2, 16] {
            for isolates in [0, 1000] {
                let graph =
                    Laplacian::from_edges(128 + isolates, (1..128).map(|v| (v - 1, v, 1.0)))
                        .unwrap();
                let options = CmgOptions {
                    direct_threshold: 2,
                    max_levels,
                    max_hierarchy_nnz_factor: fill,
                    ..Default::default()
                };
                let pre = CmgPreconditioner::build(&graph, options).unwrap();
                let sum: usize = pre.hierarchy().report().matrix_nonzeros().iter().sum();
                if fill == 1.0 && max_levels == 16 {
                    assert_eq!(
                        pre.hierarchy().report().terminal_reason(),
                        TerminalReason::StagnatedFill
                    );
                    assert!(
                        sum > graph.matrix_nnz(),
                        "crossing level exceeds the old fill-only bound"
                    );
                }
                let estimate = CmgMemoryEstimate::conservative(
                    CmgProblemSize {
                        vertices: graph.vertex_count(),
                        input_edges: graph.edge_count(),
                        canonical_edges: graph.edge_count(),
                        right_hand_sides: 2,
                    },
                    options,
                    ParallelOptions {
                        threads: 2,
                        ..Default::default()
                    },
                )
                .unwrap();
                assert!(estimate.retained_solver_bytes() >= pre.retained_bytes());
                assert!(estimate.workspace_bytes_each() >= PcgWorkspace::new(&pre).byte_len());
                assert_eq!(estimate.complete_build_peak_bytes(), None);
                assert!(estimate.admit_complete_build(usize::MAX).is_err());
            }
        }
    }
}

#[test]
fn duplicate_maps_keep_input_capacity_in_the_repeated_estimate() {
    let topology = PreparedLaplacianTopology::prepare(2, std::iter::repeat_n((0, 1), 512)).unwrap();
    let estimate = RepeatedPcgMemoryEstimate::conservative(
        CmgProblemSize {
            vertices: 2,
            input_edges: 512,
            canonical_edges: 1,
            right_hand_sides: 1,
        },
        CmgOptions::default(),
        ParallelOptions {
            threads: 1,
            ..Default::default()
        },
        false,
        false,
    )
    .unwrap();
    assert!(
        estimate.prepared_topology_bytes() + estimate.shared_component_metadata_bytes()
            >= topology.retained_bytes()
    );
}
