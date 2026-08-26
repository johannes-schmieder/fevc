use cmg::{
    CmgOptions, CmgPreconditioner, Components, Laplacian, TerminalReason, ValidationOptions,
};

fn project_rhs(graph: &Laplacian, rhs: &[f64], compatibility_tolerance: f64) -> Vec<f64> {
    let mut projected = rhs.to_vec();
    Components::from_laplacian(graph)
        .project_rhs_in_place(
            &mut projected,
            ValidationOptions {
                symmetry_tolerance: ValidationOptions::default().symmetry_tolerance,
                compatibility_tolerance,
            },
        )
        .unwrap();
    projected
}

fn center_rhs(graph: &Laplacian, rhs: &[f64]) -> Vec<f64> {
    let mut centered = rhs.to_vec();
    Components::from_laplacian(graph)
        .center_in_place(&mut centered)
        .unwrap();
    centered
}

fn reference_apply(
    preconditioner: &CmgPreconditioner,
    level_index: usize,
    rhs: &[f64],
    iterations: usize,
) -> Vec<f64> {
    let level = &preconditioner.hierarchy().levels()[level_index];
    if let Some(reason) = level.terminal_reason() {
        return if reason == TerminalReason::Direct {
            preconditioner
                .terminal_factor()
                .unwrap()
                .solve(rhs)
                .unwrap()
        } else {
            rhs.iter()
                .zip(level.inverse_diagonal())
                .map(|(rhs_value, inverse_diagonal)| *rhs_value * *inverse_diagonal)
                .collect()
        };
    }

    let graph = level.graph();
    let aggregation = level.aggregation().unwrap();
    let mut solution = vec![0.0; rhs.len()];
    for _ in 0..iterations {
        let mut residual = graph.matvec(&solution).unwrap();
        for (value, rhs_value) in residual.iter_mut().zip(rhs) {
            *value = *rhs_value - *value;
        }
        for ((value, residual_value), inverse_diagonal) in solution
            .iter_mut()
            .zip(&residual)
            .zip(level.inverse_diagonal())
        {
            *value += *inverse_diagonal * *residual_value;
        }

        residual = graph.matvec(&solution).unwrap();
        for (value, rhs_value) in residual.iter_mut().zip(rhs) {
            *value = *rhs_value - *value;
        }
        let coarse_graph = preconditioner.hierarchy().levels()[level_index + 1].graph();
        let coarse_rhs = center_rhs(coarse_graph, &aggregation.restrict(&residual).unwrap());
        let coarse_solution = reference_apply(
            preconditioner,
            level_index + 1,
            &coarse_rhs,
            preconditioner.repeat_counts()[level_index],
        );
        aggregation
            .prolong_add_into(&coarse_solution, &mut solution)
            .unwrap();

        residual = graph.matvec(&solution).unwrap();
        for (value, rhs_value) in residual.iter_mut().zip(rhs) {
            *value = *rhs_value - *value;
        }
        for ((value, residual_value), inverse_diagonal) in solution
            .iter_mut()
            .zip(&residual)
            .zip(level.inverse_diagonal())
        {
            *value += *inverse_diagonal * *residual_value;
        }
    }
    solution
}

fn assert_vector_close(left: &[f64], right: &[f64], tolerance: f64) {
    assert_eq!(left.len(), right.len());
    for (index, (left_value, right_value)) in left.iter().zip(right).enumerate() {
        let scale = 1.0_f64.max(left_value.abs()).max(right_value.abs());
        assert!(
            (left_value - right_value).abs() <= tolerance * scale,
            "mismatch at index {index}: production={left_value:.17e}, reference={right_value:.17e}, absolute_difference={:.17e}, allowed={:.17e}",
            (left_value - right_value).abs(),
            tolerance * scale,
        );
    }
}

fn compare(graph: Laplacian) {
    let preconditioner = CmgPreconditioner::build(
        &graph,
        CmgOptions {
            direct_threshold: 2,
            ..CmgOptions::default()
        },
    )
    .unwrap();
    let known: Vec<f64> = (0..graph.vertex_count())
        .map(|index| index as f64 - 0.5 * (graph.vertex_count() - 1) as f64)
        .collect();
    let rhs = graph.matvec(&known).unwrap();
    let production = preconditioner.apply(&rhs).unwrap();
    let projected_rhs = project_rhs(
        &graph,
        &rhs,
        ValidationOptions::default().compatibility_tolerance,
    );
    let reference = reference_apply(&preconditioner, 0, &projected_rhs, 1);
    let mut compatible_workspace = preconditioner.workspace();
    let mut compatible = vec![0.0; graph.vertex_count()];
    preconditioner
        .apply_compatible_into(&projected_rhs, &mut compatible, &mut compatible_workspace)
        .unwrap();
    assert_vector_close(&production, &reference, 1.0e-12);
    assert_vector_close(&compatible, &reference, 1.0e-12);
}

#[test]
fn stationary_cycle_matches_independent_allocating_reference() {
    compare(Laplacian::from_edges(40, (0..39).map(|index| (index, index + 1, 1.0))).unwrap());

    let mut grid_edges = Vec::new();
    for row in 0..5 {
        for column in 0..6 {
            let vertex = row * 6 + column;
            if column < 5 {
                grid_edges.push((vertex, vertex + 1, 1.0));
            }
            if row < 4 {
                grid_edges.push((vertex, vertex + 6, 1.5));
            }
        }
    }
    compare(Laplacian::from_edges(30, grid_edges).unwrap());

    let mut barbell_edges = Vec::new();
    for offset in [0, 6] {
        for left in 0..6 {
            for right in (left + 1)..6 {
                barbell_edges.push((offset + left, offset + right, 1.0));
            }
        }
    }
    barbell_edges.push((5, 6, 1.0e-4));
    compare(Laplacian::from_edges(12, barbell_edges).unwrap());
}
