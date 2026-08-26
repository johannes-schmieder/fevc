#![cfg(feature = "parallel")]

use cmg::{DEFAULT_MIN_PLANNED_EDGES, ParallelPcgPolicy};

#[test]
fn default_planned_threshold_matches_qualified_full_pcg_crossover() {
    assert_eq!(DEFAULT_MIN_PLANNED_EDGES, 350_000);
    assert_eq!(
        ParallelPcgPolicy::default().min_planned_edges,
        DEFAULT_MIN_PLANNED_EDGES
    );
}
