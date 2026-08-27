//! Canonical weighted graph-Laplacian representation.

use crate::CmgError;
#[cfg(feature = "parallel")]
use crate::{ParallelExecutor, execution::PARALLEL_SETUP_MIN_ITEMS};
#[cfg(feature = "parallel")]
use rayon::prelude::*;
use std::sync::Arc;

/// A canonical undirected weighted edge with `u < v` and positive weight.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Edge {
    key: u64,
    weight: f64,
}

impl Edge {
    /// Return the lower-numbered endpoint.
    #[must_use]
    pub const fn u(self) -> usize {
        (self.key >> 32) as usize
    }

    /// Return the higher-numbered endpoint.
    #[must_use]
    pub const fn v(self) -> usize {
        (self.key as u32) as usize
    }

    /// Return the strictly positive edge weight.
    #[must_use]
    pub const fn weight(self) -> f64 {
        self.weight
    }

    /// Construct an edge from already validated internal graph data.
    pub(crate) fn from_internal_parts(
        left: usize,
        right: usize,
        weight: f64,
    ) -> Result<Self, CmgError> {
        debug_assert!(left != right);
        debug_assert!(weight.is_finite() && weight > 0.0);
        let (u, v) = if left < right {
            (left, right)
        } else {
            (right, left)
        };
        let u = u32::try_from(u).map_err(|_| CmgError::VertexIndexTooWide {
            vertex: u,
            maximum: u32::MAX as usize,
        })?;
        let v = u32::try_from(v).map_err(|_| CmgError::VertexIndexTooWide {
            vertex: v,
            maximum: u32::MAX as usize,
        })?;
        Ok(Self::from_compact_parts(u, v, weight))
    }

    #[inline]
    const fn compact_u(self) -> u32 {
        (self.key >> 32) as u32
    }

    #[inline]
    const fn compact_v(self) -> u32 {
        self.key as u32
    }

    #[inline]
    const fn from_compact_parts(u: u32, v: u32, weight: f64) -> Self {
        Self {
            key: pack_endpoint_key(u, v),
            weight,
        }
    }
}

/// A deterministic edge-list representation of a weighted graph Laplacian.
#[derive(Debug, Clone)]
pub struct Laplacian {
    vertex_count: usize,
    edges: Arc<Vec<Edge>>,
    diagonal: Arc<Vec<f64>>,
    matrix_nnz: usize,
    operator_norm_bound: f64,
    lineage: Arc<()>,
}

impl PartialEq for Laplacian {
    fn eq(&self, other: &Self) -> bool {
        self.vertex_count == other.vertex_count
            && self.edges == other.edges
            && self.diagonal == other.diagonal
            && self.matrix_nnz == other.matrix_nnz
            && self.operator_norm_bound == other.operator_norm_bound
    }
}

impl Laplacian {
    /// Return principal retained heap bytes for canonical edges and the
    /// diagonal. Shared `Arc` control blocks and allocator bookkeeping are not
    /// included.
    #[must_use]
    pub fn retained_bytes(&self) -> usize {
        self.edges
            .capacity()
            .saturating_mul(core::mem::size_of::<Edge>())
            .saturating_add(
                self.diagonal
                    .capacity()
                    .saturating_mul(core::mem::size_of::<f64>()),
            )
    }

    /// Build a Laplacian from undirected positive-weight edges.
    ///
    /// Endpoint order is canonicalized, duplicate edges are sorted by weight
    /// and summed with compensated summation, and the final edge list is sorted
    /// lexicographically by endpoint pair.
    pub fn from_edges<I>(vertex_count: usize, edges: I) -> Result<Self, CmgError>
    where
        I: IntoIterator<Item = (usize, usize, f64)>,
    {
        let mut raw = collect_validated_edges(vertex_count, edges)?;
        raw.sort_unstable_by(compare_raw_edges);
        Self::from_sorted_raw_edges(vertex_count, raw)
    }

    /// Build a Laplacian while parallelizing deterministic edge sorting.
    ///
    /// Validation and compensated duplicate aggregation are identical to
    /// [`Self::from_edges`]. The package-owned executor is used only when the
    /// collected edge count exceeds its configured parallel threshold.
    #[cfg(feature = "parallel")]
    pub fn from_edges_with_executor<I>(
        vertex_count: usize,
        edges: I,
        executor: &ParallelExecutor,
    ) -> Result<Self, CmgError>
    where
        I: IntoIterator<Item = (usize, usize, f64)>,
    {
        let mut raw = collect_validated_edges(vertex_count, edges)?;
        if raw.len() >= PARALLEL_SETUP_MIN_ITEMS && executor.should_parallel(raw.len()) {
            executor.install(|| raw.par_sort_unstable_by(compare_raw_edges));
        } else {
            raw.sort_unstable_by(compare_raw_edges);
        }
        Self::from_sorted_raw_edges(vertex_count, raw)
    }

    pub(crate) fn from_compact_edges(
        vertex_count: usize,
        mut raw: Vec<Edge>,
    ) -> Result<Self, CmgError> {
        sort_compact_edge_endpoints(&mut raw);
        Self::from_endpoint_sorted_raw_edges(vertex_count, raw)
    }

    #[cfg(feature = "parallel")]
    pub(crate) fn from_compact_edges_with_executor(
        vertex_count: usize,
        mut raw: Vec<Edge>,
        executor: &ParallelExecutor,
    ) -> Result<Self, CmgError> {
        if raw.len() >= PARALLEL_SETUP_MIN_ITEMS && executor.should_parallel(raw.len()) {
            executor.install(|| raw.par_sort_unstable_by(compare_raw_edges));
        } else {
            sort_compact_edge_endpoints(&mut raw);
            return Self::from_endpoint_sorted_raw_edges(vertex_count, raw);
        }
        Self::from_sorted_raw_edges(vertex_count, raw)
    }

    fn from_endpoint_sorted_raw_edges(
        vertex_count: usize,
        raw: Vec<Edge>,
    ) -> Result<Self, CmgError> {
        Self::from_sorted_raw_edges_with_mode(vertex_count, raw, false)
    }

    fn from_sorted_raw_edges(vertex_count: usize, raw: Vec<Edge>) -> Result<Self, CmgError> {
        Self::from_sorted_raw_edges_with_mode(vertex_count, raw, true)
    }

    fn from_sorted_raw_edges_with_mode(
        vertex_count: usize,
        mut raw: Vec<Edge>,
        weights_are_sorted: bool,
    ) -> Result<Self, CmgError> {
        // Equal endpoint pairs are contiguous after sorting. Merge them into
        // the front of the compact input buffer so graph construction does not
        // allocate a separate full-capacity canonical vector. Accumulate the
        // diagonal in the same canonical edge order while each merged edge is
        // already hot, avoiding a second full edge pass.
        let mut diagonal = vec![0.0; vertex_count];
        let mut write_index = 0;
        if weights_are_sorted {
            let mut read_index = 0;
            while read_index < raw.len() {
                let key = raw[read_index].key;
                let u = raw[read_index].compact_u();
                let v = raw[read_index].compact_v();
                let mut sum = 0.0;
                let mut correction = 0.0;
                while read_index < raw.len() && raw[read_index].key == key {
                    compensated_add(&mut sum, &mut correction, raw[read_index].weight);
                    read_index += 1;
                }
                write_merged_edge(&mut raw, &mut diagonal, write_index, u, v, sum + correction)?;
                write_index += 1;
            }
        } else {
            let mut group_start = 0;
            while group_start < raw.len() {
                let key = raw[group_start].key;
                let u = raw[group_start].compact_u();
                let v = raw[group_start].compact_v();
                let mut group_end = group_start + 1;
                while group_end < raw.len() && raw[group_end].key == key {
                    group_end += 1;
                }
                if group_end - group_start > 1 {
                    raw[group_start..group_end]
                        .sort_unstable_by(|left, right| left.weight.total_cmp(&right.weight));
                }
                let mut sum = 0.0;
                let mut correction = 0.0;
                for edge in &raw[group_start..group_end] {
                    compensated_add(&mut sum, &mut correction, edge.weight);
                }
                write_merged_edge(&mut raw, &mut diagonal, write_index, u, v, sum + correction)?;
                write_index += 1;
                group_start = group_end;
            }
        }
        raw.truncate(write_index);
        // Filtered coarse-edge iterators have a zero lower size hint, so their
        // vectors may grow beyond the final length. Do not retain that spare
        // capacity in every hierarchy level.
        if raw.capacity() != raw.len() {
            raw.shrink_to_fit();
        }

        let mut diagonal_nnz = 0_usize;
        let mut maximum_degree = 0.0_f64;
        for &degree in &diagonal {
            diagonal_nnz += usize::from(degree != 0.0);
            maximum_degree = maximum_degree.max(degree);
        }
        let matrix_nnz = diagonal_nnz + 2 * raw.len();
        let operator_norm_bound = 2.0 * maximum_degree;

        Ok(Self {
            vertex_count,
            edges: Arc::new(raw),
            diagonal: Arc::new(diagonal),
            matrix_nnz,
            operator_norm_bound,
            lineage: Arc::new(()),
        })
    }

    /// Return the number of vertices, including isolated vertices.
    #[must_use]
    pub const fn vertex_count(&self) -> usize {
        self.vertex_count
    }

    /// Return the number of canonical undirected edges.
    #[must_use]
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Return the canonical edge slice.
    #[must_use]
    pub fn edges(&self) -> &[Edge] {
        self.edges.as_slice()
    }

    /// Return the weighted degree (Laplacian diagonal) of every vertex.
    #[must_use]
    pub fn diagonal(&self) -> &[f64] {
        self.diagonal.as_slice()
    }

    /// Return the number of nonzero entries in the corresponding symmetric
    /// sparse matrix.
    #[must_use]
    pub const fn matrix_nnz(&self) -> usize {
        self.matrix_nnz
    }

    /// Return an inexpensive upper bound on the Euclidean operator norm.
    #[must_use]
    pub const fn operator_norm_bound(&self) -> f64 {
        self.operator_norm_bound
    }

    pub(crate) fn shares_lineage(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.lineage, &other.lineage)
    }

    #[cfg(feature = "parallel")]
    pub(crate) const fn lineage(&self) -> &Arc<()> {
        &self.lineage
    }

    /// Compute `output = L * input` without allocating.
    pub fn matvec_into(&self, input: &[f64], output: &mut [f64]) -> Result<(), CmgError> {
        if input.len() != self.vertex_count {
            return Err(CmgError::dimension(
                "Laplacian::matvec input",
                self.vertex_count,
                input.len(),
            ));
        }
        if output.len() != self.vertex_count {
            return Err(CmgError::dimension(
                "Laplacian::matvec output",
                self.vertex_count,
                output.len(),
            ));
        }
        output.fill(0.0);
        for edge in self.edges.iter() {
            let difference = edge.weight * (input[edge.u()] - input[edge.v()]);
            output[edge.u()] += difference;
            output[edge.v()] -= difference;
        }
        Ok(())
    }

    /// Compute and return `L * input`.
    pub fn matvec(&self, input: &[f64]) -> Result<Vec<f64>, CmgError> {
        let mut output = vec![0.0; self.vertex_count];
        self.matvec_into(input, &mut output)?;
        Ok(output)
    }

    /// Compute the Laplacian energy `input^T L input` from the edge identity.
    pub fn energy(&self, input: &[f64]) -> Result<f64, CmgError> {
        if input.len() != self.vertex_count {
            return Err(CmgError::dimension(
                "Laplacian::energy",
                self.vertex_count,
                input.len(),
            ));
        }
        Ok(compensated_sum(self.edges.iter().map(|edge| {
            let difference = input[edge.u()] - input[edge.v()];
            edge.weight * difference * difference
        })))
    }

    /// Materialize the Laplacian as a row-major dense matrix.
    ///
    /// This is intended for tests, diagnostics, and genuinely small systems.
    #[must_use]
    pub fn to_dense(&self) -> Vec<Vec<f64>> {
        let mut dense = vec![vec![0.0; self.vertex_count]; self.vertex_count];
        for edge in self.edges.iter() {
            dense[edge.u()][edge.u()] += edge.weight;
            dense[edge.v()][edge.v()] += edge.weight;
            dense[edge.u()][edge.v()] -= edge.weight;
            dense[edge.v()][edge.u()] -= edge.weight;
        }
        dense
    }
}

fn collect_validated_edges<I>(vertex_count: usize, edges: I) -> Result<Vec<Edge>, CmgError>
where
    I: IntoIterator<Item = (usize, usize, f64)>,
{
    let iterator = edges.into_iter();
    let mut raw = Vec::with_capacity(iterator.size_hint().0);
    for (left, right, weight) in iterator {
        if left >= vertex_count {
            return Err(CmgError::VertexOutOfBounds {
                vertex: left,
                vertex_count,
            });
        }
        if right >= vertex_count {
            return Err(CmgError::VertexOutOfBounds {
                vertex: right,
                vertex_count,
            });
        }
        if left > u32::MAX as usize {
            return Err(CmgError::VertexIndexTooWide {
                vertex: left,
                maximum: u32::MAX as usize,
            });
        }
        if right > u32::MAX as usize {
            return Err(CmgError::VertexIndexTooWide {
                vertex: right,
                maximum: u32::MAX as usize,
            });
        }
        if left == right {
            return Err(CmgError::SelfLoop { vertex: left });
        }
        if !weight.is_finite() || weight <= 0.0 {
            return Err(CmgError::InvalidEdgeWeight {
                u: left,
                v: right,
                weight,
            });
        }
        let (u, v) = if left < right {
            (left, right)
        } else {
            (right, left)
        };
        raw.push(Edge::from_compact_parts(u as u32, v as u32, weight));
    }
    Ok(raw)
}

const fn pack_endpoint_key(u: u32, v: u32) -> u64 {
    ((u as u64) << 32) | v as u64
}

#[inline]
const fn endpoint_key(edge: &Edge) -> u64 {
    edge.key
}

fn compare_raw_edges(left: &Edge, right: &Edge) -> core::cmp::Ordering {
    endpoint_key(left)
        .cmp(&endpoint_key(right))
        .then_with(|| left.weight.total_cmp(&right.weight))
}

fn sort_compact_edge_endpoints(raw: &mut [Edge]) {
    raw.sort_unstable_by_key(endpoint_key);
}

#[cfg(test)]
fn sort_compact_edges_two_stage(raw: &mut [Edge]) {
    sort_compact_edge_endpoints(raw);
    let mut start = 0;
    while start < raw.len() {
        let key = endpoint_key(&raw[start]);
        let mut end = start + 1;
        while end < raw.len() && endpoint_key(&raw[end]) == key {
            end += 1;
        }
        if end - start > 1 {
            raw[start..end].sort_unstable_by(|left, right| left.weight.total_cmp(&right.weight));
        }
        start = end;
    }
}

fn write_merged_edge(
    raw: &mut [Edge],
    diagonal: &mut [f64],
    write_index: usize,
    u: u32,
    v: u32,
    weight: f64,
) -> Result<(), CmgError> {
    if !weight.is_finite() || weight <= 0.0 {
        return Err(CmgError::InvalidEdgeWeight {
            u: u as usize,
            v: v as usize,
            weight,
        });
    }
    raw[write_index] = Edge::from_compact_parts(u, v, weight);
    diagonal[u as usize] += weight;
    diagonal[v as usize] += weight;
    Ok(())
}

#[inline]
fn compensated_add(sum: &mut f64, correction: &mut f64, value: f64) {
    let next = *sum + value;
    *correction += if sum.abs() >= value.abs() {
        (*sum - next) + value
    } else {
        (value - next) + *sum
    };
    *sum = next;
}

pub(crate) fn compensated_sum<I>(values: I) -> f64
where
    I: IntoIterator<Item = f64>,
{
    let mut sum = 0.0;
    let mut correction = 0.0;
    for value in values {
        compensated_add(&mut sum, &mut correction, value);
    }
    sum + correction
}

pub(crate) fn close(left: f64, right: f64, tolerance: f64) -> bool {
    let scale = 1.0_f64.max(left.abs()).max(right.abs());
    (left - right).abs() <= tolerance * scale
}

#[cfg(test)]
mod tests {
    use super::Laplacian;

    #[test]
    fn clones_share_lineage_but_independent_equal_graphs_do_not() {
        let graph = Laplacian::from_edges(3, [(0, 1, 1.0), (1, 2, 2.0)]).unwrap();
        let clone = graph.clone();
        let rebuilt = Laplacian::from_edges(3, [(0, 1, 1.0), (1, 2, 2.0)]).unwrap();

        assert!(graph.shares_lineage(&clone));
        assert!(!graph.shares_lineage(&rebuilt));
        assert_eq!(graph, rebuilt);
        assert_eq!(graph.matrix_nnz(), 7);
        assert_eq!(graph.operator_norm_bound(), 6.0);
    }

    #[test]
    fn cached_invariants_include_isolated_vertices_correctly() {
        let graph = Laplacian::from_edges(4, [(0, 1, 2.5)]).unwrap();
        assert_eq!(graph.matrix_nnz(), 4);
        assert_eq!(graph.operator_norm_bound(), 5.0);
    }
}

#[cfg(test)]
mod compact_edge_layout_tests {
    use super::{Edge, Laplacian};
    use crate::CmgError;

    #[test]
    fn edge_uses_compact_endpoints() {
        assert_eq!(std::mem::size_of::<Edge>(), 16);
        assert_eq!(std::mem::align_of::<Edge>(), 8);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn endpoint_above_u32_is_rejected_before_graph_allocation() {
        let vertex = u32::MAX as usize + 1;
        let error = Laplacian::from_edges(vertex + 1, [(0, vertex, 1.0)]).unwrap_err();
        assert_eq!(
            error,
            CmgError::VertexIndexTooWide {
                vertex,
                maximum: u32::MAX as usize,
            }
        );
    }
}

#[cfg(test)]
mod two_stage_sort_equivalence_tests {
    use super::{Edge, compare_raw_edges, sort_compact_edges_two_stage};

    #[test]
    fn endpoint_then_weight_order_matches_total_comparator() {
        let mut candidate = vec![
            Edge::from_compact_parts(4, 9, 2.0),
            Edge::from_compact_parts(1, 7, 3.0),
            Edge::from_compact_parts(4, 9, 1.0),
            Edge::from_compact_parts(1, 7, 1.0),
            Edge::from_compact_parts(2, 8, 4.0),
            Edge::from_compact_parts(1, 7, 2.0),
            Edge::from_compact_parts(2, 8, 0.5),
        ];
        let mut reference = candidate.clone();
        reference.sort_unstable_by(compare_raw_edges);
        sort_compact_edges_two_stage(&mut candidate);
        assert_eq!(candidate, reference);
    }
}

#[cfg(test)]
mod one_pass_merge_arithmetic_tests {
    use super::{compensated_add, compensated_sum};

    #[test]
    fn incremental_compensation_matches_iterator_helper_bitwise() {
        let values = [1.0e100, 1.0, 2.0, 3.0, 1.0e-100, 7.0, 9.0];
        let expected = compensated_sum(values);
        let mut sum = 0.0;
        let mut correction = 0.0;
        for value in values {
            compensated_add(&mut sum, &mut correction, value);
        }
        assert_eq!((sum + correction).to_bits(), expected.to_bits());
    }
}

#[cfg(test)]
mod fused_merge_diagonal_tests {
    use super::Laplacian;

    #[test]
    fn fused_diagonal_matches_canonical_edge_scan_bitwise() {
        let graph = Laplacian::from_edges(
            6,
            [
                (4, 1, 0.25),
                (0, 3, 7.0),
                (1, 4, 1.5),
                (2, 5, 3.0),
                (3, 0, 0.125),
                (4, 1, 2.25),
                (5, 2, 0.75),
            ],
        )
        .unwrap();
        let mut scanned = vec![0.0_f64; graph.vertex_count()];
        for edge in graph.edges() {
            scanned[edge.u()] += edge.weight();
            scanned[edge.v()] += edge.weight();
        }
        assert_eq!(
            graph
                .diagonal()
                .iter()
                .map(|value| value.to_bits())
                .collect::<Vec<_>>(),
            scanned
                .iter()
                .map(|value| value.to_bits())
                .collect::<Vec<_>>()
        );
    }
}

#[cfg(test)]
mod fused_diagonal_statistics_tests {
    use super::Laplacian;

    #[test]
    fn one_pass_diagonal_statistics_match_expected_values() {
        let graph = Laplacian::from_edges(4, [(0, 1, 2.0), (1, 2, 3.0)]).unwrap();
        assert_eq!(graph.matrix_nnz(), 7);
        assert_eq!(graph.operator_norm_bound().to_bits(), 10.0_f64.to_bits());
    }
}

#[cfg(test)]
mod local_duplicate_merge_tests {
    use super::{Laplacian, sort_compact_edge_endpoints};

    #[test]
    fn endpoint_only_compact_path_matches_public_total_order_path() {
        let edges = vec![
            (4, 1, 4.0),
            (1, 4, 0.25),
            (3, 0, 8.0),
            (4, 1, 2.0),
            (0, 3, 0.5),
            (2, 5, 1.0),
        ];
        let public = Laplacian::from_edges(6, edges.clone()).unwrap();
        let mut compact = edges
            .into_iter()
            .map(|(u, v, weight)| super::Edge::from_internal_parts(u, v, weight).unwrap())
            .collect::<Vec<_>>();
        sort_compact_edge_endpoints(&mut compact);
        let local = Laplacian::from_endpoint_sorted_raw_edges(6, compact).unwrap();
        assert_eq!(local, public);
    }
}

#[cfg(test)]
mod shared_laplacian_storage_tests {
    use super::Laplacian;
    use std::sync::Arc;

    #[test]
    fn clones_share_immutable_edge_and_diagonal_storage() {
        let graph =
            Laplacian::from_edges(5, [(0, 1, 1.0), (1, 2, 2.0), (2, 3, 3.0), (3, 4, 4.0)]).unwrap();
        let clone = graph.clone();
        assert!(Arc::ptr_eq(&graph.edges, &clone.edges));
        assert!(Arc::ptr_eq(&graph.diagonal, &clone.diagonal));
        assert!(graph.shares_lineage(&clone));
        assert_eq!(graph, clone);
    }

    #[test]
    fn independently_built_equal_graphs_do_not_share_storage() {
        let left = Laplacian::from_edges(3, [(0, 1, 1.0), (1, 2, 2.0)]).unwrap();
        let right = Laplacian::from_edges(3, [(0, 1, 1.0), (1, 2, 2.0)]).unwrap();
        assert_eq!(left, right);
        assert!(!Arc::ptr_eq(&left.edges, &right.edges));
        assert!(!Arc::ptr_eq(&left.diagonal, &right.diagonal));
    }
}

#[cfg(test)]
mod cached_endpoint_key_tests {
    use super::{Edge, endpoint_key};

    #[test]
    fn cached_key_preserves_layout_and_endpoint_access() {
        let edge = Edge::from_compact_parts(17, 91, 2.5);
        assert_eq!(std::mem::size_of::<Edge>(), 16);
        assert_eq!(edge.u(), 17);
        assert_eq!(edge.v(), 91);
        assert_eq!(endpoint_key(&edge), (17_u64 << 32) | 91_u64);
        assert_eq!(edge.weight().to_bits(), 2.5_f64.to_bits());
    }
}
