use cmg::{CmgError, CmgOptions, CmgPreconditioner, Laplacian, TerminalReason};

fn dot(left: &[f64], right: &[f64]) -> f64 {
    left.iter().zip(right).map(|(x, y)| x * y).sum()
}

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

fn path(vertex_count: usize) -> Laplacian {
    Laplacian::from_edges(
        vertex_count,
        (0..vertex_count.saturating_sub(1)).map(|vertex| (vertex, vertex + 1, 1.0)),
    )
    .unwrap()
}

#[test]
fn direct_terminal_application_matches_exact_grounded_solve() {
    let graph = Laplacian::from_edges(3, [(0, 1, 2.0), (1, 2, 3.0)]).unwrap();
    let preconditioner = CmgPreconditioner::build(&graph, CmgOptions::default()).unwrap();
    assert_eq!(
        preconditioner.hierarchy().report().terminal_reason(),
        TerminalReason::Direct
    );
    assert!(preconditioner.terminal_factor().is_some());
    let rhs = [6.0, -12.0, 6.0];
    assert_vector_close(
        &preconditioner.apply(&rhs).unwrap(),
        &[1.0, -2.0, 0.0],
        1.0e-14,
    );
}

#[test]
fn full_contraction_terminal_is_upstream_damped_jacobi() {
    let graph = Laplacian::from_edges(6, (1..6).map(|leaf| (0, leaf, 1.0))).unwrap();
    let options = CmgOptions {
        direct_threshold: 1,
        ..CmgOptions::default()
    };
    let preconditioner = CmgPreconditioner::build(&graph, options).unwrap();
    assert_eq!(
        preconditioner.hierarchy().report().terminal_reason(),
        TerminalReason::FullContraction
    );
    let rhs = [5.0, -1.0, -1.0, -1.0, -1.0, -1.0];
    assert_eq!(
        preconditioner.apply(&rhs).unwrap(),
        [0.5, -0.5, -0.5, -0.5, -0.5, -0.5]
    );
}

#[test]
fn penultimate_repeat_uses_direct_factor_nonzeros() {
    let graph = path(40);
    let options = CmgOptions {
        direct_threshold: 8,
        ..CmgOptions::default()
    };
    let preconditioner = CmgPreconditioner::build(&graph, options).unwrap();
    assert_eq!(
        preconditioner.hierarchy().report().terminal_reason(),
        TerminalReason::Direct
    );
    assert!(preconditioner.hierarchy().levels().len() >= 2);
    let factor = preconditioner.terminal_factor().unwrap();
    let penultimate = preconditioner.hierarchy().levels().len() - 2;
    let fine_nonzeros = preconditioner.hierarchy().levels()[penultimate]
        .graph()
        .matrix_nnz();
    let expected = if factor.factor_nonzeros() == 0 {
        1
    } else {
        (fine_nonzeros / factor.factor_nonzeros())
            .saturating_sub(1)
            .max(1)
    };
    assert_eq!(preconditioner.repeat_counts()[penultimate], expected);
}

#[test]
fn forced_multilevel_cycle_is_linear_symmetric_and_positive() {
    let graph = path(24);
    let options = CmgOptions {
        direct_threshold: 2,
        ..CmgOptions::default()
    };
    let preconditioner = CmgPreconditioner::build(&graph, options).unwrap();
    assert!(preconditioner.hierarchy().levels().len() >= 2);

    let center = 0.5 * (graph.vertex_count() - 1) as f64;
    let u: Vec<f64> = (0..graph.vertex_count())
        .map(|vertex| vertex as f64 - center)
        .collect();
    let v: Vec<f64> = (0..graph.vertex_count())
        .map(|vertex| if vertex % 2 == 0 { 1.0 } else { -1.0 })
        .collect();
    let combination: Vec<f64> = u
        .iter()
        .zip(&v)
        .map(|(left, right)| 1.25 * left - 0.75 * right)
        .collect();

    let mu = preconditioner.apply(&u).unwrap();
    let mv = preconditioner.apply(&v).unwrap();
    let combined = preconditioner.apply(&combination).unwrap();
    let expected: Vec<f64> = mu
        .iter()
        .zip(&mv)
        .map(|(left, right)| 1.25 * left - 0.75 * right)
        .collect();
    assert_vector_close(&combined, &expected, 1.0e-11);

    let left_inner = dot(&u, &mv);
    let right_inner = dot(&mu, &v);
    let scale = 1.0_f64.max(left_inner.abs()).max(right_inner.abs());
    assert!((left_inner - right_inner).abs() <= 1.0e-11 * scale);
    assert!(dot(&u, &mu) > 0.0);
}

#[test]
fn caller_owned_workspace_is_reusable_and_deterministic() {
    let graph = path(24);
    let options = CmgOptions {
        direct_threshold: 2,
        ..CmgOptions::default()
    };
    let preconditioner = CmgPreconditioner::build(&graph, options).unwrap();
    let rhs: Vec<f64> = (0..24)
        .map(|vertex| if vertex < 12 { 1.0 } else { -1.0 })
        .collect();
    let mut workspace = preconditioner.workspace();
    assert_eq!(
        workspace.dimensions(),
        preconditioner.hierarchy().report().vertex_counts()
    );
    let mut first = vec![0.0; 24];
    let mut second = vec![0.0; 24];
    preconditioner
        .apply_into(&rhs, &mut first, &mut workspace)
        .unwrap();
    preconditioner
        .apply_into(&rhs, &mut second, &mut workspace)
        .unwrap();
    assert_eq!(first, second);
}

#[test]
fn incompatible_rhs_is_rejected_at_the_public_boundary() {
    let graph = path(4);
    let preconditioner = CmgPreconditioner::build(&graph, CmgOptions::default()).unwrap();
    assert!(matches!(
        preconditioner.apply(&[1.0, 0.0, 0.0, 0.0]),
        Err(CmgError::IncompatibleLaplacianRhs { .. })
    ));
}
