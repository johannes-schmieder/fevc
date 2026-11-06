#![cfg(feature = "parallel")]
//! Team-carrying solves and applications are bit-identical at every worker
//! ceiling, one included, and in every execution mode: reductions and
//! single-component centering use the team's fixed-chunk tree whenever a
//! length reaches its threshold, and the serial and concurrent modes run on a
//! one-lane view of the same team. Row operators (gather) reproduce the edge
//! list (scatter). Team-free calls keep the serial compensated recurrence.

use cmg::{
    ApplyContext, CmgOptions, CmgPreconditioner, Laplacian, ParallelCmgPlan, ParallelExecutor,
    ParallelOptions, ParallelPcgExecution, ParallelPcgPolicy, ParallelPcgSolver, PcgBatchMut,
    PcgBatchRef, PcgBatchWorkspace, PcgDiagnostics, PcgOptions, PcgResult, PcgWorkspace,
    SolveContext, solve_pcg_batch_into, solve_pcg_with,
};

/// One connected component of 3,000 vertices: a weighted path plus
/// deterministic chords, so coarse levels stay above the 512-entry threshold.
type Edges = Vec<(usize, usize, f64)>;

fn problem() -> (Laplacian, Edges, Vec<Vec<f64>>) {
    let n = 3_000;
    let mut edges: Vec<(usize, usize, f64)> = (0..n - 1)
        .map(|v| (v, v + 1, 0.5 + (v % 7) as f64 / 5.0))
        .collect();
    edges.extend((0..n).map(|v| (v, (v * 37 + 11) % n, 0.25 + (v % 5) as f64 / 3.0)));
    edges.retain(|&(u, v, _)| u != v);
    let graph = Laplacian::from_edges(n, edges.clone()).unwrap();
    let right_hand_sides = (0..3)
        .map(|k| {
            let target: Vec<f64> = (0..n)
                .map(|v| ((v * 29 + k * 13) % 97) as f64 / 7.0 - 6.0)
                .collect();
            graph.matvec(&target).unwrap()
        })
        .collect();
    (graph, edges, right_hand_sides)
}

/// Small thresholds: reductions and centering use the fixed-chunk tree from
/// 512 entries on, so the 3,000-vertex finest level and its large coarse
/// levels exercise it.
fn executor(threads: usize) -> ParallelExecutor {
    ParallelExecutor::new(ParallelOptions {
        threads,
        min_parallel_len: 64,
        reduction_chunk_size: 64,
        ..ParallelOptions::default()
    })
    .unwrap()
}

fn options() -> CmgOptions {
    CmgOptions {
        direct_threshold: 64,
        ..CmgOptions::default()
    }
}

/// Solution bits and diagnostics (their debug form is bit-exact for
/// finite values).
fn record(result: &PcgResult) -> (Vec<u64>, String) {
    (
        result.solution().iter().map(|x| x.to_bits()).collect(),
        format!("{:?}", result.diagnostics()),
    )
}

/// An independent residual from the edge list, without CMG kernels.
fn relative_residual(n: usize, edges: &[(usize, usize, f64)], x: &[f64], b: &[f64]) -> f64 {
    let mut product = vec![0.0; n];
    for &(u, v, w) in edges {
        let flow = w * (x[u] - x[v]);
        product[u] += flow;
        product[v] -= flow;
    }
    let residual: f64 = product
        .iter()
        .zip(b)
        .map(|(p, b)| (b - p) * (b - p))
        .sum::<f64>()
        .sqrt();
    let scale: f64 = b.iter().map(|b| b * b).sum::<f64>().sqrt();
    residual / scale
}

#[test]
fn team_solves_are_identical_across_workers_and_execution_modes() {
    let (graph, edges, right_hand_sides) = problem();
    let n = graph.vertex_count();
    let pcg = PcgOptions::default();
    let policies = [
        // Serial single solve (below 350,000 edges), across-RHS batches.
        ParallelPcgPolicy::default(),
        // Planned single solve, across-RHS batches from two lanes.
        ParallelPcgPolicy {
            min_planned_edges: 1,
            min_across_rhs_concurrency: 2,
        },
        // Planned single solve and batch.
        ParallelPcgPolicy {
            min_planned_edges: 1,
            min_across_rhs_concurrency: 64,
        },
    ];
    let mut single = None;
    let mut batch = None;
    let mut modes = std::collections::BTreeSet::new();
    for threads in [1, 2, 4, 8] {
        let executor = executor(threads);
        let team = executor.team();
        for policy in policies {
            let label = format!("{threads} threads, {policy:?}");
            let solver =
                ParallelPcgSolver::build_with_policy(&graph, options(), team, policy).unwrap();
            let one = record(&solver.solve(&right_hand_sides[0], pcg, team).unwrap());
            modes.insert(format!(
                "{:?}",
                solver.select_batch_execution(1, team).unwrap().execution()
            ));
            let many = solver.solve_batch(&right_hand_sides, pcg, team).unwrap();
            modes.insert(format!("{:?}", many.report().execution()));
            let many: Vec<_> = many.results().iter().map(record).collect();
            assert_eq!(many[0], one, "{label}: batch and single solves differ");
            match (&single, &batch) {
                (Some(s), Some(b)) => {
                    assert_eq!(&one, s, "{label}: single solve");
                    assert_eq!(&many, b, "{label}: batch");
                }
                _ => {
                    single = Some(one);
                    batch = Some(many);
                }
            }
        }
    }
    // All three modes were exercised.
    for mode in [
        ParallelPcgExecution::Serial,
        ParallelPcgExecution::Planned,
        ParallelPcgExecution::AcrossRightHandSides,
    ] {
        assert!(
            modes.contains(&format!("{mode:?}")),
            "{mode:?} not in {modes:?}"
        );
    }
    // Independent science: an edge-list residual (no CMG kernel) reproduces
    // each solve's reported relative residual (b - Lx cancels about eight
    // digits, so its summation order shows at 1e-6 relative), whose
    // certificate holds; the
    // team-free serial recurrence certifies too and agrees within rounding of
    // the reduction order.
    let preconditioner = CmgPreconditioner::build(&graph, options()).unwrap();
    let executor = executor(1);
    let solver = ParallelPcgSolver::build_with_policy(
        &graph,
        options(),
        executor.team(),
        ParallelPcgPolicy::default(),
    )
    .unwrap();
    let team_results = solver
        .solve_batch(&right_hand_sides, pcg, executor.team())
        .unwrap();
    for ((rhs, (bits, _)), result) in right_hand_sides
        .iter()
        .zip(batch.unwrap())
        .zip(team_results.results())
    {
        let x: Vec<f64> = bits.into_iter().map(f64::from_bits).collect();
        let diagnostics = result.diagnostics();
        let independent = relative_residual(n, &edges, &x, rhs);
        assert!(
            (independent - diagnostics.relative_residual()).abs() <= 1.0e-6 * independent,
            "{independent} vs {diagnostics:?}"
        );
        assert!(diagnostics.backward_error() <= pcg.relative_tolerance);
        assert!(diagnostics.residual_norm() <= diagnostics.tolerance());
        let mut workspace = PcgWorkspace::new(&preconditioner);
        let serial = solve_pcg_with(
            &graph,
            &preconditioner,
            rhs,
            pcg,
            &mut workspace,
            SolveContext::default(),
        )
        .unwrap();
        assert!(serial.diagnostics().backward_error() <= pcg.relative_tolerance);
        assert_eq!(serial.diagnostics().iterations(), diagnostics.iterations());
        // Solutions are determined up to the nullspace: compare centered.
        let mean = |v: &[f64]| v.iter().sum::<f64>() / v.len() as f64;
        let (xm, sm) = (mean(&x), mean(serial.solution()));
        let scale = x.iter().map(|v| (v - xm).abs()).fold(1.0, f64::max);
        for (a, b) in x.iter().zip(serial.solution()) {
            assert!(((a - xm) - (b - sm)).abs() <= 1.0e-6 * scale);
        }
    }
}

#[test]
fn one_lane_applications_and_row_operators_reproduce_every_ceiling() {
    let (graph, _, right_hand_sides) = problem();
    let n = graph.vertex_count();
    let preconditioner = CmgPreconditioner::build(&graph, options()).unwrap();
    let rhs = &right_hand_sides[1];
    let mut reference: Option<Vec<u64>> = None;
    let mut operators = std::collections::BTreeSet::new();
    for threads in [1, 2, 4, 8] {
        let executor = executor(threads);
        let team = executor.team();
        let plan = ParallelCmgPlan::build(&preconditioner, team).unwrap();
        operators.insert(plan.operator_count());
        for context in [
            ApplyContext {
                plan: Some(&plan),
                team: Some(team),
                cancel: None,
            },
            ApplyContext {
                plan: None,
                team: Some(team.single_lane()),
                cancel: None,
            },
        ] {
            let mut workspace = preconditioner.try_workspace().unwrap();
            let mut output = vec![0.0; n];
            preconditioner
                .apply_compatible_into_with(rhs, &mut output, &mut workspace, context)
                .unwrap();
            let bits: Vec<u64> = output.iter().map(|x| x.to_bits()).collect();
            match &reference {
                Some(expected) => assert_eq!(&bits, expected, "{threads} threads"),
                None => reference = Some(bits),
            }
        }
    }
    // Gather operators existed for some ceilings and not others: the
    // representation is a resource choice, not an arithmetic one.
    assert!(operators.contains(&0) && operators.iter().any(|&count| count > 0));
}

/// The caller-buffer batch path (fereg's): the solver's serial context, the
/// planned context and concurrent right-hand sides on the team produce the
/// same solutions and diagnostics at every worker ceiling.
#[test]
fn caller_buffer_batches_are_identical_across_workers_and_contexts() {
    let (graph, _, right_hand_sides) = problem();
    let n = graph.vertex_count();
    let count = right_hand_sides.len();
    let rhs: Vec<f64> = right_hand_sides.concat();
    let preconditioner = CmgPreconditioner::build(&graph, options()).unwrap();
    let mut reference: Option<(Vec<u64>, String)> = None;
    for threads in [1, 2, 4, 8] {
        let executor = executor(threads);
        let team = executor.team();
        let plan = ParallelCmgPlan::build(&preconditioner, team).unwrap();
        for (name, context) in [
            ("serial", ParallelPcgSolver::serial(team)),
            (
                "planned",
                SolveContext {
                    plan: Some(&plan),
                    team: Some(team),
                    ..SolveContext::default()
                },
            ),
            (
                "concurrent",
                SolveContext {
                    team: Some(team),
                    concurrent_right_hand_sides: true,
                    ..SolveContext::default()
                },
            ),
        ] {
            let mut bank =
                PcgBatchWorkspace::new_bank_with_budget(&preconditioner, count, usize::MAX)
                    .unwrap();
            let mut solutions = vec![0.0; n * count];
            let mut diagnostics = vec![PcgDiagnostics::default(); count];
            solve_pcg_batch_into(
                &graph,
                &preconditioner,
                PcgBatchRef::contiguous(&rhs, count, n).unwrap(),
                None,
                PcgBatchMut::contiguous(&mut solutions, count, n).unwrap(),
                &mut diagnostics,
                PcgOptions::default(),
                &mut bank,
                context,
            )
            .unwrap();
            let observed = (
                solutions.iter().map(|x| x.to_bits()).collect(),
                format!("{diagnostics:?}"),
            );
            match &reference {
                Some(expected) => assert_eq!(&observed, expected, "{threads} threads, {name}"),
                None => reference = Some(observed),
            }
        }
    }
}
