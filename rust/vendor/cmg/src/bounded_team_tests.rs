//! Witnesses for bounded cancellation and the worker ceiling on a team:
//! construction stages, plan and workspace preparation, PCG publication and
//! concurrency on a pool larger than the ceiling.

use std::collections::BTreeSet;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use crate::execution::gauge;
use crate::{
    ApplyContext, BuildContext, BuildOptions, CmgBatchWorkspace, CmgError, CmgHierarchy,
    CmgOptions, CmgPreconditioner, Laplacian, ParallelCmgPlan, ParallelOptions, PcgBatchMut,
    PcgBatchRef, PcgBatchWorkspace, PcgDiagnostics, PcgOptions, SolveContext, Team,
    solve_pcg_batch_into,
};

/// Each vertex joins its next five neighbors: m = 5n, dense enough for row
/// operators and parallel CSR construction, and above the setup threshold.
fn banded(n: usize) -> Vec<(usize, usize, f64)> {
    (0..n)
        .flat_map(|i| {
            (1..=5)
                .filter(move |d| i + d < n)
                .map(move |d| (i, i + d, 1.0 + ((i * d) % 7) as f64))
        })
        .collect()
}

fn options() -> ParallelOptions {
    ParallelOptions {
        min_parallel_len: 64,
        reduction_chunk_size: 256,
        ..ParallelOptions::default()
    }
}

fn pool(threads: usize) -> rayon::ThreadPool {
    rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .unwrap()
}

/// Counts queries; cancels from the zero-based query `stop` onwards.
struct Stop {
    calls: AtomicUsize,
    stop: usize,
}

impl Stop {
    fn new(stop: usize) -> Self {
        Self {
            calls: AtomicUsize::new(0),
            stop,
        }
    }

    fn query(&self) -> bool {
        self.calls.fetch_add(1, Ordering::SeqCst) >= self.stop
    }

    fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

/// Cancel `operation` at query `stop`; the reported phase, or `None` when the
/// run finished first (lane polls depend on chunk claiming, so a run may end
/// before `stop`; a query that answered true never yields success).
fn cancel_at<T>(
    stop: usize,
    operation: &impl Fn(crate::Cancellation<'_>) -> Result<T, CmgError>,
) -> Option<&'static str> {
    let cancel = Stop::new(stop);
    match operation(&|| cancel.query()) {
        Err(CmgError::Cancelled { phase }) => Some(phase),
        Ok(_) => {
            assert!(cancel.calls() <= stop, "poll {stop} ignored");
            None
        }
        Err(error) => panic!("poll {stop}: {error}"),
    }
}

/// Run `operation` once uncancelled to count its queries, then cancel at
/// about `samples` evenly spread indices; return the ordered phases.
fn sweep<T>(
    samples: usize,
    operation: impl Fn(crate::Cancellation<'_>) -> Result<T, CmgError>,
) -> (Vec<(usize, &'static str)>, BTreeSet<&'static str>) {
    let never = Stop::new(usize::MAX);
    operation(&|| never.query()).unwrap_or_else(|error| panic!("uncancelled run: {error}"));
    let total = never.calls();
    assert!(total > 0, "{total} queries");
    let stride = (total / samples).max(1);
    let ordered: Vec<_> = (0..total)
        .step_by(stride)
        .chain([total - 1])
        .filter_map(|stop| cancel_at(stop, &operation).map(|phase| (stop, phase)))
        .collect();
    let phases = ordered.iter().map(|&(_, phase)| phase).collect();
    (ordered, phases)
}

/// Cancel `operation` at the first meter poll of `phase` (immediately after
/// a meter poll of `after`, when given): the caller's sticky query turns true
/// when the test-only probe around the production query (`cancel::probe`)
/// names `phase` and stays true. Unlike a query ordinal, this reaches the
/// phase whatever the lanes' chunk claiming. `after` tells checkpoints apart
/// that share a phase name (the parallel CSR finish directly follows the
/// parallel fill's join poll; the serial one never does). The production query
/// decides and the meter reports it, so the run must fail with exactly
/// `phase`, publish nothing, and never poll `publication` afterwards.
fn latch<T>(
    phase: &'static str,
    after: Option<&str>,
    publication: Option<&str>,
    operation: impl Fn(crate::Cancellation<'_>) -> Result<T, CmgError>,
) {
    let latched = AtomicBool::new(false);
    let published = AtomicBool::new(false);
    // Meter polls are the coordinator's and run in sequence; lane polls
    // carry no probe phase and are ignored here.
    let previous = std::sync::Mutex::new(None::<&'static str>);
    let query = || {
        let current = crate::cancel::probe::phase();
        if latched.load(Ordering::SeqCst) {
            if current.is_some() && current == publication {
                published.store(true, Ordering::SeqCst);
            }
            return true;
        }
        let Some(current) = current else {
            return false;
        };
        let before = previous.lock().unwrap().replace(current);
        if current == phase && (after.is_none() || before == after) {
            latched.store(true, Ordering::SeqCst);
            return true;
        }
        false
    };
    let result = operation(&query);
    assert!(
        latched.load(Ordering::SeqCst),
        "{phase} (after {after:?}) was never polled"
    );
    match result {
        Err(CmgError::Cancelled { phase: reported }) => assert_eq!(reported, phase),
        Ok(_) => panic!("{phase}: published after cancellation"),
        Err(error) => panic!("{phase}: {error}"),
    }
    assert!(
        !published.load(Ordering::SeqCst),
        "{phase}: publication polled after cancellation"
    );
}

/// Cancel at every query between the samples bracketing `phase`'s first and
/// last sampled occurrence, collecting the phases met there.
fn around<T>(
    ordered: &[(usize, &'static str)],
    phase: &str,
    operation: impl Fn(crate::Cancellation<'_>) -> Result<T, CmgError>,
) -> BTreeSet<&'static str> {
    let first = ordered.iter().position(|&(_, p)| p == phase).unwrap();
    let last = ordered.iter().rposition(|&(_, p)| p == phase).unwrap();
    let start = ordered[first.saturating_sub(1)].0;
    let end = ordered[(last + 1).min(ordered.len() - 1)].0;
    let mut phases = BTreeSet::new();
    for stop in (start..=end).filter(|stop| {
        // Interior of the stage is already sampled; sweep its two edges.
        *stop <= ordered[first].0 || *stop >= ordered[last].0
    }) {
        phases.extend(cancel_at(stop, &operation));
    }
    phases
}

#[test]
fn team_construction_cancels_inside_mapping_compaction_and_sort() {
    let pool = pool(4);
    let team = Team::new(&pool, 4, options()).unwrap();
    let edges = banded(30_000);
    let reference = Laplacian::from_edges(30_000, edges.iter().copied()).unwrap();
    // Initial graph from an iterator: serial mapping, in-place team sort.
    let (_, phases) = sweep(120, |cancel| {
        Laplacian::from_edges_with(
            30_000,
            edges.iter().copied(),
            BuildContext {
                team: Some(team),
                cancel: Some(cancel),
                ..BuildContext::default()
            },
        )
    });
    assert!(!phases.is_empty());
    let initial = |cancel: crate::Cancellation<'_>| {
        Laplacian::from_edges_with(
            30_000,
            edges.iter().copied(),
            BuildContext {
                team: Some(team),
                cancel: Some(cancel),
                ..BuildContext::default()
            },
        )
    };
    latch("team edge sort", None, None, initial);
    let never = || false;
    let graph = Laplacian::from_edges_with(
        30_000,
        edges.iter().copied(),
        BuildContext {
            team: Some(team),
            cancel: Some(&never),
            ..BuildContext::default()
        },
    )
    .unwrap();
    assert_eq!(graph, reference);

    // Hierarchy: the fine contraction maps, compacts and sorts on the team.
    let options = BuildOptions::from(CmgOptions::default());
    let serial = CmgHierarchy::build_with(&graph, options, BuildContext::default()).unwrap();
    let build = |cancel: crate::Cancellation<'_>| {
        CmgHierarchy::build_with(
            &graph,
            options,
            BuildContext {
                team: Some(team),
                cancel: Some(cancel),
                ..BuildContext::default()
            },
        )
    };
    let (ordered, mut phases) = sweep(400, build);
    phases.extend(around(&ordered, "parallel contracted edge mapping", build));
    // The mapping stage polls the raw query on its lanes (no meter phase),
    // so it stays covered by the bracketing sweep; the coordinator's phases
    // are latched exactly.
    assert!(
        phases.contains("parallel contracted edge mapping"),
        "{phases:?}"
    );
    for phase in [
        "parallel contracted edge slots",
        "parallel contracted edge compaction",
        "team edge sort",
    ] {
        latch(phase, None, None, build);
    }
    let hosted = CmgHierarchy::build_with(
        &graph,
        options,
        BuildContext {
            team: Some(team),
            cancel: Some(&never),
            ..BuildContext::default()
        },
    )
    .unwrap();
    assert_eq!(hosted, serial);
}

#[test]
fn plan_and_workspace_preparation_cancel_at_every_stage_and_rebuild_exactly() {
    let pool = pool(4);
    let team = Team::new(&pool, 4, options()).unwrap();
    let graph = Laplacian::from_edges(30_000, banded(30_000)).unwrap();
    let preconditioner = CmgPreconditioner::build_with(
        &graph,
        CmgOptions {
            direct_threshold: 64,
            ..CmgOptions::default()
        }
        .into(),
        BuildContext::default(),
    )
    .unwrap();
    let reference_plan = ParallelCmgPlan::build(&preconditioner, team).unwrap();
    assert!(reference_plan.operator_count() > 0);
    let plan_with = |cancel: crate::Cancellation<'_>| {
        ParallelCmgPlan::build_with(
            &preconditioner,
            BuildContext {
                team: Some(team),
                cancel: Some(cancel),
                budget_bytes: Some(usize::MAX),
            },
        )
    };
    // Every required caller phase, the single CSR finish checkpoint
    // included, cancels exactly there: latched by phase, not by a sampled
    // query ordinal (lanes claim CSR fill blocks dynamically and keep
    // per-lane counters, so ordinals move between runs).
    for (phase, after) in [
        ("plan operators", None),
        ("CSR row counts", None),
        ("CSR row offsets", None),
        ("CSR incoming scatter", None),
        ("parallel CSR fill", None),
        ("CSR construction finish", Some("parallel CSR fill")),
    ] {
        latch(phase, after, Some("plan publication"), plan_with);
    }
    // Supplementary coverage: sampled ordinals land in plan preparation.
    let (_, phases) = sweep(300, plan_with);
    assert!(
        phases.iter().all(|phase| phase.starts_with("CSR")
            || phase.starts_with("parallel CSR")
            || phase.starts_with("plan")),
        "{phases:?}"
    );
    // Without eligible levels the plan's polls are deterministic; the last
    // one precedes publication.
    let path = Laplacian::from_edges(2_000, (1..2_000).map(|v| (v - 1, v, 1.0))).unwrap();
    let sparse =
        CmgPreconditioner::build_with(&path, CmgOptions::default().into(), BuildContext::default())
            .unwrap();
    let plan_of = |cancel: crate::Cancellation<'_>| {
        ParallelCmgPlan::build_with(
            &sparse,
            BuildContext {
                team: Some(team),
                cancel: Some(cancel),
                ..BuildContext::default()
            },
        )
        .map(|plan| assert_eq!(plan.operator_count(), 0))
    };
    let (ordered, _) = sweep(usize::MAX, plan_of);
    assert_eq!(
        ordered.last().map(|&(_, phase)| phase),
        Some("plan publication")
    );
    // A plan rebuilt after the cancellations applies identical cycle bits.
    let never = || false;
    let plan = ParallelCmgPlan::build_with(
        &preconditioner,
        BuildContext {
            team: Some(team),
            cancel: Some(&never),
            ..BuildContext::default()
        },
    )
    .unwrap();
    let rhs = graph
        .matvec(
            &(0..30_000)
                .map(|v| ((v % 97) as f64).sin())
                .collect::<Vec<_>>(),
        )
        .unwrap();
    let apply = |plan: &ParallelCmgPlan| {
        let mut workspace = preconditioner.try_workspace().unwrap();
        let mut output = vec![0.0; rhs.len()];
        preconditioner
            .apply_compatible_into_with(
                &rhs,
                &mut output,
                &mut workspace,
                ApplyContext {
                    plan: Some(plan),
                    team: Some(team),
                    cancel: None,
                },
            )
            .unwrap();
        output.iter().map(|v| v.to_bits()).collect::<Vec<_>>()
    };
    assert_eq!(apply(&plan), apply(&reference_plan));

    // PCG workspace bank: layout and preparation.
    let bytes = PcgBatchWorkspace::bank_allocation_bytes(&preconditioner, 3).unwrap();
    let (_, phases) = sweep(200, |cancel| {
        PcgBatchWorkspace::bank_allocation_bytes_with(&preconditioner, 3, Some(cancel))
            .map(|value| assert_eq!(value, bytes))
    });
    assert!(phases.contains("CMG workspace layouts"), "{phases:?}");
    let (_, phases) = sweep(300, |cancel| {
        PcgBatchWorkspace::new_bank_with(
            &preconditioner,
            3,
            BuildContext {
                cancel: Some(cancel),
                budget_bytes: Some(bytes),
                ..BuildContext::default()
            },
        )
    });
    for phase in [
        "PCG batch RHS staging",
        "PCG projected RHS",
        "component workspace sums",
        "CMG workspace residual",
        "PCG batch workspace bank",
    ] {
        assert!(phases.contains(phase), "{phase} not in {phases:?}");
    }
    assert!(matches!(
        PcgBatchWorkspace::new_bank_with(
            &preconditioner,
            3,
            BuildContext {
                budget_bytes: Some(bytes - 1),
                ..BuildContext::default()
            },
        ),
        Err(CmgError::MemoryBudgetExceeded { .. })
    ));
    let solve = |bank: &mut [PcgBatchWorkspace]| {
        let mut solution = vec![0.0; rhs.len()];
        let mut diagnostics = [PcgDiagnostics::default()];
        solve_pcg_batch_into(
            &graph,
            &preconditioner,
            PcgBatchRef::contiguous(&rhs, 1, rhs.len()).unwrap(),
            None,
            PcgBatchMut::contiguous(&mut solution, 1, rhs.len()).unwrap(),
            &mut diagnostics,
            PcgOptions::default(),
            bank,
            SolveContext::default(),
        )
        .unwrap();
        (
            solution.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
            diagnostics,
        )
    };
    let mut fresh = PcgBatchWorkspace::new_bank_with(
        &preconditioner,
        3,
        BuildContext {
            cancel: Some(&never),
            budget_bytes: Some(bytes),
            ..BuildContext::default()
        },
    )
    .unwrap();
    let mut ordinary = PcgBatchWorkspace::new_bank_with_budget(&preconditioner, 3, bytes).unwrap();
    assert_eq!(solve(&mut fresh), solve(&mut ordinary));

    // Interleaved workspace layout is metered under its controlled query.
    let layout = CmgBatchWorkspace::allocation_bytes(&preconditioner, 3).unwrap();
    let (_, phases) = sweep(50, |cancel| {
        CmgBatchWorkspace::allocation_bytes_controlled(&preconditioner, 3, cancel)
            .map(|value| assert_eq!(value, layout))
    });
    assert!(phases.contains("CMG batch layout"), "{phases:?}");
    let (_, phases) = sweep(200, |cancel| {
        CmgBatchWorkspace::try_new_controlled(&preconditioner, 3, layout, cancel)
    });
    assert!(phases.contains("CMG batch layout"), "{phases:?}");
    assert!(phases.contains("CMG batch workspace"), "{phases:?}");
}

#[test]
fn pcg_cancellation_after_join_or_at_publication_is_reported() {
    let pool = pool(4);
    let team = Team::new(&pool, 4, options()).unwrap();
    let n = 20_000;
    let graph = Laplacian::from_edges(n, banded(n)).unwrap();
    let preconditioner = CmgPreconditioner::build_with(
        &graph,
        CmgOptions::default().into(),
        BuildContext::default(),
    )
    .unwrap();
    let count = 4;
    let rhs: Vec<f64> = (0..count)
        .flat_map(|lane| {
            graph
                .matvec(
                    &(0..n)
                        .map(|v| ((v * (lane + 3)) % 41) as f64)
                        .collect::<Vec<_>>(),
                )
                .unwrap()
        })
        .collect();
    let mut bank =
        PcgBatchWorkspace::new_bank_with_budget(&preconditioner, count, usize::MAX).unwrap();
    let mut run = |rhs: &[f64], context: SolveContext<'_>| {
        let mut solutions = vec![0.0; n * count];
        let mut diagnostics = vec![PcgDiagnostics::default(); count];
        solve_pcg_batch_into(
            &graph,
            &preconditioner,
            PcgBatchRef::contiguous(rhs, count, n).unwrap(),
            None,
            PcgBatchMut::contiguous(&mut solutions, count, n).unwrap(),
            &mut diagnostics,
            PcgOptions::default(),
            &mut bank,
            context,
        )
        .map(|()| solutions.iter().map(|v| v.to_bits()).collect::<Vec<_>>())
    };
    for concurrent in [true, false] {
        let base = SolveContext {
            team: Some(team),
            concurrent_right_hand_sides: concurrent,
            ..SolveContext::default()
        };
        let reference = run(&rhs, base).unwrap();
        let never = Stop::new(usize::MAX);
        assert_eq!(
            run(
                &rhs,
                SolveContext {
                    cancel: Some(&|| never.query()),
                    ..base
                }
            )
            .unwrap(),
            reference
        );
        let total = never.calls();
        // The last polls belong to the coordinator after every lane joined:
        // the join poll, bounded publication and the final poll before success.
        for stop in [total - 1, total - 2, total - 3] {
            let cancel = Stop::new(stop);
            assert!(
                matches!(
                    run(
                        &rhs,
                        SolveContext {
                            cancel: Some(&|| cancel.query()),
                            ..base
                        }
                    ),
                    Err(CmgError::Cancelled { .. })
                ),
                "concurrent={concurrent} stop={stop} of {total}"
            );
        }
        assert_eq!(run(&rhs, base).unwrap(), reference);
    }
    // A numerical failure of an earlier RHS cannot mask a lane's cancellation.
    let mut bad = rhs.clone();
    bad[3] = f64::NAN;
    let concurrent = SolveContext {
        team: Some(team),
        concurrent_right_hand_sides: true,
        ..SolveContext::default()
    };
    assert!(matches!(
        run(&bad, concurrent),
        Err(CmgError::NonFiniteMatrixValue { .. })
    ));
    let always = || true;
    assert!(matches!(
        run(
            &bad,
            SolveContext {
                cancel: Some(&always),
                ..concurrent
            }
        ),
        Err(CmgError::Cancelled { .. })
    ));
}

#[test]
fn ordinary_kernels_never_exceed_the_worker_ceiling_on_a_larger_pool() {
    let _witness = gauge::WITNESS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let pool = pool(8);
    let team = Team::new(&pool, 3, options()).unwrap();
    gauge::watch(&pool);
    let n = 30_000;
    let edges = banded(n);
    let context = BuildContext {
        team: Some(team),
        ..BuildContext::default()
    };
    // Initial graph (bounded staged map and sort) and hierarchy contraction.
    let graph = Laplacian::build(n, &edges, context).unwrap();
    let graph_from_iter = Laplacian::from_edges_with(n, edges.iter().copied(), context).unwrap();
    assert_eq!(graph, graph_from_iter);
    let preconditioner = CmgPreconditioner::build_with(
        &graph,
        CmgOptions {
            direct_threshold: 64,
            ..CmgOptions::default()
        }
        .into(),
        context,
    )
    .unwrap();
    // Plan construction (parallel CSR fill), planned and interleaved cycles
    // (rows, row operators, centering) and planned PCG reductions.
    let plan = ParallelCmgPlan::build(&preconditioner, team).unwrap();
    let rhs = graph
        .matvec(&(0..n).map(|v| ((v % 53) as f64).cos()).collect::<Vec<_>>())
        .unwrap();
    let planned = ApplyContext {
        plan: Some(&plan),
        team: Some(team),
        cancel: None,
    };
    let mut workspace = preconditioner.try_workspace().unwrap();
    let mut output = vec![0.0; n];
    preconditioner
        .apply_compatible_into_with(&rhs, &mut output, &mut workspace, planned)
        .unwrap();
    let interleaved: Vec<f64> = rhs.iter().flat_map(|&v| [v, -v]).collect();
    let mut batch = CmgBatchWorkspace::try_new(&preconditioner, 2, usize::MAX).unwrap();
    let mut batch_output = vec![0.0; 2 * n];
    preconditioner
        .apply_compatible_batch_into(&interleaved, &mut batch_output, 2, &mut batch, planned)
        .unwrap();
    // Six caller workspaces: concurrent solves still use at most three lanes.
    let count = 6;
    let batch_rhs: Vec<f64> = (0..count).flat_map(|_| rhs.iter().copied()).collect();
    let mut bank =
        PcgBatchWorkspace::new_bank_with_budget(&preconditioner, count, usize::MAX).unwrap();
    for context in [
        SolveContext {
            plan: Some(&plan),
            team: Some(team),
            ..SolveContext::default()
        },
        SolveContext {
            team: Some(team),
            concurrent_right_hand_sides: true,
            ..SolveContext::default()
        },
    ] {
        let mut solutions = vec![0.0; n * count];
        let mut diagnostics = vec![PcgDiagnostics::default(); count];
        solve_pcg_batch_into(
            &graph,
            &preconditioner,
            PcgBatchRef::contiguous(&batch_rhs, count, n).unwrap(),
            None,
            PcgBatchMut::contiguous(&mut solutions, count, n).unwrap(),
            &mut diagnostics,
            PcgOptions::default(),
            &mut bank,
            context,
        )
        .unwrap();
    }
    let max = gauge::max();
    assert!(max <= 3, "{max} concurrent CMG lanes on a ceiling of 3");
    assert!(max >= 2, "kernels did not run in parallel ({max})");
}
