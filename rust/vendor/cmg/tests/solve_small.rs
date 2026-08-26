use cmg::{
    CmgError, CmgOptions, CmgPreconditioner, Laplacian, PcgOptions, PcgWorkspace, solve_pcg,
    solve_pcg_batch, solve_pcg_with_workspace,
};

fn path(vertex_count: usize) -> Laplacian {
    Laplacian::from_edges(
        vertex_count,
        (0..vertex_count.saturating_sub(1)).map(|vertex| (vertex, vertex + 1, 1.0)),
    )
    .unwrap()
}

fn norm(values: &[f64]) -> f64 {
    values.iter().map(|value| value * value).sum::<f64>().sqrt()
}

fn residual(graph: &Laplacian, rhs: &[f64], solution: &[f64]) -> Vec<f64> {
    graph
        .matvec(solution)
        .unwrap()
        .iter()
        .zip(rhs)
        .map(|(matrix_value, rhs_value)| rhs_value - matrix_value)
        .collect()
}

fn assert_vector_close(left: &[f64], right: &[f64], tolerance: f64) {
    assert_eq!(left.len(), right.len());
    for (left_value, right_value) in left.iter().zip(right) {
        let scale = 1.0_f64.max(left_value.abs()).max(right_value.abs());
        assert!((left_value - right_value).abs() <= tolerance * scale);
    }
}

#[test]
fn exact_direct_preconditioner_converges_in_one_iteration() {
    let graph = path(5);
    let known = [3.0, 1.0, -2.0, 4.0, 0.0];
    let rhs = graph.matvec(&known).unwrap();
    let preconditioner = CmgPreconditioner::build(&graph, CmgOptions::default()).unwrap();
    let options = PcgOptions {
        relative_tolerance: 1.0e-12,
        ..PcgOptions::default()
    };
    let result = solve_pcg(&graph, &preconditioner, &rhs, options).unwrap();
    assert_eq!(result.iterations(), 1);
    let fresh = residual(&graph, &rhs, result.solution());
    assert!(norm(&fresh) <= result.tolerance());
    assert!((result.residual_norm() - norm(&fresh)).abs() <= 1.0e-14);
    assert!(result.backward_error() <= options.relative_tolerance);

    let mean = known.iter().sum::<f64>() / known.len() as f64;
    let expected: Vec<f64> = known.iter().map(|value| value - mean).collect();
    assert_vector_close(result.solution(), &expected, 1.0e-12);
}

#[test]
fn zero_rhs_returns_without_iteration() {
    let graph = path(8);
    let preconditioner = CmgPreconditioner::build(&graph, CmgOptions::default()).unwrap();
    let result = solve_pcg(&graph, &preconditioner, &[0.0; 8], PcgOptions::default()).unwrap();
    assert_eq!(result.iterations(), 0);
    assert_eq!(result.solution(), &[0.0; 8]);
    assert_eq!(result.residual_norm(), 0.0);
}

#[test]
fn forced_multilevel_path_has_a_fresh_residual_certificate() {
    let graph = path(64);
    let center = 0.5 * (graph.vertex_count() - 1) as f64;
    let known: Vec<f64> = (0..graph.vertex_count())
        .map(|vertex| {
            let x = vertex as f64 - center;
            0.01 * x * x * x - 0.2 * x
        })
        .collect();
    let rhs = graph.matvec(&known).unwrap();
    let preconditioner = CmgPreconditioner::build(
        &graph,
        CmgOptions {
            direct_threshold: 2,
            ..CmgOptions::default()
        },
    )
    .unwrap();
    let options = PcgOptions {
        relative_tolerance: 1.0e-10,
        max_iterations: 500,
        residual_recompute_interval: 7,
        ..PcgOptions::default()
    };
    let result = solve_pcg(&graph, &preconditioner, &rhs, options).unwrap();
    let fresh = residual(&graph, &rhs, result.solution());
    assert!(norm(&fresh) <= result.tolerance());
    assert!((norm(&fresh) - result.residual_norm()).abs() <= 1.0e-11);
    assert!(result.iterations() > 0);
}

#[test]
fn disconnected_components_are_solved_on_the_quotient_space() {
    let graph = Laplacian::from_edges(6, [(0, 1, 2.0), (1, 2, 1.0), (3, 4, 3.0)]).unwrap();
    let known = [2.0, -1.0, 4.0, 7.0, 2.0, 0.0];
    let rhs = graph.matvec(&known).unwrap();
    let preconditioner = CmgPreconditioner::build(&graph, CmgOptions::default()).unwrap();
    let result = solve_pcg(
        &graph,
        &preconditioner,
        &rhs,
        PcgOptions {
            relative_tolerance: 1.0e-12,
            ..PcgOptions::default()
        },
    )
    .unwrap();
    assert!(norm(&residual(&graph, &rhs, result.solution())) <= result.tolerance());
    assert_eq!(result.solution()[5], 0.0);
}

#[test]
fn batch_and_individual_solves_are_identical() {
    let graph = path(32);
    let preconditioner = CmgPreconditioner::build(
        &graph,
        CmgOptions {
            direct_threshold: 2,
            ..CmgOptions::default()
        },
    )
    .unwrap();
    let first_known: Vec<f64> = (0..32).map(|vertex| vertex as f64 - 15.5).collect();
    let second_known: Vec<f64> = (0..32)
        .map(|vertex| if vertex % 2 == 0 { 1.0 } else { -1.0 })
        .collect();
    let right_hand_sides = vec![
        graph.matvec(&first_known).unwrap(),
        graph.matvec(&second_known).unwrap(),
    ];
    let options = PcgOptions {
        relative_tolerance: 1.0e-10,
        ..PcgOptions::default()
    };
    let batch = solve_pcg_batch(&graph, &preconditioner, &right_hand_sides, options).unwrap();
    for (rhs, batch_result) in right_hand_sides.iter().zip(&batch) {
        let individual = solve_pcg(&graph, &preconditioner, rhs, options).unwrap();
        assert_eq!(batch_result, &individual);
    }
}

#[test]
fn explicit_workspace_can_be_reused_across_distinct_rhs() {
    let graph = path(16);
    let preconditioner = CmgPreconditioner::build(&graph, CmgOptions::default()).unwrap();
    let mut workspace = PcgWorkspace::new(&preconditioner);
    let first = graph
        .matvec(&(0..16).map(|value| value as f64).collect::<Vec<_>>())
        .unwrap();
    let second = graph
        .matvec(
            &(0..16)
                .map(|value| if value < 8 { 1.0 } else { -1.0 })
                .collect::<Vec<_>>(),
        )
        .unwrap();
    let options = PcgOptions::default();
    let first_result =
        solve_pcg_with_workspace(&graph, &preconditioner, &first, options, &mut workspace).unwrap();
    let second_result =
        solve_pcg_with_workspace(&graph, &preconditioner, &second, options, &mut workspace)
            .unwrap();
    assert!(first_result.residual_norm() <= first_result.tolerance());
    assert!(second_result.residual_norm() <= second_result.tolerance());
}

#[test]
fn iteration_budget_and_compatibility_failures_are_explicit() {
    let graph = path(64);
    let preconditioner = CmgPreconditioner::build(
        &graph,
        CmgOptions {
            direct_threshold: 2,
            ..CmgOptions::default()
        },
    )
    .unwrap();
    let known: Vec<f64> = (0..64).map(|vertex| (vertex as f64).sin()).collect();
    let rhs = graph.matvec(&known).unwrap();
    let error = solve_pcg(
        &graph,
        &preconditioner,
        &rhs,
        PcgOptions {
            relative_tolerance: 1.0e-15,
            max_iterations: 1,
            ..PcgOptions::default()
        },
    )
    .unwrap_err();
    assert!(matches!(
        error,
        CmgError::MaximumIterations { .. } | CmgError::ResidualVerificationFailed { .. }
    ));

    assert!(matches!(
        solve_pcg(&graph, &preconditioner, &[1.0; 64], PcgOptions::default()),
        Err(CmgError::IncompatibleLaplacianRhs { .. })
    ));
}
