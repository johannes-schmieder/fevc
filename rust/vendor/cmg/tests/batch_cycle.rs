//! Fixed-cycle multi-RHS contracts, independent numerical and allocation witnesses.
use cmg::{
    CmgBatchWorkspace, CmgError, CmgOptions, CmgPreconditioner, CmgStorageReport, Laplacian,
};
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

thread_local! {
    static CALLS: Cell<usize> = const { Cell::new(0) };
    static LIVE: Cell<isize> = const { Cell::new(0) };
    static FAIL: Cell<Option<usize>> = const { Cell::new(None) };
}
struct Observed;
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
            let _ = LIVE.try_with(|live| live.set(live.get() + layout.size() as isize));
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
            let _ =
                LIVE.try_with(|live| live.set(live.get() + size as isize - old.size() as isize));
        }
        ptr
    }
}
#[global_allocator]
static ALLOCATOR: Observed = Observed;

fn graph(vertices: usize, connected: bool) -> Laplacian {
    let mut edges = Vec::new();
    for vertex in 0..vertices.saturating_sub(1) {
        if connected || vertex % 83 != 0 {
            edges.push((vertex, vertex + 1, 0.5 + (vertex % 7) as f64 / 8.0));
            if vertex + 3 < vertices && (connected || vertex % 83 < 79) {
                edges.push((vertex, vertex + 3, 0.25));
            }
        }
    }
    Laplacian::from_edges(vertices, edges).unwrap()
}
fn inputs(graph: &Laplacian, columns: usize) -> Vec<f64> {
    let n = graph.vertex_count();
    let mut interleaved = vec![0.0; n * columns];
    for lane in 0..columns {
        let x: Vec<_> = (0..n)
            .map(|i| ((i * 17 + lane * 31) % 53) as f64 / 8.0)
            .collect();
        let rhs = graph.matvec(&x).unwrap();
        for row in 0..n {
            interleaved[row * columns + lane] = rhs[row];
        }
    }
    interleaved
}
fn check_scalar(cmg: &CmgPreconditioner, graph: &Laplacian, capacity: usize) {
    let bytes = CmgBatchWorkspace::allocation_bytes(cmg, capacity).unwrap();
    let mut workspace = CmgBatchWorkspace::try_new(cmg, capacity, bytes).unwrap();
    assert_eq!(workspace.capacity(), capacity);
    assert_eq!(
        CmgStorageReport::measure(&[&workspace])
            .unwrap()
            .total_bytes()
            .unwrap(),
        bytes
    );
    // Alternating sizes catches stale lanes and reinterpreted prefix strides.
    for columns in [capacity, 1, capacity.saturating_sub(1), 0, capacity] {
        let rhs = inputs(graph, columns);
        let mut output = vec![f64::NAN; rhs.len()];
        let calls = CALLS.with(Cell::get);
        cmg.apply_compatible_batch_into(
            &rhs,
            &mut output,
            columns,
            &mut workspace,
            cmg::ApplyContext::default(),
        )
        .unwrap();
        assert_eq!(calls, CALLS.with(Cell::get), "serial application allocated");
        let first = output.clone();
        cmg.apply_compatible_batch_into(
            &rhs,
            &mut output,
            columns,
            &mut workspace,
            cmg::ApplyContext::default(),
        )
        .unwrap();
        assert_eq!(first, output);
        for lane in 0..columns {
            let scalar_rhs: Vec<_> = (0..graph.vertex_count())
                .map(|row| rhs[row * columns + lane])
                .collect();
            let mut scalar = vec![0.0; graph.vertex_count()];
            cmg.apply_compatible_into(&scalar_rhs, &mut scalar, &mut cmg.workspace())
                .unwrap();
            for (row, &value) in scalar.iter().enumerate() {
                assert_eq!(
                    value.to_bits(),
                    output[row * columns + lane].to_bits(),
                    "row {row}, lane {lane}, columns {columns}"
                );
            }
        }
    }
}
#[test]
fn serial_matches_scalar_direct_multilevel_components_and_diagonal_terminals() {
    for (vertices, connected, threshold, max_levels) in [
        (0, true, 700, 128),
        (1, true, 700, 128),
        (30, true, 700, 128),
        (512, true, 8, 128),
        (511, false, 8, 128),
        (512, false, 8, 1),
    ] {
        let graph = graph(vertices, connected);
        let cmg = CmgPreconditioner::build(
            &graph,
            CmgOptions {
                direct_threshold: threshold,
                max_levels,
                ..CmgOptions::default()
            },
        )
        .unwrap();
        for capacity in [1, 2, 4, 7] {
            check_scalar(&cmg, &graph, capacity);
        }
    }
}
#[test]
fn checked_admission_failures_and_shape_rejection_leave_no_live_allocations() {
    let graph = graph(512, false);
    let cmg = CmgPreconditioner::build(
        &graph,
        CmgOptions {
            direct_threshold: 8,
            ..CmgOptions::default()
        },
    )
    .unwrap();
    assert!(CmgBatchWorkspace::allocation_bytes(&cmg, 0).is_err());
    assert!(CmgBatchWorkspace::allocation_bytes(&cmg, usize::MAX).is_err());
    for capacity in [1, 4] {
        let bytes = CmgBatchWorkspace::allocation_bytes(&cmg, capacity).unwrap();
        let before_calls = CALLS.with(Cell::get);
        assert!(matches!(
            CmgBatchWorkspace::try_new(&cmg, capacity, bytes - 1),
            Err(CmgError::MemoryBudgetExceeded { .. })
        ));
        assert_eq!(before_calls, CALLS.with(Cell::get));
        let baseline = LIVE.with(Cell::get);
        let before = CALLS.with(Cell::get);
        let mut workspace = CmgBatchWorkspace::try_new(&cmg, capacity, bytes).unwrap();
        let calls = CALLS.with(Cell::get) - before;
        assert_eq!(LIVE.with(Cell::get) - baseline, bytes as isize);
        assert!(
            cmg.apply_compatible_batch_into(
                &[],
                &mut [],
                capacity + 1,
                &mut workspace,
                cmg::ApplyContext::default()
            )
            .is_err()
        );
        assert!(
            cmg.apply_compatible_batch_into(
                &[0.0],
                &mut [],
                1,
                &mut workspace,
                cmg::ApplyContext::default()
            )
            .is_err()
        );
        let different = CmgPreconditioner::build(&graph, CmgOptions::default()).unwrap();
        let rhs = inputs(&graph, 1);
        let mut output = vec![0.0; rhs.len()];
        assert!(
            different
                .apply_compatible_batch_into(
                    &rhs,
                    &mut output,
                    1,
                    &mut workspace,
                    cmg::ApplyContext::default()
                )
                .is_err()
        );
        drop(workspace);
        for fail_at in 0..calls {
            let live = LIVE.with(Cell::get);
            FAIL.with(|fail| fail.set(Some(fail_at)));
            let result = CmgBatchWorkspace::try_new(&cmg, capacity, bytes);
            FAIL.with(|fail| fail.set(None));
            assert!(
                matches!(result, Err(CmgError::AllocationFailed { .. })),
                "failed allocation {fail_at} returned {result:?}"
            );
            assert_eq!(live, LIVE.with(Cell::get));
        }
    }
}
#[test]
fn multilevel_cycle_is_linear_symmetric_positive_and_lanes_are_independent() {
    let graph = graph(257, false);
    let cmg = CmgPreconditioner::build(
        &graph,
        CmgOptions {
            direct_threshold: 4,
            ..CmgOptions::default()
        },
    )
    .unwrap();
    assert!(cmg.hierarchy().levels().len() > 1);
    let mut rhs = inputs(&graph, 3);
    for row in rhs.chunks_exact_mut(3) {
        row[2] = 1.25 * row[0] - 0.75 * row[1];
    }
    let bytes = CmgBatchWorkspace::allocation_bytes(&cmg, 4).unwrap();
    let mut workspace = CmgBatchWorkspace::try_new(&cmg, 4, bytes).unwrap();
    let mut result = vec![0.0; rhs.len()];
    cmg.apply_compatible_batch_into(
        &rhs,
        &mut result,
        3,
        &mut workspace,
        cmg::ApplyContext::default(),
    )
    .unwrap();
    let dot = |a: usize, b: usize| {
        rhs.chunks_exact(3)
            .zip(result.chunks_exact(3))
            .map(|(x, y)| x[a] * y[b])
            .sum::<f64>()
    };
    assert!(dot(0, 0) > 0.0 && dot(1, 1) > 0.0);
    assert!((dot(0, 1) - dot(1, 0)).abs() < 1e-11 * dot(0, 0).abs().max(1.0));
    for row in result.chunks_exact(3) {
        assert!((row[2] - (1.25 * row[0] - 0.75 * row[1])).abs() < 1e-11);
    }
}
#[cfg(feature = "experimental-components")]
#[test]
fn pruned_and_component_terminal_options_match_scalar() {
    use cmg::ComponentBuildOptions;
    let graph = graph(513, false);
    for preserve in [false, true] {
        let cmg = CmgPreconditioner::build_with(
            &graph,
            cmg::BuildOptions {
                hierarchy: CmgOptions {
                    direct_threshold: 16,
                    ..CmgOptions::default()
                },
                components: ComponentBuildOptions {
                    prune_coarse_isolates: true,
                    preserve_unpruned_stopping: preserve,
                    factor_terminal_components: true,
                },
            },
            cmg::BuildContext {
                budget_bytes: Some(usize::MAX),
                ..cmg::BuildContext::default()
            },
        )
        .unwrap();
        check_scalar(&cmg, &graph, 4);
    }
}
#[cfg(feature = "parallel")]
#[test]
fn planned_batch_preserves_scalar_single_rhs_and_is_repeatable_across_workers() {
    use cmg::{ParallelCmgPlan, ParallelExecutor, ParallelOptions};
    let graph = graph(2048, true);
    let cmg = CmgPreconditioner::build(
        &graph,
        CmgOptions {
            direct_threshold: 16,
            ..CmgOptions::default()
        },
    )
    .unwrap();
    let mut prior = None;
    for threads in [2, 4] {
        let executor = ParallelExecutor::new(ParallelOptions {
            threads,
            min_parallel_len: 1,
            reduction_chunk_size: 32,
            ..ParallelOptions::default()
        })
        .unwrap();
        let plan = ParallelCmgPlan::build(&cmg, executor.team()).unwrap();
        assert!(plan.operator_count() > 0);
        for capacity in [1, 4] {
            let bytes = CmgBatchWorkspace::allocation_bytes(&cmg, capacity).unwrap();
            let mut workspace = CmgBatchWorkspace::try_new(&cmg, capacity, bytes).unwrap();
            for columns in [capacity, 1, capacity] {
                let rhs = inputs(&graph, columns);
                let mut output = vec![0.0; rhs.len()];
                cmg.apply_compatible_batch_into(
                    &rhs,
                    &mut output,
                    columns,
                    &mut workspace,
                    cmg::ApplyContext {
                        plan: Some(&plan),
                        team: Some(executor.team()),
                        cancel: None,
                    },
                )
                .unwrap();
                let first = output.clone();
                cmg.apply_compatible_batch_into(
                    &rhs,
                    &mut output,
                    columns,
                    &mut workspace,
                    cmg::ApplyContext {
                        plan: Some(&plan),
                        team: Some(executor.team()),
                        cancel: None,
                    },
                )
                .unwrap();
                assert_eq!(first, output);
                for lane in 0..columns {
                    let scalar_rhs: Vec<_> = (0..graph.vertex_count())
                        .map(|row| rhs[row * columns + lane])
                        .collect();
                    let mut scalar = vec![0.0; graph.vertex_count()];
                    cmg.apply_compatible_into_with(
                        &scalar_rhs,
                        &mut scalar,
                        &mut cmg.workspace(),
                        cmg::ApplyContext {
                            plan: Some(&plan),
                            team: Some(executor.team()),
                            cancel: None,
                        },
                    )
                    .unwrap();
                    for (row, &value) in scalar.iter().enumerate() {
                        if capacity == 1 {
                            assert_eq!(value.to_bits(), output[row].to_bits());
                        } else {
                            assert!(
                                (value - output[row * columns + lane]).abs()
                                    < 1e-11 * value.abs().max(1.0)
                            );
                        }
                    }
                }
                if capacity == 4 && columns == 4 {
                    if let Some(prior) = &prior {
                        assert_eq!(prior, &output);
                    }
                    prior = Some(output);
                }
            }
        }
    }
}
