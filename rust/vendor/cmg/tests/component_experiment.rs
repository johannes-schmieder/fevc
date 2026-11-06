#![cfg(feature = "experimental-components")]

use cmg::{
    CmgError, CmgOptions, CmgPreconditioner, ComponentBuildOptions, Components, GroundedLdl,
    Laplacian, PcgOptions, solve_pcg,
};

fn mixed(path: usize, pairs: usize) -> Laplacian {
    Laplacian::from_edges(
        path + pairs * 2,
        (0..path.saturating_sub(1)).map(|v| (v, v + 1, 1.0)).chain(
            (0..pairs).map(|c| (path + 2 * c, path + 2 * c + 1, 0.5 + (c % 7) as f64 / 4.0)),
        ),
    )
    .unwrap()
}

fn build(graph: &Laplacian, threshold: usize, prune: bool, blocks: bool) -> CmgPreconditioner {
    CmgPreconditioner::build_with(
        graph,
        cmg::BuildOptions {
            hierarchy: CmgOptions {
                direct_threshold: threshold,
                ..CmgOptions::default()
            },
            components: ComponentBuildOptions {
                prune_coarse_isolates: prune,
                factor_terminal_components: blocks,
                ..ComponentBuildOptions::default()
            },
        },
        cmg::BuildContext::default(),
    )
    .unwrap()
}

fn vector(graph: &Laplacian, seed: usize) -> Vec<f64> {
    let mut v: Vec<_> = (0..graph.vertex_count())
        .map(|i| ((i * 17 + seed * 13) % 43) as f64 / 8.0)
        .collect();
    Components::from_laplacian(graph)
        .center_in_place(&mut v)
        .unwrap();
    v
}

fn close(a: &[f64], b: &[f64], tol: f64) {
    assert_eq!(a.len(), b.len());
    for (&a, &b) in a.iter().zip(b) {
        assert!(
            (a - b).abs() <= tol * 1.0_f64.max(a.abs()).max(b.abs()),
            "{a} != {b}"
        );
    }
}

fn dot(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).map(|(a, b)| a * b).sum()
}

#[test]
fn connected_and_disabled_paths_preserve_the_original_hierarchy_and_cycle() {
    for graph in [mixed(128, 0), mixed(128, 30)] {
        let reference = CmgPreconditioner::build(
            &graph,
            CmgOptions {
                direct_threshold: 8,
                ..CmgOptions::default()
            },
        )
        .unwrap();
        let disabled = build(&graph, 8, false, false);
        assert_eq!(reference.hierarchy(), disabled.hierarchy());
        assert_eq!(
            reference.apply(&vector(&graph, 1)).unwrap(),
            disabled.apply(&vector(&graph, 1)).unwrap()
        );
        if Components::from_laplacian(&graph).count() == 1 {
            let enabled = build(&graph, 8, true, true);
            assert_eq!(reference.hierarchy(), enabled.hierarchy());
            assert_eq!(reference.terminal_factor(), enabled.terminal_factor());
            assert_eq!(
                reference.apply(&vector(&graph, 1)).unwrap(),
                enabled.apply(&vector(&graph, 1)).unwrap()
            );
            assert_eq!(reference.retained_bytes(), enabled.retained_bytes());
        }
    }
}

#[test]
fn mixed_graph_retires_coarse_isolates_and_retains_parent_rows() {
    let graph = mixed(4096, 1000);
    let pre = build(&graph, 700, true, false);
    assert_eq!(pre.hierarchy().report().vertex_counts(), &[6096, 1024, 256]);
    let first = &pre.hierarchy().levels()[0];
    let transfer = first.pruned_transfer().unwrap();
    assert!(first.aggregation().is_none());
    assert_eq!(transfer.fine_dimension(), 6096);
    assert_eq!(transfer.represented_coarse_dimension(), 2024);
    assert_eq!(transfer.coarse_dimension(), 1024);
    assert_eq!(transfer.entries().len(), 4096);
    for level in &pre.hierarchy().levels()[1..] {
        assert!(level.graph().diagonal().iter().all(|&x| x > 0.0));
    }
    assert_eq!(pre.workspace().dimensions(), &[6096, 1024, 256]);
    assert!(pre.workspace().byte_len() < build(&graph, 700, false, false).workspace().byte_len());
}

#[test]
fn preserving_stopping_compacts_the_original_hierarchy_without_early_dense_factors() {
    let bipartite = Laplacian::from_edges(
        656,
        (0..128)
            .flat_map(|u| (0..6).map(move |k| (u, 128 + (u + k * 17) % 128, 0.5 + k as f64 / 4.0)))
            .chain((0..200).map(|c| (256 + c * 2, 257 + c * 2, 1.0))),
    )
    .unwrap();
    for graph in [mixed(384, 200), mixed(384, 7), bipartite] {
        for (max_levels, max_fill) in [(100, 4.0), (2, 4.0), (100, 1.0)] {
            let options = CmgOptions {
                direct_threshold: 80,
                max_levels,
                max_hierarchy_nnz_factor: max_fill,
                ..CmgOptions::default()
            };
            let baseline = CmgPreconditioner::build(&graph, options).unwrap();
            let compact = CmgPreconditioner::build_with(
                &graph,
                cmg::BuildOptions {
                    hierarchy: options,
                    components: ComponentBuildOptions {
                        prune_coarse_isolates: true,
                        preserve_unpruned_stopping: true,
                        factor_terminal_components: true,
                    },
                },
                cmg::BuildContext::default(),
            )
            .unwrap();
            let old = baseline.hierarchy().levels();
            let new = compact.hierarchy().levels();
            assert_eq!(old.len(), new.len());
            for (index, (old, new)) in old.iter().zip(new).enumerate() {
                let expected = if index == 0 {
                    old.graph().clone()
                } else {
                    let mut map = vec![usize::MAX; old.graph().vertex_count()];
                    let mut n = 0;
                    for (v, &degree) in old.graph().diagonal().iter().enumerate() {
                        if degree > 0.0 {
                            map[v] = n;
                            n += 1;
                        }
                    }
                    Laplacian::from_edges(
                        n,
                        old.graph()
                            .edges()
                            .iter()
                            .map(|e| (map[e.u()], map[e.v()], e.weight())),
                    )
                    .unwrap()
                };
                assert_eq!(new.graph(), &expected);
                if new.graph().vertex_count() > 0 {
                    assert_eq!(old.terminal_reason(), new.terminal_reason());
                    assert_eq!(old.repeat(), new.repeat());
                }
            }
            if new.last().unwrap().graph().vertex_count() > 0 {
                assert_eq!(
                    baseline.terminal_factor().is_some(),
                    compact.terminal_factor().is_some()
                );
            }
            // Removed coarse rows have identically zero range. Compare the
            // complete stationary action, not only a favorable solve timing.
            for seed in [1, 7] {
                let rhs = vector(&graph, seed);
                close(
                    &baseline.apply(&rhs).unwrap(),
                    &compact.apply(&rhs).unwrap(),
                    1e-10,
                );
                let known_rhs = graph.matvec(&rhs).unwrap();
                let solved =
                    solve_pcg(&graph, &compact, &known_rhs, PcgOptions::default()).unwrap();
                let ax = graph.matvec(solved.solution()).unwrap();
                let residual = known_rhs
                    .iter()
                    .zip(ax)
                    .map(|(b, ax)| (b - ax).powi(2))
                    .sum::<f64>()
                    .sqrt();
                assert!(residual <= solved.tolerance());
            }
        }
    }
}

#[test]
fn preserving_stopping_handles_empty_children_and_finest_isolates() {
    for graph in [
        mixed(0, 200),
        Laplacian::from_edges(500, (0..127).map(|v| (v, v + 1, 1.0))).unwrap(),
        Laplacian::from_edges(0, []).unwrap(),
    ] {
        let baseline = build(&graph, 8, false, false);
        let compact = CmgPreconditioner::build_with(
            &graph,
            cmg::BuildOptions {
                hierarchy: CmgOptions {
                    direct_threshold: 8,
                    ..CmgOptions::default()
                },
                components: ComponentBuildOptions {
                    prune_coarse_isolates: true,
                    preserve_unpruned_stopping: true,
                    factor_terminal_components: true,
                },
            },
            cmg::BuildContext::default(),
        )
        .unwrap();
        let rhs = vector(&graph, 2);
        close(
            &baseline.apply(&rhs).unwrap(),
            &compact.apply(&rhs).unwrap(),
            1e-10,
        );
        assert_eq!(compact.workspace().dimensions()[0], graph.vertex_count());
        assert!(compact.hierarchy().levels().len() < 10);
    }
}

#[test]
fn empty_child_keeps_both_smoothing_sweeps_with_reused_workspaces() {
    let triangles = Laplacian::from_edges(
        240,
        (0..80).flat_map(|c| {
            [
                (3 * c, 3 * c + 1, 1.0),
                (3 * c + 1, 3 * c + 2, 2.0),
                (3 * c, 3 * c + 2, 0.5),
            ]
        }),
    )
    .unwrap();
    for graph in [mixed(0, 120), triangles] {
        let pre = build(&graph, 4, true, true);
        assert_eq!(pre.hierarchy().levels()[1].graph().vertex_count(), 0);
        let inverse = pre.hierarchy().levels()[0].inverse_diagonal();
        let mut workspace = pre.workspace();
        let mut output = vec![f64::NAN; graph.vertex_count()];
        for seed in [1, 3, 1] {
            let rhs = graph.matvec(&vector(&graph, seed)).unwrap();
            let mut expected: Vec<_> = rhs.iter().zip(inverse).map(|(b, d)| b * d).collect();
            let ax = graph.matvec(&expected).unwrap();
            for (((x, b), d), ax) in expected.iter_mut().zip(&rhs).zip(inverse).zip(ax) {
                *x += d * (b - ax);
            }
            pre.apply_compatible_into(&rhs, &mut output, &mut workspace)
                .unwrap();
            assert_eq!(
                output.iter().map(|x| x.to_bits()).collect::<Vec<_>>(),
                expected.iter().map(|x| x.to_bits()).collect::<Vec<_>>()
            );
            #[cfg(feature = "parallel")]
            for threads in [1, 2, 4] {
                let executor = cmg::ParallelExecutor::new(cmg::ParallelOptions {
                    threads,
                    min_parallel_len: 1,
                    ..cmg::ParallelOptions::default()
                })
                .unwrap();
                let plan = cmg::ParallelCmgPlan::build(&pre, executor.team()).unwrap();
                pre.apply_compatible_into_with(
                    &rhs,
                    &mut output,
                    &mut workspace,
                    cmg::ApplyContext {
                        plan: Some(&plan),
                        team: Some(executor.team()),
                        cancel: None,
                    },
                )
                .unwrap();
                close(&output, &expected, 1e-12);
            }
        }
    }
}

#[test]
fn partial_transfer_is_transposed_and_galerkin_and_dimension_checked() {
    let graph = mixed(80, 12);
    let pre = build(&graph, 4, true, false);
    let levels = pre.hierarchy().levels();
    let t = levels[0].pruned_transfer().unwrap();
    let fine = vector(&graph, 2);
    let coarse: Vec<_> = (0..t.coarse_dimension()).map(|i| i as f64 / 7.0).collect();
    let mut r = vec![0.0; coarse.len()];
    t.restrict_into(&fine, &mut r).unwrap();
    let mut p = vec![0.0; fine.len()];
    t.prolong_add_into(&coarse, &mut p).unwrap();
    assert!((dot(&fine, &p) - dot(&r, &coarse)).abs() < 1e-10);
    let ap = graph.matvec(&p).unwrap();
    t.restrict_into(&ap, &mut r).unwrap();
    close(&r, &levels[1].graph().matvec(&coarse).unwrap(), 1e-12);
    assert!(p[80..].iter().all(|&x| x == 0.0));
    assert!(t.restrict_into(&fine[..fine.len() - 1], &mut r).is_err());
    assert!(
        t.prolong_add_into(&coarse[..coarse.len() - 1], &mut p)
            .is_err()
    );
}

#[test]
fn pruned_cycle_is_linear_symmetric_positive_and_repeatable_on_the_range() {
    for graph in [
        mixed(128, 30),
        mixed(0, 40),
        Laplacian::from_edges(135, (0..127).map(|v| (v, v + 1, 1.0))).unwrap(),
    ] {
        let pre = build(&graph, 4, true, true);
        let u = vector(&graph, 1);
        let v = vector(&graph, 2);
        let combination: Vec<_> = u.iter().zip(&v).map(|(a, b)| 1.25 * a - 0.75 * b).collect();
        let mu = pre.apply(&u).unwrap();
        let mv = pre.apply(&v).unwrap();
        let expected: Vec<_> = mu
            .iter()
            .zip(&mv)
            .map(|(a, b)| 1.25 * a - 0.75 * b)
            .collect();
        close(&pre.apply(&combination).unwrap(), &expected, 1e-10);
        let left = dot(&u, &mv);
        let right = dot(&mu, &v);
        assert!((left - right).abs() < 1e-10 * 1.0_f64.max(left.abs()).max(right.abs()));
        assert!(dot(&u, &mu) > 0.0);
        let mut workspace = pre.workspace();
        let mut out = vec![f64::NAN; u.len()];
        pre.apply_into(&v, &mut out, &mut workspace).unwrap();
        pre.apply_into(&u, &mut out, &mut workspace).unwrap();
        assert_eq!(out, mu);
    }
}

#[test]
fn empty_and_all_retired_graphs_and_incompatible_isolates_are_handled() {
    for n in [0, 1, 1000] {
        let graph = Laplacian::from_edges(n, []).unwrap();
        let pre = build(&graph, 4, true, true);
        assert_eq!(pre.apply(&vec![0.0; n]).unwrap(), vec![0.0; n]);
    }
    let graph = mixed(0, 1000);
    let pre = build(&graph, 700, true, true);
    assert_eq!(pre.hierarchy().report().vertex_counts(), &[2000, 0]);
    let graph = Laplacian::from_edges(81, (0..79).map(|v| (v, v + 1, 1.0))).unwrap();
    let pre = build(&graph, 4, true, true);
    let mut rhs = vec![0.0; 81];
    rhs[80] = 1.0;
    assert!(matches!(
        pre.apply(&rhs),
        Err(CmgError::IncompatibleLaplacianRhs { .. })
    ));
}

#[test]
fn block_factors_match_grounded_reference_with_interleaved_components() {
    for seed in 0..12 {
        // Interleave original vertex indices so component order differs from
        // global degree ordering; include dense blocks, paths, and an isolate.
        let graph = Laplacian::from_edges(
            25,
            (0..3).flat_map(|c| {
                (0..7).flat_map(move |u| {
                    (u + 1..8)
                        .filter(move |&v| v == u + 1 || (u * 3 + v + seed + c) % 4 == 0)
                        .map(move |v| {
                            (
                                3 * u + c,
                                3 * v + c,
                                0.25 + ((u + v + seed) % 9) as f64 / 4.0,
                            )
                        })
                })
            }),
        )
        .unwrap();
        let direct = GroundedLdl::factor(&graph).unwrap();
        let blocks = GroundedLdl::factor_by_component(&graph).unwrap();
        assert_eq!(direct.anchors(), blocks.anchors());
        assert_eq!(direct.factor_nonzeros(), blocks.factor_nonzeros());
        let rhs = graph.matvec(&vector(&graph, seed)).unwrap();
        close(
            &blocks.solve(&rhs).unwrap(),
            &direct.solve(&rhs).unwrap(),
            1e-12,
        );
    }
}

#[test]
fn sparse_terminal_factors_match_dense_arithmetic_across_fill_and_scale() {
    for n in [5, 83, 173] {
        for density in [0, 5, 1] {
            for scale in [1e-150, 1.0, 1e150] {
                let mut edges = Vec::new();
                for u in 0..n {
                    for v in u + 1..n {
                        if v == u + 1 || (density > 0 && (u * 17 + v * 11) % density == 0) {
                            edges.push((
                                (u * 37 + 3) % n,
                                (v * 37 + 3) % n,
                                scale * 10.0_f64.powi(((u * 7 + v * 13) % 7) as i32 - 3),
                            ));
                        }
                    }
                }
                let graph = Laplacian::from_edges(n, edges).unwrap();
                let dense = GroundedLdl::factor(&graph).unwrap();
                let sparse = GroundedLdl::factor_by_component(&graph).unwrap();
                assert_eq!(dense, sparse, "n={n}, density={density}, scale={scale}");
                let rhs = graph.matvec(&vector(&graph, density)).unwrap();
                assert_eq!(
                    dense
                        .solve(&rhs)
                        .unwrap()
                        .into_iter()
                        .map(f64::to_bits)
                        .collect::<Vec<_>>(),
                    sparse
                        .solve(&rhs)
                        .unwrap()
                        .into_iter()
                        .map(f64::to_bits)
                        .collect::<Vec<_>>()
                );
            }
        }
    }
}

#[test]
fn sparse_terminal_rejects_a_pivot_lost_to_roundoff() {
    let graph = Laplacian::from_edges(3, [(0, 1, 1e100), (1, 2, 1.0)]).unwrap();
    let dense = GroundedLdl::factor(&graph).unwrap_err();
    let sparse = GroundedLdl::factor_by_component(&graph).unwrap_err();
    assert!(matches!(sparse, CmgError::NonPositivePivot { .. }));
    assert_eq!(dense, sparse);
}

#[test]
fn certified_solutions_match_known_component_centered_solutions() {
    for graph in [mixed(128, 40), mixed(0, 349), mixed(0, 350)] {
        let known = vector(&graph, 3);
        let rhs = graph.matvec(&known).unwrap();
        for (prune, blocks) in [(true, false), (false, true), (true, true)] {
            let pre = build(&graph, 8, prune, blocks);
            let result = solve_pcg(
                &graph,
                &pre,
                &rhs,
                PcgOptions {
                    relative_tolerance: 1e-12,
                    ..PcgOptions::default()
                },
            )
            .unwrap();
            close(result.solution(), &known, 1e-7);
            let ax = graph.matvec(result.solution()).unwrap();
            let residual: Vec<_> = rhs.iter().zip(ax).map(|(b, ax)| b - ax).collect();
            assert!(dot(&residual, &residual).sqrt() <= result.tolerance());
        }
    }
}

#[test]
fn heterogeneous_interleaved_components_preserve_range_and_block_separation() {
    let sizes = [1usize, 2, 3, 5, 9, 17, 33];
    let n: usize = sizes.iter().sum();
    let mut offset = 0;
    let mut edges = Vec::new();
    for (c, size) in sizes.into_iter().enumerate() {
        for u in 0..size {
            for v in u + 1..size {
                if v == u + 1 || (u * 7 + v + c) % 7 == 0 {
                    let w = 10.0_f64.powi(((u + v + c) % 7) as i32 - 3);
                    edges.push(((offset + u) * 17 % n, (offset + v) * 17 % n, w));
                }
            }
        }
        offset += size;
    }
    let graph = Laplacian::from_edges(n, edges).unwrap();
    let components = Components::from_laplacian(&graph);
    let pre = build(&graph, 4, true, true);
    for c in 0..components.count() {
        let mut rhs = graph.matvec(&vector(&graph, c + 1)).unwrap();
        for (v, b) in rhs.iter_mut().enumerate() {
            if components.labels()[v] != c {
                *b = 0.0;
            }
        }
        let action = pre.apply(&rhs).unwrap();
        for (v, &x) in action.iter().enumerate() {
            if components.labels()[v] != c {
                assert_eq!(x, 0.0);
            }
        }
        let result = solve_pcg(&graph, &pre, &rhs, PcgOptions::default()).unwrap();
        assert!(result.residual_norm() <= result.tolerance());
    }
}

#[test]
fn prepared_numeric_generations_keep_pruning_fixed_and_certify_current_operator() {
    use cmg::{PcgWorkspace, PreparedLaplacianTopology};
    let original = mixed(80, 30);
    let topology = PreparedLaplacianTopology::prepare(
        original.vertex_count(),
        original.edges().iter().map(|e| (e.u(), e.v())),
    )
    .unwrap();
    let weights: Vec<_> = original.edges().iter().map(|e| e.weight()).collect();
    let first = topology.assemble(&weights).unwrap();
    for preserve_unpruned_stopping in [false, true] {
        let pre = CmgPreconditioner::build_with(
            &first,
            cmg::BuildOptions {
                hierarchy: CmgOptions {
                    direct_threshold: 4,
                    ..CmgOptions::default()
                },
                components: ComponentBuildOptions {
                    prune_coarse_isolates: true,
                    factor_terminal_components: true,
                    preserve_unpruned_stopping,
                },
            },
            cmg::BuildContext::default(),
        )
        .unwrap();
        let hierarchy = pre.hierarchy().clone();
        let mut ws = PcgWorkspace::new(&pre);
        let known = vector(&first, 4);
        let mut out = vec![0.0; known.len()];
        for scale in [0.1, 1.0, 10.0] {
            let current = topology
                .assemble(
                    &weights
                        .iter()
                        .enumerate()
                        .map(|(i, w)| w * scale * (0.9 + 0.02 * (i % 11) as f64))
                        .collect::<Vec<_>>(),
                )
                .unwrap();
            let rhs = current.matvec(&known).unwrap();
            let result = cmg::solve_pcg_into(
                &current,
                &pre,
                &rhs,
                None,
                &mut out,
                PcgOptions {
                    relative_tolerance: 1e-12,
                    ..PcgOptions::default()
                },
                &mut ws,
                cmg::SolveContext {
                    retained_preconditioner: true,
                    ..cmg::SolveContext::default()
                },
            )
            .unwrap();
            close(&out, &known, 1e-7);
            assert_eq!(pre.hierarchy(), &hierarchy);
            let ax = current.matvec(&out).unwrap();
            let r: Vec<_> = rhs.iter().zip(ax).map(|(b, ax)| b - ax).collect();
            assert!(dot(&r, &r).sqrt() <= result.tolerance());
        }
        let unrelated = PreparedLaplacianTopology::prepare(
            first.vertex_count(),
            first.edges().iter().map(|e| (e.u(), e.v())),
        )
        .unwrap()
        .assemble(&weights)
        .unwrap();
        let rhs = unrelated.matvec(&known).unwrap();
        assert!(
            cmg::solve_pcg_into(
                &unrelated,
                &pre,
                &rhs,
                None,
                &mut out,
                PcgOptions::default(),
                &mut ws,
                cmg::SolveContext {
                    retained_preconditioner: true,
                    ..cmg::SolveContext::default()
                }
            )
            .is_err()
        );
    }
}

#[cfg(feature = "parallel")]
#[test]
fn existing_parallel_plans_and_workspace_budgets_accept_compact_levels() {
    use cmg::{ParallelCmgPlan, ParallelExecutor, ParallelOptions, PcgWorkspace, solve_pcg_batch};
    let graph = mixed(128, 40);
    for preserve_unpruned_stopping in [false, true] {
        let pre = CmgPreconditioner::build_with(
            &graph,
            cmg::BuildOptions {
                hierarchy: CmgOptions {
                    direct_threshold: 4,
                    ..CmgOptions::default()
                },
                components: ComponentBuildOptions {
                    prune_coarse_isolates: true,
                    factor_terminal_components: true,
                    preserve_unpruned_stopping,
                },
            },
            cmg::BuildContext::default(),
        )
        .unwrap();
        let rhs = graph.matvec(&vector(&graph, 1)).unwrap();
        for threads in [1, 2, 4] {
            let bytes = PcgWorkspace::new(&pre).byte_len();
            let executor = ParallelExecutor::new(ParallelOptions {
                threads,
                min_parallel_len: 1,
                reduction_chunk_size: 16,
                workspace_memory_budget_bytes: Some(bytes),
            })
            .unwrap();
            let plan = ParallelCmgPlan::build(&pre, executor.team()).unwrap();
            let mut ws = PcgWorkspace::new(&pre);
            let a = cmg::solve_pcg_with(
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
            let b = cmg::solve_pcg_with(
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
            assert_eq!(a, b);
            assert!(a.residual_norm() <= a.tolerance());
            let batch = solve_pcg_batch(
                &graph,
                &pre,
                &[rhs.clone(), rhs.clone()],
                PcgOptions::default(),
            )
            .unwrap();
            assert_eq!(batch[0], batch[1]);
            assert_eq!(executor.team().batch_concurrency(bytes, 2).unwrap(), 1);
        }
    }
}
