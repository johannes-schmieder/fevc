//! Construction and diagnostics for the stationary CMG hierarchy.

use crate::cancel::{Meter, Unmetered};
use crate::execution::Team;
use crate::forest::build_forest_aggregation_labels;
use crate::preconditioner::Width;
use crate::{Aggregation, CmgError, CmgOptions, Laplacian};

/// Level-loop choices that do not change arithmetic beyond the declared
/// component options.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct LevelPolicy {
    /// Retire isolated coarse vertices after contraction.
    #[cfg(feature = "experimental-components")]
    pub(crate) prune_coarse_isolates: bool,
    /// Count retired isolates in vertex-based stopping decisions.
    #[cfg(feature = "experimental-components")]
    pub(crate) preserve_unpruned_stopping: bool,
    /// Reserve `max_levels` level descriptors up front (admitted builds).
    pub(crate) reserve_levels: bool,
}
use std::time::Instant;

crate::storage::storage_fields!(CmgHierarchy, levels, report);
crate::storage::storage_fields!(HierarchyBuildReport, vertex_counts, matrix_nonzeros);
crate::storage::storage_fields!(HierarchyLevel, graph, inverse_diagonal, transfer);
impl crate::storage::Storage for LevelTransfer {
    fn visit(
        &self,
        visitor: &mut dyn FnMut(crate::storage::Allocation) -> Result<(), CmgError>,
    ) -> Result<(), CmgError> {
        match self {
            Self::Full(v) => crate::storage::Storage::visit(v, visitor),
            #[cfg(feature = "experimental-components")]
            Self::Pruned(v) => crate::storage::Storage::visit(v, visitor),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HierarchyPhaseRecord {
    pub(crate) level: usize,
    pub(crate) phase: &'static str,
    pub(crate) nanoseconds: u128,
}

/// Phase records a profiled hierarchy build can emit: option validation and
/// graph setup, at most seven per level and the final report.
pub(crate) fn phase_record_bound(max_levels: usize) -> usize {
    max_levels.saturating_mul(7).saturating_add(3)
}

/// Time one phase into `records`, whose capacity was reserved fallibly for
/// [`phase_record_bound`] records before the build; a push never grows it.
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
        debug_assert!(records.len() < records.capacity());
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

// A level owns exactly one transfer representation. Keeping mutually exclusive
// formats in one enum avoids inflating every connected level with unused state.
#[derive(Debug, Clone, PartialEq, Eq)]
enum LevelTransfer {
    Full(Aggregation),
    #[cfg(feature = "experimental-components")]
    Pruned(crate::PrunedTransfer),
}

/// One immutable hierarchy level.
#[derive(Debug, Clone, PartialEq)]
pub struct HierarchyLevel {
    graph: Laplacian,
    inverse_diagonal: Vec<f64>,
    transfer: Option<LevelTransfer>,
    repeat: usize,
    terminal_reason: Option<TerminalReason>,
}

impl HierarchyLevel {
    /// Restrict `rhs - product` lanes through this level's transfer.
    pub(crate) fn restrict_residual_lanes<W: Width, M: Meter>(
        &self,
        rhs: &[f64],
        product: &[f64],
        coarse: &mut [f64],
        width: W,
        meter: &mut M,
    ) -> Result<(), CmgError> {
        match self.transfer.as_ref().ok_or(CmgError::InvalidHierarchy {
            context: "nonterminal level has no transfer",
        })? {
            LevelTransfer::Full(aggregation) => {
                aggregation.restrict_residual_lanes(rhs, product, coarse, width, meter)
            }
            #[cfg(feature = "experimental-components")]
            LevelTransfer::Pruned(transfer) => {
                transfer.restrict_residual_lanes(rhs, product, coarse, width, meter)
            }
        }
    }

    /// Add prolonged coarse lanes through this level's transfer; a full
    /// aggregation uses `team` above its parallel threshold.
    pub(crate) fn prolong_add_lanes<W: Width, M: Meter>(
        &self,
        coarse: &[f64],
        fine: &mut [f64],
        width: W,
        team: Option<Team<'_>>,
        meter: &mut M,
    ) -> Result<(), CmgError> {
        match self.transfer.as_ref().ok_or(CmgError::InvalidHierarchy {
            context: "nonterminal level has no transfer",
        })? {
            LevelTransfer::Full(aggregation) => {
                aggregation.prolong_add_lanes(coarse, fine, width, team, meter)
            }
            #[cfg(feature = "experimental-components")]
            LevelTransfer::Pruned(transfer) => {
                transfer.prolong_add_lanes(coarse, fine, width, meter)
            }
        }
    }

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

    /// Return the full fine-to-coarse aggregation, when present.
    ///
    /// Experimental pruned levels instead expose `pruned_transfer`; they do
    /// not pretend that every fine row has a surviving coarse degree of freedom.
    #[must_use]
    pub const fn aggregation(&self) -> Option<&Aggregation> {
        match &self.transfer {
            Some(LevelTransfer::Full(aggregation)) => Some(aggregation),
            _ => None,
        }
    }

    /// Return the experimental partial transfer, if coarse isolates were retired.
    #[cfg(feature = "experimental-components")]
    #[must_use]
    pub const fn pruned_transfer(&self) -> Option<&crate::PrunedTransfer> {
        match &self.transfer {
            Some(LevelTransfer::Pruned(transfer)) => Some(transfer),
            _ => None,
        }
    }

    fn transfer_retained_bytes(&self) -> usize {
        match &self.transfer {
            Some(LevelTransfer::Full(aggregation)) => aggregation.retained_bytes(),
            #[cfg(feature = "experimental-components")]
            Some(LevelTransfer::Pruned(transfer)) => transfer.retained_bytes(),
            None => 0,
        }
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
    fn retained_bytes(&self) -> usize {
        self.vertex_counts
            .capacity()
            .saturating_mul(core::mem::size_of::<usize>())
            .saturating_add(
                self.matrix_nonzeros
                    .capacity()
                    .saturating_mul(core::mem::size_of::<usize>()),
            )
    }

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
    /// Conservative new requested layouts for hierarchy construction,
    /// including all retained levels and the largest temporary stage, for
    /// ordinary and pruned component policies. The parallel contraction kernel
    /// adds one fixed slot bank. Borrowed graph, allocator overhead, runtime,
    /// factors and later lazy accessors are excluded. Arc control blocks follow
    /// the reviewed layout model, not a stable ABI.
    pub fn construction_allocation_bound(
        vertices: usize,
        edges: usize,
        options: CmgOptions,
        parallel: bool,
    ) -> Result<usize, CmgError> {
        let (retained, scratch) = Self::serial_allocation_parts(vertices, edges, options)?;
        let bound = crate::storage::add(retained, scratch)?;
        if parallel {
            crate::storage::add(bound, crate::storage::bytes::<Option<crate::Edge>>(edges)?)
        } else {
            Ok(bound)
        }
    }

    pub(crate) fn serial_allocation_parts(
        n: usize,
        m: usize,
        options: CmgOptions,
    ) -> Result<(usize, usize), CmgError> {
        use crate::storage::{add, arc_bytes, bytes};
        let levels = options.validate()?.max_levels;
        let count = n.checked_mul(levels).ok_or(CmgError::AllocationFailed {
            context: "hierarchy vertex budget overflows",
        })?;
        let arcs = add(
            add(arc_bytes::<Vec<crate::Edge>>()?, arc_bytes::<Vec<f64>>()?)?,
            arc_bytes::<()>()?,
        )?;
        let graph = add(add(bytes::<crate::Edge>(m)?, bytes::<f64>(n)?)?, arcs)?;
        let graphs = graph
            .checked_mul(levels - 1)
            .ok_or(CmgError::AllocationFailed {
                context: "hierarchy graph budget overflows",
            })?;
        let transfers = bytes::<usize>(count)?.max(bytes::<(u32, u32)>(count)?);
        let retained = add(
            add(
                add(
                    bytes::<HierarchyLevel>(levels)?,
                    bytes::<[usize; 2]>(levels)?,
                )?,
                graphs,
            )?,
            add(bytes::<f64>(count)?, transfers)?,
        )?;
        // Heavy parent + split parent + native indegree + walk; selected
        // weights + two i64 ancestor banks; visited flags. Labeling is smaller.
        let forest = add(
            add(bytes::<[usize; 4]>(n)?, bytes::<[i64; 3]>(n)?)?,
            bytes::<bool>(n)?,
        )?;
        let aggregation = add(bytes::<usize>(n)?, bytes::<u32>(n)?)?;
        let contraction = add(graph, bytes::<crate::Edge>(m)?)?;
        // Unpruned and compact children, isolate map and old/new transfer
        // entries coexist. Include the still-live full aggregation as well.
        let pruning = add(
            add(add(graph, graph)?, bytes::<[usize; 2]>(n)?)?,
            bytes::<[(u32, u32); 2]>(n)?,
        )?;
        Ok((
            retained,
            forest.max(aggregation).max(contraction).max(pruning),
        ))
    }

    /// Return principal retained heap bytes for every graph, diagonal smoother,
    /// aggregation, and hierarchy report. Allocator bookkeeping is excluded.
    #[must_use]
    pub fn retained_bytes(&self) -> usize {
        self.levels
            .iter()
            .fold(self.report.retained_bytes(), |bytes, level| {
                bytes
                    .saturating_add(level.graph.retained_bytes())
                    .saturating_add(
                        level
                            .inverse_diagonal
                            .capacity()
                            .saturating_mul(core::mem::size_of::<f64>()),
                    )
                    .saturating_add(level.transfer_retained_bytes())
            })
    }

    /// Build a hierarchy from a weighted graph Laplacian.
    pub fn build(graph: &Laplacian, options: CmgOptions) -> Result<Self, CmgError> {
        Self::build_metered::<_, false>(
            graph,
            options,
            LevelPolicy::default(),
            None,
            &mut Unmetered,
        )
        .map(|(hierarchy, _)| hierarchy)
    }

    /// Build a hierarchy under a construction context with the component
    /// level policy of `options`. With a budget, [`Self::construction_allocation_bound`]
    /// (with `parallel` set when a team is supplied) is admitted before the
    /// first allocation; cancellation and team follow
    /// [`crate::CmgPreconditioner::build_with`]. Bits never depend on the context.
    pub fn build_with(
        graph: &Laplacian,
        options: crate::BuildOptions,
        context: crate::BuildContext<'_>,
    ) -> Result<Self, CmgError> {
        if let Some(budget) = context.budget_bytes {
            crate::storage::admit(
                Self::construction_allocation_bound(
                    graph.vertex_count(),
                    graph.edge_count(),
                    options.hierarchy,
                    context.team().is_some(),
                )?,
                budget,
            )?;
        }
        let policy = options.level_policy(context.budget_bytes.is_some());
        match context.cancel {
            Some(cancel) => Self::build_metered::<_, false>(
                graph,
                options.hierarchy,
                policy,
                context.team(),
                &mut crate::cancel::CancelMeter::new(cancel),
            ),
            None => Self::build_metered::<_, false>(
                graph,
                options.hierarchy,
                policy,
                context.team(),
                &mut Unmetered,
            ),
        }
        .map(|(hierarchy, _)| hierarchy)
    }

    /// The single hierarchy loop: forest grouping, aggregation, stopping
    /// rules, Galerkin contraction, optional isolate pruning and smoother
    /// finalization, level by level. A team only selects the deterministic
    /// parallel contraction kernel; a polling meter only adds bounded polls;
    /// `PROFILE` only records phase timings. None changes a single bit.
    pub(crate) fn build_metered<M: Meter, const PROFILE: bool>(
        graph: &Laplacian,
        options: CmgOptions,
        policy: LevelPolicy,
        team: Option<Team<'_>>,
        meter: &mut M,
    ) -> Result<(Self, Vec<HierarchyPhaseRecord>), CmgError> {
        meter.work("hierarchy initialization", 128)?;
        let mut phase_records = if PROFILE {
            // Validation is repeated below as a timed phase; an invalid
            // option reports the same error after this one-record reserve.
            let records = options
                .validate()
                .map_or(1, |options| phase_record_bound(options.max_levels));
            meter.try_capacity(records, "hierarchy profile records")?
        } else {
            Vec::new()
        };
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
        let mut levels = if policy.reserve_levels || M::POLLS {
            meter.try_capacity(options.max_levels, "hierarchy levels")?
        } else {
            Vec::new()
        };
        // Pruned isolates still count in vertex stopping decisions when
        // preserving unpruned stopping; otherwise this stays zero.
        #[cfg_attr(not(feature = "experimental-components"), allow(unused_mut))]
        let mut retired_vertices = 0usize;
        let terminal_reason;

        loop {
            meter.work("hierarchy terminal decision", 128)?;
            let level_index = levels.len();
            let n = current.vertex_count();
            let decision_n = n + retired_vertices;
            let direct = measure_hierarchy_phase::<PROFILE, _>(
                &mut phase_records,
                level_index,
                "direct_terminal_check",
                || n == 0 || decision_n <= 1 || decision_n < options.direct_threshold,
            );
            if direct {
                terminal_reason = TerminalReason::Direct;
                let level = measure_hierarchy_phase::<PROFILE, _>(
                    &mut phase_records,
                    level_index,
                    "inverse_diagonal_and_level_finalization",
                    || make_level(current, None, 0, Some(terminal_reason), meter),
                )?;
                meter.work("hierarchy level publication", 32)?;
                levels.push(level);
                break;
            }

            let (labels, aggregate_count) = measure_hierarchy_phase::<PROFILE, _>(
                &mut phase_records,
                level_index,
                "forest_select_split_low_degree_and_label",
                || {
                    build_forest_aggregation_labels(
                        &current,
                        options.low_effective_degree_threshold,
                        team,
                        meter,
                    )
                },
            )?;
            let aggregation = measure_hierarchy_phase::<PROFILE, _>(
                &mut phase_records,
                level_index,
                "aggregation_construction",
                || Aggregation::from_forest_labels(labels, aggregate_count, meter),
            )?;
            meter.work("hierarchy stopping rules", 128)?;
            let decision_coarse_count = aggregation.coarse_dimension() + retired_vertices;
            let terminal = measure_hierarchy_phase::<PROFILE, _>(
                &mut phase_records,
                level_index,
                "hierarchy_bookkeeping_and_fill_checks",
                || {
                    if decision_coarse_count == 1 {
                        return Some(TerminalReason::FullContraction);
                    }
                    cumulative_nonzeros = cumulative_nonzeros.saturating_add(current.matrix_nnz());
                    if decision_coarse_count >= decision_n.saturating_sub(1) {
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
                    || make_level(current, Some(aggregation), 0, Some(reason), meter),
                )?;
                meter.work("hierarchy level publication", 32)?;
                levels.push(level);
                break;
            }

            let coarse = measure_hierarchy_phase::<PROFILE, _>(
                &mut phase_records,
                level_index,
                "coarse_edge_map_sort_merge_and_graph_finalization",
                || aggregation.contract_with(&current, team, meter),
            )?;
            #[cfg(feature = "experimental-components")]
            let (coarse, pruned_transfer) = if policy.prune_coarse_isolates {
                match crate::component_experiment::prune(&aggregation, &coarse, meter)? {
                    Some((transfer, compact)) => {
                        // Isolates contribute zero matrix nonzeros. Keep just
                        // their count for the ordinary vertex stopping checks;
                        // no retired row survives in graph or cycle storage.
                        meter.work("hierarchy retired vertex count", 32)?;
                        if policy.preserve_unpruned_stopping {
                            retired_vertices += coarse.vertex_count() - compact.vertex_count();
                        }
                        (compact, Some(transfer))
                    }
                    None => (coarse, None),
                }
            } else {
                (coarse, None)
            };
            meter.work("hierarchy repeat count", 32)?;
            let repeat = measure_hierarchy_phase::<PROFILE, _>(
                &mut phase_records,
                level_index,
                "repeat_count_initialization",
                || repeat_from_nonzeros(current.matrix_nnz(), coarse.matrix_nnz()),
            );
            #[allow(unused_mut)]
            let mut level = measure_hierarchy_phase::<PROFILE, _>(
                &mut phase_records,
                level_index,
                "inverse_diagonal_and_level_finalization",
                || make_level(current, Some(aggregation), repeat, None, meter),
            )?;
            meter.work("hierarchy level publication", 64)?;
            #[cfg(feature = "experimental-components")]
            if let Some(transfer) = pruned_transfer {
                level.transfer = Some(LevelTransfer::Pruned(transfer));
            }
            levels.push(level);
            current = coarse;
        }

        let report = measure_hierarchy_phase::<PROFILE, _>(
            &mut phase_records,
            levels.len().saturating_sub(1),
            "hierarchy_report_bookkeeping",
            || -> Result<HierarchyBuildReport, CmgError> {
                let mut vertex_counts =
                    meter.try_capacity(levels.len(), "hierarchy vertex report")?;
                let mut matrix_nonzeros =
                    meter.try_capacity(levels.len(), "hierarchy nonzero report")?;
                for level in &levels {
                    meter.work("hierarchy report", 32)?;
                    vertex_counts.push(level.graph.vertex_count());
                    matrix_nonzeros.push(level.graph.matrix_nnz());
                }
                Ok(HierarchyBuildReport {
                    terminal_reason,
                    vertex_counts,
                    matrix_nonzeros,
                    cumulative_coarsened_nonzeros: cumulative_nonzeros,
                })
            },
        )?;
        meter.work("hierarchy publication", 32)?;
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

fn make_level<M: Meter>(
    graph: Laplacian,
    aggregation: Option<Aggregation>,
    repeat: usize,
    terminal_reason: Option<TerminalReason>,
    meter: &mut M,
) -> Result<HierarchyLevel, CmgError> {
    let mut inverse_diagonal =
        meter.try_capacity(graph.vertex_count(), "hierarchy inverse diagonal")?;
    for &degree in graph.diagonal() {
        meter.work("hierarchy inverse diagonal", 16)?;
        inverse_diagonal.push(if degree > 0.0 { 0.5 / degree } else { 0.0 });
    }
    meter.work("hierarchy level finalization", 32)?;
    Ok(HierarchyLevel {
        graph,
        inverse_diagonal,
        transfer: aggregation.map(LevelTransfer::Full),
        repeat,
        terminal_reason,
    })
}

fn repeat_from_nonzeros(fine_nonzeros: usize, coarse_nonzeros: usize) -> usize {
    if coarse_nonzeros == 0 {
        return 1;
    }
    (fine_nonzeros / coarse_nonzeros).saturating_sub(1).max(1)
}
