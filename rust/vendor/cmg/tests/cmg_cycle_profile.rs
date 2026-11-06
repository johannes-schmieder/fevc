#![cfg(feature = "cycle-profiling")]

use cmg::{
    CmgApplyPhase as Phase, CmgOptions, CmgPreconditioner, Components, Laplacian, ParallelCmgPlan,
    ParallelExecutor, ParallelOptions, PcgOptions, PcgWorkspace,
};

#[test]
fn exclusive_cycle_counts_and_arithmetic_match_recursive_execution() {
    for (n, pairs) in [(0, 0), (1, 0), (64, 0), (81, 20), (0, 48)] {
        let graph = Laplacian::from_edges(
            n + 2 * pairs,
            (0..n.saturating_sub(1))
                .map(|v| (v, v + 1, 1.0))
                .chain((0..pairs).map(|c| (n + 2 * c, n + 2 * c + 1, 2.0))),
        )
        .unwrap();
        for max_levels in [1, 128] {
            let options = CmgOptions {
                direct_threshold: 5,
                max_levels,
                ..CmgOptions::default()
            };
            for prune in [false, true] {
                #[cfg(feature = "experimental-components")]
                let pre = CmgPreconditioner::build_with(
                    &graph,
                    cmg::BuildOptions {
                        hierarchy: options,
                        components: cmg::ComponentBuildOptions {
                            prune_coarse_isolates: prune,
                            preserve_unpruned_stopping: true,
                            factor_terminal_components: true,
                        },
                    },
                    cmg::BuildContext::default(),
                )
                .unwrap();
                #[cfg(not(feature = "experimental-components"))]
                let pre = {
                    let _ = prune;
                    CmgPreconditioner::build(&graph, options).unwrap()
                };
                let mut x: Vec<_> = (0..graph.vertex_count())
                    .map(|i| ((i * 13) % 17) as f64)
                    .collect();
                Components::from_laplacian(&graph)
                    .center_in_place(&mut x)
                    .unwrap();
                let rhs = graph.matvec(&x).unwrap();
                let mut ws = pre.workspace();
                let mut expected = vec![0.0; rhs.len()];
                let mut actual = expected.clone();
                for _ in 0..2 {
                    pre.apply_compatible_into(&rhs, &mut expected, &mut ws)
                        .unwrap();
                    let profile = pre
                        .profile_apply_compatible_into(
                            &rhs,
                            &mut actual,
                            &mut ws,
                            cmg::ApplyContext::default(),
                        )
                        .unwrap();
                    assert!(
                        actual
                            .iter()
                            .zip(&expected)
                            .all(|(a, b)| a.to_bits() == b.to_bits())
                    );
                    let mut visits = 1;
                    let mut iterations_per_call = 1;
                    for (index, (sample, level)) in profile
                        .levels()
                        .iter()
                        .zip(pre.hierarchy().levels())
                        .enumerate()
                    {
                        assert_eq!(sample.visits(), visits);
                        if level.is_terminal() {
                            assert_eq!(sample.calls(Phase::Terminal), visits);
                            assert_eq!(sample.iterations(), 0);
                        } else {
                            let iterations = visits * iterations_per_call;
                            assert_eq!(sample.iterations(), iterations);
                            assert_eq!(sample.calls(Phase::Initialization), visits);
                            assert_eq!(sample.calls(Phase::Smoothing), 2 * iterations - visits);
                            let child = &pre.hierarchy().levels()[index + 1];
                            let next = if cfg!(feature = "experimental-components")
                                && child.graph().vertex_count() == 0
                            {
                                0
                            } else {
                                iterations
                            };
                            for phase in [
                                Phase::ResidualMatvec,
                                Phase::Restriction,
                                Phase::Centering,
                                Phase::Prolongation,
                            ] {
                                assert_eq!(sample.calls(phase), next);
                            }
                            visits = next;
                            iterations_per_call = level.repeat();
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn planned_profile_preserves_solution_and_accounts_exclusive_time() {
    let graph =
        Laplacian::from_edges(129, (0..128).map(|i| (i, i + 1, 1.0 + (i % 7) as f64))).unwrap();
    let pre = CmgPreconditioner::build(
        &graph,
        CmgOptions {
            direct_threshold: 8,
            ..CmgOptions::default()
        },
    )
    .unwrap();
    let rhs = graph
        .matvec(&(0..129).map(|i| ((i * 7) % 19) as f64).collect::<Vec<_>>())
        .unwrap();
    for threads in [1, 2, 3, 4] {
        let executor = ParallelExecutor::new(ParallelOptions {
            threads,
            min_parallel_len: 1,
            reduction_chunk_size: 16,
            ..ParallelOptions::default()
        })
        .unwrap();
        let plan = ParallelCmgPlan::build(&pre, executor.team()).unwrap();
        let mut ws = PcgWorkspace::new(&pre);
        let reference = cmg::solve_pcg_with(
            &graph,
            &pre,
            &rhs,
            PcgOptions::default(),
            &mut ws,
            cmg::SolveContext {
                plan: Some(&plan),
                team: Some(executor.team()),
                ..cmg::SolveContext::default()
            },
        )
        .unwrap();
        let actual = cmg::profile_pcg(
            &graph,
            &pre,
            &rhs,
            PcgOptions::default(),
            cmg::SolveContext {
                plan: Some(&plan),
                team: Some(executor.team()),
                ..cmg::SolveContext::default()
            },
        )
        .unwrap();
        assert!(
            actual
                .solution()
                .iter()
                .zip(reference.solution())
                .all(|(a, b)| a.to_bits() == b.to_bits())
        );
        assert_eq!(actual.iterations(), reference.iterations());
        assert_eq!(actual.residual_norm(), reference.residual_norm());
        let profile = actual.profile();
        assert_eq!(
            profile.cycle().levels()[0].visits(),
            profile.preconditioner().calls()
        );
        assert!(profile.cycle().attributed_nanoseconds() <= profile.preconditioner().nanoseconds());
        assert!(profile.attributed_nanoseconds() <= profile.total_nanoseconds());
    }
}
