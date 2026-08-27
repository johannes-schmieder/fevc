//! Conservative pre-build estimates and exact retained-memory reports.

use crate::{CmgError, CmgOptions, ParallelOptions};
#[cfg(feature = "parallel")]
use crate::{ParallelPcgSolver, ParallelPcgWorkspace};

/// Dimensions needed for a conservative CMG memory estimate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CmgProblemSize {
    /// Finest-level graph vertices.
    pub vertices: usize,
    /// Edges submitted to graph construction before duplicate aggregation.
    pub input_edges: usize,
    /// Exact or conservative upper bound on canonical finest-level edges.
    pub canonical_edges: usize,
    /// Maximum simultaneously submitted right-hand sides.
    pub right_hand_sides: usize,
}

/// Conservative principal-heap memory estimate available before construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CmgMemoryEstimate {
    build_peak_bytes: usize,
    retained_solver_bytes: usize,
    workspace_bytes_each: usize,
    workspace_pool_bytes: usize,
    total_retained_bytes: usize,
}

impl CmgMemoryEstimate {
    /// Estimate a complete hierarchy, optional parallel plan, and reusable
    /// workspace pool with checked arithmetic.
    pub fn conservative(
        problem: CmgProblemSize,
        cmg_options: CmgOptions,
        parallel_options: ParallelOptions,
    ) -> Result<Self, CmgError> {
        let cmg_options = cmg_options.validate()?;
        let parallel_options = parallel_options.validate()?;
        if problem.vertices == 0
            || problem.right_hand_sides == 0
            || problem.canonical_edges > problem.input_edges
        {
            return Err(CmgError::InvalidHierarchy {
                context: "CMG memory-estimate dimensions are inconsistent",
            });
        }

        let usize_bytes = core::mem::size_of::<usize>();
        let initial_nnz = checked_add(
            problem.vertices,
            checked_mul(problem.canonical_edges, 2, "initial matrix nonzeros")?,
            "initial matrix nonzeros",
        )?;
        let hierarchy_nnz = checked_ceil_product(
            initial_nnz,
            cmg_options.max_hierarchy_nnz_factor,
            "hierarchy nonzeros",
        )?;
        let level_vertices = checked_mul(
            problem.vertices,
            cmg_options.max_levels,
            "hierarchy vertices",
        )?
        .min(checked_add(
            problem.vertices,
            hierarchy_nnz,
            "hierarchy vertex/nonzero bound",
        )?);

        // Edge storage, graph diagonals, smoother diagonals, compact
        // aggregation labels, reports, and conservative component metadata.
        let hierarchy_bytes = checked_sum(&[
            checked_mul(hierarchy_nnz, 8, "hierarchy sparse storage")?,
            checked_mul(level_vertices, 24, "hierarchy vertex storage")?,
            checked_mul(cmg_options.max_levels, usize_bytes * 4, "hierarchy reports")?,
        ])?;
        let direct = cmg_options.direct_threshold.min(problem.vertices);
        let terminal_factor_bytes = checked_mul(
            checked_mul(direct, direct, "terminal factor entries")?,
            16,
            "terminal factor storage",
        )?;
        let retained_solver_bytes = checked_add(
            hierarchy_bytes,
            terminal_factor_bytes,
            "retained preconditioner",
        )?;

        // A planned CSR operator can retain row offsets plus a directed copy
        // of each hierarchy nonzero. This bounds every eligible level.
        let plan_bytes = checked_sum(&[
            checked_mul(hierarchy_nnz, 16, "parallel plan sparse storage")?,
            checked_mul(level_vertices, 8, "parallel plan row storage")?,
        ])?;
        // Dense parallel row construction temporarily retains one canonical
        // edge-index vector and bounded row-count/offset vectors for one
        // operator at a time. Use hierarchy-wide bounds for a conservative
        // pre-build estimate.
        let plan_build_scratch_bytes = checked_sum(&[
            checked_mul(
                hierarchy_nnz,
                usize_bytes,
                "parallel plan edge-index scratch",
            )?,
            checked_mul(level_vertices, usize_bytes * 4, "parallel plan row scratch")?,
        ])?;
        let workspace_bytes_each = checked_sum(&[
            checked_mul(problem.vertices, 64, "PCG finest vectors")?,
            checked_mul(level_vertices, 80, "recursive CMG workspace")?,
            checked_mul(direct, 24, "terminal workspace")?,
        ])?;
        let requested_concurrency = parallel_options
            .threads
            .max(1)
            .min(problem.right_hand_sides);
        if let Some(budget_bytes) = parallel_options.workspace_memory_budget_bytes {
            if budget_bytes < workspace_bytes_each {
                return Err(CmgError::MemoryBudgetExceeded {
                    required_bytes: workspace_bytes_each,
                    budget_bytes,
                });
            }
        }
        let budget_concurrency = parallel_options
            .workspace_memory_budget_bytes
            .map_or(requested_concurrency, |budget| {
                budget / workspace_bytes_each.max(1)
            });
        let concurrency = requested_concurrency.min(budget_concurrency);
        let workspace_pool_bytes =
            checked_mul(workspace_bytes_each, concurrency, "CMG workspace pool")?;
        let total_retained_bytes =
            checked_sum(&[retained_solver_bytes, plan_bytes, workspace_pool_bytes])?;

        // Graph construction first retains all submitted edges, then the
        // canonical graph while hierarchy construction is live.
        let raw_input_bytes = checked_mul(problem.input_edges, 16, "raw input edges")?;
        let build_peak_bytes = checked_sum(&[
            raw_input_bytes,
            retained_solver_bytes,
            plan_bytes,
            plan_build_scratch_bytes,
            workspace_pool_bytes,
        ])?;
        Ok(Self {
            build_peak_bytes,
            retained_solver_bytes,
            workspace_bytes_each,
            workspace_pool_bytes,
            total_retained_bytes,
        })
    }

    /// Return conservative peak bytes during hierarchy and plan construction.
    #[must_use]
    pub const fn build_peak_bytes(self) -> usize {
        self.build_peak_bytes
    }

    /// Return conservative immutable solver bytes after construction.
    #[must_use]
    pub const fn retained_solver_bytes(self) -> usize {
        self.retained_solver_bytes
    }

    /// Return conservative bytes required by one PCG workspace.
    #[must_use]
    pub const fn workspace_bytes_each(self) -> usize {
        self.workspace_bytes_each
    }

    /// Return conservative retained bytes for the reusable workspace pool.
    #[must_use]
    pub const fn workspace_pool_bytes(self) -> usize {
        self.workspace_pool_bytes
    }

    /// Return conservative complete retained bytes after construction.
    #[must_use]
    pub const fn total_retained_bytes(self) -> usize {
        self.total_retained_bytes
    }
}

/// Exact principal retained heap bytes after solver/workspace construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CmgMemoryReport {
    preconditioner_bytes: usize,
    parallel_plan_bytes: usize,
    workspace_bytes_each: usize,
    workspace_pool_bytes: usize,
    total_retained_bytes: usize,
}

impl CmgMemoryReport {
    #[cfg(feature = "parallel")]
    pub(crate) fn new(solver: &ParallelPcgSolver, workspace: &ParallelPcgWorkspace) -> Self {
        let preconditioner_bytes = solver.preconditioner().retained_bytes();
        let parallel_plan_bytes = solver.initialized_plan_bytes();
        let workspace_bytes_each = solver.workspace_bytes();
        let workspace_pool_bytes = workspace.byte_len();
        let total_retained_bytes = preconditioner_bytes
            .saturating_add(parallel_plan_bytes)
            .saturating_add(workspace_pool_bytes);
        Self {
            preconditioner_bytes,
            parallel_plan_bytes,
            workspace_bytes_each,
            workspace_pool_bytes,
            total_retained_bytes,
        }
    }

    /// Return exact immutable preconditioner bytes.
    #[must_use]
    pub const fn preconditioner_bytes(self) -> usize {
        self.preconditioner_bytes
    }

    /// Return exact bytes retained by an initialized parallel plan.
    #[must_use]
    pub const fn parallel_plan_bytes(self) -> usize {
        self.parallel_plan_bytes
    }

    /// Return exact bytes required by one PCG workspace.
    #[must_use]
    pub const fn workspace_bytes_each(self) -> usize {
        self.workspace_bytes_each
    }

    /// Return exact retained bytes in the supplied reusable workspace pool.
    #[must_use]
    pub const fn workspace_pool_bytes(self) -> usize {
        self.workspace_pool_bytes
    }

    /// Return exact complete retained bytes for solver, plan, and workspaces.
    #[must_use]
    pub const fn total_retained_bytes(self) -> usize {
        self.total_retained_bytes
    }
}

fn checked_ceil_product(
    value: usize,
    factor: f64,
    context: &'static str,
) -> Result<usize, CmgError> {
    let product = (value as f64) * factor;
    if !product.is_finite() || product > usize::MAX as f64 {
        return Err(CmgError::InvalidHierarchy { context });
    }
    Ok(product.ceil() as usize)
}

fn checked_mul(left: usize, right: usize, context: &'static str) -> Result<usize, CmgError> {
    left.checked_mul(right)
        .ok_or(CmgError::InvalidHierarchy { context })
}

fn checked_add(left: usize, right: usize, context: &'static str) -> Result<usize, CmgError> {
    left.checked_add(right)
        .ok_or(CmgError::InvalidHierarchy { context })
}

fn checked_sum(values: &[usize]) -> Result<usize, CmgError> {
    values.iter().try_fold(0_usize, |total, value| {
        total.checked_add(*value).ok_or(CmgError::InvalidHierarchy {
            context: "CMG memory estimate overflows",
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conservative_estimate_is_monotone_and_checked() {
        let small = CmgMemoryEstimate::conservative(
            CmgProblemSize {
                vertices: 1_000,
                input_edges: 5_000,
                canonical_edges: 2_000,
                right_hand_sides: 2,
            },
            CmgOptions::default(),
            ParallelOptions {
                threads: 2,
                ..ParallelOptions::default()
            },
        )
        .unwrap();
        let large = CmgMemoryEstimate::conservative(
            CmgProblemSize {
                vertices: 2_000,
                input_edges: 10_000,
                canonical_edges: 4_000,
                right_hand_sides: 4,
            },
            CmgOptions::default(),
            ParallelOptions {
                threads: 4,
                ..ParallelOptions::default()
            },
        )
        .unwrap();
        assert!(large.build_peak_bytes() > small.build_peak_bytes());
        assert!(large.total_retained_bytes() > small.total_retained_bytes());
        assert!(small.build_peak_bytes() >= small.total_retained_bytes());

        assert!(
            CmgMemoryEstimate::conservative(
                CmgProblemSize {
                    vertices: usize::MAX,
                    input_edges: usize::MAX,
                    canonical_edges: usize::MAX,
                    right_hand_sides: 1,
                },
                CmgOptions::default(),
                ParallelOptions::default(),
            )
            .is_err()
        );
    }
}
