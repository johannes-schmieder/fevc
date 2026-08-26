use cmg::{
    CmgHierarchy, CmgOptions, CmgPreconditioner, Laplacian, PcgOptions, SddmMatrix,
    ValidationOptions, solve_pcg,
};

fn base_edges() -> Vec<(usize, usize, f64)> {
    vec![
        (0, 1, 1.0),
        (1, 2, 0.5),
        (2, 3, 1.5),
        (3, 4, 0.75),
        (4, 5, 1.25),
        (5, 6, 0.5),
        (6, 7, 1.0),
        (7, 8, 1.5),
        (8, 9, 0.75),
        (9, 10, 1.25),
        (10, 11, 0.5),
        (11, 0, 1.0),
        (0, 6, 0.25),
        (3, 9, 0.5),
    ]
}

fn split_edges() -> Vec<(usize, usize, f64)> {
    let mut split = Vec::new();
    for (index, (left, right, weight)) in base_edges().into_iter().enumerate() {
        let first = if index % 2 == 0 { 0.25 } else { 0.5 };
        split.push((right, left, first * weight));
        split.push((left, right, (1.0 - first) * weight));
    }
    split.reverse();
    split
}

#[test]
fn graph_hierarchy_preconditioner_and_solve_are_order_invariant() {
    let graph = Laplacian::from_edges(12, base_edges()).unwrap();
    let permuted = Laplacian::from_edges(12, split_edges()).unwrap();
    assert_eq!(graph, permuted);

    let options = CmgOptions {
        direct_threshold: 2,
        ..CmgOptions::default()
    };
    assert_eq!(
        CmgHierarchy::build(&graph, options).unwrap(),
        CmgHierarchy::build(&permuted, options).unwrap()
    );
    let first = CmgPreconditioner::build(&graph, options).unwrap();
    let second = CmgPreconditioner::build(&permuted, options).unwrap();
    assert_eq!(first, second);

    let known: Vec<f64> = (0..12).map(|index| index as f64 - 5.5).collect();
    let rhs = graph.matvec(&known).unwrap();
    let solve_options = PcgOptions {
        relative_tolerance: 1.0e-12,
        ..PcgOptions::default()
    };
    assert_eq!(
        solve_pcg(&graph, &first, &rhs, solve_options).unwrap(),
        solve_pcg(&permuted, &second, &rhs, solve_options).unwrap()
    );
}

#[test]
fn sddm_duplicate_aggregation_is_order_invariant() {
    let first = SddmMatrix::from_parts(
        vec![3.0, 4.0, 3.0],
        [(0, 1, -1.0), (1, 2, -0.5), (0, 2, -0.25)],
        ValidationOptions::default(),
    )
    .unwrap();
    let second = SddmMatrix::from_parts(
        vec![3.0, 4.0, 3.0],
        [
            (2, 0, -0.125),
            (2, 1, -0.25),
            (1, 0, -0.75),
            (0, 2, -0.125),
            (1, 2, -0.25),
            (0, 1, -0.25),
        ],
        ValidationOptions::default(),
    )
    .unwrap();
    assert_eq!(first, second);
    assert_eq!(
        first.augment(ValidationOptions::default()).unwrap(),
        second.augment(ValidationOptions::default()).unwrap()
    );
}

#[test]
fn repeated_workspace_and_batch_order_do_not_change_answers() {
    let graph = Laplacian::from_edges(20, (0..19).map(|index| (index, index + 1, 1.0))).unwrap();
    let preconditioner = CmgPreconditioner::build(
        &graph,
        CmgOptions {
            direct_threshold: 2,
            ..CmgOptions::default()
        },
    )
    .unwrap();
    let options = PcgOptions {
        relative_tolerance: 1.0e-12,
        ..PcgOptions::default()
    };
    let rhs_a = graph
        .matvec(&(0..20).map(|index| index as f64 - 9.5).collect::<Vec<_>>())
        .unwrap();
    let rhs_b = graph
        .matvec(
            &(0..20)
                .map(|index| if index % 2 == 0 { 1.0 } else { -1.0 })
                .collect::<Vec<_>>(),
        )
        .unwrap();
    let forward = cmg::solve_pcg_batch(
        &graph,
        &preconditioner,
        &[rhs_a.clone(), rhs_b.clone()],
        options,
    )
    .unwrap();
    let reverse = cmg::solve_pcg_batch(
        &graph,
        &preconditioner,
        &[rhs_b.clone(), rhs_a.clone()],
        options,
    )
    .unwrap();
    assert_eq!(forward[0], reverse[1]);
    assert_eq!(forward[1], reverse[0]);
    assert_eq!(
        forward[0],
        solve_pcg(&graph, &preconditioner, &rhs_a, options).unwrap()
    );
    assert_eq!(
        forward[1],
        solve_pcg(&graph, &preconditioner, &rhs_b, options).unwrap()
    );
}
