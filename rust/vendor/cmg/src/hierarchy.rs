//! Construction and diagnostics for the stationary CMG hierarchy.

#[cfg(feature = "parallel")]
use crate::ParallelExecutor;
use crate::forest::build_forest_aggregation_labels;
#[cfg(feature = "parallel")]
use crate::forest::build_forest_aggregation_labels_with_executor;
use crate::{Aggregation, CmgError, CmgOptions, Laplacian};
use std::time::Instant;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HierarchyPhaseRecord {
    pub(crate) level: usize,
    pub(crate) phase: &'static str,
    pub(crate) nanoseconds: u128,
}

#[inline]
fn measure_hierarchy_phase<const PROFILE: bool, Output>(
    records: &mut Vec<HierarchyPhaseRecord>,
    level: usize,
    phase: &'static str,
    operation: impl FnOnce() -> Output,
) -> Output {
    if PROFILE {
        let start = Instant::now();
        let output = operation();
        records.push(HierarchyPhaseRecord {
            level,
            phase,
            nanoseconds: start.elapsed().as_nanos(),
        });
        output
    } else {
        operation()
    }
}

/// The reason hierarchy construction terminated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalReason {
    /// The graph was below the configured direct threshold.
    Direct,
    /// Forest grouping contracted the entire graph to one aggregate.
    FullContraction,
    /// Coarsening removed fewer than two vertices.
    StagnatedVertexReduction,
    /// Cumulative hierarchy nonzeros exceeded the configured fill guard.
    StagnatedFill,
    /// The configured hierarchy-level safety limit was reached.
    MaximumLevels,
}

impl TerminalReason {
    /// Return whether the terminal uses diagonal iteration instead of a direct
    /// factorization.
    #[must_use]
    pub const fn is_iterative(self) -> bool {
        !matches!(self, Self::Direct)
    }
}

/// One immutable hierarchy level.
#[derive(Debug, Clone, PartialEq)]
pub struct HierarchyLevel {
    graph: Laplacian,
    inverse_diagonal: Vec<f64>,
    aggregation: Option<Aggregation>,
    repeat: usize,
    terminal_reason: Option<TerminalReason>,
}

impl HierarchyLevel {
    /// Return the graph at this level.
    #[must_use]
    pub const fn graph(&self) -> &Laplacian {
        &self.graph
    }

    /// Return the upstream damped-Jacobi inverse diagonal `1 / (2 d_i)`.
    /// Isolated vertices receive zero.
    #[must_use]
    pub fn inverse_diagonal(&self) -> &[f64] {
        &self.inverse_diagonal
    }

    /// Return the fine-to-coarse aggregation for a non-direct level.
    #[must_use]
    pub const fn aggregation(&self) -> Option<&Aggregation> {
        self.aggregation.as_ref()
    }

    /// Return the recursive repeat count.
    ///
    /// A hierarchy built by [`CmgHierarchy::build`] initially carries the
    /// nonzero-ratio estimate. When the hierarchy is owned by a complete CMG
    /// preconditioner, the level preceding a direct terminal is recalibrated
    /// from the grounded LDL factor exactly as in upstream CMG.
    #[must_use]
    pub const fn repeat(&self) -> usize {
        self.repeat
    }

    /// Return the terminal reason when this is the last level.
    #[must_use]
    pub const fn terminal_reason(&self) -> Option<TerminalReason> {
        self.terminal_reason
    }

    /// Return whether this is the terminal level.
    #[must_use]
    pub const fn is_terminal(&self) -> bool {
        self.terminal_reason.is_some()
    }
}

/// Summary diagnostics from hierarchy construction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HierarchyBuildReport {
    terminal_reason: TerminalReason,
    vertex_counts: Vec<usize>,
    matrix_nonzeros: Vec<usize>,
    cumulative_coarsened_nonzeros: usize,
}

impl HierarchyBuildReport {
    /// Return the terminal reason.
    #[must_use]
    pub const fn terminal_reason(&self) -> TerminalReason {
        self.terminal_reason
    }

    /// Return the vertex count at each stored level.
    #[must_use]
    pub fn vertex_counts(&self) -> &[usize] {
        &self.vertex_counts
    }

    /// Return the symmetric matrix nonzero count at each stored level.
    #[must_use]
    pub fn matrix_nonzeros(&self) -> &[usize] {
        &self.matrix_nonzeros
    }

    /// Return the cumulative nonzeros counted by the upstream fill guard.
    #[must_use]
    pub const fn cumulative_coarsened_nonzeros(&self) -> usize {
        self.cumulative_coarsened_nonzeros
    }
}

/// A deterministic CMG hierarchy.
#[derive(Debug, Clone, PartialEq)]
pub struct CmgHierarchy {
    levels: Vec<HierarchyLevel>,
    report: HierarchyBuildReport,
}

impl CmgHierarchy {
    /// Build a hierarchy from a weighted graph Laplacian.
    pub fn build(graph: &Laplacian, options: CmgOptions) -> Result<Self, CmgError> {
        Self::build_with_kernels::<false, _, _>(
            graph,
            options,
            build_forest_aggregation_labels,
            |aggregation, current| aggregation.contract(current),
        )
    }

    /// Build a hierarchy with deterministic parallel coarse-graph contraction.
    ///
    /// Forest selection and splitting remain serial in this checkpoint. The
    /// supplied package-owned executor maps and sorts coarse edges while
    /// preserving the exact serial hierarchy.
    #[cfg(feature = "parallel")]
    pub fn build_with_executor(
        graph: &Laplacian,
        options: CmgOptions,
        executor: &ParallelExecutor,
    ) -> Result<Self, CmgError> {
        Self::build_with_kernels::<false, _, _>(
            graph,
            options,
            |current, threshold| {
                build_forest_aggregation_labels_with_executor(current, threshold, executor)
            },
            |aggregation, current| aggregation.contract_with_executor(current, executor),
        )
    }

    #[cfg(all(feature = "parallel", feature = "profiling"))]
    pub(crate) fn build_with_executor_profiled(
        graph: &Laplacian,
        options: CmgOptions,
        executor: &ParallelExecutor,
    ) -> Result<(Self, Vec<HierarchyPhaseRecord>), CmgError> {
        Self::build_with_kernels_profiled(
            graph,
            options,
            |current, threshold| {
                build_forest_aggregation_labels_with_executor(current, threshold, executor)
            },
            |aggregation, current| aggregation.contract_with_executor(current, executor),
        )
    }

    fn build_with_kernels<const PROFILE: bool, Group, Contract>(
        graph: &Laplacian,
        options: CmgOptions,
        group: Group,
        contract: Contract,
    ) -> Result<Self, CmgError>
    where
        Group: FnMut(&Laplacian, f64) -> Result<(Vec<usize>, usize), CmgError>,
        Contract: FnMut(&Aggregation, &Laplacian) -> Result<Laplacian, CmgError>,
    {
        Self::build_with_kernels_impl::<PROFILE, _, _>(graph, options, group, contract)
            .map(|(hierarchy, _)| hierarchy)
    }

    #[cfg(feature = "profiling")]
    fn build_with_kernels_profiled<Group, Contract>(
        graph: &Laplacian,
        options: CmgOptions,
        group: Group,
        contract: Contract,
    ) -> Result<(Self, Vec<HierarchyPhaseRecord>), CmgError>
    where
        Group: FnMut(&Laplacian, f64) -> Result<(Vec<usize>, usize), CmgError>,
        Contract: FnMut(&Aggregation, &Laplacian) -> Result<Laplacian, CmgError>,
    {
        Self::build_with_kernels_impl::<true, _, _>(graph, options, group, contract)
    }

    fn build_with_kernels_impl<const PROFILE: bool, Group, Contract>(
        graph: &Laplacian,
        options: CmgOptions,
        mut group: Group,
        mut contract: Contract,
    ) -> Result<(Self, Vec<HierarchyPhaseRecord>), CmgError>
    where
        Group: FnMut(&Laplacian, f64) -> Result<(Vec<usize>, usize), CmgError>,
        Contract: FnMut(&Aggregation, &Laplacian) -> Result<Laplacian, CmgError>,
    {
        let mut phase_records = Vec::new();
        let options = measure_hierarchy_phase::<PROFILE, _>(
            &mut phase_records,
            0,
            "option_validation",
            || options.validate(),
        )?;
        let initial_nonzeros = graph.matrix_nnz();
        let mut cumulative_nonzeros = 0_usize;
        let mut current = measure_hierarchy_phase::<PROFILE, _>(
            &mut phase_records,
            0,
            "graph_clone_reference_setup",
            || graph.clone(),
        );
        let mut levels = Vec::new();
        let terminal_reason;

        loop {
            let level_index = levels.len();
            let n = current.vertex_count();
            let direct = measure_hierarchy_phase::<PROFILE, _>(
                &mut phase_records,
                level_index,
                "direct_terminal_check",
                || n <= 1 || n < options.direct_threshold,
            );
            if direct {
                terminal_reason = TerminalReason::Direct;
                let level = measure_hierarchy_phase::<PROFILE, _>(
                    &mut phase_records,
                    level_index,
                    "inverse_diagonal_and_level_finalization",
                    || make_level(current, None, 0, Some(terminal_reason)),
                );
                levels.push(level);
                break;
            }

            let (labels, aggregate_count) = measure_hierarchy_phase::<PROFILE, _>(
                &mut phase_records,
                level_index,
                "forest_select_split_low_degree_and_label",
                || group(&current, options.low_effective_degree_threshold),
            )?;
            let aggregation = measure_hierarchy_phase::<PROFILE, _>(
                &mut phase_records,
                level_index,
                "aggregation_construction",
                || Aggregation::from_forest_labels(labels, aggregate_count),
            );
            let coarse_count = aggregation.coarse_dimension();
            let terminal = measure_hierarchy_phase::<PROFILE, _>(
                &mut phase_records,
                level_index,
                "hierarchy_bookkeeping_and_fill_checks",
                || {
                    if coarse_count == 1 {
                        return Some(TerminalReason::FullContraction);
                    }
                    cumulative_nonzeros = cumulative_nonzeros.saturating_add(current.matrix_nnz());
                    if coarse_count >= n.saturating_sub(1) {
                        return Some(TerminalReason::StagnatedVertexReduction);
                    }
                    let fill_limit = options.max_hierarchy_nnz_factor * initial_nonzeros as f64;
                    if cumulative_nonzeros as f64 > fill_limit {
                        return Some(TerminalReason::StagnatedFill);
                    }
                    if levels.len() + 1 >= options.max_levels {
                        return Some(TerminalReason::MaximumLevels);
                    }
                    None
                },
            );
            if let Some(reason) = terminal {
                terminal_reason = reason;
                let level = measure_hierarchy_phase::<PROFILE, _>(
                    &mut phase_records,
                    level_index,
                    "inverse_diagonal_and_level_finalization",
                    || make_level(current, Some(aggregation), 0, Some(terminal_reason)),
                );
                levels.push(level);
                break;
            }

            let coarse = measure_hierarchy_phase::<PROFILE, _>(
                &mut phase_records,
                level_index,
                "coarse_edge_map_sort_merge_and_graph_finalization",
                || contract(&aggregation, &current),
            )?;
            let repeat = measure_hierarchy_phase::<PROFILE, _>(
                &mut phase_records,
                level_index,
                "repeat_count_initialization",
                || repeat_from_nonzeros(current.matrix_nnz(), coarse.matrix_nnz()),
            );
            let level = measure_hierarchy_phase::<PROFILE, _>(
                &mut phase_records,
                level_index,
                "inverse_diagonal_and_level_finalization",
                || make_level(current, Some(aggregation), repeat, None),
            );
            levels.push(level);
            current = coarse;
        }

        let report = measure_hierarchy_phase::<PROFILE, _>(
            &mut phase_records,
            levels.len().saturating_sub(1),
            "hierarchy_report_bookkeeping",
            || HierarchyBuildReport {
                terminal_reason,
                vertex_counts: levels
                    .iter()
                    .map(|level| level.graph.vertex_count())
                    .collect(),
                matrix_nonzeros: levels
                    .iter()
                    .map(|level| level.graph.matrix_nnz())
                    .collect(),
                cumulative_coarsened_nonzeros: cumulative_nonzeros,
            },
        );
        Ok((Self { levels, report }, phase_records))
    }

    /// Return all levels from fine to coarse.
    #[must_use]
    pub fn levels(&self) -> &[HierarchyLevel] {
        &self.levels
    }

    /// Return the build report.
    #[must_use]
    pub const fn report(&self) -> &HierarchyBuildReport {
        &self.report
    }

    pub(crate) fn set_repeat(&mut self, level_index: usize, repeat: usize) -> Result<(), CmgError> {
        let level = self
            .levels
            .get_mut(level_index)
            .ok_or(CmgError::InvalidHierarchy {
                context: "repeat update references a missing hierarchy level",
            })?;
        if level.is_terminal() || repeat == 0 {
            return Err(CmgError::InvalidHierarchy {
                context: "repeat update must target a nonterminal level with a positive count",
            });
        }
        level.repeat = repeat;
        Ok(())
    }
}

fn make_level(
    graph: Laplacian,
    aggregation: Option<Aggregation>,
    repeat: usize,
    terminal_reason: Option<TerminalReason>,
) -> HierarchyLevel {
    let inverse_diagonal = graph
        .diagonal()
        .iter()
        .map(|degree| if *degree > 0.0 { 0.5 / *degree } else { 0.0 })
        .collect();
    HierarchyLevel {
        graph,
        inverse_diagonal,
        aggregation,
        repeat,
        terminal_reason,
    }
}

fn repeat_from_nonzeros(fine_nonzeros: usize, coarse_nonzeros: usize) -> usize {
    if coarse_nonzeros == 0 {
        return 1;
    }
    (fine_nonzeros / coarse_nonzeros).saturating_sub(1).max(1)
}
