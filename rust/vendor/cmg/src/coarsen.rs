//! Deterministic restriction, prolongation, and Galerkin graph contraction.

use crate::cancel::{Meter, Unmetered};
#[cfg(feature = "parallel")]
use crate::execution::PARALLEL_SETUP_MIN_ITEMS;
use crate::execution::Team;
use crate::preconditioner::Width;
use crate::storage::StoredIndex;
use crate::{CmgError, Edge, Laplacian};
use std::sync::OnceLock;

crate::storage::storage_fields!(Aggregation, labels, native_labels, sizes);
impl crate::storage::Storage for LabelStorage {
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

/// A zero-based partition of fine vertices into coarse aggregates.
#[derive(Debug, Clone)]
enum LabelStorage {
    Compact(Vec<u32>),
    Native(Vec<usize>),
}

/// A zero-based partition of fine vertices into coarse aggregates.
#[derive(Debug)]
pub struct Aggregation {
    labels: LabelStorage,
    native_labels: OnceLock<Vec<usize>>,
    aggregate_count: usize,
    sizes: OnceLock<Vec<usize>>,
}

impl Clone for Aggregation {
    fn clone(&self) -> Self {
        Self {
            labels: self.labels.clone(),
            native_labels: OnceLock::new(),
            aggregate_count: self.aggregate_count,
            sizes: OnceLock::new(),
        }
    }
}

impl PartialEq for Aggregation {
    fn eq(&self, other: &Self) -> bool {
        self.aggregate_count == other.aggregate_count
            && self.fine_dimension() == other.fine_dimension()
            && (0..self.fine_dimension()).all(|index| self.label_at(index) == other.label_at(index))
    }
}

impl Eq for Aggregation {}

impl Aggregation {
    pub(crate) fn retained_bytes(&self) -> usize {
        let labels = match &self.labels {
            LabelStorage::Compact(labels) => labels
                .capacity()
                .saturating_mul(core::mem::size_of::<u32>()),
            LabelStorage::Native(labels) => labels
                .capacity()
                .saturating_mul(core::mem::size_of::<usize>()),
        };
        labels
            .saturating_add(self.native_labels.get().map_or(0, |labels| {
                labels
                    .capacity()
                    .saturating_mul(core::mem::size_of::<usize>())
            }))
            .saturating_add(self.sizes.get().map_or(0, |sizes| {
                sizes
                    .capacity()
                    .saturating_mul(core::mem::size_of::<usize>())
            }))
    }

    /// Construct an aggregation from explicit labels and a coarse dimension.
    pub fn new(labels: Vec<usize>, aggregate_count: usize) -> Result<Self, CmgError> {
        for &label in &labels {
            if label >= aggregate_count {
                return Err(CmgError::VertexOutOfBounds {
                    vertex: label,
                    vertex_count: aggregate_count,
                });
            }
        }
        Self::from_forest_labels(labels, aggregate_count, &mut Unmetered)
    }

    /// Compact (or native-width) labels from validated forest labels.
    pub(crate) fn from_forest_labels<M: Meter>(
        labels: Vec<usize>,
        aggregate_count: usize,
        meter: &mut M,
    ) -> Result<Self, CmgError> {
        let labels = if aggregate_count <= (u32::MAX as usize).saturating_add(1) {
            meter.allocation(
                "aggregation compact labels",
                crate::storage::bytes::<u32>(labels.len())?,
            )?;
            let mut compact =
                crate::storage::try_capacity(labels.len(), "aggregation compact labels")?;
            if M::POLLS {
                for &label in &labels {
                    meter.work("aggregation compact label copy", 16)?;
                    debug_assert!(label < aggregate_count);
                    compact.push(label as u32);
                }
            } else {
                compact.extend(labels.iter().map(|&label| label as u32));
            }
            LabelStorage::Compact(compact)
        } else {
            LabelStorage::Native(labels)
        };
        Ok(Self {
            labels,
            native_labels: OnceLock::new(),
            aggregate_count,
            sizes: OnceLock::new(),
        })
    }

    /// Return the fine-to-coarse labels.
    ///
    /// Hierarchy-built aggregations retain compact labels internally. The
    /// native-width compatibility slice is materialized lazily only when this
    /// public accessor is called.
    #[must_use]
    pub fn labels(&self) -> &[usize] {
        match &self.labels {
            LabelStorage::Native(labels) => labels,
            LabelStorage::Compact(labels) => self
                .native_labels
                .get_or_init(|| labels.iter().map(|&label| label as usize).collect()),
        }
    }

    /// Return aggregate sizes.
    ///
    /// Production hierarchy kernels need only the aggregate count. The full
    /// native-width size vector is materialized lazily for API compatibility.
    #[must_use]
    pub fn sizes(&self) -> &[usize] {
        self.sizes.get_or_init(|| {
            let mut sizes = vec![0; self.aggregate_count];
            match &self.labels {
                LabelStorage::Compact(labels) => {
                    for &label in labels {
                        sizes[label as usize] += 1;
                    }
                }
                LabelStorage::Native(labels) => {
                    for &label in labels {
                        sizes[label] += 1;
                    }
                }
            }
            sizes
        })
    }

    /// Return the fine dimension.
    #[must_use]
    pub fn fine_dimension(&self) -> usize {
        match &self.labels {
            LabelStorage::Compact(labels) => labels.len(),
            LabelStorage::Native(labels) => labels.len(),
        }
    }

    /// Return the coarse dimension.
    #[must_use]
    pub const fn coarse_dimension(&self) -> usize {
        self.aggregate_count
    }

    #[inline]
    pub(crate) fn label_at(&self, index: usize) -> usize {
        match &self.labels {
            LabelStorage::Compact(labels) => labels[index] as usize,
            LabelStorage::Native(labels) => labels[index],
        }
    }

    /// Restrict by summing fine values within every aggregate.
    pub fn restrict(&self, fine: &[f64]) -> Result<Vec<f64>, CmgError> {
        let mut coarse = vec![0.0; self.coarse_dimension()];
        self.restrict_into(fine, &mut coarse)?;
        Ok(coarse)
    }

    /// Restrict into caller-owned storage.
    pub fn restrict_into(&self, fine: &[f64], coarse: &mut [f64]) -> Result<(), CmgError> {
        if fine.len() != self.fine_dimension() {
            return Err(CmgError::dimension(
                "Aggregation::restrict fine",
                self.fine_dimension(),
                fine.len(),
            ));
        }
        if coarse.len() != self.coarse_dimension() {
            return Err(CmgError::dimension(
                "Aggregation::restrict coarse",
                self.coarse_dimension(),
                coarse.len(),
            ));
        }
        coarse.fill(0.0);
        match &self.labels {
            LabelStorage::Compact(labels) => {
                for (&value, &label) in fine.iter().zip(labels) {
                    coarse[label as usize] += value;
                }
            }
            LabelStorage::Native(labels) => {
                for (&value, &label) in fine.iter().zip(labels) {
                    coarse[label] += value;
                }
            }
        }
        Ok(())
    }

    /// Restrict `rhs - product` for vertex-major lanes: each lane subtracts per
    /// fine vertex and adds into its aggregate in ascending fine order.
    pub(crate) fn restrict_residual_lanes<W: Width, M: Meter>(
        &self,
        rhs: &[f64],
        product: &[f64],
        coarse: &mut [f64],
        width: W,
        meter: &mut M,
    ) -> Result<(), CmgError> {
        let columns = width.get();
        let fine = self.fine_dimension() * columns;
        for (context, actual) in [
            ("Aggregation residual rhs", rhs.len()),
            ("Aggregation residual matvec", product.len()),
        ] {
            if actual != fine {
                return Err(CmgError::dimension(context, fine, actual));
            }
        }
        if coarse.len() != self.coarse_dimension() * columns {
            return Err(CmgError::dimension(
                "Aggregation residual coarse",
                self.coarse_dimension() * columns,
                coarse.len(),
            ));
        }
        crate::cancel::zero(meter, "CMG restriction", coarse)?;
        match &self.labels {
            LabelStorage::Compact(labels) => {
                restrict_lanes(labels, rhs, product, coarse, columns, meter)
            }
            LabelStorage::Native(labels) => {
                restrict_lanes(labels, rhs, product, coarse, columns, meter)
            }
        }
    }

    /// Prolong by copying each coarse value to its fine aggregate members.
    pub fn prolong(&self, coarse: &[f64]) -> Result<Vec<f64>, CmgError> {
        let mut fine = vec![0.0; self.fine_dimension()];
        self.prolong_into(coarse, &mut fine)?;
        Ok(fine)
    }

    /// Prolong into caller-owned storage.
    pub fn prolong_into(&self, coarse: &[f64], fine: &mut [f64]) -> Result<(), CmgError> {
        validate_prolong_dimensions(self, coarse, fine)?;
        match &self.labels {
            LabelStorage::Compact(labels) => {
                for (value, &label) in fine.iter_mut().zip(labels) {
                    *value = coarse[label as usize];
                }
            }
            LabelStorage::Native(labels) => {
                for (value, &label) in fine.iter_mut().zip(labels) {
                    *value = coarse[label];
                }
            }
        }
        Ok(())
    }

    /// Add a prolonged coarse vector to a fine vector in place.
    pub fn prolong_add_into(&self, coarse: &[f64], fine: &mut [f64]) -> Result<(), CmgError> {
        validate_prolong_dimensions(self, coarse, fine)?;
        match &self.labels {
            LabelStorage::Compact(labels) => {
                for (value, &label) in fine.iter_mut().zip(labels) {
                    *value += coarse[label as usize];
                }
            }
            LabelStorage::Native(labels) => {
                for (value, &label) in fine.iter_mut().zip(labels) {
                    *value += coarse[label];
                }
            }
        }
        Ok(())
    }

    /// Add the prolonged coarse lanes to every fine member; fine rows run on
    /// the team above its parallel threshold.
    pub(crate) fn prolong_add_lanes<W: Width, M: Meter>(
        &self,
        coarse: &[f64],
        fine: &mut [f64],
        width: W,
        #[cfg_attr(not(feature = "parallel"), allow(unused_variables))] team: Option<Team<'_>>,
        meter: &mut M,
    ) -> Result<(), CmgError> {
        let columns = width.get();
        if coarse.len() != self.coarse_dimension() * columns {
            return Err(CmgError::dimension(
                "Aggregation::prolong coarse",
                self.coarse_dimension() * columns,
                coarse.len(),
            ));
        }
        if fine.len() != self.fine_dimension() * columns {
            return Err(CmgError::dimension(
                "Aggregation::prolong fine",
                self.fine_dimension() * columns,
                fine.len(),
            ));
        }
        match &self.labels {
            LabelStorage::Compact(labels) => {
                prolong_lanes(labels, coarse, fine, columns, team, meter)
            }
            LabelStorage::Native(labels) => {
                prolong_lanes(labels, coarse, fine, columns, team, meter)
            }
        }
    }

    /// Form the exact graph Laplacian `R L R^T`.
    pub fn contract(&self, graph: &Laplacian) -> Result<Laplacian, CmgError> {
        self.contract_with(graph, None, &mut Unmetered)
    }

    /// The single contraction kernel. With a team whose policy enables parallel
    /// work on at least 65,536 edges, edges are mapped into fixed slots and
    /// sorted in place on the team's pool; otherwise mapping is a counted serial
    /// scan. Both preserve input order before the identical canonical sort, so
    /// the coarse graph is bit-for-bit the same.
    pub(crate) fn contract_with<M: Meter>(
        &self,
        graph: &Laplacian,
        #[allow(unused_variables)] team: Option<Team<'_>>,
        meter: &mut M,
    ) -> Result<Laplacian, CmgError> {
        self.validate_contract_graph(graph)?;
        #[cfg(feature = "parallel")]
        if let Some(team) = team.filter(|team| {
            graph.edge_count() >= PARALLEL_SETUP_MIN_ITEMS
                && team.should_parallel(graph.edge_count())
        }) {
            return self.contract_parallel(graph, team, meter);
        }
        meter.allocation(
            "contracted edges",
            crate::storage::bytes::<Edge>(graph.edge_count())?,
        )?;
        let mut coarse_edges =
            crate::storage::try_capacity(graph.edge_count(), "contracted edges")?;
        match &self.labels {
            LabelStorage::Compact(labels) => {
                for edge in graph.edges() {
                    meter.work("contracted edge mapping", 64)?;
                    let left = labels[edge.u()] as usize;
                    let right = labels[edge.v()] as usize;
                    if left != right {
                        coarse_edges.push(Edge::from_internal_parts(left, right, edge.weight())?);
                    }
                }
            }
            LabelStorage::Native(labels) => {
                for edge in graph.edges() {
                    meter.work("contracted edge mapping", 64)?;
                    let left = labels[edge.u()];
                    let right = labels[edge.v()];
                    if left != right {
                        coarse_edges.push(Edge::from_internal_parts(left, right, edge.weight())?);
                    }
                }
            }
        }
        Laplacian::canonical(self.coarse_dimension(), coarse_edges, meter)
    }

    /// Fixed slot and edge banks belong to the admitted parallel bound. Workers
    /// only write disjoint slots in bounded logical chunks capped at the worker
    /// ceiling; the slot bank is filled and compacted in counted segments and
    /// released before the in-place team sort. Every stage polls the meter's
    /// query; allocations are those of the earlier in-place parallel sort.
    #[cfg(feature = "parallel")]
    fn contract_parallel<M: Meter>(
        &self,
        graph: &Laplacian,
        team: Team<'_>,
        meter: &mut M,
    ) -> Result<Laplacian, CmgError> {
        const CHUNK: usize = 16;
        let mut mapped: Vec<Option<Edge>> = crate::cancel::try_filled(
            meter,
            graph.edge_count(),
            None,
            "parallel contracted edge slots",
        )?;
        let mut coarse_edges =
            meter.try_capacity(graph.edge_count(), "parallel contracted edges")?;
        crate::graph::stage(
            "parallel contracted edge mapping",
            CHUNK,
            team,
            &mut mapped,
            meter.query(),
            |chunk, slots| {
                for (offset, slot) in slots.iter_mut().enumerate() {
                    let edge = graph.edges()[chunk * CHUNK + offset];
                    let (left, right) = match &self.labels {
                        LabelStorage::Compact(labels) => {
                            (labels[edge.u()] as usize, labels[edge.v()] as usize)
                        }
                        LabelStorage::Native(labels) => (labels[edge.u()], labels[edge.v()]),
                    };
                    if left != right {
                        *slot = Some(Edge::from_internal_parts(left, right, edge.weight())?);
                    }
                }
                Ok(())
            },
        )?;
        // Preserve input order before the unchanged canonical sort. At most m
        // items are appended to capacity m; this cannot grow.
        crate::cancel::chunked(
            meter,
            "parallel contracted edge compaction",
            mapped.len(),
            8,
            |range| {
                coarse_edges.extend(mapped[range].iter().flatten().copied());
                Ok(())
            },
        )?;
        drop(mapped);
        Laplacian::canonical_parallel(self.coarse_dimension(), coarse_edges, team, meter)
    }

    fn validate_contract_graph(&self, graph: &Laplacian) -> Result<(), CmgError> {
        if graph.vertex_count() != self.fine_dimension() {
            return Err(CmgError::dimension(
                "Aggregation::contract",
                self.fine_dimension(),
                graph.vertex_count(),
            ));
        }
        Ok(())
    }
}

#[inline(always)]
fn restrict_lanes<L: StoredIndex, M: Meter>(
    labels: &[L],
    rhs: &[f64],
    product: &[f64],
    coarse: &mut [f64],
    columns: usize,
    meter: &mut M,
) -> Result<(), CmgError> {
    // Keep each subtraction and the ascending fine-vertex addition order.
    crate::cancel::row_lanes(
        meter,
        "CMG restriction",
        labels.len(),
        columns,
        1,
        #[inline(always)]
        |fines, lanes| {
            for (fine, label) in fines.clone().zip(&labels[fines]) {
                let target = label.index() * columns;
                for lane in lanes.clone() {
                    coarse[target + lane] +=
                        rhs[fine * columns + lane] - product[fine * columns + lane];
                }
            }
            Ok(())
        },
    )
}

#[inline(always)]
fn prolong_lanes<L: StoredIndex, M: Meter>(
    labels: &[L],
    coarse: &[f64],
    fine: &mut [f64],
    columns: usize,
    #[cfg_attr(not(feature = "parallel"), allow(unused_variables))] team: Option<Team<'_>>,
    meter: &mut M,
) -> Result<(), CmgError> {
    // By value, as in the cycle's row operations: the parallel path must not
    // make the serial loop reload the width after each polled segment.
    let operation = move |row: usize, first: usize, values: &mut [f64]| {
        let source = labels[row].index() * columns + first;
        for (lane, value) in values.iter_mut().enumerate() {
            *value += coarse[source + lane];
        }
    };
    #[cfg(feature = "parallel")]
    if let Some(team) = team.filter(|team| team.should_parallel(fine.len())) {
        let rows =
            crate::preconditioner::rows_parallel(team, meter.query(), fine, columns, operation);
        meter.joined("CMG prolongation")?;
        return rows;
    }
    crate::preconditioner::rows_serial(meter, "CMG prolongation", fine, columns, operation)
}

fn validate_prolong_dimensions(
    aggregation: &Aggregation,
    coarse: &[f64],
    fine: &[f64],
) -> Result<(), CmgError> {
    if coarse.len() != aggregation.coarse_dimension() {
        return Err(CmgError::dimension(
            "Aggregation::prolong coarse",
            aggregation.coarse_dimension(),
            coarse.len(),
        ));
    }
    if fine.len() != aggregation.fine_dimension() {
        return Err(CmgError::dimension(
            "Aggregation::prolong fine",
            aggregation.fine_dimension(),
            fine.len(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod compact_aggregation_label_tests {
    use super::{Aggregation, LabelStorage};

    #[test]
    fn compact_storage_preserves_public_labels_and_algebra() {
        let aggregation = Aggregation::new(vec![0, 0, 1, 2, 2], 3).unwrap();
        assert!(matches!(&aggregation.labels, LabelStorage::Compact(_)));
        assert!(aggregation.native_labels.get().is_none());
        assert!(aggregation.sizes.get().is_none());

        let fine = [1.0, 2.0, 3.0, 4.0, 5.0];
        assert_eq!(aggregation.restrict(&fine).unwrap(), vec![3.0, 3.0, 9.0]);
        assert_eq!(
            aggregation.prolong(&[10.0, 20.0, 30.0]).unwrap(),
            vec![10.0, 10.0, 20.0, 30.0, 30.0]
        );
        assert_eq!(aggregation.labels(), &[0, 0, 1, 2, 2]);
        assert!(aggregation.native_labels.get().is_some());
        assert!(aggregation.sizes.get().is_none());
        assert_eq!(aggregation.sizes(), &[2, 1, 2]);
        assert!(aggregation.sizes.get().is_some());
    }

    #[test]
    fn clone_does_not_duplicate_materialized_native_cache() {
        let aggregation = Aggregation::new(vec![0, 1, 1, 2], 3).unwrap();
        let _ = aggregation.labels();
        let cloned = aggregation.clone();
        assert_eq!(aggregation, cloned);
        let _ = aggregation.sizes();
        assert!(cloned.native_labels.get().is_none());
        assert!(cloned.sizes.get().is_none());
        assert_eq!(cloned.labels(), &[0, 1, 1, 2]);
        assert_eq!(cloned.sizes(), &[1, 2, 1]);
    }
}

#[cfg(test)]
mod residual_restriction_tests {
    use super::*;
    use crate::preconditioner::One;

    #[test]
    fn fused_restriction_keeps_cancellation_order_and_checks_dimensions() {
        let labels = vec![2, 0, 1, 2, 2, 1, 0, 2];
        let rhs = [1e100, 5.0, -0.0, 1.0, -1e100, 2.0, -5.0, 3.0];
        let ax = [0.0, 1.0, 0.0, 0.5, 0.0, 1e-100, -1.0, -0.0];
        let residual: Vec<_> = rhs.iter().zip(ax).map(|(b, a)| b - a).collect();
        for native in [false, true] {
            let mut aggregation = Aggregation::new(labels.clone(), 3).unwrap();
            if native {
                aggregation.labels = LabelStorage::Native(labels.clone());
            }
            let expected = aggregation.restrict(&residual).unwrap();
            let mut actual = vec![f64::NAN; 3];
            aggregation
                .restrict_residual_lanes(&rhs, &ax, &mut actual, One, &mut Unmetered)
                .unwrap();
            assert_eq!(
                actual.iter().map(|x| x.to_bits()).collect::<Vec<_>>(),
                expected.iter().map(|x| x.to_bits()).collect::<Vec<_>>()
            );
            for (b, a) in [(&rhs[..7], &ax[..]), (&rhs[..], &ax[..7])] {
                assert!(
                    aggregation
                        .restrict_residual_lanes(b, a, &mut actual, One, &mut Unmetered)
                        .is_err()
                );
                assert_eq!(actual, expected);
            }
            assert!(
                aggregation
                    .restrict_residual_lanes(&rhs, &ax, &mut [99.0; 2], One, &mut Unmetered)
                    .is_err()
            );
        }
        let empty = Aggregation::new(Vec::new(), 0).unwrap();
        empty
            .restrict_residual_lanes(&[], &[], &mut [], One, &mut Unmetered)
            .unwrap();
    }
}

#[cfg(all(test, feature = "experimental-components"))]
mod controlled_aggregation_tests {
    use super::*;
    use crate::cancel::{CancelMeter, StopAt};

    #[test]
    fn controlled_labels_and_contraction_preserve_values_and_cancel() {
        let labels = vec![0, 0, 1, 1, 2, 2, 3, 3, 4];
        let graph = Laplacian::from_edges(
            9,
            [
                (0, 1, 1.0),
                (0, 2, 1e16),
                (1, 3, 1.0),
                (1, 2, 2.0),
                (3, 4, 0.5),
                (5, 7, 3.0),
                (6, 8, 1.0),
            ],
        )
        .unwrap();
        let original = graph.clone();
        let expected = Aggregation::from_forest_labels(labels.clone(), 5, &mut Unmetered).unwrap();
        let never = || false;
        let actual = Aggregation::from_forest_labels(
            labels.clone(),
            5,
            &mut CancelMeter::every_operation(&never),
        )
        .unwrap();
        assert_eq!(actual, expected);
        let count = StopAt::never();
        let query = || count.query();
        let coarse = actual
            .contract_with(&graph, None, &mut CancelMeter::every_operation(&query))
            .unwrap();
        assert_eq!(coarse, expected.contract(&graph).unwrap());
        for stop in 0..count.polls() {
            let cancel = StopAt::at(Some(stop));
            let query = || cancel.query();
            assert!(matches!(
                actual.contract_with(&graph, None, &mut CancelMeter::every_operation(&query)),
                Err(CmgError::Cancelled { .. })
            ));
            assert_eq!(cancel.polls(), stop + 1);
        }
        let always = || true;
        assert!(matches!(
            Aggregation::from_forest_labels(labels, 5, &mut CancelMeter::every_operation(&always)),
            Err(CmgError::Cancelled { .. })
        ));
        assert_eq!(graph, original);
    }
}
