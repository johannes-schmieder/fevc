//! Canonical weighted graph-Laplacian representation.

use crate::cancel::{CancelMeter, Meter, Unmetered};
#[cfg(feature = "parallel")]
use crate::execution::PARALLEL_SETUP_MIN_ITEMS;
use crate::{CmgError, Components};
use std::sync::Arc;

#[cfg(feature = "parallel")]
mod hosted;
#[cfg(feature = "parallel")]
pub(crate) use hosted::stage;
mod edge_sort;

crate::storage::storage_fields!(
    Laplacian,
    edges,
    diagonal,
    lineage,
    prepared_topology_lineage,
    prepared_components
);

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
#[derive(Clone)]
pub struct Laplacian {
    vertex_count: usize,
    edges: Arc<Vec<Edge>>,
    diagonal: Arc<Vec<f64>>,
    matrix_nnz: usize,
    operator_norm_bound: f64,
    lineage: Arc<()>,
    prepared_topology_lineage: Option<Arc<()>>,
    prepared_components: Option<Arc<Components>>,
}

impl core::fmt::Debug for Laplacian {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("Laplacian")
            .field("vertex_count", &self.vertex_count)
            .field("edges", &self.edges)
            .field("diagonal", &self.diagonal)
            .field("matrix_nnz", &self.matrix_nnz)
            .field("operator_norm_bound", &self.operator_norm_bound)
            .field("lineage", &self.lineage)
            .finish()
    }
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
    /// lexicographically by endpoint pair. This is the serial convenience form
    /// of [`Self::from_edges_with`].
    pub fn from_edges<I>(vertex_count: usize, edges: I) -> Result<Self, CmgError>
    where
        I: IntoIterator<Item = (usize, usize, f64)>,
    {
        let raw = edges
            .into_iter()
            .map(|edge| map_edge(vertex_count, edge))
            .collect::<Result<Vec<_>, _>>()?;
        Self::canonical(vertex_count, raw, &mut Unmetered)
    }

    /// Requested layouts admitted by [`Self::build`] and
    /// [`Self::from_edges_with`]: two compact edge banks, the diagonal and three
    /// Arc owners, including modeled control blocks. Borrowed input and
    /// allocator overhead are excluded. Sorting is in place on every path; the
    /// second bank covers the transient compaction copy that
    /// `from_sorted_edges` allocates while the merged bank is still live.
    pub fn construction_allocation_bound(vertices: usize, edges: usize) -> Result<usize, CmgError> {
        use crate::storage::{add, arc_bytes, bytes};
        let edge_bytes = bytes::<Edge>(edges)?;
        let mut required = add(edge_bytes, edge_bytes)?;
        required = add(required, bytes::<f64>(vertices)?)?;
        required = add(required, arc_bytes::<Vec<Edge>>()?)?;
        required = add(required, arc_bytes::<Vec<f64>>()?)?;
        add(required, arc_bytes::<()>()?)
    }

    /// Build a Laplacian from a borrowed edge slice under a construction context.
    ///
    /// Canonical ordering and arithmetic match [`Self::from_edges`]. A budget is
    /// admitted against [`Self::construction_allocation_bound`] before the first
    /// allocation. With a cancellation query every bounded operation is counted,
    /// the query is polled at the fixed cadence and before every allocation, and
    /// a true query returns [`CmgError::Cancelled`] with no graph. With a team,
    /// edge sets of at least 65,536 records are validated and mapped in fixed
    /// logical chunks (cancellation polled before every chunk) and sorted in
    /// place on the team (polled inside every partition); duplicate reduction
    /// and the diagonal keep the serial canonical order.
    pub fn build(
        vertex_count: usize,
        edges: &[(usize, usize, f64)],
        context: crate::BuildContext<'_>,
    ) -> Result<Self, CmgError> {
        if let Some(budget) = context.budget_bytes {
            crate::storage::admit(
                Self::construction_allocation_bound(vertex_count, edges.len())?,
                budget,
            )?;
        }
        #[cfg(feature = "parallel")]
        if let Some(team) = context.team.filter(|_| edges.len() >= hosted::MIN_EDGES) {
            return match context.cancel {
                Some(cancel) => {
                    hosted::build(vertex_count, edges, team, &mut CancelMeter::new(cancel))
                }
                None => hosted::build(vertex_count, edges, team, &mut Unmetered),
            };
        }
        match context.cancel {
            Some(cancel) => Self::build_serial(vertex_count, edges, &mut CancelMeter::new(cancel)),
            None => Self::build_serial(vertex_count, edges, &mut Unmetered),
        }
    }

    /// Build a Laplacian from an exact-size edge iterator under a context.
    ///
    /// Validation and canonical arithmetic match [`Self::from_edges`]. With a
    /// team, edge sets of at least 65,536 records sort in place on the team's
    /// pool when its policy enables parallel work; the iterator is consumed
    /// serially. Budget and cancellation follow [`Self::build`].
    pub fn from_edges_with<I>(
        vertex_count: usize,
        edges: I,
        context: crate::BuildContext<'_>,
    ) -> Result<Self, CmgError>
    where
        I: IntoIterator<Item = (usize, usize, f64)>,
        I::IntoIter: ExactSizeIterator,
    {
        let edges = edges.into_iter();
        if let Some(budget) = context.budget_bytes {
            crate::storage::admit(
                Self::construction_allocation_bound(vertex_count, edges.len())?,
                budget,
            )?;
        }
        match context.cancel {
            Some(cancel) => {
                Self::from_iter_with(vertex_count, edges, context, &mut CancelMeter::new(cancel))
            }
            None => Self::from_iter_with(vertex_count, edges, context, &mut Unmetered),
        }
    }

    fn from_iter_with<M: Meter>(
        vertex_count: usize,
        edges: impl ExactSizeIterator<Item = (usize, usize, f64)>,
        context: crate::BuildContext<'_>,
        meter: &mut M,
    ) -> Result<Self, CmgError> {
        let raw = map_edges(vertex_count, edges, meter)?;
        #[cfg(feature = "parallel")]
        if let Some(team) = context
            .team
            .filter(|team| raw.len() >= PARALLEL_SETUP_MIN_ITEMS && team.should_parallel(raw.len()))
        {
            return Self::canonical_parallel(vertex_count, raw, team, meter);
        }
        let _ = context;
        Self::canonical(vertex_count, raw, meter)
    }

    fn build_serial<M: Meter>(
        vertex_count: usize,
        edges: &[(usize, usize, f64)],
        meter: &mut M,
    ) -> Result<Self, CmgError> {
        // The admission formula is fixed-work checked scalar arithmetic.
        meter.work("input graph memory bound", 128)?;
        let raw = map_edges(vertex_count, edges.iter().copied(), meter)?;
        Self::canonical(vertex_count, raw, meter)
    }

    /// Canonicalize owned compact edges: total-order sort, then compensated
    /// duplicate reduction with the diagonal in canonical edge order. A
    /// polling meter sorts with the counted in-place introsort (polled inside
    /// every partition); the ordinary path uses the standard endpoint sort.
    pub(crate) fn canonical<M: Meter>(
        vertex_count: usize,
        mut raw: Vec<Edge>,
        meter: &mut M,
    ) -> Result<Self, CmgError> {
        if M::POLLS {
            // Sorting by the full comparator gives precisely the order of the
            // endpoint sort followed by each duplicate group's weight sort.
            edge_sort::sort_serial(&mut raw, meter, "coarse edge sorting")?;
            Self::from_sorted_edges(vertex_count, raw, true, meter)
        } else {
            sort_compact_edge_endpoints(&mut raw);
            Self::from_sorted_edges(vertex_count, raw, false, meter)
        }
    }

    /// Canonicalize with the team's in-place bounded introsort: polled inside
    /// every partition, capped at the worker ceiling and allocation-free. The
    /// total comparator makes the order identical to the serial sort.
    #[cfg(feature = "parallel")]
    pub(crate) fn canonical_parallel<M: Meter>(
        vertex_count: usize,
        mut raw: Vec<Edge>,
        team: crate::Team<'_>,
        meter: &mut M,
    ) -> Result<Self, CmgError> {
        edge_sort::sort(&mut raw, team, meter)?;
        Self::from_sorted_edges(vertex_count, raw, true, meter)
    }

    /// Merge endpoint-sorted edges into the front of their own buffer and
    /// accumulate the diagonal while each merged edge is hot. Unless weights
    /// are already sorted within each endpoint group, each group is sorted by
    /// weight before its compensated sum.
    pub(super) fn from_sorted_edges<M: Meter>(
        vertex_count: usize,
        mut raw: Vec<Edge>,
        weights_are_sorted: bool,
        meter: &mut M,
    ) -> Result<Self, CmgError> {
        let mut diagonal = if M::POLLS {
            meter.allocation(
                "coarse graph diagonal",
                crate::storage::bytes::<f64>(vertex_count)?,
            )?;
            let mut diagonal = crate::storage::try_capacity(vertex_count, "coarse graph diagonal")?;
            for _ in 0..vertex_count {
                meter.work("coarse graph diagonal initialization", 8)?;
                diagonal.push(0.0);
            }
            diagonal
        } else {
            crate::storage::try_filled(vertex_count, 0.0, "coarse graph diagonal")?
        };
        let mut read_index = 0;
        let mut write_index = 0;
        while read_index < raw.len() {
            meter.work("coarse graph merge group", 32)?;
            let key = raw[read_index].key;
            let u = raw[read_index].compact_u();
            let v = raw[read_index].compact_v();
            if !weights_are_sorted {
                let mut group_end = read_index + 1;
                while group_end < raw.len() && raw[group_end].key == key {
                    group_end += 1;
                }
                if group_end - read_index > 1 {
                    raw[read_index..group_end]
                        .sort_unstable_by(|left, right| left.weight.total_cmp(&right.weight));
                }
            }
            let mut sum = 0.0;
            let mut correction = 0.0;
            while read_index < raw.len() && raw[read_index].key == key {
                meter.work("coarse graph compensated merge", 64)?;
                compensated_add(&mut sum, &mut correction, raw[read_index].weight);
                read_index += 1;
            }
            meter.work("coarse graph merged edge", 64)?;
            write_merged_edge(&mut raw, &mut diagonal, write_index, u, v, sum + correction)?;
            write_index += 1;
        }
        raw.truncate(write_index);
        // Filtered and merged banks may retain spare capacity. Do not keep it
        // in every hierarchy level.
        if raw.capacity() != raw.len() {
            meter.allocation(
                "coarse edge compaction",
                crate::storage::bytes::<Edge>(raw.len())?,
            )?;
            let mut compact = crate::storage::try_capacity(raw.len(), "coarse edge compaction")?;
            if M::POLLS {
                for &edge in &raw {
                    meter.work("coarse edge compaction copy", 8)?;
                    compact.push(edge);
                }
            } else {
                compact.extend_from_slice(&raw);
            }
            raw = compact;
        }
        let mut diagonal_nnz = 0;
        let mut maximum_degree = 0.0_f64;
        for &degree in &diagonal {
            meter.work("coarse graph invariants", 16)?;
            diagonal_nnz += usize::from(degree != 0.0);
            maximum_degree = maximum_degree.max(degree);
        }
        meter.work("coarse graph metadata", 32)?;
        let matrix_nnz = diagonal_nnz + 2 * raw.len();
        let operator_norm_bound = 2.0 * maximum_degree;
        meter.allocation(
            "coarse graph edge owner",
            crate::storage::arc_bytes::<Vec<Edge>>()?,
        )?;
        let edges = Arc::new(raw);
        meter.allocation(
            "coarse graph diagonal owner",
            crate::storage::arc_bytes::<Vec<f64>>()?,
        )?;
        let diagonal = Arc::new(diagonal);
        meter.allocation("coarse graph lineage", crate::storage::arc_bytes::<()>()?)?;
        Ok(Self {
            vertex_count,
            edges,
            diagonal,
            matrix_nnz,
            operator_norm_bound,
            lineage: Arc::new(()),
            prepared_topology_lineage: None,
            prepared_components: None,
        })
    }

    /// Publish a prepared numeric frame. The diagonal metadata scan is counted
    /// in segments and the shared handles are allocated after a poll; the
    /// caller polls once more before returning the graph.
    pub(crate) fn from_prepared_parts<M: crate::cancel::Meter>(
        vertex_count: usize,
        edges: Vec<Edge>,
        diagonal: Vec<f64>,
        prepared_topology_lineage: Arc<()>,
        prepared_components: Arc<Components>,
        meter: &mut M,
    ) -> Result<Self, CmgError> {
        debug_assert_eq!(diagonal.len(), vertex_count);
        let mut diagonal_nnz = 0_usize;
        let mut maximum_degree = 0.0_f64;
        crate::cancel::chunked(
            meter,
            "prepared numeric metadata",
            diagonal.len(),
            2,
            |range| {
                for &degree in &diagonal[range] {
                    diagonal_nnz += usize::from(degree != 0.0);
                    maximum_degree = maximum_degree.max(degree);
                }
                Ok(())
            },
        )?;
        let matrix_nnz = diagonal_nnz + 2 * edges.len();
        let operator_norm_bound = 2.0 * maximum_degree;
        meter.allocation(
            "prepared numeric publication",
            crate::storage::arc_bytes::<Vec<Edge>>()?,
        )?;
        let edges = Arc::new(edges);
        meter.allocation(
            "prepared diagonal publication",
            crate::storage::arc_bytes::<Vec<f64>>()?,
        )?;
        let diagonal = Arc::new(diagonal);
        meter.allocation(
            "prepared numeric lineage",
            crate::storage::arc_bytes::<()>()?,
        )?;
        let lineage = Arc::new(());
        Ok(Self {
            vertex_count,
            edges,
            diagonal,
            matrix_nnz,
            operator_norm_bound,
            lineage,
            prepared_topology_lineage: Some(prepared_topology_lineage),
            prepared_components: Some(prepared_components),
        })
    }

    /// Remove only zero-degree rows, preserving canonical edge and addition order.
    ///
    /// A monotone relabeling cannot change canonical ordering or introduce
    /// duplicates. If all removed rows form a suffix, even the endpoint indices
    /// are unchanged and the canonical edge storage is shared.
    #[cfg(feature = "experimental-components")]
    pub(crate) fn without_isolated_vertices<M: Meter>(
        &self,
        meter: &mut M,
    ) -> Result<Option<(Vec<usize>, Self)>, CmgError> {
        let mut has_isolate = false;
        for &degree in self.diagonal.iter() {
            meter.work("isolate detection", 8)?;
            if degree.partial_cmp(&0.0) != Some(core::cmp::Ordering::Greater) {
                has_isolate = true;
                break;
            }
        }
        if !has_isolate {
            return Ok(None);
        }
        meter.allocation(
            "isolate vertex map",
            crate::storage::bytes::<usize>(self.vertex_count)?,
        )?;
        let mut map = crate::storage::try_capacity(self.vertex_count, "isolate vertex map")?;
        let mut dimension = 0;
        let mut endpoints_unchanged = true;
        for (vertex, &degree) in self.diagonal.iter().enumerate() {
            meter.work("isolate vertex mapping", 32)?;
            if degree > 0.0 {
                map.push(dimension);
                endpoints_unchanged &= vertex == dimension;
                dimension += 1;
            } else {
                map.push(usize::MAX);
            }
        }
        let edges = if endpoints_unchanged {
            Arc::clone(&self.edges)
        } else {
            meter.allocation(
                "isolate compact edges",
                crate::storage::bytes::<Edge>(self.edge_count())?,
            )?;
            let mut edges =
                crate::storage::try_capacity(self.edge_count(), "isolate compact edges")?;
            for &edge in self.edges.iter() {
                meter.work("isolate edge mapping", 32)?;
                debug_assert!(map[edge.u()] < map[edge.v()]);
                edges.push(Edge::from_compact_parts(
                    map[edge.u()] as u32,
                    map[edge.v()] as u32,
                    edge.weight(),
                ));
            }
            meter.allocation(
                "isolate edge owner",
                crate::storage::arc_bytes::<Vec<Edge>>()?,
            )?;
            Arc::new(edges)
        };
        meter.allocation(
            "isolate compact diagonal",
            crate::storage::bytes::<f64>(dimension)?,
        )?;
        let mut diagonal = crate::storage::try_capacity(dimension, "isolate compact diagonal")?;
        for &degree in self.diagonal.iter() {
            meter.work("isolate diagonal copy", 16)?;
            if degree > 0.0 {
                diagonal.push(degree);
            }
        }
        meter.allocation(
            "isolate diagonal owner",
            crate::storage::arc_bytes::<Vec<f64>>()?,
        )?;
        let diagonal = Arc::new(diagonal);
        meter.allocation("isolate graph lineage", crate::storage::arc_bytes::<()>()?)?;
        Ok(Some((
            map,
            Self {
                vertex_count: dimension,
                edges,
                diagonal,
                matrix_nnz: self.matrix_nnz,
                operator_norm_bound: self.operator_norm_bound,
                lineage: Arc::new(()),
                prepared_topology_lineage: None,
                prepared_components: None,
            },
        )))
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

    pub(crate) fn shares_prepared_topology(&self, other: &Self) -> bool {
        matches!(
            (&self.prepared_topology_lineage, &other.prepared_topology_lineage),
            (Some(left), Some(right)) if Arc::ptr_eq(left, right)
        ) && matches!(
            (&self.prepared_components, &other.prepared_components),
            (Some(left), Some(right)) if Arc::ptr_eq(left, right)
        )
    }

    pub(crate) fn prepared_components(&self) -> Option<&Arc<Components>> {
        self.prepared_components.as_ref()
    }

    pub(crate) fn belongs_to_prepared_topology(
        &self,
        lineage: &Arc<()>,
        components: &Arc<Components>,
    ) -> bool {
        self.prepared_topology_lineage
            .as_ref()
            .is_some_and(|candidate| Arc::ptr_eq(candidate, lineage))
            && self
                .prepared_components
                .as_ref()
                .is_some_and(|candidate| Arc::ptr_eq(candidate, components))
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
        self.matvec_lanes(
            input,
            output,
            crate::preconditioner::One,
            &mut crate::cancel::Unmetered,
        )
    }

    /// The one edge-list (scatter) kernel for vertex-major lanes: every lane
    /// accumulates edge differences in canonical edge order from zero. Edges
    /// are counted in bounded chunks when the meter polls.
    pub(crate) fn matvec_lanes<W: crate::preconditioner::Width, M: crate::cancel::Meter>(
        &self,
        input: &[f64],
        output: &mut [f64],
        width: W,
        meter: &mut M,
    ) -> Result<(), CmgError> {
        let columns = width.get();
        if input.len() != output.len() {
            return Err(CmgError::dimension(
                "Laplacian matvec lanes",
                output.len(),
                input.len(),
            ));
        }
        // One length for both slices lets each slot's bounds check cover the
        // input and the output, as in the earlier scalar kernel.
        let input = &input[..output.len()];
        crate::cancel::zero(meter, "Laplacian matvec", output)?;
        // Each output slot still receives its edges in canonical order; wide
        // batches only split an edge's lanes into counted tiles.
        crate::cancel::row_lanes(
            meter,
            "Laplacian matvec",
            self.edges.len(),
            columns,
            1,
            #[inline(always)]
            |edges, lanes| {
                for edge in &self.edges[edges] {
                    let u = edge.u() * columns;
                    let v = edge.v() * columns;
                    for lane in lanes.clone() {
                        let difference = edge.weight * (input[u + lane] - input[v + lane]);
                        output[u + lane] += difference;
                        output[v + lane] -= difference;
                    }
                }
                Ok(())
            },
        )
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

/// Validate one input edge and map it to compact canonical endpoints.
pub(crate) fn map_edge(
    vertex_count: usize,
    (left, right, weight): (usize, usize, f64),
) -> Result<Edge, CmgError> {
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
    Ok(Edge::from_compact_parts(u as u32, v as u32, weight))
}

fn map_edges<M: Meter>(
    vertex_count: usize,
    edges: impl ExactSizeIterator<Item = (usize, usize, f64)>,
    meter: &mut M,
) -> Result<Vec<Edge>, CmgError> {
    let mut raw = meter.try_capacity(edges.len(), "input compact edges")?;
    for edge in edges {
        meter.work("input edge validation and mapping", 128)?;
        raw.push(map_edge(vertex_count, edge)?);
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
pub(crate) fn compensated_add(sum: &mut f64, correction: &mut f64, value: f64) {
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

    #[cfg(feature = "experimental-components")]
    #[test]
    fn compact_isolates_preserves_canonical_values_and_drops_lineage() {
        for edges in [
            vec![],
            vec![(0, 1, 0.25), (1, 2, 4.0)],
            vec![(2, 5, 0.125), (5, 11, 1e3), (2, 11, 1e-3), (2, 5, 0.5)],
        ] {
            let graph = Laplacian::from_edges(12, edges).unwrap();
            let (map, compact) = graph
                .without_isolated_vertices(&mut crate::cancel::Unmetered)
                .unwrap()
                .unwrap();
            let reference = Laplacian::from_edges(
                compact.vertex_count(),
                graph
                    .edges()
                    .iter()
                    .map(|e| (map[e.u()], map[e.v()], e.weight())),
            )
            .unwrap();
            assert_eq!(compact, reference);
            assert_eq!(compact.matrix_nnz(), graph.matrix_nnz());
            assert_eq!(compact.operator_norm_bound(), graph.operator_norm_bound());
            assert!(!compact.shares_lineage(&graph));
            assert!(compact.prepared_topology_lineage.is_none());
            let vector: Vec<_> = (0..12).map(|i| i as f64 / 7.0).collect();
            let active: Vec<_> = vector
                .iter()
                .zip(&map)
                .filter_map(|(&x, &m)| (m != usize::MAX).then_some(x))
                .collect();
            let expected: Vec<_> = graph
                .matvec(&vector)
                .unwrap()
                .into_iter()
                .zip(&map)
                .filter_map(|(x, &m)| (m != usize::MAX).then_some(x))
                .collect();
            assert_eq!(compact.matvec(&active).unwrap(), expected);
            if graph
                .edges()
                .iter()
                .all(|e| map[e.u()] == e.u() && map[e.v()] == e.v())
            {
                assert!(std::sync::Arc::ptr_eq(&graph.edges, &compact.edges));
            }
        }
        let connected = Laplacian::from_edges(2, [(0, 1, 1.0)]).unwrap();
        assert!(
            connected
                .without_isolated_vertices(&mut crate::cancel::Unmetered)
                .unwrap()
                .is_none()
        );
    }

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
        let local =
            Laplacian::from_sorted_edges(6, compact, false, &mut crate::cancel::Unmetered).unwrap();
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

#[cfg(test)]
// Context literals keep `..Default::default()` for their feature-gated fields.
#[allow(clippy::needless_update)]
mod controlled_graph_tests {
    use super::*;
    use crate::cancel::{CancelMeter, StopAt};

    fn ctx(budget: usize, cancel: crate::Cancellation<'_>) -> crate::BuildContext<'_> {
        crate::BuildContext {
            cancel: Some(cancel),
            budget_bytes: Some(budget),
            ..crate::BuildContext::default()
        }
    }

    fn raw_edges() -> Vec<Edge> {
        [
            (0, 2, 1e16),
            (2, 4, 3.0),
            (0, 2, 1.0),
            (1, 4, 0.125),
            (0, 2, 2.0),
            (2, 4, 1e-16),
            (1, 4, 0.25),
            (0, 1, 7.0),
        ]
        .into_iter()
        .map(|(u, v, w)| Edge::from_internal_parts(u, v, w).unwrap())
        .collect()
    }

    #[test]
    fn public_controlled_graph_matches_and_cancels_at_every_poll() {
        let input: Vec<_> = raw_edges()
            .into_iter()
            .map(|edge| (edge.v(), edge.u(), edge.weight()))
            .collect();
        let expected = Laplacian::from_edges(6, input.iter().copied()).unwrap();
        let count = StopAt::never();
        let query = || count.query();
        let actual = Laplacian::build(6, &input, ctx(usize::MAX, &query)).unwrap();
        assert_eq!(actual, expected);
        // Small inputs stay below the counted cadence: only allocations poll.
        assert!(count.polls() > 0);
        for stop in 0..count.polls() {
            let cancel = StopAt::at(Some(stop));
            let query = || cancel.query();
            assert!(matches!(
                Laplacian::build(6, &input, ctx(usize::MAX, &query)),
                Err(CmgError::Cancelled { .. })
            ));
            assert_eq!(cancel.polls(), stop + 1);
        }
        let count = StopAt::never();
        let query = || count.query();
        let metered =
            Laplacian::build_serial(6, &input, &mut CancelMeter::every_operation(&query)).unwrap();
        assert_eq!(metered, expected);
        for stop in 0..count.polls() {
            let cancel = StopAt::at(Some(stop));
            let query = || cancel.query();
            assert!(matches!(
                Laplacian::build_serial(6, &input, &mut CancelMeter::every_operation(&query)),
                Err(CmgError::Cancelled { .. })
            ));
            assert_eq!(cancel.polls(), stop + 1);
        }
    }

    #[test]
    fn public_controlled_graph_validates_edges_and_admits_before_allocation() {
        let never = || false;
        for (count, input) in [
            (2, vec![(2, 1, 1.0)]),
            (2, vec![(0, 2, 1.0)]),
            (2, vec![(1, 1, 1.0)]),
            (2, vec![(0, 1, 0.0)]),
            (2, vec![(0, 1, -1.0)]),
            (2, vec![(0, 1, f64::INFINITY)]),
            (2, vec![(0, 1, f64::NAN)]),
        ] {
            let expected = Laplacian::from_edges(count, input.iter().copied()).unwrap_err();
            let actual = Laplacian::build(count, &input, ctx(usize::MAX, &never)).unwrap_err();
            // Debug comparison handles the NaN carried in the weight error.
            assert_eq!(format!("{actual:?}"), format!("{expected:?}"));
        }
        // Memory refusal precedes the first allocation and therefore every poll.
        let unreachable = || -> bool { panic!("memory refusal must precede any poll") };
        assert!(matches!(
            Laplacian::build(2, &[(0, 1, 1.0)], ctx(0, &unreachable)),
            Err(CmgError::MemoryBudgetExceeded { .. })
        ));
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn public_controlled_graph_rejects_wide_endpoints() {
        let wide = u32::MAX as usize + 1;
        let never = || false;
        for input in [[(wide, 0, 1.0)], [(0, wide, 1.0)]] {
            assert_eq!(
                Laplacian::build(wide + 1, &input, ctx(usize::MAX, &never)).unwrap_err(),
                CmgError::VertexIndexTooWide {
                    vertex: wide,
                    maximum: u32::MAX as usize
                }
            );
        }
    }

    #[test]
    fn controlled_graph_preserves_bits_and_cancels_at_every_poll() {
        let expected = Laplacian::canonical(6, raw_edges(), &mut crate::cancel::Unmetered).unwrap();
        let count = StopAt::never();
        let query = || count.query();
        let actual =
            Laplacian::canonical(6, raw_edges(), &mut CancelMeter::every_operation(&query))
                .unwrap();
        assert_eq!(actual, expected);
        assert_eq!(
            actual
                .diagonal()
                .iter()
                .map(|x| x.to_bits())
                .collect::<Vec<_>>(),
            expected
                .diagonal()
                .iter()
                .map(|x| x.to_bits())
                .collect::<Vec<_>>()
        );
        for stop in 0..count.polls() {
            let cancel = StopAt::at(Some(stop));
            let query = || cancel.query();
            assert!(matches!(
                Laplacian::canonical(6, raw_edges(), &mut CancelMeter::every_operation(&query),),
                Err(CmgError::Cancelled { .. })
            ));
            assert_eq!(cancel.polls(), stop + 1);
        }
    }
}
