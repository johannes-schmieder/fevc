use cmg::{CmgError, GroundedLdl, Laplacian};

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

#[test]
fn weighted_path_factor_and_solve_match_exact_grounded_system() {
    let graph = Laplacian::from_edges(3, [(0, 1, 2.0), (1, 2, 3.0)]).unwrap();
    let factor = GroundedLdl::factor(&graph).unwrap();
    assert_eq!(factor.anchors(), [2]);
    assert_eq!(factor.permutation(), [0, 1]);
    assert_eq!(factor.factor_nonzeros(), 3);

    let rhs = [6.0, -12.0, 6.0];
    let solution = factor.solve(&rhs).unwrap();
    assert_vector_close(&solution, &[1.0, -2.0, 0.0], 1.0e-14);
    assert_vector_close(&graph.matvec(&solution).unwrap(), &rhs, 1.0e-14);
}

#[test]
fn static_degree_order_places_star_leaves_before_center() {
    let graph = Laplacian::from_edges(5, (1..5).map(|leaf| (0, leaf, 1.0))).unwrap();
    let factor = GroundedLdl::factor(&graph).unwrap();
    assert_eq!(factor.anchors(), [4]);
    assert_eq!(factor.permutation(), [1, 2, 3, 0]);
    assert_eq!(factor.factor_nonzeros(), 7);
}

#[test]
fn disconnected_components_and_isolated_vertices_are_grounded_independently() {
    let graph = Laplacian::from_edges(5, [(0, 1, 2.0), (2, 3, 1.0)]).unwrap();
    let factor = GroundedLdl::factor(&graph).unwrap();
    assert_eq!(factor.anchors(), [1, 3, 4]);
    assert_eq!(factor.permutation(), [0, 2]);

    let rhs = [6.0, -6.0, -2.0, 2.0, 0.0];
    let solution = factor.solve(&rhs).unwrap();
    assert_vector_close(&solution, &[3.0, 0.0, -2.0, 0.0, 0.0], 1.0e-14);
    assert_vector_close(&graph.matvec(&solution).unwrap(), &rhs, 1.0e-14);
}

#[test]
fn solution_is_the_known_vector_shifted_to_the_anchor_gauge() {
    let graph = Laplacian::from_edges(3, [(0, 1, 2.0), (1, 2, 3.0)]).unwrap();
    let known = [4.0, -1.0, 2.0];
    let rhs = graph.matvec(&known).unwrap();
    let solution = GroundedLdl::factor(&graph).unwrap().solve(&rhs).unwrap();
    assert_vector_close(&solution, &[2.0, -3.0, 0.0], 1.0e-14);
    assert_vector_close(&graph.matvec(&solution).unwrap(), &rhs, 1.0e-14);
}

#[test]
fn incompatible_rhs_is_rejected_before_substitution() {
    let graph = Laplacian::from_edges(2, [(0, 1, 1.0)]).unwrap();
    let factor = GroundedLdl::factor(&graph).unwrap();
    assert!(matches!(
        factor.solve(&[1.0, 0.0]),
        Err(CmgError::IncompatibleLaplacianRhs { component: 0, .. })
    ));
}

#[test]
fn entirely_isolated_graph_has_an_empty_factor_and_zero_solution() {
    let graph = Laplacian::from_edges(3, []).unwrap();
    let factor = GroundedLdl::factor(&graph).unwrap();
    assert_eq!(factor.anchors(), [0, 1, 2]);
    assert_eq!(factor.active_dimension(), 0);
    assert_eq!(factor.factor_nonzeros(), 0);
    assert_eq!(factor.solve(&[0.0, 0.0, 0.0]).unwrap(), [0.0, 0.0, 0.0]);
}
