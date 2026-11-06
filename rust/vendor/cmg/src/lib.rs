//! Deterministic Rust port of the Combinatorial Multigrid (CMG)
//! preconditioner.
//!
//! The implementation is developed against the pinned upstream source recorded
//! in `docs/UPSTREAM.md`.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

#[cfg(all(test, feature = "parallel"))]
mod bounded_team_tests;
mod build;
mod cancel;
mod cmg_profile;
mod coarsen;
#[cfg(feature = "experimental-components")]
mod component_experiment;
mod components;
mod csr;
mod error;
mod execution;
#[cfg(feature = "parallel")]
mod execution_stage;
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
mod prepared;
mod sddm;
mod sddm_solver;
mod storage;
mod workspace;

pub use build::{BuildContext, BuildOptions};
pub use cancel::Cancellation;
#[cfg(feature = "cycle-profiling")]
pub use cmg_profile::{CmgApplyLevelProfile, CmgApplyPhase, CmgApplyProfile};
pub use coarsen::Aggregation;
#[cfg(feature = "experimental-components")]
pub use component_experiment::{ComponentBuildOptions, PrunedTransfer};
pub use components::Components;
pub use csr::CsrLaplacian;
pub use error::CmgError;
pub use execution::ParallelOptions;
#[cfg(feature = "parallel")]
pub use execution::{ParallelExecutor, ParallelRuntimeAllowance, ParallelRuntimeEstimate, Team};
#[cfg(feature = "parallel")]
pub use execution_stage::{AdmittedStage, ControlledStage};
pub use forest::{
    ForestGrouping, build_forest_grouping, forest_components, maximum_weight_forest, split_forest,
};
pub use graph::{Edge, Laplacian};
pub use hierarchy::{CmgHierarchy, HierarchyBuildReport, HierarchyLevel, TerminalReason};
pub use ldl::GroundedLdl;
pub use memory::{
    CmgMemoryEstimate, CmgMemoryReport, CmgProblemSize, RepeatedPcgMemoryEstimate,
    RepeatedPcgMemoryReport,
};
pub use options::{CmgOptions, PcgOptions, ValidationOptions};
#[cfg(feature = "parallel")]
pub use parallel_solver::{
    DEFAULT_MIN_PLANNED_EDGES, ParallelPcgBatchReport, ParallelPcgBatchResult,
    ParallelPcgExecution, ParallelPcgPolicy, ParallelPcgSolver, ParallelPcgWorkspace,
};
#[cfg(feature = "parallel")]
pub use pcg::solve_pcg_batch_concurrent;
pub use pcg::{
    PcgBatchMut, PcgBatchRef, PcgBatchWorkspace, PcgDiagnostics, PcgResult, PcgWorkspace,
    SolveContext, solve_pcg, solve_pcg_batch, solve_pcg_batch_into, solve_pcg_into, solve_pcg_with,
};
#[cfg(feature = "profiling")]
pub use pcg::{PcgBatchPhaseProfile, profile_pcg_batch_into};
#[cfg(feature = "profiling")]
pub use pcg_profile::{PcgPhaseProfile, PcgPhaseSample, ProfiledPcgResult, profile_pcg};
#[cfg(feature = "parallel")]
pub use preconditioner::ParallelCmgPlan;
pub use preconditioner::{ApplyContext, CmgBatchWorkspace, CmgPreconditioner};
#[cfg(feature = "profiling")]
pub use preconditioner::{
    HierarchyPhaseProfile, ParallelPlanBuildProfile, ParallelPlanLevelProfile,
    PreconditionerBuildProfile,
};
pub use prepared::{PreparedLaplacianTopology, PreparedLaplacianWorkspace};
pub use sddm::{SddmAugmentation, SddmMatrix};
pub use sddm_solver::{SddmResult, SddmSolver, SddmWorkspace, solve_sddm};
pub use storage::{CmgStorage, CmgStorageReport};
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
