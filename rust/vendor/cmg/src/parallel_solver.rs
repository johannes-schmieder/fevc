//! Prepared, memory-aware execution for repeated parallel PCG solves.

use std::sync::{Mutex, OnceLock};

use crate::pcg::{result_from_diagnostics, solve_owned_concurrently, solve_pcg_in_workspace};
use crate::{
    BuildContext, CmgError, CmgOptions, CmgPreconditioner, Laplacian, ParallelCmgPlan,
    ParallelOptions, PcgOptions, PcgResult, PcgWorkspace, SolveContext, Team,
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
    /// CMG path when a routed operator or sufficiently large deterministic
    /// parallel vector operation exists.
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

crate::storage::storage_fields!(ParallelPcgBatchResult, results);

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

crate::storage::storage_fields!(ParallelPcgWorkspace, workspaces);
crate::storage::storage_fields!(ParallelPcgSolver, preconditioner, plan);

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

/// A reusable owner of one CMG hierarchy, its lazy parallel plan and routing
/// policy for one execution shape.
///
/// The solver owns no pool. It records the shape (worker ceiling and
/// [`ParallelOptions`]) of the team it was prepared for; every call that
/// executes borrows a team of exactly that shape, so routing and plans stay
/// valid. Automatic routing is observable through
/// [`Self::select_batch_execution`] and can be bypassed with
/// [`crate::solve_pcg_into`] and [`crate::solve_pcg_batch_into`].
#[derive(Debug)]
pub struct ParallelPcgSolver {
    preconditioner: CmgPreconditioner,
    plan: OnceLock<ParallelCmgPlan>,
    plan_initialization: Mutex<()>,
    eligible_plan_operators: usize,
    eligible_parallel_vector_work: bool,
    shape: ParallelOptions,
    policy: ParallelPcgPolicy,
    workspace_bytes: usize,
}

impl ParallelPcgSolver {
    pub(crate) fn initialized_plan_bytes(&self) -> usize {
        self.plan.get().map_or(0, ParallelCmgPlan::byte_len)
    }

    /// Return exact principal retained heap bytes for this solver and a
    /// compatible reusable workspace pool.
    #[must_use]
    pub fn memory_report(&self, workspace: &ParallelPcgWorkspace) -> crate::CmgMemoryReport {
        crate::CmgMemoryReport::new(self, workspace)
    }

    /// Build a hierarchy on `team` and prepare it with the default policy.
    pub fn build(
        graph: &Laplacian,
        cmg_options: CmgOptions,
        team: Team<'_>,
    ) -> Result<Self, CmgError> {
        Self::build_with_policy(graph, cmg_options, team, ParallelPcgPolicy::default())
    }

    /// Build with an explicit routing policy.
    pub fn build_with_policy(
        graph: &Laplacian,
        cmg_options: CmgOptions,
        team: Team<'_>,
        policy: ParallelPcgPolicy,
    ) -> Result<Self, CmgError> {
        let preconditioner = CmgPreconditioner::build_with(
            graph,
            cmg_options.into(),
            BuildContext {
                team: Some(team),
                ..BuildContext::default()
            },
        )?;
        Self::from_preconditioner(preconditioner, team, policy)
    }

    /// Prepare an existing immutable preconditioner for `team`'s shape.
    pub fn from_preconditioner(
        preconditioner: CmgPreconditioner,
        team: Team<'_>,
        policy: ParallelPcgPolicy,
    ) -> Result<Self, CmgError> {
        let policy = policy.validate()?;
        let shape = team.options();
        let eligible_plan_operators =
            ParallelCmgPlan::eligible_operator_count(&preconditioner, shape);
        let eligible_parallel_vector_work = shape.threads > 1
            && preconditioner.finest_components().count() == 1
            && preconditioner.hierarchy().levels()[0]
                .graph()
                .vertex_count()
                >= shape.reduction_floor();
        let workspace_bytes = PcgWorkspace::required_bytes(&preconditioner);
        Ok(Self {
            preconditioner,
            plan: OnceLock::new(),
            plan_initialization: Mutex::new(()),
            eligible_plan_operators,
            eligible_parallel_vector_work,
            shape,
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

    /// Return the execution shape every borrowed team must have: options
    /// whose `threads` is the worker ceiling.
    #[must_use]
    pub const fn execution_shape(&self) -> ParallelOptions {
        self.shape
    }

    /// Return the plan if it has been initialized.
    #[must_use]
    pub fn initialized_plan(&self) -> Option<&ParallelCmgPlan> {
        self.plan.get()
    }

    /// Return the selectively routed parallel hierarchy plan, constructing it
    /// on `team` on first access and reporting any construction error.
    pub fn try_plan(&self, team: Team<'_>) -> Result<&ParallelCmgPlan, CmgError> {
        self.check_team(team)?;
        self.ensure_plan(team, None, None)
    }

    /// New CMG-owned plan layouts still needed by this solver (zero if already
    /// initialized). Does not include retained parents or Rayon runtime memory.
    pub fn plan_construction_allocation_bound(&self) -> Result<usize, CmgError> {
        if self.plan.get().is_some() {
            Ok(0)
        } else {
            ParallelCmgPlan::construction_allocation_bound_for(&self.preconditioner, self.shape)
        }
    }

    /// Initialize this solver's existing lazy plan with a scoped CMG-owned
    /// layout budget; no second plan or pool is created. Once initialized,
    /// subsequent calls allocate nothing and accept a zero incremental budget.
    /// Runtime/OS synchronization resources and already retained owners must be
    /// covered separately. Other unbudgeted methods still permit lazy setup.
    pub fn try_plan_with_budget(
        &self,
        team: Team<'_>,
        budget_bytes: usize,
    ) -> Result<&ParallelCmgPlan, CmgError> {
        self.check_team(team)?;
        self.ensure_plan(team, Some(budget_bytes), None)
    }

    /// Initialize the lazy plan under a construction context carrying this
    /// solver's team, an optional budget (as [`Self::try_plan_with_budget`])
    /// and an optional cancellation query (as [`ParallelCmgPlan::build_with`]).
    /// A cancelled initialization leaves the plan uninitialized; a later call
    /// builds it again. An initialized plan is returned without allocation.
    pub fn try_plan_with(
        &self,
        context: crate::BuildContext<'_>,
    ) -> Result<&ParallelCmgPlan, CmgError> {
        let team = context.team.ok_or(CmgError::InvalidOption {
            name: "ParallelPcgSolver plan requires a team",
            value: 0.0,
        })?;
        self.check_team(team)?;
        self.ensure_plan(team, context.budget_bytes, context.cancel)
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

    /// Select and report the execution strategy without performing a solve,
    /// initializing the plan on `team` when the planned strategy is selected.
    pub fn select_batch_execution(
        &self,
        rhs_count: usize,
        team: Team<'_>,
    ) -> Result<ParallelPcgBatchReport, CmgError> {
        self.check_team(team)?;
        if rhs_count == 0 {
            return Ok(self.report(ParallelPcgExecution::Serial, 0, 0));
        }
        let planned_eligible = self.shape.threads > 1
            && (self.eligible_plan_operators > 0 || self.eligible_parallel_vector_work)
            && self.graph().edges().len() >= self.policy.min_planned_edges;
        let concurrency = if rhs_count == 1 {
            1
        } else {
            self.shape
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
        if execution == ParallelPcgExecution::Planned {
            self.ensure_plan(team, None, None)?;
        }
        Ok(self.report(execution, rhs_count, retained_count))
    }

    /// Solve one RHS using the automatically selected single-system strategy.
    pub fn solve(
        &self,
        rhs: &[f64],
        options: PcgOptions,
        team: Team<'_>,
    ) -> Result<PcgResult, CmgError> {
        let mut workspace = self.workspace();
        self.solve_with_workspace(rhs, options, &mut workspace, team)
    }

    /// Solve one RHS with a caller-owned reusable workspace pool.
    pub fn solve_with_workspace(
        &self,
        rhs: &[f64],
        options: PcgOptions,
        workspace: &mut ParallelPcgWorkspace,
        team: Team<'_>,
    ) -> Result<PcgResult, CmgError> {
        workspace.ensure_count(1, &self.preconditioner);
        let context = match self.select_batch_execution(1, team)?.execution {
            ParallelPcgExecution::Planned => self.planned(team)?,
            ParallelPcgExecution::Serial | ParallelPcgExecution::AcrossRightHandSides => {
                Self::serial(team)
            }
        };
        self.solve_one(rhs, options, &mut workspace.workspaces[0], context)
    }

    /// Solve a batch with an automatically selected memory-aware strategy.
    pub fn solve_batch(
        &self,
        right_hand_sides: &[Vec<f64>],
        options: PcgOptions,
        team: Team<'_>,
    ) -> Result<ParallelPcgBatchResult, CmgError> {
        let mut workspace = self.workspace();
        self.solve_batch_with_workspace(right_hand_sides, options, &mut workspace, team)
    }

    /// Solve a batch while reusing a caller-owned workspace pool.
    pub fn solve_batch_with_workspace(
        &self,
        right_hand_sides: &[Vec<f64>],
        options: PcgOptions,
        workspace: &mut ParallelPcgWorkspace,
        team: Team<'_>,
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
        let report = self.select_batch_execution(right_hand_sides.len(), team)?;
        if right_hand_sides.is_empty() {
            return Ok(ParallelPcgBatchResult {
                results: Vec::new(),
                report,
            });
        }
        workspace.ensure_count(report.concurrency.max(1), &self.preconditioner);
        let results = match report.execution {
            ParallelPcgExecution::Serial | ParallelPcgExecution::Planned => {
                let context = if report.execution == ParallelPcgExecution::Planned {
                    self.planned(team)?
                } else {
                    Self::serial(team)
                };
                right_hand_sides
                    .iter()
                    .map(|rhs| self.solve_one(rhs, options, &mut workspace.workspaces[0], context))
                    .collect::<Result<Vec<_>, CmgError>>()?
            }
            ParallelPcgExecution::AcrossRightHandSides => solve_owned_concurrently(
                self.graph(),
                &self.preconditioner,
                right_hand_sides,
                options,
                team,
                &mut workspace.workspaces[..report.concurrency],
            )?,
        };
        Ok(ParallelPcgBatchResult { results, report })
    }

    /// The serial context: one lane of `team`, so a serial solve keeps the
    /// planned solve's reduction and centering arithmetic.
    #[must_use]
    pub fn serial(team: Team<'_>) -> SolveContext<'_> {
        SolveContext {
            team: Some(team.single_lane()),
            ..SolveContext::default()
        }
    }

    fn planned<'s>(&'s self, team: Team<'s>) -> Result<SolveContext<'s>, CmgError> {
        Ok(SolveContext {
            plan: Some(self.ensure_plan(team, None, None)?),
            team: Some(team),
            ..SolveContext::default()
        })
    }

    fn solve_one(
        &self,
        rhs: &[f64],
        options: PcgOptions,
        workspace: &mut PcgWorkspace,
        context: SolveContext<'_>,
    ) -> Result<PcgResult, CmgError> {
        let diagnostics = solve_pcg_in_workspace(
            self.graph(),
            &self.preconditioner,
            rhs,
            None,
            options,
            workspace,
            context,
        )?;
        Ok(result_from_diagnostics(
            workspace.solution().to_vec(),
            diagnostics,
        ))
    }

    fn check_team(&self, team: Team<'_>) -> Result<(), CmgError> {
        if team.options() == self.shape {
            Ok(())
        } else {
            Err(CmgError::InvalidHierarchy {
                context: "team differs from the parallel PCG solver's execution shape",
            })
        }
    }

    fn ensure_plan(
        &self,
        team: Team<'_>,
        budget_bytes: Option<usize>,
        cancel: Option<crate::Cancellation<'_>>,
    ) -> Result<&ParallelCmgPlan, CmgError> {
        if let Some(plan) = self.plan.get() {
            return Ok(plan);
        }
        let _initialization = self
            .plan_initialization
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(plan) = self.plan.get() {
            return Ok(plan);
        }
        let plan = ParallelCmgPlan::build_with(
            &self.preconditioner,
            crate::BuildContext {
                team: Some(team),
                cancel,
                budget_bytes,
            },
        )?;
        if self.plan.set(plan).is_err() {
            return Err(CmgError::InvalidHierarchy {
                context: "parallel CMG plan was initialized concurrently",
            });
        }
        self.plan.get().ok_or(CmgError::InvalidHierarchy {
            context: "parallel CMG plan initialization failed",
        })
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
            plan_bytes: self.plan.get().map_or(0, ParallelCmgPlan::byte_len),
        }
    }
}
