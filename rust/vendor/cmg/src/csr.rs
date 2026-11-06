//! Deterministic row-oriented Laplacian storage for solve kernels.

use crate::cancel::{Meter, chunked};
#[cfg(feature = "parallel")]
use crate::execution::Team;
use crate::preconditioner::Width;
use crate::storage::{add, bytes};
use crate::{CmgError, Laplacian};
#[cfg(feature = "parallel")]
use std::sync::Arc;
use std::time::Instant;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct CsrBuildProfile {
    pub(crate) row_counts_nanoseconds: u128,
    pub(crate) row_offsets_nanoseconds: u128,
    pub(crate) allocation_nanoseconds: u128,
    pub(crate) scatter_nanoseconds: u128,
    pub(crate) validation_nanoseconds: u128,
}

#[inline]
fn measure_build_phase<const PROFILE: bool, Output>(
    nanoseconds: &mut u128,
    operation: impl FnOnce() -> Output,
) -> Output {
    if PROFILE {
        let start = Instant::now();
        let output = operation();
        *nanoseconds = nanoseconds.saturating_add(start.elapsed().as_nanos());
        output
    } else {
        operation()
    }
}

#[derive(Debug, Clone, PartialEq)]
enum RowOffsets {
    Compact(Vec<u32>),
    Native(Vec<usize>),
}

impl RowOffsets {
    fn byte_len(&self) -> usize {
        match self {
            Self::Compact(values) => values.len().saturating_mul(core::mem::size_of::<u32>()),
            Self::Native(values) => values.len().saturating_mul(core::mem::size_of::<usize>()),
        }
    }

    const fn is_compact(&self) -> bool {
        matches!(self, Self::Compact(_))
    }

    fn row_count(&self) -> usize {
        match self {
            Self::Compact(values) => values.len().saturating_sub(1),
            Self::Native(values) => values.len().saturating_sub(1),
        }
    }

    fn last(&self) -> usize {
        match self {
            Self::Compact(values) => values.last().copied().unwrap_or(0) as usize,
            Self::Native(values) => values.last().copied().unwrap_or(0),
        }
    }

    #[inline]
    fn bounds(&self, row: usize) -> (usize, usize) {
        debug_assert!(row < self.row_count());
        match self {
            Self::Compact(values) => (values[row] as usize, values[row + 1] as usize),
            Self::Native(values) => (values[row], values[row + 1]),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum ColumnIndices {
    Compact(Vec<u32>),
    Native(Vec<usize>),
}

impl ColumnIndices {
    fn byte_len(&self) -> usize {
        match self {
            Self::Compact(values) => values.len().saturating_mul(core::mem::size_of::<u32>()),
            Self::Native(values) => values.len().saturating_mul(core::mem::size_of::<usize>()),
        }
    }

    const fn is_compact(&self) -> bool {
        matches!(self, Self::Compact(_))
    }
}

/// A deterministic row-oriented representation of a weighted graph Laplacian.
///
/// Each undirected graph edge is stored twice, once in each endpoint row. Rows
/// own their output entries, making this representation suitable for
/// deterministic parallel matrix-vector products without atomics. Canonical
/// graph ordering guarantees ascending neighbor indices within every row.
#[derive(Debug, Clone)]
pub struct CsrLaplacian {
    vertex_count: usize,
    row_offsets: RowOffsets,
    columns: ColumnIndices,
    weights: Vec<f64>,
    #[cfg(feature = "parallel")]
    source_lineage: Arc<()>,
}

#[cfg(feature = "parallel")]
crate::storage::storage_fields!(CsrLaplacian, row_offsets, columns, weights, source_lineage);
#[cfg(not(feature = "parallel"))]
crate::storage::storage_fields!(CsrLaplacian, row_offsets, columns, weights);

impl crate::storage::Storage for RowOffsets {
    fn visit(
        &self,
        visitor: &mut dyn FnMut(crate::storage::Allocation) -> Result<(), CmgError>,
    ) -> Result<(), CmgError> {
        match self {
            Self::Compact(v) => crate::storage::Storage::visit(v, visitor),
            Self::Native(v) => crate::storage::Storage::visit(v, visitor),
        }
    }
}
impl crate::storage::Storage for ColumnIndices {
    fn visit(
        &self,
        visitor: &mut dyn FnMut(crate::storage::Allocation) -> Result<(), CmgError>,
    ) -> Result<(), CmgError> {
        match self {
            Self::Compact(v) => crate::storage::Storage::visit(v, visitor),
            Self::Native(v) => crate::storage::Storage::visit(v, visitor),
        }
    }
}

impl PartialEq for CsrLaplacian {
    fn eq(&self, other: &Self) -> bool {
        self.vertex_count == other.vertex_count
            && self.row_offsets == other.row_offsets
            && self.columns == other.columns
            && self.weights == other.weights
    }
}

impl CsrLaplacian {
    /// Upper bound on new CMG-owned requested layouts during serial CSR
    /// construction. Includes retained arrays and overlapping cursor scratch;
    /// excludes borrowed graph/lineage, allocator overhead and inline roots.
    pub fn construction_allocation_bound(vertices: usize, edges: usize) -> Result<usize, CmgError> {
        add(
            Self::retained_allocation_bytes(vertices, edges)?,
            bytes::<usize>(vertices)?,
        )
    }

    pub(crate) fn retained_allocation_bytes(
        vertices: usize,
        edges: usize,
    ) -> Result<usize, CmgError> {
        let directed = edges.checked_mul(2).ok_or(CmgError::AllocationFailed {
            context: "CSR directed-entry count overflows",
        })?;
        let offsets = add(vertices, 1)?;
        let rows = if directed <= u32::MAX as usize {
            bytes::<u32>(offsets)?
        } else {
            bytes::<usize>(offsets)?
        };
        let columns = if vertices <= u32::MAX as usize {
            bytes::<u32>(directed)?
        } else {
            bytes::<usize>(directed)?
        };
        add(add(rows, columns)?, bytes::<f64>(directed)?)
    }

    /// Admit all new serial CSR layouts before allocating. Shared graph lineage
    /// is borrowed, not newly allocated. Existing graphs/operators remain the
    /// caller's separately charged parents.
    pub fn from_laplacian_with_budget(
        graph: &Laplacian,
        budget_bytes: usize,
    ) -> Result<Self, CmgError> {
        crate::storage::admit(
            Self::construction_allocation_bound(graph.vertex_count(), graph.edge_count())?,
            budget_bytes,
        )?;
        Self::from_laplacian(graph)
    }

    /// CMG-owned construction layouts for the unchanged team routing.
    /// Rayon runtime allocations, allocator overhead and existing parents are
    /// explicitly excluded; this is not whole-action/runtime admission.
    #[cfg(feature = "parallel")]
    pub fn construction_allocation_bound_with_team(
        graph: &Laplacian,
        team: Team<'_>,
    ) -> Result<usize, CmgError> {
        Self::construction_allocation_bound_for_shape(graph, team.options())
    }

    #[cfg(feature = "parallel")]
    pub(crate) fn construction_allocation_bound_for_shape(
        graph: &Laplacian,
        shape: crate::ParallelOptions,
    ) -> Result<usize, CmgError> {
        let n = graph.vertex_count();
        let m = graph.edge_count();
        if !Self::uses_parallel_construction(graph, shape) {
            return Self::construction_allocation_bound(n, m);
        }
        let directed = m.checked_mul(2).ok_or(CmgError::AllocationFailed {
            context: "CSR directed-entry count overflows",
        })?;
        let rows = if directed <= u32::MAX as usize {
            bytes::<u32>(add(n, 1)?)?
        } else {
            bytes::<usize>(add(n, 1)?)?
        };
        let native_offsets = bytes::<usize>(add(n, 1)?)?;
        let two_offsets = add(native_offsets, native_offsets)?;
        // Counts convert in-place to cursors. Account for the outgoing count
        // bank until its prefix sum returns, then for incoming edge scatter.
        let counting = add(rows, bytes::<[usize; 3]>(n)?)?;
        let offsets = add(add(rows, two_offsets)?, bytes::<[usize; 2]>(n)?)?;
        let incoming = add(
            add(add(rows, two_offsets)?, bytes::<usize>(n)?)?,
            bytes::<usize>(m)?,
        )?;
        let blocks = n.div_ceil(shape.work_chunk_len(n));
        let descriptors = if n <= u32::MAX as usize {
            bytes::<RowFillBlock<'_, u32>>(blocks)?
        } else {
            bytes::<RowFillBlock<'_, usize>>(blocks)?
        };
        let filling = add(
            add(
                add(Self::retained_allocation_bytes(n, m)?, two_offsets)?,
                bytes::<usize>(m)?,
            )?,
            descriptors,
        )?;
        Ok(counting.max(offsets).max(incoming).max(filling))
    }

    #[cfg(feature = "parallel")]
    fn uses_parallel_construction(graph: &Laplacian, shape: crate::ParallelOptions) -> bool {
        shape.threads > 1
            && shape.should_parallel(graph.edge_count())
            && graph.edge_count() >= graph.vertex_count().saturating_mul(4)
    }

    /// Freeze a canonical edge-list Laplacian into deterministic row storage.
    pub fn from_laplacian(graph: &Laplacian) -> Result<Self, CmgError> {
        Self::from_laplacian_impl::<false, _>(graph, &mut crate::cancel::Unmetered)
            .map(|(operator, _)| operator)
    }

    /// Row operator for one level on `team`; `PROFILE` only times phases and a
    /// polling meter only adds bounded polls (lanes poll inside their rows).
    #[cfg(feature = "parallel")]
    pub(crate) fn from_laplacian_with_team_impl<const PROFILE: bool, M: Meter>(
        graph: &Laplacian,
        team: Team<'_>,
        meter: &mut M,
    ) -> Result<(Self, CsrBuildProfile), CmgError> {
        if !Self::uses_parallel_construction(graph, team.options()) {
            return Self::from_laplacian_impl::<PROFILE, M>(graph, meter);
        }
        Self::from_laplacian_parallel_impl::<PROFILE, M>(graph, team, meter)
    }

    #[cfg(feature = "parallel")]
    fn from_laplacian_parallel_impl<const PROFILE: bool, M: Meter>(
        graph: &Laplacian,
        team: Team<'_>,
        meter: &mut M,
    ) -> Result<(Self, CsrBuildProfile), CmgError> {
        let mut profile = CsrBuildProfile::default();
        let vertex_count = graph.vertex_count();
        let edge_count = graph.edge_count();
        let directed_entries = edge_count
            .checked_mul(2)
            .ok_or(CmgError::InvalidHierarchy {
                context: "CSR directed-entry count overflowed usize",
            })?;

        let (row_counts, incoming_counts, outgoing_counts) =
            measure_build_phase::<PROFILE, _>(&mut profile.row_counts_nanoseconds, || {
                let mut row_counts = meter.try_filled(vertex_count, 0_usize, "CSR row counts")?;
                let mut incoming_counts =
                    meter.try_filled(vertex_count, 0_usize, "CSR incoming counts")?;
                let mut outgoing_counts =
                    meter.try_filled(vertex_count, 0_usize, "CSR outgoing counts")?;
                chunked(meter, "CSR row counts", edge_count, 16, |range| {
                    for edge in &graph.edges()[range] {
                        row_counts[edge.u()] += 1;
                        row_counts[edge.v()] += 1;
                        outgoing_counts[edge.u()] += 1;
                        incoming_counts[edge.v()] += 1;
                    }
                    Ok(())
                })?;
                Ok::<_, CmgError>((row_counts, incoming_counts, outgoing_counts))
            })?;
        let (row_offsets, incoming_offsets, outgoing_offsets, mut incoming_next) =
            measure_build_phase::<PROFILE, _>(&mut profile.row_offsets_nanoseconds, || {
                let (row_offsets, _) =
                    row_offsets_and_cursors(row_counts, directed_entries, meter)?;
                let (incoming_offsets, incoming_next) =
                    native_offsets_and_cursors(incoming_counts, edge_count, meter)?;
                let (outgoing_offsets, _) =
                    native_offsets_and_cursors(outgoing_counts, edge_count, meter)?;
                Ok::<_, CmgError>((
                    row_offsets,
                    incoming_offsets,
                    outgoing_offsets,
                    incoming_next,
                ))
            })?;
        measure_build_phase::<PROFILE, _>(&mut profile.validation_nanoseconds, || {
            if row_offsets.last() != directed_entries
                || incoming_offsets.last().copied().unwrap_or(0) != edge_count
                || outgoing_offsets.last().copied().unwrap_or(0) != edge_count
            {
                Err(CmgError::InvalidHierarchy {
                    context: "parallel CSR row counts do not match edge counts",
                })
            } else {
                Ok(())
            }
        })?;

        let mut incoming_edges =
            measure_build_phase::<PROFILE, _>(&mut profile.allocation_nanoseconds, || {
                meter.try_filled(edge_count, 0_usize, "CSR incoming edges")
            })?;
        measure_build_phase::<PROFILE, _>(&mut profile.scatter_nanoseconds, || {
            chunked(meter, "CSR incoming scatter", edge_count, 8, |range| {
                for edge_index in range {
                    let edge = graph.edges()[edge_index];
                    let position = incoming_next[edge.v()];
                    incoming_edges[position] = edge_index;
                    incoming_next[edge.v()] += 1;
                }
                Ok(())
            })
        })?;
        drop(incoming_next);

        let mut weights =
            measure_build_phase::<PROFILE, _>(&mut profile.allocation_nanoseconds, || {
                meter.try_filled(directed_entries, 0.0, "CSR weights")
            })?;
        let fill = RowFill {
            graph,
            row_offsets: &row_offsets,
            incoming_offsets: &incoming_offsets,
            outgoing_offsets: &outgoing_offsets,
            incoming_edges: &incoming_edges,
            rows_per_block: team.work_chunk_len(vertex_count),
        };
        let columns = if vertex_count <= u32::MAX as usize {
            let mut columns =
                measure_build_phase::<PROFILE, _>(&mut profile.allocation_nanoseconds, || {
                    meter.try_filled(directed_entries, 0_u32, "CSR compact columns")
                })?;
            measure_build_phase::<PROFILE, _>(&mut profile.scatter_nanoseconds, || {
                fill.rows_parallel(
                    &mut columns,
                    &mut weights,
                    |vertex| vertex as u32,
                    team,
                    meter,
                )
            })?;
            ColumnIndices::Compact(columns)
        } else {
            let mut columns =
                measure_build_phase::<PROFILE, _>(&mut profile.allocation_nanoseconds, || {
                    meter.try_filled(directed_entries, 0_usize, "CSR native columns")
                })?;
            measure_build_phase::<PROFILE, _>(&mut profile.scatter_nanoseconds, || {
                fill.rows_parallel(&mut columns, &mut weights, |vertex| vertex, team, meter)
            })?;
            ColumnIndices::Native(columns)
        };

        measure_build_phase::<PROFILE, _>(&mut profile.validation_nanoseconds, || {
            if PROFILE && !rows_are_sorted(&row_offsets, &columns) {
                return Err(CmgError::InvalidHierarchy {
                    context: "profiled parallel CSR rows are not sorted",
                });
            }
            debug_assert!(rows_are_sorted(&row_offsets, &columns));
            Ok(())
        })?;
        meter.checkpoint("CSR construction finish")?;
        Ok((
            Self {
                vertex_count,
                row_offsets,
                columns,
                weights,
                source_lineage: Arc::clone(graph.lineage()),
            },
            profile,
        ))
    }

    fn from_laplacian_impl<const PROFILE: bool, M: Meter>(
        graph: &Laplacian,
        meter: &mut M,
    ) -> Result<(Self, CsrBuildProfile), CmgError> {
        let mut profile = CsrBuildProfile::default();
        let vertex_count = graph.vertex_count();
        let directed_entries =
            graph
                .edge_count()
                .checked_mul(2)
                .ok_or(CmgError::InvalidHierarchy {
                    context: "CSR directed-entry count overflowed usize",
                })?;

        let row_counts =
            measure_build_phase::<PROFILE, _>(&mut profile.row_counts_nanoseconds, || {
                let mut counts = meter.try_filled(vertex_count, 0_usize, "CSR row counts")?;
                chunked(meter, "CSR row counts", graph.edge_count(), 8, |range| {
                    for edge in &graph.edges()[range] {
                        counts[edge.u()] += 1;
                        counts[edge.v()] += 1;
                    }
                    Ok(())
                })?;
                Ok::<_, CmgError>(counts)
            })?;
        let (row_offsets, mut next) =
            measure_build_phase::<PROFILE, _>(&mut profile.row_offsets_nanoseconds, || {
                row_offsets_and_cursors(row_counts, directed_entries, meter)
            })?;
        measure_build_phase::<PROFILE, _>(&mut profile.validation_nanoseconds, || {
            if row_offsets.last() != directed_entries {
                Err(CmgError::InvalidHierarchy {
                    context: "CSR row counts do not match directed-entry count",
                })
            } else {
                Ok(())
            }
        })?;

        let mut weights =
            measure_build_phase::<PROFILE, _>(&mut profile.allocation_nanoseconds, || {
                meter.try_filled(directed_entries, 0.0, "CSR weights")
            })?;
        let columns = if vertex_count <= u32::MAX as usize {
            let mut columns =
                measure_build_phase::<PROFILE, _>(&mut profile.allocation_nanoseconds, || {
                    meter.try_filled(directed_entries, 0_u32, "CSR compact columns")
                })?;
            measure_build_phase::<PROFILE, _>(&mut profile.scatter_nanoseconds, || {
                scatter_rows(
                    graph,
                    &mut next,
                    &mut columns,
                    &mut weights,
                    |vertex| vertex as u32,
                    meter,
                )
            })?;
            ColumnIndices::Compact(columns)
        } else {
            let mut columns =
                measure_build_phase::<PROFILE, _>(&mut profile.allocation_nanoseconds, || {
                    meter.try_filled(directed_entries, 0_usize, "CSR native columns")
                })?;
            measure_build_phase::<PROFILE, _>(&mut profile.scatter_nanoseconds, || {
                scatter_rows(
                    graph,
                    &mut next,
                    &mut columns,
                    &mut weights,
                    |vertex| vertex,
                    meter,
                )
            })?;
            ColumnIndices::Native(columns)
        };

        measure_build_phase::<PROFILE, _>(&mut profile.validation_nanoseconds, || {
            if PROFILE && !rows_are_sorted(&row_offsets, &columns) {
                return Err(CmgError::InvalidHierarchy {
                    context: "profiled CSR rows are not sorted",
                });
            }
            debug_assert!(rows_are_sorted(&row_offsets, &columns));
            Ok(())
        })?;
        meter.checkpoint("CSR construction finish")?;
        Ok((
            Self {
                vertex_count,
                row_offsets,
                columns,
                weights,
                #[cfg(feature = "parallel")]
                source_lineage: Arc::clone(graph.lineage()),
            },
            profile,
        ))
    }

    /// Return the number of rows and vertices.
    #[must_use]
    pub const fn vertex_count(&self) -> usize {
        self.vertex_count
    }

    /// Return the number of directed off-diagonal entries.
    #[must_use]
    pub fn directed_entry_count(&self) -> usize {
        self.weights.len()
    }

    #[cfg(feature = "parallel")]
    pub(crate) fn shares_lineage(&self, graph: &Laplacian) -> bool {
        Arc::ptr_eq(&self.source_lineage, graph.lineage())
    }

    /// Return whether neighbor indices use four-byte storage.
    #[must_use]
    pub const fn uses_compact_indices(&self) -> bool {
        self.columns.is_compact()
    }

    /// Return whether row offsets use four-byte storage.
    #[must_use]
    pub const fn uses_compact_row_offsets(&self) -> bool {
        self.row_offsets.is_compact()
    }

    /// Return the principal retained heap bytes.
    #[must_use]
    pub fn byte_len(&self) -> usize {
        self.row_offsets
            .byte_len()
            .saturating_add(self.columns.byte_len())
            .saturating_add(
                self.weights
                    .len()
                    .saturating_mul(core::mem::size_of::<f64>()),
            )
    }

    /// Compute `output = L * input` without allocating.
    pub fn matvec_into(&self, input: &[f64], output: &mut [f64]) -> Result<(), CmgError> {
        self.validate_matvec_dimensions(input, output)?;
        self.matvec_lanes(
            input,
            output,
            crate::preconditioner::One,
            None,
            &mut crate::cancel::Unmetered,
        )
    }

    /// Compute `output = L * input` on a borrowed team.
    ///
    /// Every row is evaluated in its canonical neighbor order, so the
    /// arithmetic for an individual row is independent of worker scheduling.
    /// Small problems and one-worker teams use the serial row kernel.
    #[cfg(feature = "parallel")]
    pub fn matvec_into_with_team(
        &self,
        input: &[f64],
        output: &mut [f64],
        team: Team<'_>,
    ) -> Result<(), CmgError> {
        self.validate_matvec_dimensions(input, output)?;
        self.matvec_lanes(
            input,
            output,
            crate::preconditioner::One,
            Some(team),
            &mut crate::cancel::Unmetered,
        )
    }

    /// The one row (gather) kernel for vertex-major lanes: every lane of a row
    /// sums its neighbors in canonical order from zero. When its work reaches
    /// the team's parallel threshold ([`Self::matvec_uses_team`]), fixed row
    /// blocks run on the team and poll the meter's query; serially, rows are
    /// counted by their entries.
    pub(crate) fn matvec_lanes<W: Width, M: Meter>(
        &self,
        input: &[f64],
        output: &mut [f64],
        width: W,
        #[cfg_attr(not(feature = "parallel"), allow(unused_variables))] team: Option<
            crate::execution::Team<'_>,
        >,
        meter: &mut M,
    ) -> Result<(), CmgError> {
        match &self.columns {
            ColumnIndices::Compact(columns) => {
                self.matvec_rows(columns, input, output, width, team, meter)
            }
            ColumnIndices::Native(columns) => {
                self.matvec_rows(columns, input, output, width, team, meter)
            }
        }
    }

    #[inline(always)]
    fn matvec_rows<C: crate::storage::StoredIndex, W: Width, M: Meter>(
        &self,
        neighbors: &[C],
        input: &[f64],
        output: &mut [f64],
        width: W,
        #[cfg_attr(not(feature = "parallel"), allow(unused_variables))] team: Option<
            crate::execution::Team<'_>,
        >,
        meter: &mut M,
    ) -> Result<(), CmgError> {
        let columns = width.get();
        #[cfg(feature = "parallel")]
        if let Some(team) = team.filter(|&team| self.matvec_uses_team(team, columns)) {
            let rows = crate::preconditioner::rows_parallel_with(
                team,
                meter.query(),
                output,
                columns,
                |row, values, poll| self.row_product(neighbors, input, columns, row, values, poll),
            );
            meter.joined("CSR matvec")?;
            return rows;
        }
        for (row, values) in output.chunks_exact_mut(columns).enumerate() {
            self.row_product(neighbors, input, columns, row, values, meter)?;
        }
        Ok(())
    }

    /// Whether a product over `columns` lanes runs on `team`. The gate counts
    /// the row kernel's work, directed entries times lanes (at least one unit
    /// per row), against the team's `min_parallel_len`, so a level with few
    /// dense rows runs on the team while tiny levels stay serial. Rows keep
    /// their serial arithmetic either way, so the choice never changes bits.
    #[cfg(feature = "parallel")]
    #[inline]
    pub(crate) fn matvec_uses_team(&self, team: Team<'_>, columns: usize) -> bool {
        team.should_parallel(
            self.directed_entry_count()
                .saturating_mul(columns)
                .max(self.vertex_count),
        )
    }

    /// One row of every lane, summed from zero over its neighbors in canonical
    /// order. A polling progress counts the row in segments while it runs, so
    /// a hub row is interruptible; the accumulation order is unchanged.
    #[inline(always)]
    fn row_product<C: crate::storage::StoredIndex, P: crate::cancel::Progress>(
        &self,
        neighbors: &[C],
        input: &[f64],
        columns: usize,
        row: usize,
        values: &mut [f64],
        progress: &mut P,
    ) -> Result<(), CmgError> {
        let (start, end) = self.row_offsets.bounds(row);
        // A batch wider than one counted segment traverses the row once per
        // lane tile; every lane still sums its neighbors in canonical order.
        for lanes in crate::cancel::lane_tiles::<P>(columns, 1) {
            let tile = &mut values[lanes.clone()];
            tile.fill(0.0);
            crate::cancel::chunked(progress, "CSR matvec", end - start, tile.len(), |range| {
                let entries = start + range.start..start + range.end;
                for (neighbor, &weight) in neighbors[entries.clone()]
                    .iter()
                    .zip(&self.weights[entries])
                {
                    let neighbor = neighbor.index();
                    for (lane, value) in (lanes.start..).zip(tile.iter_mut()) {
                        *value += weight
                            * (input[row * columns + lane] - input[neighbor * columns + lane]);
                    }
                }
                Ok(())
            })?;
            progress.advance("CSR matvec", tile.len())?;
        }
        Ok(())
    }

    fn validate_matvec_dimensions(&self, input: &[f64], output: &[f64]) -> Result<(), CmgError> {
        if input.len() != self.vertex_count {
            return Err(CmgError::dimension(
                "CsrLaplacian::matvec input",
                self.vertex_count,
                input.len(),
            ));
        }
        if output.len() != self.vertex_count {
            return Err(CmgError::dimension(
                "CsrLaplacian::matvec output",
                self.vertex_count,
                output.len(),
            ));
        }
        Ok(())
    }

    /// Compute and return `L * input`.
    pub fn matvec(&self, input: &[f64]) -> Result<Vec<f64>, CmgError> {
        let mut output = vec![0.0; self.vertex_count];
        self.matvec_into(input, &mut output)?;
        Ok(output)
    }
}

/// Serial scatter of every edge into both of its rows, in canonical order.
fn scatter_rows<C: Copy, M: Meter>(
    graph: &Laplacian,
    next: &mut [usize],
    columns: &mut [C],
    weights: &mut [f64],
    column: impl Fn(usize) -> C,
    meter: &mut M,
) -> Result<(), CmgError> {
    chunked(meter, "CSR scatter", graph.edge_count(), 16, |range| {
        for edge in &graph.edges()[range] {
            let left = next[edge.u()];
            columns[left] = column(edge.v());
            weights[left] = edge.weight();
            next[edge.u()] += 1;

            let right = next[edge.v()];
            columns[right] = column(edge.u());
            weights[right] = edge.weight();
            next[edge.v()] += 1;
        }
        Ok(())
    })
}

fn row_offsets_and_cursors<M: Meter>(
    mut counts: Vec<usize>,
    directed_entries: usize,
    meter: &mut M,
) -> Result<(RowOffsets, Vec<usize>), CmgError> {
    // Once the prefix sum has consumed each degree, the degree buffer has the
    // exact shape needed by the scatter cursors. Rewriting it in place avoids
    // allocating and initializing a second per-row `Vec<usize>`.
    if directed_entries <= u32::MAX as usize {
        let mut offsets = meter.try_capacity(add(counts.len(), 1)?, "CSR compact row offsets")?;
        offsets.push(0_u32);
        let mut running = 0_usize;
        chunked(meter, "CSR row offsets", counts.len(), 8, |range| {
            for count in &mut counts[range] {
                let width = *count;
                *count = running;
                running = running
                    .checked_add(width)
                    .ok_or(CmgError::InvalidHierarchy {
                        context: "CSR row offsets overflowed usize",
                    })?;
                offsets.push(
                    u32::try_from(running).map_err(|_| CmgError::InvalidHierarchy {
                        context: "CSR compact row offset exceeded u32::MAX",
                    })?,
                );
            }
            Ok(())
        })?;
        Ok((RowOffsets::Compact(offsets), counts))
    } else {
        let mut offsets = meter.try_capacity(add(counts.len(), 1)?, "CSR native row offsets")?;
        offsets.push(0_usize);
        let mut running = 0_usize;
        chunked(meter, "CSR row offsets", counts.len(), 8, |range| {
            for count in &mut counts[range] {
                let width = *count;
                *count = running;
                running = running
                    .checked_add(width)
                    .ok_or(CmgError::InvalidHierarchy {
                        context: "CSR row offsets overflowed usize",
                    })?;
                offsets.push(running);
            }
            Ok(())
        })?;
        Ok((RowOffsets::Native(offsets), counts))
    }
}

#[cfg(feature = "parallel")]
fn native_offsets_and_cursors<M: Meter>(
    mut counts: Vec<usize>,
    expected_entries: usize,
    meter: &mut M,
) -> Result<(Vec<usize>, Vec<usize>), CmgError> {
    let mut offsets = meter.try_capacity(add(counts.len(), 1)?, "CSR auxiliary row offsets")?;
    offsets.push(0);
    let mut running = 0_usize;
    chunked(meter, "CSR auxiliary offsets", counts.len(), 8, |range| {
        for count in &mut counts[range] {
            let width = *count;
            *count = running;
            running = running
                .checked_add(width)
                .ok_or(CmgError::InvalidHierarchy {
                    context: "CSR auxiliary offsets overflowed usize",
                })?;
            offsets.push(running);
        }
        Ok(())
    })?;
    if running != expected_entries {
        return Err(CmgError::InvalidHierarchy {
            context: "CSR auxiliary offsets do not match edge count",
        });
    }
    Ok((offsets, counts))
}

#[cfg(feature = "parallel")]
struct RowFillBlock<'a, Column> {
    first_row: usize,
    entry_base: usize,
    columns: &'a mut [Column],
    weights: &'a mut [f64],
}

#[cfg(feature = "parallel")]
fn row_fill_blocks<'a, Column, M: Meter>(
    columns: &'a mut [Column],
    weights: &'a mut [f64],
    row_offsets: &RowOffsets,
    rows_per_block: usize,
    meter: &mut M,
) -> Result<Vec<RowFillBlock<'a, Column>>, CmgError> {
    let mut blocks = meter.try_capacity(
        row_offsets.row_count().div_ceil(rows_per_block),
        "CSR fill blocks",
    )?;
    let mut remaining_columns = columns;
    let mut remaining_weights = weights;
    let mut first_row = 0_usize;
    let mut entry_base = 0_usize;
    while first_row < row_offsets.row_count() {
        let end_row = first_row
            .saturating_add(rows_per_block)
            .min(row_offsets.row_count());
        let entry_end = if end_row == row_offsets.row_count() {
            row_offsets.last()
        } else {
            row_offsets.bounds(end_row).0
        };
        meter.work("CSR fill blocks", 32)?;
        let block_len = entry_end - entry_base;
        let (block_columns, columns_tail) = remaining_columns.split_at_mut(block_len);
        let (block_weights, weights_tail) = remaining_weights.split_at_mut(block_len);
        blocks.push(RowFillBlock {
            first_row,
            entry_base,
            columns: block_columns,
            weights: block_weights,
        });
        remaining_columns = columns_tail;
        remaining_weights = weights_tail;
        first_row = end_row;
        entry_base = entry_end;
    }
    debug_assert!(remaining_columns.is_empty());
    debug_assert!(remaining_weights.is_empty());
    Ok(blocks)
}

/// Borrowed inputs of the parallel row fill.
#[cfg(feature = "parallel")]
struct RowFill<'a> {
    graph: &'a Laplacian,
    row_offsets: &'a RowOffsets,
    incoming_offsets: &'a [usize],
    outgoing_offsets: &'a [usize],
    incoming_edges: &'a [usize],
    rows_per_block: usize,
}

#[cfg(feature = "parallel")]
impl RowFill<'_> {
    /// Fill fixed row blocks on the team; each lane counts every row's
    /// entries in segments while it writes them and polls the meter's query.
    fn rows_parallel<C: Copy + Send, M: Meter>(
        &self,
        columns: &mut [C],
        weights: &mut [f64],
        column: impl Fn(usize) -> C + Sync,
        team: Team<'_>,
        meter: &mut M,
    ) -> Result<(), CmgError> {
        let mut blocks = row_fill_blocks(
            columns,
            weights,
            self.row_offsets,
            self.rows_per_block,
            meter,
        )?;
        let filled = team.chunks_mut(&mut blocks, 1, meter.query(), |_, block, poll| {
            self.block(&mut block[0], &column, poll)
        });
        meter.joined("parallel CSR fill")?;
        filled
    }

    fn block<C: Copy>(
        &self,
        block: &mut RowFillBlock<'_, C>,
        column: &impl Fn(usize) -> C,
        poll: &mut crate::cancel::LanePoll<'_>,
    ) -> Result<(), CmgError> {
        let graph = self.graph;
        let end_row = block
            .first_row
            .saturating_add(self.rows_per_block)
            .min(graph.vertex_count());
        for row in block.first_row..end_row {
            let mut position = self.row_offsets.bounds(row).0 - block.entry_base;
            let incoming =
                &self.incoming_edges[self.incoming_offsets[row]..self.incoming_offsets[row + 1]];
            chunked(poll, "parallel CSR fill", incoming.len(), 8, |range| {
                for &edge_index in &incoming[range] {
                    let edge = graph.edges()[edge_index];
                    block.columns[position] = column(edge.u());
                    block.weights[position] = edge.weight();
                    position += 1;
                }
                Ok(())
            })?;
            let outgoing =
                &graph.edges()[self.outgoing_offsets[row]..self.outgoing_offsets[row + 1]];
            chunked(poll, "parallel CSR fill", outgoing.len(), 8, |range| {
                for edge in &outgoing[range] {
                    debug_assert_eq!(edge.u(), row);
                    block.columns[position] = column(edge.v());
                    block.weights[position] = edge.weight();
                    position += 1;
                }
                Ok(())
            })?;
            debug_assert_eq!(position, self.row_offsets.bounds(row).1 - block.entry_base);
            crate::cancel::Progress::advance(poll, "parallel CSR fill", 8)?;
        }
        Ok(())
    }
}

fn rows_are_sorted(row_offsets: &RowOffsets, columns: &ColumnIndices) -> bool {
    (0..row_offsets.row_count()).all(|row| {
        let (start, end) = row_offsets.bounds(row);
        match columns {
            ColumnIndices::Compact(columns) => {
                columns[start..end].windows(2).all(|pair| pair[0] < pair[1])
            }
            ColumnIndices::Native(columns) => {
                columns[start..end].windows(2).all(|pair| pair[0] < pair[1])
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::CsrLaplacian;
    use crate::Laplacian;

    fn assert_close(left: &[f64], right: &[f64]) {
        assert_eq!(left.len(), right.len());
        for (left_value, right_value) in left.iter().zip(right) {
            let scale = 1.0_f64.max(left_value.abs()).max(right_value.abs());
            assert!((left_value - right_value).abs() <= 2.0e-15 * scale);
        }
    }

    #[test]
    fn row_matvec_matches_edge_matvec_on_varied_graphs() {
        let graphs = [
            Laplacian::from_edges(1, []).unwrap(),
            Laplacian::from_edges(4, [(0, 1, 2.0), (1, 2, 3.0), (2, 3, 5.0)]).unwrap(),
            Laplacian::from_edges(6, (1..6).map(|leaf| (0, leaf, leaf as f64))).unwrap(),
            Laplacian::from_edges(
                7,
                [
                    (6, 2, 1.0),
                    (0, 4, 2.0),
                    (4, 0, 3.0),
                    (1, 5, 4.0),
                    (2, 3, 5.0),
                ],
            )
            .unwrap(),
        ];

        for graph in graphs {
            let input: Vec<f64> = (0..graph.vertex_count())
                .map(|vertex| (vertex as f64 - 2.5) / 3.0)
                .collect();
            let edge_result = graph.matvec(&input).unwrap();
            let csr = CsrLaplacian::from_laplacian(&graph).unwrap();
            let csr_result = csr.matvec(&input).unwrap();
            assert_close(&edge_result, &csr_result);
            assert_eq!(csr.directed_entry_count(), 2 * graph.edge_count());
            assert_eq!(csr.vertex_count(), graph.vertex_count());
        }
    }

    #[test]
    fn compact_storage_is_used_for_normal_graph_dimensions() {
        let graph = Laplacian::from_edges(3, [(0, 1, 1.0), (1, 2, 1.0)]).unwrap();
        let csr = CsrLaplacian::from_laplacian(&graph).unwrap();
        assert!(csr.uses_compact_indices());
        assert!(csr.uses_compact_row_offsets());
        assert!(csr.byte_len() >= (graph.vertex_count() + 1) * core::mem::size_of::<u32>());
    }

    #[cfg(feature = "parallel")]
    #[test]
    fn parallel_row_matvec_is_bitwise_equal_to_serial_row_matvec() {
        use crate::{ParallelExecutor, ParallelOptions};

        let graph = Laplacian::from_edges(
            20_000,
            (0..19_999).map(|vertex| (vertex, vertex + 1, 0.5 + (vertex % 31) as f64 / 17.0)),
        )
        .unwrap();
        let csr = CsrLaplacian::from_laplacian(&graph).unwrap();
        let input: Vec<f64> = (0..graph.vertex_count())
            .map(|vertex| ((vertex * 37) % 101) as f64 - 50.0)
            .collect();
        let mut serial = vec![0.0; graph.vertex_count()];
        let mut parallel = vec![0.0; graph.vertex_count()];
        csr.matvec_into(&input, &mut serial).unwrap();
        let executor = ParallelExecutor::new(ParallelOptions {
            threads: 4,
            min_parallel_len: 1,
            ..ParallelOptions::default()
        })
        .unwrap();
        csr.matvec_into_with_team(&input, &mut parallel, executor.team())
            .unwrap();
        assert_eq!(serial, parallel);
    }

    #[cfg(feature = "parallel")]
    #[test]
    fn parallel_dense_construction_matches_serial_storage_exactly() {
        use crate::{ParallelExecutor, ParallelOptions};

        let vertices = 2_000;
        let graph = Laplacian::from_edges(
            vertices,
            (0..vertices).flat_map(|left| {
                (1..=12).map(move |offset| {
                    let right = (left + offset) % vertices;
                    (left, right, 0.5 + ((left + 7 * offset) % 19) as f64 / 11.0)
                })
            }),
        )
        .unwrap();
        let serial = CsrLaplacian::from_laplacian(&graph).unwrap();
        for threads in [2, 4, 8] {
            let executor = ParallelExecutor::new(ParallelOptions {
                threads,
                min_parallel_len: 1,
                ..ParallelOptions::default()
            })
            .unwrap();
            let (parallel, _) = CsrLaplacian::from_laplacian_with_team_impl::<false, _>(
                &graph,
                executor.team(),
                &mut crate::cancel::Unmetered,
            )
            .unwrap();
            assert_eq!(parallel, serial);
            assert_eq!(parallel.byte_len(), serial.byte_len());
        }
    }
}

/// A hub row is counted while it is traversed: cancellation at any segment
/// leaves exactly the entries before it accumulated, serially and in lanes.
#[cfg(test)]
mod hub_row_cancellation_tests {
    use super::CsrLaplacian;
    use crate::Laplacian;
    use crate::cancel::Recorder;

    const LEAVES: usize = 200_000;

    fn star() -> (Laplacian, CsrLaplacian, Vec<f64>) {
        let graph =
            Laplacian::from_edges(LEAVES + 1, (1..=LEAVES).map(|leaf| (0, leaf, 1.0))).unwrap();
        let operator = CsrLaplacian::from_laplacian(&graph).unwrap();
        // Every hub term is weight * (1 - 0) = 1, so the hub output counts terms.
        let mut input = vec![0.0; LEAVES + 1];
        input[0] = 1.0;
        (graph, operator, input)
    }

    #[test]
    fn serial_hub_row_polls_inside_its_traversal() {
        let (_, operator, input) = star();
        let super::ColumnIndices::Compact(neighbors) = &operator.columns else {
            panic!("compact star columns");
        };
        let mut row = [0.0];
        let mut full = Recorder::default();
        operator
            .row_product(neighbors, &input, 1, 0, &mut row, &mut full)
            .unwrap();
        assert_eq!(row[0], LEAVES as f64);
        assert_eq!(full.advances, LEAVES.div_ceil(1024) + 1);
        assert!(full.max_units <= 1024);
        for stop in [1, 7, full.advances - 2, full.advances - 1] {
            let mut cancelled = Recorder {
                stop_at: Some(stop),
                ..Recorder::default()
            };
            let mut row = [0.0];
            assert!(
                operator
                    .row_product(neighbors, &input, 1, 0, &mut row, &mut cancelled)
                    .is_err()
            );
            // Exactly `stop` segments of at most 1024 terms ran before it.
            assert_eq!(row[0], (stop * 1024).min(LEAVES) as f64);
        }
    }

    /// A batch wider than one counted segment runs a row per lane tile: every
    /// count covers at most 1024 lane-entry updates, and a cancellation at
    /// the first count leaves only the first tile's first entry applied.
    #[test]
    fn wide_batch_rows_are_counted_in_lane_tiles() {
        let graph = Laplacian::from_edges(40, (1..40).map(|leaf| (0, leaf, 1.0))).unwrap();
        let operator = CsrLaplacian::from_laplacian(&graph).unwrap();
        let super::ColumnIndices::Compact(neighbors) = &operator.columns else {
            panic!("compact star columns");
        };
        let lanes = 1_100;
        let mut input = vec![0.0; 40 * lanes];
        input[..lanes].fill(1.0);
        let mut row = vec![f64::NAN; lanes];
        let mut full = Recorder::default();
        operator
            .row_product(neighbors, &input, lanes, 0, &mut row, &mut full)
            .unwrap();
        assert_eq!(row, vec![39.0; lanes]);
        assert!(full.max_units <= 1024, "{}", full.max_units);
        // 39 single-entry segments plus the tile count in the full tile; three
        // 13-entry segments plus the tile count in the 76-lane tile.
        assert_eq!(full.advances, (39 + 1) + (3 + 1));
        assert_eq!(full.units, 39 * lanes + lanes);
        let mut row = vec![f64::NAN; lanes];
        let mut cancelled = Recorder {
            stop_at: Some(1),
            ..Recorder::default()
        };
        assert!(
            operator
                .row_product(neighbors, &input, lanes, 0, &mut row, &mut cancelled)
                .is_err()
        );
        assert_eq!(row[..1024], [1.0; 1024]);
        assert!(row[1024..].iter().all(|value| value.is_nan()));
    }

    #[cfg(feature = "parallel")]
    #[test]
    fn lane_polls_inside_a_hub_row_and_stops_there() {
        let (_, operator, input) = star();
        let executor = crate::ParallelExecutor::new(crate::ParallelOptions {
            threads: 2,
            min_parallel_len: 1,
            ..crate::ParallelOptions::default()
        })
        .unwrap();
        let cancel = || true;
        let mut meter = crate::cancel::CancelMeter::new(&cancel);
        let mut output = vec![f64::NAN; LEAVES + 1];
        let result = operator.matvec_lanes(
            &input,
            &mut output,
            crate::preconditioner::One,
            Some(executor.team()),
            &mut meter,
        );
        assert!(matches!(result, Err(crate::CmgError::Cancelled { .. })));
        // The lane that claimed block 0 stopped inside the hub row at its first
        // poll: 64 segments of 1024 terms, far fewer than the row's 200,000.
        assert_eq!(output[0], 65_536.0);
        let mut output = vec![0.0; LEAVES + 1];
        operator
            .matvec_lanes(
                &input,
                &mut output,
                crate::preconditioner::One,
                Some(executor.team()),
                &mut crate::cancel::Unmetered,
            )
            .unwrap();
        assert_eq!(output[0], LEAVES as f64);
    }
}

#[cfg(test)]
mod compact_row_offset_tests {
    use super::CsrLaplacian;
    use crate::Laplacian;

    #[test]
    fn compact_offsets_preserve_row_matvec() {
        let graph = Laplacian::from_edges(
            10,
            (0..9).map(|vertex| (vertex, vertex + 1, 1.0 + vertex as f64)),
        )
        .unwrap();
        let csr = CsrLaplacian::from_laplacian(&graph).unwrap();
        assert!(csr.uses_compact_row_offsets());
        let input: Vec<_> = (0..10).map(|index| index as f64 - 4.0).collect();
        assert_eq!(csr.matvec(&input).unwrap(), graph.matvec(&input).unwrap());
    }
}
