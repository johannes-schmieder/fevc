#![cfg(feature = "parallel")]

use cmg::{
    CmgMemoryEstimate, CmgOptions, CmgProblemSize, Laplacian, ParallelOptions, ParallelPcgSolver,
};

#[test]
fn exact_report_is_bounded_by_the_conservative_estimate() {
    let vertices = 2_000;
    let input: Vec<_> = (0..20_000)
        .map(|index| {
            let left = index % 1_000;
            let right = 1_000 + ((index * 37 + index / 11) % 1_000);
            (left, right, 1.0 + (index % 7) as f64 / 10.0)
        })
        .collect();
    let graph = Laplacian::from_edges(vertices, input.iter().copied()).unwrap();
    let parallel = ParallelOptions {
        threads: 4,
        ..ParallelOptions::default()
    };
    let estimate = CmgMemoryEstimate::conservative(
        CmgProblemSize {
            vertices,
            input_edges: input.len(),
            canonical_edges: graph.edge_count(),
            right_hand_sides: 4,
        },
        CmgOptions::default(),
        parallel,
    )
    .unwrap();
    let solver = ParallelPcgSolver::build(&graph, CmgOptions::default(), parallel).unwrap();
    let workspace = solver.workspace();
    let report = solver.memory_report(&workspace);
    assert!(report.preconditioner_bytes() > 0);
    assert!(report.workspace_bytes_each() > 0);
    assert!(report.total_retained_bytes() <= estimate.total_retained_bytes());
    assert!(report.total_retained_bytes() <= estimate.build_peak_bytes());
}
