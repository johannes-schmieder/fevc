#![cfg(all(feature = "parallel", feature = "experimental-components"))]
use cmg::*;

const COMPONENT: ComponentBuildOptions = ComponentBuildOptions {
    prune_coarse_isolates: true,
    preserve_unpruned_stopping: true,
    factor_terminal_components: true,
};

#[test]
fn executor_component_setup_matches_serial_including_parallel_sort_boundary() {
    let dense = Laplacian::from_edges(
        612,
        (0..256)
            .flat_map(|u| (256..512).map(move |v| (u, v, 1.0)))
            .chain((512..562).step_by(2).map(|u| (u, u + 1, 0.5))),
    )
    .unwrap();
    assert!(dense.edge_count() >= 65_536);
    for threads in [1, 4] {
        let executor = ParallelExecutor::new(ParallelOptions {
            threads,
            min_parallel_len: 1,
            ..Default::default()
        })
        .unwrap();
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
                        ..Default::default()
                    };
                    let serial = CmgPreconditioner::build_with(
                        &dense,
                        cmg::BuildOptions {
                            hierarchy: options,
                            components: experiment,
                        },
                        cmg::BuildContext::default(),
                    )
                    .unwrap();
                    let parallel = CmgPreconditioner::build_with(
                        &dense,
                        cmg::BuildOptions {
                            hierarchy: options,
                            components: experiment,
                        },
                        cmg::BuildContext {
                            team: Some(executor.team()),
                            ..cmg::BuildContext::default()
                        },
                    )
                    .unwrap();
                    assert_eq!(serial, parallel);
                    let rhs = dense
                        .matvec(&(0..612).map(|v| (v % 13) as f64).collect::<Vec<_>>())
                        .unwrap();
                    assert_eq!(serial.apply(&rhs).unwrap(), parallel.apply(&rhs).unwrap());
                    // This dense finest graph selects parallel CSR fill with
                    // four workers. The independent TLS allocation fixture
                    // intentionally covers serial CSR only; do not confuse it
                    // with a cross-thread/runtime allocation measurement.
                    let bound =
                        ParallelCmgPlan::construction_allocation_bound(&parallel, executor.team())
                            .unwrap();
                    assert!(
                        ParallelCmgPlan::build_with_budget(&parallel, executor.team(), bound - 1)
                            .is_err()
                    );
                    let admitted =
                        ParallelCmgPlan::build_with_budget(&parallel, executor.team(), bound)
                            .unwrap();
                    let reference = ParallelCmgPlan::build(&parallel, executor.team()).unwrap();
                    if threads == 4 {
                        assert!(admitted.operator_count() > 0);
                    }
                    let mut workspace = parallel.try_workspace().unwrap();
                    let mut expected = vec![0.0; 612];
                    let mut actual = vec![0.0; 612];
                    parallel
                        .apply_compatible_into_with(
                            &rhs,
                            &mut expected,
                            &mut workspace,
                            cmg::ApplyContext {
                                plan: Some(&reference),
                                team: Some(executor.team()),
                                cancel: None,
                            },
                        )
                        .unwrap();
                    parallel
                        .apply_compatible_into_with(
                            &rhs,
                            &mut actual,
                            &mut workspace,
                            cmg::ApplyContext {
                                plan: Some(&admitted),
                                team: Some(executor.team()),
                                cancel: None,
                            },
                        )
                        .unwrap();
                    assert_eq!(actual, expected);
                }
            }
        }
        assert!(
            CmgStorageReport::measure(&[&executor])
                .unwrap()
                .runtime_heap_unbounded()
        );
    }
    assert!(
        ParallelExecutor::new_with_memory_budget(ParallelOptions::default(), usize::MAX).is_err()
    );
}

#[test]
fn prepared_component_banks_cover_serial_planned_and_concurrent_rhs() {
    // A material path, many pairs, isolates, a cycle and a tiny weighted bridge.
    let mut endpoints: Vec<_> = (1..64).map(|i| (i - 1, i)).collect();
    endpoints.push((0, 63));
    endpoints.extend((64..104).step_by(2).map(|i| (i, i + 1)));
    let topology =
        PreparedLaplacianTopology::prepare_with_budget(112, &endpoints, usize::MAX).unwrap();
    let mut weights = vec![1.0; endpoints.len()];
    weights[31] = 1e-9;
    // The alternate route around the cycle keeps this case well-conditioned;
    // A genuine bridge with an analytic resistance oracle is tested separately.
    let first = topology.assemble(&weights).unwrap();
    let mut target: Vec<_> = (0..112).map(|i| ((i * 7) % 19) as f64).collect();
    topology.components().center_in_place(&mut target).unwrap();
    for threads in [1, 4] {
        let executor = ParallelExecutor::new(ParallelOptions {
            threads,
            min_parallel_len: 1,
            reduction_chunk_size: 16,
            ..Default::default()
        })
        .unwrap();
        let pre = CmgPreconditioner::build_with(
            &first,
            cmg::BuildOptions {
                hierarchy: CmgOptions {
                    direct_threshold: 8,
                    ..Default::default()
                },
                components: COMPONENT,
            },
            cmg::BuildContext {
                team: Some(executor.team()),
                ..cmg::BuildContext::default()
            },
        )
        .unwrap();
        let plan = ParallelCmgPlan::build(&pre, executor.team()).unwrap();
        let bank_bytes = PcgBatchWorkspace::bank_allocation_bytes(&pre, threads).unwrap();
        let mut bank = PcgBatchWorkspace::new_bank_with_budget(&pre, threads, bank_bytes).unwrap();
        let before = CmgStorageReport::measure(&[&bank]).unwrap();
        for scale in [0.5, 1.0, 2.0] {
            let graph = topology
                .assemble(&weights.iter().map(|w| scale * w).collect::<Vec<_>>())
                .unwrap();
            let rhs = graph.matvec(&target).unwrap();
            let packed: Vec<_> = (0..4).flat_map(|_| rhs.iter().copied()).collect();
            let mut output = vec![f64::NAN; packed.len()];
            let mut diagnostic = [PcgDiagnostics::default(); 4];
            for planned in [false, true] {
                let rhs_view = PcgBatchRef::contiguous(&packed, 4, 112).unwrap();
                let out_view = PcgBatchMut::contiguous(&mut output, 4, 112).unwrap();
                let options = PcgOptions {
                    relative_tolerance: 1e-12,
                    ..Default::default()
                };
                if planned {
                    cmg::solve_pcg_batch_into(
                        &graph,
                        &pre,
                        rhs_view,
                        None,
                        out_view,
                        &mut diagnostic,
                        options,
                        core::slice::from_mut(&mut bank[0]),
                        cmg::SolveContext {
                            plan: Some(&plan),
                            team: Some(executor.team()),
                            retained_preconditioner: true,
                            ..cmg::SolveContext::default()
                        },
                    )
                    .unwrap();
                } else {
                    cmg::solve_pcg_batch_into(
                        &graph,
                        &pre,
                        rhs_view,
                        None,
                        out_view,
                        &mut diagnostic,
                        options,
                        &mut bank,
                        cmg::SolveContext {
                            team: Some(executor.team()),
                            concurrent_right_hand_sides: true,
                            retained_preconditioner: true,
                            ..cmg::SolveContext::default()
                        },
                    )
                    .unwrap();
                }
                for (x, d) in output.chunks(112).zip(&diagnostic) {
                    assert!(x.iter().zip(&target).all(|(a, b)| (a - b).abs() < 1e-6));
                    let residual = graph
                        .matvec(x)
                        .unwrap()
                        .iter()
                        .zip(&rhs)
                        .map(|(ax, b)| (b - ax).powi(2))
                        .sum::<f64>()
                        .sqrt();
                    assert!(residual <= d.tolerance());
                }
                assert_eq!(CmgStorageReport::measure(&[&bank]).unwrap(), before);
            }
        }
    }
}

#[test]
fn dyadic_weak_bridge_matches_independent_resistance_oracle() {
    // Frozen before execution: endpoint unit injection on a 64-vertex path.
    // Unit resistances except a 2^24 bridge. Path current is exactly one, so
    // adjacent voltage differences equal resistance. All reference arithmetic
    // below is exactly representable; no production solver builds the oracle.
    const N: usize = 64;
    const BRIDGE_RESISTANCE: f64 = 16_777_216.0;
    const FORWARD_RELATIVE_LIMIT: f64 = 1e-6;
    const RESIDUAL_LIMIT: f64 = 1e-8 * std::f64::consts::SQRT_2;
    let graph = Laplacian::from_edges(
        N,
        (1..N).map(|v| {
            (
                v - 1,
                v,
                if v == 32 {
                    1.0 / BRIDGE_RESISTANCE
                } else {
                    1.0
                },
            )
        }),
    )
    .unwrap();
    let mut reference = [0.0; N];
    for v in (1..N).rev() {
        reference[v - 1] = reference[v] + if v == 32 { BRIDGE_RESISTANCE } else { 1.0 };
    }
    let mean = reference.iter().sum::<f64>() / N as f64;
    for value in &mut reference {
        *value -= mean;
    }
    let scale = reference.iter().map(|x| x.abs()).fold(0.0, f64::max);
    let mut rhs = [0.0; N];
    rhs[0] = 1.0;
    rhs[N - 1] = -1.0;
    for threads in [1, 4] {
        let executor = ParallelExecutor::new(ParallelOptions {
            threads,
            min_parallel_len: 1,
            ..Default::default()
        })
        .unwrap();
        for direct_threshold in [8, N + 1] {
            for component in [false, true] {
                let options = CmgOptions {
                    direct_threshold,
                    ..Default::default()
                };
                let pre = if component {
                    CmgPreconditioner::build_with(
                        &graph,
                        cmg::BuildOptions {
                            hierarchy: options,
                            components: COMPONENT,
                        },
                        cmg::BuildContext {
                            team: Some(executor.team()),
                            ..cmg::BuildContext::default()
                        },
                    )
                } else {
                    CmgPreconditioner::build_with(
                        &graph,
                        cmg::BuildOptions::from(options),
                        cmg::BuildContext {
                            team: Some(executor.team()),
                            ..cmg::BuildContext::default()
                        },
                    )
                }
                .unwrap();
                let plan =
                    ParallelCmgPlan::build_with_budget(&pre, executor.team(), usize::MAX).unwrap();
                let mut workspace = PcgWorkspace::try_new(&pre).unwrap();
                let mut solution = [0.0; N];
                let diagnostics = cmg::solve_pcg_into(
                    &graph,
                    &pre,
                    &rhs,
                    None,
                    &mut solution,
                    PcgOptions {
                        // This oracle requires an RHS-scaled residual, not the
                        // default backward error (which grows with ||x||).
                        // Keep both independently frozen accuracy gates below.
                        relative_tolerance: 0.0,
                        absolute_tolerance: RESIDUAL_LIMIT,
                        ..Default::default()
                    },
                    &mut workspace,
                    cmg::SolveContext {
                        plan: Some(&plan),
                        team: Some(executor.team()),
                        ..cmg::SolveContext::default()
                    },
                )
                .unwrap();
                assert!(diagnostics.residual_norm() <= diagnostics.tolerance());
                let forward = solution
                    .iter()
                    .zip(reference)
                    .map(|(x, r)| (x - r).abs())
                    .fold(0.0, f64::max)
                    / scale;
                assert!(
                    forward <= FORWARD_RELATIVE_LIMIT,
                    "forward error {forward:e}"
                );
                // Independent path-current divergence; not graph.matvec or the
                // solver's reported residual. Keep the original certificate too.
                let mut residual = rhs;
                for v in 1..N {
                    let resistance = if v == 32 { BRIDGE_RESISTANCE } else { 1.0 };
                    let current = (solution[v - 1] - solution[v]) / resistance;
                    residual[v - 1] -= current;
                    residual[v] += current;
                }
                let residual = residual.iter().map(|r| r * r).sum::<f64>().sqrt();
                assert!(
                    residual <= RESIDUAL_LIMIT,
                    "independent residual {residual:e}"
                );
                println!(
                    "WEAK_BRIDGE_ORACLE_SUCCESS threads={threads} direct={direct_threshold} component={component} forward={forward:e} residual={residual:e}"
                );
            }
        }
    }
}
