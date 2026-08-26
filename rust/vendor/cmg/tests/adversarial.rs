use cmg::{CmgOptions, CmgPreconditioner, Components, Laplacian, PcgOptions, solve_pcg};

fn cycle(vertex_count: usize) -> Laplacian {
    Laplacian::from_edges(
        vertex_count,
        (0..vertex_count).map(|vertex| {
            (
                vertex,
                (vertex + 1) % vertex_count,
                1.0 + 0.25 * (vertex % 3) as f64,
            )
        }),
    )
    .unwrap()
}

fn grid(rows: usize, columns: usize) -> Laplacian {
    let mut edges = Vec::new();
    for row in 0..rows {
        for column in 0..columns {
            let vertex = row * columns + column;
            if column + 1 < columns {
                edges.push((vertex, vertex + 1, 1.0 + 0.25 * (row % 2) as f64));
            }
            if row + 1 < rows {
                edges.push((vertex, vertex + columns, 1.5 + 0.25 * (column % 2) as f64));
            }
        }
    }
    Laplacian::from_edges(rows * columns, edges).unwrap()
}

fn complete(vertex_count: usize) -> Laplacian {
    Laplacian::from_edges(
        vertex_count,
        (0..vertex_count).flat_map(|left| {
            ((left + 1)..vertex_count)
                .map(move |right| (left, right, 1.0 + 0.125 * ((left + 3 * right) % 5) as f64))
        }),
    )
    .unwrap()
}

fn barbell(clique_size: usize, bridge_weight: f64) -> Laplacian {
    let mut edges = Vec::new();
    for offset in [0, clique_size] {
        for left in 0..clique_size {
            for right in (left + 1)..clique_size {
                edges.push((offset + left, offset + right, 1.0));
            }
        }
    }
    edges.push((clique_size - 1, clique_size, bridge_weight));
    Laplacian::from_edges(2 * clique_size, edges).unwrap()
}

fn lollipop(clique_size: usize, path_size: usize) -> Laplacian {
    let mut edges = Vec::new();
    for left in 0..clique_size {
        for right in (left + 1)..clique_size {
            edges.push((left, right, 1.0));
        }
    }
    edges.push((clique_size - 1, clique_size, 0.25));
    for vertex in clique_size..(clique_size + path_size - 1) {
        edges.push((vertex, vertex + 1, 0.75 + 0.25 * (vertex % 3) as f64));
    }
    Laplacian::from_edges(clique_size + path_size, edges).unwrap()
}

fn worker_firm_graph(workers: usize, firms: usize) -> Laplacian {
    let mut edges = Vec::new();
    for worker in 0..workers {
        let first = worker % firms;
        let second = (worker + 1) % firms;
        edges.push((worker, workers + first, 1.0 + 0.25 * (worker % 4) as f64));
        edges.push((worker, workers + second, 0.5 + 0.125 * (worker % 3) as f64));
    }
    Laplacian::from_edges(workers + firms, edges).unwrap()
}

fn solve_known(graph: &Laplacian, seed: f64, vector_tolerance: Option<f64>) {
    let mut expected: Vec<f64> = (0..graph.vertex_count())
        .map(|vertex| {
            let coordinate = vertex as f64 + seed;
            coordinate.sin() + 0.125 * coordinate.cos() + 0.01 * coordinate
        })
        .collect();
    let components = Components::from_laplacian(graph);
    components.center_in_place(&mut expected).unwrap();
    let rhs = graph.matvec(&expected).unwrap();
    let preconditioner = CmgPreconditioner::build(
        graph,
        CmgOptions {
            direct_threshold: 2,
            max_levels: 128,
            ..CmgOptions::default()
        },
    )
    .unwrap();
    let options = PcgOptions {
        relative_tolerance: 1.0e-10,
        absolute_tolerance: 0.0,
        max_iterations: 5_000,
        residual_recompute_interval: 11,
        ..PcgOptions::default()
    };
    let result = solve_pcg(graph, &preconditioner, &rhs, options).unwrap();
    assert!(result.residual_norm() <= result.tolerance());
    assert!(result.backward_error() <= options.relative_tolerance);
    assert!(result.solution().iter().all(|value| value.is_finite()));

    let fresh = graph.matvec(result.solution()).unwrap();
    let residual_norm = rhs
        .iter()
        .zip(fresh)
        .map(|(rhs_value, matrix_value)| {
            let residual = rhs_value - matrix_value;
            residual * residual
        })
        .sum::<f64>()
        .sqrt();
    assert!(residual_norm <= result.tolerance());

    if let Some(tolerance) = vector_tolerance {
        for (actual, expected_value) in result.solution().iter().zip(&expected) {
            let scale = 1.0_f64.max(actual.abs()).max(expected_value.abs());
            assert!((actual - expected_value).abs() <= tolerance * scale);
        }
    }
}

#[test]
fn graph_family_qualification() {
    solve_known(&cycle(18), 0.25, Some(1.0e-7));
    solve_known(&grid(5, 6), 0.5, Some(1.0e-7));
    solve_known(&complete(10), 0.75, Some(1.0e-8));
    solve_known(&barbell(6, 1.0e-6), 1.0, None);
    solve_known(&lollipop(6, 10), 1.25, Some(1.0e-5));
    solve_known(&worker_firm_graph(8, 7), 1.5, Some(1.0e-7));
}

#[test]
fn disconnected_components_and_isolates_are_solved_together() {
    let edges = (0..8)
        .map(|vertex| (vertex, (vertex + 1) % 8, 1.0))
        .chain((8..14).map(|vertex| (vertex, vertex + 1, 0.75)));
    let graph = Laplacian::from_edges(17, edges).unwrap();
    solve_known(&graph, 2.0, Some(1.0e-7));
}

#[test]
fn global_weight_scaling_preserves_certified_solves() {
    for scale in [1.0e-9, 1.0, 1.0e9] {
        let graph = Laplacian::from_edges(
            28,
            (0..27).map(|vertex| {
                let weight = if vertex % 2 == 0 { 0.5 } else { 2.0 };
                (vertex, vertex + 1, scale * weight)
            }),
        )
        .unwrap();
        solve_known(&graph, 2.5, Some(1.0e-6));
    }
}

#[test]
fn heterogeneous_edge_weights_remain_numerically_safe() {
    let graph = Laplacian::from_edges(
        36,
        (0..35).map(|vertex| {
            let weight = if vertex % 2 == 0 { 1.0e-2 } else { 1.0e2 };
            (vertex, vertex + 1, weight)
        }),
    )
    .unwrap();
    solve_known(&graph, 3.0, None);
}
