//! Deterministic row-oriented Laplacian storage for solve kernels.

#[cfg(feature = "parallel")]
use crate::ParallelExecutor;
use crate::{CmgError, Laplacian};
#[cfg(feature = "parallel")]
use rayon::prelude::*;
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

    #[cfg(feature = "profiling")]
    pub(crate) fn from_laplacian_profiled(
        graph: &Laplacian,
    ) -> Result<(Self, CsrBuildProfile), CmgError> {
        Self::from_laplacian_impl::<true>(graph)
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
        let row_offsets = measure_build_phase::<PROFILE, _>(
            &mut profile.row_offsets_nanoseconds,
            || -> Result<RowOffsets, CmgError> {
                if directed_entries <= u32::MAX as usize {
                    let mut offsets = Vec::with_capacity(vertex_count + 1);
                    offsets.push(0_u32);
                    let mut running = 0_usize;
                    for count in row_counts {
                        running = running
                            .checked_add(count)
                            .ok_or(CmgError::InvalidHierarchy {
                                context: "CSR row offsets overflowed usize",
                            })?;
                        offsets.push(u32::try_from(running).map_err(|_| {
                            CmgError::InvalidHierarchy {
                                context: "CSR compact row offset exceeded u32::MAX",
                            }
                        })?);
                    }
                    Ok(RowOffsets::Compact(offsets))
                } else {
                    let mut offsets = Vec::with_capacity(vertex_count + 1);
                    offsets.push(0_usize);
                    for count in row_counts {
                        let next = offsets
                            .last()
                            .copied()
                            .unwrap_or(0_usize)
                            .checked_add(count)
                            .ok_or(CmgError::InvalidHierarchy {
                                context: "CSR row offsets overflowed usize",
                            })?;
                        offsets.push(next);
                    }
                    Ok(RowOffsets::Native(offsets))
                }
            },
        )?;
        measure_build_phase::<PROFILE, _>(&mut profile.validation_nanoseconds, || {
            if row_offsets.last() != directed_entries {
                Err(CmgError::InvalidHierarchy {
                    context: "CSR row counts do not match directed-entry count",
                })
            } else {
                Ok(())
            }
        })?;

        let (mut next, mut weights) =
            measure_build_phase::<PROFILE, _>(&mut profile.allocation_nanoseconds, || {
                (
                    (0..vertex_count)
                        .map(|row| row_offsets.bounds(row).0)
                        .collect::<Vec<_>>(),
                    vec![0.0; directed_entries],
                )
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
