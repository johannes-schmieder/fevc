//! Reusable end-to-end CMG solves for SDDM matrices.

use crate::graph::compensated_sum;
use crate::{
    CmgError, CmgOptions, CmgPreconditioner, PcgOptions, PcgWorkspace, SddmAugmentation,
    SddmMatrix, ValidationOptions, solve_pcg_with_workspace,
};

const MAX_CERTIFICATE_REFINEMENTS: usize = 3;

/// A reusable CMG solver for one fixed SDDM matrix.
///
/// Construction performs the exact upstream extra-vertex augmentation, builds
/// one immutable CMG hierarchy on the resulting Laplacian, and retains the
/// original matrix for an independent final residual certificate.
#[derive(Debug, Clone, PartialEq)]
pub struct SddmSolver {
    matrix: SddmMatrix,
    augmentation: SddmAugmentation,
    preconditioner: CmgPreconditioner,
    operator_norm_bound: f64,
}

impl SddmSolver {
    /// Build a reusable solver from an owned SDDM matrix.
    pub fn build(
        matrix: SddmMatrix,
        cmg_options: CmgOptions,
        validation: ValidationOptions,
    ) -> Result<Self, CmgError> {
        let validation = validation.validate()?;
        let augmentation = matrix.augment(validation)?;
        let preconditioner = CmgPreconditioner::build(augmentation.graph(), cmg_options)?;
        let operator_norm_bound = 2.0 * matrix.diagonal().iter().copied().fold(0.0, f64::max);
        Ok(Self {
            matrix,
            augmentation,
            preconditioner,
            operator_norm_bound,
        })
    }

    /// Build a reusable solver by cloning a borrowed SDDM matrix.
    pub fn from_matrix(
        matrix: &SddmMatrix,
        cmg_options: CmgOptions,
        validation: ValidationOptions,
    ) -> Result<Self, CmgError> {
        Self::build(matrix.clone(), cmg_options, validation)
    }

    /// Return the original SDDM matrix.
    #[must_use]
    pub const fn matrix(&self) -> &SddmMatrix {
        &self.matrix
    }

    /// Return the exact Laplacian augmentation map.
    #[must_use]
    pub const fn augmentation(&self) -> &SddmAugmentation {
        &self.augmentation
    }

    /// Return the CMG preconditioner built on the augmented Laplacian.
    #[must_use]
    pub const fn preconditioner(&self) -> &CmgPreconditioner {
        &self.preconditioner
    }

    /// Return the conservative Euclidean operator-norm bound `2 max_i A_ii`.
    #[must_use]
    pub const fn operator_norm_bound(&self) -> f64 {
        self.operator_norm_bound
    }

    /// Allocate reusable work arrays for repeated right-hand sides.
    #[must_use]
    pub fn workspace(&self) -> SddmWorkspace {
        SddmWorkspace::new(self)
    }

    /// Solve one right-hand side with a newly allocated workspace.
    pub fn solve(&self, rhs: &[f64], options: PcgOptions) -> Result<SddmResult, CmgError> {
        let mut workspace = self.workspace();
        self.solve_with_workspace(rhs, options, &mut workspace)
    }

    /// Solve one right-hand side using caller-owned reusable storage.
    pub fn solve_with_workspace(
        &self,
        rhs: &[f64],
        options: PcgOptions,
        workspace: &mut SddmWorkspace,
    ) -> Result<SddmResult, CmgError> {
        let requested_options = options.validate()?;
        let dimension = self.matrix.dimension();
        if rhs.len() != dimension {
            return Err(CmgError::dimension(
                "SddmSolver::solve rhs",
                dimension,
                rhs.len(),
            ));
        }
        workspace.validate(self)?;

        let lifted = self.augmentation.lift_rhs(rhs)?;
        workspace.lifted_rhs.copy_from_slice(&lifted);
        let rhs_norm = euclidean_norm(rhs);
        let mut solve_options = requested_options;
        let mut refinements = 0_usize;
        let mut total_iterations = 0_usize;
        let mut total_restarts = 0_usize;

        loop {
            let augmented = solve_pcg_with_workspace(
                self.augmentation.graph(),
                &self.preconditioner,
                &workspace.lifted_rhs,
                solve_options,
                &mut workspace.pcg,
            )?;
            total_iterations += augmented.iterations();
            total_restarts += augmented.restarts();
            let solution = self.augmentation.extract_solution(augmented.solution())?;

            self.matrix
                .matvec_into(&solution, &mut workspace.original_residual)?;
            for (residual, rhs_value) in workspace.original_residual.iter_mut().zip(rhs) {
                *residual = *rhs_value - *residual;
            }

            let solution_norm = euclidean_norm(&solution);
            let residual_norm = euclidean_norm(&workspace.original_residual);
            let tolerance = requested_options.absolute_tolerance
                + requested_options.relative_tolerance
                    * (rhs_norm + self.operator_norm_bound * solution_norm);
            if !residual_norm.is_finite() {
                return Err(CmgError::PcgBreakdown {
                    iteration: total_iterations,
                    quantity: "original SDDM residual norm",
                    value: residual_norm,
                });
            }
            if !tolerance.is_finite() {
                return Err(CmgError::PcgBreakdown {
                    iteration: total_iterations,
                    quantity: "original SDDM residual tolerance",
                    value: tolerance,
                });
            }

            if residual_norm <= tolerance {
                let relative_residual = if rhs_norm > 0.0 {
                    residual_norm / rhs_norm
                } else {
                    residual_norm
                };
                let denominator = rhs_norm + self.operator_norm_bound * solution_norm;
                let backward_error = if denominator > 0.0 {
                    residual_norm / denominator
                } else {
                    0.0
                };

                return Ok(SddmResult {
                    solution,
                    iterations: total_iterations,
                    restarts: total_restarts,
                    refinements,
                    residual_norm,
                    relative_residual,
                    backward_error,
                    tolerance,
                    augmented_residual_norm: augmented.residual_norm(),
                    augmented_backward_error: augmented.backward_error(),
                });
            }

            if refinements == MAX_CERTIFICATE_REFINEMENTS {
                return Err(CmgError::ResidualVerificationFailed {
                    iteration: total_iterations,
                    residual_norm,
                    tolerance,
                });
            }

            refinements += 1;
            let absolute_target = (0.25 * tolerance).max(f64::from_bits(1));
            solve_options = PcgOptions {
                relative_tolerance: 0.0,
                absolute_tolerance: absolute_target,
                max_iterations: requested_options.max_iterations,
                residual_recompute_interval: requested_options.residual_recompute_interval,
                validation: requested_options.validation,
            };
        }
    }

    /// Solve multiple right-hand sides sequentially with one workspace.
    pub fn solve_batch(
        &self,
        right_hand_sides: &[Vec<f64>],
        options: PcgOptions,
    ) -> Result<Vec<SddmResult>, CmgError> {
        let mut workspace = self.workspace();
        right_hand_sides
            .iter()
            .map(|rhs| self.solve_with_workspace(rhs, options, &mut workspace))
            .collect()
    }
}

/// Reusable storage for repeated SDDM solves with one fixed solver.
#[derive(Debug, Clone)]
pub struct SddmWorkspace {
    lifted_rhs: Vec<f64>,
    original_residual: Vec<f64>,
    pcg: PcgWorkspace,
}

impl SddmWorkspace {
    fn new(solver: &SddmSolver) -> Self {
        Self {
            lifted_rhs: vec![0.0; solver.augmentation.graph().vertex_count()],
            original_residual: vec![0.0; solver.matrix.dimension()],
            pcg: PcgWorkspace::new(&solver.preconditioner),
        }
    }

    /// Return the original SDDM dimension.
    #[must_use]
    pub fn dimension(&self) -> usize {
        self.original_residual.len()
    }

    /// Return the augmented Laplacian dimension.
    #[must_use]
    pub fn augmented_dimension(&self) -> usize {
        self.lifted_rhs.len()
    }

    fn validate(&self, solver: &SddmSolver) -> Result<(), CmgError> {
        let original_dimension = solver.matrix.dimension();
        if self.original_residual.len() != original_dimension {
            return Err(CmgError::dimension(
                "SddmWorkspace original residual",
                original_dimension,
                self.original_residual.len(),
            ));
        }
        let augmented_dimension = solver.augmentation.graph().vertex_count();
        if self.lifted_rhs.len() != augmented_dimension {
            return Err(CmgError::dimension(
                "SddmWorkspace lifted rhs",
                augmented_dimension,
                self.lifted_rhs.len(),
            ));
        }
        if self.pcg.dimension() != augmented_dimension {
            return Err(CmgError::dimension(
                "SddmWorkspace PCG",
                augmented_dimension,
                self.pcg.dimension(),
            ));
        }
        Ok(())
    }
}

/// A successfully certified solution of the original SDDM system.
#[derive(Debug, Clone, PartialEq)]
pub struct SddmResult {
    solution: Vec<f64>,
    iterations: usize,
    restarts: usize,
    refinements: usize,
    residual_norm: f64,
    relative_residual: f64,
    backward_error: f64,
    tolerance: f64,
    augmented_residual_norm: f64,
    augmented_backward_error: f64,
}

impl SddmResult {
    /// Return the solution of the original SDDM system.
    #[must_use]
    pub fn solution(&self) -> &[f64] {
        &self.solution
    }

    /// Consume the result and return the solution vector.
    #[must_use]
    pub fn into_solution(self) -> Vec<f64> {
        self.solution
    }

    /// Return total augmented-Laplacian PCG iterations across all attempts.
    #[must_use]
    pub const fn iterations(&self) -> usize {
        self.iterations
    }

    /// Return explicit residual-replacement restarts across all attempts.
    #[must_use]
    pub const fn restarts(&self) -> usize {
        self.restarts
    }

    /// Return the number of stricter augmented-system refinement solves.
    #[must_use]
    pub const fn refinements(&self) -> usize {
        self.refinements
    }

    /// Return the fresh residual norm in the original SDDM system.
    #[must_use]
    pub const fn residual_norm(&self) -> f64 {
        self.residual_norm
    }

    /// Return the original-system relative residual.
    #[must_use]
    pub const fn relative_residual(&self) -> f64 {
        self.relative_residual
    }

    /// Return the original-system normwise backward-error certificate.
    #[must_use]
    pub const fn backward_error(&self) -> f64 {
        self.backward_error
    }

    /// Return the original-system absolute residual threshold.
    #[must_use]
    pub const fn tolerance(&self) -> f64 {
        self.tolerance
    }

    /// Return the fresh residual norm in the augmented Laplacian system.
    #[must_use]
    pub const fn augmented_residual_norm(&self) -> f64 {
        self.augmented_residual_norm
    }

    /// Return the augmented-system backward error reported by PCG.
    #[must_use]
    pub const fn augmented_backward_error(&self) -> f64 {
        self.augmented_backward_error
    }
}

/// Build a temporary solver and solve one SDDM right-hand side.
pub fn solve_sddm(
    matrix: &SddmMatrix,
    rhs: &[f64],
    cmg_options: CmgOptions,
    pcg_options: PcgOptions,
) -> Result<SddmResult, CmgError> {
    let solver = SddmSolver::from_matrix(matrix, cmg_options, pcg_options.validation)?;
    solver.solve(rhs, pcg_options)
}

fn euclidean_norm(values: &[f64]) -> f64 {
    let scale = values.iter().map(|value| value.abs()).fold(0.0, f64::max);
    if scale == 0.0 {
        0.0
    } else {
        scale
            * compensated_sum(values.iter().map(|value| {
                let scaled = *value / scale;
                scaled * scaled
            }))
            .sqrt()
    }
}
