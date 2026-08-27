//! Deterministic row-oriented Laplacian storage for solve kernels.

#[cfg(feature = "parallel")]
use crate::ParallelExecutor;
use crate::{CmgError, Laplacian};
#[cfg(feature = "parallel")]
use rayon::prelude::*;
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

impl PartialEq for CsrLaplacian {
    fn eq(&self, other: &Self) -> bool {
        self.vertex_count == other.vertex_count
            && self.row_offsets == other.row_offsets
            && self.columns == other.columns
            && self.weights == other.weights
    }
}

impl CsrLaplacian {
    /// Freeze a canonical edge-list Laplacian into deterministic row storage.
    pub fn from_laplacian(graph: &Laplacian) -> Result<Self, CmgError> {
        Self::from_laplacian_impl::<false>(graph).map(|(operator, _)| operator)
    }

    #[cfg(feature = "parallel")]
    pub(crate) fn from_laplacian_with_executor(
        graph: &Laplacian,
        executor: &ParallelExecutor,
    ) -> Result<Self, CmgError> {
        Self::from_laplacian_with_executor_impl::<false>(graph, executor)
            .map(|(operator, _)| operator)
    }

    #[cfg(all(feature = "parallel", feature = "profiling"))]
    pub(crate) fn from_laplacian_with_executor_profiled(
        graph: &Laplacian,
        executor: &ParallelExecutor,
    ) -> Result<(Self, CsrBuildProfile), CmgError> {
        Self::from_laplacian_with_executor_impl::<true>(graph, executor)
    }

    #[cfg(feature = "parallel")]
    fn from_laplacian_with_executor_impl<const PROFILE: bool>(
        graph: &Laplacian,
        executor: &ParallelExecutor,
    ) -> Result<(Self, CsrBuildProfile), CmgError> {
        let dense_enough = graph.edge_count() >= graph.vertex_count().saturating_mul(4);
        if executor.thread_count() <= 1
            || !executor.should_parallel(graph.edge_count())
            || !dense_enough
        {
            return Self::from_laplacian_impl::<PROFILE>(graph);
        }
        Self::from_laplacian_parallel_impl::<PROFILE>(graph, executor)
    }

    #[cfg(feature = "parallel")]
    fn from_laplacian_parallel_impl<const PROFILE: bool>(
        graph: &Laplacian,
        executor: &ParallelExecutor,
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
                let mut row_counts = vec![0_usize; vertex_count];
                let mut incoming_counts = vec![0_usize; vertex_count];
                let mut outgoing_counts = vec![0_usize; vertex_count];
                for edge in graph.edges() {
                    row_counts[edge.u()] += 1;
                    row_counts[edge.v()] += 1;
                    outgoing_counts[edge.u()] += 1;
                    incoming_counts[edge.v()] += 1;
                }
                (row_counts, incoming_counts, outgoing_counts)
            });
        let (row_offsets, incoming_offsets, outgoing_offsets, mut incoming_next) =
            measure_build_phase::<PROFILE, _>(&mut profile.row_offsets_nanoseconds, || {
                let (row_offsets, _) = row_offsets_and_cursors(row_counts, directed_entries)?;
                let (incoming_offsets, incoming_next) =
                    native_offsets_and_cursors(incoming_counts, edge_count)?;
                let (outgoing_offsets, _) =
                    native_offsets_and_cursors(outgoing_counts, edge_count)?;
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
                vec![0_usize; edge_count]
            });
        measure_build_phase::<PROFILE, _>(&mut profile.scatter_nanoseconds, || {
            for (edge_index, edge) in graph.edges().iter().enumerate() {
                let position = incoming_next[edge.v()];
                incoming_edges[position] = edge_index;
                incoming_next[edge.v()] += 1;
            }
        });
        drop(incoming_next);

        let mut weights =
            measure_build_phase::<PROFILE, _>(&mut profile.allocation_nanoseconds, || {
                vec![0.0; directed_entries]
            });
        let columns = if vertex_count <= u32::MAX as usize {
            let mut columns =
                measure_build_phase::<PROFILE, _>(&mut profile.allocation_nanoseconds, || {
                    vec![0_u32; directed_entries]
                });
            measure_build_phase::<PROFILE, _>(&mut profile.scatter_nanoseconds, || {
                fill_compact_rows_parallel(
                    graph,
                    &row_offsets,
                    &incoming_offsets,
                    &outgoing_offsets,
                    &incoming_edges,
                    &mut columns,
                    &mut weights,
                    executor,
                );
            });
            ColumnIndices::Compact(columns)
        } else {
            let mut columns =
                measure_build_phase::<PROFILE, _>(&mut profile.allocation_nanoseconds, || {
                    vec![0_usize; directed_entries]
                });
            measure_build_phase::<PROFILE, _>(&mut profile.scatter_nanoseconds, || {
                fill_native_rows_parallel(
                    graph,
                    &row_offsets,
                    &incoming_offsets,
                    &outgoing_offsets,
                    &incoming_edges,
                    &mut columns,
                    &mut weights,
                    executor,
                );
            });
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

    fn from_laplacian_impl<const PROFILE: bool>(
        graph: &Laplacian,
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
                let mut counts = vec![0_usize; vertex_count];
                for edge in graph.edges() {
                    counts[edge.u()] += 1;
                    counts[edge.v()] += 1;
                }
                counts
            });
        let (row_offsets, mut next) =
            measure_build_phase::<PROFILE, _>(&mut profile.row_offsets_nanoseconds, || {
                row_offsets_and_cursors(row_counts, directed_entries)
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
                vec![0.0; directed_entries]
            });
        let columns = if vertex_count <= u32::MAX as usize {
            let mut columns =
                measure_build_phase::<PROFILE, _>(&mut profile.allocation_nanoseconds, || {
                    vec![0_u32; directed_entries]
                });
            measure_build_phase::<PROFILE, _>(&mut profile.scatter_nanoseconds, || {
                for edge in graph.edges() {
                    let left = next[edge.u()];
                    columns[left] = edge.v() as u32;
                    weights[left] = edge.weight();
                    next[edge.u()] += 1;

                    let right = next[edge.v()];
                    columns[right] = edge.u() as u32;
                    weights[right] = edge.weight();
                    next[edge.v()] += 1;
                }
            });
            ColumnIndices::Compact(columns)
        } else {
            let mut columns =
                measure_build_phase::<PROFILE, _>(&mut profile.allocation_nanoseconds, || {
                    vec![0_usize; directed_entries]
                });
            measure_build_phase::<PROFILE, _>(&mut profile.scatter_nanoseconds, || {
                for edge in graph.edges() {
                    let left = next[edge.u()];
                    columns[left] = edge.v();
                    weights[left] = edge.weight();
                    next[edge.u()] += 1;

                    let right = next[edge.v()];
                    columns[right] = edge.u();
                    weights[right] = edge.weight();
                    next[edge.v()] += 1;
                }
            });
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

        match &self.columns {
            ColumnIndices::Compact(columns) => {
                for row in 0..self.vertex_count {
                    let center = input[row];
                    let mut sum = 0.0;
                    let (start, end) = self.row_offsets.bounds(row);
                    for index in start..end {
                        sum += self.weights[index] * (center - input[columns[index] as usize]);
                    }
                    output[row] = sum;
                }
            }
            ColumnIndices::Native(columns) => {
                for row in 0..self.vertex_count {
                    let center = input[row];
                    let mut sum = 0.0;
                    let (start, end) = self.row_offsets.bounds(row);
                    for index in start..end {
                        sum += self.weights[index] * (center - input[columns[index]]);
                    }
                    output[row] = sum;
                }
            }
        }
        Ok(())
    }

    /// Compute `output = L * input` using the supplied package-owned pool.
    ///
    /// Every row is evaluated in its canonical neighbor order, so the
    /// arithmetic for an individual row is independent of worker scheduling.
    /// Small problems and one-thread executors use the serial row kernel.
    #[cfg(feature = "parallel")]
    pub fn matvec_into_parallel(
        &self,
        input: &[f64],
        output: &mut [f64],
        executor: &ParallelExecutor,
    ) -> Result<(), CmgError> {
        self.validate_matvec_dimensions(input, output)?;
        if !executor.should_parallel(self.vertex_count) {
            return self.matvec_into(input, output);
        }

        let rows_per_chunk = executor.work_chunk_len(self.vertex_count);
        executor.install(|| match &self.columns {
            ColumnIndices::Compact(columns) => output
                .par_chunks_mut(rows_per_chunk)
                .enumerate()
                .for_each(|(chunk_index, chunk)| {
                    let first_row = chunk_index * rows_per_chunk;
                    for (offset, value) in chunk.iter_mut().enumerate() {
                        let row = first_row + offset;
                        let center = input[row];
                        let mut sum = 0.0;
                        let (start, end) = self.row_offsets.bounds(row);
                        for index in start..end {
                            sum += self.weights[index] * (center - input[columns[index] as usize]);
                        }
                        *value = sum;
                    }
                }),
            ColumnIndices::Native(columns) => output
                .par_chunks_mut(rows_per_chunk)
                .enumerate()
                .for_each(|(chunk_index, chunk)| {
                    let first_row = chunk_index * rows_per_chunk;
                    for (offset, value) in chunk.iter_mut().enumerate() {
                        let row = first_row + offset;
                        let center = input[row];
                        let mut sum = 0.0;
                        let (start, end) = self.row_offsets.bounds(row);
                        for index in start..end {
                            sum += self.weights[index] * (center - input[columns[index]]);
                        }
                        *value = sum;
                    }
                }),
        });
        Ok(())
    }

    #[cfg(feature = "parallel")]
    pub(crate) fn maximum_weight_neighbors_with_executor(
        &self,
        executor: &ParallelExecutor,
    ) -> (Vec<usize>, Vec<f64>) {
        if !executor.should_parallel(self.directed_entry_count()) {
            return self.maximum_weight_neighbors_serial();
        }
        let selections: Vec<(usize, f64)> = executor.install(|| {
            (0..self.vertex_count)
                .into_par_iter()
                .map(|row| self.maximum_weight_neighbor(row))
                .collect()
        });
        selections.into_iter().unzip()
    }

    #[cfg(feature = "parallel")]
    fn maximum_weight_neighbors_serial(&self) -> (Vec<usize>, Vec<f64>) {
        (0..self.vertex_count)
            .map(|row| self.maximum_weight_neighbor(row))
            .unzip()
    }

    #[cfg(feature = "parallel")]
    fn maximum_weight_neighbor(&self, row: usize) -> (usize, f64) {
        let mut best_neighbor = row;
        let mut best_weight = 0.0;
        match &self.columns {
            ColumnIndices::Compact(columns) => {
                let (start, end) = self.row_offsets.bounds(row);
                for (&neighbor, &weight) in
                    columns[start..end].iter().zip(&self.weights[start..end])
                {
                    let neighbor = neighbor as usize;
                    if weight > best_weight || (weight == best_weight && neighbor < best_neighbor) {
                        best_neighbor = neighbor;
                        best_weight = weight;
                    }
                }
            }
            ColumnIndices::Native(columns) => {
                let (start, end) = self.row_offsets.bounds(row);
                for (&neighbor, &weight) in
                    columns[start..end].iter().zip(&self.weights[start..end])
                {
                    if weight > best_weight || (weight == best_weight && neighbor < best_neighbor) {
                        best_neighbor = neighbor;
                        best_weight = weight;
                    }
                }
            }
        }
        (best_neighbor, best_weight)
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

fn row_offsets_and_cursors(
    mut counts: Vec<usize>,
    directed_entries: usize,
) -> Result<(RowOffsets, Vec<usize>), CmgError> {
    // Once the prefix sum has consumed each degree, the degree buffer has the
    // exact shape needed by the scatter cursors. Rewriting it in place avoids
    // allocating and initializing a second per-row `Vec<usize>`.
    if directed_entries <= u32::MAX as usize {
        let mut offsets = Vec::with_capacity(counts.len() + 1);
        offsets.push(0_u32);
        let mut running = 0_usize;
        for count in &mut counts {
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
        Ok((RowOffsets::Compact(offsets), counts))
    } else {
        let mut offsets = Vec::with_capacity(counts.len() + 1);
        offsets.push(0_usize);
        let mut running = 0_usize;
        for count in &mut counts {
            let width = *count;
            *count = running;
            running = running
                .checked_add(width)
                .ok_or(CmgError::InvalidHierarchy {
                    context: "CSR row offsets overflowed usize",
                })?;
            offsets.push(running);
        }
        Ok((RowOffsets::Native(offsets), counts))
    }
}

#[cfg(feature = "parallel")]
fn native_offsets_and_cursors(
    mut counts: Vec<usize>,
    expected_entries: usize,
) -> Result<(Vec<usize>, Vec<usize>), CmgError> {
    let mut offsets = Vec::with_capacity(counts.len() + 1);
    offsets.push(0);
    let mut running = 0_usize;
    for count in &mut counts {
        let width = *count;
        *count = running;
        running = running
            .checked_add(width)
            .ok_or(CmgError::InvalidHierarchy {
                context: "CSR auxiliary offsets overflowed usize",
            })?;
        offsets.push(running);
    }
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
fn row_fill_blocks<'a, Column>(
    columns: &'a mut [Column],
    weights: &'a mut [f64],
    row_offsets: &RowOffsets,
    rows_per_block: usize,
) -> Vec<RowFillBlock<'a, Column>> {
    let mut blocks = Vec::new();
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
    blocks
}

#[cfg(feature = "parallel")]
#[allow(clippy::too_many_arguments)]
fn fill_compact_rows_parallel(
    graph: &Laplacian,
    row_offsets: &RowOffsets,
    incoming_offsets: &[usize],
    outgoing_offsets: &[usize],
    incoming_edges: &[usize],
    columns: &mut [u32],
    weights: &mut [f64],
    executor: &ParallelExecutor,
) {
    let rows_per_block = executor.work_chunk_len(graph.vertex_count());
    let blocks = row_fill_blocks(columns, weights, row_offsets, rows_per_block);
    executor.install(|| {
        blocks.into_par_iter().for_each(|block| {
            fill_compact_row_block(
                block,
                graph,
                row_offsets,
                incoming_offsets,
                outgoing_offsets,
                incoming_edges,
                rows_per_block,
            );
        });
    });
}

#[cfg(feature = "parallel")]
#[allow(clippy::too_many_arguments)]
fn fill_native_rows_parallel(
    graph: &Laplacian,
    row_offsets: &RowOffsets,
    incoming_offsets: &[usize],
    outgoing_offsets: &[usize],
    incoming_edges: &[usize],
    columns: &mut [usize],
    weights: &mut [f64],
    executor: &ParallelExecutor,
) {
    let rows_per_block = executor.work_chunk_len(graph.vertex_count());
    let blocks = row_fill_blocks(columns, weights, row_offsets, rows_per_block);
    executor.install(|| {
        blocks.into_par_iter().for_each(|block| {
            fill_native_row_block(
                block,
                graph,
                row_offsets,
                incoming_offsets,
                outgoing_offsets,
                incoming_edges,
                rows_per_block,
            );
        });
    });
}

#[cfg(feature = "parallel")]
#[allow(clippy::too_many_arguments)]
fn fill_compact_row_block(
    block: RowFillBlock<'_, u32>,
    graph: &Laplacian,
    row_offsets: &RowOffsets,
    incoming_offsets: &[usize],
    outgoing_offsets: &[usize],
    incoming_edges: &[usize],
    rows_per_block: usize,
) {
    let end_row = block
        .first_row
        .saturating_add(rows_per_block)
        .min(graph.vertex_count());
    for row in block.first_row..end_row {
        let mut position = row_offsets.bounds(row).0 - block.entry_base;
        for &edge_index in &incoming_edges[incoming_offsets[row]..incoming_offsets[row + 1]] {
            let edge = graph.edges()[edge_index];
            block.columns[position] = edge.u() as u32;
            block.weights[position] = edge.weight();
            position += 1;
        }
        for edge in &graph.edges()[outgoing_offsets[row]..outgoing_offsets[row + 1]] {
            debug_assert_eq!(edge.u(), row);
            block.columns[position] = edge.v() as u32;
            block.weights[position] = edge.weight();
            position += 1;
        }
        debug_assert_eq!(position, row_offsets.bounds(row).1 - block.entry_base);
    }
}

#[cfg(feature = "parallel")]
#[allow(clippy::too_many_arguments)]
fn fill_native_row_block(
    block: RowFillBlock<'_, usize>,
    graph: &Laplacian,
    row_offsets: &RowOffsets,
    incoming_offsets: &[usize],
    outgoing_offsets: &[usize],
    incoming_edges: &[usize],
    rows_per_block: usize,
) {
    let end_row = block
        .first_row
        .saturating_add(rows_per_block)
        .min(graph.vertex_count());
    for row in block.first_row..end_row {
        let mut position = row_offsets.bounds(row).0 - block.entry_base;
        for &edge_index in &incoming_edges[incoming_offsets[row]..incoming_offsets[row + 1]] {
            let edge = graph.edges()[edge_index];
            block.columns[position] = edge.u();
            block.weights[position] = edge.weight();
            position += 1;
        }
        for edge in &graph.edges()[outgoing_offsets[row]..outgoing_offsets[row + 1]] {
            debug_assert_eq!(edge.u(), row);
            block.columns[position] = edge.v();
            block.weights[position] = edge.weight();
            position += 1;
        }
        debug_assert_eq!(position, row_offsets.bounds(row).1 - block.entry_base);
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
        csr.matvec_into_parallel(&input, &mut parallel, &executor)
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
            let parallel = CsrLaplacian::from_laplacian_with_executor(&graph, &executor).unwrap();
            assert_eq!(parallel, serial);
            assert_eq!(parallel.byte_len(), serial.byte_len());
        }
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
