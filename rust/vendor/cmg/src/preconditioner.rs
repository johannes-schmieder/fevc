//! Stationary recursive CMG preconditioner application.

use crate::components::CenteringPlan;
use crate::{
    CmgError, CmgHierarchy, CmgOptions, CmgWorkspace, Components, GroundedLdl, Laplacian,
    TerminalReason, ValidationOptions,
};
#[cfg(feature = "parallel")]
use crate::{CsrLaplacian, ParallelExecutor};
#[cfg(feature = "parallel")]
use rayon::prelude::*;
#[cfg(feature = "parallel")]
use std::sync::Arc;
#[cfg(feature = "profiling")]
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

    /// Return prefix-sum and compact row-offset construction time.
    #[must_use]
    pub const fn row_offsets_nanoseconds(&self) -> u128 {
        self.row_offsets_nanoseconds
    }

    /// Return row cursor, column, and weight allocation/initialization time.
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
/// enough to use the supplied executor.
#[cfg(feature = "parallel")]
#[derive(Debug, Clone)]
pub struct ParallelCmgPlan {
    level_operators: Vec<Option<CsrLaplacian>>,
    level_lineages: Vec<Arc<()>>,
}

#[cfg(feature = "parallel")]
impl ParallelCmgPlan {
    /// Build deterministic row operators for one immutable preconditioner.
    pub fn build(
        preconditioner: &CmgPreconditioner,
        executor: &ParallelExecutor,
    ) -> Result<Self, CmgError> {
        let level_operators = preconditioner
            .hierarchy
            .levels()
            .iter()
            .map(|level| {
                let graph = level.graph();
                let density_floor = graph
                    .vertex_count()
                    .saturating_add(graph.vertex_count() / 4);
                if level.terminal_reason().is_none()
                    && graph.edges().len() >= density_floor
                    && executor.should_parallel(graph.edges().len())
                {
                    CsrLaplacian::from_laplacian(graph).map(Some)
                } else {
                    Ok(None)
                }
            })
            .collect::<Result<Vec<_>, CmgError>>()?;
        let level_lineages = preconditioner
            .hierarchy
            .levels()
            .iter()
            .map(|level| Arc::clone(level.graph().lineage()))
            .collect();
        Ok(Self {
            level_operators,
            level_lineages,
        })
    }

    /// Build the exact production plan and retain per-level eligibility timings.
    #[cfg(feature = "profiling")]
    pub fn build_profiled(
        preconditioner: &CmgPreconditioner,
        executor: &ParallelExecutor,
    ) -> Result<(Self, ParallelPlanBuildProfile), CmgError> {
        let total_start = Instant::now();
        let mut level_operators = Vec::with_capacity(preconditioner.hierarchy.levels().len());
        let mut profiles = Vec::with_capacity(preconditioner.hierarchy.levels().len());
        for (level_index, level) in preconditioner.hierarchy.levels().iter().enumerate() {
            let graph = level.graph();
            let density_floor = graph
                .vertex_count()
                .saturating_add(graph.vertex_count() / 4);
            let reason = if level.terminal_reason().is_some() {
                "terminal"
            } else if graph.edges().len() < density_floor {
                "below-density-floor"
            } else if !executor.should_parallel(graph.edges().len()) {
                "below-executor-threshold"
            } else {
                "eligible"
            };
            let eligible = reason == "eligible";
            let start = Instant::now();
            let (operator, csr_profile) = if eligible {
                let (operator, profile) = CsrLaplacian::from_laplacian_profiled(graph)?;
                (Some(operator), profile)
            } else {
                (None, Default::default())
            };
            let construction_nanoseconds = start.elapsed().as_nanos();
            profiles.push(ParallelPlanLevelProfile {
                level: level_index,
                vertices: graph.vertex_count(),
                edges: graph.edge_count(),
                eligible,
                reason,
                construction_nanoseconds,
                row_counts_nanoseconds: csr_profile.row_counts_nanoseconds,
                row_offsets_nanoseconds: csr_profile.row_offsets_nanoseconds,
                allocation_nanoseconds: csr_profile.allocation_nanoseconds,
                scatter_nanoseconds: csr_profile.scatter_nanoseconds,
                validation_nanoseconds: csr_profile.validation_nanoseconds,
                retained_bytes: operator.as_ref().map_or(0, CsrLaplacian::byte_len),
            });
            level_operators.push(operator);
        }
        let level_lineages = preconditioner
            .hierarchy
            .levels()
            .iter()
            .map(|level| Arc::clone(level.graph().lineage()))
            .collect();
        let plan = Self {
            level_operators,
            level_lineages,
        };
        let profile = ParallelPlanBuildProfile {
            levels: profiles,
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

    /// Apply a component-compatible right-hand side with deterministic parallel
    /// level kernels and caller-owned workspace.
    pub fn apply_compatible_into(
        &self,
        preconditioner: &CmgPreconditioner,
        rhs: &[f64],
        output: &mut [f64],
        workspace: &mut CmgWorkspace,
        executor: &ParallelExecutor,
    ) -> Result<(), CmgError> {
        self.apply_compatible_into_with_validation(
            preconditioner,
            rhs,
            output,
            workspace,
            ValidationOptions::default(),
            executor,
        )
    }

    /// Apply a compatible right-hand side with explicit validation tolerances.
    pub fn apply_compatible_into_with_validation(
        &self,
        preconditioner: &CmgPreconditioner,
        rhs: &[f64],
        output: &mut [f64],
        workspace: &mut CmgWorkspace,
        validation: ValidationOptions,
        executor: &ParallelExecutor,
    ) -> Result<(), CmgError> {
        preconditioner
            .apply_compatible_into_with_plan(rhs, output, workspace, validation, self, executor)
    }

    pub(crate) fn apply_compatible_into_prevalidated(
        &self,
        preconditioner: &CmgPreconditioner,
        rhs: &[f64],
        output: &mut [f64],
        workspace: &mut CmgWorkspace,
        validation: ValidationOptions,
        executor: &ParallelExecutor,
    ) -> Result<(), CmgError> {
        preconditioner.apply_compatible_into_with_prevalidated_plan(
            rhs, output, workspace, validation, self, executor,
        )
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

    pub(crate) fn finest_matvec_into(
        &self,
        graph: &Laplacian,
        input: &[f64],
        output: &mut [f64],
        executor: &ParallelExecutor,
    ) -> Result<(), CmgError> {
        self.matvec_into(0, graph, input, output, executor)
    }

    fn matvec_into(
        &self,
        level_index: usize,
        graph: &Laplacian,
        input: &[f64],
        output: &mut [f64],
        executor: &ParallelExecutor,
    ) -> Result<(), CmgError> {
        match self.level_operator(level_index) {
            Some(operator) => operator.matvec_into_parallel(input, output, executor),
            None => graph.matvec_into(input, output),
        }
    }
}

/// An immutable stationary CMG preconditioner.
#[derive(Debug, Clone, PartialEq)]
pub struct CmgPreconditioner {
    hierarchy: CmgHierarchy,
    finest_components: Components,
    coarse_centering: Vec<CenteringPlan>,
    direct_terminal: Option<GroundedLdl>,
    repeat_counts: Vec<usize>,
}

impl CmgPreconditioner {
    /// Build the complete hierarchy and any direct terminal factorization.
    pub fn build(graph: &Laplacian, options: CmgOptions) -> Result<Self, CmgError> {
        Self::from_hierarchy(CmgHierarchy::build(graph, options)?)
    }

    /// Build with deterministic parallel hierarchy contraction and sorting.
    ///
    /// The resulting hierarchy, terminal factor, and repeat counts are exactly
    /// the same as [`Self::build`].
    #[cfg(feature = "parallel")]
    pub fn build_with_executor(
        graph: &Laplacian,
        options: CmgOptions,
        executor: &ParallelExecutor,
    ) -> Result<Self, CmgError> {
        Self::from_hierarchy(CmgHierarchy::build_with_executor(graph, options, executor)?)
    }

    /// Build through the production hierarchy and finalization paths while timing both stages.
    #[cfg(all(feature = "parallel", feature = "profiling"))]
    pub fn build_with_executor_profiled(
        graph: &Laplacian,
        options: CmgOptions,
        executor: &ParallelExecutor,
    ) -> Result<(Self, PreconditionerBuildProfile), CmgError> {
        let total_start = Instant::now();
        let hierarchy_start = Instant::now();
        let (hierarchy, hierarchy_phases) =
            CmgHierarchy::build_with_executor_profiled(graph, options, executor)?;
        let hierarchy_nanoseconds = hierarchy_start.elapsed().as_nanos();
        let finalization_start = Instant::now();
        let preconditioner = Self::from_hierarchy(hierarchy)?;
        let finalization_nanoseconds = finalization_start.elapsed().as_nanos();
        let profile = PreconditionerBuildProfile {
            hierarchy_nanoseconds,
            hierarchy_phases: hierarchy_phases
                .into_iter()
                .map(|record| HierarchyPhaseProfile {
                    level: record.level,
                    phase: record.phase,
                    nanoseconds: record.nanoseconds,
                })
                .collect(),
            finalization_nanoseconds,
            total_nanoseconds: total_start.elapsed().as_nanos(),
        };
        Ok((preconditioner, profile))
    }

    fn from_hierarchy(mut hierarchy: CmgHierarchy) -> Result<Self, CmgError> {
        let finest = hierarchy
            .levels()
            .first()
            .ok_or(CmgError::InvalidHierarchy {
                context: "hierarchy contains no finest level",
            })?;
        let finest_components = Components::from_laplacian(finest.graph());
        let coarse_centering = hierarchy
            .levels()
            .iter()
            .skip(1)
            .map(|level| CenteringPlan::from_laplacian(level.graph()))
            .collect();
        let direct_terminal = if hierarchy.report().terminal_reason() == TerminalReason::Direct {
            let terminal = hierarchy
                .levels()
                .last()
                .ok_or(CmgError::InvalidHierarchy {
                    context: "hierarchy contains no terminal level",
                })?;
            Some(GroundedLdl::factor(terminal.graph())?)
        } else {
            None
        };

        let mut repeat_counts: Vec<usize> = hierarchy
            .levels()
            .iter()
            .map(|level| level.repeat())
            .collect();
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

    pub(crate) fn finest_components(&self) -> &Components {
        &self.finest_components
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

    /// Apply the preconditioner using a newly allocated workspace.
    pub fn apply(&self, rhs: &[f64]) -> Result<Vec<f64>, CmgError> {
        let mut workspace = self.workspace();
        let mut output = vec![0.0; self.hierarchy.levels()[0].graph().vertex_count()];
        self.apply_into(rhs, &mut output, &mut workspace)?;
        Ok(output)
    }

    /// Apply the preconditioner into caller-owned output and workspace.
    pub fn apply_into(
        &self,
        rhs: &[f64],
        output: &mut [f64],
        workspace: &mut CmgWorkspace,
    ) -> Result<(), CmgError> {
        self.apply_into_with_validation(rhs, output, workspace, ValidationOptions::default())
    }

    /// Apply a right-hand side already known to be component-compatible.
    ///
    /// This skips the fine-level compatibility scan and projection performed by
    /// [`Self::apply_into`]. It is intended for Krylov solvers that validate and
    /// project a submitted right-hand side once, then keep residuals in the
    /// Laplacian range. Dimension, workspace, and option checks remain enabled;
    /// recursive coarse-level roundoff is removed by deterministic component centering.
    pub fn apply_compatible_into(
        &self,
        rhs: &[f64],
        output: &mut [f64],
        workspace: &mut CmgWorkspace,
    ) -> Result<(), CmgError> {
        self.apply_compatible_into_with_validation(
            rhs,
            output,
            workspace,
            ValidationOptions::default(),
        )
    }

    /// Apply an already compatible right-hand side with explicit validation
    /// tolerances for public validation; recursive coarse residuals are centered.
    ///
    /// Callers are responsible for ensuring component-wise compatibility. An
    /// incompatible right-hand side does not represent a solvable Laplacian
    /// system and should use [`Self::apply_into`] instead.
    pub fn apply_compatible_into_with_validation(
        &self,
        rhs: &[f64],
        output: &mut [f64],
        workspace: &mut CmgWorkspace,
        validation: ValidationOptions,
    ) -> Result<(), CmgError> {
        let dimension = self.hierarchy.levels()[0].graph().vertex_count();
        if rhs.len() != dimension {
            return Err(CmgError::dimension(
                "CmgPreconditioner::apply compatible rhs",
                dimension,
                rhs.len(),
            ));
        }
        if output.len() != dimension {
            return Err(CmgError::dimension(
                "CmgPreconditioner::apply compatible output",
                dimension,
                output.len(),
            ));
        }
        workspace.validate(
            &self.hierarchy,
            self.direct_terminal.as_ref(),
            &self.finest_components,
            &self.coarse_centering,
        )?;
        validation.validate()?;
        self.apply_level(0, rhs, output, workspace, 1)
    }

    #[cfg(feature = "parallel")]
    fn apply_compatible_into_with_plan(
        &self,
        rhs: &[f64],
        output: &mut [f64],
        workspace: &mut CmgWorkspace,
        validation: ValidationOptions,
        plan: &ParallelCmgPlan,
        executor: &ParallelExecutor,
    ) -> Result<(), CmgError> {
        plan.validate(self)?;
        self.apply_compatible_into_with_prevalidated_plan(
            rhs, output, workspace, validation, plan, executor,
        )
    }

    #[cfg(feature = "parallel")]
    fn apply_compatible_into_with_prevalidated_plan(
        &self,
        rhs: &[f64],
        output: &mut [f64],
        workspace: &mut CmgWorkspace,
        validation: ValidationOptions,
        plan: &ParallelCmgPlan,
        executor: &ParallelExecutor,
    ) -> Result<(), CmgError> {
        let dimension = self.hierarchy.levels()[0].graph().vertex_count();
        if rhs.len() != dimension {
            return Err(CmgError::dimension(
                "ParallelCmgPlan::apply compatible rhs",
                dimension,
                rhs.len(),
            ));
        }
        if output.len() != dimension {
            return Err(CmgError::dimension(
                "ParallelCmgPlan::apply compatible output",
                dimension,
                output.len(),
            ));
        }
        workspace.validate(
            &self.hierarchy,
            self.direct_terminal.as_ref(),
            &self.finest_components,
            &self.coarse_centering,
        )?;
        validation.validate()?;
        self.apply_level_with_plan(0, rhs, output, workspace, 1, plan, executor)
    }

    /// Apply with explicit compatibility-validation tolerances.
    ///
    /// A component sum accepted as floating-point roundoff is projected to
    /// exact zero before the stationary CMG cycle is evaluated.
    pub fn apply_into_with_validation(
        &self,
        rhs: &[f64],
        output: &mut [f64],
        workspace: &mut CmgWorkspace,
        validation: ValidationOptions,
    ) -> Result<(), CmgError> {
        let dimension = self.hierarchy.levels()[0].graph().vertex_count();
        if rhs.len() != dimension {
            return Err(CmgError::dimension(
                "CmgPreconditioner::apply rhs",
                dimension,
                rhs.len(),
            ));
        }
        if output.len() != dimension {
            return Err(CmgError::dimension(
                "CmgPreconditioner::apply output",
                dimension,
                output.len(),
            ));
        }
        workspace.validate(
            &self.hierarchy,
            self.direct_terminal.as_ref(),
            &self.finest_components,
            &self.coarse_centering,
        )?;
        let mut projected_rhs = workspace.take_projected_rhs();
        projected_rhs.copy_from_slice(rhs);
        let result = (|| {
            let mut component_workspace = workspace.take_component();
            let projection = self.finest_components.project_rhs_in_place_with_workspace(
                &mut projected_rhs,
                validation,
                &mut component_workspace,
            );
            workspace.put_component(component_workspace);
            projection?;
            self.apply_level(0, &projected_rhs, output, workspace, 1)
        })();
        workspace.put_projected_rhs(projected_rhs);
        result
    }

    #[cfg(feature = "parallel")]
    fn apply_level_with_plan(
        &self,
        level_index: usize,
        rhs: &[f64],
        output: &mut [f64],
        workspace: &mut CmgWorkspace,
        iterations: usize,
        plan: &ParallelCmgPlan,
        executor: &ParallelExecutor,
    ) -> Result<(), CmgError> {
        let level = &self.hierarchy.levels()[level_index];
        let dimension = level.graph().vertex_count();
        if rhs.len() != dimension || output.len() != dimension {
            return Err(CmgError::InvalidHierarchy {
                context: "parallel recursive vector dimension does not match hierarchy level",
            });
        }

        if let Some(reason) = level.terminal_reason() {
            if reason == TerminalReason::Direct {
                let factor = self
                    .direct_terminal
                    .as_ref()
                    .ok_or(CmgError::InvalidHierarchy {
                        context: "direct terminal is missing its LDL factor",
                    })?;
                let mut local = workspace.take_level(level_index);
                let result = factor.solve_into_compatible(
                    rhs,
                    output,
                    &mut local.factor_forward,
                    &mut local.factor_solution,
                );
                workspace.put_level(level_index, local);
                return result;
            }
            assign_scaled_planned(output, level.inverse_diagonal(), rhs, executor, false);
            return Ok(());
        }

        let aggregation = level.aggregation().ok_or(CmgError::InvalidHierarchy {
            context: "nonterminal level has no aggregation",
        })?;
        if iterations == 0 {
            return Err(CmgError::InvalidHierarchy {
                context: "nonterminal level has zero stationary iterations",
            });
        }
        let child_iterations = self.repeat_counts[level_index];
        if child_iterations == 0 {
            return Err(CmgError::InvalidHierarchy {
                context: "nonterminal level has zero child recursive repeats",
            });
        }
        let parallel_level = plan.level_operator(level_index).is_some();

        let mut local = workspace.take_level(level_index);
        let result = (|| {
            fill_planned(output, 0.0, executor, parallel_level);
            for iteration in 0..iterations {
                if iteration == 0 {
                    assign_scaled_planned(
                        output,
                        level.inverse_diagonal(),
                        rhs,
                        executor,
                        parallel_level,
                    );
                } else {
                    plan.matvec_into(
                        level_index,
                        level.graph(),
                        output,
                        &mut local.residual,
                        executor,
                    )?;
                    jacobi_add_planned(
                        output,
                        level.inverse_diagonal(),
                        rhs,
                        &local.residual,
                        executor,
                        parallel_level,
                    );
                }

                plan.matvec_into(
                    level_index,
                    level.graph(),
                    output,
                    &mut local.residual,
                    executor,
                )?;
                residual_from_matvec_planned(&mut local.residual, rhs, executor, parallel_level);
                aggregation.restrict_into(&local.residual, &mut local.coarse_rhs)?;
                let centering = &self.coarse_centering[level_index];
                let mut centering_workspace = workspace.take_centering(level_index);
                let centering_result = centering.center_in_place_with_workspace(
                    &mut local.coarse_rhs,
                    &mut centering_workspace,
                );
                workspace.put_centering(level_index, centering_workspace);
                centering_result?;
                fill_planned(&mut local.coarse_correction, 0.0, executor, parallel_level);
                self.apply_level_with_plan(
                    level_index + 1,
                    &local.coarse_rhs,
                    &mut local.coarse_correction,
                    workspace,
                    child_iterations,
                    plan,
                    executor,
                )?;
                if parallel_level {
                    aggregation.prolong_add_into_with_executor(
                        &local.coarse_correction,
                        output,
                        executor,
                    )?;
                } else {
                    aggregation.prolong_add_into(&local.coarse_correction, output)?;
                }

                plan.matvec_into(
                    level_index,
                    level.graph(),
                    output,
                    &mut local.residual,
                    executor,
                )?;
                jacobi_add_planned(
                    output,
                    level.inverse_diagonal(),
                    rhs,
                    &local.residual,
                    executor,
                    parallel_level,
                );
            }
            Ok(())
        })();
        workspace.put_level(level_index, local);
        result
    }

    fn apply_level(
        &self,
        level_index: usize,
        rhs: &[f64],
        output: &mut [f64],
        workspace: &mut CmgWorkspace,
        iterations: usize,
    ) -> Result<(), CmgError> {
        let level = &self.hierarchy.levels()[level_index];
        let dimension = level.graph().vertex_count();
        if rhs.len() != dimension || output.len() != dimension {
            return Err(CmgError::InvalidHierarchy {
                context: "recursive vector dimension does not match hierarchy level",
            });
        }

        if let Some(reason) = level.terminal_reason() {
            if reason == TerminalReason::Direct {
                let factor = self
                    .direct_terminal
                    .as_ref()
                    .ok_or(CmgError::InvalidHierarchy {
                        context: "direct terminal is missing its LDL factor",
                    })?;
                let mut local = workspace.take_level(level_index);
                let result = factor.solve_into_compatible(
                    rhs,
                    output,
                    &mut local.factor_forward,
                    &mut local.factor_solution,
                );
                workspace.put_level(level_index, local);
                return result;
            }
            for ((value, inverse_diagonal), rhs_value) in
                output.iter_mut().zip(level.inverse_diagonal()).zip(rhs)
            {
                *value = *inverse_diagonal * *rhs_value;
            }
            return Ok(());
        }

        let aggregation = level.aggregation().ok_or(CmgError::InvalidHierarchy {
            context: "nonterminal level has no aggregation",
        })?;
        if iterations == 0 {
            return Err(CmgError::InvalidHierarchy {
                context: "nonterminal level has zero stationary iterations",
            });
        }
        let child_iterations = self.repeat_counts[level_index];
        if child_iterations == 0 {
            return Err(CmgError::InvalidHierarchy {
                context: "nonterminal level has zero child recursive repeats",
            });
        }

        let mut local = workspace.take_level(level_index);
        let result = (|| {
            output.fill(0.0);
            for iteration in 0..iterations {
                if iteration == 0 {
                    for ((value, inverse_diagonal), rhs_value) in
                        output.iter_mut().zip(level.inverse_diagonal()).zip(rhs)
                    {
                        *value = *inverse_diagonal * *rhs_value;
                    }
                } else {
                    level.graph().matvec_into(output, &mut local.residual)?;
                    for (((value, inverse_diagonal), rhs_value), matrix_value) in output
                        .iter_mut()
                        .zip(level.inverse_diagonal())
                        .zip(rhs)
                        .zip(&local.residual)
                    {
                        *value += *inverse_diagonal * (*rhs_value - *matrix_value);
                    }
                }

                level.graph().matvec_into(output, &mut local.residual)?;
                for (residual, rhs_value) in local.residual.iter_mut().zip(rhs) {
                    *residual = *rhs_value - *residual;
                }
                aggregation.restrict_into(&local.residual, &mut local.coarse_rhs)?;
                let centering = &self.coarse_centering[level_index];
                let mut centering_workspace = workspace.take_centering(level_index);
                // Restricted residuals are component-compatible in exact
                // arithmetic. Remove only floating-point null-space drift before
                // the recursive solve instead of repeating full public-boundary
                // compatibility validation and exact correction passes.
                let centering_result = centering.center_in_place_with_workspace(
                    &mut local.coarse_rhs,
                    &mut centering_workspace,
                );
                workspace.put_centering(level_index, centering_workspace);
                centering_result?;
                local.coarse_correction.fill(0.0);
                self.apply_level(
                    level_index + 1,
                    &local.coarse_rhs,
                    &mut local.coarse_correction,
                    workspace,
                    child_iterations,
                )?;
                aggregation.prolong_add_into(&local.coarse_correction, output)?;

                level.graph().matvec_into(output, &mut local.residual)?;
                for (((value, inverse_diagonal), rhs_value), matrix_value) in output
                    .iter_mut()
                    .zip(level.inverse_diagonal())
                    .zip(rhs)
                    .zip(&local.residual)
                {
                    *value += *inverse_diagonal * (*rhs_value - *matrix_value);
                }
            }
            Ok(())
        })();
        workspace.put_level(level_index, local);
        result
    }
}

#[cfg(feature = "parallel")]
fn fill_planned(values: &mut [f64], value: f64, executor: &ParallelExecutor, parallel: bool) {
    if parallel {
        fill_parallel(values, value, executor);
    } else {
        values.fill(value);
    }
}

#[cfg(feature = "parallel")]
fn assign_scaled_planned(
    output: &mut [f64],
    inverse_diagonal: &[f64],
    rhs: &[f64],
    executor: &ParallelExecutor,
    parallel: bool,
) {
    if parallel {
        assign_scaled_parallel(output, inverse_diagonal, rhs, executor);
    } else {
        for ((value, &diagonal), &right) in output.iter_mut().zip(inverse_diagonal).zip(rhs) {
            *value = diagonal * right;
        }
    }
}

#[cfg(feature = "parallel")]
fn jacobi_add_planned(
    output: &mut [f64],
    inverse_diagonal: &[f64],
    rhs: &[f64],
    matvec: &[f64],
    executor: &ParallelExecutor,
    parallel: bool,
) {
    if parallel {
        jacobi_add_parallel(output, inverse_diagonal, rhs, matvec, executor);
    } else {
        for (((value, &diagonal), &right), &product) in
            output.iter_mut().zip(inverse_diagonal).zip(rhs).zip(matvec)
        {
            *value += diagonal * (right - product);
        }
    }
}

#[cfg(feature = "parallel")]
fn residual_from_matvec_planned(
    matvec: &mut [f64],
    rhs: &[f64],
    executor: &ParallelExecutor,
    parallel: bool,
) {
    if parallel {
        residual_from_matvec_parallel(matvec, rhs, executor);
    } else {
        for (value, &right) in matvec.iter_mut().zip(rhs) {
            *value = right - *value;
        }
    }
}

#[cfg(feature = "parallel")]
fn fill_parallel(values: &mut [f64], value: f64, executor: &ParallelExecutor) {
    if executor.should_parallel(values.len()) {
        executor.install(|| values.par_iter_mut().for_each(|item| *item = value));
    } else {
        values.fill(value);
    }
}

#[cfg(feature = "parallel")]
fn assign_scaled_parallel(
    output: &mut [f64],
    inverse_diagonal: &[f64],
    rhs: &[f64],
    executor: &ParallelExecutor,
) {
    if executor.should_parallel(output.len()) {
        executor.install(|| {
            output
                .par_iter_mut()
                .zip(inverse_diagonal.par_iter())
                .zip(rhs.par_iter())
                .for_each(|((value, inverse_diagonal), rhs_value)| {
                    *value = *inverse_diagonal * *rhs_value;
                });
        });
    } else {
        for ((value, inverse_diagonal), rhs_value) in
            output.iter_mut().zip(inverse_diagonal).zip(rhs)
        {
            *value = *inverse_diagonal * *rhs_value;
        }
    }
}

#[cfg(feature = "parallel")]
fn jacobi_add_parallel(
    output: &mut [f64],
    inverse_diagonal: &[f64],
    rhs: &[f64],
    matrix_value: &[f64],
    executor: &ParallelExecutor,
) {
    if executor.should_parallel(output.len()) {
        executor.install(|| {
            output
                .par_iter_mut()
                .zip(inverse_diagonal.par_iter())
                .zip(rhs.par_iter())
                .zip(matrix_value.par_iter())
                .for_each(|(((value, inverse_diagonal), rhs_value), matrix_value)| {
                    *value += *inverse_diagonal * (*rhs_value - *matrix_value);
                });
        });
    } else {
        for (((value, inverse_diagonal), rhs_value), matrix_value) in output
            .iter_mut()
            .zip(inverse_diagonal)
            .zip(rhs)
            .zip(matrix_value)
        {
            *value += *inverse_diagonal * (*rhs_value - *matrix_value);
        }
    }
}

#[cfg(feature = "parallel")]
fn residual_from_matvec_parallel(residual: &mut [f64], rhs: &[f64], executor: &ParallelExecutor) {
    if executor.should_parallel(residual.len()) {
        executor.install(|| {
            residual
                .par_iter_mut()
                .zip(rhs.par_iter())
                .for_each(|(value, rhs_value)| *value = *rhs_value - *value);
        });
    } else {
        for (value, rhs_value) in residual.iter_mut().zip(rhs) {
            *value = *rhs_value - *value;
        }
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
