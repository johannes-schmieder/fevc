//! Component-grounded, static-degree-ordered LDL^T terminal solver.

use crate::{CmgError, Components, Laplacian, ValidationOptions};

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
    fn from_dense(lower: &[Vec<f64>]) -> Self {
        let dimension = lower.len();
        let packed_slots = dimension.saturating_mul(dimension.saturating_sub(1)) / 2;
        let strict_nonzeros = lower
            .iter()
            .enumerate()
            .map(|(row, values)| values[..row].iter().filter(|value| **value != 0.0).count())
            .sum::<usize>();

        let packed_bytes = packed_slots.saturating_mul(core::mem::size_of::<f64>());
        let sparse_bytes = strict_nonzeros
            .saturating_mul(2 * core::mem::size_of::<u32>() + 2 * core::mem::size_of::<f64>())
            .saturating_add(2 * (dimension + 1).saturating_mul(core::mem::size_of::<usize>()));

        if dimension <= u32::MAX as usize && sparse_bytes < packed_bytes {
            let mut row_offsets = Vec::with_capacity(dimension + 1);
            let mut columns = Vec::with_capacity(strict_nonzeros);
            let mut row_values = Vec::with_capacity(strict_nonzeros);
            row_offsets.push(0);
            for (row, values) in lower.iter().enumerate() {
                for (column, value) in values[..row].iter().copied().enumerate() {
                    if value != 0.0 {
                        columns.push(column as u32);
                        row_values.push(value);
                    }
                }
                row_offsets.push(columns.len());
            }

            let mut column_counts = vec![0_usize; dimension];
            for &column in &columns {
                column_counts[column as usize] += 1;
            }
            let mut column_offsets = Vec::with_capacity(dimension + 1);
            column_offsets.push(0);
            for count in column_counts {
                column_offsets.push(column_offsets.last().copied().unwrap_or(0) + count);
            }
            let mut next = column_offsets[..dimension].to_vec();
            let mut rows = vec![0_u32; strict_nonzeros];
            let mut column_values = vec![0.0; strict_nonzeros];
            for row in 0..dimension {
                for index in row_offsets[row]..row_offsets[row + 1] {
                    let column = columns[index] as usize;
                    let destination = next[column];
                    rows[destination] = row as u32;
                    column_values[destination] = row_values[index];
                    next[column] += 1;
                }
            }

            Self::Sparse {
                row_offsets,
                columns,
                row_values,
                column_offsets,
                rows,
                column_values,
            }
        } else {
            let mut values = Vec::with_capacity(packed_slots);
            for (row, dense_row) in lower.iter().enumerate() {
                values.extend_from_slice(&dense_row[..row]);
            }
            Self::Packed { values }
        }
    }

    fn forward_correction(&self, row: usize, forward: &[f64]) -> f64 {
        match self {
            Self::Packed { values } => {
                let start = row.saturating_mul(row.saturating_sub(1)) / 2;
                values[start..start + row]
                    .iter()
                    .zip(&forward[..row])
                    .map(|(lower_value, previous)| lower_value * previous)
                    .sum()
            }
            Self::Sparse {
                row_offsets,
                columns,
                row_values,
                ..
            } => (row_offsets[row]..row_offsets[row + 1])
                .map(|index| row_values[index] * forward[columns[index] as usize])
                .sum(),
        }
    }

    fn backward_correction(&self, row: usize, solution: &[f64]) -> f64 {
        match self {
            Self::Packed { values } => ((row + 1)..solution.len())
                .map(|later| {
                    let index = later.saturating_mul(later.saturating_sub(1)) / 2 + row;
                    values[index] * solution[later]
                })
                .sum(),
            Self::Sparse {
                column_offsets,
                rows,
                column_values,
                ..
            } => (column_offsets[row]..column_offsets[row + 1])
                .map(|index| column_values[index] * solution[rows[index] as usize])
                .sum(),
        }
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
    /// Factor the graph Laplacian after grounding one vertex per component.
    pub fn factor(graph: &Laplacian) -> Result<Self, CmgError> {
        let vertex_count = graph.vertex_count();
        let components = Components::from_laplacian(graph);

        let mut anchors = vec![0; components.count()];
        for (vertex, &component) in components.labels().iter().enumerate() {
            anchors[component] = vertex;
        }
        let mut is_anchor = vec![false; vertex_count];
        for &anchor in &anchors {
            is_anchor[anchor] = true;
        }

        let active_vertices: Vec<usize> = (0..vertex_count)
            .filter(|vertex| !is_anchor[*vertex])
            .collect();
        let mut pattern_nonzeros = vec![0_usize; vertex_count];
        for &vertex in &active_vertices {
            // Every active grounded row retains its positive diagonal.
            pattern_nonzeros[vertex] = 1;
        }
        for edge in graph.edges() {
            if !is_anchor[edge.u()] && !is_anchor[edge.v()] {
                pattern_nonzeros[edge.u()] += 1;
                pattern_nonzeros[edge.v()] += 1;
            }
        }

        let mut permutation = active_vertices;
        permutation.sort_by_key(|&vertex| (pattern_nonzeros[vertex], vertex));
        let dimension = permutation.len();

        // Assemble the ordered grounded matrix directly. The previous path
        // first materialized the full graph matrix and then copied the active
        // permutation into this second dense buffer. Direct assembly removes
        // one vertex_count^2 allocation and its complete permutation scan.
        let mut matrix = vec![vec![0.0; dimension]; dimension];
        let mut factor_index = vec![usize::MAX; vertex_count];
        for (factor_vertex, &original_vertex) in permutation.iter().enumerate() {
            factor_index[original_vertex] = factor_vertex;
            matrix[factor_vertex][factor_vertex] = graph.diagonal()[original_vertex];
        }
        for edge in graph.edges() {
            let factor_u = factor_index[edge.u()];
            let factor_v = factor_index[edge.v()];
            if factor_u == usize::MAX || factor_v == usize::MAX {
                continue;
            }
            matrix[factor_u][factor_v] -= edge.weight();
            matrix[factor_v][factor_u] -= edge.weight();
        }

        let mut dense_lower = vec![vec![0.0; dimension]; dimension];
        let mut diagonal = vec![0.0; dimension];
        for (row, values) in dense_lower.iter_mut().enumerate() {
            values[row] = 1.0;
        }

        for column in 0..dimension {
            let mut pivot = matrix[column][column];
            for previous in 0..column {
                let value = dense_lower[column][previous];
                pivot -= value * value * diagonal[previous];
            }
            if !pivot.is_finite() || pivot <= 0.0 {
                return Err(CmgError::NonPositivePivot {
                    vertex: permutation[column],
                    value: pivot,
                });
            }
            diagonal[column] = pivot;

            for row in (column + 1)..dimension {
                let mut value = matrix[row][column];
                for previous in 0..column {
                    value -= dense_lower[row][previous]
                        * dense_lower[column][previous]
                        * diagonal[previous];
                }
                dense_lower[row][column] = value / pivot;
            }
        }

        let strict_nonzeros = dense_lower
            .iter()
            .enumerate()
            .map(|(row, values)| values[..row].iter().filter(|value| **value != 0.0).count())
            .sum::<usize>();
        let factor_nonzeros = dimension.saturating_add(strict_nonzeros);
        let lower = LowerFactor::from_dense(&dense_lower);

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
        self.solve_into_compatible(rhs, &mut solution, &mut forward, &mut factor_solution)?;
        Ok(solution)
    }

    pub(crate) fn solve_into_compatible(
        &self,
        rhs: &[f64],
        solution: &mut [f64],
        forward: &mut [f64],
        factor_solution: &mut [f64],
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

        for row in 0..dimension {
            forward[row] = rhs[self.permutation[row]] - self.lower.forward_correction(row, forward);
        }
        for (value, pivot) in forward.iter_mut().zip(&self.diagonal) {
            *value /= *pivot;
        }
        for row in (0..dimension).rev() {
            factor_solution[row] =
                forward[row] - self.lower.backward_correction(row, factor_solution);
        }

        solution.fill(0.0);
        for (factor_index, &vertex) in self.permutation.iter().enumerate() {
            solution[vertex] = factor_solution[factor_index];
        }
        Ok(())
    }
}
