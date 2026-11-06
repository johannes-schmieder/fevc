//! Stationary recursive CMG preconditioner construction and application.

/// Run `$body` with the context's meter: a cancellation meter that also polls
/// once at completion, or the unmetered fast path.
macro_rules! with_meter {
    ($cancel:expr, $meter:ident => $body:expr) => {
        match $cancel {
            Some(cancel) => {
                let $meter = &mut crate::cancel::CancelMeter::new(cancel);
                let result = $body;
                result.and_then(|value| {
                    crate::cancel::Meter::checkpoint($meter, "CMG application")?;
                    Ok(value)
                })
            }
            None => {
                let $meter = &mut crate::cancel::Unmetered;
                $body
            }
        }
    };
}
pub(crate) use with_meter;

mod batch;
mod cycle;
pub use batch::CmgBatchWorkspace;
pub(crate) use cycle::{LevelParts, One, Parts, Store, Width, rows_serial};
#[cfg(feature = "parallel")]
pub(crate) use cycle::{rows_parallel, rows_parallel_with};

use crate::cancel::{Meter, Unmetered};
use crate::cmg_profile::CycleRecorder;
use crate::components::CenteringPlan;
#[cfg(feature = "parallel")]
use crate::csr::CsrBuildProfile;
#[cfg(feature = "parallel")]
use crate::execution::Team;
use crate::{
    BuildContext, BuildOptions, Cancellation, CmgError, CmgHierarchy, CmgOptions, CmgWorkspace,
    Components, GroundedLdl, Laplacian, TerminalReason, ValidationOptions,
};
#[cfg(feature = "parallel")]
use crate::{CsrLaplacian, HierarchyLevel};
use cycle::Cycle;
use std::sync::Arc;
use std::time::Instant;

/// Timing for one hierarchy level considered during parallel-plan construction.
#[cfg(feature = "profiling")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParallelPlanLevelProfile {
    level: usize,
    vertices: usize,
    edges: usize,
    eligible: bool,
    reason: &'static str,
    construction_nanoseconds: u128,
    row_counts_nanoseconds: u128,
    row_offsets_nanoseconds: u128,
    allocation_nanoseconds: u128,
    scatter_nanoseconds: u128,
    validation_nanoseconds: u128,
    retained_bytes: usize,
}

#[cfg(feature = "profiling")]
impl ParallelPlanLevelProfile {
    /// Return the zero-based hierarchy level.
    #[must_use]
    pub const fn level(&self) -> usize {
        self.level
    }

    /// Return the level's vertex count.
    #[must_use]
    pub const fn vertices(&self) -> usize {
        self.vertices
    }

    /// Return the level's canonical edge count.
    #[must_use]
    pub const fn edges(&self) -> usize {
        self.edges
    }

    /// Return whether the production eligibility rule retained an operator.
    #[must_use]
    pub const fn eligible(&self) -> bool {
        self.eligible
    }

    /// Return the production routing reason.
    #[must_use]
    pub const fn reason(&self) -> &'static str {
        self.reason
    }

    /// Return production operator-construction wall time.
    #[must_use]
    pub const fn construction_nanoseconds(&self) -> u128 {
        self.construction_nanoseconds
    }

    /// Return row-degree counting time.
    #[must_use]
    pub const fn row_counts_nanoseconds(&self) -> u128 {
        self.row_counts_nanoseconds
    }

    /// Return prefix-sum, compact row-offset, and scatter-cursor preparation time.
    #[must_use]
    pub const fn row_offsets_nanoseconds(&self) -> u128 {
        self.row_offsets_nanoseconds
    }

    /// Return column and weight allocation/initialization time.
    #[must_use]
    pub const fn allocation_nanoseconds(&self) -> u128 {
        self.allocation_nanoseconds
    }

    /// Return deterministic edge-scatter time.
    #[must_use]
    pub const fn scatter_nanoseconds(&self) -> u128 {
        self.scatter_nanoseconds
    }

    /// Return production invariant-validation time.
    #[must_use]
    pub const fn validation_nanoseconds(&self) -> u128 {
        self.validation_nanoseconds
    }

    /// Return retained bytes for this level's operator.
    #[must_use]
    pub const fn retained_bytes(&self) -> usize {
        self.retained_bytes
    }
}

/// Production parallel-plan profile, recorded without changing the retained plan.
#[cfg(feature = "profiling")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParallelPlanBuildProfile {
    levels: Vec<ParallelPlanLevelProfile>,
    total_nanoseconds: u128,
}

#[cfg(feature = "profiling")]
impl ParallelPlanBuildProfile {
    /// Return per-level eligibility and construction records.
    #[must_use]
    pub fn levels(&self) -> &[ParallelPlanLevelProfile] {
        &self.levels
    }

    /// Return complete production-plan construction wall time.
    #[must_use]
    pub const fn total_nanoseconds(&self) -> u128 {
        self.total_nanoseconds
    }
}

/// One phase from the exact production hierarchy control path.
#[cfg(feature = "profiling")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HierarchyPhaseProfile {
    level: usize,
    phase: &'static str,
    nanoseconds: u128,
}

#[cfg(feature = "profiling")]
impl HierarchyPhaseProfile {
    /// Return the zero-based hierarchy level.
    #[must_use]
    pub const fn level(&self) -> usize {
        self.level
    }

    /// Return the production phase label.
    #[must_use]
    pub const fn phase(&self) -> &'static str {
        self.phase
    }

    /// Return elapsed wall time for this phase.
    #[must_use]
    pub const fn nanoseconds(&self) -> u128 {
        self.nanoseconds
    }
}

/// Production hierarchy and preconditioner-finalization wall times.
#[cfg(feature = "profiling")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreconditionerBuildProfile {
    hierarchy_nanoseconds: u128,
    hierarchy_phases: Vec<HierarchyPhaseProfile>,
    finalization_nanoseconds: u128,
    total_nanoseconds: u128,
}

#[cfg(feature = "profiling")]
impl PreconditionerBuildProfile {
    /// Return production hierarchy construction wall time.
    #[must_use]
    pub const fn hierarchy_nanoseconds(&self) -> u128 {
        self.hierarchy_nanoseconds
    }

    /// Return per-level records from the shared production hierarchy path.
    #[must_use]
    pub fn hierarchy_phases(&self) -> &[HierarchyPhaseProfile] {
        &self.hierarchy_phases
    }

    /// Return component metadata, centering, terminal factor, and repeat-finalization time.
    #[must_use]
    pub const fn finalization_nanoseconds(&self) -> u128 {
        self.finalization_nanoseconds
    }

    /// Return complete production preconditioner construction wall time.
    #[must_use]
    pub const fn total_nanoseconds(&self) -> u128 {
        self.total_nanoseconds
    }
}

/// Precomputed row-oriented operators for deterministic parallel CMG application.
///
/// The plan is optional and separate from [`CmgPreconditioner`], so serial users
/// retain the compact one-edge-per-undirected-edge hierarchy without duplicated
/// adjacency storage. Operators are retained only for nonterminal levels large
/// enough to use the supplied team.
#[cfg(feature = "parallel")]
#[derive(Debug, Clone)]
pub struct ParallelCmgPlan {
    level_operators: Vec<Option<CsrLaplacian>>,
    level_lineages: Vec<Arc<()>>,
}

#[cfg(feature = "parallel")]
impl ParallelCmgPlan {
    /// Peak new CMG-owned requested layouts for plan construction, including
    /// descriptor banks, retained earlier operators and current CSR scratch.
    /// Borrowed hierarchy/lineage, Rayon runtime and allocator/OS storage are
    /// excluded. This is not a complete action/runtime admission bound.
    pub fn construction_allocation_bound(
        preconditioner: &CmgPreconditioner,
        team: Team<'_>,
    ) -> Result<usize, CmgError> {
        Self::construction_allocation_bound_for(preconditioner, team.options())
    }

    /// The bound of [`Self::construction_allocation_bound`] for a team shape.
    pub(crate) fn construction_allocation_bound_for(
        preconditioner: &CmgPreconditioner,
        shape: crate::ParallelOptions,
    ) -> Result<usize, CmgError> {
        use crate::storage::{add, bytes};
        let levels = preconditioner.hierarchy.levels();
        let mut retained = add(
            bytes::<Option<CsrLaplacian>>(levels.len())?,
            bytes::<Arc<()>>(levels.len())?,
        )?;
        let mut peak = retained;
        for level in levels {
            if Self::level_is_eligible(level, shape) {
                let graph = level.graph();
                peak = peak.max(add(
                    retained,
                    CsrLaplacian::construction_allocation_bound_for_shape(graph, shape)?,
                )?);
                retained = add(
                    retained,
                    CsrLaplacian::retained_allocation_bytes(
                        graph.vertex_count(),
                        graph.edge_count(),
                    )?,
                )?;
            }
        }
        Ok(peak)
    }

    /// Admit new CMG-owned plan layouts before allocating, without changing
    /// operator eligibility or parallel setup. Requires separate admission of
    /// live parents/old plans and a policy for the uncovered Rayon runtime.
    pub fn build_with_budget(
        preconditioner: &CmgPreconditioner,
        team: Team<'_>,
        budget_bytes: usize,
    ) -> Result<Self, CmgError> {
        Self::build_with(
            preconditioner,
            BuildContext {
                team: Some(team),
                budget_bytes: Some(budget_bytes),
                ..BuildContext::default()
            },
        )
    }

    /// Build the plan under a construction context, which must carry the
    /// team. A budget admits [`Self::construction_allocation_bound`] before the
    /// first allocation. With a cancellation query every row-operator stage is
    /// counted (lanes poll inside their rows), the query is polled before every
    /// allocation and before returning, and a true query returns
    /// [`CmgError::Cancelled`] with every partial operator dropped. Operators
    /// and eligibility never depend on the context.
    pub fn build_with(
        preconditioner: &CmgPreconditioner,
        context: BuildContext<'_>,
    ) -> Result<Self, CmgError> {
        let team = context.team.ok_or(CmgError::InvalidOption {
            name: "ParallelCmgPlan requires a team",
            value: 0.0,
        })?;
        if let Some(budget) = context.budget_bytes {
            crate::storage::admit(
                Self::construction_allocation_bound(preconditioner, team)?,
                budget,
            )?;
        }
        match context.cancel {
            Some(cancel) => Self::build_levels::<false, _>(
                preconditioner,
                team,
                &mut crate::cancel::CancelMeter::new(cancel),
                |_, _, _, _| Ok(()),
            ),
            None => Self::build_levels::<false, _>(
                preconditioner,
                team,
                &mut crate::cancel::Unmetered,
                |_, _, _, _| Ok(()),
            ),
        }
    }

    pub(crate) fn eligible_operator_count(
        preconditioner: &CmgPreconditioner,
        shape: crate::ParallelOptions,
    ) -> usize {
        preconditioner
            .hierarchy
            .levels()
            .iter()
            .filter(|level| Self::level_is_eligible(level, shape))
            .count()
    }

    fn level_is_eligible(level: &HierarchyLevel, shape: crate::ParallelOptions) -> bool {
        let graph = level.graph();
        let density_floor = graph
            .vertex_count()
            .saturating_add(graph.vertex_count() / 4);
        level.terminal_reason().is_none()
            && graph.edges().len() >= density_floor
            && shape.should_parallel(graph.edges().len())
    }

    /// Build deterministic row operators for one immutable preconditioner.
    pub fn build(preconditioner: &CmgPreconditioner, team: Team<'_>) -> Result<Self, CmgError> {
        Self::build_with(
            preconditioner,
            BuildContext {
                team: Some(team),
                ..BuildContext::default()
            },
        )
    }

    /// The one plan loop: a row operator for every eligible level. `PROFILE`
    /// times each level's construction and passes it to `record` with the
    /// operator's phase timings; it never changes eligibility or an operator.
    fn build_levels<const PROFILE: bool, M: Meter>(
        preconditioner: &CmgPreconditioner,
        team: Team<'_>,
        meter: &mut M,
        mut record: impl FnMut(
            usize,
            &HierarchyLevel,
            Option<&CsrLaplacian>,
            (u128, CsrBuildProfile),
        ) -> Result<(), CmgError>,
    ) -> Result<Self, CmgError> {
        let levels = preconditioner.hierarchy.levels();
        let mut level_operators = meter.try_capacity(levels.len(), "plan operators")?;
        let mut level_lineages = meter.try_capacity(levels.len(), "plan lineages")?;
        for (index, level) in levels.iter().enumerate() {
            meter.work("plan level", 64)?;
            let graph = level.graph();
            let start = PROFILE.then(Instant::now);
            let (operator, profile) = if Self::level_is_eligible(level, team.options()) {
                let (operator, profile) =
                    CsrLaplacian::from_laplacian_with_team_impl::<PROFILE, M>(graph, team, meter)?;
                (Some(operator), profile)
            } else {
                (None, CsrBuildProfile::default())
            };
            if let Some(start) = start {
                record(
                    index,
                    level,
                    operator.as_ref(),
                    (start.elapsed().as_nanos(), profile),
                )?;
            }
            level_operators.push(operator);
            level_lineages.push(Arc::clone(graph.lineage()));
        }
        meter.checkpoint("plan publication")?;
        Ok(Self {
            level_operators,
            level_lineages,
        })
    }

    /// Build the exact production plan and retain per-level eligibility timings.
    /// Profile records are reserved fallibly before the first operator.
    #[cfg(feature = "profiling")]
    pub fn build_profiled(
        preconditioner: &CmgPreconditioner,
        team: Team<'_>,
    ) -> Result<(Self, ParallelPlanBuildProfile), CmgError> {
        let total_start = Instant::now();
        let shape = team.options();
        let mut levels =
            crate::storage::try_capacity(preconditioner.hierarchy.levels().len(), "plan profile")?;
        let plan = Self::build_levels::<true, _>(
            preconditioner,
            team,
            &mut crate::cancel::Unmetered,
            |level_index, level, operator, (construction_nanoseconds, csr)| {
                let graph = level.graph();
                let density_floor = graph
                    .vertex_count()
                    .saturating_add(graph.vertex_count() / 4);
                let reason = if level.terminal_reason().is_some() {
                    "terminal"
                } else if graph.edges().len() < density_floor {
                    "below-density-floor"
                } else if !shape.should_parallel(graph.edges().len()) {
                    "below-executor-threshold"
                } else {
                    "eligible"
                };
                // Capacity is one record per level: this push never grows.
                levels.push(ParallelPlanLevelProfile {
                    level: level_index,
                    vertices: graph.vertex_count(),
                    edges: graph.edge_count(),
                    eligible: operator.is_some(),
                    reason,
                    construction_nanoseconds,
                    row_counts_nanoseconds: csr.row_counts_nanoseconds,
                    row_offsets_nanoseconds: csr.row_offsets_nanoseconds,
                    allocation_nanoseconds: csr.allocation_nanoseconds,
                    scatter_nanoseconds: csr.scatter_nanoseconds,
                    validation_nanoseconds: csr.validation_nanoseconds,
                    retained_bytes: operator.map_or(0, CsrLaplacian::byte_len),
                });
                Ok(())
            },
        )?;
        let profile = ParallelPlanBuildProfile {
            levels,
            total_nanoseconds: total_start.elapsed().as_nanos(),
        };
        Ok((plan, profile))
    }

    /// Return the number of hierarchy levels carrying a row operator.
    #[must_use]
    pub fn operator_count(&self) -> usize {
        self.level_operators.iter().flatten().count()
    }

    /// Return retained heap bytes for all row operators.
    #[must_use]
    pub fn byte_len(&self) -> usize {
        self.level_operators
            .iter()
            .flatten()
            .map(CsrLaplacian::byte_len)
            .sum()
    }

    pub(crate) fn validate(&self, preconditioner: &CmgPreconditioner) -> Result<(), CmgError> {
        if self.level_operators.len() != preconditioner.hierarchy.levels().len()
            || self.level_lineages.len() != preconditioner.hierarchy.levels().len()
        {
            return Err(CmgError::dimension(
                "ParallelCmgPlan level count",
                preconditioner.hierarchy.levels().len(),
                self.level_operators.len().min(self.level_lineages.len()),
            ));
        }
        for ((operator, lineage), level) in self
            .level_operators
            .iter()
            .zip(&self.level_lineages)
            .zip(preconditioner.hierarchy.levels())
        {
            if !Arc::ptr_eq(lineage, level.graph().lineage())
                || operator
                    .as_ref()
                    .is_some_and(|operator| !operator.shares_lineage(level.graph()))
            {
                return Err(CmgError::InvalidHierarchy {
                    context: "parallel CMG plan belongs to a different hierarchy",
                });
            }
        }
        Ok(())
    }

    fn level_operator(&self, level_index: usize) -> Option<&CsrLaplacian> {
        self.level_operators
            .get(level_index)
            .and_then(Option::as_ref)
    }

    /// Finest-level `output = L input` with this plan's row operator, when
    /// retained, else the edge list.
    pub(crate) fn finest_matvec<M: Meter>(
        &self,
        graph: &Laplacian,
        input: &[f64],
        output: &mut [f64],
        team: Option<Team<'_>>,
        meter: &mut M,
    ) -> Result<(), CmgError> {
        match self.level_operator(0) {
            Some(operator) => operator.matvec_lanes(input, output, One, team, meter),
            None => graph.matvec_lanes(input, output, One, meter),
        }
    }
}

/// An immutable stationary CMG preconditioner.
#[derive(Debug, Clone, PartialEq)]
pub struct CmgPreconditioner {
    hierarchy: CmgHierarchy,
    finest_components: Arc<Components>,
    coarse_centering: Vec<CenteringPlan>,
    direct_terminal: Option<GroundedLdl>,
    repeat_counts: Vec<usize>,
}

crate::storage::storage_fields!(
    CmgPreconditioner,
    hierarchy,
    finest_components,
    coarse_centering,
    direct_terminal,
    repeat_counts
);
#[cfg(feature = "parallel")]
crate::storage::storage_fields!(ParallelCmgPlan, level_operators, level_lineages);

impl CmgPreconditioner {
    /// Conservative requested storage for one scalar application workspace,
    /// before constructing a hierarchy or its caches.
    ///
    /// Bounds [`Self::workspace_allocation_bytes`] when the initial vertex count,
    /// every level dimension/component count, and terminal active dimension are
    /// at most `vertices`, with at most `options.max_levels` stored levels. This
    /// includes level/dimension/centering descriptors, finest projection banks,
    /// per-level residual/coarse/centering arrays, and terminal factor work arrays.
    ///
    /// The query allocates nothing and takes constant work. All options are
    /// validated first; invalid options or size/layout overflow return an error.
    /// The bound excludes the retained preconditioner, input/output buffers,
    /// batch workspaces, parallel runtime/caches, inline root values and allocator
    /// overhead. It is neither cumulative construction admission nor an RSS bound.
    pub fn scalar_workspace_allocation_bound(
        vertices: usize,
        options: CmgOptions,
    ) -> Result<usize, CmgError> {
        let options = options.validate()?;
        CmgWorkspace::allocation_bound(vertices, options.max_levels)
    }

    /// Checked requested layouts for one scalar application workspace.
    /// Excludes allocator overhead and the retained preconditioner itself.
    pub fn workspace_allocation_bytes(&self) -> Result<usize, CmgError> {
        self.workspace_allocation_bytes_metered(&mut crate::cancel::Unmetered)
    }

    /// The one scalar-workspace layout query; a polling meter only adds polls.
    pub(crate) fn workspace_allocation_bytes_metered<M: Meter>(
        &self,
        meter: &mut M,
    ) -> Result<usize, CmgError> {
        CmgWorkspace::allocation_bytes(
            &self.hierarchy,
            self.direct_terminal.as_ref(),
            &self.finest_components,
            &self.coarse_centering,
            meter,
        )
    }

    /// The one scalar-workspace preparation; a polling meter only adds polls.
    pub(crate) fn try_workspace_metered<M: Meter>(
        &self,
        meter: &mut M,
    ) -> Result<CmgWorkspace, CmgError> {
        CmgWorkspace::try_new(
            &self.hierarchy,
            self.direct_terminal.as_ref(),
            &self.finest_components,
            &self.coarse_centering,
            meter,
        )
    }

    /// Cancellable `workspace_allocation_bytes`: the same value and errors,
    /// computed with bounded work between polls of `cancel` and one final poll.
    /// A true query returns `CmgError::Cancelled`.
    pub fn workspace_allocation_bytes_controlled(
        &self,
        cancel: crate::Cancellation<'_>,
    ) -> Result<usize, CmgError> {
        self.workspace_allocation_bytes_metered(&mut crate::cancel::CancelMeter::new(cancel))
    }

    #[cfg(feature = "parallel")]
    pub(crate) fn workspace_bytes(&self) -> usize {
        CmgWorkspace::required_bytes(
            &self.hierarchy,
            self.direct_terminal.as_ref(),
            &self.finest_components,
            &self.coarse_centering,
        )
    }

    pub(crate) fn validate_workspace(&self, workspace: &CmgWorkspace) -> Result<(), CmgError> {
        workspace.validate(
            &self.hierarchy,
            self.direct_terminal.as_ref(),
            &self.finest_components,
            &self.coarse_centering,
        )
    }

    /// Return principal retained heap bytes for the complete immutable
    /// preconditioner, including hierarchy, component metadata, terminal
    /// factorization, and recursive repeat counts.
    #[must_use]
    pub fn retained_bytes(&self) -> usize {
        self.hierarchy
            .retained_bytes()
            .saturating_add(self.component_metadata_bytes())
            .saturating_add(
                self.direct_terminal
                    .as_ref()
                    .map_or(0, GroundedLdl::byte_len),
            )
            .saturating_add(
                self.repeat_counts
                    .capacity()
                    .saturating_mul(core::mem::size_of::<usize>()),
            )
    }

    /// Build the complete hierarchy and any direct terminal factorization
    /// serially, with the ordinary component policy, no budget and no polling.
    pub fn build(graph: &Laplacian, options: CmgOptions) -> Result<Self, CmgError> {
        Self::build_with(graph, BuildOptions::from(options), BuildContext::default())
    }

    /// Conservative new requested layouts for [`Self::build_with`], including
    /// hierarchy, centering metadata and direct-factor construction for the
    /// selected component policy. `parallel` adds the parallel contraction slot
    /// bank used when a team is supplied. Borrowed graph/prepared topology,
    /// allocator/OS storage, any runtime, solve workspaces, plans and later lazy
    /// aggregation accessors are excluded. Arc layouts follow the reviewed,
    /// non-ABI-stable standard-library model. Every terminal reason except
    /// direct skips factorization; pruning only reduces the terminal size.
    pub fn construction_allocation_bound(
        vertices: usize,
        edges: usize,
        options: BuildOptions,
        parallel: bool,
    ) -> Result<usize, CmgError> {
        use crate::storage::{add, arc_bytes, bytes};
        let (n, m) = (vertices, edges);
        let hierarchy_options = options.hierarchy.validate()?;
        let (hierarchy, scratch) = CmgHierarchy::serial_allocation_parts(n, m, hierarchy_options)?;
        let count =
            n.checked_mul(hierarchy_options.max_levels)
                .ok_or(CmgError::AllocationFailed {
                    context: "centering budget overflows",
                })?;
        let metadata = add(
            add(bytes::<[usize; 2]>(count)?, arc_bytes::<Components>()?)?,
            add(
                bytes::<CenteringPlan>(hierarchy_options.max_levels - 1)?,
                bytes::<usize>(hierarchy_options.max_levels)?,
            )?,
        )?;
        // Direct terminals have n < threshold, except the n<=1/empty shortcut.
        let terminal_n = n.min(hierarchy_options.direct_threshold.saturating_sub(1).max(1));
        let factor = if options.factor_terminal_components() {
            #[cfg(feature = "experimental-components")]
            {
                GroundedLdl::component_construction_allocation_bound(terminal_n, m)?
            }
            #[cfg(not(feature = "experimental-components"))]
            unreachable!()
        } else {
            GroundedLdl::construction_allocation_bound(terminal_n)?
        };
        let finalization = add(metadata, factor.max(bytes::<[usize; 4]>(n)?))?;
        let serial = add(hierarchy, scratch.max(finalization))?;
        if parallel {
            add(serial, bytes::<Option<crate::Edge>>(m)?)
        } else {
            Ok(serial)
        }
    }

    /// Build a preconditioner under a construction context.
    ///
    /// This is the one construction pipeline: hierarchy loop, finest
    /// components, coarse centering plans, direct terminal factor and repeat
    /// adjustment. With a budget, [`Self::construction_allocation_bound`] (with
    /// `parallel` set when a team is supplied) is admitted before the first
    /// allocation and every vector bank is fallible. With a cancellation query,
    /// every traversal and numerical update is a bounded counted operation; the
    /// query is polled at the fixed cadence and before every allocation, and a
    /// true query returns [`CmgError::Cancelled`] with no partial state. With a
    /// team, edge sets above the parallel setup threshold contract and sort on
    /// the team's pool, and dense enough levels select their heavy forest edges
    /// in row-owned lanes (within the admitted contraction slot bank); forest
    /// splitting, labeling and factorization stay serial and no pool is
    /// created. Hierarchy, factor, repeats and cycle bits never depend
    /// on the context.
    pub fn build_with(
        graph: &Laplacian,
        options: BuildOptions,
        context: BuildContext<'_>,
    ) -> Result<Self, CmgError> {
        match context.cancel {
            Some(cancel) => Self::build_metered::<_, false>(
                graph,
                options,
                context,
                &mut crate::cancel::CancelMeter::new(cancel),
            ),
            None => Self::build_metered::<_, false>(graph, options, context, &mut Unmetered),
        }
        .map(|(preconditioner, _)| preconditioner)
    }

    /// Build through [`Self::build_with`]'s pipeline while timing its stages.
    /// The phase-record and profile banks are reserved fallibly, counted under
    /// the context's meter and, with a budget, admitted with the build bound.
    #[cfg(feature = "profiling")]
    pub fn build_with_profiled(
        graph: &Laplacian,
        options: BuildOptions,
        context: BuildContext<'_>,
    ) -> Result<(Self, PreconditionerBuildProfile), CmgError> {
        match context.cancel {
            Some(cancel) => Self::build_profiled_metered(
                graph,
                options,
                context,
                &mut crate::cancel::CancelMeter::new(cancel),
            ),
            None => Self::build_profiled_metered(graph, options, context, &mut Unmetered),
        }
    }

    #[cfg(feature = "profiling")]
    fn build_profiled_metered<M: Meter>(
        graph: &Laplacian,
        options: BuildOptions,
        context: BuildContext<'_>,
        meter: &mut M,
    ) -> Result<(Self, PreconditionerBuildProfile), CmgError> {
        let total_start = Instant::now();
        let (preconditioner, (hierarchy_phases, hierarchy_nanoseconds, finalization_nanoseconds)) =
            Self::build_metered::<_, true>(graph, options, context, meter)?;
        let mut phases = meter.try_capacity(hierarchy_phases.len(), "hierarchy phase profile")?;
        for record in hierarchy_phases {
            meter.work("hierarchy phase profile", 16)?;
            phases.push(HierarchyPhaseProfile {
                level: record.level,
                phase: record.phase,
                nanoseconds: record.nanoseconds,
            });
        }
        meter.checkpoint("profiled preconditioner publication")?;
        let profile = PreconditionerBuildProfile {
            hierarchy_nanoseconds,
            hierarchy_phases: phases,
            finalization_nanoseconds,
            total_nanoseconds: total_start.elapsed().as_nanos(),
        };
        Ok((preconditioner, profile))
    }

    /// Requested layouts of a profiled build's phase-record and profile banks.
    fn profile_allocation_bound(options: BuildOptions) -> Result<usize, CmgError> {
        let records =
            crate::hierarchy::phase_record_bound(options.hierarchy.validate()?.max_levels);
        #[cfg(feature = "profiling")]
        let profile = crate::storage::bytes::<HierarchyPhaseProfile>(records)?;
        #[cfg(not(feature = "profiling"))]
        let profile = 0;
        crate::storage::add(
            crate::storage::bytes::<crate::hierarchy::HierarchyPhaseRecord>(records)?,
            profile,
        )
    }

    #[allow(clippy::type_complexity)]
    fn build_metered<M: Meter, const PROFILE: bool>(
        graph: &Laplacian,
        options: BuildOptions,
        context: BuildContext<'_>,
        meter: &mut M,
    ) -> Result<
        (
            Self,
            (Vec<crate::hierarchy::HierarchyPhaseRecord>, u128, u128),
        ),
        CmgError,
    > {
        // The allocation-bound formula uses fixed-work checked scalar arithmetic.
        meter.work("preconditioner admission", 1024)?;
        if let Some(budget) = context.budget_bytes {
            let mut required = Self::construction_allocation_bound(
                graph.vertex_count(),
                graph.edge_count(),
                options,
                context.team().is_some(),
            )?;
            if PROFILE {
                required = crate::storage::add(required, Self::profile_allocation_bound(options)?)?;
            }
            crate::storage::admit(required, budget)?;
        }
        let hierarchy_start = PROFILE.then(Instant::now);
        let (hierarchy, phases) = CmgHierarchy::build_metered::<M, PROFILE>(
            graph,
            options.hierarchy,
            options.level_policy(context.budget_bytes.is_some()),
            context.team(),
            meter,
        )?;
        let hierarchy_nanoseconds = hierarchy_start.map_or(0, |start| start.elapsed().as_nanos());
        let finalization_start = PROFILE.then(Instant::now);
        let preconditioner =
            Self::finalize(hierarchy, options.factor_terminal_components(), meter)?;
        let finalization_nanoseconds =
            finalization_start.map_or(0, |start| start.elapsed().as_nanos());
        Ok((
            preconditioner,
            (phases, hierarchy_nanoseconds, finalization_nanoseconds),
        ))
    }

    /// Finest components, coarse centering plans, terminal factor and the
    /// factor-based penultimate repeat adjustment.
    pub(crate) fn finalize<M: Meter>(
        mut hierarchy: CmgHierarchy,
        factor_terminal_components: bool,
        meter: &mut M,
    ) -> Result<Self, CmgError> {
        meter.work("preconditioner finest components", 64)?;
        let finest = hierarchy
            .levels()
            .first()
            .ok_or(CmgError::InvalidHierarchy {
                context: "hierarchy contains no finest level",
            })?;
        let finest_components = if let Some(components) = finest.graph().prepared_components() {
            Arc::clone(components)
        } else {
            let components = Components::from_endpoints(
                finest.graph().vertex_count(),
                finest
                    .graph()
                    .edges()
                    .iter()
                    .map(|edge| (edge.u(), edge.v())),
                meter,
            )?;
            meter.allocation(
                "finest component owner",
                crate::storage::arc_bytes::<Components>()?,
            )?;
            Arc::new(components)
        };
        let mut coarse_centering =
            meter.try_capacity(hierarchy.levels().len() - 1, "coarse centering plans")?;
        for level in hierarchy.levels().iter().skip(1) {
            meter.work("coarse centering plan", 32)?;
            coarse_centering.push(CenteringPlan::from_laplacian(level.graph(), meter)?);
        }
        meter.work("preconditioner terminal decision", 64)?;
        let direct_terminal = if hierarchy.report().terminal_reason() == TerminalReason::Direct {
            let terminal = hierarchy
                .levels()
                .last()
                .ok_or(CmgError::InvalidHierarchy {
                    context: "hierarchy contains no terminal level",
                })?;
            // Terminal admission is implied by the whole-build bound.
            meter.work("terminal admission", 128)?;
            Some(GroundedLdl::factor_metered(
                terminal.graph(),
                factor_terminal_components,
                meter,
            )?)
        } else {
            None
        };
        let mut repeat_counts =
            meter.try_capacity(hierarchy.levels().len(), "preconditioner repeat counts")?;
        for level in hierarchy.levels() {
            meter.work("preconditioner repeat counts", 16)?;
            repeat_counts.push(level.repeat());
        }
        meter.work("preconditioner factor repeat adjustment", 128)?;
        if hierarchy.levels().len() >= 2 {
            if let Some(factor) = &direct_terminal {
                let penultimate = hierarchy.levels().len() - 2;
                let repeat = repeat_from_nonzeros(
                    hierarchy.levels()[penultimate].graph().matrix_nnz(),
                    factor.factor_nonzeros(),
                );
                repeat_counts[penultimate] = repeat;
                hierarchy.set_repeat(penultimate, repeat)?;
            }
        }
        meter.work("preconditioner publication", 64)?;
        Ok(Self {
            hierarchy,
            finest_components,
            coarse_centering,
            direct_terminal,
            repeat_counts,
        })
    }

    /// Return the immutable hierarchy.
    #[must_use]
    pub const fn hierarchy(&self) -> &CmgHierarchy {
        &self.hierarchy
    }

    /// Return effective recursive repeat counts for all levels.
    #[must_use]
    pub fn repeat_counts(&self) -> &[usize] {
        &self.repeat_counts
    }

    /// Return the direct terminal factor when the hierarchy ends directly.
    #[must_use]
    pub const fn terminal_factor(&self) -> Option<&GroundedLdl> {
        self.direct_terminal.as_ref()
    }

    pub(crate) fn matches_graph(&self, graph: &Laplacian) -> bool {
        let finest = self.hierarchy.levels()[0].graph();
        finest.shares_lineage(graph) || finest == graph
    }

    pub(crate) fn matches_prepared_topology(&self, graph: &Laplacian) -> bool {
        self.hierarchy.levels()[0]
            .graph()
            .shares_prepared_topology(graph)
    }

    pub(crate) fn finest_components(&self) -> &Components {
        &self.finest_components
    }

    pub(crate) fn finest_component_bytes(&self) -> usize {
        self.finest_components.byte_len()
    }

    /// Return retained heap bytes for fine validation and coarse centering metadata.
    #[must_use]
    pub fn component_metadata_bytes(&self) -> usize {
        self.finest_components.byte_len()
            + self
                .coarse_centering
                .iter()
                .map(CenteringPlan::byte_len)
                .sum::<usize>()
    }

    /// Allocate reusable storage compatible with this preconditioner.
    #[must_use]
    pub fn workspace(&self) -> CmgWorkspace {
        CmgWorkspace::new(
            &self.hierarchy,
            self.direct_terminal.as_ref(),
            &self.finest_components,
            &self.coarse_centering,
        )
    }

    /// Fallibly allocate reusable storage compatible with this preconditioner.
    pub fn try_workspace(&self) -> Result<CmgWorkspace, CmgError> {
        self.try_workspace_metered(&mut crate::cancel::Unmetered)
    }

    /// Cancellable `try_workspace` with the same layout, contents and errors.
    /// It polls `cancel` before every allocation, at the fixed construction
    /// cadence while initializing storage and before returning. A true query
    /// drops the partial workspace and returns `CmgError::Cancelled`.
    pub fn try_workspace_controlled(
        &self,
        cancel: crate::Cancellation<'_>,
    ) -> Result<CmgWorkspace, CmgError> {
        self.try_workspace_metered(&mut crate::cancel::CancelMeter::new(cancel))
    }

    /// Apply the preconditioner using a newly allocated workspace.
    pub fn apply(&self, rhs: &[f64]) -> Result<Vec<f64>, CmgError> {
        let mut workspace = self.workspace();
        let mut output = vec![0.0; self.hierarchy.levels()[0].graph().vertex_count()];
        self.apply_into(rhs, &mut output, &mut workspace)?;
        Ok(output)
    }

    /// Apply the preconditioner into caller-owned output and workspace,
    /// validating and projecting the right-hand side with default tolerances.
    pub fn apply_into(
        &self,
        rhs: &[f64],
        output: &mut [f64],
        workspace: &mut CmgWorkspace,
    ) -> Result<(), CmgError> {
        self.apply_projected_into(
            rhs,
            output,
            workspace,
            ValidationOptions::default(),
            ApplyContext::default(),
        )
    }

    /// Validate and project a right-hand side, then apply the fixed cycle.
    ///
    /// A component sum accepted as floating-point roundoff is projected to
    /// exact zero before the stationary CMG cycle is evaluated.
    pub fn apply_projected_into(
        &self,
        rhs: &[f64],
        output: &mut [f64],
        workspace: &mut CmgWorkspace,
        validation: ValidationOptions,
        context: ApplyContext<'_>,
    ) -> Result<(), CmgError> {
        self.validate_application(rhs, output, workspace, context, "CmgPreconditioner::apply")?;
        validation.validate()?;
        with_meter!(context.cancel, meter => {
            let mut projected_rhs = workspace.take_projected_rhs();
            let result = (|| {
                crate::cancel::chunked(meter, "CMG projected RHS copy", rhs.len(), 1, |range| {
                    projected_rhs[range.clone()].copy_from_slice(&rhs[range]);
                    Ok(())
                })?;
                let mut component_workspace = workspace.take_component();
                let projection = self.finest_components.project_rhs_in_place_with_workspace(
                    &mut projected_rhs,
                    validation,
                    &mut component_workspace,
                    meter,
                );
                workspace.put_component(component_workspace);
                projection?;
                self.cycle_scalar(&projected_rhs, output, workspace, context, meter, &mut ())
            })();
            workspace.put_projected_rhs(projected_rhs);
            result
        })
    }

    /// Apply a right-hand side already known to be component-compatible.
    ///
    /// This skips the fine-level compatibility scan and projection performed by
    /// [`Self::apply_into`]. It is intended for Krylov solvers that validate and
    /// project a submitted right-hand side once, then keep residuals in the
    /// Laplacian range. Dimension and workspace checks remain enabled;
    /// recursive coarse-level roundoff is removed by deterministic component
    /// centering. An incompatible right-hand side does not represent a
    /// solvable Laplacian system and should use [`Self::apply_into`] instead.
    pub fn apply_compatible_into(
        &self,
        rhs: &[f64],
        output: &mut [f64],
        workspace: &mut CmgWorkspace,
    ) -> Result<(), CmgError> {
        self.apply_compatible_into_with(rhs, output, workspace, ApplyContext::default())
    }

    /// Apply the fixed cycle to a compatible right-hand side under `context`.
    pub fn apply_compatible_into_with(
        &self,
        rhs: &[f64],
        output: &mut [f64],
        workspace: &mut CmgWorkspace,
        context: ApplyContext<'_>,
    ) -> Result<(), CmgError> {
        self.validate_application(
            rhs,
            output,
            workspace,
            context,
            "CmgPreconditioner::apply compatible",
        )?;
        with_meter!(context.cancel, meter => {
            self.cycle_scalar(rhs, output, workspace, context, meter, &mut ())
        })
    }

    /// Apply the cycle of [`Self::apply_compatible_into_with`] while recording
    /// exclusive phase timings; parent timings exclude recursive children.
    #[cfg(feature = "cycle-profiling")]
    pub fn profile_apply_compatible_into(
        &self,
        rhs: &[f64],
        output: &mut [f64],
        workspace: &mut CmgWorkspace,
        context: ApplyContext<'_>,
    ) -> Result<crate::CmgApplyProfile, CmgError> {
        self.validate_application(
            rhs,
            output,
            workspace,
            context,
            "CmgPreconditioner::apply compatible",
        )?;
        let mut profile = crate::CmgApplyProfile::new(self.hierarchy.levels().len());
        with_meter!(context.cancel, meter => {
            self.cycle_scalar(rhs, output, workspace, context, meter, &mut profile)
        })?;
        Ok(profile)
    }

    /// The prevalidated scalar cycle used by the solvers, under their meter.
    pub(crate) fn apply_compatible_metered<M: Meter, R: CycleRecorder>(
        &self,
        rhs: &[f64],
        output: &mut [f64],
        workspace: &mut CmgWorkspace,
        context: ApplyContext<'_>,
        meter: &mut M,
        recorder: &mut R,
    ) -> Result<(), CmgError> {
        self.cycle_scalar(rhs, output, workspace, context, meter, recorder)
    }

    fn validate_application(
        &self,
        rhs: &[f64],
        output: &[f64],
        workspace: &CmgWorkspace,
        context: ApplyContext<'_>,
        name: &'static str,
    ) -> Result<(), CmgError> {
        context.validate(self)?;
        let dimension = self.hierarchy.levels()[0].graph().vertex_count();
        if rhs.len() != dimension {
            return Err(CmgError::dimension(name, dimension, rhs.len()));
        }
        if output.len() != dimension {
            return Err(CmgError::dimension(name, dimension, output.len()));
        }
        self.validate_workspace(workspace)
    }

    fn cycle_scalar<M: Meter, R: CycleRecorder>(
        &self,
        rhs: &[f64],
        output: &mut [f64],
        workspace: &mut CmgWorkspace,
        #[cfg_attr(not(feature = "parallel"), allow(unused_variables))] context: ApplyContext<'_>,
        meter: &mut M,
        recorder: &mut R,
    ) -> Result<(), CmgError> {
        Cycle {
            cmg: self,
            store: workspace,
            #[cfg(feature = "parallel")]
            plan: context.plan,
            #[cfg(feature = "parallel")]
            team: context.team,
        }
        .level(0, rhs, output, 1, meter, recorder)
    }
}

/// Resources for one fixed-cycle application.
///
/// The default is the serial, uncancellable cycle. A [`ParallelCmgPlan`]
/// selects its row operator on every level where it retained one; a team runs
/// those levels' row kernels and transfers in parallel and centers a scalar
/// workspace's single large coarse component with fixed-chunk team sums. These
/// are the existing scalar-planned and interleaved arithmetic: results never
/// depend on the worker count beyond the plan's operator set and that
/// centering choice. With `cancel`, every kernel polls the sticky query after
/// a bounded number of processed items (edges, rows, transfer entries, factor
/// entries), parallel lanes poll it between bounded sub-ranges, and all lanes
/// join before [`CmgError::Cancelled`] is returned; outputs are then
/// unspecified and workspaces stay reusable. No context changes results.
#[derive(Clone, Copy, Default)]
pub struct ApplyContext<'a> {
    /// Row operators retained for this preconditioner's large levels.
    #[cfg(feature = "parallel")]
    pub plan: Option<&'a ParallelCmgPlan>,
    /// The borrowed host team for planned levels and scalar centering.
    #[cfg(feature = "parallel")]
    pub team: Option<Team<'a>>,
    /// The caller's sticky cancellation query.
    pub cancel: Option<Cancellation<'a>>,
}

impl core::fmt::Debug for ApplyContext<'_> {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut debug = formatter.debug_struct("ApplyContext");
        #[cfg(feature = "parallel")]
        debug
            .field("plan", &self.plan.map(ParallelCmgPlan::operator_count))
            .field("team", &self.team);
        debug.field("cancel", &self.cancel.is_some()).finish()
    }
}

impl ApplyContext<'_> {
    fn validate(&self, cmg: &CmgPreconditioner) -> Result<(), CmgError> {
        #[cfg(feature = "parallel")]
        if let Some(plan) = self.plan {
            plan.validate(cmg)?;
        }
        let _ = cmg;
        Ok(())
    }
}

fn repeat_from_nonzeros(fine_nonzeros: usize, denominator_nonzeros: usize) -> usize {
    if denominator_nonzeros == 0 {
        return 1;
    }
    (fine_nonzeros / denominator_nonzeros)
        .saturating_sub(1)
        .max(1)
}

#[cfg(all(test, feature = "experimental-components"))]
// Context literals keep `..Default::default()` for their feature-gated fields.
#[allow(clippy::needless_update)]
mod controlled_build_tests {
    use super::*;
    use crate::ComponentBuildOptions;

    #[test]
    fn rounded_matvec_probe_matches_unchanged_cycle_on_the_selected_hierarchy() {
        // Preserve the original weak-edge probe as a diagnostic, not as a
        // relaxed linearity certificate. Floating L*x and affine combinations
        // need not lie exactly in the component range, and cancellation can be
        // amplified by the weak edge. Exact compatible dipoles retain the
        // independent 1e-11 checks in tests/controlled_build.rs.
        for disconnected in [false, true] {
            for scale in [1e-90, 1.0, 1e90] {
                let components = if disconnected { 2 } else { 1 };
                let n = if disconnected { 50 } else { 24 };
                let graph = Laplacian::from_edges(
                    n,
                    (0..components).flat_map(|c| {
                        (1..24).map(move |v| {
                            (
                                c * 24 + v - 1,
                                c * 24 + v,
                                scale
                                    * if v == 12 {
                                        1.0 / 65536.0
                                    } else {
                                        1.0 + (v % 5) as f64 / 4.0
                                    },
                            )
                        })
                    }),
                )
                .unwrap();
                let selected = CmgPreconditioner::build_with(
                    &graph,
                    BuildOptions {
                        hierarchy: CmgOptions {
                            max_levels: 8,
                            ..Default::default()
                        },
                        components: ComponentBuildOptions {
                            prune_coarse_isolates: true,
                            preserve_unpruned_stopping: true,
                            factor_terminal_components: true,
                        },
                    },
                    BuildContext {
                        cancel: Some(&|| false),
                        budget_bytes: Some(usize::MAX),
                        ..BuildContext::default()
                    },
                )
                .unwrap();
                let legacy =
                    CmgPreconditioner::finalize(selected.hierarchy.clone(), true, &mut Unmetered)
                        .unwrap();
                assert_eq!(selected, legacy);
                let u = graph
                    .matvec(&(0..n).map(|i| (i % 7) as f64 - 3.0).collect::<Vec<_>>())
                    .unwrap();
                let v = graph
                    .matvec(
                        &(0..n)
                            .map(|i| ((i * 3) % 11) as f64 - 5.0)
                            .collect::<Vec<_>>(),
                    )
                    .unwrap();
                let input: Vec<_> = u.iter().zip(&v).map(|(a, b)| 1.25 * a - 0.75 * b).collect();
                let outputs: Vec<_> = [&u, &v, &input]
                    .into_iter()
                    .map(|rhs| {
                        let output = selected.apply(rhs).unwrap();
                        let old = legacy.apply(rhs).unwrap();
                        assert_eq!(
                            output.iter().map(|x| x.to_bits()).collect::<Vec<_>>(),
                            old.iter().map(|x| x.to_bits()).collect::<Vec<_>>()
                        );
                        output
                    })
                    .collect();
                let expected: Vec<_> = outputs[0]
                    .iter()
                    .zip(&outputs[1])
                    .map(|(a, b)| 1.25 * a - 0.75 * b)
                    .collect();
                let output_scale = outputs[2]
                    .iter()
                    .chain(&expected)
                    .fold(0.0_f64, |s, x| s.max(x.abs()));
                let defect = outputs[2]
                    .iter()
                    .zip(&expected)
                    .fold(0.0_f64, |s, (a, b)| s.max((a - b).abs()))
                    / output_scale;
                let moment_defect = [&u, &v, &input]
                    .into_iter()
                    .flat_map(|rhs| {
                        (0..components).map(move |c| {
                            rhs[c * 24..(c + 1) * 24].iter().sum::<f64>().abs() / scale
                        })
                    })
                    .fold(0.0_f64, f64::max);
                assert!(defect.is_finite() && moment_defect.is_finite());
                eprintln!(
                    "rounded matvec diagnostic: disconnected={disconnected}, scale={scale:e}, relative_linearity_defect={defect:e}, component_sum_defect_over_edge_scale={moment_defect:e}; legacy selected-hierarchy action is bit-exact"
                );
            }
        }
    }
}

/// Profiled builds reserve their record banks fallibly under the context:
/// admitted with the build bound, polled by its meter, same preconditioner.
#[cfg(all(test, feature = "profiling"))]
mod profiled_storage_tests {
    use super::*;

    #[test]
    fn profile_banks_are_admitted_and_cancellable() {
        let graph = Laplacian::from_edges(
            600,
            (0..600).flat_map(|i| {
                (1..=3)
                    .filter(move |d| i + d < 600)
                    .map(move |d| (i, i + d, 1.0))
            }),
        )
        .unwrap();
        let options = BuildOptions::from(CmgOptions {
            direct_threshold: 32,
            ..CmgOptions::default()
        });
        let ordinary =
            CmgPreconditioner::build_with(&graph, options, BuildContext::default()).unwrap();
        let bound = CmgPreconditioner::construction_allocation_bound(
            graph.vertex_count(),
            graph.edge_count(),
            options,
            false,
        )
        .unwrap()
            + CmgPreconditioner::profile_allocation_bound(options).unwrap();
        let profiled = |budget: usize, cancel: Option<crate::Cancellation<'_>>| {
            CmgPreconditioner::build_with_profiled(
                &graph,
                options,
                BuildContext {
                    team: None,
                    cancel,
                    budget_bytes: Some(budget),
                },
            )
        };
        let (built, profile) = profiled(bound, None).unwrap();
        assert_eq!(built.hierarchy(), ordinary.hierarchy());
        assert!(!profile.hierarchy_phases().is_empty());
        assert!(matches!(
            profiled(bound - 1, None),
            Err(CmgError::MemoryBudgetExceeded { .. })
        ));
        // Cancellation reaches the record reserve and the profile conversion.
        let polls = std::sync::atomic::AtomicUsize::new(0);
        let count = || {
            polls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            false
        };
        profiled(bound, Some(&count)).unwrap();
        let total = polls.load(std::sync::atomic::Ordering::SeqCst);
        let mut phases = std::collections::BTreeSet::new();
        for stop in 0..total {
            let calls = std::sync::atomic::AtomicUsize::new(0);
            let cancel = || calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst) >= stop;
            match profiled(bound, Some(&cancel)) {
                Err(CmgError::Cancelled { phase }) => {
                    phases.insert(phase);
                }
                other => panic!("poll {stop} of {total}: {:?}", other.err()),
            }
        }
        for phase in [
            "hierarchy profile records",
            "hierarchy phase profile",
            "profiled preconditioner publication",
        ] {
            assert!(phases.contains(phase), "{phase} not in {phases:?}");
        }
    }
}

/// The planned matvec gate counts entries times lanes, not rows: a coarse level
/// with fewer rows than `min_parallel_len` but many entries runs on the team,
/// and every lane count reproduces the serial row products bit for bit.
#[cfg(all(test, feature = "parallel"))]
mod matvec_gate_tests {
    use crate::cancel::Unmetered;
    use crate::{
        CmgOptions, CmgPreconditioner, Laplacian, ParallelCmgPlan, ParallelExecutor,
        ParallelOptions,
    };

    /// Workers with three spells at skewed firms plus a firm path: coarse
    /// levels of 700 to 1,100 vertices carry more than 20,000 edges each.
    fn worker_firm() -> Laplacian {
        let (workers, firms) = (12_000, 1_500);
        let mut state = 7_u64;
        let mut below = |bound: usize| {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            ((state >> 11) % bound as u64) as usize
        };
        let mut edges = Vec::new();
        for worker in 0..workers {
            for _ in 0..3 {
                let firm = below(firms).min(below(firms));
                edges.push((worker, workers + firm, 1.0 + below(4) as f64));
            }
        }
        edges.extend((1..firms).map(|firm| (workers + firm - 1, workers + firm, 0.5)));
        Laplacian::from_edges(workers + firms, edges).unwrap()
    }

    #[test]
    fn dense_levels_below_the_row_threshold_run_on_the_team_with_serial_bits() {
        let graph = worker_firm();
        let preconditioner = CmgPreconditioner::build(&graph, CmgOptions::default()).unwrap();
        let levels = preconditioner.hierarchy().levels();
        let threshold = ParallelOptions::default().min_parallel_len;
        for threads in [2, 4, 8] {
            let executor = ParallelExecutor::new(ParallelOptions {
                threads,
                ..ParallelOptions::default()
            })
            .unwrap();
            let team = executor.team();
            let plan = ParallelCmgPlan::build(&preconditioner, team).unwrap();
            let mut straddling = 0;
            for (index, level) in levels.iter().enumerate() {
                let Some(operator) = plan.level_operator(index) else {
                    continue;
                };
                let rows = operator.vertex_count();
                for columns in [1, 3] {
                    // A planned level has at least `min_parallel_len` edges, so
                    // its entries open the gate; a one-lane view stays serial.
                    assert!(operator.matvec_uses_team(team, columns));
                    assert!(!operator.matvec_uses_team(team.single_lane(), columns));
                    let input: Vec<f64> = (0..rows * columns)
                        .map(|i| ((i * 37) % 101) as f64 / 8.0 - 6.0)
                        .collect();
                    let mut serial = vec![0.0; rows * columns];
                    let mut parallel = vec![f64::NAN; rows * columns];
                    // `usize` is the interleaved width; one lane is the scalar layout.
                    operator
                        .matvec_lanes(&input, &mut serial, columns, None, &mut Unmetered)
                        .unwrap();
                    operator
                        .matvec_lanes(&input, &mut parallel, columns, Some(team), &mut Unmetered)
                        .unwrap();
                    let bits =
                        |values: &[f64]| values.iter().map(|v| v.to_bits()).collect::<Vec<_>>();
                    assert_eq!(
                        bits(&parallel),
                        bits(&serial),
                        "{threads} threads, level {index}"
                    );
                }
                if rows < threshold {
                    straddling += 1;
                    assert!(level.graph().edge_count() * 2 >= threshold);
                }
            }
            assert!(levels[0].graph().vertex_count() >= threshold);
            assert!(
                straddling >= 2,
                "{threads} threads: {straddling} straddling levels"
            );
        }
    }

    #[test]
    fn tiny_operators_stay_serial() {
        let graph = Laplacian::from_edges(10, (0..9).map(|v| (v, v + 1, 1.0))).unwrap();
        let operator = crate::CsrLaplacian::from_laplacian(&graph).unwrap();
        let executor = ParallelExecutor::new(ParallelOptions {
            threads: 8,
            ..ParallelOptions::default()
        })
        .unwrap();
        // 18 entries times 64 lanes stay below the default 4,096 units; 228
        // lanes reach them.
        assert!(!operator.matvec_uses_team(executor.team(), 1));
        assert!(!operator.matvec_uses_team(executor.team(), 64));
        assert!(operator.matvec_uses_team(executor.team(), 228));
    }
}
