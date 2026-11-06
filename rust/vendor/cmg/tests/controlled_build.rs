#![cfg(feature = "experimental-components")]
//! Cancellation-only controlled construction: exact bits, every poll, parking.
// Context literals keep `..Default::default()` for their feature-gated fields.
#![allow(clippy::needless_update)]

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use cmg::{
    ApplyContext, CmgBatchWorkspace, CmgError, CmgOptions, CmgPreconditioner,
    ComponentBuildOptions, Laplacian,
};

fn graph(n: usize) -> Laplacian {
    let mut edges = Vec::new();
    for i in 0..n.saturating_sub(1) {
        if i % 29 != 28 {
            edges.push((i, i + 1, (i % 7 + 1) as f64));
        }
        if i + 3 < n && i % 29 < 25 {
            edges.push((i, i + 3, 0.125));
        }
    }
    Laplacian::from_edges(n, edges).unwrap()
}

fn bits(v: &[f64]) -> Vec<u64> {
    v.iter().map(|x| x.to_bits()).collect()
}

/// Counts polls and requests cancellation from one zero-based poll onwards.
struct StopAt {
    polls: AtomicUsize,
    stop: usize,
}

impl StopAt {
    fn new(stop: usize) -> Self {
        Self {
            polls: AtomicUsize::new(0),
            stop,
        }
    }

    fn query(&self) -> bool {
        self.polls.fetch_add(1, Ordering::Relaxed) >= self.stop
    }

    fn polls(&self) -> usize {
        self.polls.load(Ordering::Relaxed)
    }
}

#[test]
fn uncancelled_construction_preserves_hierarchy_factor_and_cycle_bits() {
    for n in [0, 1, 2, 9, 65, 257] {
        let graph = graph(n);
        for prune in [false, true] {
            for preserve in [false, true] {
                for factor in [false, true] {
                    let experiment = ComponentBuildOptions {
                        prune_coarse_isolates: prune,
                        preserve_unpruned_stopping: preserve,
                        factor_terminal_components: factor,
                    };
                    let options = CmgOptions {
                        direct_threshold: 8,
                        ..CmgOptions::default()
                    };
                    let old = CmgPreconditioner::build_with(
                        &graph,
                        cmg::BuildOptions {
                            hierarchy: options,
                            components: experiment,
                        },
                        cmg::BuildContext {
                            budget_bytes: Some(usize::MAX),
                            ..cmg::BuildContext::default()
                        },
                    )
                    .unwrap();
                    let first = StopAt::new(usize::MAX);
                    let query = || first.query();
                    let new = CmgPreconditioner::build_with(
                        &graph,
                        cmg::BuildOptions {
                            hierarchy: options,
                            components: experiment,
                        },
                        cmg::BuildContext {
                            cancel: Some(&query),
                            budget_bytes: Some(usize::MAX),
                            ..cmg::BuildContext::default()
                        },
                    )
                    .unwrap();
                    assert_eq!(old.hierarchy(), new.hierarchy());
                    assert_eq!(old.terminal_factor(), new.terminal_factor());
                    assert_eq!(old.repeat_counts(), new.repeat_counts());
                    let probe: Vec<_> = (0..n)
                        .map(|i| ((i * 11 % 17) as f64 - 8.0) * 0.25)
                        .collect();
                    let rhs = graph.matvec(&probe).unwrap();
                    assert_eq!(
                        bits(&old.apply(&rhs).unwrap()),
                        bits(&new.apply(&rhs).unwrap())
                    );
                    // The poll schedule is a deterministic function of the input.
                    let second = StopAt::new(usize::MAX);
                    let query = || second.query();
                    CmgPreconditioner::build_with(
                        &graph,
                        cmg::BuildOptions {
                            hierarchy: options,
                            components: experiment,
                        },
                        cmg::BuildContext {
                            cancel: Some(&query),
                            budget_bytes: Some(usize::MAX),
                            ..cmg::BuildContext::default()
                        },
                    )
                    .unwrap();
                    assert!(first.polls() > 0);
                    assert_eq!(first.polls(), second.polls());
                }
            }
        }
    }
}

#[test]
fn terminal_factor_follows_the_direct_threshold_without_a_work_screen() {
    // Large direct terminals are factored whenever the hierarchy options say
    // so; no construction-work allowance can redirect them to coarsening.
    let graph = graph(600);
    let experiment = ComponentBuildOptions {
        prune_coarse_isolates: true,
        preserve_unpruned_stopping: true,
        factor_terminal_components: true,
    };
    let never = || false;
    let built = CmgPreconditioner::build_with(
        &graph,
        cmg::BuildOptions {
            hierarchy: CmgOptions::default(),
            components: experiment,
        },
        cmg::BuildContext {
            cancel: Some(&never),
            budget_bytes: Some(usize::MAX),
            ..cmg::BuildContext::default()
        },
    )
    .unwrap();
    assert_eq!(built.hierarchy().levels().len(), 1);
    assert!(built.terminal_factor().is_some());
}

#[test]
fn cancellation_at_every_poll_returns_cancelled_and_a_fresh_build_recovers() {
    let graph = graph(65);
    let options = CmgOptions {
        direct_threshold: 3,
        ..CmgOptions::default()
    };
    let experiment = ComponentBuildOptions {
        prune_coarse_isolates: true,
        preserve_unpruned_stopping: true,
        factor_terminal_components: true,
    };
    let build = |cancel: cmg::Cancellation<'_>| {
        CmgPreconditioner::build_with(
            &graph,
            cmg::BuildOptions {
                hierarchy: options,
                components: experiment,
            },
            cmg::BuildContext {
                cancel: Some(cancel),
                budget_bytes: Some(usize::MAX),
                ..cmg::BuildContext::default()
            },
        )
    };
    let count = StopAt::new(usize::MAX);
    let query = || count.query();
    let reference = build(&query).unwrap();
    for stop in 0..count.polls() {
        let cancel = StopAt::new(stop);
        let query = || cancel.query();
        assert!(matches!(build(&query), Err(CmgError::Cancelled { .. })));
        // Cancellation is observed at the first true poll, never later.
        assert_eq!(cancel.polls(), stop + 1);
    }
    let never = || false;
    assert_eq!(build(&never).unwrap(), reference);
}

#[test]
fn cancellation_query_can_park_then_continue_or_cancel_without_a_pool() {
    for cancel in [false, true] {
        let (ready_tx, ready_rx) = std::sync::mpsc::sync_channel(0);
        let (grant_tx, grant_rx) = std::sync::mpsc::sync_channel::<bool>(0);
        let worker = std::thread::spawn(move || {
            let graph = graph(5);
            let first = AtomicBool::new(true);
            let decided = AtomicBool::new(false);
            let ready_tx = std::sync::Mutex::new(ready_tx);
            let grant_rx = std::sync::Mutex::new(grant_rx);
            let query = || {
                if first.swap(false, Ordering::Relaxed) {
                    ready_tx.lock().unwrap().send(()).unwrap();
                    decided.store(grant_rx.lock().unwrap().recv().unwrap(), Ordering::Relaxed);
                }
                decided.load(Ordering::Relaxed)
            };
            CmgPreconditioner::build_with(
                &graph,
                cmg::BuildOptions {
                    hierarchy: CmgOptions::default(),
                    components: ComponentBuildOptions::default(),
                },
                cmg::BuildContext {
                    cancel: Some(&query),
                    budget_bytes: Some(usize::MAX),
                    ..cmg::BuildContext::default()
                },
            )
        });
        ready_rx.recv().unwrap();
        grant_tx.send(cancel).unwrap();
        let result = worker.join().unwrap();
        if cancel {
            assert!(matches!(result, Err(CmgError::Cancelled { .. })));
        } else {
            assert!(result.is_ok());
        }
    }
}

fn dot(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

fn close(a: &[f64], b: &[f64]) {
    let scale = a.iter().chain(b).fold(0.0_f64, |s, x| s.max(x.abs()));
    for (a, b) in a.iter().zip(b) {
        assert!(
            (a - b).abs() <= 1e-11 * scale,
            "{a} != {b} at scale {scale}"
        );
    }
}

#[test]
fn controlled_weighted_cycles_are_fixed_linear_symmetric_and_positive_on_the_range() {
    let experiment = ComponentBuildOptions {
        prune_coarse_isolates: true,
        preserve_unpruned_stopping: true,
        factor_terminal_components: true,
    };
    for disconnected in [false, true] {
        for scale in [1e-90, 1.0, 1e90] {
            let n = if disconnected { 50 } else { 24 };
            let edges = (0..if disconnected { 2 } else { 1 }).flat_map(|c| {
                (1..24).map(move |v| {
                    (
                        c * 24 + v - 1,
                        c * 24 + v,
                        scale
                            * if v == 12 {
                                1.0 / 65536.0
                            } else {
                                1.0 + (v % 5) as f64 / 4.0
                            },
                    )
                })
            });
            let graph = Laplacian::from_edges(n, edges).unwrap();
            let options = CmgOptions {
                max_levels: 8,
                direct_threshold: 4,
                ..Default::default()
            };
            let never = || false;
            let build = || {
                CmgPreconditioner::build_with(
                    &graph,
                    cmg::BuildOptions {
                        hierarchy: options,
                        components: experiment,
                    },
                    cmg::BuildContext {
                        cancel: Some(&never),
                        budget_bytes: Some(usize::MAX),
                        ..cmg::BuildContext::default()
                    },
                )
                .unwrap()
            };
            let pre = build();
            assert!(pre.hierarchy().levels().len() > 1);
            let retry = build();
            assert_eq!(pre, retry);
            // Exact within-component dipoles are independently in the original
            // Laplacian range. They avoid contaminating the linearity witness
            // with a second floating-point matvec's cancellation error.
            // Check both nearby dipoles and dipoles crossing the weak edge.
            for (u_end, v_end) in [(1, 3), (23, 21)] {
                let mut u = vec![0.0; n];
                let mut v = vec![0.0; n];
                for c in 0..if disconnected { 2 } else { 1 } {
                    u[c * 24] = scale;
                    u[c * 24 + u_end] = -scale;
                    v[c * 24 + 2] = scale;
                    v[c * 24 + v_end] = -scale;
                }
                let mu = pre.apply(&u).unwrap();
                let mv = pre.apply(&v).unwrap();
                assert_eq!(bits(&mu), bits(&retry.apply(&u).unwrap()));
                assert_eq!(bits(&mu), bits(&pre.apply(&u).unwrap()));
                let input: Vec<_> = u.iter().zip(&v).map(|(a, b)| 1.25 * a - 0.75 * b).collect();
                let expected: Vec<_> = mu
                    .iter()
                    .zip(&mv)
                    .map(|(a, b)| 1.25 * a - 0.75 * b)
                    .collect();
                close(&pre.apply(&input).unwrap(), &expected);
                let left = dot(&u, &mv);
                let right = dot(&mu, &v);
                assert!((left - right).abs() <= 1e-11 * left.abs().max(right.abs()));
                assert!(dot(&u, &mu) > 0.0 && dot(&v, &mv) > 0.0);
                assert!(mu.iter().chain(&mv).all(|x| x.is_finite()));
                if disconnected {
                    assert_eq!(&mu[48..], &[0.0, 0.0]);
                }
            }
        }
    }
}

#[test]
fn scalar_workspace_preparation_polls_inside_initialization_and_recovers() {
    // 200,000 isolated vertices give a single level whose residual, projected
    // RHS and seven component banks initialize 1.8 million values.
    let isolated = Laplacian::from_edges(200_000, []).unwrap();
    let preconditioner = CmgPreconditioner::build(&isolated, CmgOptions::default()).unwrap();
    let never = || false;
    assert_eq!(
        preconditioner
            .workspace_allocation_bytes_controlled(&never)
            .unwrap(),
        preconditioner.workspace_allocation_bytes().unwrap()
    );
    let cancelled = || true;
    assert!(matches!(
        preconditioner.workspace_allocation_bytes_controlled(&cancelled),
        Err(CmgError::Cancelled { .. })
    ));

    let count = StopAt::new(usize::MAX);
    let query = || count.query();
    let reference = preconditioner.try_workspace_controlled(&query).unwrap();
    // Eight units per initialized value at the fixed 65,536-unit cadence,
    // far more polls than the handful of allocations.
    assert!(count.polls() >= 9 * 200_000 * 8 / 65_536);
    for stop in [0, 1, count.polls() / 2, count.polls() - 1] {
        let cancel = StopAt::new(stop);
        let query = || cancel.query();
        assert!(matches!(
            preconditioner.try_workspace_controlled(&query),
            Err(CmgError::Cancelled { .. })
        ));
        assert_eq!(cancel.polls(), stop + 1);
    }

    let mut ordinary = preconditioner.try_workspace().unwrap();
    let mut controlled = preconditioner.try_workspace_controlled(&never).unwrap();
    assert_eq!(controlled.dimensions(), ordinary.dimensions());
    assert_eq!(controlled.byte_len(), ordinary.byte_len());
    assert_eq!(reference.byte_len(), ordinary.byte_len());
    let rhs = vec![0.0; 200_000];
    let mut expected = vec![1.0; 200_000];
    let mut actual = vec![1.0; 200_000];
    preconditioner
        .apply_compatible_into(&rhs, &mut expected, &mut ordinary)
        .unwrap();
    preconditioner
        .apply_compatible_into(&rhs, &mut actual, &mut controlled)
        .unwrap();
    assert_eq!(bits(&actual), bits(&expected));

    // A connected multilevel fixture applies identical bits as well.
    let connected = graph(257);
    let preconditioner = CmgPreconditioner::build(
        &connected,
        CmgOptions {
            direct_threshold: 3,
            ..CmgOptions::default()
        },
    )
    .unwrap();
    assert!(preconditioner.hierarchy().levels().len() > 1);
    let rhs = connected
        .matvec(&(0..257).map(|v| (v as f64 / 5.0).cos()).collect::<Vec<_>>())
        .unwrap();
    let mut expected = vec![0.0; 257];
    let mut actual = vec![0.0; 257];
    preconditioner
        .apply_compatible_into(
            &rhs,
            &mut expected,
            &mut preconditioner.try_workspace().unwrap(),
        )
        .unwrap();
    preconditioner
        .apply_compatible_into(
            &rhs,
            &mut actual,
            &mut preconditioner.try_workspace_controlled(&never).unwrap(),
        )
        .unwrap();
    assert_eq!(bits(&actual), bits(&expected));
}

#[test]
fn batch_workspace_preparation_polls_inside_initialization_and_recovers() {
    let connected = graph(4_097);
    let preconditioner = CmgPreconditioner::build(
        &connected,
        CmgOptions {
            direct_threshold: 3,
            ..CmgOptions::default()
        },
    )
    .unwrap();
    assert!(preconditioner.hierarchy().levels().len() > 1);
    let never = || false;
    for columns in [1, 3] {
        let bytes = CmgBatchWorkspace::allocation_bytes(&preconditioner, columns).unwrap();
        assert!(matches!(
            CmgBatchWorkspace::try_new_controlled(&preconditioner, columns, bytes - 1, &never),
            Err(CmgError::MemoryBudgetExceeded { .. })
        ));
        let count = StopAt::new(usize::MAX);
        let query = || count.query();
        let mut controlled =
            CmgBatchWorkspace::try_new_controlled(&preconditioner, columns, bytes, &query).unwrap();
        // Eight units per initialized value: the zero fill itself is polled.
        assert!(count.polls() >= 3 * 4_097 * columns * 8 / 65_536);
        for stop in [0, 1, count.polls() / 2, count.polls() - 1] {
            let cancel = StopAt::new(stop);
            let query = || cancel.query();
            assert!(matches!(
                CmgBatchWorkspace::try_new_controlled(&preconditioner, columns, bytes, &query),
                Err(CmgError::Cancelled { .. })
            ));
            assert_eq!(cancel.polls(), stop + 1);
        }
        let mut ordinary = CmgBatchWorkspace::try_new(&preconditioner, columns, bytes).unwrap();
        assert_eq!(controlled.capacity(), ordinary.capacity());
        let rhs: Vec<f64> = connected
            .matvec(
                &(0..4_097)
                    .map(|v| (v as f64 / 7.0).sin())
                    .collect::<Vec<_>>(),
            )
            .unwrap()
            .into_iter()
            .flat_map(|value| (0..columns).map(move |lane| value * (lane + 1) as f64))
            .collect();
        let mut expected = vec![0.0; rhs.len()];
        let mut actual = vec![0.0; rhs.len()];
        preconditioner
            .apply_compatible_batch_into(
                &rhs,
                &mut expected,
                columns,
                &mut ordinary,
                ApplyContext::default(),
            )
            .unwrap();
        preconditioner
            .apply_compatible_batch_into(
                &rhs,
                &mut actual,
                columns,
                &mut controlled,
                ApplyContext::default(),
            )
            .unwrap();
        assert_eq!(bits(&actual), bits(&expected));
    }
}
