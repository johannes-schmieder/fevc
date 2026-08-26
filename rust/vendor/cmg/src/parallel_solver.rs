//! Prepared, memory-aware execution for repeated parallel PCG solves.

use rayon::prelude::*;

use crate::{
    CmgError, CmgOptions, CmgPreconditioner, Laplacian, ParallelCmgPlan, ParallelExecutor,
    ParallelOptions, PcgOptions, PcgResult, PcgWorkspace, solve_pcg_with_plan_and_workspace,
    solve_pcg_with_workspace,
};

/// Default finest-graph edge threshold for planned single-RHS execution.
///
/// This conservative threshold is derived from the retained hosted-runner
/// full-PCG crossover matrix. It is measured in canonical retained edges:
/// nominal 300,000-edge sparse worker-firm systems stay below the threshold,
/// while the nominal 375,000-edge class crosses it and benefits materially.
/// It is a routing heuristic rather than a mathematical CMG constant and can
/// be overridden through [`ParallelPcgPolicy`].
pub const DEFAULT_MIN_PLANNED_EDGES: usize = 350_000;

/// Routing policy for a prepared parallel PCG solver.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParallelPcgPolicy {
    /// Minimum finest-graph edge count before a single RHS uses the parallel
    /// CMG plan when at least one routed operator exists.
    pub min_planned_edges: usize,
    /// Minimum feasible simultaneous-workspace count before a batch is routed
    /// across independent right-hand sides.
    pub min_across_rhs_concurrency: usize,
}

impl Default for ParallelPcgPolicy {
    fn default() -> Self {
        Self {
            min_planned_edges: DEFAULT_MIN_PLANNED_EDGES,
            min_across_rhs_concurrency: 2,
        }
    }
}

impl ParallelPcgPolicy {
    /// Validate policy fields.
    pub fn validate(self) -> Result<Self, CmgError> {
        if self.min_across_rhs_concurrency < 2 {
            return Err(CmgError::InvalidOption {
                name: "min_across_rhs_concurrency",
                value: self.min_across_rhs_concurrency as f64,
            });
        }
        Ok(self)
    }
}

/// Actual execution strategy selected for one solve or one batch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParallelPcgExecution {
    /// Certified serial PCG with one reusable workspace.
    Serial,
    /// Certified PCG using a selectively routed [`ParallelCmgPlan`].
    Planned,
    /// Independent certified serial PCG solves distributed across RHSs.
    AcrossRightHandSides,
}

/// Observable routing and memory decision for one requested batch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParallelPcgBatchReport {
    execution: ParallelPcgExecution,
    rhs_count: usize,
    concurrency: usize,
    workspace_bytes_each: usize,
    workspace_pool_bytes: usize,
    plan_bytes: usize,
}

impl ParallelPcgBatchReport {
    /// Return the selected execution strategy.
    #[must_use]
    pub const fn execution(&self) -> ParallelPcgExecution {
        self.execution
    }

    /// Return the number of submitted right-hand sides.
    #[must_use]
    pub const fn rhs_count(&self) -> usize {
        self.rhs_count
    }

    /// Return the maximum simultaneous solve count selected for this batch.
    #[must_use]
    pub const fn concurrency(&self) -> usize {
        self.concurrency
    }

    /// Return retained bytes required by one PCG workspace.
    #[must_use]
    pub const fn workspace_bytes_each(&self) -> usize {
        self.workspace_bytes_each
    }

    /// Return requested retained bytes for the reusable workspace pool.
    #[must_use]
    pub const fn workspace_pool_bytes(&self) -> usize {
        self.workspace_pool_bytes
    }

    /// Return retained bytes in the optional parallel hierarchy plan.
    #[must_use]
    pub const fn plan_bytes(&self) -> usize {
        self.plan_bytes
    }
}

/// Results and routing report from an automatically selected batch strategy.
#[derive(Debug, Clone, PartialEq)]
pub struct ParallelPcgBatchResult {
    results: Vec<PcgResult>,
    report: ParallelPcgBatchReport,
}

impl ParallelPcgBatchResult {
    /// Borrow results in input order.
    #[must_use]
    pub fn results(&self) -> &[PcgResult] {
        &self.results
    }

    /// Consume the wrapper and return results in input order.
    #[must_use]
    pub fn into_results(self) -> Vec<PcgResult> {
        self.results
    }

    /// Return the routing and memory report.
    #[must_use]
    pub const fn report(&self) -> ParallelPcgBatchReport {
        self.report
    }
}

/// Reusable workspaces for prepared single- and multi-RHS solves.
#[derive(Debug)]
pub struct ParallelPcgWorkspace {
    workspaces: Vec<PcgWorkspace>,
}

impl ParallelPcgWorkspace {
    /// Allocate one workspace; the pool expands on demand for batch execution.
    #[must_use]
    pub fn new(preconditioner: &CmgPreconditioner) -> Self {
        Self {
            workspaces: vec![PcgWorkspace::new(preconditioner)],
        }
    }

    /// Return the number of retained workspaces.
    #[must_use]
    pub fn workspace_count(&self) -> usize {
        self.workspaces.len()
    }

    /// Return retained principal heap bytes for the complete workspace pool.
    #[must_use]
    pub fn byte_len(&self) -> usize {
        self.workspaces
            .iter()
            .map(PcgWorkspace::byte_len)
            .fold(0_usize, usize::saturating_add)
    }

    fn ensure_count(&mut self, count: usize, preconditioner: &CmgPreconditioner) {
        self.workspaces
            .extend((self.workspaces.len()..count).map(|_| PcgWorkspace::new(preconditioner)));
    }
}

/// A reusable owner of one CMG hierarchy, optional parallel plan, and thread pool.
///
/// The prepared solver keeps the serial low-level APIs unchanged. Automatic
/// routing is observable through [`Self::select_batch_execution`] and can be
/// bypassed by using the existing explicit serial, planned, or batch functions.
#[derive(Debug)]
pub struct ParallelPcgSolver {
    preconditioner: CmgPreconditioner,
    plan: ParallelCmgPlan,
    executor: ParallelExecutor,
    policy: ParallelPcgPolicy,
    workspace_bytes: usize,
}

impl ParallelPcgSolver {
    /// Build a parallel hierarchy, routed plan, and executor with the default policy.
    pub fn build(
        graph: &Laplacian,
        cmg_options: CmgOptions,
        parallel_options: ParallelOptions,
    ) -> Result<Self, CmgError> {
        Self::build_with_policy(
            graph,
            cmg_options,
            parallel_options,
            ParallelPcgPolicy::default(),
        )
    }

    /// Build with an explicit routing policy.
    pub fn build_with_policy(
        graph: &Laplacian,
        cmg_options: CmgOptions,
        parallel_options: ParallelOptions,
        policy: ParallelPcgPolicy,
    ) -> Result<Self, CmgError> {
        let executor = ParallelExecutor::new(parallel_options)?;
        let preconditioner = CmgPreconditioner::build_with_executor(graph, cmg_options, &executor)?;
        Self::from_preconditioner(preconditioner, executor, policy)
    }

    /// Combine an existing immutable preconditioner with an executor and policy.
    pub fn from_preconditioner(
        preconditioner: CmgPreconditioner,
        executor: ParallelExecutor,
        policy: ParallelPcgPolicy,
    ) -> Result<Self, CmgError> {
        let policy = policy.validate()?;
        let plan = ParallelCmgPlan::build(&preconditioner, &executor)?;
        let workspace_bytes = PcgWorkspace::new(&preconditioner).byte_len();
        Ok(Self {
            preconditioner,
            plan,
            executor,
            policy,
            workspace_bytes,
        })
    }

    /// Return the immutable finest graph owned by the hierarchy.
    #[must_use]
    pub fn graph(&self) -> &Laplacian {
        self.preconditioner.hierarchy().levels()[0].graph()
    }

    /// Return the immutable CMG preconditioner.
    #[must_use]
    pub const fn preconditioner(&self) -> &CmgPreconditioner {
        &self.preconditioner
    }

    /// Return the selectively routed parallel hierarchy plan.
    #[must_use]
    pub const fn plan(&self) -> &ParallelCmgPlan {
        &self.plan
    }

    /// Return the package-owned executor.
    #[must_use]
    pub const fn executor(&self) -> &ParallelExecutor {
        &self.executor
    }

    /// Return the validated routing policy.
    #[must_use]
    pub const fn policy(&self) -> ParallelPcgPolicy {
        self.policy
    }

    /// Allocate a reusable workspace pool for this solver.
    #[must_use]
    pub fn workspace(&self) -> ParallelPcgWorkspace {
        ParallelPcgWorkspace::new(&self.preconditioner)
    }

    /// Return bytes retained by one PCG workspace.
    #[must_use]
    pub const fn workspace_bytes(&self) -> usize {
        self.workspace_bytes
    }

    /// Select and report the execution strategy without performing a solve.
    pub fn select_batch_execution(
        &self,
        rhs_count: usize,
    ) -> Result<ParallelPcgBatchReport, CmgError> {
        if rhs_count == 0 {
            return Ok(self.report(ParallelPcgExecution::Serial, 0, 0));
        }
        let planned_eligible = self.executor.thread_count() > 1
            && self.plan.operator_count() > 0
            && self.graph().edges().len() >= self.policy.min_planned_edges;
        let concurrency = if rhs_count == 1 {
            1
        } else {
            self.executor
                .batch_concurrency(self.workspace_bytes, rhs_count)?
        };
        let execution = if rhs_count == 1 {
            if planned_eligible {
                ParallelPcgExecution::Planned
            } else {
                ParallelPcgExecution::Serial
            }
        } else if concurrency >= self.policy.min_across_rhs_concurrency {
            ParallelPcgExecution::AcrossRightHandSides
        } else if planned_eligible {
            ParallelPcgExecution::Planned
        } else {
            ParallelPcgExecution::Serial
        };
        let retained_count = match execution {
            ParallelPcgExecution::AcrossRightHandSides => concurrency,
            ParallelPcgExecution::Serial | ParallelPcgExecution::Planned => 1,
        };
        Ok(self.report(execution, rhs_count, retained_count))
    }

    /// Solve one RHS using the automatically selected single-system strategy.
    pub fn solve(&self, rhs: &[f64], options: PcgOptions) -> Result<PcgResult, CmgError> {
        let mut workspace = self.workspace();
        self.solve_with_workspace(rhs, options, &mut workspace)
    }

    /// Solve one RHS with a caller-owned reusable workspace pool.
    pub fn solve_with_workspace(
        &self,
        rhs: &[f64],
        options: PcgOptions,
        workspace: &mut ParallelPcgWorkspace,
    ) -> Result<PcgResult, CmgError> {
        workspace.ensure_count(1, &self.preconditioner);
        match self.select_batch_execution(1)?.execution {
            ParallelPcgExecution::Planned => solve_pcg_with_plan_and_workspace(
                self.graph(),
                &self.preconditioner,
                &self.plan,
                rhs,
                options,
                &mut workspace.workspaces[0],
                &self.executor,
            ),
            ParallelPcgExecution::Serial | ParallelPcgExecution::AcrossRightHandSides => {
                solve_pcg_with_workspace(
                    self.graph(),
                    &self.preconditioner,
                    rhs,
                    options,
                    &mut workspace.workspaces[0],
                )
            }
        }
    }

    /// Solve a batch with an automatically selected memory-aware strategy.
    pub fn solve_batch(
        &self,
        right_hand_sides: &[Vec<f64>],
        options: PcgOptions,
    ) -> Result<ParallelPcgBatchResult, CmgError> {
        let mut workspace = self.workspace();
        self.solve_batch_with_workspace(right_hand_sides, options, &mut workspace)
    }

    /// Solve a batch while reusing a caller-owned workspace pool.
    pub fn solve_batch_with_workspace(
        &self,
        right_hand_sides: &[Vec<f64>],
        options: PcgOptions,
        workspace: &mut ParallelPcgWorkspace,
    ) -> Result<ParallelPcgBatchResult, CmgError> {
        for rhs in right_hand_sides {
            if rhs.len() != self.graph().vertex_count() {
                return Err(CmgError::dimension(
                    "ParallelPcgSolver batch rhs",
                    self.graph().vertex_count(),
                    rhs.len(),
                ));
            }
        }
        let report = self.select_batch_execution(right_hand_sides.len())?;
        if right_hand_sides.is_empty() {
            return Ok(ParallelPcgBatchResult {
                results: Vec::new(),
                report,
            });
        }
        workspace.ensure_count(report.concurrency.max(1), &self.preconditioner);
        let results = match report.execution {
            ParallelPcgExecution::Serial => right_hand_sides
                .iter()
                .map(|rhs| {
                    solve_pcg_with_workspace(
                        self.graph(),
                        &self.preconditioner,
                        rhs,
                        options,
                        &mut workspace.workspaces[0],
                    )
                })
                .collect::<Result<Vec<_>, CmgError>>()?,
            ParallelPcgExecution::Planned => right_hand_sides
                .iter()
                .map(|rhs| {
                    solve_pcg_with_plan_and_workspace(
                        self.graph(),
                        &self.preconditioner,
                        &self.plan,
                        rhs,
                        options,
                        &mut workspace.workspaces[0],
                        &self.executor,
                    )
                })
                .collect::<Result<Vec<_>, CmgError>>()?,
            ParallelPcgExecution::AcrossRightHandSides => {
                let mut results = Vec::with_capacity(right_hand_sides.len());
                for rhs_chunk in right_hand_sides.chunks(report.concurrency) {
                    let chunk_results: Vec<Result<PcgResult, CmgError>> =
                        self.executor.install(|| {
                            workspace.workspaces[..rhs_chunk.len()]
                                .par_iter_mut()
                                .zip(rhs_chunk.par_iter())
                                .map(|(pcg_workspace, rhs)| {
                                    solve_pcg_with_workspace(
                                        self.graph(),
                                        &self.preconditioner,
                                        rhs,
                                        options,
                                        pcg_workspace,
                                    )
                                })
                                .collect()
                        });
                    for result in chunk_results {
                        results.push(result?);
                    }
                }
                results
            }
        };
        Ok(ParallelPcgBatchResult { results, report })
    }

    fn report(
        &self,
        execution: ParallelPcgExecution,
        rhs_count: usize,
        concurrency: usize,
    ) -> ParallelPcgBatchReport {
        ParallelPcgBatchReport {
            execution,
            rhs_count,
            concurrency,
            workspace_bytes_each: self.workspace_bytes,
            workspace_pool_bytes: self.workspace_bytes.saturating_mul(concurrency),
            plan_bytes: self.plan.byte_len(),
        }
    }
}
