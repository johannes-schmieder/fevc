//! Reusable canonical topology for changing-weight Laplacians.

use crate::{CmgError, Components, Edge, Laplacian};
use std::sync::Arc;

crate::storage::storage_fields!(
    PreparedLaplacianTopology,
    canonical_keys,
    group_offsets,
    group_input_indices,
    input_to_canonical,
    components,
    lineage
);
crate::storage::storage_fields!(PreparedLaplacianWorkspace, duplicate_weights);

/// Canonical, validated graph structure reusable across numeric weight frames.
#[derive(Debug, Clone)]
pub struct PreparedLaplacianTopology {
    vertex_count: usize,
    input_edge_count: usize,
    canonical_keys: Vec<u64>,
    group_offsets: Vec<usize>,
    group_input_indices: Vec<usize>,
    input_to_canonical: Vec<usize>,
    components: Arc<Components>,
    lineage: Arc<()>,
    maximum_duplicate_count: usize,
}

impl PreparedLaplacianTopology {
    /// Requested-layout upper bound for `prepare_with_budget`, including the
    /// indexed input, all maps, union/find scratch, and Arc blocks. Caller
    /// endpoint storage and allocator overhead are excluded. No executor is used.
    pub fn preparation_allocation_bound(
        vertices: usize,
        input_edges: usize,
    ) -> Result<usize, CmgError> {
        use crate::storage::{add, arc_bytes, bytes};
        let offsets = input_edges
            .checked_add(1)
            .ok_or(CmgError::AllocationFailed {
                context: "prepared offset size overflows",
            })?;
        let mut total = bytes::<(u64, usize)>(input_edges)?;
        total = add(total, bytes::<u64>(input_edges)?)?;
        total = add(total, bytes::<usize>(offsets)?)?;
        total = add(total, bytes::<[usize; 2]>(input_edges)?)?;
        total = add(total, bytes::<[usize; 4]>(vertices)?)?;
        add(add(total, arc_bytes::<Components>()?)?, arc_bytes::<()>()?)
    }

    /// Admit before reading/allocating topology data. The slice fixes the input
    /// length; arbitrary iterator size hints cannot bypass this bound.
    pub fn prepare_with_budget(
        vertices: usize,
        endpoints: &[(usize, usize)],
        budget_bytes: usize,
    ) -> Result<Self, CmgError> {
        Self::prepare_with(
            vertices,
            endpoints,
            crate::BuildContext {
                budget_bytes: Some(budget_bytes),
                ..crate::BuildContext::default()
            },
        )
    }

    /// New numeric graph layouts; existing topology and assembly scratch are
    /// borrowed and must be charged separately, as must an older live frame.
    pub fn numeric_allocation_bytes(&self) -> Result<usize, CmgError> {
        use crate::storage::{add, arc_bytes, bytes};
        let payload = add(
            bytes::<Edge>(self.canonical_keys.len())?,
            bytes::<f64>(self.vertex_count)?,
        )?;
        add(
            add(
                add(payload, arc_bytes::<Vec<Edge>>()?)?,
                arc_bytes::<Vec<f64>>()?,
            )?,
            arc_bytes::<()>()?,
        )
    }

    /// Assemble with a pre-allocation incremental requested-layout budget.
    /// Includes the new graph's owned Arc blocks, but not shared topology blocks.
    pub fn assemble_with_workspace_budgeted(
        &self,
        weights: &[f64],
        workspace: &mut PreparedLaplacianWorkspace,
        budget_bytes: usize,
    ) -> Result<Laplacian, CmgError> {
        self.assemble_with(
            weights,
            workspace,
            crate::BuildContext {
                budget_bytes: Some(budget_bytes),
                ..crate::BuildContext::default()
            },
        )
    }

    /// Assemble one numeric frame under a construction context; its team is
    /// unused. A budget admits [`Self::numeric_allocation_bytes`] before the
    /// first allocation. With a cancellation query, weight validation, every
    /// duplicate group (gathered, sorted by counted heapsort and summed in
    /// counted segments) and the diagonal are polled at the fixed cadence and
    /// before every allocation, and once more before the frame is returned.
    /// The graph's bits never depend on the context.
    pub fn assemble_with(
        &self,
        weights: &[f64],
        workspace: &mut PreparedLaplacianWorkspace,
        context: crate::BuildContext<'_>,
    ) -> Result<Laplacian, CmgError> {
        if let Some(budget) = context.budget_bytes {
            crate::storage::admit(self.numeric_allocation_bytes()?, budget)?;
        }
        match context.cancel {
            Some(cancel) => self.assemble_metered(
                weights,
                workspace,
                &mut crate::cancel::CancelMeter::new(cancel),
            ),
            None => self.assemble_metered(weights, workspace, &mut crate::cancel::Unmetered),
        }
    }
    /// Prepare from a borrowed slice under a construction context; its team is
    /// unused. A budget admits [`Self::preparation_allocation_bound`] first.
    /// With a cancellation query, endpoint validation, the counted heapsort of
    /// the indexed endpoints, duplicate grouping and component discovery are
    /// polled at the fixed cadence and before every allocation, and once more
    /// before the topology is returned. The topology never depends on the
    /// context: the sort key (canonical endpoints, input index) is unique.
    pub fn prepare_with(
        vertices: usize,
        endpoints: &[(usize, usize)],
        context: crate::BuildContext<'_>,
    ) -> Result<Self, CmgError> {
        if let Some(budget) = context.budget_bytes {
            crate::storage::admit(
                Self::preparation_allocation_bound(vertices, endpoints.len())?,
                budget,
            )?;
        }
        match context.cancel {
            Some(cancel) => Self::prepare_metered(
                vertices,
                endpoints.iter().copied(),
                &mut crate::cancel::CancelMeter::new(cancel),
            ),
            None => Self::prepare_metered(
                vertices,
                endpoints.iter().copied(),
                &mut crate::cancel::Unmetered,
            ),
        }
    }

    /// Validate and canonicalize undirected endpoints once.
    pub fn prepare<I>(vertex_count: usize, endpoints: I) -> Result<Self, CmgError>
    where
        I: IntoIterator<Item = (usize, usize)>,
    {
        Self::prepare_metered(vertex_count, endpoints, &mut crate::cancel::Unmetered)
    }

    fn prepare_metered<I, M: crate::cancel::Meter>(
        vertex_count: usize,
        endpoints: I,
        meter: &mut M,
    ) -> Result<Self, CmgError>
    where
        I: IntoIterator<Item = (usize, usize)>,
    {
        let iterator = endpoints.into_iter();
        meter.allocation(
            "prepared topology endpoints",
            iterator.size_hint().0.saturating_mul(16),
        )?;
        let mut indexed = Vec::new();
        indexed
            .try_reserve_exact(iterator.size_hint().0)
            .map_err(|_| CmgError::AllocationFailed {
                context: "prepared topology endpoints",
            })?;
        for (input_index, (left, right)) in iterator.enumerate() {
            meter.work("prepared endpoint validation", 16)?;
            validate_endpoint(vertex_count, left)?;
            validate_endpoint(vertex_count, right)?;
            if left == right {
                return Err(CmgError::SelfLoop { vertex: left });
            }
            let (u, v) = if left < right {
                (left as u32, right as u32)
            } else {
                (right as u32, left as u32)
            };
            if indexed.len() == indexed.capacity() {
                meter.allocation("prepared topology endpoints", 16)?;
                indexed
                    .try_reserve(1)
                    .map_err(|_| CmgError::AllocationFailed {
                        context: "prepared topology endpoints",
                    })?;
            }
            indexed.push((pack_key(u, v), input_index));
        }
        meter.sort_by(&mut indexed, Ord::cmp, "prepared endpoint sort")?;

        let input_edge_count = indexed.len();
        let mut canonical_keys = Vec::new();
        let mut group_offsets = Vec::new();
        let mut group_input_indices = Vec::new();
        let mut input_to_canonical = Vec::new();
        meter.allocation(
            "prepared canonical endpoints",
            crate::storage::bytes::<u64>(input_edge_count)?,
        )?;
        canonical_keys
            .try_reserve_exact(input_edge_count)
            .map_err(|_| CmgError::AllocationFailed {
                context: "prepared canonical endpoints",
            })?;
        meter.allocation(
            "prepared duplicate offsets",
            crate::storage::bytes::<usize>(input_edge_count.saturating_add(1))?,
        )?;
        group_offsets
            .try_reserve_exact(input_edge_count.saturating_add(1))
            .map_err(|_| CmgError::AllocationFailed {
                context: "prepared duplicate offsets",
            })?;
        meter.allocation(
            "prepared duplicate order",
            crate::storage::bytes::<usize>(input_edge_count)?,
        )?;
        group_input_indices
            .try_reserve_exact(input_edge_count)
            .map_err(|_| CmgError::AllocationFailed {
                context: "prepared duplicate order",
            })?;
        meter.allocation(
            "prepared input edge map",
            crate::storage::bytes::<usize>(input_edge_count)?,
        )?;
        input_to_canonical
            .try_reserve_exact(input_edge_count)
            .map_err(|_| CmgError::AllocationFailed {
                context: "prepared input edge map",
            })?;
        crate::cancel::chunked(
            meter,
            "prepared input edge map",
            input_edge_count,
            1,
            |range| {
                input_to_canonical.resize(input_to_canonical.len() + range.len(), 0);
                Ok(())
            },
        )?;
        group_offsets.push(0);

        let mut maximum_duplicate_count = 0;
        let mut cursor = 0;
        while cursor < indexed.len() {
            let key = indexed[cursor].0;
            let canonical_index = canonical_keys.len();
            canonical_keys.push(key);
            let start = cursor;
            while cursor < indexed.len() && indexed[cursor].0 == key {
                meter.work("prepared duplicate grouping", 16)?;
                let input_index = indexed[cursor].1;
                group_input_indices.push(input_index);
                input_to_canonical[input_index] = canonical_index;
                cursor += 1;
            }
            maximum_duplicate_count = maximum_duplicate_count.max(cursor - start);
            group_offsets.push(cursor);
        }

        let components = Components::from_endpoints(
            vertex_count,
            canonical_keys.iter().map(|&key| (key_u(key), key_v(key))),
            meter,
        )?;
        meter.allocation(
            "prepared topology owners",
            crate::storage::arc_bytes::<Components>()?,
        )?;
        let components = Arc::new(components);
        meter.allocation(
            "prepared topology lineage",
            crate::storage::arc_bytes::<()>()?,
        )?;
        let lineage = Arc::new(());
        let topology = Self {
            vertex_count,
            input_edge_count,
            canonical_keys,
            group_offsets,
            group_input_indices,
            input_to_canonical,
            components,
            lineage,
            maximum_duplicate_count,
        };
        meter.checkpoint("prepared topology publication")?;
        Ok(topology)
    }

    /// Return the number of vertices, including isolates.
    #[must_use]
    pub const fn vertex_count(&self) -> usize {
        self.vertex_count
    }

    /// Return the number of original input edges.
    #[must_use]
    pub const fn input_edge_count(&self) -> usize {
        self.input_edge_count
    }

    /// Return the number of canonical undirected edges.
    #[must_use]
    pub fn canonical_edge_count(&self) -> usize {
        self.canonical_keys.len()
    }

    /// Return deterministic connected-component metadata.
    #[must_use]
    pub fn components(&self) -> &Components {
        &self.components
    }

    /// Map each original input edge to its canonical edge index.
    #[must_use]
    pub fn input_edge_to_canonical_edge(&self) -> &[usize] {
        &self.input_to_canonical
    }

    /// Return principal retained bytes for topology, maps, and components.
    #[must_use]
    pub fn retained_bytes(&self) -> usize {
        self.canonical_keys
            .capacity()
            .saturating_mul(core::mem::size_of::<u64>())
            .saturating_add(
                self.group_offsets
                    .capacity()
                    .saturating_mul(core::mem::size_of::<usize>()),
            )
            .saturating_add(
                self.group_input_indices
                    .capacity()
                    .saturating_mul(core::mem::size_of::<usize>()),
            )
            .saturating_add(
                self.input_to_canonical
                    .capacity()
                    .saturating_mul(core::mem::size_of::<usize>()),
            )
            .saturating_add(self.components.byte_len())
    }

    /// Allocate numeric-assembly scratch sized for this topology.
    pub fn workspace(&self) -> Result<PreparedLaplacianWorkspace, CmgError> {
        PreparedLaplacianWorkspace::new(self)
    }

    /// Assemble one changing-weight numeric frame.
    pub fn assemble(&self, weights: &[f64]) -> Result<Laplacian, CmgError> {
        let mut workspace = self.workspace()?;
        self.assemble_with_workspace(weights, &mut workspace)
    }

    /// Assemble one numeric frame while reusing duplicate-summation scratch.
    pub fn assemble_with_workspace(
        &self,
        weights: &[f64],
        workspace: &mut PreparedLaplacianWorkspace,
    ) -> Result<Laplacian, CmgError> {
        self.assemble_metered(weights, workspace, &mut crate::cancel::Unmetered)
    }

    fn assemble_metered<M: crate::cancel::Meter>(
        &self,
        weights: &[f64],
        workspace: &mut PreparedLaplacianWorkspace,
        meter: &mut M,
    ) -> Result<Laplacian, CmgError> {
        use crate::cancel::chunked;
        if weights.len() != self.input_edge_count {
            return Err(CmgError::dimension(
                "PreparedLaplacianTopology weights",
                self.input_edge_count,
                weights.len(),
            ));
        }
        chunked(
            meter,
            "prepared weight validation",
            weights.len(),
            4,
            |range| {
                for (&weight, &canonical_index) in weights[range.clone()]
                    .iter()
                    .zip(&self.input_to_canonical[range])
                {
                    if !weight.is_finite() || weight <= 0.0 {
                        let key = self.canonical_keys[canonical_index];
                        return Err(CmgError::InvalidEdgeWeight {
                            u: key_u(key),
                            v: key_v(key),
                            weight,
                        });
                    }
                }
                Ok(())
            },
        )?;
        workspace.validate(self.maximum_duplicate_count)?;

        meter.allocation(
            "prepared numeric canonical edges",
            crate::storage::bytes::<Edge>(self.canonical_keys.len())?,
        )?;
        let mut edges = Vec::new();
        edges
            .try_reserve_exact(self.canonical_keys.len())
            .map_err(|_| CmgError::AllocationFailed {
                context: "prepared numeric canonical edges",
            })?;
        let mut diagonal = crate::cancel::try_filled(
            meter,
            self.vertex_count,
            0.0_f64,
            "prepared numeric diagonal",
        )?;

        for canonical_index in 0..self.canonical_keys.len() {
            meter.work("prepared numeric edge", 32)?;
            let start = self.group_offsets[canonical_index];
            let end = self.group_offsets[canonical_index + 1];
            let duplicates = &mut workspace.duplicate_weights;
            duplicates.clear();
            chunked(
                meter,
                "prepared duplicate gather",
                end - start,
                4,
                |range| {
                    duplicates.extend(
                        self.group_input_indices[start + range.start..start + range.end]
                            .iter()
                            .map(|&input_index| weights[input_index]),
                    );
                    Ok(())
                },
            )?;
            meter.sort_by(duplicates, f64::total_cmp, "prepared duplicate sort")?;
            let (mut sum, mut correction) = (0.0, 0.0);
            chunked(
                meter,
                "prepared duplicate sum",
                duplicates.len(),
                4,
                |range| {
                    for &value in &duplicates[range] {
                        crate::graph::compensated_add(&mut sum, &mut correction, value);
                    }
                    Ok(())
                },
            )?;
            let weight = sum + correction;
            let key = self.canonical_keys[canonical_index];
            let u = key_u(key);
            let v = key_v(key);
            if !weight.is_finite() || weight <= 0.0 {
                return Err(CmgError::InvalidEdgeWeight { u, v, weight });
            }
            edges.push(Edge::from_internal_parts(u, v, weight)?);
            diagonal[u] += weight;
            diagonal[v] += weight;
        }
        let graph = Laplacian::from_prepared_parts(
            self.vertex_count,
            edges,
            diagonal,
            Arc::clone(&self.lineage),
            Arc::clone(&self.components),
            meter,
        )?;
        meter.checkpoint("prepared numeric frame")?;
        Ok(graph)
    }

    pub(crate) fn matches_graph(&self, graph: &Laplacian) -> bool {
        graph.belongs_to_prepared_topology(&self.lineage, &self.components)
    }
}

/// Caller-owned scratch for repeated prepared numeric assembly.
#[derive(Debug, Clone)]
pub struct PreparedLaplacianWorkspace {
    duplicate_weights: Vec<f64>,
}

impl PreparedLaplacianWorkspace {
    /// Requested duplicate-weight scratch layout for the supplied topology.
    pub fn allocation_bytes(topology: &PreparedLaplacianTopology) -> Result<usize, CmgError> {
        crate::storage::bytes::<f64>(topology.maximum_duplicate_count)
    }

    /// Admit duplicate-weight scratch before its fallible allocation.
    pub fn new_with_budget(
        topology: &PreparedLaplacianTopology,
        budget_bytes: usize,
    ) -> Result<Self, CmgError> {
        Self::new_with(
            topology,
            crate::BuildContext {
                budget_bytes: Some(budget_bytes),
                ..crate::BuildContext::default()
            },
        )
    }

    /// Allocate scratch compatible with a prepared topology.
    pub fn new(topology: &PreparedLaplacianTopology) -> Result<Self, CmgError> {
        Self::new_with(topology, crate::BuildContext::default())
    }

    /// Allocate scratch under a construction context; its team is unused. A
    /// budget admits [`Self::allocation_bytes`] first. With a cancellation
    /// query, the context is polled before the fallible allocation and once
    /// more before the scratch is returned; a cancelled call keeps nothing.
    pub fn new_with(
        topology: &PreparedLaplacianTopology,
        context: crate::BuildContext<'_>,
    ) -> Result<Self, CmgError> {
        if let Some(budget) = context.budget_bytes {
            crate::storage::admit(Self::allocation_bytes(topology)?, budget)?;
        }
        let poll = |phase| match context.cancel {
            Some(cancel) if cancel() => Err(crate::cancel::CancelMeter::cancelled(phase)),
            _ => Ok(()),
        };
        poll("prepared duplicate weights")?;
        let mut duplicate_weights = Vec::new();
        duplicate_weights
            .try_reserve_exact(topology.maximum_duplicate_count)
            .map_err(|_| CmgError::AllocationFailed {
                context: "prepared duplicate weights",
            })?;
        poll("prepared workspace publication")?;
        Ok(Self { duplicate_weights })
    }

    /// Return retained scratch bytes.
    #[must_use]
    pub fn byte_len(&self) -> usize {
        self.duplicate_weights
            .capacity()
            .saturating_mul(core::mem::size_of::<f64>())
    }

    fn validate(&self, required_capacity: usize) -> Result<(), CmgError> {
        if self.duplicate_weights.capacity() < required_capacity {
            return Err(CmgError::InvalidHierarchy {
                context: "prepared Laplacian workspace is too small",
            });
        }
        Ok(())
    }
}

fn validate_endpoint(vertex_count: usize, vertex: usize) -> Result<(), CmgError> {
    if vertex >= vertex_count {
        return Err(CmgError::VertexOutOfBounds {
            vertex,
            vertex_count,
        });
    }
    if vertex > u32::MAX as usize {
        return Err(CmgError::VertexIndexTooWide {
            vertex,
            maximum: u32::MAX as usize,
        });
    }
    Ok(())
}

const fn pack_key(u: u32, v: u32) -> u64 {
    ((u as u64) << 32) | v as u64
}

const fn key_u(key: u64) -> usize {
    (key >> 32) as usize
}

const fn key_v(key: u64) -> usize {
    key as u32 as usize
}

#[cfg(test)]
// Context literals keep `..Default::default()` for their feature-gated fields.
#[allow(clippy::needless_update)]
mod cancellable_assembly_tests {
    use super::*;
    use crate::cancel::StopAt;

    #[test]
    fn long_duplicate_groups_poll_inside_and_frames_keep_their_bits() {
        // One pair repeated 30,000 times plus a path: a single long group.
        let mut endpoints: Vec<(usize, usize)> = (0..30_000).map(|_| (0, 1)).collect();
        endpoints.extend((1..500).map(|v| (v, v + 1)));
        let topology = PreparedLaplacianTopology::prepare(501, endpoints.iter().copied()).unwrap();
        let weights: Vec<f64> = (0..endpoints.len())
            .map(|i| 1.0 + ((i * 37) % 101) as f64 * 1e-3)
            .collect();
        let mut workspace = topology.workspace().unwrap();
        let reference = topology
            .assemble_with_workspace(&weights, &mut workspace)
            .unwrap();
        let never = StopAt::never();
        let query = || never.query();
        let polled = topology
            .assemble_with(
                &weights,
                &mut workspace,
                crate::BuildContext {
                    cancel: Some(&query),
                    budget_bytes: Some(topology.numeric_allocation_bytes().unwrap()),
                    ..crate::BuildContext::default()
                },
            )
            .unwrap();
        assert_eq!(polled.edges(), reference.edges());
        assert_eq!(polled.diagonal(), reference.diagonal());
        let total = never.polls();
        let mut phases = std::collections::BTreeSet::new();
        for stop in 0..total {
            let cancel = StopAt::at(Some(stop));
            let query = || cancel.query();
            match topology.assemble_with(
                &weights,
                &mut workspace,
                crate::BuildContext {
                    cancel: Some(&query),
                    ..crate::BuildContext::default()
                },
            ) {
                Err(CmgError::Cancelled { phase }) => {
                    phases.insert(phase);
                }
                other => panic!("poll {stop} of {total}: {:?}", other.err()),
            }
        }
        for phase in [
            "prepared weight validation",
            "prepared duplicate gather",
            "prepared duplicate sort",
            "prepared duplicate sum",
            "prepared numeric frame",
        ] {
            assert!(phases.contains(phase), "{phase} not in {phases:?}");
        }
        let after = topology
            .assemble_with_workspace(&weights, &mut workspace)
            .unwrap();
        assert_eq!(after.edges(), reference.edges());
    }

    #[test]
    fn topology_preparation_polls_inside_sort_and_grouping() {
        let endpoints: Vec<(usize, usize)> = (0..20_000)
            .map(|i| ((i * 7919) % 3001, (i * 104_729 + 1) % 3001))
            .map(|(u, v)| if u == v { (u, (v + 1) % 3001) } else { (u, v) })
            .collect();
        let reference =
            PreparedLaplacianTopology::prepare(3001, endpoints.iter().copied()).unwrap();
        let never = StopAt::never();
        let query = || never.query();
        let polled = PreparedLaplacianTopology::prepare_with(
            3001,
            &endpoints,
            crate::BuildContext {
                cancel: Some(&query),
                ..crate::BuildContext::default()
            },
        )
        .unwrap();
        assert_eq!(polled.canonical_keys, reference.canonical_keys);
        assert_eq!(polled.group_offsets, reference.group_offsets);
        assert_eq!(polled.group_input_indices, reference.group_input_indices);
        assert_eq!(polled.input_to_canonical, reference.input_to_canonical);
        assert_eq!(*polled.components, *reference.components);
        let total = never.polls();
        let mut phases = std::collections::BTreeSet::new();
        for stop in (0..total).step_by((total / 300).max(1)).chain([total - 1]) {
            let cancel = StopAt::at(Some(stop));
            let query = || cancel.query();
            match PreparedLaplacianTopology::prepare_with(
                3001,
                &endpoints,
                crate::BuildContext {
                    cancel: Some(&query),
                    ..crate::BuildContext::default()
                },
            ) {
                Err(CmgError::Cancelled { phase }) => {
                    phases.insert(phase);
                }
                other => panic!("poll {stop} of {total}: {:?}", other.err()),
            }
        }
        for phase in [
            "prepared endpoint validation",
            "prepared endpoint sort",
            "prepared duplicate grouping",
            "prepared topology publication",
        ] {
            assert!(phases.contains(phase), "{phase} not in {phases:?}");
        }
    }

    /// Every poll of topology preparation and numeric assembly: the four map
    /// reserves each poll first, the diagonal metadata scan is counted, the
    /// shared handles poll before allocation, and the frame polls once more
    /// after publication. Each cancellation keeps nothing; the same topology
    /// and workspace then rebuild the reference exactly.
    #[test]
    fn allocations_and_numeric_publication_tail_cancel_and_rebuild_exactly() {
        type Run<'r> = &'r dyn Fn(crate::Cancellation<'_>) -> Result<(), CmgError>;
        fn total(run: Run<'_>) -> usize {
            let never = StopAt::never();
            run(&|| never.query()).unwrap();
            never.polls()
        }
        fn collect(run: Run<'_>) -> std::collections::BTreeMap<&'static str, usize> {
            let total = total(run);
            let mut phases = std::collections::BTreeMap::new();
            for stop in 0..total {
                let cancel = StopAt::at(Some(stop));
                match run(&|| cancel.query()) {
                    Err(CmgError::Cancelled { phase }) => *phases.entry(phase).or_default() += 1,
                    other => panic!("poll {stop} of {total}: {other:?}"),
                }
                assert_eq!(cancel.polls(), stop + 1);
            }
            phases
        }
        fn context(cancel: crate::Cancellation<'_>) -> crate::BuildContext<'_> {
            crate::BuildContext {
                cancel: Some(cancel),
                ..crate::BuildContext::default()
            }
        }

        let endpoints: Vec<(usize, usize)> = (0..20_000)
            .map(|i| ((i * 7919) % 3001, (i * 104_729 + 1) % 3001))
            .map(|(u, v)| if u == v { (u, (v + 1) % 3001) } else { (u, v) })
            .collect();
        let reference =
            PreparedLaplacianTopology::prepare(3001, endpoints.iter().copied()).unwrap();
        let phases = collect(&|cancel| {
            PreparedLaplacianTopology::prepare_with(3001, &endpoints, context(cancel)).map(drop)
        });
        for phase in [
            "prepared canonical endpoints",
            "prepared duplicate offsets",
            "prepared duplicate order",
            "prepared topology owners",
            "prepared topology lineage",
            "prepared topology publication",
        ] {
            assert_eq!(phases.get(phase), Some(&1), "{phase} in {phases:?}");
        }
        assert!(phases["prepared input edge map"] >= 1, "{phases:?}");
        let again = PreparedLaplacianTopology::prepare_with(
            3001,
            &endpoints,
            crate::BuildContext::default(),
        )
        .unwrap();
        assert_eq!(again.canonical_keys, reference.canonical_keys);
        assert_eq!(again.group_offsets, reference.group_offsets);
        assert_eq!(again.input_to_canonical, reference.input_to_canonical);

        // A long path: the diagonal metadata scan spans several poll intervals.
        let n = 200_000;
        let path: Vec<(usize, usize)> = (1..n).map(|v| (v - 1, v)).collect();
        let topology = PreparedLaplacianTopology::prepare(n, path.iter().copied()).unwrap();
        let weights: Vec<f64> = (1..n).map(|v| 1.0 + (v % 13) as f64 / 8.0).collect();
        let phases = collect(&|cancel| {
            PreparedLaplacianWorkspace::new_with(&topology, context(cancel)).map(drop)
        });
        assert_eq!(
            phases.into_iter().collect::<Vec<_>>(),
            [
                ("prepared duplicate weights", 1),
                ("prepared workspace publication", 1)
            ]
        );
        let scratch = core::cell::RefCell::new(
            PreparedLaplacianWorkspace::new_with(&topology, crate::BuildContext::default())
                .unwrap(),
        );
        let reference = topology
            .assemble_with_workspace(&weights, &mut scratch.borrow_mut())
            .unwrap();
        let phases = collect(&|cancel| {
            topology
                .assemble_with(&weights, &mut scratch.borrow_mut(), context(cancel))
                .map(drop)
        });
        // 200,000 diagonal entries at two units each: six poll intervals.
        assert!(phases["prepared numeric metadata"] >= 5, "{phases:?}");
        for phase in [
            "prepared numeric publication",
            "prepared diagonal publication",
            "prepared numeric lineage",
        ] {
            assert_eq!(phases.get(phase), Some(&1), "{phase} in {phases:?}");
        }
        assert_eq!(phases.get("prepared numeric frame"), Some(&1));
        let again = topology
            .assemble_with_workspace(&weights, &mut scratch.borrow_mut())
            .unwrap();
        assert_eq!(again.edges(), reference.edges());
        assert_eq!(
            again
                .diagonal()
                .iter()
                .map(|d| d.to_bits())
                .collect::<Vec<_>>(),
            reference
                .diagonal()
                .iter()
                .map(|d| d.to_bits())
                .collect::<Vec<_>>()
        );
        assert_eq!(again.matrix_nnz(), reference.matrix_nnz());
        assert_eq!(
            again.operator_norm_bound().to_bits(),
            reference.operator_norm_bound().to_bits()
        );
    }
}
