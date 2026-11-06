//! Conservative pre-build estimates and exact retained-memory reports.

use crate::{CmgError, CmgOptions, ParallelOptions};
use crate::{
    CmgPreconditioner, Laplacian, PcgBatchWorkspace, PcgDiagnostics, PreparedLaplacianTopology,
    PreparedLaplacianWorkspace,
};
#[cfg(feature = "parallel")]
use crate::{ParallelCmgPlan, ParallelPcgSolver, ParallelPcgWorkspace};

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
    /// Principal-array estimate for explicit component setup. The retained
    /// factor and workspace bounds cover all component option combinations;
    /// transient ordered-sparse factor construction is not whole-build admitted.
    #[cfg(feature = "experimental-components")]
    pub fn conservative_component(
        problem: CmgProblemSize,
        options: CmgOptions,
        parallel: ParallelOptions,
        _component: crate::ComponentBuildOptions,
    ) -> Result<Self, CmgError> {
        Self::conservative(problem, options, parallel)
    }

    /// A complete CMG-owned construction peak is not yet proven. Principal
    /// arrays omit setup/container overlaps, and parallel runtime queues have
    /// no admitted bound. Never treat `build_peak_bytes` as a whole-action cap.
    pub const fn complete_build_peak_bytes(self) -> Option<usize> {
        None
    }

    /// Reject a whole-build budget until complete coverage is available. This
    /// fails before allocation and never substitutes a sampled multiplier.
    pub fn admit_complete_build(self, budget_bytes: usize) -> Result<(), CmgError> {
        match self.complete_build_peak_bytes() {
            Some(bytes) => crate::storage::admit(bytes, budget_bytes),
            None => Err(CmgError::InvalidHierarchy {
                context: "complete CMG construction memory coverage is unavailable",
            }),
        }
    }

    /// Estimate principal arrays for the hierarchy, optional parallel plan and
    /// reusable workspace pool with checked arithmetic. This excludes complete
    /// descriptor/Arc/runtime storage and construction overlap; it is not
    /// whole-action admission. See `complete_build_peak_bytes`.
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
        // The guard checks *after* including a level. A terminal can also be
        // reached before that check. In either case one additional level, at
        // most as large as the finest, must be covered (see ownership proof).
        let level_nnz_bound = checked_mul(
            initial_nnz,
            cmg_options.max_levels,
            "hierarchy level nonzeros",
        )?;
        let fill_nnz_bound = checked_ceil_product(
            initial_nnz,
            cmg_options.max_hierarchy_nnz_factor,
            "hierarchy nonzeros",
        )
        .and_then(|limit| checked_add(limit, initial_nnz, "hierarchy crossing level"));
        // A very large valid fill factor may overflow the floating threshold;
        // the independent checked level bound still applies.
        let hierarchy_nnz =
            fill_nnz_bound.map_or(level_nnz_bound, |fill| fill.min(level_nnz_bound));
        let level_vertices = checked_mul(
            problem.vertices,
            cmg_options.max_levels,
            "hierarchy vertices",
        )?;

        // Edge storage, graph diagonals, smoother diagonals, compact
        // aggregation labels, reports, and conservative component metadata.
        let hierarchy_bytes = checked_sum(&[
            checked_mul(hierarchy_nnz, 8, "hierarchy sparse storage")?,
            checked_mul(
                level_vertices,
                8 + 5 * usize_bytes,
                "hierarchy vertex storage",
            )?,
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
            checked_mul(
                problem.vertices,
                152 + 2 * usize_bytes,
                "PCG finest vectors and component scratch",
            )?,
            checked_mul(level_vertices, 48, "recursive CMG workspace")?,
            checked_mul(direct, 24, "terminal workspace")?,
        ])?;
        let requested_concurrency = parallel_options
            .resolved_threads()
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

    /// Return the legacy principal-array construction estimate, not a complete
    /// hierarchy/plan/action peak. Unmodeled temporaries, containers and runtime
    /// storage require separate coverage; see `complete_build_peak_bytes`.
    #[must_use]
    pub const fn build_peak_bytes(self) -> usize {
        self.build_peak_bytes
    }

    /// Return conservative principal-array immutable solver bytes, excluding
    /// full container/Arc/runtime ownership.
    #[must_use]
    pub const fn retained_solver_bytes(self) -> usize {
        self.retained_solver_bytes
    }

    /// Return conservative principal-array bytes for one PCG workspace, not all
    /// nested container capacities. Use `PcgWorkspace::allocation_bytes` for the
    /// scoped requested-layout query.
    #[must_use]
    pub const fn workspace_bytes_each(self) -> usize {
        self.workspace_bytes_each
    }

    /// Return conservative principal-array bytes for the reusable workspace
    /// pool, excluding outer/nested descriptors and coexistence during growth.
    #[must_use]
    pub const fn workspace_pool_bytes(self) -> usize {
        self.workspace_pool_bytes
    }

    /// Return the sum of retained principal-array estimates, not complete owned
    /// storage. Use `CmgStorageReport` for the covered live capacity union.
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

    /// Return the immutable preconditioner's principal-array report, excluding
    /// full container/Arc/runtime ownership.
    #[must_use]
    pub const fn preconditioner_bytes(self) -> usize {
        self.preconditioner_bytes
    }

    /// Return principal-array bytes retained by an initialized parallel plan,
    /// excluding descriptors, shared Arc blocks and any spare capacity.
    #[must_use]
    pub const fn parallel_plan_bytes(self) -> usize {
        self.parallel_plan_bytes
    }

    /// Return principal-array bytes required by one PCG workspace, excluding
    /// nested descriptor banks and spare capacity.
    #[must_use]
    pub const fn workspace_bytes_each(self) -> usize {
        self.workspace_bytes_each
    }

    /// Return principal-array bytes in the supplied reusable workspace pool,
    /// excluding nested/outer descriptor banks and spare capacity.
    #[must_use]
    pub const fn workspace_pool_bytes(self) -> usize {
        self.workspace_pool_bytes
    }

    /// Return the sum of principal-array reports for solver, plan and workspaces,
    /// not complete retained ownership. Use `CmgStorageReport` for the covered
    /// live capacity union and retain its explicit runtime exclusion.
    #[must_use]
    pub const fn total_retained_bytes(self) -> usize {
        self.total_retained_bytes
    }
}

/// Conservative principal-array estimate for repeated prepared-topology PCG.
/// Not a complete construction, retained ownership or runtime admission bound.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepeatedPcgMemoryEstimate {
    prepared_topology_bytes: usize,
    shared_component_metadata_bytes: usize,
    numeric_assembly_scratch_bytes: usize,
    current_numeric_graph_bytes: usize,
    retained_preconditioner_bytes: usize,
    parallel_plan_bytes: usize,
    workspace_bytes_each: usize,
    workspace_pool_bytes: usize,
    total_solver_retained_bytes: usize,
    caller_logical_bytes: usize,
    build_peak_bytes: usize,
}

impl RepeatedPcgMemoryEstimate {
    /// Estimate prepared topology, current numeric state, a retained hierarchy,
    /// an optional plan, and caller-owned batch workspaces with checked arithmetic.
    pub fn conservative(
        problem: CmgProblemSize,
        cmg_options: CmgOptions,
        parallel_options: ParallelOptions,
        retain_parallel_plan: bool,
        include_initial_guesses: bool,
    ) -> Result<Self, CmgError> {
        let parallel_options = parallel_options.validate()?;
        let unbounded_options = ParallelOptions {
            workspace_memory_budget_bytes: None,
            ..parallel_options
        };
        let legacy = CmgMemoryEstimate::conservative(problem, cmg_options, unbounded_options)?;
        let usize_bytes = core::mem::size_of::<usize>();
        let shared_component_metadata_bytes = checked_mul(
            problem.vertices,
            usize_bytes * 2,
            "prepared shared components",
        )?;
        // The legacy hierarchy bound already includes conservative component
        // metadata. Count only the topology-owned maps here so the shared
        // finest components are not included twice in the combined total.
        // prepare() reserves keys and offsets at the *input* count and keeps
        // that capacity even when duplicates collapse to a single edge.
        let prepared_topology_bytes = checked_sum(&[
            checked_mul(problem.input_edges, 8, "prepared canonical keys")?,
            checked_mul(
                checked_add(problem.input_edges, 1, "prepared group offsets")?,
                usize_bytes,
                "prepared group offsets",
            )?,
            checked_mul(problem.input_edges, usize_bytes * 2, "prepared input maps")?,
        ])?;
        let numeric_assembly_scratch_bytes =
            checked_mul(problem.input_edges, 8, "prepared duplicate scratch")?;
        let current_numeric_graph_bytes = checked_sum(&[
            checked_mul(problem.canonical_edges, 16, "current canonical edges")?,
            checked_mul(problem.vertices, 8, "current numeric diagonal")?,
        ])?;
        let inferred_plan_bytes = legacy
            .total_retained_bytes
            .checked_sub(legacy.retained_solver_bytes)
            .and_then(|value| value.checked_sub(legacy.workspace_pool_bytes))
            .ok_or(CmgError::InvalidHierarchy {
                context: "repeated PCG plan estimate underflows",
            })?;
        let parallel_plan_bytes = if retain_parallel_plan {
            inferred_plan_bytes
        } else {
            0
        };
        let workspace_bytes_each = checked_add(
            legacy.workspace_bytes_each,
            checked_mul(problem.vertices, 16, "batch gather staging")?,
            "repeated PCG workspace",
        )?;
        let requested_concurrency = parallel_options
            .resolved_threads()
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
        let workspace_pool_bytes = checked_mul(
            workspace_bytes_each,
            concurrency,
            "repeated PCG workspace pool",
        )?;
        let retained_preconditioner_bytes = legacy.retained_solver_bytes;
        let total_solver_retained_bytes = checked_sum(&[
            prepared_topology_bytes,
            numeric_assembly_scratch_bytes,
            current_numeric_graph_bytes,
            retained_preconditioner_bytes,
            parallel_plan_bytes,
            workspace_pool_bytes,
        ])?;
        let logical_vectors = 2_usize + usize::from(include_initial_guesses);
        let caller_logical_bytes = checked_sum(&[
            checked_mul(
                checked_mul(
                    problem.vertices,
                    problem.right_hand_sides,
                    "caller batch values",
                )?,
                checked_mul(logical_vectors, 8, "caller vector bytes")?,
                "caller logical vectors",
            )?,
            checked_mul(
                problem.right_hand_sides,
                core::mem::size_of::<PcgDiagnostics>(),
                "caller diagnostics",
            )?,
        ])?;
        let build_peak_bytes = checked_sum(&[
            total_solver_retained_bytes,
            checked_mul(problem.input_edges, 16, "prepared topology build records")?,
        ])?;
        Ok(Self {
            prepared_topology_bytes,
            shared_component_metadata_bytes,
            numeric_assembly_scratch_bytes,
            current_numeric_graph_bytes,
            retained_preconditioner_bytes,
            parallel_plan_bytes,
            workspace_bytes_each,
            workspace_pool_bytes,
            total_solver_retained_bytes,
            caller_logical_bytes,
            build_peak_bytes,
        })
    }

    /// Return conservative prepared-topology bytes excluding shared components.
    #[must_use]
    pub const fn prepared_topology_bytes(self) -> usize {
        self.prepared_topology_bytes
    }

    /// Return shared component bytes included once in the hierarchy estimate.
    #[must_use]
    pub const fn shared_component_metadata_bytes(self) -> usize {
        self.shared_component_metadata_bytes
    }

    /// Return conservative numeric-assembly scratch bytes.
    #[must_use]
    pub const fn numeric_assembly_scratch_bytes(self) -> usize {
        self.numeric_assembly_scratch_bytes
    }

    /// Return conservative current numeric graph bytes.
    #[must_use]
    pub const fn current_numeric_graph_bytes(self) -> usize {
        self.current_numeric_graph_bytes
    }

    /// Return conservative retained stale-preconditioner bytes.
    #[must_use]
    pub const fn retained_preconditioner_bytes(self) -> usize {
        self.retained_preconditioner_bytes
    }

    /// Return conservative optional parallel-plan bytes.
    #[must_use]
    pub const fn parallel_plan_bytes(self) -> usize {
        self.parallel_plan_bytes
    }

    /// Return conservative bytes for each reusable batch workspace.
    #[must_use]
    pub const fn workspace_bytes_each(self) -> usize {
        self.workspace_bytes_each
    }

    /// Return conservative retained workspace-pool bytes.
    #[must_use]
    pub const fn workspace_pool_bytes(self) -> usize {
        self.workspace_pool_bytes
    }

    /// Return conservative solver-retained principal-array bytes, excluding
    /// caller arrays and complete container/Arc/runtime storage.
    #[must_use]
    pub const fn total_solver_retained_bytes(self) -> usize {
        self.total_solver_retained_bytes
    }

    /// Return logical bytes in caller RHS, guess, solution, and diagnostic buffers.
    #[must_use]
    pub const fn caller_logical_bytes(self) -> usize {
        self.caller_logical_bytes
    }

    /// Return the legacy principal-array preparation/construction estimate,
    /// not a complete action peak. Full temporary overlap, descriptor/Arc banks
    /// and runtime allocations require separate coverage.
    #[must_use]
    pub const fn build_peak_bytes(self) -> usize {
        self.build_peak_bytes
    }
}

/// Exact principal retained bytes for a prepared current frame and retained hierarchy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepeatedPcgMemoryReport {
    prepared_topology_bytes: usize,
    numeric_assembly_scratch_bytes: usize,
    current_numeric_graph_bytes: usize,
    retained_preconditioner_bytes: usize,
    shared_component_metadata_bytes: usize,
    parallel_plan_bytes: usize,
    workspace_pool_bytes: usize,
    total_solver_retained_bytes: usize,
    caller_logical_bytes: usize,
}

impl RepeatedPcgMemoryReport {
    /// Report exact serial retained bytes without a parallel plan.
    pub fn serial(
        topology: &PreparedLaplacianTopology,
        assembly_workspace: &PreparedLaplacianWorkspace,
        current_graph: &Laplacian,
        retained_preconditioner: &CmgPreconditioner,
        workspaces: &[PcgBatchWorkspace],
        right_hand_sides: usize,
        include_initial_guesses: bool,
    ) -> Result<Self, CmgError> {
        Self::build(
            topology,
            assembly_workspace,
            current_graph,
            retained_preconditioner,
            workspaces,
            0,
            right_hand_sides,
            include_initial_guesses,
        )
    }

    /// Report exact retained bytes including an optional parallel plan.
    #[cfg(feature = "parallel")]
    #[allow(clippy::too_many_arguments)]
    pub fn with_parallel_plan(
        topology: &PreparedLaplacianTopology,
        assembly_workspace: &PreparedLaplacianWorkspace,
        current_graph: &Laplacian,
        retained_preconditioner: &CmgPreconditioner,
        plan: Option<&ParallelCmgPlan>,
        workspaces: &[PcgBatchWorkspace],
        right_hand_sides: usize,
        include_initial_guesses: bool,
    ) -> Result<Self, CmgError> {
        if let Some(plan) = plan {
            plan.validate(retained_preconditioner)?;
        }
        Self::build(
            topology,
            assembly_workspace,
            current_graph,
            retained_preconditioner,
            workspaces,
            plan.map_or(0, ParallelCmgPlan::byte_len),
            right_hand_sides,
            include_initial_guesses,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn build(
        topology: &PreparedLaplacianTopology,
        assembly_workspace: &PreparedLaplacianWorkspace,
        current_graph: &Laplacian,
        retained_preconditioner: &CmgPreconditioner,
        workspaces: &[PcgBatchWorkspace],
        parallel_plan_bytes: usize,
        right_hand_sides: usize,
        include_initial_guesses: bool,
    ) -> Result<Self, CmgError> {
        if !topology.matches_graph(current_graph)
            || !retained_preconditioner.matches_prepared_topology(current_graph)
        {
            return Err(CmgError::InvalidHierarchy {
                context: "repeated memory report objects have incompatible topology",
            });
        }
        let prepared_topology_bytes = topology.retained_bytes();
        let numeric_assembly_scratch_bytes = assembly_workspace.byte_len();
        let current_numeric_graph_bytes = current_graph.retained_bytes();
        let shared_component_metadata_bytes = retained_preconditioner.finest_component_bytes();
        let retained_preconditioner_bytes = retained_preconditioner
            .retained_bytes()
            .checked_sub(shared_component_metadata_bytes)
            .ok_or(CmgError::InvalidHierarchy {
                context: "retained preconditioner component accounting underflows",
            })?;
        for workspace in workspaces {
            workspace.validate(retained_preconditioner)?;
        }
        let workspace_pool_bytes = workspaces.iter().try_fold(0_usize, |total, workspace| {
            checked_add(total, workspace.byte_len(), "repeated workspace report")
        })?;
        let total_solver_retained_bytes = checked_sum(&[
            prepared_topology_bytes,
            numeric_assembly_scratch_bytes,
            current_numeric_graph_bytes,
            retained_preconditioner_bytes,
            parallel_plan_bytes,
            workspace_pool_bytes,
        ])?;
        let logical_vectors = 2_usize + usize::from(include_initial_guesses);
        let caller_logical_bytes = checked_sum(&[
            checked_mul(
                checked_mul(
                    current_graph.vertex_count(),
                    right_hand_sides,
                    "reported caller batch values",
                )?,
                checked_mul(logical_vectors, 8, "reported caller vector bytes")?,
                "reported caller logical vectors",
            )?,
            checked_mul(
                right_hand_sides,
                core::mem::size_of::<PcgDiagnostics>(),
                "reported caller diagnostics",
            )?,
        ])?;
        Ok(Self {
            prepared_topology_bytes,
            numeric_assembly_scratch_bytes,
            current_numeric_graph_bytes,
            retained_preconditioner_bytes,
            shared_component_metadata_bytes,
            parallel_plan_bytes,
            workspace_pool_bytes,
            total_solver_retained_bytes,
            caller_logical_bytes,
        })
    }

    /// Return exact prepared-topology bytes, including shared components once.
    #[must_use]
    pub const fn prepared_topology_bytes(self) -> usize {
        self.prepared_topology_bytes
    }

    /// Return exact numeric-assembly scratch bytes.
    #[must_use]
    pub const fn numeric_assembly_scratch_bytes(self) -> usize {
        self.numeric_assembly_scratch_bytes
    }

    /// Return exact current numeric graph bytes.
    #[must_use]
    pub const fn current_numeric_graph_bytes(self) -> usize {
        self.current_numeric_graph_bytes
    }

    /// Return exact retained hierarchy bytes after shared-component de-duplication.
    #[must_use]
    pub const fn retained_preconditioner_bytes(self) -> usize {
        self.retained_preconditioner_bytes
    }

    /// Return component bytes shared by topology and retained hierarchy.
    #[must_use]
    pub const fn shared_component_metadata_bytes(self) -> usize {
        self.shared_component_metadata_bytes
    }

    /// Return exact optional plan bytes.
    #[must_use]
    pub const fn parallel_plan_bytes(self) -> usize {
        self.parallel_plan_bytes
    }

    /// Return exact caller-owned workspace-pool bytes.
    #[must_use]
    pub const fn workspace_pool_bytes(self) -> usize {
        self.workspace_pool_bytes
    }

    /// Return exact solver-retained bytes, excluding caller data arrays.
    #[must_use]
    pub const fn total_solver_retained_bytes(self) -> usize {
        self.total_solver_retained_bytes
    }

    /// Return logical caller-buffer bytes separately from solver retention.
    #[must_use]
    pub const fn caller_logical_bytes(self) -> usize {
        self.caller_logical_bytes
    }
}

fn checked_ceil_product(
    value: usize,
    factor: f64,
    context: &'static str,
) -> Result<usize, CmgError> {
    // Round outward to cover conversion and multiplication rounding in the
    // builder's floating fill comparison, including integer-to-f64 rounding
    // of its cumulative count. next_up is written via bits for Rust 1.85.
    let upward = |x: f64| f64::from_bits(x.to_bits() + 1);
    let product = upward(upward(value as f64) * factor);
    if !product.is_finite() || product >= usize::MAX as f64 {
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
