use cmg::{
    CmgError, CmgOptions, CmgPreconditioner, CmgProblemSize, Laplacian, ParallelOptions,
    PcgBatchMut, PcgBatchRef, PcgBatchWorkspace, PcgDiagnostics, PcgOptions, PcgWorkspace,
    PreparedLaplacianTopology, RepeatedPcgMemoryEstimate, RepeatedPcgMemoryReport, solve_pcg,
};

fn strict_options() -> PcgOptions {
    PcgOptions {
        relative_tolerance: 1.0e-11,
        absolute_tolerance: 1.0e-13,
        max_iterations: 500,
        residual_recompute_interval: 3,
        ..PcgOptions::default()
    }
}

fn norm(values: &[f64]) -> f64 {
    values.iter().map(|value| value * value).sum::<f64>().sqrt()
}

fn fresh_residual(graph: &Laplacian, rhs: &[f64], solution: &[f64]) -> f64 {
    let product = graph.matvec(solution).unwrap();
    norm(
        &rhs.iter()
            .zip(product)
            .map(|(rhs, value)| rhs - value)
            .collect::<Vec<_>>(),
    )
}

fn assert_close(left: &[f64], right: &[f64], tolerance: f64) {
    assert_eq!(left.len(), right.len());
    for (&left, &right) in left.iter().zip(right) {
        let scale = 1.0_f64.max(left.abs()).max(right.abs());
        assert!((left - right).abs() <= tolerance * scale);
    }
}

fn dense_grounded_oracle(graph: &Laplacian, rhs: &[f64]) -> Vec<f64> {
    let components = cmg::Components::from_laplacian(graph);
    let anchors: Vec<_> = components
        .sizes()
        .iter()
        .enumerate()
        .map(|(component, _)| {
            components
                .labels()
                .iter()
                .rposition(|&label| label == component)
                .unwrap()
        })
        .collect();
    let variables: Vec<_> = (0..graph.vertex_count())
        .filter(|vertex| !anchors.contains(vertex))
        .collect();
    let dense = graph.to_dense();
    let mut matrix: Vec<Vec<f64>> = variables
        .iter()
        .map(|&row| variables.iter().map(|&column| dense[row][column]).collect())
        .collect();
    let mut reduced_rhs: Vec<f64> = variables.iter().map(|&vertex| rhs[vertex]).collect();
    for pivot in 0..matrix.len() {
        let pivot_row = (pivot..matrix.len())
            .max_by(|left, right| {
                matrix[*left][pivot]
                    .abs()
                    .total_cmp(&matrix[*right][pivot].abs())
            })
            .unwrap();
        matrix.swap(pivot, pivot_row);
        reduced_rhs.swap(pivot, pivot_row);
        let diagonal = matrix[pivot][pivot];
        assert!(diagonal.is_finite() && diagonal != 0.0);
        let pivot_tail = matrix[pivot][(pivot + 1)..].to_vec();
        for row in (pivot + 1)..matrix.len() {
            let multiplier = matrix[row][pivot] / diagonal;
            matrix[row][pivot] = 0.0;
            for (value, pivot_value) in matrix[row][(pivot + 1)..].iter_mut().zip(&pivot_tail) {
                *value -= multiplier * pivot_value;
            }
            reduced_rhs[row] -= multiplier * reduced_rhs[pivot];
        }
    }
    let mut reduced_solution = vec![0.0; variables.len()];
    for row in (0..variables.len()).rev() {
        let tail: f64 = matrix[row][(row + 1)..]
            .iter()
            .zip(&reduced_solution[(row + 1)..])
            .map(|(coefficient, value)| coefficient * value)
            .sum();
        reduced_solution[row] = (reduced_rhs[row] - tail) / matrix[row][row];
    }
    let mut solution = vec![0.0; graph.vertex_count()];
    for (&vertex, &value) in variables.iter().zip(&reduced_solution) {
        solution[vertex] = value;
    }
    components.center_in_place(&mut solution).unwrap();
    solution
}

#[test]
fn prepared_assembly_matches_legacy_duplicates_permutations_and_components() {
    let endpoints = [(2, 0), (0, 2), (1, 2), (4, 3), (3, 4), (2, 1)];
    let weights = [1.0e-12, 4.0, 0.25, 7.0, 0.5, 2.0];
    let topology = PreparedLaplacianTopology::prepare(6, endpoints).unwrap();
    let mut workspace = topology.workspace().unwrap();
    let prepared = topology
        .assemble_with_workspace(&weights, &mut workspace)
        .unwrap();
    let legacy = Laplacian::from_edges(
        6,
        endpoints
            .into_iter()
            .zip(weights)
            .map(|((u, v), weight)| (u, v, weight)),
    )
    .unwrap();
    assert_eq!(prepared, legacy);
    assert_eq!(topology.input_edge_count(), 6);
    assert_eq!(topology.canonical_edge_count(), 3);
    assert_eq!(topology.components().sizes(), &[3, 2, 1]);
    assert_eq!(topology.input_edge_to_canonical_edge(), &[0, 0, 1, 2, 2, 1]);

    let permutation = [5, 3, 0, 4, 2, 1];
    let permuted_topology =
        PreparedLaplacianTopology::prepare(6, permutation.iter().map(|&index| endpoints[index]))
            .unwrap();
    let permuted_weights: Vec<_> = permutation.iter().map(|&index| weights[index]).collect();
    assert_eq!(
        permuted_topology.assemble(&permuted_weights).unwrap(),
        legacy
    );
    assert_eq!(
        workspace.byte_len(),
        topology.workspace().unwrap().byte_len()
    );
}

#[test]
fn prepared_assembly_rejects_every_invalid_numeric_frame_before_publication() {
    let topology = PreparedLaplacianTopology::prepare(4, [(0, 1), (1, 2), (2, 3)]).unwrap();
    assert!(matches!(
        topology.assemble(&[1.0, 2.0]),
        Err(CmgError::DimensionMismatch { .. })
    ));
    for invalid in [0.0, -1.0, f64::INFINITY, f64::NEG_INFINITY, f64::NAN] {
        assert!(matches!(
            topology.assemble(&[1.0, invalid, 2.0]),
            Err(CmgError::InvalidEdgeWeight { .. })
        ));
    }
    assert!(matches!(
        PreparedLaplacianTopology::prepare(3, [(0, 0)]),
        Err(CmgError::SelfLoop { .. })
    ));
    assert!(matches!(
        PreparedLaplacianTopology::prepare(3, [(0, 3)]),
        Err(CmgError::VertexOutOfBounds { .. })
    ));
}

#[test]
fn caller_buffer_zero_start_is_bitwise_identical_and_warm_start_is_projected() {
    let graph =
        Laplacian::from_edges(7, [(0, 1, 2.0), (1, 2, 0.5), (0, 2, 1.5), (3, 4, 3.0)]).unwrap();
    let target = [2.0, -4.0, 3.0, 8.0, -1.0, 0.0, 0.0];
    let rhs = graph.matvec(&target).unwrap();
    let preconditioner = CmgPreconditioner::build(&graph, CmgOptions::default()).unwrap();
    let owned = solve_pcg(&graph, &preconditioner, &rhs, strict_options()).unwrap();
    let mut workspace = PcgWorkspace::new(&preconditioner);
    let mut solution = vec![f64::NAN; graph.vertex_count()];
    let diagnostics = cmg::solve_pcg_into(
        &graph,
        &preconditioner,
        &rhs,
        None,
        &mut solution,
        strict_options(),
        &mut workspace,
        cmg::SolveContext::default(),
    )
    .unwrap();
    assert_eq!(diagnostics, owned.diagnostics());
    for (&left, &right) in solution.iter().zip(owned.solution()) {
        assert_eq!(left.to_bits(), right.to_bits());
    }
    assert_close(&solution, &dense_grounded_oracle(&graph, &rhs), 1.0e-10);
    let bytes = workspace.byte_len();
    let warm = target.map(|value| value + 17.0);
    let warm_diagnostics = cmg::solve_pcg_into(
        &graph,
        &preconditioner,
        &rhs,
        Some(&warm),
        &mut solution,
        strict_options(),
        &mut workspace,
        cmg::SolveContext::default(),
    )
    .unwrap();
    assert_eq!(workspace.byte_len(), bytes);
    assert_eq!(warm_diagnostics.iterations(), 0);
    assert!(fresh_residual(&graph, &rhs, &solution) <= warm_diagnostics.tolerance());
    assert_eq!(solution[5], 0.0);
    assert_eq!(solution[6], 0.0);

    let mut nonfinite = warm;
    nonfinite[2] = f64::NAN;
    assert!(matches!(
        cmg::solve_pcg_into(
            &graph,
            &preconditioner,
            &rhs,
            Some(&nonfinite),
            &mut solution,
            strict_options(),
            &mut workspace,
            cmg::SolveContext::default()
        ),
        Err(CmgError::NonFiniteMatrixValue { .. })
    ));
}

#[test]
fn strided_batch_matches_owned_results_and_dense_oracle() {
    let graph =
        Laplacian::from_edges(6, [(0, 1, 2.0), (1, 2, 0.75), (0, 2, 1.25), (3, 4, 3.0)]).unwrap();
    let targets = [
        [2.0, -1.0, 4.0, 7.0, -2.0, 0.0],
        [-3.0, 5.0, 1.0, -4.0, 6.0, 0.0],
    ];
    let rhs: Vec<Vec<f64>> = targets
        .iter()
        .map(|target| graph.matvec(target).unwrap())
        .collect();
    let mut vertex_major = vec![91.0; graph.vertex_count() * 3];
    for vertex in 0..graph.vertex_count() {
        for rhs_index in 0..2 {
            vertex_major[vertex * 3 + rhs_index] = rhs[rhs_index][vertex];
        }
    }
    let rhs_view = PcgBatchRef::strided(&vertex_major, 2, graph.vertex_count(), 1, 3).unwrap();
    let mut output = vec![-77.0; graph.vertex_count() * 4];
    let output_view = PcgBatchMut::strided(&mut output, 2, graph.vertex_count(), 1, 4).unwrap();
    let mut diagnostics = [PcgDiagnostics::default(); 2];
    let preconditioner = CmgPreconditioner::build(&graph, CmgOptions::default()).unwrap();
    let mut workspace = PcgBatchWorkspace::new(&preconditioner).unwrap();
    let bytes = workspace.byte_len();
    cmg::solve_pcg_batch_into(
        &graph,
        &preconditioner,
        rhs_view,
        None,
        output_view,
        &mut diagnostics,
        strict_options(),
        core::slice::from_mut(&mut workspace),
        cmg::SolveContext::default(),
    )
    .unwrap();
    assert_eq!(workspace.byte_len(), bytes);
    for rhs_index in 0..2 {
        let solution: Vec<_> = (0..graph.vertex_count())
            .map(|vertex| output[vertex * 4 + rhs_index])
            .collect();
        let oracle = dense_grounded_oracle(&graph, &rhs[rhs_index]);
        assert_close(&solution, &oracle, 1.0e-10);
        let owned = solve_pcg(&graph, &preconditioner, &rhs[rhs_index], strict_options()).unwrap();
        assert_close(&solution, owned.solution(), 1.0e-12);
        assert!(
            fresh_residual(&graph, &rhs[rhs_index], &solution)
                <= diagnostics[rhs_index].tolerance()
        );
    }

    let mut guesses = vec![0.0; graph.vertex_count() * 3];
    for vertex in 0..graph.vertex_count() {
        for rhs_index in 0..2 {
            guesses[vertex * 3 + rhs_index] = targets[rhs_index][vertex] + 11.0;
        }
    }
    cmg::solve_pcg_batch_into(
        &graph,
        &preconditioner,
        rhs_view,
        Some(PcgBatchRef::strided(&guesses, 2, graph.vertex_count(), 1, 3).unwrap()),
        PcgBatchMut::strided(&mut output, 2, graph.vertex_count(), 1, 4).unwrap(),
        &mut diagnostics,
        strict_options(),
        core::slice::from_mut(&mut workspace),
        cmg::SolveContext::default(),
    )
    .unwrap();
    assert!(diagnostics.iter().all(|value| value.iterations() == 0));

    let mut overlapping = vec![0.0; 16];
    assert!(PcgBatchMut::strided(&mut overlapping, 3, 3, 1, 1).is_err());
    assert!(PcgBatchRef::strided(&vertex_major, 2, 6, usize::MAX, 3).is_err());
}

#[test]
fn empty_batches_are_supported_and_nonfinite_rhs_is_rejected() {
    let mut empty_zero_dimension = [];
    assert!(PcgBatchRef::contiguous(&[], 3, 0).is_ok());
    assert!(PcgBatchMut::contiguous(&mut empty_zero_dimension, 3, 0).is_ok());

    let graph = Laplacian::from_edges(3, [(0, 1, 1.0), (1, 2, 1.0)]).unwrap();
    let preconditioner = CmgPreconditioner::build(&graph, CmgOptions::default()).unwrap();
    let mut workspace = PcgBatchWorkspace::new(&preconditioner).unwrap();
    let mut empty_output = [];
    cmg::solve_pcg_batch_into(
        &graph,
        &preconditioner,
        PcgBatchRef::contiguous(&[], 0, 3).unwrap(),
        None,
        PcgBatchMut::contiguous(&mut empty_output, 0, 3).unwrap(),
        &mut [],
        strict_options(),
        core::slice::from_mut(&mut workspace),
        cmg::SolveContext::default(),
    )
    .unwrap();

    let mut output = [0.0; 3];
    let mut diagnostics = [PcgDiagnostics::default()];
    assert!(matches!(
        cmg::solve_pcg_batch_into(
            &graph,
            &preconditioner,
            PcgBatchRef::contiguous(&[1.0, f64::NAN, -1.0], 1, 3).unwrap(),
            None,
            PcgBatchMut::contiguous(&mut output, 1, 3).unwrap(),
            &mut diagnostics,
            strict_options(),
            core::slice::from_mut(&mut workspace),
            cmg::SolveContext::default()
        ),
        Err(CmgError::NonFiniteMatrixValue { .. })
    ));
}

#[test]
fn retained_preconditioner_sequences_are_certified_against_each_current_graph() {
    let endpoints = [(0, 1), (1, 2), (2, 3), (0, 2), (4, 5)];
    let topology = PreparedLaplacianTopology::prepare(7, endpoints).unwrap();
    let frames = [
        [1.0, 1.0, 1.0, 0.5, 2.0],
        [1.08, 0.92, 1.04, 0.48, 2.1],
        [1.0e-3, 7.0e-3, 3.0e-3, 5.0e-3, 2.0e-3],
        [1.0e3, 1.0e-4, 5.0e1, 2.0e-2, 7.0e2],
        [2.0e-2, 4.0e2, 9.0e-5, 3.0e1, 8.0e-3],
    ];
    let first = topology.assemble(&frames[0]).unwrap();
    let cloned_topology = topology.clone();
    let retained = CmgPreconditioner::build(&first, CmgOptions::default()).unwrap();
    let mut retained_workspace = PcgWorkspace::new(&retained);
    let target = [3.0, -2.0, 7.0, -4.0, 1.5, -2.5, 0.0];
    let mut prior_solution = vec![0.0; 7];
    for (frame_index, weights) in frames.iter().enumerate() {
        let current = cloned_topology.assemble(weights).unwrap();
        let rhs = current.matvec(&target).unwrap();
        let fresh = CmgPreconditioner::build(&current, CmgOptions::default()).unwrap();
        let fresh_result = solve_pcg(&current, &fresh, &rhs, strict_options()).unwrap();
        let mut solution = vec![0.0; 7];
        let guess = (frame_index > 0).then_some(prior_solution.as_slice());
        let result = cmg::solve_pcg_into(
            &current,
            &retained,
            &rhs,
            guess,
            &mut solution,
            strict_options(),
            &mut retained_workspace,
            cmg::SolveContext {
                retained_preconditioner: true,
                ..cmg::SolveContext::default()
            },
        );
        if let Ok(diagnostics) = result {
            assert!(fresh_residual(&current, &rhs, &solution) <= diagnostics.tolerance());
            assert_close(&solution, fresh_result.solution(), 2.0e-8);
            assert_close(&solution, &dense_grounded_oracle(&current, &rhs), 2.0e-8);
            let mut batch_solution = vec![0.0; 7];
            let mut batch_diagnostics = [PcgDiagnostics::default()];
            let mut batch_workspace = PcgBatchWorkspace::new(&retained).unwrap();
            cmg::solve_pcg_batch_into(
                &current,
                &retained,
                PcgBatchRef::contiguous(&rhs, 1, 7).unwrap(),
                guess.map(|values| PcgBatchRef::contiguous(values, 1, 7).unwrap()),
                PcgBatchMut::contiguous(&mut batch_solution, 1, 7).unwrap(),
                &mut batch_diagnostics,
                strict_options(),
                core::slice::from_mut(&mut batch_workspace),
                cmg::SolveContext {
                    retained_preconditioner: true,
                    ..cmg::SolveContext::default()
                },
            )
            .unwrap();
            assert_close(&batch_solution, fresh_result.solution(), 2.0e-8);
            prior_solution.copy_from_slice(&solution);
        }
    }

    let unrelated = PreparedLaplacianTopology::prepare(7, endpoints).unwrap();
    let unrelated_graph = unrelated.assemble(&frames[1]).unwrap();
    let rhs = unrelated_graph.matvec(&target).unwrap();
    assert!(matches!(
        cmg::solve_pcg_into(
            &unrelated_graph,
            &retained,
            &rhs,
            None,
            &mut prior_solution,
            strict_options(),
            &mut retained_workspace,
            cmg::SolveContext {
                retained_preconditioner: true,
                ..cmg::SolveContext::default()
            }
        ),
        Err(CmgError::InvalidHierarchy { .. })
    ));
}

#[test]
fn retained_solver_never_accepts_an_old_operator_only_initial_guess() {
    let topology = PreparedLaplacianTopology::prepare(4, [(0, 1), (1, 2), (2, 3)]).unwrap();
    let old = topology.assemble(&[1.0, 1.0, 1.0]).unwrap();
    let current = topology.assemble(&[100.0, 0.01, 17.0]).unwrap();
    let retained = CmgPreconditioner::build(&old, CmgOptions::default()).unwrap();
    let old_solution = [-2.0, 1.0, 4.0, -3.0];
    let rhs = old.matvec(&old_solution).unwrap();
    assert_eq!(fresh_residual(&old, &rhs, &old_solution), 0.0);
    assert!(fresh_residual(&current, &rhs, &old_solution) > 1.0);
    let mut workspace = PcgWorkspace::new(&retained);
    let mut solution = vec![0.0; 4];
    let result = cmg::solve_pcg_into(
        &current,
        &retained,
        &rhs,
        Some(&old_solution),
        &mut solution,
        PcgOptions {
            relative_tolerance: 1.0e-14,
            absolute_tolerance: 0.0,
            max_iterations: 1,
            residual_recompute_interval: 1,
            ..PcgOptions::default()
        },
        &mut workspace,
        cmg::SolveContext {
            retained_preconditioner: true,
            ..cmg::SolveContext::default()
        },
    );
    assert!(result.is_err());
}

#[test]
fn repeated_memory_report_counts_current_and_retained_numeric_states() {
    let topology = PreparedLaplacianTopology::prepare(5, [(0, 1), (1, 2), (2, 3)]).unwrap();
    let assembly_workspace = topology.workspace().unwrap();
    let old = topology.assemble(&[1.0, 2.0, 3.0]).unwrap();
    let current = topology.assemble(&[3.0, 2.0, 1.0]).unwrap();
    let retained = CmgPreconditioner::build(&old, CmgOptions::default()).unwrap();
    let workspaces = [PcgBatchWorkspace::new(&retained).unwrap()];
    assert_eq!(retained.try_workspace().unwrap().dimensions(), &[5]);
    assert_eq!(PcgWorkspace::try_new(&retained).unwrap().dimension(), 5);
    let report = RepeatedPcgMemoryReport::serial(
        &topology,
        &assembly_workspace,
        &current,
        &retained,
        &workspaces,
        11,
        true,
    )
    .unwrap();
    assert_eq!(
        report.current_numeric_graph_bytes(),
        current.retained_bytes()
    );
    assert!(report.prepared_topology_bytes() >= report.shared_component_metadata_bytes());
    assert!(report.retained_preconditioner_bytes() > 0);
    assert!(report.workspace_pool_bytes() >= workspaces[0].byte_len());
    assert!(report.total_solver_retained_bytes() > current.retained_bytes());
    assert!(report.caller_logical_bytes() > 0);

    let estimate = RepeatedPcgMemoryEstimate::conservative(
        CmgProblemSize {
            vertices: 5,
            input_edges: 3,
            canonical_edges: 3,
            right_hand_sides: 11,
        },
        CmgOptions::default(),
        ParallelOptions {
            threads: 8,
            ..ParallelOptions::default()
        },
        true,
        true,
    )
    .unwrap();
    assert_eq!(
        estimate.shared_component_metadata_bytes(),
        5 * 2 * core::mem::size_of::<usize>()
    );
    assert!(estimate.total_solver_retained_bytes() >= estimate.prepared_topology_bytes());
    assert!(estimate.build_peak_bytes() >= estimate.total_solver_retained_bytes());
}

#[cfg(feature = "profiling")]
#[test]
fn profiled_caller_batch_is_bitwise_identical_to_production() {
    let graph = Laplacian::from_edges(8, (0..7).map(|vertex| (vertex, vertex + 1, 1.0))).unwrap();
    let preconditioner = CmgPreconditioner::build(&graph, CmgOptions::default()).unwrap();
    let right_hand_sides: Vec<Vec<f64>> = (0..3)
        .map(|rhs| {
            let target: Vec<_> = (0..8)
                .map(|vertex| (vertex * (rhs + 2)) as f64 - 3.5)
                .collect();
            graph.matvec(&target).unwrap()
        })
        .collect();
    let packed: Vec<_> = right_hand_sides.iter().flatten().copied().collect();
    let mut production_output = vec![0.0; packed.len()];
    let mut profiled_output = vec![0.0; packed.len()];
    let mut production_diagnostics = vec![PcgDiagnostics::default(); 3];
    let mut profiled_diagnostics = vec![PcgDiagnostics::default(); 3];
    let mut production_workspace = PcgBatchWorkspace::new(&preconditioner).unwrap();
    let mut profiled_workspace = PcgBatchWorkspace::new(&preconditioner).unwrap();
    cmg::solve_pcg_batch_into(
        &graph,
        &preconditioner,
        PcgBatchRef::contiguous(&packed, 3, 8).unwrap(),
        None,
        PcgBatchMut::contiguous(&mut production_output, 3, 8).unwrap(),
        &mut production_diagnostics,
        strict_options(),
        core::slice::from_mut(&mut production_workspace),
        cmg::SolveContext::default(),
    )
    .unwrap();
    let profile = cmg::profile_pcg_batch_into(
        &graph,
        &preconditioner,
        PcgBatchRef::contiguous(&packed, 3, 8).unwrap(),
        None,
        PcgBatchMut::contiguous(&mut profiled_output, 3, 8).unwrap(),
        &mut profiled_diagnostics,
        strict_options(),
        &mut profiled_workspace,
        cmg::SolveContext::default(),
    )
    .unwrap();
    assert_eq!(production_diagnostics, profiled_diagnostics);
    for (&production, &profiled) in production_output.iter().zip(&profiled_output) {
        assert_eq!(production.to_bits(), profiled.to_bits());
    }
    assert!(profile.total_nanoseconds() >= profile.solve_nanoseconds());
    assert_eq!(profile.result_construction_nanoseconds(), 0);
}

#[cfg(feature = "parallel")]
#[test]
fn parallel_caller_buffers_cover_one_and_eight_threads_and_retained_plan() {
    use cmg::{ParallelCmgPlan, ParallelExecutor, ParallelOptions};

    let topology =
        PreparedLaplacianTopology::prepare(64, (0..63).map(|vertex| (vertex, vertex + 1))).unwrap();
    let old = topology.assemble(&vec![1.0; 63]).unwrap();
    let current = topology
        .assemble(
            &(0..63)
                .map(|index| if index % 2 == 0 { 0.5 } else { 2.0 })
                .collect::<Vec<_>>(),
        )
        .unwrap();
    let retained = CmgPreconditioner::build(
        &old,
        CmgOptions {
            direct_threshold: 2,
            ..CmgOptions::default()
        },
    )
    .unwrap();
    let targets: Vec<Vec<f64>> = (0..4)
        .map(|rhs| {
            (0..64)
                .map(|vertex| ((vertex * (rhs + 3) + 7) % 29) as f64 - 14.0)
                .collect()
        })
        .collect();
    let right_hand_sides: Vec<Vec<f64>> = targets
        .iter()
        .map(|target| current.matvec(target).unwrap())
        .collect();
    let packed: Vec<_> = right_hand_sides.iter().flatten().copied().collect();
    for threads in [1, 8] {
        let executor = ParallelExecutor::new(ParallelOptions {
            threads,
            min_parallel_len: 1,
            reduction_chunk_size: 16,
            ..ParallelOptions::default()
        })
        .unwrap();
        let fresh = CmgPreconditioner::build(&current, CmgOptions::default()).unwrap();
        let mut workspaces: Vec<_> = (0..threads.min(4))
            .map(|_| PcgBatchWorkspace::new(&fresh).unwrap())
            .collect();
        let mut output = vec![0.0; packed.len()];
        let mut diagnostics = vec![PcgDiagnostics::default(); 4];
        cmg::solve_pcg_batch_into(
            &current,
            &fresh,
            PcgBatchRef::contiguous(&packed, 4, 64).unwrap(),
            None,
            PcgBatchMut::contiguous(&mut output, 4, 64).unwrap(),
            &mut diagnostics,
            strict_options(),
            &mut workspaces,
            cmg::SolveContext {
                team: Some(executor.team()),
                concurrent_right_hand_sides: true,
                ..cmg::SolveContext::default()
            },
        )
        .unwrap();
        for rhs in 0..4 {
            let oracle = dense_grounded_oracle(&current, &right_hand_sides[rhs]);
            assert_close(&output[rhs * 64..][..64], &oracle, 2.0e-8);
            assert!(
                fresh_residual(&current, &right_hand_sides[rhs], &output[rhs * 64..][..64])
                    <= diagnostics[rhs].tolerance()
            );
        }

        let fresh_plan = ParallelCmgPlan::build(&fresh, executor.team()).unwrap();
        let mut planned_workspace = PcgBatchWorkspace::new(&fresh).unwrap();
        cmg::solve_pcg_batch_into(
            &current,
            &fresh,
            PcgBatchRef::contiguous(&packed, 4, 64).unwrap(),
            None,
            PcgBatchMut::contiguous(&mut output, 4, 64).unwrap(),
            &mut diagnostics,
            strict_options(),
            core::slice::from_mut(&mut planned_workspace),
            cmg::SolveContext {
                plan: Some(&fresh_plan),
                team: Some(executor.team()),
                ..cmg::SolveContext::default()
            },
        )
        .unwrap();
        for rhs in 0..4 {
            assert_close(
                &output[rhs * 64..][..64],
                &dense_grounded_oracle(&current, &right_hand_sides[rhs]),
                2.0e-8,
            );
            assert!(
                fresh_residual(&current, &right_hand_sides[rhs], &output[rhs * 64..][..64])
                    <= diagnostics[rhs].tolerance()
            );
        }

        let mut retained_workspaces: Vec<_> = (0..threads.min(4))
            .map(|_| PcgBatchWorkspace::new(&retained).unwrap())
            .collect();
        cmg::solve_pcg_batch_into(
            &current,
            &retained,
            PcgBatchRef::contiguous(&packed, 4, 64).unwrap(),
            None,
            PcgBatchMut::contiguous(&mut output, 4, 64).unwrap(),
            &mut diagnostics,
            strict_options(),
            &mut retained_workspaces,
            cmg::SolveContext {
                team: Some(executor.team()),
                concurrent_right_hand_sides: true,
                retained_preconditioner: true,
                ..cmg::SolveContext::default()
            },
        )
        .unwrap();
        for rhs in 0..4 {
            assert_close(
                &output[rhs * 64..][..64],
                &dense_grounded_oracle(&current, &right_hand_sides[rhs]),
                2.0e-6,
            );
            assert!(
                fresh_residual(&current, &right_hand_sides[rhs], &output[rhs * 64..][..64])
                    <= diagnostics[rhs].tolerance()
            );
        }

        let plan = ParallelCmgPlan::build(&retained, executor.team()).unwrap();
        let mut retained_workspace = PcgBatchWorkspace::new(&retained).unwrap();
        cmg::solve_pcg_batch_into(
            &current,
            &retained,
            PcgBatchRef::contiguous(&packed, 4, 64).unwrap(),
            None,
            PcgBatchMut::contiguous(&mut output, 4, 64).unwrap(),
            &mut diagnostics,
            strict_options(),
            core::slice::from_mut(&mut retained_workspace),
            cmg::SolveContext {
                plan: Some(&plan),
                team: Some(executor.team()),
                retained_preconditioner: true,
                ..cmg::SolveContext::default()
            },
        )
        .unwrap();
        for rhs in 0..4 {
            assert_close(
                &output[rhs * 64..][..64],
                &dense_grounded_oracle(&current, &right_hand_sides[rhs]),
                2.0e-6,
            );
            assert!(
                fresh_residual(&current, &right_hand_sides[rhs], &output[rhs * 64..][..64])
                    <= diagnostics[rhs].tolerance()
            );
        }

        let mut single_solution = vec![0.0; 64];
        let mut single_workspace = PcgWorkspace::new(&fresh);
        cmg::solve_pcg_into(
            &current,
            &fresh,
            &right_hand_sides[0],
            None,
            &mut single_solution,
            strict_options(),
            &mut single_workspace,
            cmg::SolveContext {
                plan: Some(&fresh_plan),
                team: Some(executor.team()),
                ..cmg::SolveContext::default()
            },
        )
        .unwrap();
        assert_close(
            &single_solution,
            &dense_grounded_oracle(&current, &right_hand_sides[0]),
            2.0e-8,
        );

        let mut retained_single_workspace = PcgWorkspace::new(&retained);
        cmg::solve_pcg_into(
            &current,
            &retained,
            &right_hand_sides[0],
            None,
            &mut single_solution,
            strict_options(),
            &mut retained_single_workspace,
            cmg::SolveContext {
                plan: Some(&plan),
                team: Some(executor.team()),
                retained_preconditioner: true,
                ..cmg::SolveContext::default()
            },
        )
        .unwrap();
        assert_close(
            &single_solution,
            &dense_grounded_oracle(&current, &right_hand_sides[0]),
            2.0e-6,
        );
    }
}
