use cmg::{
    CmgError, CmgOptions, PcgOptions, SddmMatrix, SddmSolver, ValidationOptions, solve_sddm,
};

fn assert_vector_close(left: &[f64], right: &[f64], tolerance: f64) {
    assert_eq!(left.len(), right.len());
    for (left_value, right_value) in left.iter().zip(right) {
        let scale = 1.0_f64.max(left_value.abs()).max(right_value.abs());
        assert!(
            (left_value - right_value).abs() <= tolerance * scale,
            "{left_value} differs from {right_value}"
        );
    }
}

fn residual(matrix: &SddmMatrix, rhs: &[f64], solution: &[f64]) -> Vec<f64> {
    matrix
        .matvec(solution)
        .unwrap()
        .iter()
        .zip(rhs)
        .map(|(matrix_value, rhs_value)| rhs_value - matrix_value)
        .collect()
}

fn norm(values: &[f64]) -> f64 {
    values.iter().map(|value| value * value).sum::<f64>().sqrt()
}

#[test]
fn strictly_dominant_small_system_is_solved_and_certified() {
    let matrix = SddmMatrix::from_dense(
        &[
            vec![4.0, -1.0, 0.0],
            vec![-1.0, 3.0, -1.0],
            vec![0.0, -1.0, 2.0],
        ],
        ValidationOptions::default(),
    )
    .unwrap();
    let known = [2.0, -1.0, 3.0];
    let rhs = matrix.matvec(&known).unwrap();
    let options = PcgOptions {
        relative_tolerance: 1.0e-12,
        ..PcgOptions::default()
    };
    let result = solve_sddm(&matrix, &rhs, CmgOptions::default(), options).unwrap();
    assert_vector_close(result.solution(), &known, 1.0e-11);
    let fresh = residual(&matrix, &rhs, result.solution());
    assert!(norm(&fresh) <= result.tolerance());
    assert!((norm(&fresh) - result.residual_norm()).abs() <= 1.0e-13);
    assert!(result.backward_error() <= options.relative_tolerance);
}

#[test]
fn forced_multilevel_strictly_dominant_path_is_solved() {
    let n = 64;
    let mut diagonal = vec![2.25; n];
    diagonal[0] = 1.25;
    diagonal[n - 1] = 1.25;
    let matrix = SddmMatrix::from_parts(
        diagonal,
        (0..(n - 1)).map(|vertex| (vertex, vertex + 1, -1.0)),
        ValidationOptions::default(),
    )
    .unwrap();
    let known: Vec<f64> = (0..n)
        .map(|vertex| {
            let x = vertex as f64 - 0.5 * (n - 1) as f64;
            x.sin() + 0.01 * x
        })
        .collect();
    let rhs = matrix.matvec(&known).unwrap();
    let solver = SddmSolver::from_matrix(
        &matrix,
        CmgOptions {
            direct_threshold: 2,
            ..CmgOptions::default()
        },
        ValidationOptions::default(),
    )
    .unwrap();
    let options = PcgOptions {
        relative_tolerance: 1.0e-10,
        max_iterations: 1_000,
        residual_recompute_interval: 7,
        ..PcgOptions::default()
    };
    let result = solver.solve(&rhs, options).unwrap();
    assert!(result.residual_norm() <= result.tolerance());
    assert_vector_close(result.solution(), &known, 1.0e-7);
    assert!(solver.augmentation().is_augmented());
    assert!(result.iterations() > 0);
    assert!(result.refinements() <= 3);
}

#[test]
fn singular_laplacian_sddm_uses_component_gauge() {
    let matrix = SddmMatrix::from_dense(
        &[
            vec![1.0, -1.0, 0.0],
            vec![-1.0, 2.0, -1.0],
            vec![0.0, -1.0, 1.0],
        ],
        ValidationOptions::default(),
    )
    .unwrap();
    let known = [4.0, -2.0, 1.0];
    let rhs = matrix.matvec(&known).unwrap();
    let solver =
        SddmSolver::from_matrix(&matrix, CmgOptions::default(), ValidationOptions::default())
            .unwrap();
    assert!(!solver.augmentation().is_augmented());
    let result = solver.solve(&rhs, PcgOptions::default()).unwrap();
    assert!(norm(&residual(&matrix, &rhs, result.solution())) <= result.tolerance());
    let mean = known.iter().sum::<f64>() / known.len() as f64;
    let expected: Vec<f64> = known.iter().map(|value| value - mean).collect();
    assert_vector_close(result.solution(), &expected, 1.0e-10);
}

#[test]
fn incompatible_rhs_on_a_singular_block_is_rejected() {
    let matrix = SddmMatrix::from_dense(
        &[vec![1.0, -1.0], vec![-1.0, 1.0]],
        ValidationOptions::default(),
    )
    .unwrap();
    let solver =
        SddmSolver::from_matrix(&matrix, CmgOptions::default(), ValidationOptions::default())
            .unwrap();
    assert!(matches!(
        solver.solve(&[1.0, 0.0], PcgOptions::default()),
        Err(CmgError::IncompatibleLaplacianRhs { .. })
    ));
}

#[test]
fn workspace_and_batch_results_match_individual_solves() {
    let matrix = SddmMatrix::from_dense(
        &[
            vec![3.0, -1.0, 0.0],
            vec![-1.0, 3.0, -1.0],
            vec![0.0, -1.0, 3.0],
        ],
        ValidationOptions::default(),
    )
    .unwrap();
    let solver =
        SddmSolver::from_matrix(&matrix, CmgOptions::default(), ValidationOptions::default())
            .unwrap();
    let right_hand_sides = vec![
        matrix.matvec(&[1.0, 2.0, 3.0]).unwrap(),
        matrix.matvec(&[-2.0, 0.5, 4.0]).unwrap(),
    ];
    let options = PcgOptions {
        relative_tolerance: 1.0e-12,
        ..PcgOptions::default()
    };
    let batch = solver.solve_batch(&right_hand_sides, options).unwrap();
    let mut workspace = solver.workspace();
    assert_eq!(workspace.dimension(), 3);
    assert_eq!(workspace.augmented_dimension(), 4);
    for (rhs, batch_result) in right_hand_sides.iter().zip(&batch) {
        let reused = solver
            .solve_with_workspace(rhs, options, &mut workspace)
            .unwrap();
        let individual = solver.solve(rhs, options).unwrap();
        assert_eq!(batch_result, &individual);
        assert_eq!(batch_result, &reused);
    }
}
