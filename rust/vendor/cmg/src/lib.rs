//! Deterministic Rust port of the Combinatorial Multigrid (CMG)
//! preconditioner.
//!
//! The implementation is developed against the pinned upstream source recorded
//! in `UPSTREAM.md`. Numerical modules are added in recoverable checkpoints; the
//! live status is maintained in `PLAN.md`.

#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![allow(clippy::needless_range_loop, clippy::too_many_arguments)]

mod cancel;
mod coarsen;
mod components;
mod csr;
mod error;
mod execution;
mod forest;
mod graph;
mod hierarchy;
mod ldl;
mod memory;
mod options;
#[cfg(feature = "parallel")]
mod parallel_solver;
mod pcg;
#[cfg(feature = "profiling")]
mod pcg_profile;
mod preconditioner;
mod sddm;
mod sddm_solver;
#[cfg(feature = "parallel")]
mod vckss_bridge;
mod workspace;

pub use coarsen::Aggregation;
pub use components::Components;
pub use csr::CsrLaplacian;
pub use error::CmgError;
#[cfg(feature = "parallel")]
pub use execution::ParallelExecutor;
pub use execution::ParallelOptions;
pub use forest::{
    ForestGrouping, build_forest_grouping, forest_components, maximum_weight_forest, split_forest,
};
#[cfg(feature = "parallel")]
pub use forest::{build_forest_grouping_with_executor, maximum_weight_forest_with_executor};
pub use graph::{Edge, Laplacian};
pub use hierarchy::{CmgHierarchy, HierarchyBuildReport, HierarchyLevel, TerminalReason};
pub use ldl::GroundedLdl;
pub use memory::{CmgMemoryEstimate, CmgMemoryReport, CmgProblemSize};
pub use options::{CmgOptions, PcgOptions, ValidationOptions};
#[cfg(feature = "parallel")]
pub use parallel_solver::{
    DEFAULT_MIN_PLANNED_EDGES, ParallelPcgBatchReport, ParallelPcgBatchResult,
    ParallelPcgExecution, ParallelPcgPolicy, ParallelPcgSolver, ParallelPcgWorkspace,
};
pub use pcg::{PcgResult, PcgWorkspace, solve_pcg, solve_pcg_batch, solve_pcg_with_workspace};
#[cfg(feature = "parallel")]
pub use pcg::{
    solve_pcg_batch_parallel, solve_pcg_batch_with_executor, solve_pcg_with_plan,
    solve_pcg_with_plan_and_initial_guess_and_workspace,
    solve_pcg_with_plan_and_initial_guess_and_workspace_cancellable,
    solve_pcg_with_plan_and_workspace, solve_pcg_with_plan_and_workspace_cancellable,
};
pub use pcg::{
    solve_pcg_with_initial_guess_and_workspace,
    solve_pcg_with_initial_guess_and_workspace_cancellable, solve_pcg_with_workspace_cancellable,
};
#[cfg(feature = "profiling")]
pub use pcg_profile::{PcgPhaseProfile, PcgPhaseSample, ProfiledPcgResult, profile_pcg_with_plan};
pub use preconditioner::CmgPreconditioner;
#[cfg(feature = "parallel")]
pub use preconditioner::ParallelCmgPlan;
#[cfg(feature = "profiling")]
pub use preconditioner::{
    HierarchyPhaseProfile, ParallelPlanBuildProfile, ParallelPlanLevelProfile,
    PreconditionerBuildProfile,
};
pub use sddm::{SddmAugmentation, SddmMatrix};
pub use sddm_solver::{SddmResult, SddmSolver, SddmWorkspace, solve_sddm};
#[cfg(feature = "parallel")]
pub use vckss_bridge::VckssContiguousPcgWorkspace;
pub use workspace::CmgWorkspace;

/// Return the pinned upstream CMG commit used as the behavioral reference.
#[must_use]
pub const fn upstream_commit() -> &'static str {
    "19752fc102f8cae8e34f66457bfaccb1aaa60375"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upstream_pin_is_stable() {
        assert_eq!(
            upstream_commit(),
            "19752fc102f8cae8e34f66457bfaccb1aaa60375"
        );
    }
}
