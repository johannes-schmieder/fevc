use cmg::{
    CmgOptions, GroundedLdl, Laplacian, PcgOptions, SddmMatrix, SddmSolver, ValidationOptions,
};

fn dense_solve(mut matrix: Vec<Vec<f64>>, mut rhs: Vec<f64>) -> Vec<f64> {
    let n = matrix.len();
    assert_eq!(rhs.len(), n);
    for row in &matrix {
        assert_eq!(row.len(), n);
    }
    for pivot_column in 0..n {
        let pivot_row = (pivot_column..n)
            .max_by(|left, right| {
                matrix[*left][pivot_column]
                    .abs()
                    .total_cmp(&matrix[*right][pivot_column].abs())
            })
            .unwrap();
        assert!(matrix[pivot_row][pivot_column].abs() > 1.0e-14);
        matrix.swap(pivot_column, pivot_row);
        rhs.swap(pivot_column, pivot_row);
        let pivot = matrix[pivot_column][pivot_column];
        let pivot_tail = matrix[pivot_column][(pivot_column + 1)..].to_vec();
        for row in (pivot_column + 1)..n {
            let multiplier = matrix[row][pivot_column] / pivot;
            matrix[row][pivot_column] = 0.0;
            for (entry, pivot_entry) in matrix[row][(pivot_column + 1)..]
                .iter_mut()
                .zip(&pivot_tail)
            {
                *entry -= multiplier * *pivot_entry;
            }
            rhs[row] -= multiplier * rhs[pivot_column];
        }
    }
    let mut solution = vec![0.0; n];
    for row in (0..n).rev() {
        let tail: f64 = matrix[row][(row + 1)..]
            .iter()
            .zip(&solution[(row + 1)..])
            .map(|(coefficient, value)| coefficient * value)
            .sum();
        solution[row] = (rhs[row] - tail) / matrix[row][row];
    }
    solution
}

fn assert_vector_close(left: &[f64], right: &[f64], tolerance: f64) {
    assert_eq!(left.len(), right.len());
    for (left_value, right_value) in left.iter().zip(right) {
        let scale = 1.0_f64.max(left_value.abs()).max(right_value.abs());
        assert!((left_value - right_value).abs() <= tolerance * scale);
    }
}

#[test]
fn sddm_solver_matches_independent_dense_elimination() {
    for n in 1..=10 {
        let mut off_diagonal = Vec::new();
        let mut off_sums = vec![0.0; n];
        for left in 0..n {
            for right in (left + 1)..n {
                if (7 * left + 3 * right) % 4 != 0 {
                    let weight = 0.125 * (1 + (left + 2 * right) % 7) as f64;
                    off_diagonal.push((left, right, -weight));
                    off_sums[left] += weight;
                    off_sums[right] += weight;
                }
            }
        }
        let diagonal: Vec<f64> = off_sums
            .iter()
            .enumerate()
            .map(|(index, off_sum)| *off_sum + 0.5 + 0.125 * index as f64)
            .collect();
        let matrix =
            SddmMatrix::from_parts(diagonal, off_diagonal, ValidationOptions::default()).unwrap();
        let rhs: Vec<f64> = (0..n)
            .map(|index| (index as f64 + 0.25).cos() + 0.1 * index as f64)
            .collect();
        let expected = dense_solve(matrix.to_dense(), rhs.clone());
        let solver =
            SddmSolver::from_matrix(&matrix, CmgOptions::default(), ValidationOptions::default())
                .unwrap();
        let result = solver
            .solve(
                &rhs,
                PcgOptions {
                    relative_tolerance: 1.0e-13,
                    absolute_tolerance: 1.0e-14,
                    ..PcgOptions::default()
                },
            )
            .unwrap();
        assert_vector_close(result.solution(), &expected, 1.0e-10);
        assert!(result.residual_norm() <= result.tolerance());
    }
}

#[test]
fn grounded_ldl_matches_independent_dense_reduced_solves() {
    let graphs = [
        Laplacian::from_edges(5, (0..4).map(|index| (index, index + 1, 1.0))).unwrap(),
        Laplacian::from_edges(
            6,
            (0..6).map(|index| (index, (index + 1) % 6, 1.0 + 0.25 * index as f64)),
        )
        .unwrap(),
        Laplacian::from_edges(7, (1..7).map(|leaf| (0, leaf, 0.5 + 0.25 * leaf as f64))).unwrap(),
    ];

    for graph in graphs {
        let n = graph.vertex_count();
        let anchor = n - 1;
        let dense = graph.to_dense();
        let reduced: Vec<Vec<f64>> = (0..anchor)
            .map(|row| (0..anchor).map(|column| dense[row][column]).collect())
            .collect();
        let reduced_rhs: Vec<f64> = (0..anchor)
            .map(|index| (index as f64 + 0.5).sin())
            .collect();
        let mut rhs = reduced_rhs.clone();
        rhs.push(-reduced_rhs.iter().sum::<f64>());
        let mut expected = dense_solve(reduced, reduced_rhs);
        expected.push(0.0);
        let factor = GroundedLdl::factor(&graph).unwrap();
        let actual = factor.solve(&rhs).unwrap();
        assert_vector_close(&actual, &expected, 1.0e-11);
    }
}
