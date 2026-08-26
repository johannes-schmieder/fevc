#![cfg(feature = "profiling")]

use cmg::{
    CmgOptions, CmgPreconditioner, Laplacian, ParallelCmgPlan, ParallelExecutor, ParallelOptions,
};

#[test]
fn profiled_setup_preserves_production_objects() {
    let graph = Laplacian::from_edges(
        20_000,
        (0..19_999)
            .map(|vertex| (vertex, vertex + 1, 1.0))
            .chain((0..19_998).map(|vertex| (vertex, vertex + 2, 0.5))),
    )
    .unwrap();
    let executor = ParallelExecutor::new(ParallelOptions {
        threads: 4,
        min_parallel_len: 1_024,
        ..ParallelOptions::default()
    })
    .unwrap();
    let production =
        CmgPreconditioner::build_with_executor(&graph, CmgOptions::default(), &executor).unwrap();
    let (profiled, setup_profile) =
        CmgPreconditioner::build_with_executor_profiled(&graph, CmgOptions::default(), &executor)
            .unwrap();
    assert_eq!(profiled, production);
    assert!(setup_profile.total_nanoseconds() >= setup_profile.hierarchy_nanoseconds());
    assert!(setup_profile.total_nanoseconds() >= setup_profile.finalization_nanoseconds());
    assert!(!setup_profile.hierarchy_phases().is_empty());
    assert!(
        setup_profile.hierarchy_nanoseconds()
            >= setup_profile
                .hierarchy_phases()
                .iter()
                .map(|phase| phase.nanoseconds())
                .sum::<u128>()
    );

    let plan = ParallelCmgPlan::build(&production, &executor).unwrap();
    let (profiled_plan, plan_profile) =
        ParallelCmgPlan::build_profiled(&production, &executor).unwrap();
    assert_eq!(profiled_plan.operator_count(), plan.operator_count());
    assert_eq!(profiled_plan.byte_len(), plan.byte_len());
    assert_eq!(
        plan_profile.levels().len(),
        production.hierarchy().levels().len()
    );
    assert_eq!(
        plan_profile
            .levels()
            .iter()
            .filter(|level| level.eligible())
            .count(),
        plan.operator_count()
    );
    for level in plan_profile
        .levels()
        .iter()
        .filter(|level| level.eligible())
    {
        let measured = level
            .row_counts_nanoseconds()
            .saturating_add(level.row_offsets_nanoseconds())
            .saturating_add(level.allocation_nanoseconds())
            .saturating_add(level.scatter_nanoseconds())
            .saturating_add(level.validation_nanoseconds());
        assert!(measured > 0);
        assert!(level.construction_nanoseconds() >= measured);
    }
}
