//! Component-grounded, static-degree-ordered LDL^T terminal solver.

use crate::cancel::{Meter, Progress, Unmetered, chunked};
use crate::storage::{add, bytes};
use crate::{CmgError, Components, Laplacian, ValidationOptions};

fn strict_slots(dimension: usize) -> Result<usize, CmgError> {
    dimension
        .checked_mul(dimension.saturating_sub(1))
        .map(|value| value / 2)
        .ok_or(CmgError::AllocationFailed {
            context: "terminal triangular size overflows",
        })
}

fn dense_bank<M: Meter>(dimension: usize, meter: &mut M) -> Result<Vec<Vec<f64>>, CmgError> {
    let mut rows = meter.try_capacity(dimension, "terminal dense row descriptors")?;
    for _ in 0..dimension {
        meter.work("terminal dense banks", 128)?;
        rows.push(meter.try_filled(dimension, 0.0, "terminal dense row")?);
    }
    Ok(rows)
}

#[cfg(feature = "experimental-components")]
fn compact_fallibly<T: Copy, M: Meter>(values: Vec<T>, meter: &mut M) -> Result<Vec<T>, CmgError> {
    if values.len() == values.capacity() {
        return Ok(values);
    }
    let mut compact = meter.try_capacity(values.len(), "terminal compact conversion")?;
    if M::POLLS {
        for &value in &values {
            meter.work("terminal compaction copy", 8)?;
            compact.push(value);
        }
    } else {
        compact.extend_from_slice(&values);
    }
    Ok(compact)
}

crate::storage::storage_fields!(
    GroundedLdl,
    components,
    anchors,
    permutation,
    lower,
    diagonal
);
impl crate::storage::Storage for LowerFactor {
    fn visit(
        &self,
        visitor: &mut dyn FnMut(crate::storage::Allocation) -> Result<(), CmgError>,
    ) -> Result<(), CmgError> {
        match self {
            Self::Packed { values } => values.visit(visitor),
            Self::Sparse {
                row_offsets,
                columns,
                row_values,
                column_offsets,
                rows,
                column_values,
            } => {
                row_offsets.visit(visitor)?;
                columns.visit(visitor)?;
                row_values.visit(visitor)?;
                column_offsets.visit(visitor)?;
                rows.visit(visitor)?;
                column_values.visit(visitor)
            }
        }
    }
}

/// The empty sum of the toolchain's `Iterator::sum` for `f64` (its fold seed),
/// so segmented corrections reproduce the reference sums bit for bit.
fn empty_sum() -> f64 {
    core::iter::empty::<f64>().sum()
}

#[derive(Debug, Clone, PartialEq)]
enum LowerFactor {
    Packed {
        values: Vec<f64>,
    },
    Sparse {
        row_offsets: Vec<usize>,
        columns: Vec<u32>,
        row_values: Vec<f64>,
        column_offsets: Vec<usize>,
        rows: Vec<u32>,
        column_values: Vec<f64>,
    },
}

impl LowerFactor {
    /// Every lane's strict-row correction, accumulated into `sums` from zero
    /// in canonical entry order. A polling progress counts the row in
    /// segments while it runs; the per-lane accumulation order is unchanged.
    fn batch_correction<P: Progress>(
        &self,
        row: usize,
        input: &[f64],
        columns: usize,
        lanes: core::ops::Range<usize>,
        forward: bool,
        sums: &mut [f64],
        progress: &mut P,
    ) -> Result<(), CmgError> {
        const PHASE: &str = "terminal row correction";
        let sums = &mut sums[lanes.clone()];
        sums.fill(0.0);
        let width = sums.len();
        let mut add_entry = |source: usize, coefficient: f64| {
            for (lane, value) in (lanes.start..).zip(sums.iter_mut()) {
                *value += coefficient * input[source * columns + lane];
            }
        };
        match self {
            Self::Packed { values } if forward => {
                let start = row * row.saturating_sub(1) / 2;
                chunked(progress, PHASE, row, width, |range| {
                    for column in range {
                        add_entry(column, values[start + column]);
                    }
                    Ok(())
                })
            }
            Self::Packed { values } => {
                let mut index = row * (row + 1) / 2 + row;
                let first = row + 1;
                let count = (input.len() / columns).saturating_sub(first);
                chunked(progress, PHASE, count, width, |range| {
                    for later in first + range.start..first + range.end {
                        add_entry(later, values[index]);
                        index += later;
                    }
                    Ok(())
                })
            }
            Self::Sparse {
                row_offsets,
                columns: entries,
                row_values,
                ..
            } if forward => {
                let start = row_offsets[row];
                chunked(
                    progress,
                    PHASE,
                    row_offsets[row + 1] - start,
                    width,
                    |range| {
                        for index in start + range.start..start + range.end {
                            add_entry(entries[index] as usize, row_values[index]);
                        }
                        Ok(())
                    },
                )
            }
            Self::Sparse {
                column_offsets,
                rows,
                column_values,
                ..
            } => {
                let start = column_offsets[row];
                chunked(
                    progress,
                    PHASE,
                    column_offsets[row + 1] - start,
                    width,
                    |range| {
                        for index in start + range.start..start + range.end {
                            add_entry(rows[index] as usize, column_values[index]);
                        }
                        Ok(())
                    },
                )
            }
        }
    }

    fn from_dense<M: Meter>(lower: &[Vec<f64>], meter: &mut M) -> Result<Self, CmgError> {
        let dimension = lower.len();
        let packed_slots = dimension.saturating_mul(dimension.saturating_sub(1)) / 2;
        let strict_nonzeros =
            strict_nonzeros(lower, "terminal count rows", "terminal count values", meter)?;

        let packed_bytes = packed_slots.saturating_mul(core::mem::size_of::<f64>());
        let sparse_bytes = strict_nonzeros
            .saturating_mul(2 * core::mem::size_of::<u32>() + 2 * core::mem::size_of::<f64>())
            .saturating_add(2 * (dimension + 1).saturating_mul(core::mem::size_of::<usize>()));

        if dimension <= u32::MAX as usize && sparse_bytes < packed_bytes {
            let mut row_offsets =
                meter.try_capacity(add(dimension, 1)?, "terminal sparse row offsets")?;
            let mut columns = meter.try_capacity(strict_nonzeros, "terminal sparse columns")?;
            let mut row_values =
                meter.try_capacity(strict_nonzeros, "terminal sparse row values")?;
            row_offsets.push(0);
            for (row, values) in lower.iter().enumerate() {
                meter.work("terminal factor conversion", 128)?;
                for (column, value) in values[..row].iter().copied().enumerate() {
                    meter.work("terminal factor conversion", 128)?;
                    if value != 0.0 {
                        columns.push(column as u32);
                        row_values.push(value);
                    }
                }
                row_offsets.push(columns.len());
            }

            Self::from_sparse_rows(row_offsets, columns, row_values, meter)
        } else {
            let mut values = meter.try_capacity(packed_slots, "terminal packed values")?;
            for (row, dense_row) in lower.iter().enumerate() {
                meter.work("terminal factor conversion", 128)?;
                if M::POLLS {
                    for &value in &dense_row[..row] {
                        meter.work("terminal packed copy", 8)?;
                        values.push(value);
                    }
                } else {
                    values.extend_from_slice(&dense_row[..row]);
                }
            }
            Ok(Self::Packed { values })
        }
    }

    fn from_sparse_rows<M: Meter>(
        row_offsets: Vec<usize>,
        columns: Vec<u32>,
        row_values: Vec<f64>,
        meter: &mut M,
    ) -> Result<Self, CmgError> {
        let dimension = row_offsets.len() - 1;
        let strict_nonzeros = columns.len();
        let mut column_counts = meter.try_filled(dimension, 0_usize, "terminal column counts")?;
        for &column in &columns {
            meter.work("terminal factor conversion", 128)?;
            column_counts[column as usize] += 1;
        }
        let mut column_offsets =
            meter.try_capacity(add(dimension, 1)?, "terminal column offsets")?;
        column_offsets.push(0);
        for count in column_counts {
            meter.work("terminal factor conversion", 128)?;
            column_offsets.push(column_offsets.last().copied().unwrap_or(0) + count);
        }
        let mut next = meter.copy_slice(&column_offsets[..dimension], "terminal column cursors")?;
        let mut rows = meter.try_filled(strict_nonzeros, 0_u32, "terminal transposed rows")?;
        let mut column_values =
            meter.try_filled(strict_nonzeros, 0.0, "terminal transposed values")?;
        for row in 0..dimension {
            meter.work("terminal factor conversion", 128)?;
            for index in row_offsets[row]..row_offsets[row + 1] {
                meter.work("terminal factor conversion", 128)?;
                let column = columns[index] as usize;
                let destination = next[column];
                rows[destination] = row as u32;
                column_values[destination] = row_values[index];
                next[column] += 1;
            }
        }

        Ok(Self::Sparse {
            row_offsets,
            columns,
            row_values,
            column_offsets,
            rows,
            column_values,
        })
    }

    /// Add the forward correction of `row` to `sum` term by term, in the
    /// order of the reference `Iterator::sum`; seed `sum` with [`empty_sum()`].
    /// A polling progress counts the row in segments while it runs.
    fn forward_correction_into<P: Progress>(
        &self,
        row: usize,
        forward: &[f64],
        sum: &mut f64,
        progress: &mut P,
    ) -> Result<(), CmgError> {
        const PHASE: &str = "terminal forward solve";
        match self {
            Self::Packed { values } => {
                let start = row.saturating_mul(row.saturating_sub(1)) / 2;
                chunked(progress, PHASE, row, 1, |range| {
                    *sum = values[start + range.start..start + range.end]
                        .iter()
                        .zip(&forward[range])
                        .map(|(lower_value, previous)| lower_value * previous)
                        .fold(*sum, |total, term| total + term);
                    Ok(())
                })
            }
            Self::Sparse {
                row_offsets,
                columns,
                row_values,
                ..
            } => {
                let start = row_offsets[row];
                chunked(progress, PHASE, row_offsets[row + 1] - start, 1, |range| {
                    *sum = (start + range.start..start + range.end)
                        .map(|index| row_values[index] * forward[columns[index] as usize])
                        .fold(*sum, |total, term| total + term);
                    Ok(())
                })
            }
        }
    }

    /// Add the backward correction of `row` to `sum` as
    /// [`Self::forward_correction_into`] does for the forward solve.
    fn backward_correction_into<P: Progress>(
        &self,
        row: usize,
        solution: &[f64],
        sum: &mut f64,
        progress: &mut P,
    ) -> Result<(), CmgError> {
        const PHASE: &str = "terminal backward solve";
        match self {
            Self::Packed { values } => {
                // Consecutive packed rows differ in length by one. Advance the
                // column position instead of multiplying triangular indices
                // for every coefficient; retain the reference summation order.
                let mut index = row.saturating_mul(row.saturating_add(1)) / 2 + row;
                let first = row + 1;
                let count = solution.len().saturating_sub(first);
                chunked(progress, PHASE, count, 1, |range| {
                    for (later, &value) in solution
                        .iter()
                        .enumerate()
                        .take(first + range.end)
                        .skip(first + range.start)
                    {
                        *sum += values[index] * value;
                        index += later;
                    }
                    Ok(())
                })
            }
            Self::Sparse {
                column_offsets,
                rows,
                column_values,
                ..
            } => {
                let start = column_offsets[row];
                chunked(
                    progress,
                    PHASE,
                    column_offsets[row + 1] - start,
                    1,
                    |range| {
                        *sum = (start + range.start..start + range.end)
                            .map(|index| column_values[index] * solution[rows[index] as usize])
                            .fold(*sum, |total, term| total + term);
                        Ok(())
                    },
                )
            }
        }
    }

    #[cfg(all(test, feature = "experimental-components"))]
    fn forward_correction(&self, row: usize, forward: &[f64]) -> f64 {
        let mut sum = empty_sum();
        self.forward_correction_into(row, forward, &mut sum, &mut crate::cancel::Unmetered)
            .unwrap_or_else(|_| unreachable!());
        sum
    }

    #[cfg(all(test, feature = "experimental-components"))]
    fn backward_correction(&self, row: usize, solution: &[f64]) -> f64 {
        let mut sum = empty_sum();
        self.backward_correction_into(row, solution, &mut sum, &mut crate::cancel::Unmetered)
            .unwrap_or_else(|_| unreachable!());
        sum
    }

    fn byte_len(&self) -> usize {
        match self {
            Self::Packed { values } => values.len().saturating_mul(core::mem::size_of::<f64>()),
            Self::Sparse {
                row_offsets,
                columns,
                row_values,
                column_offsets,
                rows,
                column_values,
            } => row_offsets
                .len()
                .saturating_add(column_offsets.len())
                .saturating_mul(core::mem::size_of::<usize>())
                .saturating_add(
                    columns
                        .len()
                        .saturating_add(rows.len())
                        .saturating_mul(core::mem::size_of::<u32>()),
                )
                .saturating_add(
                    row_values
                        .len()
                        .saturating_add(column_values.len())
                        .saturating_mul(core::mem::size_of::<f64>()),
                ),
        }
    }
}

/// A deterministic direct solver for a graph Laplacian on its quotient space.
///
/// One highest-index vertex is grounded in each connected component. The
/// remaining grounded matrix is ordered by static row nonzero count and then by
/// original vertex index, matching the simple degree-ordering principle used by
/// the upstream terminal factorization.
#[derive(Debug, Clone, PartialEq)]
pub struct GroundedLdl {
    vertex_count: usize,
    components: Components,
    anchors: Vec<usize>,
    permutation: Vec<usize>,
    lower: LowerFactor,
    diagonal: Vec<f64>,
    factor_nonzeros: usize,
}

impl GroundedLdl {
    /// Solve vertex-major lanes in the scalar solve's per-lane order. Rows are
    /// counted by their factor entries when the meter polls.
    pub(crate) fn solve_batch_compatible<M: Meter>(
        &self,
        rhs: &[f64],
        solution: &mut [f64],
        columns: usize,
        forward: &mut [f64],
        factor_solution: &mut [f64],
        meter: &mut M,
    ) -> Result<(), CmgError> {
        // The output row is temporary correction storage until scattering. This
        // avoids another allocation and loads each factor entry once per tile.
        let scratch_len = columns.min(solution.len());
        let dimension = self.active_dimension();
        // Wide batches run each row in lane tiles: every lane keeps its own
        // correction order, and the counted work per tile stays bounded.
        for row in 0..dimension {
            for lanes in crate::cancel::lane_tiles::<M>(columns, 1) {
                let scratch = &mut solution[..scratch_len];
                self.lower.batch_correction(
                    row,
                    forward,
                    columns,
                    lanes.clone(),
                    true,
                    scratch,
                    meter,
                )?;
                meter.items("terminal forward solve", lanes.len())?;
                for lane in lanes {
                    let value = rhs[self.permutation[row] * columns + lane] - scratch[lane];
                    forward[row * columns + lane] = value;
                    #[cfg(feature = "experimental-components")]
                    {
                        factor_solution[row * columns + lane] = value / self.diagonal[row];
                    }
                }
            }
        }
        #[cfg(not(feature = "experimental-components"))]
        crate::cancel::row_lanes(
            meter,
            "terminal diagonal solve",
            dimension,
            columns,
            1,
            #[inline(always)]
            |rows, lanes| {
                for row in rows {
                    for lane in lanes.clone() {
                        forward[row * columns + lane] /= self.diagonal[row];
                    }
                }
                Ok(())
            },
        )?;
        for row in (0..dimension).rev() {
            for lanes in crate::cancel::lane_tiles::<M>(columns, 1) {
                let scratch = &mut solution[..scratch_len];
                self.lower.batch_correction(
                    row,
                    factor_solution,
                    columns,
                    lanes.clone(),
                    false,
                    scratch,
                    meter,
                )?;
                meter.items("terminal backward solve", lanes.len())?;
                for lane in lanes {
                    #[cfg(feature = "experimental-components")]
                    {
                        factor_solution[row * columns + lane] -= scratch[lane];
                    }
                    #[cfg(not(feature = "experimental-components"))]
                    {
                        factor_solution[row * columns + lane] =
                            forward[row * columns + lane] - scratch[lane];
                    }
                }
            }
        }
        crate::cancel::zero(meter, "terminal scatter", solution)?;
        crate::cancel::row_lanes(
            meter,
            "terminal scatter",
            self.permutation.len(),
            columns,
            1,
            #[inline(always)]
            |rows, lanes| {
                for row in rows {
                    let vertex = self.permutation[row];
                    solution[vertex * columns + lanes.start..vertex * columns + lanes.end]
                        .copy_from_slice(
                            &factor_solution
                                [row * columns + lanes.start..row * columns + lanes.end],
                        );
                }
                Ok(())
            },
        )
    }

    /// Upper bound on all new requested heap layouts during ordinary terminal
    /// construction, including component discovery, both dense row banks and
    /// simultaneous packed/sparse conversion. Borrowed graph/allocator overhead
    /// is excluded. The bound uses vertex count, not an empirical multiplier.
    pub fn construction_allocation_bound(vertices: usize) -> Result<usize, CmgError> {
        let n = vertices;
        let square = n.checked_mul(n).ok_or(CmgError::AllocationFailed {
            context: "terminal square size overflows",
        })?;
        let q = strict_slots(n)?;
        // Retained component labels/sizes, anchors, permutation, pattern and
        // factor index: at most six native arrays; is_anchor is a boolean bank.
        let metadata = add(bytes::<[usize; 6]>(n)?, bytes::<bool>(n)?)?;
        let dense = add(bytes::<[f64; 2]>(square)?, bytes::<[Vec<f64>; 2]>(n)?)?;
        let conversion = add(
            add(bytes::<[usize; 2]>(add(n, 1)?)?, bytes::<usize>(n)?)?,
            bytes::<(u32, u32, f64, f64)>(q)?,
        )?;
        let factoring = add(add(add(metadata, dense)?, bytes::<f64>(n)?)?, conversion)?;
        Ok(factoring.max(bytes::<[usize; 4]>(n)?))
    }

    /// Factor under a construction context, admitting the bound of the
    /// selected representation when a budget is given and polling cancellation
    /// at the fixed cadence when a query is given. A team is not used: the
    /// terminal factor is serial. Arithmetic and order match [`Self::factor`]
    /// (or [`Self::factor_by_component`] when `by_component`).
    pub fn factor_with(
        graph: &Laplacian,
        by_component: bool,
        context: crate::BuildContext<'_>,
    ) -> Result<Self, CmgError> {
        #[cfg(not(feature = "experimental-components"))]
        if by_component {
            return Err(CmgError::InvalidOption {
                name: "by_component requires experimental-components",
                value: 1.0,
            });
        }
        if let Some(budget) = context.budget_bytes {
            let bound = if by_component {
                #[cfg(feature = "experimental-components")]
                {
                    Self::component_construction_allocation_bound(
                        graph.vertex_count(),
                        graph.edge_count(),
                    )?
                }
                #[cfg(not(feature = "experimental-components"))]
                unreachable!()
            } else {
                Self::construction_allocation_bound(graph.vertex_count())?
            };
            crate::storage::admit(bound, budget)?;
        }
        match context.cancel {
            Some(cancel) => Self::factor_metered(
                graph,
                by_component,
                &mut crate::cancel::CancelMeter::new(cancel),
            ),
            None => Self::factor_metered(graph, by_component, &mut Unmetered),
        }
    }

    /// Upper bound for the bounded component-factor constructor. Worst-case
    /// fill is triangular in vertices; actual provisioning uses the sum of
    /// per-component triangular sizes and one current block, not one dense
    /// matrix across disconnected components. Runtime/borrowed parents excluded.
    #[cfg(feature = "experimental-components")]
    pub fn component_construction_allocation_bound(
        vertices: usize,
        edges: usize,
    ) -> Result<usize, CmgError> {
        let n = vertices;
        let q = strict_slots(n)?;
        let offset = bytes::<usize>(add(n, 1)?)?;
        let metadata = bytes::<[usize; 4]>(n)?;
        let grouping = add(
            add(add(offset, offset)?, bytes::<[usize; 2]>(n)?)?,
            bytes::<usize>(edges)?,
        )?;
        let factors = add(
            add(bytes::<f64>(n)?, offset)?,
            add(bytes::<u32>(q)?, bytes::<f64>(q)?)?,
        )?;
        let parents = add(add(metadata, grouping)?, factors)?;
        // Input reorder is sequential with the ordered-sparse numeric bank.
        let input = add(add(offset, bytes::<usize>(edges)?)?, bytes::<usize>(n)?)?;
        let numeric = add(
            add(add(offset, offset)?, bytes::<usize>(edges)?)?,
            add(
                add(bytes::<[usize; 2]>(n)?, bytes::<[f64; 2]>(n)?)?,
                add(bytes::<(u32, u32, f64)>(q)?, bytes::<usize>(q)?)?,
            )?,
        )?;
        // Existing row banks coexist with exact compaction, packed conversion,
        // or transposed sparse banks; maximum, not sum, of those alternatives.
        let packed_or_compact = bytes::<f64>(q)?;
        let transposed = add(
            add(offset, bytes::<usize>(n)?)?,
            add(bytes::<u32>(q)?, bytes::<f64>(q)?)?,
        )?;
        let preparation = add(bytes::<[usize; 5]>(n)?, bytes::<bool>(n)?)?;
        Ok(add(
            parents,
            input.max(numeric).max(packed_or_compact).max(transposed),
        )?
        .max(preparation)
        .max(bytes::<[usize; 4]>(n)?))
    }

    /// Factor the graph Laplacian after grounding one vertex per component.
    pub fn factor(graph: &Laplacian) -> Result<Self, CmgError> {
        Self::factor_metered(graph, false, &mut Unmetered)
    }

    /// Factor disconnected blocks independently, preserving each block's
    /// highest-index anchor and static degree order.
    ///
    /// This experimental builder groups the exposed permutation by component.
    /// Ordered sparse updates skip zero factor entries, including for a single
    /// connected block. Retained factors use shared packed or sparse buffers,
    /// without one solver allocation per component.
    #[cfg(feature = "experimental-components")]
    pub fn factor_by_component(graph: &Laplacian) -> Result<Self, CmgError> {
        Self::factor_metered(graph, true, &mut Unmetered)
    }

    /// The single factorization: component grounding, static degree order,
    /// then dense or ordered-sparse component elimination.
    pub(crate) fn factor_metered<M: Meter>(
        graph: &Laplacian,
        by_component: bool,
        meter: &mut M,
    ) -> Result<Self, CmgError> {
        let vertex_count = graph.vertex_count();
        let components = Components::from_endpoints(
            vertex_count,
            graph.edges().iter().map(|e| (e.u(), e.v())),
            meter,
        )?;

        let mut anchors = meter.try_filled(components.count(), 0, "terminal anchors")?;
        for (vertex, &component) in components.labels().iter().enumerate() {
            meter.work("terminal factor preparation", 128)?;
            anchors[component] = vertex;
        }
        let mut is_anchor = meter.try_filled(vertex_count, false, "terminal anchor flags")?;
        for &anchor in &anchors {
            meter.work("terminal factor preparation", 128)?;
            is_anchor[anchor] = true;
        }

        let mut active_vertices = meter.try_capacity(vertex_count, "terminal permutation")?;
        for (vertex, &anchor) in is_anchor.iter().enumerate() {
            meter.work("terminal active vertices", 16)?;
            if !anchor {
                active_vertices.push(vertex);
            }
        }
        let mut pattern_nonzeros =
            meter.try_filled(vertex_count, 0_usize, "terminal pattern counts")?;
        for &vertex in &active_vertices {
            meter.work("terminal factor preparation", 128)?;
            // Every active grounded row retains its positive diagonal.
            pattern_nonzeros[vertex] = 1;
        }
        for edge in graph.edges() {
            meter.work("terminal factor preparation", 128)?;
            if !is_anchor[edge.u()] && !is_anchor[edge.v()] {
                pattern_nonzeros[edge.u()] += 1;
                pattern_nonzeros[edge.v()] += 1;
            }
        }

        // Keys end with the unique vertex, so any correct sort gives one order.
        let mut permutation = active_vertices;
        #[cfg(feature = "experimental-components")]
        if by_component {
            let key = |&vertex: &usize| {
                (
                    components.labels()[vertex],
                    pattern_nonzeros[vertex],
                    vertex,
                )
            };
            meter.sort_by(
                &mut permutation,
                |left, right| key(left).cmp(&key(right)),
                "terminal permutation sort",
            )?;
            drop(is_anchor);
            drop(pattern_nonzeros);
            let (lower, diagonal, factor_nonzeros) =
                factor_components(graph, &components, &permutation, meter)?;
            return Ok(Self {
                vertex_count,
                components,
                anchors,
                permutation,
                lower,
                diagonal,
                factor_nonzeros,
            });
        }
        let _ = by_component;
        let key = |&vertex: &usize| (pattern_nonzeros[vertex], vertex);
        meter.sort_by(
            &mut permutation,
            |left, right| key(left).cmp(&key(right)),
            "terminal permutation sort",
        )?;
        let dimension = permutation.len();

        // Assemble the ordered grounded matrix directly, without first
        // materializing the full graph matrix and a permuted copy.
        let mut matrix = dense_bank(dimension, meter)?;
        let mut factor_index =
            meter.try_filled(vertex_count, usize::MAX, "terminal factor index")?;
        for (factor_vertex, &original_vertex) in permutation.iter().enumerate() {
            meter.work("terminal factor preparation", 128)?;
            factor_index[original_vertex] = factor_vertex;
            matrix[factor_vertex][factor_vertex] = graph.diagonal()[original_vertex];
        }
        for edge in graph.edges() {
            meter.work("terminal factor preparation", 128)?;
            let factor_u = factor_index[edge.u()];
            let factor_v = factor_index[edge.v()];
            if factor_u == usize::MAX || factor_v == usize::MAX {
                continue;
            }
            matrix[factor_u][factor_v] -= edge.weight();
            matrix[factor_v][factor_u] -= edge.weight();
        }

        let (dense_lower, diagonal) = factor_dense(&matrix, &permutation, meter)?;
        let strict_nonzeros = strict_nonzeros(
            &dense_lower,
            "terminal factor count rows",
            "terminal factor count values",
            meter,
        )?;
        let factor_nonzeros = dimension.saturating_add(strict_nonzeros);
        let lower = LowerFactor::from_dense(&dense_lower, meter)?;

        Ok(Self {
            vertex_count,
            components,
            anchors,
            permutation,
            lower,
            diagonal,
            factor_nonzeros,
        })
    }

    /// Return the original graph dimension.
    #[must_use]
    pub const fn vertex_count(&self) -> usize {
        self.vertex_count
    }

    /// Return the grounded vertex in each component.
    #[must_use]
    pub fn anchors(&self) -> &[usize] {
        &self.anchors
    }

    /// Return factor-order positions as original graph vertex indices.
    #[must_use]
    pub fn permutation(&self) -> &[usize] {
        &self.permutation
    }

    /// Return the dimension of the grounded positive-definite system.
    #[must_use]
    pub fn active_dimension(&self) -> usize {
        self.permutation.len()
    }

    /// Return the number of nonzeros in the unit lower factor.
    ///
    /// This is the denominator used by the upstream repeat heuristic before a
    /// direct terminal level.
    #[must_use]
    pub const fn factor_nonzeros(&self) -> usize {
        self.factor_nonzeros
    }

    /// Return the principal heap bytes retained by the factorization.
    #[must_use]
    pub fn byte_len(&self) -> usize {
        self.anchors
            .len()
            .saturating_add(self.permutation.len())
            .saturating_mul(core::mem::size_of::<usize>())
            .saturating_add(
                self.diagonal
                    .len()
                    .saturating_mul(core::mem::size_of::<f64>()),
            )
            .saturating_add(self.lower.byte_len())
            .saturating_add(
                self.components
                    .labels()
                    .len()
                    .saturating_add(self.components.sizes().len())
                    .saturating_mul(core::mem::size_of::<usize>()),
            )
    }

    /// Solve a compatible Laplacian system using default validation tolerances.
    pub fn solve(&self, rhs: &[f64]) -> Result<Vec<f64>, CmgError> {
        self.solve_with_validation(rhs, ValidationOptions::default())
    }

    /// Solve a compatible Laplacian system with explicit validation tolerances.
    ///
    /// The returned gauge sets every component anchor to zero.
    pub fn solve_with_validation(
        &self,
        rhs: &[f64],
        options: ValidationOptions,
    ) -> Result<Vec<f64>, CmgError> {
        if rhs.len() != self.vertex_count {
            return Err(CmgError::dimension(
                "GroundedLdl::solve",
                self.vertex_count,
                rhs.len(),
            ));
        }
        self.components.validate_rhs(rhs, options)?;
        let mut solution = vec![0.0; self.vertex_count];
        let mut forward = vec![0.0; self.active_dimension()];
        let mut factor_solution = vec![0.0; self.active_dimension()];
        self.solve_into_compatible(
            rhs,
            &mut solution,
            &mut forward,
            &mut factor_solution,
            &mut Unmetered,
        )?;
        Ok(solution)
    }

    pub(crate) fn solve_into_compatible<M: Meter>(
        &self,
        rhs: &[f64],
        solution: &mut [f64],
        forward: &mut [f64],
        factor_solution: &mut [f64],
        meter: &mut M,
    ) -> Result<(), CmgError> {
        if rhs.len() != self.vertex_count {
            return Err(CmgError::dimension(
                "GroundedLdl::solve_into rhs",
                self.vertex_count,
                rhs.len(),
            ));
        }
        if solution.len() != self.vertex_count {
            return Err(CmgError::dimension(
                "GroundedLdl::solve_into solution",
                self.vertex_count,
                solution.len(),
            ));
        }
        let dimension = self.active_dimension();
        if forward.len() != dimension {
            return Err(CmgError::dimension(
                "GroundedLdl::solve_into forward",
                dimension,
                forward.len(),
            ));
        }
        if factor_solution.len() != dimension {
            return Err(CmgError::dimension(
                "GroundedLdl::solve_into factor solution",
                dimension,
                factor_solution.len(),
            ));
        }

        #[cfg(feature = "experimental-components")]
        {
            for row in 0..dimension {
                let mut correction = empty_sum();
                self.lower
                    .forward_correction_into(row, forward, &mut correction, meter)?;
                meter.items("terminal forward solve", 1)?;
                let value = rhs[self.permutation[row]] - correction;
                // Later forward rows need the unscaled value. The existing
                // second scratch holds D^-1 y until its backward row is solved.
                forward[row] = value;
                factor_solution[row] = value / self.diagonal[row];
            }
            for row in (0..dimension).rev() {
                let mut correction = empty_sum();
                self.lower.backward_correction_into(
                    row,
                    factor_solution,
                    &mut correction,
                    meter,
                )?;
                meter.items("terminal backward solve", 1)?;
                factor_solution[row] -= correction;
            }
        }
        #[cfg(not(feature = "experimental-components"))]
        {
            for row in 0..dimension {
                let mut correction = empty_sum();
                self.lower
                    .forward_correction_into(row, forward, &mut correction, meter)?;
                meter.items("terminal forward solve", 1)?;
                forward[row] = rhs[self.permutation[row]] - correction;
            }
            chunked(meter, "terminal diagonal solve", dimension, 1, |range| {
                for (value, pivot) in forward[range.clone()].iter_mut().zip(&self.diagonal[range]) {
                    *value /= *pivot;
                }
                Ok(())
            })?;
            for row in (0..dimension).rev() {
                let mut correction = empty_sum();
                self.lower.backward_correction_into(
                    row,
                    factor_solution,
                    &mut correction,
                    meter,
                )?;
                meter.items("terminal backward solve", 1)?;
                factor_solution[row] = forward[row] - correction;
            }
        }

        crate::cancel::zero(meter, "terminal scatter", solution)?;
        chunked(
            meter,
            "terminal scatter",
            self.permutation.len(),
            1,
            |range| {
                for factor_index in range {
                    solution[self.permutation[factor_index]] = factor_solution[factor_index];
                }
                Ok(())
            },
        )
    }
}

fn factor_dense<M: Meter>(
    matrix: &[Vec<f64>],
    permutation: &[usize],
    meter: &mut M,
) -> Result<(Vec<Vec<f64>>, Vec<f64>), CmgError> {
    let dimension = permutation.len();
    let mut dense_lower = dense_bank(dimension, meter)?;
    let mut diagonal = meter.try_filled(dimension, 0.0, "terminal diagonal")?;
    for (row, values) in dense_lower.iter_mut().enumerate() {
        meter.work("terminal dense numeric", 128)?;
        values[row] = 1.0;
    }

    for column in 0..dimension {
        meter.work("terminal dense numeric", 128)?;
        let mut pivot = matrix[column][column];
        for (previous, diagonal_value) in diagonal.iter().copied().enumerate().take(column) {
            meter.work("terminal dense numeric", 128)?;
            let value = dense_lower[column][previous];
            pivot -= value * value * diagonal_value;
        }
        if !pivot.is_finite() || pivot <= 0.0 {
            return Err(CmgError::NonPositivePivot {
                vertex: permutation[column],
                value: pivot,
            });
        }
        diagonal[column] = pivot;

        for row in (column + 1)..dimension {
            meter.work("terminal dense numeric", 128)?;
            let mut value = matrix[row][column];
            for (previous, diagonal_value) in diagonal.iter().copied().enumerate().take(column) {
                meter.work("terminal dense numeric", 128)?;
                value -=
                    dense_lower[row][previous] * dense_lower[column][previous] * diagonal_value;
            }
            dense_lower[row][column] = value / pivot;
        }
    }

    Ok((dense_lower, diagonal))
}

#[cfg(feature = "experimental-components")]
fn factor_components<M: Meter>(
    graph: &Laplacian,
    components: &Components,
    permutation: &[usize],
    meter: &mut M,
) -> Result<(LowerFactor, Vec<f64>, usize), CmgError> {
    let dimension = permutation.len();
    if dimension <= 1 {
        let diagonal = if let Some(&vertex) = permutation.first() {
            let pivot = graph.diagonal()[vertex];
            if !pivot.is_finite() || pivot <= 0.0 {
                return Err(CmgError::NonPositivePivot {
                    vertex,
                    value: pivot,
                });
            }
            meter.try_filled(1, pivot, "terminal singleton pivot")?
        } else {
            Vec::new()
        };
        return Ok((
            LowerFactor::Packed { values: Vec::new() },
            diagonal,
            dimension,
        ));
    }
    let count = components.count();
    let mut offsets = meter.try_capacity(add(count, 1)?, "terminal component offsets")?;
    offsets.push(0);
    for &size in components.sizes() {
        meter.work("terminal component factor", 128)?;
        offsets.push(offsets.last().unwrap() + size.saturating_sub(1));
    }
    let mut factor_index = meter.try_filled(
        graph.vertex_count(),
        usize::MAX,
        "terminal component factor index",
    )?;
    for (index, &vertex) in permutation.iter().enumerate() {
        meter.work("terminal component factor", 128)?;
        factor_index[vertex] = index;
    }
    // Group only grounded edges with flat counting-sort storage. Do not scan
    // every edge again for every component, or allocate a graph per block.
    let mut edge_offsets =
        meter.try_filled(add(count, 1)?, 0usize, "terminal component edge offsets")?;
    for edge in graph.edges() {
        meter.work("terminal component factor", 128)?;
        if factor_index[edge.u()] != usize::MAX && factor_index[edge.v()] != usize::MAX {
            edge_offsets[components.labels()[edge.u()] + 1] += 1;
        }
    }
    for c in 0..count {
        meter.work("terminal component factor", 128)?;
        edge_offsets[c + 1] += edge_offsets[c];
    }
    let mut next = meter.copy_slice(&edge_offsets[..count], "terminal component edge cursors")?;
    let mut edge_indices = meter.try_filled(
        edge_offsets[count],
        0usize,
        "terminal component edge indices",
    )?;
    for (index, edge) in graph.edges().iter().enumerate() {
        meter.work("terminal component factor", 128)?;
        if factor_index[edge.u()] != usize::MAX && factor_index[edge.v()] != usize::MAX {
            let c = components.labels()[edge.u()];
            edge_indices[next[c]] = index;
            next[c] += 1;
        }
    }
    let mut capacity = 0;
    for &size in components.sizes() {
        meter.work("terminal component capacity", 32)?;
        capacity = add(capacity, strict_slots(size.saturating_sub(1))?)?;
    }
    let mut factors = ComponentFactorRows {
        diagonal: meter.try_capacity(dimension, "terminal component diagonal")?,
        row_offsets: meter.try_capacity(add(dimension, 1)?, "terminal component row offsets")?,
        columns: meter.try_capacity(capacity, "terminal component bounded columns")?,
        values: meter.try_capacity(capacity, "terminal component bounded values")?,
    };
    factors.row_offsets.push(0);
    for component in 0..count {
        meter.work("terminal component factor", 128)?;
        let start = offsets[component];
        let end = offsets[component + 1];
        let size = end - start;
        if size == 0 {
            continue;
        }
        if size == 1 {
            let vertex = permutation[start];
            let pivot = graph.diagonal()[vertex];
            if !pivot.is_finite() || pivot <= 0.0 {
                return Err(CmgError::NonPositivePivot {
                    vertex,
                    value: pivot,
                });
            }
            factors.diagonal.push(pivot);
            factors.row_offsets.push(factors.columns.len());
            continue;
        }
        factors.append_block(
            graph,
            &permutation[start..end],
            &factor_index,
            &edge_indices[edge_offsets[component]..edge_offsets[component + 1]],
            start,
            meter,
        )?;
    }
    let ComponentFactorRows {
        diagonal,
        row_offsets,
        mut columns,
        mut values,
    } = factors;
    let factor_nonzeros = dimension + values.len();
    // byte_len reports principal factor storage; discard growth slack before
    // these buffers become retained state (the legacy dense path sizes exactly).
    columns = compact_fallibly(columns, meter)?;
    values = compact_fallibly(values, meter)?;
    let packed_slots = dimension.saturating_mul(dimension.saturating_sub(1)) / 2;
    let sparse_bytes = values
        .len()
        .saturating_mul(24)
        .saturating_add((dimension + 1).saturating_mul(2 * core::mem::size_of::<usize>()));
    let lower = if packed_slots.saturating_mul(8) <= sparse_bytes {
        let mut packed =
            meter.try_filled(packed_slots, 0.0, "terminal component packed conversion")?;
        for row in 0..dimension {
            meter.work("terminal component factor", 128)?;
            let start = row.saturating_mul(row.saturating_sub(1)) / 2;
            for index in row_offsets[row]..row_offsets[row + 1] {
                meter.work("terminal component factor", 128)?;
                packed[start + columns[index] as usize] = values[index];
            }
        }
        LowerFactor::Packed { values: packed }
    } else {
        LowerFactor::from_sparse_rows(row_offsets, columns, values, meter)?
    };
    Ok((lower, diagonal, factor_nonzeros))
}

// Nonzero columns support ordered left-looking updates; row links visit prior
// columns in exactly the dense reference's increasing-column arithmetic order.
#[cfg(feature = "experimental-components")]
struct ComponentFactorRows {
    diagonal: Vec<f64>,
    row_offsets: Vec<usize>,
    columns: Vec<u32>,
    values: Vec<f64>,
}

#[cfg(feature = "experimental-components")]
impl ComponentFactorRows {
    fn append_block<M: Meter>(
        &mut self,
        graph: &Laplacian,
        permutation: &[usize],
        factor_index: &[usize],
        edge_indices: &[usize],
        base: usize,
        meter: &mut M,
    ) -> Result<(), CmgError> {
        let dimension = permutation.len();
        let mut input_offsets =
            meter.try_filled(add(dimension, 1)?, 0usize, "terminal block input offsets")?;
        for &index in edge_indices {
            meter.work("terminal sparse numeric", 128)?;
            let edge = graph.edges()[index];
            let column = factor_index[edge.u()].min(factor_index[edge.v()]) - base;
            input_offsets[column + 1] += 1;
        }
        for column in 0..dimension {
            meter.work("terminal sparse numeric", 128)?;
            input_offsets[column + 1] += input_offsets[column];
        }
        let mut input_edges =
            meter.try_filled(edge_indices.len(), 0usize, "terminal block input edges")?;
        {
            let mut next =
                meter.copy_slice(&input_offsets[..dimension], "terminal block input cursors")?;
            for &index in edge_indices {
                meter.work("terminal sparse numeric", 128)?;
                let edge = graph.edges()[index];
                let column = factor_index[edge.u()].min(factor_index[edge.v()]) - base;
                input_edges[next[column]] = index;
                next[column] += 1;
            }
        }
        // The arithmetic scan reads only rows and values. Keep the row-link
        // bookkeeping in separate arrays so it does not occupy that scan's
        // cache lines.
        let capacity = strict_slots(dimension)?;
        let mut entry_rows = meter.try_capacity(capacity, "terminal block entry rows")?;
        let mut entry_columns = meter.try_capacity(capacity, "terminal block entry columns")?;
        let mut entry_values = meter.try_capacity(capacity, "terminal block entry values")?;
        let mut next_in_row = meter.try_capacity(capacity, "terminal block row links")?;
        let mut column_offsets =
            meter.try_capacity(add(dimension, 1)?, "terminal block column offsets")?;
        column_offsets.push(0);
        let mut first = meter.try_filled(dimension, usize::MAX, "terminal block first links")?;
        let mut last = meter.try_filled(dimension, usize::MAX, "terminal block last links")?;
        let mut work = meter.try_filled(dimension, 0.0, "terminal block numeric work")?;
        let mut diagonal = meter.try_filled(dimension, 0.0, "terminal block diagonal")?;
        for column in 0..dimension {
            meter.work("terminal sparse numeric", 128)?;
            if M::POLLS {
                for value in &mut work[column + 1..] {
                    meter.work("terminal sparse clear", 8)?;
                    *value = 0.0;
                }
            } else {
                work[column + 1..].fill(0.0);
            }
            for &index in &input_edges[input_offsets[column]..input_offsets[column + 1]] {
                meter.work("terminal sparse numeric", 128)?;
                let edge = graph.edges()[index];
                let row = factor_index[edge.u()].max(factor_index[edge.v()]) - base;
                work[row] -= edge.weight();
            }
            let mut pivot = graph.diagonal()[permutation[column]];
            let mut link = first[column];
            while link != usize::MAX {
                meter.work("terminal sparse numeric", 128)?;
                let previous = entry_values[link];
                let k = entry_columns[link] as usize;
                pivot -= previous * previous * diagonal[k];
                // Column rows are increasing, so the suffix after this link
                // contains precisely the rows below the current pivot.
                let end = column_offsets[k + 1];
                for (&row, &value) in entry_rows[link + 1..end]
                    .iter()
                    .zip(&entry_values[link + 1..end])
                {
                    meter.work("terminal sparse numeric", 128)?;
                    work[row as usize] -= value * previous * diagonal[k];
                }
                link = next_in_row[link];
            }
            if !pivot.is_finite() || pivot <= 0.0 {
                return Err(CmgError::NonPositivePivot {
                    vertex: permutation[column],
                    value: pivot,
                });
            }
            diagonal[column] = pivot;
            for row in column + 1..dimension {
                meter.work("terminal sparse numeric", 128)?;
                if work[row] == 0.0 {
                    continue;
                }
                let value = work[row] / pivot;
                if !value.is_finite() {
                    return Err(CmgError::NonFiniteMatrixValue {
                        row: permutation[row],
                        column: permutation[column],
                        value,
                    });
                }
                if value != 0.0 {
                    let index = entry_values.len();
                    entry_rows.push(row as u32);
                    entry_columns.push(column as u32);
                    entry_values.push(value);
                    next_in_row.push(usize::MAX);
                    if last[row] == usize::MAX {
                        first[row] = index;
                    } else {
                        next_in_row[last[row]] = index;
                    }
                    last[row] = index;
                }
            }
            column_offsets.push(entry_values.len());
        }
        if M::POLLS {
            for value in diagonal {
                meter.work("terminal sparse diagonal copy", 8)?;
                self.diagonal.push(value);
            }
        } else {
            self.diagonal.extend(diagonal);
        }
        for mut link in first {
            meter.work("terminal sparse numeric", 128)?;
            while link != usize::MAX {
                meter.work("terminal sparse numeric", 128)?;
                self.columns
                    .push((base + entry_columns[link] as usize) as u32);
                self.values.push(entry_values[link]);
                link = next_in_row[link];
            }
            self.row_offsets.push(self.columns.len());
        }
        Ok(())
    }
}

/// Strict lower-triangle nonzero count of a dense row bank.
fn strict_nonzeros<M: Meter>(
    lower: &[Vec<f64>],
    row_phase: &'static str,
    value_phase: &'static str,
    meter: &mut M,
) -> Result<usize, CmgError> {
    let mut count = 0;
    for (row, values) in lower.iter().enumerate() {
        meter.work(row_phase, 8)?;
        for &value in &values[..row] {
            meter.work(value_phase, 8)?;
            count += usize::from(value != 0.0);
        }
    }
    Ok(count)
}

#[cfg(all(test, feature = "experimental-components"))]
mod scaled_forward_tests {
    use super::*;

    fn reference(factor: &GroundedLdl, rhs: &[f64]) -> Vec<f64> {
        let n = factor.active_dimension();
        let mut forward = vec![0.0; n];
        let mut backward = vec![0.0; n];
        for row in 0..n {
            forward[row] =
                rhs[factor.permutation[row]] - factor.lower.forward_correction(row, &forward);
        }
        for (value, pivot) in forward.iter_mut().zip(&factor.diagonal) {
            *value /= pivot;
        }
        for row in (0..n).rev() {
            backward[row] = forward[row] - factor.lower.backward_correction(row, &backward);
        }
        let mut solution = vec![0.0; factor.vertex_count()];
        for (i, &vertex) in factor.permutation.iter().enumerate() {
            solution[vertex] = backward[i];
        }
        solution
    }

    #[test]
    fn fused_scaling_matches_packed_sparse_and_reused_scratch() {
        let mut saw_packed = false;
        let mut saw_sparse = false;
        for n in [0usize, 1, 2, 7, 65] {
            for dense in [false, true] {
                for scale in [1e-100, 1.0, 1e100] {
                    let edges = (0..n).flat_map(|u| {
                        ((u + 1)..n)
                            .filter(move |&v| dense || v == u + 1)
                            .map(move |v| (u, v, scale * (0.5 + ((u * 7 + v * 11) % 17) as f64)))
                    });
                    let graph = Laplacian::from_edges(n + 3, edges).unwrap();
                    for factor in [
                        GroundedLdl::factor(&graph).unwrap(),
                        GroundedLdl::factor_by_component(&graph).unwrap(),
                    ] {
                        saw_packed |= matches!(factor.lower, LowerFactor::Packed { .. });
                        saw_sparse |= matches!(factor.lower, LowerFactor::Sparse { .. });
                        let mut forward = vec![f64::NAN; factor.active_dimension()];
                        let mut backward = forward.clone();
                        let mut actual = vec![f64::NAN; graph.vertex_count()];
                        for offset in [0, 5, 0] {
                            let target: Vec<_> = (0..graph.vertex_count())
                                .map(|v| ((v * 13 + offset) % 31) as f64 - 15.0)
                                .collect();
                            let rhs = graph.matvec(&target).unwrap();
                            let expected = reference(&factor, &rhs);
                            factor
                                .solve_into_compatible(
                                    &rhs,
                                    &mut actual,
                                    &mut forward,
                                    &mut backward,
                                    &mut Unmetered,
                                )
                                .unwrap();
                            assert_eq!(
                                actual.iter().map(|x| x.to_bits()).collect::<Vec<_>>(),
                                expected.iter().map(|x| x.to_bits()).collect::<Vec<_>>()
                            );
                        }
                    }
                }
            }
        }
        assert!(saw_packed && saw_sparse);
    }
}

#[cfg(all(test, feature = "experimental-components"))]
mod controlled_factor_tests {
    use super::*;
    use crate::cancel::{CancelMeter, StopAt, Unmetered};

    fn float_bits(values: &[f64]) -> Vec<u64> {
        values.iter().map(|value| value.to_bits()).collect()
    }

    fn assert_factor_bits(actual: &GroundedLdl, expected: &GroundedLdl) {
        assert_eq!(actual, expected);
        assert_eq!(float_bits(&actual.diagonal), float_bits(&expected.diagonal));
        match (&actual.lower, &expected.lower) {
            (LowerFactor::Packed { values: a }, LowerFactor::Packed { values: b }) => {
                assert_eq!(float_bits(a), float_bits(b));
            }
            (
                LowerFactor::Sparse {
                    row_values: a,
                    column_values: at,
                    ..
                },
                LowerFactor::Sparse {
                    row_values: b,
                    column_values: bt,
                    ..
                },
            ) => {
                assert_eq!(float_bits(a), float_bits(b));
                assert_eq!(float_bits(at), float_bits(bt));
            }
            _ => panic!("factor representations differ"),
        }
    }

    fn build(
        graph: &Laplacian,
        blocks: bool,
        meter: &mut CancelMeter<'_>,
    ) -> Result<GroundedLdl, CmgError> {
        GroundedLdl::factor_metered(graph, blocks, meter)
    }

    #[test]
    fn metered_factors_match_unmetered_bits() {
        for n in [0_usize, 1, 2, 6, 24, 65] {
            for dense in [false, true] {
                for disconnected in [false, true] {
                    let stride = if disconnected { 3 } else { 1 };
                    for scale in [1e-90, 1.0, 1e90] {
                        let edges = (0..n).flat_map(|u| {
                            ((u + 1)..n)
                                .filter(move |&v| {
                                    u % stride == v % stride && (dense || v == u + stride)
                                })
                                .map(move |v| {
                                    (u, v, scale * (0.5 + ((u * 7 + v * 11) % 17) as f64))
                                })
                        });
                        let graph = Laplacian::from_edges(n + 2, edges).unwrap();
                        for blocks in [false, true] {
                            let never = || false;
                            let mut meter = CancelMeter::new(&never);
                            let actual = build(&graph, blocks, &mut meter).unwrap();
                            let expected =
                                GroundedLdl::factor_metered(&graph, blocks, &mut Unmetered)
                                    .unwrap();
                            assert_factor_bits(&actual, &expected);
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn controlled_factor_cancels_at_every_small_graph_poll() {
        let graph =
            Laplacian::from_edges(4, [(0, 1, 2.0), (1, 2, 3.0), (0, 2, 0.5), (2, 3, 4.0)]).unwrap();
        for blocks in [false, true] {
            let count = StopAt::never();
            let query = || count.query();
            build(&graph, blocks, &mut CancelMeter::every_operation(&query)).unwrap();
            assert!(count.polls() > 4);
            for stop in 0..count.polls() {
                let cancel = StopAt::at(Some(stop));
                let query = || cancel.query();
                assert!(matches!(
                    build(&graph, blocks, &mut CancelMeter::every_operation(&query)),
                    Err(CmgError::Cancelled { .. })
                ));
                assert_eq!(cancel.polls(), stop + 1);
            }
        }
    }
}

/// Long factor rows are counted while they are traversed: a cancellation at
/// any segment leaves exactly the earlier terms summed, and uncancelled rows
/// equal the reference `Iterator::sum` bit for bit.
#[cfg(test)]
mod long_row_cancellation_tests {
    use super::{LowerFactor, empty_sum};
    use crate::cancel::{Recorder, Unmetered};

    const ENTRIES: usize = 100_000;

    fn long_sparse_row(value: impl Fn(usize) -> f64) -> LowerFactor {
        // Row ENTRIES of a lower factor has every earlier column.
        let mut row_offsets = vec![0; ENTRIES + 1];
        row_offsets.push(ENTRIES);
        let columns: Vec<u32> = (0..ENTRIES as u32).collect();
        let row_values: Vec<f64> = (0..ENTRIES).map(value).collect();
        LowerFactor::from_sparse_rows(row_offsets, columns, row_values, &mut Unmetered).unwrap()
    }

    #[test]
    fn forward_and_batch_rows_poll_inside_and_keep_reference_bits() {
        let factor = long_sparse_row(|i| 1.0 + (i % 7) as f64 * 0.25);
        let unit = long_sparse_row(|_| 1.0);
        let LowerFactor::Sparse { row_values, .. } = &factor else {
            panic!("sparse factor");
        };
        let forward: Vec<f64> = (0..=ENTRIES)
            .map(|i| ((i * 31 % 97) as f64).sin())
            .collect();
        let reference: f64 = (0..ENTRIES).map(|i| row_values[i] * forward[i]).sum();
        let mut sum = empty_sum();
        let mut full = Recorder::default();
        factor
            .forward_correction_into(ENTRIES, &forward, &mut sum, &mut full)
            .unwrap();
        assert_eq!(sum.to_bits(), reference.to_bits());
        assert_eq!(full.advances, ENTRIES.div_ceil(1024));
        assert!(full.max_units <= 1024);
        let ones = vec![1.0; ENTRIES + 1];
        for stop in [1, 30, full.advances - 1] {
            let mut sum = empty_sum();
            let mut cancelled = Recorder {
                stop_at: Some(stop),
                ..Recorder::default()
            };
            assert!(
                unit.forward_correction_into(ENTRIES, &ones, &mut sum, &mut cancelled)
                    .is_err()
            );
            assert_eq!(sum, (stop * 1024) as f64);
            // The interleaved kernel leaves its partial lanes in `sums`.
            let lanes = 3;
            let interleaved = vec![1.0; (ENTRIES + 1) * lanes];
            let mut sums = vec![f64::NAN; lanes];
            let mut cancelled = Recorder {
                stop_at: Some(stop),
                ..Recorder::default()
            };
            assert!(
                unit.batch_correction(
                    ENTRIES,
                    &interleaved,
                    lanes,
                    0..lanes,
                    true,
                    &mut sums,
                    &mut cancelled
                )
                .is_err()
            );
            let per_segment = 1024 / lanes;
            assert_eq!(sums, vec![(stop * per_segment) as f64; lanes]);
        }
    }

    /// A batch wider than one counted segment traverses the long row once per
    /// lane tile: every count covers at most 1024 lane-entry updates, the
    /// counts are deterministic, and a cancellation at the first count leaves
    /// only the first tile's first entry applied.
    #[test]
    fn wide_batch_rows_are_counted_in_lane_tiles() {
        let unit = long_sparse_row(|_| 1.0);
        let lanes = 1_100;
        let tiles: Vec<_> = crate::cancel::lane_tiles::<Recorder>(lanes, 1).collect();
        assert_eq!(tiles, [0..1024, 1024..1100]);
        let interleaved = vec![1.0; (ENTRIES + 1) * lanes];
        let mut sums = vec![f64::NAN; lanes];
        let mut full = Recorder::default();
        for tile in &tiles {
            unit.batch_correction(
                ENTRIES,
                &interleaved,
                lanes,
                tile.clone(),
                true,
                &mut sums,
                &mut full,
            )
            .unwrap();
        }
        assert_eq!(sums, vec![ENTRIES as f64; lanes]);
        assert!(full.max_units <= 1024, "{}", full.max_units);
        assert_eq!(full.units, ENTRIES * lanes);
        // One entry per segment in the full tile, thirteen in the 76-lane tile.
        assert_eq!(full.advances, ENTRIES + ENTRIES.div_ceil(13));
        let mut sums = vec![f64::NAN; lanes];
        let mut cancelled = Recorder {
            stop_at: Some(1),
            ..Recorder::default()
        };
        assert!(
            unit.batch_correction(
                ENTRIES,
                &interleaved,
                lanes,
                tiles[0].clone(),
                true,
                &mut sums,
                &mut cancelled,
            )
            .is_err()
        );
        assert_eq!(sums[..1024], [1.0; 1024]);
        assert!(sums[1024..].iter().all(|value| value.is_nan()));
    }

    #[test]
    fn packed_backward_rows_poll_inside_and_keep_reference_bits() {
        let dimension: usize = 3_000;
        let packed = dimension * (dimension - 1) / 2;
        let factor = LowerFactor::Packed {
            values: (0..packed)
                .map(|i| ((i % 13) as f64 - 6.0) * 0.125)
                .collect(),
        };
        let LowerFactor::Packed { values } = &factor else {
            unreachable!()
        };
        let solution: Vec<f64> = (0..dimension)
            .map(|i| ((i * 17 % 29) as f64).cos())
            .collect();
        let mut index = 0;
        let reference: f64 = solution
            .iter()
            .enumerate()
            .skip(1)
            .map(|(later, &value)| {
                let product = values[index] * value;
                index += later;
                product
            })
            .sum();
        let mut sum = empty_sum();
        let mut full = Recorder::default();
        factor
            .backward_correction_into(0, &solution, &mut sum, &mut full)
            .unwrap();
        assert_eq!(sum.to_bits(), reference.to_bits());
        assert_eq!(full.advances, (dimension - 1).div_ceil(1024));
        let mut sum = empty_sum();
        let mut cancelled = Recorder {
            stop_at: Some(1),
            ..Recorder::default()
        };
        assert!(
            factor
                .backward_correction_into(0, &solution, &mut sum, &mut cancelled)
                .is_err()
        );
        let mut index = 0;
        let first: f64 = (1..=1024)
            .map(|later| {
                let product = values[index] * solution[later];
                index += later;
                product
            })
            .sum();
        assert_eq!(sum.to_bits(), first.to_bits());
    }
}
