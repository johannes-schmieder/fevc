use cmg::{CmgError, Components, Laplacian, ValidationOptions};

fn assert_close(left: f64, right: f64, tolerance: f64) {
    let scale = 1.0_f64.max(left.abs()).max(right.abs());
    assert!(
        (left - right).abs() <= tolerance * scale,
        "{left} differs from {right}"
    );
}

#[test]
fn path_dense_matvec_and_energy_are_exact() {
    let graph = Laplacian::from_edges(3, [(0, 1, 2.0), (1, 2, 3.0)]).unwrap();
    assert_eq!(graph.diagonal(), &[2.0, 5.0, 3.0]);
    assert_eq!(
        graph.to_dense(),
        vec![
            vec![2.0, -2.0, 0.0],
            vec![-2.0, 5.0, -3.0],
            vec![0.0, -3.0, 3.0]
        ]
    );
    let x = [1.0, -2.0, 4.0];
    assert_eq!(graph.matvec(&x).unwrap(), vec![6.0, -24.0, 18.0]);
    assert_close(graph.energy(&x).unwrap(), 126.0, 1.0e-15);
    let lx = graph.matvec(&x).unwrap();
    let quadratic: f64 = x.iter().zip(lx).map(|(left, right)| left * right).sum();
    assert_close(graph.energy(&x).unwrap(), quadratic, 1.0e-15);
}

#[test]
fn canonicalization_is_independent_of_input_order() {
    let first =
        Laplacian::from_edges(4, [(3, 1, 0.25), (0, 2, 4.0), (1, 3, 0.75), (2, 0, 1.0)]).unwrap();
    let second =
        Laplacian::from_edges(4, [(0, 2, 1.0), (3, 1, 0.75), (2, 0, 4.0), (1, 3, 0.25)]).unwrap();
    assert_eq!(first, second);
    assert_eq!(first.edge_count(), 2);
    assert_eq!(first.edges()[0].weight(), 5.0);
    assert_eq!(first.edges()[1].weight(), 1.0);
}

#[test]
fn graph_rejects_invalid_edges() {
    assert!(matches!(
        Laplacian::from_edges(2, [(0, 2, 1.0)]),
        Err(CmgError::VertexOutOfBounds { .. })
    ));
    assert!(matches!(
        Laplacian::from_edges(2, [(1, 1, 1.0)]),
        Err(CmgError::SelfLoop { .. })
    ));
    assert!(matches!(
        Laplacian::from_edges(2, [(0, 1, 0.0)]),
        Err(CmgError::InvalidEdgeWeight { .. })
    ));
}

#[test]
fn components_include_isolated_vertices_and_center_deterministically() {
    let graph = Laplacian::from_edges(6, [(0, 2, 1.0), (3, 4, 2.0)]).unwrap();
    let components = Components::from_laplacian(&graph);
    assert_eq!(components.labels(), &[0, 1, 0, 2, 2, 3]);
    assert_eq!(components.sizes(), &[2, 1, 2, 1]);

    let rhs = [3.0, 0.0, -3.0, 2.0, -2.0, 0.0];
    components
        .validate_rhs(&rhs, ValidationOptions::default())
        .unwrap();
    let bad_rhs = [3.0, 1.0, -3.0, 2.0, -2.0, 0.0];
    assert!(matches!(
        components.validate_rhs(&bad_rhs, ValidationOptions::default()),
        Err(CmgError::IncompatibleLaplacianRhs { component: 1, .. })
    ));

    let mut values = [5.0, 7.0, 1.0, -2.0, 4.0, 9.0];
    components.center_in_place(&mut values).unwrap();
    assert_eq!(values, [2.0, 0.0, -2.0, -3.0, 3.0, 0.0]);
}

#[test]
fn projection_accepts_roundoff_but_rejects_material_nullspace_mass() {
    let graph = Laplacian::from_edges(4, [(0, 1, 1.0), (1, 2, 1.0), (2, 3, 1.0)]).unwrap();
    let components = Components::from_laplacian(&graph);
    let options = ValidationOptions::default();

    let mut roundoff = [1.0e9, -1.0e9, 1.0e-7, 0.0];
    let projection_norm = components
        .project_rhs_in_place(&mut roundoff, options)
        .unwrap();
    assert!(projection_norm > 0.0);
    assert_eq!(components.sums(&roundoff).unwrap(), vec![0.0]);

    let mut incompatible = [1.0, 0.0, 0.0, 0.0];
    assert!(matches!(
        components.project_rhs_in_place(&mut incompatible, options),
        Err(CmgError::IncompatibleLaplacianRhs { component: 0, .. })
    ));
}
