#[cfg(feature = "parallel")]
use cmg::{
    CmgOptions, CmgPreconditioner, Laplacian, ParallelCmgPlan, ParallelExecutor, ParallelOptions,
    PcgOptions, PcgWorkspace,
};

#[cfg(feature = "parallel")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // A sparse bipartite worker--firm graph with three edges per worker.
    let per_side = 10_000;
    let firm_offset = per_side;
    let mut edges = Vec::with_capacity(3 * per_side);
    for worker in 0..per_side {
        edges.push((worker, firm_offset + worker, 1.0));
        edges.push((worker, firm_offset + (worker + 1) % per_side, 0.75));
        edges.push((worker, firm_offset + (5 * worker + 17) % per_side, 1.25));
    }
    let graph = Laplacian::from_edges(2 * per_side, edges)?;

    // Construct a compatible RHS as b = L x_star.
    let target: Vec<f64> = (0..graph.vertex_count())
        .map(|vertex| (vertex % 127) as f64 / 23.0 - 2.0)
        .collect();
    let rhs = graph.matvec(&target)?;

    // Build the hierarchy once. The optional plan retains row-oriented
    // operators only on hierarchy levels where parallel execution is expected
    // to pay for its extra memory.
    let preconditioner = CmgPreconditioner::build(&graph, CmgOptions::default())?;
    let executor = ParallelExecutor::new(ParallelOptions {
        threads: 0, // detect available parallelism; set an explicit cap if desired
        ..ParallelOptions::default()
    })?;
    let plan = ParallelCmgPlan::build(&preconditioner, executor.team())?;
    let mut workspace = PcgWorkspace::new(&preconditioner);

    // Reuse the same executor, plan, and workspace for subsequent right-hand
    // sides on the unchanged weighted graph.
    let result = cmg::solve_pcg_with(
        &graph,
        &preconditioner,
        &rhs,
        PcgOptions::default(),
        &mut workspace,
        cmg::SolveContext {
            plan: Some(&plan),
            team: Some(executor.team()),
            ..cmg::SolveContext::default()
        },
    )?;

    println!("threads: {}", executor.thread_count());
    println!("parallel hierarchy operators: {}", plan.operator_count());
    println!("plan bytes: {}", plan.byte_len());
    println!("iterations: {}", result.iterations());
    println!("backward error: {:.3e}", result.backward_error());
    Ok(())
}

#[cfg(not(feature = "parallel"))]
fn main() {
    eprintln!("run with: cargo run --example parallel_pcg --features parallel");
}
