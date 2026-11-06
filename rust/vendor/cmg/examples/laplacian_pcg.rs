use cmg::{CmgOptions, CmgPreconditioner, Laplacian, PcgOptions, solve_pcg};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Weighted path 0 --(1)-- 1 --(1)-- 2.
    let graph = Laplacian::from_edges(3, [(0, 1, 1.0), (1, 2, 1.0)])?;

    // A Laplacian right-hand side must sum to zero on each connected component.
    let rhs = [1.0, 0.0, -1.0];

    // Build once and reuse for every right-hand side on this graph.
    let preconditioner = CmgPreconditioner::build(&graph, CmgOptions::default())?;
    let result = solve_pcg(&graph, &preconditioner, &rhs, PcgOptions::default())?;

    println!("solution: {:?}", result.solution());
    println!("iterations: {}", result.iterations());
    println!("backward error: {:.3e}", result.backward_error());

    // The quotient-space normalization gives the zero-mean solution [1, 0, -1].
    for (actual, expected) in result.solution().iter().zip([1.0, 0.0, -1.0]) {
        assert!((actual - expected).abs() <= 1.0e-10);
    }
    Ok(())
}
