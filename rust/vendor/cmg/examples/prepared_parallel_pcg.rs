#[cfg(feature = "parallel")]
use cmg::{
    CmgOptions, Laplacian, ParallelExecutor, ParallelOptions, ParallelPcgSolver, PcgOptions,
};

#[cfg(feature = "parallel")]
fn compatible_rhs(graph: &Laplacian, rhs_index: usize) -> Result<Vec<f64>, cmg::CmgError> {
    let target: Vec<f64> = (0..graph.vertex_count())
        .map(|vertex| {
            let first = ((17 * vertex + 31 * rhs_index) % 257) as f64 - 128.0;
            let second = ((43 * vertex + 19 * rhs_index) % 101) as f64 - 50.0;
            first / 37.0 + second / 113.0
        })
        .collect();
    graph.matvec(&target)
}

#[cfg(feature = "parallel")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // A bipartite worker--firm graph with three weighted edges per worker.
    let per_side = 25_000;
    let firm_offset = per_side;
    let mut edges = Vec::with_capacity(3 * per_side);
    for worker in 0..per_side {
        edges.push((worker, firm_offset + worker, 1.0));
        edges.push((worker, firm_offset + (worker + 1) % per_side, 0.75));
        edges.push((worker, firm_offset + (5 * worker + 17) % per_side, 1.25));
    }
    let graph = Laplacian::from_edges(2 * per_side, edges)?;
    let right_hand_sides: Vec<Vec<f64>> = (0..4)
        .map(|index| compatible_rhs(&graph, index))
        .collect::<Result<_, _>>()?;

    // The application owns one pool; CMG borrows it as a team and never
    // creates its own. Build once and reuse the hierarchy, optional
    // within-solve plan and bounded workspace pool for all later solves.
    let executor = ParallelExecutor::new(ParallelOptions {
        threads: 0, // detect available CPUs; use an explicit cap if desired
        workspace_memory_budget_bytes: Some(512 * 1024 * 1024),
        ..ParallelOptions::default()
    })?;
    let team = executor.team();
    let solver = ParallelPcgSolver::build(&graph, CmgOptions::default(), team)?;
    let mut workspace = solver.workspace();
    let batch = solver.solve_batch_with_workspace(
        &right_hand_sides,
        PcgOptions::default(),
        &mut workspace,
        team,
    )?;
    let report = batch.report();

    println!("threads: {}", team.worker_ceiling());
    println!("execution: {:?}", report.execution());
    println!("concurrency: {}", report.concurrency());
    println!("parallel-plan bytes: {}", report.plan_bytes());
    println!("workspace-pool bytes: {}", report.workspace_pool_bytes());
    for (index, result) in batch.results().iter().enumerate() {
        println!(
            "rhs {index}: iterations={}, backward_error={:.3e}",
            result.iterations(),
            result.backward_error(),
        );
    }
    Ok(())
}

#[cfg(not(feature = "parallel"))]
fn main() {
    eprintln!("run with: cargo run --release --example prepared_parallel_pcg --features parallel");
}
