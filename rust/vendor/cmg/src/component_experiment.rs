//! Opt-in component experiments. Ordinary builders retain the upstream cycle.

use crate::cancel::{Meter, row_lanes};
use crate::preconditioner::Width;
use crate::{Aggregation, CmgError, Laplacian};

crate::storage::storage_fields!(PrunedTransfer, entries);

/// Independent experimental changes, for attribution against the ordinary builder.
///
/// These options change setup only. Every resulting preconditioner is fixed
/// across applications; RHS values and convergence never determine its structure.
#[derive(Debug, Clone, Copy, Default)]
pub struct ComponentBuildOptions {
    /// Retire isolated coarse vertices after contraction, retaining parent smoothing.
    pub prune_coarse_isolates: bool,
    /// Keep retired representatives in vertex-based stopping decisions.
    ///
    /// With pruning enabled, this separates compact storage from early direct
    /// factorization. Retired representatives carry no edges, vectors or work.
    /// An empty compact child always terminates directly. Ignored without pruning.
    pub preserve_unpruned_stopping: bool,
    /// Use ordered sparse factors for direct terminals, one component at a time.
    pub factor_terminal_components: bool,
}

/// Restriction to surviving coarse vertices, with transpose prolongation.
///
/// Unlike an [`Aggregation`], some fine rows have no coarse entry. Only whole
/// isolated coarse vertices are removed. Entries retain increasing fine-vertex
/// order, so restriction keeps the original order of floating-point additions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrunedTransfer {
    fine_dimension: usize,
    coarse_dimension: usize,
    represented_coarse_dimension: usize,
    entries: Vec<(u32, u32)>,
}

impl PrunedTransfer {
    /// Dimension of the complete parent level, including retired components.
    pub const fn fine_dimension(&self) -> usize {
        self.fine_dimension
    }

    /// Dimension of the compact child graph.
    pub const fn coarse_dimension(&self) -> usize {
        self.coarse_dimension
    }

    /// Number of aggregates before isolated coarse vertices were removed.
    pub const fn represented_coarse_dimension(&self) -> usize {
        self.represented_coarse_dimension
    }

    /// Surviving `(fine_vertex, coarse_vertex)` entries in fine-vertex order.
    pub fn entries(&self) -> &[(u32, u32)] {
        &self.entries
    }

    pub(crate) fn retained_bytes(&self) -> usize {
        self.entries
            .capacity()
            .saturating_mul(core::mem::size_of::<(u32, u32)>())
    }

    /// Restrict into the compact child, ignoring retired coarse degrees of freedom.
    pub fn restrict_into(&self, fine: &[f64], coarse: &mut [f64]) -> Result<(), CmgError> {
        self.validate(fine.len(), coarse.len())?;
        coarse.fill(0.0);
        if self.has_fine_prefix() {
            for (&value, &(_, coarse_index)) in fine.iter().zip(&self.entries) {
                coarse[coarse_index as usize] += value;
            }
        } else {
            for &(fine_index, coarse_index) in &self.entries {
                coarse[coarse_index as usize] += fine[fine_index as usize];
            }
        }
        Ok(())
    }

    /// Restrict `rhs - product` for vertex-major lanes in increasing fine order,
    /// ignoring retired rows.
    pub(crate) fn restrict_residual_lanes<W: Width, M: Meter>(
        &self,
        rhs: &[f64],
        product: &[f64],
        coarse: &mut [f64],
        width: W,
        meter: &mut M,
    ) -> Result<(), CmgError> {
        let columns = width.get();
        self.validate_lanes(rhs.len(), coarse.len(), columns)?;
        self.validate_lanes(product.len(), coarse.len(), columns)?;
        crate::cancel::zero(meter, "CMG pruned restriction", coarse)?;
        row_lanes(
            meter,
            "CMG pruned restriction",
            self.entries.len(),
            columns,
            1,
            #[inline(always)]
            |entries, lanes| {
                for &(fine, target) in &self.entries[entries] {
                    let (fine, target) = (fine as usize * columns, target as usize * columns);
                    for lane in lanes.clone() {
                        coarse[target + lane] += rhs[fine + lane] - product[fine + lane];
                    }
                }
                Ok(())
            },
        )
    }

    /// Add the transpose action for vertex-major lanes; retired rows are unchanged.
    pub(crate) fn prolong_add_lanes<W: Width, M: Meter>(
        &self,
        coarse: &[f64],
        fine: &mut [f64],
        width: W,
        meter: &mut M,
    ) -> Result<(), CmgError> {
        let columns = width.get();
        self.validate_lanes(fine.len(), coarse.len(), columns)?;
        row_lanes(
            meter,
            "CMG pruned prolongation",
            self.entries.len(),
            columns,
            1,
            #[inline(always)]
            |entries, lanes| {
                for &(target, source) in &self.entries[entries] {
                    let (target, source) = (target as usize * columns, source as usize * columns);
                    for lane in lanes.clone() {
                        fine[target + lane] += coarse[source + lane];
                    }
                }
                Ok(())
            },
        )
    }

    fn validate_lanes(&self, fine: usize, coarse: usize, columns: usize) -> Result<(), CmgError> {
        if fine != self.fine_dimension * columns {
            return Err(CmgError::dimension(
                "PrunedTransfer fine",
                self.fine_dimension * columns,
                fine,
            ));
        }
        if coarse != self.coarse_dimension * columns {
            return Err(CmgError::dimension(
                "PrunedTransfer coarse",
                self.coarse_dimension * columns,
                coarse,
            ));
        }
        Ok(())
    }

    /// Add the transpose action; fine rows without an entry remain unchanged.
    pub fn prolong_add_into(&self, coarse: &[f64], fine: &mut [f64]) -> Result<(), CmgError> {
        self.validate(fine.len(), coarse.len())?;
        if self.has_fine_prefix() {
            for (value, &(_, coarse_index)) in fine.iter_mut().zip(&self.entries) {
                *value += coarse[coarse_index as usize];
            }
        } else {
            for &(fine_index, coarse_index) in &self.entries {
                fine[fine_index as usize] += coarse[coarse_index as usize];
            }
        }
        Ok(())
    }

    fn has_fine_prefix(&self) -> bool {
        // Entries are unique and sorted by fine index, so this last-index test
        // proves that every preceding fine row is represented contiguously.
        self.entries
            .last()
            .is_none_or(|&(fine, _)| fine as usize + 1 == self.entries.len())
    }

    fn validate(&self, fine: usize, coarse: usize) -> Result<(), CmgError> {
        if fine != self.fine_dimension {
            return Err(CmgError::dimension(
                "PrunedTransfer fine",
                self.fine_dimension,
                fine,
            ));
        }
        if coarse != self.coarse_dimension {
            return Err(CmgError::dimension(
                "PrunedTransfer coarse",
                self.coarse_dimension,
                coarse,
            ));
        }
        Ok(())
    }
}

/// Retire isolated coarse vertices, returning the compact transfer and graph.
pub(crate) fn prune<M: Meter>(
    aggregation: &Aggregation,
    coarse: &Laplacian,
    meter: &mut M,
) -> Result<Option<(PrunedTransfer, Laplacian)>, CmgError> {
    let Some((map, graph)) = coarse.without_isolated_vertices(meter)? else {
        return Ok(None);
    };
    meter.allocation(
        "pruned transfer entries",
        crate::storage::bytes::<(u32, u32)>(aggregation.fine_dimension())?,
    )?;
    let mut entries =
        crate::storage::try_capacity(aggregation.fine_dimension(), "pruned transfer entries")?;
    // The public graph representation already bounds vertex indices to u32.
    for fine in 0..aggregation.fine_dimension() {
        meter.work("pruned transfer mapping", 32)?;
        let label = aggregation.label_at(fine);
        if map[label] != usize::MAX {
            entries.push((fine as u32, map[label] as u32));
        }
    }
    if entries.len() != entries.capacity() {
        meter.allocation(
            "pruned transfer compaction",
            crate::storage::bytes::<(u32, u32)>(entries.len())?,
        )?;
        let mut compact =
            crate::storage::try_capacity(entries.len(), "pruned transfer compaction")?;
        for &entry in &entries {
            meter.work("pruned transfer compaction copy", 8)?;
            compact.push(entry);
        }
        entries = compact;
    }
    Ok(Some((
        PrunedTransfer {
            fine_dimension: aggregation.fine_dimension(),
            coarse_dimension: graph.vertex_count(),
            represented_coarse_dimension: coarse.vertex_count(),
            entries,
        },
        graph,
    )))
}

#[cfg(test)]
mod residual_restriction_tests {
    use super::*;
    use crate::cancel::Unmetered;
    use crate::preconditioner::One;

    #[test]
    fn pruned_residual_restriction_preserves_order_and_ignores_retired_rows() {
        let rhs = [1e100, 1.0, -1e100, 3.0, -0.0, 5.0, f64::NAN, f64::NAN];
        let ax = [0.0, 0.5, 0.0, -0.0, 0.0, 1.0, f64::NAN, f64::NAN];
        for entries in [
            vec![(0, 0), (1, 0), (2, 0), (3, 0)],
            vec![(0, 0), (2, 0), (4, 1), (5, 1)],
            vec![],
        ] {
            let transfer = PrunedTransfer {
                fine_dimension: 8,
                coarse_dimension: 2,
                represented_coarse_dimension: 4,
                entries,
            };
            let residual: Vec<_> = rhs.iter().zip(ax).map(|(b, a)| b - a).collect();
            let mut expected = [f64::NAN; 2];
            transfer.restrict_into(&residual, &mut expected).unwrap();
            let mut actual = [f64::NAN; 2];
            transfer
                .restrict_residual_lanes(&rhs, &ax, &mut actual, One, &mut Unmetered)
                .unwrap();
            assert_eq!(actual.map(f64::to_bits), expected.map(f64::to_bits));
            assert!(
                transfer
                    .restrict_residual_lanes(&rhs[..7], &ax, &mut actual, One, &mut Unmetered)
                    .is_err()
            );
            assert!(
                transfer
                    .restrict_residual_lanes(&rhs, &ax[..7], &mut actual, One, &mut Unmetered)
                    .is_err()
            );
            assert_eq!(actual.map(f64::to_bits), expected.map(f64::to_bits));
        }
    }
}

#[cfg(test)]
mod controlled_prune_tests {
    use super::*;
    use crate::cancel::{CancelMeter, StopAt};

    #[test]
    fn metered_pruning_matches_and_cancels_at_every_poll() {
        let aggregation = Aggregation::new(vec![0, 0, 1, 2, 3, 4, 4, 5], 6).unwrap();
        // Exercise interspersed isolates, suffix isolates, no isolates, and all isolates.
        for edges in [
            vec![(1, 4, 2.0)],
            vec![(0, 1, 1.0)],
            vec![
                (0, 1, 1.0),
                (1, 2, 2.0),
                (2, 3, 3.0),
                (3, 4, 4.0),
                (4, 5, 5.0),
            ],
            vec![],
        ] {
            let graph = Laplacian::from_edges(6, edges).unwrap();
            let expected = prune(&aggregation, &graph, &mut crate::cancel::Unmetered).unwrap();
            let count = StopAt::never();
            let query = || count.query();
            let actual = prune(
                &aggregation,
                &graph,
                &mut CancelMeter::every_operation(&query),
            )
            .unwrap();
            assert_eq!(actual, expected);
            assert!(count.polls() > 0);
            for stop in 0..count.polls() {
                let cancel = StopAt::at(Some(stop));
                let query = || cancel.query();
                let error = prune(
                    &aggregation,
                    &graph,
                    &mut CancelMeter::every_operation(&query),
                )
                .unwrap_err();
                assert_eq!(cancel.polls(), stop + 1);
                assert!(matches!(error, CmgError::Cancelled { .. }));
            }
        }
    }
}
