//! Error types for CMG construction and solves.

use core::fmt;

/// Errors returned by graph validation, hierarchy construction, and solves.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum CmgError {
    /// A vector or matrix dimension did not match the expected dimension.
    DimensionMismatch {
        /// Context in which the mismatch occurred.
        context: &'static str,
        /// Expected length or dimension.
        expected: usize,
        /// Actual length or dimension.
        actual: usize,
    },
    /// A vertex index was outside `0..vertex_count`.
    VertexOutOfBounds {
        /// Invalid vertex index.
        vertex: usize,
        /// Number of vertices in the graph.
        vertex_count: usize,
    },
    /// A vertex index cannot be represented by compact retained edge storage.
    VertexIndexTooWide {
        /// Vertex index that exceeded the compact representation.
        vertex: usize,
        /// Largest endpoint representable by the retained edge format.
        maximum: usize,
    },
    /// A self-loop was supplied to an undirected Laplacian edge list.
    SelfLoop {
        /// Vertex carrying the self-loop.
        vertex: usize,
    },
    /// An edge weight was non-finite or not strictly positive.
    InvalidEdgeWeight {
        /// First endpoint.
        u: usize,
        /// Second endpoint.
        v: usize,
        /// Invalid weight.
        weight: f64,
    },
    /// A matrix value was not finite.
    NonFiniteMatrixValue {
        /// Row index.
        row: usize,
        /// Column index.
        column: usize,
        /// Invalid value.
        value: f64,
    },
    /// A matrix expected to be symmetric was not symmetric within tolerance.
    NotSymmetric {
        /// Row index of the first detected mismatch.
        row: usize,
        /// Column index of the first detected mismatch.
        column: usize,
        /// Entry at `(row, column)`.
        forward: f64,
        /// Entry at `(column, row)`.
        reverse: f64,
    },
    /// An SDDM matrix contained a positive off-diagonal entry.
    PositiveOffDiagonal {
        /// Row index.
        row: usize,
        /// Column index.
        column: usize,
        /// Positive value.
        value: f64,
    },
    /// A diagonal entry was negative.
    NegativeDiagonal {
        /// Row/column index.
        index: usize,
        /// Negative value.
        value: f64,
    },
    /// A matrix row was not diagonally dominant.
    NotDiagonallyDominant {
        /// Row index.
        row: usize,
        /// Diagonal value.
        diagonal: f64,
        /// Sum of absolute off-diagonal values.
        off_diagonal_sum: f64,
    },
    /// A right-hand side was incompatible with a Laplacian null space.
    IncompatibleLaplacianRhs {
        /// Component label.
        component: usize,
        /// Sum of the right-hand side on that component.
        sum: f64,
        /// Allowed absolute tolerance for that component.
        tolerance: f64,
    },
    /// A grounded LDL factorization encountered a nonpositive pivot.
    NonPositivePivot {
        /// Original graph vertex associated with the pivot.
        vertex: usize,
        /// Nonpositive or non-finite pivot value.
        value: f64,
    },
    /// A constructed hierarchy violated an internal structural invariant.
    InvalidHierarchy {
        /// Description of the violated invariant.
        context: &'static str,
    },
    /// PCG encountered a nonpositive or non-finite scalar.
    PcgBreakdown {
        /// One-based iteration number, or zero for initialization.
        iteration: usize,
        /// Scalar or operation that failed.
        quantity: &'static str,
        /// Invalid value.
        value: f64,
    },
    /// PCG exhausted its iteration budget without a certified solution.
    MaximumIterations {
        /// Number of completed iterations.
        iterations: usize,
        /// Fresh original-system residual norm.
        residual_norm: f64,
        /// Required residual tolerance.
        tolerance: f64,
    },
    /// A candidate convergence decision failed a fresh residual certificate.
    ResidualVerificationFailed {
        /// Iteration at which verification failed.
        iteration: usize,
        /// Fresh original-system residual norm.
        residual_norm: f64,
        /// Required residual tolerance.
        tolerance: f64,
    },
    /// A package-owned parallel runtime could not be constructed.
    ParallelRuntime {
        /// Runtime-construction diagnostic.
        message: String,
    },
    /// A requested workspace-memory budget cannot hold one required workspace.
    MemoryBudgetExceeded {
        /// Minimum required workspace bytes.
        required_bytes: usize,
        /// Configured workspace-memory budget.
        budget_bytes: usize,
    },
    /// An option was non-finite or outside its allowed range.
    InvalidOption {
        /// Option name.
        name: &'static str,
        /// Invalid value.
        value: f64,
    },
}

impl CmgError {
    /// Construct a dimension-mismatch error.
    #[must_use]
    pub(crate) const fn dimension(context: &'static str, expected: usize, actual: usize) -> Self {
        Self::DimensionMismatch {
            context,
            expected,
            actual,
        }
    }
}

impl fmt::Display for CmgError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DimensionMismatch {
                context,
                expected,
                actual,
            } => write!(
                formatter,
                "dimension mismatch in {context}: expected {expected}, got {actual}"
            ),
            Self::VertexOutOfBounds {
                vertex,
                vertex_count,
            } => write!(formatter, "vertex {vertex} is outside 0..{vertex_count}"),
            Self::VertexIndexTooWide { vertex, maximum } => write!(
                formatter,
                "vertex {vertex} exceeds the retained edge endpoint limit {maximum}"
            ),
            Self::SelfLoop { vertex } => {
                write!(
                    formatter,
                    "self-loop at vertex {vertex} is not a Laplacian edge"
                )
            }
            Self::InvalidEdgeWeight { u, v, weight } => write!(
                formatter,
                "edge ({u}, {v}) has invalid weight {weight}; weights must be finite and positive"
            ),
            Self::NonFiniteMatrixValue { row, column, value } => write!(
                formatter,
                "matrix entry ({row}, {column}) is not finite: {value}"
            ),
            Self::NotSymmetric {
                row,
                column,
                forward,
                reverse,
            } => write!(
                formatter,
                "matrix is not symmetric at ({row}, {column}): {forward} versus {reverse}"
            ),
            Self::PositiveOffDiagonal { row, column, value } => write!(
                formatter,
                "positive off-diagonal entry at ({row}, {column}): {value}"
            ),
            Self::NegativeDiagonal { index, value } => {
                write!(formatter, "negative diagonal entry at {index}: {value}")
            }
            Self::NotDiagonallyDominant {
                row,
                diagonal,
                off_diagonal_sum,
            } => write!(
                formatter,
                "row {row} is not diagonally dominant: diagonal {diagonal}, off-diagonal sum {off_diagonal_sum}"
            ),
            Self::IncompatibleLaplacianRhs {
                component,
                sum,
                tolerance,
            } => write!(
                formatter,
                "right-hand side is incompatible on component {component}: sum {sum}, tolerance {tolerance}"
            ),
            Self::NonPositivePivot { vertex, value } => write!(
                formatter,
                "grounded LDL factorization has nonpositive pivot at vertex {vertex}: {value}"
            ),
            Self::InvalidHierarchy { context } => {
                write!(formatter, "invalid CMG hierarchy: {context}")
            }
            Self::PcgBreakdown {
                iteration,
                quantity,
                value,
            } => write!(
                formatter,
                "PCG breakdown at iteration {iteration}: {quantity} = {value}"
            ),
            Self::MaximumIterations {
                iterations,
                residual_norm,
                tolerance,
            } => write!(
                formatter,
                "PCG did not converge in {iterations} iterations: residual {residual_norm}, tolerance {tolerance}"
            ),
            Self::ResidualVerificationFailed {
                iteration,
                residual_norm,
                tolerance,
            } => write!(
                formatter,
                "PCG residual verification failed at iteration {iteration}: residual {residual_norm}, tolerance {tolerance}"
            ),
            Self::ParallelRuntime { message } => {
                write!(
                    formatter,
                    "could not construct CMG parallel runtime: {message}"
                )
            }
            Self::MemoryBudgetExceeded {
                required_bytes,
                budget_bytes,
            } => write!(
                formatter,
                "workspace memory budget {budget_bytes} bytes is below the required {required_bytes} bytes"
            ),
            Self::InvalidOption { name, value } => {
                write!(formatter, "option {name} has invalid value {value}")
            }
        }
    }
}

impl std::error::Error for CmgError {}
