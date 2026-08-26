//! Canonical SDDM matrices and the upstream extra-vertex augmentation.

use crate::graph::{close, compensated_sum};
use crate::{CmgError, Laplacian, ValidationOptions};

/// A canonical symmetric diagonally dominant M-matrix.
#[derive(Debug, Clone, PartialEq)]
pub struct SddmMatrix {
    diagonal: Vec<f64>,
    off_diagonal: Vec<(usize, usize, f64)>,
    row_off_diagonal_abs: Vec<f64>,
}

impl SddmMatrix {
    /// Construct an SDDM matrix from its diagonal and symmetric upper-triangle
    /// off-diagonal entries.
    ///
    /// Each off-diagonal value must be finite and nonpositive. Duplicate
    /// endpoint pairs are aggregated deterministically.
    pub fn from_parts<I>(
        diagonal: Vec<f64>,
        off_diagonal: I,
        options: ValidationOptions,
    ) -> Result<Self, CmgError>
    where
        I: IntoIterator<Item = (usize, usize, f64)>,
    {
        options.validate()?;
        let n = diagonal.len();
        for (index, value) in diagonal.iter().copied().enumerate() {
            if !value.is_finite() {
                return Err(CmgError::NonFiniteMatrixValue {
                    row: index,
                    column: index,
                    value,
                });
            }
            if value < 0.0 {
                return Err(CmgError::NegativeDiagonal { index, value });
            }
        }

        let mut entries = Vec::new();
        for (left, right, value) in off_diagonal {
            if left >= n {
                return Err(CmgError::VertexOutOfBounds {
                    vertex: left,
                    vertex_count: n,
                });
            }
            if right >= n {
                return Err(CmgError::VertexOutOfBounds {
                    vertex: right,
                    vertex_count: n,
                });
            }
            if left == right {
                return Err(CmgError::SelfLoop { vertex: left });
            }
            if !value.is_finite() {
                return Err(CmgError::NonFiniteMatrixValue {
                    row: left,
                    column: right,
                    value,
                });
            }
            if value > 0.0 {
                return Err(CmgError::PositiveOffDiagonal {
                    row: left,
                    column: right,
                    value,
                });
            }
            if value == 0.0 {
                continue;
            }
            let (u, v) = if left < right {
                (left, right)
            } else {
                (right, left)
            };
            entries.push((u, v, value));
        }
        entries.sort_by(|left, right| {
            left.0
                .cmp(&right.0)
                .then(left.1.cmp(&right.1))
                .then_with(|| left.2.total_cmp(&right.2))
        });

        let mut canonical = Vec::with_capacity(entries.len());
        let mut cursor = 0;
        while cursor < entries.len() {
            let u = entries[cursor].0;
            let v = entries[cursor].1;
            let start = cursor;
            while cursor < entries.len() && entries[cursor].0 == u && entries[cursor].1 == v {
                cursor += 1;
            }
            let value = compensated_sum(entries[start..cursor].iter().map(|entry| entry.2));
            if value > 0.0 {
                return Err(CmgError::PositiveOffDiagonal {
                    row: u,
                    column: v,
                    value,
                });
            }
            if value != 0.0 {
                canonical.push((u, v, value));
            }
        }

        let mut row_off_diagonal_abs = vec![0.0; n];
        for (u, v, value) in &canonical {
            let weight = -*value;
            row_off_diagonal_abs[*u] += weight;
            row_off_diagonal_abs[*v] += weight;
        }
        for (row, (&diagonal_value, &off_diagonal_sum)) in
            diagonal.iter().zip(&row_off_diagonal_abs).enumerate()
        {
            if diagonal_value < off_diagonal_sum {
                return Err(CmgError::NotDiagonallyDominant {
                    row,
                    diagonal: diagonal_value,
                    off_diagonal_sum,
                });
            }
        }

        Ok(Self {
            diagonal,
            off_diagonal: canonical,
            row_off_diagonal_abs,
        })
    }

    /// Validate and construct an SDDM matrix from a dense square matrix.
    pub fn from_dense(matrix: &[Vec<f64>], options: ValidationOptions) -> Result<Self, CmgError> {
        let options = options.validate()?;
        let n = matrix.len();
        if let Some(row) = matrix.iter().find(|row| row.len() != n) {
            return Err(CmgError::dimension(
                "SddmMatrix::from_dense row",
                n,
                row.len(),
            ));
        }
        for (row, values) in matrix.iter().enumerate() {
            for (column, value) in values.iter().copied().enumerate() {
                if !value.is_finite() {
                    return Err(CmgError::NonFiniteMatrixValue { row, column, value });
                }
            }
        }
        for (row, values) in matrix.iter().enumerate() {
            for (column, &forward) in values.iter().enumerate().skip(row + 1) {
                let reverse = matrix[column][row];
                if !close(forward, reverse, options.symmetry_tolerance) {
                    return Err(CmgError::NotSymmetric {
                        row,
                        column,
                        forward,
                        reverse,
                    });
                }
            }
        }
        let diagonal = matrix
            .iter()
            .enumerate()
            .map(|(index, row)| row[index])
            .collect();
        let mut off_diagonal = Vec::new();
        for (row, values) in matrix.iter().enumerate() {
            for (column, &forward) in values.iter().enumerate().skip(row + 1) {
                let value = 0.5 * (forward + matrix[column][row]);
                if value != 0.0 {
                    off_diagonal.push((row, column, value));
                }
            }
        }
        Self::from_parts(diagonal, off_diagonal, options)
    }

    /// Return the matrix dimension.
    #[must_use]
    pub fn dimension(&self) -> usize {
        self.diagonal.len()
    }

    /// Return the diagonal.
    #[must_use]
    pub fn diagonal(&self) -> &[f64] {
        &self.diagonal
    }

    /// Compute `output = A * input` without allocating.
    pub fn matvec_into(&self, input: &[f64], output: &mut [f64]) -> Result<(), CmgError> {
        let n = self.dimension();
        if input.len() != n {
            return Err(CmgError::dimension(
                "SddmMatrix::matvec input",
                n,
                input.len(),
            ));
        }
        if output.len() != n {
            return Err(CmgError::dimension(
                "SddmMatrix::matvec output",
                n,
                output.len(),
            ));
        }
        for ((output_value, diagonal_value), input_value) in
            output.iter_mut().zip(&self.diagonal).zip(input)
        {
            *output_value = *diagonal_value * *input_value;
        }
        for (u, v, value) in &self.off_diagonal {
            output[*u] += *value * input[*v];
            output[*v] += *value * input[*u];
        }
        Ok(())
    }

    /// Compute and return `A * input`.
    pub fn matvec(&self, input: &[f64]) -> Result<Vec<f64>, CmgError> {
        let mut output = vec![0.0; self.dimension()];
        self.matvec_into(input, &mut output)?;
        Ok(output)
    }

    /// Convert the SDDM matrix into the Laplacian used by CMG.
    ///
    /// Any positive row-sum excess is represented by an edge to one extra
    /// vertex. This is exact even when the excess is below the MATLAB wrapper's
    /// numerical strict-dominance threshold.
    pub fn augment(&self, options: ValidationOptions) -> Result<SddmAugmentation, CmgError> {
        options.validate()?;
        let n = self.dimension();
        let excess: Vec<f64> = self
            .diagonal
            .iter()
            .zip(&self.row_off_diagonal_abs)
            .map(|(diagonal, off_sum)| (*diagonal - *off_sum).max(0.0))
            .collect();
        let augmented = excess.iter().any(|value| *value > 0.0);

        let mut edges: Vec<(usize, usize, f64)> = self
            .off_diagonal
            .iter()
            .map(|(u, v, value)| (*u, *v, -*value))
            .collect();
        if augmented {
            for (vertex, weight) in excess.iter().copied().enumerate() {
                if weight > 0.0 {
                    edges.push((vertex, n, weight));
                }
            }
        }
        let graph_vertex_count = if augmented { n + 1 } else { n };
        let graph = Laplacian::from_edges(graph_vertex_count, edges)?;
        Ok(SddmAugmentation {
            graph,
            original_dimension: n,
            augmented,
            excess,
        })
    }

    /// Materialize the SDDM matrix as a dense row-major matrix.
    #[must_use]
    pub fn to_dense(&self) -> Vec<Vec<f64>> {
        let n = self.dimension();
        let mut dense = vec![vec![0.0; n]; n];
        for (index, value) in self.diagonal.iter().copied().enumerate() {
            dense[index][index] = value;
        }
        for (u, v, value) in &self.off_diagonal {
            dense[*u][*v] = *value;
            dense[*v][*u] = *value;
        }
        dense
    }
}

/// The Laplacian augmentation and extraction map for an SDDM system.
#[derive(Debug, Clone, PartialEq)]
pub struct SddmAugmentation {
    graph: Laplacian,
    original_dimension: usize,
    augmented: bool,
    excess: Vec<f64>,
}

impl SddmAugmentation {
    /// Return the Laplacian supplied to CMG.
    #[must_use]
    pub const fn graph(&self) -> &Laplacian {
        &self.graph
    }

    /// Return whether an extra vertex was added.
    #[must_use]
    pub const fn is_augmented(&self) -> bool {
        self.augmented
    }

    /// Return each original row's strict-dominance excess.
    #[must_use]
    pub fn excess(&self) -> &[f64] {
        &self.excess
    }

    /// Lift an arbitrary original right-hand side into the augmented
    /// Laplacian's range.
    pub fn lift_rhs(&self, rhs: &[f64]) -> Result<Vec<f64>, CmgError> {
        if rhs.len() != self.original_dimension {
            return Err(CmgError::dimension(
                "SddmAugmentation::lift_rhs",
                self.original_dimension,
                rhs.len(),
            ));
        }
        let mut lifted = rhs.to_vec();
        if self.augmented {
            lifted.push(-compensated_sum(rhs.iter().copied()));
        }
        Ok(lifted)
    }

    /// Extract an original SDDM solution from a Laplacian solution.
    ///
    /// For an augmented system this applies the upstream transformation
    /// `x[0..n] - x[n]`, making the result invariant to the Laplacian gauge.
    pub fn extract_solution(&self, solution: &[f64]) -> Result<Vec<f64>, CmgError> {
        if solution.len() != self.graph.vertex_count() {
            return Err(CmgError::dimension(
                "SddmAugmentation::extract_solution",
                self.graph.vertex_count(),
                solution.len(),
            ));
        }
        if self.augmented {
            let anchor = solution[self.original_dimension];
            Ok(solution[..self.original_dimension]
                .iter()
                .map(|value| *value - anchor)
                .collect())
        } else {
            Ok(solution.to_vec())
        }
    }
}
