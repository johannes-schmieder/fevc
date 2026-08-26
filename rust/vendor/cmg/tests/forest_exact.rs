use cmg::{
    Laplacian, build_forest_grouping, forest_components, maximum_weight_forest, split_forest,
};

#[test]
fn maximum_weight_forest_uses_deterministic_lowest_neighbor_ties() {
    let graph = Laplacian::from_edges(3, [(0, 1, 4.0), (0, 2, 4.0), (1, 2, 5.0)]).unwrap();
    let (parent, weight) = maximum_weight_forest(&graph);
    assert_eq!(parent, [1, 2, 1]);
    assert_eq!(weight, [4.0, 5.0, 5.0]);
}

#[test]
fn split_forest_matches_pinned_c_kernel_on_long_path_parent_vector() {
    let parent = [1, 0, 1, 2, 3, 4, 5, 6];
    assert_eq!(split_forest(&parent).unwrap(), [1, 0, 1, 2, 4, 4, 5, 6]);
}

#[test]
fn forest_components_are_order_stable_and_include_singletons() {
    let parent = [1, 0, 2, 4, 3, 5];
    let (labels, sizes) = forest_components(&parent).unwrap();
    assert_eq!(labels, [0, 0, 1, 2, 2, 3]);
    assert_eq!(sizes, [2, 1, 2, 1]);
}

#[test]
fn low_effective_degree_correction_splits_equal_weight_clique() {
    let mut edges = Vec::new();
    for left in 0..10 {
        for right in (left + 1)..10 {
            edges.push((left, right, 1.0));
        }
    }
    let graph = Laplacian::from_edges(10, edges).unwrap();
    let grouping = build_forest_grouping(&graph, 0.125).unwrap();
    assert_eq!(grouping.heavy_parent(), [1, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    assert_eq!(grouping.final_parent(), [1, 0, 2, 3, 4, 5, 6, 7, 8, 9]);
    assert_eq!(grouping.sizes(), [2, 1, 1, 1, 1, 1, 1, 1, 1]);
}

#[test]
fn isolated_vertices_remain_valid_singleton_aggregates() {
    let graph = Laplacian::from_edges(4, [(0, 1, 2.0)]).unwrap();
    let grouping = build_forest_grouping(&graph, 0.125).unwrap();
    assert_eq!(grouping.final_parent(), [1, 0, 2, 3]);
    assert_eq!(grouping.labels(), [0, 0, 1, 2]);
    assert_eq!(grouping.sizes(), [2, 1, 1]);
}
