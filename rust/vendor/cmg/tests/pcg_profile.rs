#![cfg(feature = "profiling")]

use cmg::{
    CmgOptions, CmgPreconditioner, Laplacian, ParallelCmgPlan, ParallelExecutor, ParallelOptions,
    PcgOptions, profile_pcg_with_plan, solve_pcg_with_plan,
};

fn worker_firm_graph(per_side: usize) -> Laplacian {
    let firm_offset = per_side;
    let mut edges = Vec::with_capacity(3 * per_side);
    for worker in 0..per_side {
        for (link, firm) in [
            worker,
            (worker + 1) % per_side,
            (5 * worker + 17) % per_side,
        ]
        .into_iter()
        .enumerate()
        {
            let weight = 0.5 + ((worker + 3 * link) % 19) as f64 / 11.0;
            edges.push((worker, firm_offset + firm, weight));
        }
    }
    Laplacian::from_edges(2 * per_side, edges).unwrap()
}

#[test]
fn profiled_planned_pcg_matches_production_bit_for_bit() {
    let graph = worker_firm_graph(400);
    let executor = ParallelExecutor::new(ParallelOptions {
        threads: 2,
        min_parallel_len: 1,
        ..ParallelOptions::default()
    })
    .unwrap();
    let preconditioner = CmgPreconditioner::build_with_executor(
        &graph,
        CmgOptions {
            direct_threshold: 32,
            ..CmgOptions::default()
        },
        &executor,
    )
    .unwrap();
    let plan = ParallelCmgPlan::build(&preconditioner, &executor).unwrap();
    assert!(plan.operator_count() > 0);

    let mut target: Vec<f64> = (0..graph.vertex_count())
        .map(|index| ((index % 37) as f64 - 18.0) / 7.0)
        .collect();
    let mean = target.iter().sum::<f64>() / target.len() as f64;
    for value in &mut target {
        *value -= mean;
    }
    let rhs = graph.matvec(&target).unwrap();
    let options = PcgOptions {
        residual_recompute_interval: 5,
        ..PcgOptions::default()
    };

    let production =
        solve_pcg_with_plan(&graph, &preconditioner, &plan, &rhs, options, &executor).unwrap();
    let profiled =
        profile_pcg_with_plan(&graph, &preconditioner, &plan, &rhs, options, &executor).unwrap();

    assert_eq!(production.iterations(), profiled.iterations());
    assert_eq!(
        production.residual_norm().to_bits(),
        profiled.residual_norm().to_bits()
    );
    assert_eq!(
        production.backward_error().to_bits(),
        profiled.backward_error().to_bits()
    );
    assert_eq!(production.restarts(), profiled.restarts());
    for (&expected, &actual) in production.solution().iter().zip(profiled.solution()) {
        assert_eq!(expected.to_bits(), actual.to_bits());
    }

    let profile = profiled.profile();
    assert!(profile.total_nanoseconds() >= profile.attributed_nanoseconds());
    assert!(profile.preconditioner().calls() > 0);
    assert!(profile.matvec().calls() > 0);
    assert!(profile.dot_products().calls() > 0);
    assert!(profile.vector_updates().calls() > 0);
    assert!(profile.centering().calls() > 0);
    assert!(profile.norms().calls() > 0);
    assert!(profile.certification().calls() > 0);
}
