use cmg::{Aggregation, CmgHierarchy, CmgOptions, CmgPreconditioner, Laplacian, TerminalReason};

fn dense_galerkin(graph: &Laplacian, labels: &[usize], coarse_n: usize) -> Vec<Vec<f64>> {
    let fine = graph.to_dense();
    let mut coarse = vec![vec![0.0; coarse_n]; coarse_n];
    for (row, fine_row) in fine.iter().enumerate() {
        for (column, value) in fine_row.iter().enumerate() {
            coarse[labels[row]][labels[column]] += value;
        }
    }
    coarse
}

#[test]
fn aggregation_restricts_prolongs_and_contracts_exactly() {
    let graph = Laplacian::from_edges(4, [(0, 1, 1.0), (1, 2, 2.0), (2, 3, 3.0)]).unwrap();
    let aggregation = Aggregation::new(vec![0, 0, 1, 1], 2).unwrap();
    assert_eq!(aggregation.sizes(), [2, 2]);
    assert_eq!(
        aggregation.restrict(&[1.0, 2.0, 3.0, 4.0]).unwrap(),
        [3.0, 7.0]
    );
    assert_eq!(
        aggregation.prolong(&[5.0, -1.0]).unwrap(),
        [5.0, 5.0, -1.0, -1.0]
    );

    let coarse = aggregation.contract(&graph).unwrap();
    assert_eq!(coarse.to_dense(), vec![vec![2.0, -2.0], vec![-2.0, 2.0]]);
    assert_eq!(
        coarse.to_dense(),
        dense_galerkin(&graph, aggregation.labels(), 2)
    );
}

#[test]
fn small_graph_uses_direct_terminal() {
    let graph = Laplacian::from_edges(3, [(0, 1, 1.0), (1, 2, 1.0)]).unwrap();
    let hierarchy = CmgHierarchy::build(&graph, CmgOptions::default()).unwrap();
    assert_eq!(hierarchy.levels().len(), 1);
    assert_eq!(hierarchy.report().terminal_reason(), TerminalReason::Direct);
    assert!(!hierarchy.report().terminal_reason().is_iterative());
}

#[test]
fn star_reaches_full_contraction_terminal() {
    let graph = Laplacian::from_edges(6, (1..6).map(|leaf| (0, leaf, 1.0))).unwrap();
    let options = CmgOptions {
        direct_threshold: 1,
        ..CmgOptions::default()
    };
    let hierarchy = CmgHierarchy::build(&graph, options).unwrap();
    assert_eq!(
        hierarchy.report().terminal_reason(),
        TerminalReason::FullContraction
    );
    assert!(hierarchy.report().terminal_reason().is_iterative());
}

#[test]
fn equal_weight_clique_hits_upstream_vertex_stagnation_guard() {
    let mut edges = Vec::new();
    for left in 0..10 {
        for right in (left + 1)..10 {
            edges.push((left, right, 1.0));
        }
    }
    let graph = Laplacian::from_edges(10, edges).unwrap();
    let options = CmgOptions {
        direct_threshold: 1,
        ..CmgOptions::default()
    };
    let hierarchy = CmgHierarchy::build(&graph, options).unwrap();
    assert_eq!(
        hierarchy.report().terminal_reason(),
        TerminalReason::StagnatedVertexReduction
    );
}

#[test]
fn forced_multilevel_path_strictly_reduces_nonterminal_levels() {
    let graph = Laplacian::from_edges(24, (0..23).map(|vertex| (vertex, vertex + 1, 1.0))).unwrap();
    let options = CmgOptions {
        direct_threshold: 2,
        ..CmgOptions::default()
    };
    let hierarchy = CmgHierarchy::build(&graph, options).unwrap();
    assert!(hierarchy.levels().len() >= 2);
    for pair in hierarchy.report().vertex_counts().windows(2) {
        assert!(pair[1] < pair[0]);
    }
    for level in &hierarchy.levels()[..hierarchy.levels().len() - 1] {
        assert!(level.repeat() >= 1);
    }
}

#[test]
fn direct_terminal_repeat_uses_unit_lower_factor_nonzeros() {
    let graph = Laplacian::from_edges(96, (0..95).map(|vertex| (vertex, vertex + 1, 1.0))).unwrap();
    let preconditioner = CmgPreconditioner::build(
        &graph,
        CmgOptions {
            direct_threshold: 20,
            ..CmgOptions::default()
        },
    )
    .unwrap();
    assert_eq!(
        preconditioner.hierarchy().report().terminal_reason(),
        TerminalReason::Direct
    );
    let levels = preconditioner.hierarchy().levels();
    assert!(levels.len() >= 2);
    let penultimate = levels.len() - 2;
    let fine_nonzeros = levels[penultimate].graph().matrix_nnz();
    let lower_factor_nonzeros = preconditioner
        .terminal_factor()
        .unwrap()
        .factor_nonzeros()
        .max(1);
    let expected = (fine_nonzeros / lower_factor_nonzeros)
        .saturating_sub(1)
        .max(1);
    assert_eq!(preconditioner.repeat_counts()[penultimate], expected);
    assert_eq!(levels[penultimate].repeat(), expected);
}

#[test]
fn fill_and_level_safety_guards_are_reported() {
    let graph = Laplacian::from_edges(24, (0..23).map(|vertex| (vertex, vertex + 1, 1.0))).unwrap();
    let fill_options = CmgOptions {
        direct_threshold: 1,
        max_hierarchy_nnz_factor: 0.5,
        ..CmgOptions::default()
    };
    assert_eq!(
        CmgHierarchy::build(&graph, fill_options)
            .unwrap()
            .report()
            .terminal_reason(),
        TerminalReason::StagnatedFill
    );

    let level_options = CmgOptions {
        direct_threshold: 1,
        max_levels: 1,
        ..CmgOptions::default()
    };
    assert_eq!(
        CmgHierarchy::build(&graph, level_options)
            .unwrap()
            .report()
            .terminal_reason(),
        TerminalReason::MaximumLevels
    );
}
