// SPDX-License-Identifier: GPL-3.0-only

//! Prepared diagonal- and CMG-PCG solvers for the generic W+F+Q model.
//!
//! The legacy preparation API remains a forced diagonal route. Explicit CMG
//! and automatic setup are isolated here and are not advertised by the public
//! engine or capability flags.

use std::sync::{Arc, Mutex};

use crate::cmg::{CmgOptions, CmgPreconditioner, CmgReceipt};
use crate::dense::{cholesky_factor, invert_scaled_spd, symmetric_eigen_extremes};
use crate::error::{BackendError, ErrorCode, Result};
use crate::generic_batch::{
    model_batched_pcg_with_interrupt, ModelDiagonalPreconditioner, ModelPcgReceipt,
    ModelPreconditioner,
};
use crate::interrupt::{
    checkpoint_chunk, stable_sort_by_with_interrupt, InterruptCheck, NeverInterrupt,
};
use crate::krylov::{PcgOptions, Preconditioner as FePreconditioner};
use crate::model_operator::{
    checked_matrix_length, copy_f64_with_interrupt, copy_into_with_interrupt,
    fill_f64_with_interrupt, reserve_exact, validate_finite_with_interrupt,
    zeroed_f64_with_interrupt, CanonicalModelData, ModelOperator, ModelResidual, ModelRhs,
    ModelWorkspace,
};
use crate::problem::{CompressedProblem, GroupIndex};
use crate::types::Dimensions;

const RANK_PROJECTION_PCG_TOLERANCE: f64 = 1.0e-13;
const RANK_PROJECTION_MAXIMUM_ITERATIONS: u32 = 100_000;
const RANK_PROJECTION_REPLACEMENT_INTERVAL: u32 = 25;
const RANK_PROJECTION_COMPLETE_RESIDUAL_GATE: f64 = 1.0e-11;
const RANK_NUMERICAL_TOLERANCE_FLOOR: f64 = 1.0e-12;

#[derive(Clone, Copy, Debug)]
pub struct ModelSolverOptions {
    pub pcg: PcgOptions,
    /// Relative identification threshold for the FE-residualized canonical
    /// control information. No ridge or eigenvalue clipping is applied.
    pub rank_tolerance: f64,
}

impl Default for ModelSolverOptions {
    fn default() -> Self {
        Self {
            pcg: PcgOptions {
                tolerance: 1.0e-10,
                maximum_iterations: 10_000,
                residual_replacement_interval: 50,
            },
            rank_tolerance: 1.0e-12,
        }
    }
}

impl ModelSolverOptions {
    pub fn validate(self) -> Result<Self> {
        self.pcg.validate()?;
        if !self.rank_tolerance.is_finite()
            || self.rank_tolerance <= 0.0
            || self.rank_tolerance > 1.0
        {
            return Err(BackendError::invalid(
                "model_solver",
                "model rank tolerance must lie in (0, 1]",
            ));
        }
        Ok(self)
    }

    #[must_use]
    pub fn full_residual_tolerance(self) -> f64 {
        (10.0 * self.pcg.tolerance).max(1.0e-11)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModelSolverRoute {
    Diagonal,
    Cmg,
    Auto,
}

#[derive(Clone, Copy, Debug)]
pub struct ModelRoutingOptions {
    pub route: ModelSolverRoute,
    pub cmg_minimum_dimension: usize,
    pub allow_automatic_cmg_setup_fallback: bool,
    pub solver: ModelSolverOptions,
    pub cmg: CmgOptions,
}

impl Default for ModelRoutingOptions {
    fn default() -> Self {
        Self {
            route: ModelSolverRoute::Auto,
            cmg_minimum_dimension: 2_000,
            allow_automatic_cmg_setup_fallback: true,
            solver: ModelSolverOptions::default(),
            cmg: CmgOptions::default(),
        }
    }
}

impl ModelRoutingOptions {
    pub fn validate(self) -> Result<Self> {
        if self.cmg_minimum_dimension == 0 {
            return Err(BackendError::invalid(
                "model_solver_router",
                "automatic CMG minimum dimension must be positive",
            ));
        }
        self.solver.validate()?;
        self.cmg.validate()?;
        Ok(self)
    }
}

#[derive(Clone, Debug)]
pub struct ModelSolverFallback {
    pub from: ModelSolverRoute,
    pub to: ModelSolverRoute,
    pub code: ErrorCode,
    pub message: String,
}

#[derive(Clone, Debug)]
pub struct PreparedModelSolverReceipt {
    pub requested: ModelSolverRoute,
    pub selected: ModelSolverRoute,
    pub dimension: usize,
    pub cmg: Option<CmgReceipt>,
    pub fallback: Option<ModelSolverFallback>,
}

#[derive(Clone, Debug)]
pub struct ModelCoefficients {
    pub worker: Vec<f64>,
    /// Full firm vector in the unweighted zero-sum quotient.
    pub firm: Vec<f64>,
    /// Coefficients in the caller-supplied canonical control basis.
    pub control: Vec<f64>,
}

#[derive(Clone, Debug)]
pub struct ModelSolveReceipt {
    pub pcg: ModelPcgReceipt,
    pub full_residual_tolerance: f64,
    pub full_residual: f64,
}

#[derive(Clone, Debug)]
pub struct ModelSolve {
    pub coefficients: ModelCoefficients,
    pub residual: ModelResidual,
    pub receipt: ModelSolveReceipt,
}

#[derive(Clone, Debug)]
pub struct ModelBatchSolve {
    pub columns: usize,
    /// Solutions and receipts remain in the caller's logical column order.
    pub solution: Vec<ModelSolve>,
}

#[derive(Clone, Debug)]
pub struct ControlRankReceipt {
    pub controls: usize,
    /// One lossless FE-projection receipt per logical canonical control.
    pub projection_rhs: Vec<ControlProjectionReceipt>,
    /// Certified lower bound for the generalized reciprocal condition number
    /// of the FE-residualized Schur information relative to the original
    /// weighted control Gram. This is invariant to nonsingular control-coordinate
    /// changes in exact arithmetic.
    pub rcond: f64,
    pub smallest_generalized_eigenvalue_lower: f64,
    pub largest_generalized_eigenvalue_upper: f64,
    /// Upper bound on the FE-projection error in original-Gram-whitened
    /// coordinates.
    pub projection_error_bound: f64,
    /// Frobenius upper bound on the original-Gram whitening residual.
    pub normalization_error_bound: f64,
    /// Relabel-invariant lower bound on the positive FE information spectrum.
    pub fe_information_eigenvalue_lower_bound: f64,
    pub maximum_projection_residual: f64,
    pub effective_rank_tolerance: f64,
    pub projection_pcg_tolerance: f64,
    pub projection_residual_gate: f64,
}

#[derive(Clone, Debug)]
pub struct ControlProjectionReceipt {
    pub logical_control: usize,
    pub pcg: ModelPcgReceipt,
    pub complete_residual: f64,
    pub complete_residual_tolerance: f64,
    /// Number of original worker-plus-firm score equations certified.
    pub complete_residual_equations: usize,
    /// Dimension of the zero-sum firm quotient solved by PCG.
    pub solver_dimension: usize,
}

#[derive(Debug)]
struct PreparedControlSchur {
    receipt: ControlRankReceipt,
    information: Vec<f64>,
    /// Control-major N-by-Q FE-residualized canonical controls. This artifact
    /// is retained only by the generic-JLA preparation path and is moved into
    /// its geometry phase without cloning.
    residualized_controls: Option<Vec<f64>>,
}

#[derive(Debug)]
pub struct ModelCmgBlockPreconditioner {
    firms: usize,
    controls: usize,
    fe: Arc<CmgPreconditioner>,
    /// Column-major worker-eliminated firm-by-control cross block.
    cross: Vec<f64>,
    /// Row-major inverse of the certified residualized control Schur block.
    control_inverse: Vec<f64>,
    workspace: Mutex<ModelCmgBlockWorkspace>,
    receipt: CmgReceipt,
}

#[derive(Debug)]
struct ModelCmgBlockWorkspace {
    first_firm: Vec<f64>,
    second_firm_rhs: Vec<f64>,
    control_rhs: Vec<f64>,
    control_solution: Vec<f64>,
}

impl ModelCmgBlockPreconditioner {
    fn new_with_interrupt(
        operator: &ModelOperator<'_>,
        control_schur: &PreparedControlSchur,
        options: CmgOptions,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<Self> {
        interrupt.checkpoint("model_cmg_setup")?;
        let problem = model_cmg_problem(operator, interrupt)?;
        let fe = Arc::new(CmgPreconditioner::new_with_interrupt(
            &problem, options, interrupt,
        )?);
        Self::from_fe_with_interrupt(operator, control_schur, fe, interrupt)
    }

    fn from_fe_with_interrupt(
        operator: &ModelOperator<'_>,
        control_schur: &PreparedControlSchur,
        fe: Arc<CmgPreconditioner>,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<Self> {
        interrupt.checkpoint("model_cmg_shared_setup")?;
        let receipt = fe.receipt().clone();
        let firms = operator.firms();
        let controls = operator.controls();
        let cross = prepare_model_control_cross(operator, interrupt)?;
        let control_inverse = if controls == 0 {
            Vec::new()
        } else {
            invert_scaled_spd(
                &control_schur.information,
                controls,
                RANK_NUMERICAL_TOLERANCE_FLOOR,
                interrupt,
                "model_cmg_control_schur",
            )
            .map_err(|error| {
                if error.code == ErrorCode::SingularInformation {
                    BackendError::new(
                        ErrorCode::SingularInformation,
                        "model_cmg_control_schur",
                        "certified residualized control Schur block is not invertible",
                    )
                } else {
                    error
                }
            })?
            .inverse
        };
        let workspace = ModelCmgBlockWorkspace {
            first_firm: zeroed_f64_with_interrupt(
                if controls == 0 { 0 } else { firms },
                "model CMG first firm workspace",
                interrupt,
                "model_cmg_workspace_initialize",
            )?,
            second_firm_rhs: zeroed_f64_with_interrupt(
                if controls == 0 { 0 } else { firms },
                "model CMG second firm workspace",
                interrupt,
                "model_cmg_workspace_initialize",
            )?,
            control_rhs: zeroed_f64_with_interrupt(
                controls,
                "model CMG control RHS workspace",
                interrupt,
                "model_cmg_workspace_initialize",
            )?,
            control_solution: zeroed_f64_with_interrupt(
                controls,
                "model CMG control solution workspace",
                interrupt,
                "model_cmg_workspace_initialize",
            )?,
        };
        Ok(Self {
            firms,
            controls,
            fe,
            cross,
            control_inverse,
            workspace: Mutex::new(workspace),
            receipt,
        })
    }

    #[must_use]
    pub const fn receipt(&self) -> &CmgReceipt {
        &self.receipt
    }

    /// Shared FE CMG state for a future fixed-offset solve prepared on the
    /// identical weighted worker--firm sample.
    #[must_use]
    pub fn fe_preconditioner(&self) -> Arc<CmgPreconditioner> {
        Arc::clone(&self.fe)
    }
}

impl ModelPreconditioner for ModelCmgBlockPreconditioner {
    fn dimension(&self) -> usize {
        self.firms + self.controls
    }

    fn apply_with_interrupt(
        &self,
        residual: &[f64],
        output: &mut [f64],
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<()> {
        interrupt.checkpoint("model_cmg_apply")?;
        if residual.len() != self.dimension() || output.len() != self.dimension() {
            return Err(BackendError::invalid(
                "model_cmg_apply",
                "generic CMG block application has incompatible dimensions",
            ));
        }
        validate_finite_with_interrupt(residual, interrupt, "model_cmg_apply_validate")?;
        if self.controls == 0 {
            return FePreconditioner::apply_with_interrupt(
                self.fe.as_ref(),
                residual,
                output,
                interrupt,
            );
        }
        let mut workspace = self.workspace.lock().map_err(|_| {
            BackendError::new(
                ErrorCode::ContextPoisoned,
                "model_cmg_apply",
                "generic CMG block workspace lock is poisoned",
            )
        })?;
        FePreconditioner::apply_with_interrupt(
            self.fe.as_ref(),
            &residual[..self.firms],
            &mut workspace.first_firm,
            interrupt,
        )?;
        let mut flattened_work = 0_usize;
        for control in 0..self.controls {
            let mut cross_value = 0.0;
            let mut correction = 0.0;
            for firm in 0..self.firms {
                checkpoint_chunk(interrupt, flattened_work, "model_cmg_control_rhs")?;
                compensated_add(
                    &mut cross_value,
                    &mut correction,
                    self.cross[control * self.firms + firm] * workspace.first_firm[firm],
                );
                flattened_work =
                    increment_work(flattened_work, "model CMG control-cross work overflow")?;
            }
            workspace.control_rhs[control] = residual[self.firms + control] - cross_value;
        }
        for row in 0..self.controls {
            let mut value = 0.0;
            let mut correction = 0.0;
            for column in 0..self.controls {
                checkpoint_chunk(
                    interrupt,
                    row * self.controls + column,
                    "model_cmg_control_solve",
                )?;
                compensated_add(
                    &mut value,
                    &mut correction,
                    self.control_inverse[row * self.controls + column]
                        * workspace.control_rhs[column],
                );
            }
            workspace.control_solution[row] = value;
            output[self.firms + row] = value;
        }
        flattened_work = 0;
        for firm in 0..self.firms {
            let mut cross_value = 0.0;
            let mut correction = 0.0;
            for control in 0..self.controls {
                checkpoint_chunk(interrupt, flattened_work, "model_cmg_second_firm_rhs")?;
                compensated_add(
                    &mut cross_value,
                    &mut correction,
                    self.cross[control * self.firms + firm] * workspace.control_solution[control],
                );
                flattened_work =
                    increment_work(flattened_work, "model CMG second firm-cross work overflow")?;
            }
            workspace.second_firm_rhs[firm] = residual[firm] - cross_value;
        }
        FePreconditioner::apply_with_interrupt(
            self.fe.as_ref(),
            &workspace.second_firm_rhs,
            &mut output[..self.firms],
            interrupt,
        )?;
        validate_finite_with_interrupt(output, interrupt, "model_cmg_apply_validate")
    }
}

#[derive(Debug)]
enum PreparedModelBackend {
    Diagonal(ModelDiagonalPreconditioner),
    Cmg(Box<ModelCmgBlockPreconditioner>),
}

#[derive(Debug)]
pub struct PreparedModelSolver<'a> {
    operator: ModelOperator<'a>,
    backend: PreparedModelBackend,
    options: ModelSolverOptions,
    control_schur: PreparedControlSchur,
    receipt: PreparedModelSolverReceipt,
}

/// Specialized pre-RNG solver preparation for generic JLA. The full model
/// retains its certified N-by-Q residualized controls, while the FE-only
/// solver is prepared on the identical weighted sample. On the CMG route both
/// solvers share one immutable FE hierarchy and its synchronized workspace.
#[derive(Debug)]
pub(crate) struct PreparedGenericJlaSolvers<'a> {
    pub(crate) full: PreparedModelSolver<'a>,
    pub(crate) fe: PreparedModelSolver<'a>,
    pub(crate) fe_hierarchy_reused: bool,
}

impl<'a> PreparedModelSolver<'a> {
    /// Prepare from already-canonical, column-major controls. Raw-control
    /// basis construction remains the caller's responsibility.
    pub fn prepare(data: CanonicalModelData<'a>, options: ModelSolverOptions) -> Result<Self> {
        Self::prepare_with_interrupt(data, options, &mut NeverInterrupt)
    }

    pub fn prepare_with_interrupt(
        data: CanonicalModelData<'a>,
        options: ModelSolverOptions,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<Self> {
        Self::prepare_routed_internal(
            data,
            ModelRoutingOptions {
                route: ModelSolverRoute::Diagonal,
                solver: options,
                ..ModelRoutingOptions::default()
            },
            false,
            interrupt,
        )
    }

    /// Prepare the diagonal model solver while retaining the already-certified
    /// FE-residualized controls required by generic JLA. Ordinary model-solver
    /// preparation deliberately does not retain this N-by-Q artifact.
    #[cfg(test)]
    pub(crate) fn prepare_generic_jla_with_interrupt(
        data: CanonicalModelData<'a>,
        options: ModelSolverOptions,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<Self> {
        Self::prepare_routed_internal(
            data,
            ModelRoutingOptions {
                route: ModelSolverRoute::Diagonal,
                solver: options,
                ..ModelRoutingOptions::default()
            },
            true,
            interrupt,
        )
    }

    /// Prepare the routed full and FE-only solvers as one generic-JLA setup
    /// transaction. Automatic CMG fallback is confined to failures while
    /// building the shared hierarchy or full block preconditioner. Rank and
    /// solve failures are never interpreted as routing signals.
    pub(crate) fn prepare_generic_jla_routed_with_interrupt(
        data: CanonicalModelData<'a>,
        options: ModelRoutingOptions,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<PreparedGenericJlaSolvers<'a>> {
        interrupt.checkpoint("model_solver_generic_jla_prepare")?;
        let options = options.validate()?;
        let dimension = data
            .firms
            .checked_add(data.controls.len())
            .ok_or_else(|| rank_resource("generic-JLA model dimension overflow"))?;
        let requested = options.route;
        let selected = match requested {
            ModelSolverRoute::Auto if dimension < options.cmg_minimum_dimension => {
                ModelSolverRoute::Diagonal
            }
            ModelSolverRoute::Auto => ModelSolverRoute::Cmg,
            route => route,
        };
        if selected == ModelSolverRoute::Diagonal {
            return Self::prepare_generic_jla_diagonal_pair(
                data, options, requested, None, interrupt,
            );
        }

        let operator = ModelOperator::new_with_interrupt(data, interrupt)?;
        let fe_data = CanonicalModelData {
            workers: data.workers,
            firms: data.firms,
            row_worker: data.row_worker,
            row_firm: data.row_firm,
            weight: data.weight,
            controls: &[],
        };
        let forced_cmg = ModelRoutingOptions {
            route: ModelSolverRoute::Cmg,
            allow_automatic_cmg_setup_fallback: false,
            ..options
        };
        let fe = match Self::prepare_routed_internal(fe_data, forced_cmg, false, interrupt) {
            Ok(solver) => solver,
            Err(error)
                if requested == ModelSolverRoute::Auto
                    && options.allow_automatic_cmg_setup_fallback
                    && is_model_cmg_setup_fallback_error(&error) =>
            {
                return Self::prepare_generic_jla_diagonal_pair(
                    data,
                    options,
                    requested,
                    Some(model_fallback(error)),
                    interrupt,
                );
            }
            Err(error) => return Err(error),
        };
        // Strict control projections use the already-selected FE route and
        // the fixed rank-specific PCG tolerance, independent of estimation.
        let control_schur =
            certify_control_rank(&operator, options.solver, true, Some(&fe), interrupt)?;
        let shared_fe = fe.cmg_fe_preconditioner().ok_or_else(|| {
            BackendError::invariant(
                "model_solver_generic_jla_prepare",
                "selected CMG FE solver has no reusable hierarchy",
            )
        })?;
        let preconditioner = match ModelCmgBlockPreconditioner::from_fe_with_interrupt(
            &operator,
            &control_schur,
            shared_fe,
            interrupt,
        ) {
            Ok(preconditioner) => preconditioner,
            Err(error)
                if requested == ModelSolverRoute::Auto
                    && options.allow_automatic_cmg_setup_fallback
                    && is_model_cmg_setup_fallback_error(&error) =>
            {
                return Self::prepare_generic_jla_diagonal_pair(
                    data,
                    options,
                    requested,
                    Some(model_fallback(error)),
                    interrupt,
                );
            }
            Err(error) => return Err(error),
        };
        let cmg = Some(preconditioner.receipt().clone());
        let full = Self {
            operator,
            backend: PreparedModelBackend::Cmg(Box::new(preconditioner)),
            options: options.solver,
            control_schur,
            receipt: PreparedModelSolverReceipt {
                requested,
                selected: ModelSolverRoute::Cmg,
                dimension,
                cmg,
                fallback: None,
            },
        };
        Ok(PreparedGenericJlaSolvers {
            full,
            fe,
            fe_hierarchy_reused: true,
        })
    }

    fn prepare_generic_jla_diagonal_pair(
        data: CanonicalModelData<'a>,
        options: ModelRoutingOptions,
        requested: ModelSolverRoute,
        fallback: Option<ModelSolverFallback>,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<PreparedGenericJlaSolvers<'a>> {
        let mut full = Self::prepare_routed_internal(
            data,
            ModelRoutingOptions {
                route: ModelSolverRoute::Diagonal,
                allow_automatic_cmg_setup_fallback: false,
                ..options
            },
            true,
            interrupt,
        )?;
        full.receipt.requested = requested;
        full.receipt.fallback = fallback;
        let fe = Self::prepare_with_interrupt(
            CanonicalModelData {
                workers: data.workers,
                firms: data.firms,
                row_worker: data.row_worker,
                row_firm: data.row_firm,
                weight: data.weight,
                controls: &[],
            },
            options.solver,
            interrupt,
        )?;
        Ok(PreparedGenericJlaSolvers {
            full,
            fe,
            fe_hierarchy_reused: false,
        })
    }

    pub fn prepare_routed(
        data: CanonicalModelData<'a>,
        options: ModelRoutingOptions,
    ) -> Result<Self> {
        Self::prepare_routed_with_interrupt(data, options, &mut NeverInterrupt)
    }

    pub fn prepare_routed_with_interrupt(
        data: CanonicalModelData<'a>,
        options: ModelRoutingOptions,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<Self> {
        Self::prepare_routed_internal(data, options, false, interrupt)
    }

    fn prepare_routed_internal(
        data: CanonicalModelData<'a>,
        options: ModelRoutingOptions,
        retain_residualized_controls: bool,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<Self> {
        interrupt.checkpoint("model_solver_prepare")?;
        let options = options.validate()?;
        let operator = ModelOperator::new_with_interrupt(data, interrupt)?;
        // Scientific control-rank certification always precedes route setup,
        // including automatic fallback and a zero RHS supplied later.
        let control_schur = certify_control_rank(
            &operator,
            options.solver,
            retain_residualized_controls,
            None,
            interrupt,
        )?;
        let requested = options.route;
        let selected = match requested {
            ModelSolverRoute::Auto
                if operator.parameter_count() < options.cmg_minimum_dimension =>
            {
                ModelSolverRoute::Diagonal
            }
            ModelSolverRoute::Auto => ModelSolverRoute::Cmg,
            route => route,
        };
        let mut fallback = None;
        let (selected, backend, cmg) = match selected {
            ModelSolverRoute::Diagonal => {
                let diagonal = ModelDiagonalPreconditioner::new_with_interrupt(
                    operator.reduced_diagonal(),
                    interrupt,
                )?;
                (
                    ModelSolverRoute::Diagonal,
                    PreparedModelBackend::Diagonal(diagonal),
                    None,
                )
            }
            ModelSolverRoute::Cmg => match ModelCmgBlockPreconditioner::new_with_interrupt(
                &operator,
                &control_schur,
                options.cmg,
                interrupt,
            ) {
                Ok(preconditioner) => {
                    let receipt = preconditioner.receipt().clone();
                    (
                        ModelSolverRoute::Cmg,
                        PreparedModelBackend::Cmg(Box::new(preconditioner)),
                        Some(receipt),
                    )
                }
                Err(error)
                    if requested == ModelSolverRoute::Auto
                        && options.allow_automatic_cmg_setup_fallback
                        && is_model_cmg_setup_fallback_error(&error) =>
                {
                    fallback = Some(ModelSolverFallback {
                        from: ModelSolverRoute::Cmg,
                        to: ModelSolverRoute::Diagonal,
                        code: error.code,
                        message: error.to_string(),
                    });
                    let diagonal = ModelDiagonalPreconditioner::new_with_interrupt(
                        operator.reduced_diagonal(),
                        interrupt,
                    )?;
                    (
                        ModelSolverRoute::Diagonal,
                        PreparedModelBackend::Diagonal(diagonal),
                        None,
                    )
                }
                Err(error) => return Err(error),
            },
            ModelSolverRoute::Auto => {
                return Err(BackendError::invariant(
                    "model_solver_router",
                    "model route decision left automatic routing unresolved",
                ));
            }
        };
        let receipt = PreparedModelSolverReceipt {
            requested,
            selected,
            dimension: operator.parameter_count(),
            cmg,
            fallback,
        };
        Ok(Self {
            operator,
            backend,
            options: options.solver,
            control_schur,
            receipt,
        })
    }

    #[must_use]
    pub const fn operator(&self) -> &ModelOperator<'a> {
        &self.operator
    }

    #[must_use]
    pub const fn options(&self) -> ModelSolverOptions {
        self.options
    }

    #[must_use]
    pub const fn control_rank_receipt(&self) -> &ControlRankReceipt {
        &self.control_schur.receipt
    }

    /// Move the generic-JLA-only residualized-control artifact out of the
    /// prepared solver. This is intentionally unavailable to external callers
    /// and cannot clone the potentially large N-by-Q matrix.
    pub(crate) fn take_generic_jla_residualized_controls(&mut self) -> Result<Vec<f64>> {
        self.control_schur
            .residualized_controls
            .take()
            .ok_or_else(|| {
                BackendError::invariant(
                    "model_rank",
                    "generic-JLA residualized controls were not retained or were already taken",
                )
            })
    }

    #[must_use]
    pub const fn receipt(&self) -> &PreparedModelSolverReceipt {
        &self.receipt
    }

    /// Reuse the already-prepared FE CMG state for an identical-sample
    /// fixed-offset solve without rebuilding the hierarchy.
    #[must_use]
    pub fn cmg_fe_preconditioner(&self) -> Option<Arc<CmgPreconditioner>> {
        match &self.backend {
            PreparedModelBackend::Cmg(preconditioner) => Some(preconditioner.fe_preconditioner()),
            PreparedModelBackend::Diagonal(_) => None,
        }
    }

    pub fn apply_preconditioner(&self, residual: &[f64], output: &mut [f64]) -> Result<()> {
        self.apply_preconditioner_with_interrupt(residual, output, &mut NeverInterrupt)
    }

    pub fn apply_preconditioner_with_interrupt(
        &self,
        residual: &[f64],
        output: &mut [f64],
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<()> {
        if residual.len() != self.operator.parameter_count()
            || output.len() != self.operator.parameter_count()
        {
            return Err(BackendError::invalid(
                "model_preconditioner",
                "prepared model preconditioner has incompatible dimensions",
            ));
        }
        self.preconditioner()
            .apply_with_interrupt(residual, output, interrupt)?;
        self.operator
            .project_parameters_with_interrupt(output, interrupt)
    }

    pub fn solve(&self, rhs: ModelRhs<'_>) -> Result<ModelSolve> {
        self.solve_with_interrupt(rhs, &mut NeverInterrupt)
    }

    pub fn solve_with_interrupt(
        &self,
        rhs: ModelRhs<'_>,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<ModelSolve> {
        let reduced_rhs = self.operator.reduce_rhs_with_interrupt(rhs, interrupt)?;
        let solved = model_batched_pcg_with_interrupt(
            &self.operator,
            self.preconditioner(),
            &reduced_rhs,
            1,
            self.options.pcg,
            interrupt,
        )?;
        let mut reduced = solved.solution;
        self.operator
            .project_parameters_with_interrupt(&mut reduced, interrupt)?;
        self.finish_solution(
            rhs,
            reduced,
            solved.receipt.into_iter().next().ok_or_else(|| {
                BackendError::invariant("model_solver", "scalar model solve has no PCG receipt")
            })?,
            self.options,
            interrupt,
        )
    }

    pub fn solve_batch(
        &self,
        worker_rhs: &[f64],
        firm_rhs: &[f64],
        control_rhs: &[f64],
        columns: usize,
        batch_width: usize,
    ) -> Result<ModelBatchSolve> {
        self.solve_batch_with_interrupt(
            worker_rhs,
            firm_rhs,
            control_rhs,
            columns,
            batch_width,
            &mut NeverInterrupt,
        )
    }

    /// Solve in bounded physical batches while preserving each logical
    /// column's scalar recurrence and output order. Changing `batch_width`
    /// does not change a column's arithmetic.
    pub fn solve_batch_with_interrupt(
        &self,
        worker_rhs: &[f64],
        firm_rhs: &[f64],
        control_rhs: &[f64],
        columns: usize,
        batch_width: usize,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<ModelBatchSolve> {
        self.solve_batch_with_options_and_interrupt(
            worker_rhs,
            firm_rhs,
            control_rhs,
            columns,
            batch_width,
            self.options,
            interrupt,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn solve_batch_with_options_and_interrupt(
        &self,
        worker_rhs: &[f64],
        firm_rhs: &[f64],
        control_rhs: &[f64],
        columns: usize,
        batch_width: usize,
        solve_options: ModelSolverOptions,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<ModelBatchSolve> {
        validate_batch_rhs(
            &self.operator,
            worker_rhs,
            firm_rhs,
            control_rhs,
            columns,
            batch_width,
            interrupt,
        )?;
        let mut solution = Vec::new();
        reserve_exact(&mut solution, columns, "model batch solutions")?;
        let dimension = self.operator.parameter_count();
        let mut first_column = 0_usize;
        while first_column < columns {
            interrupt.checkpoint("model_solver_batch")?;
            let width = batch_width.min(columns - first_column);
            let reduced_length = checked_matrix_length(dimension, width, "model reduced batch")?;
            let mut reduced_rhs = zeroed_f64_with_interrupt(
                reduced_length,
                "model reduced batch RHS",
                interrupt,
                "model_solver_initialize_reduced_batch",
            )?;
            for local in 0..width {
                interrupt.checkpoint("model_solver_reduce_column")?;
                let column = first_column + local;
                let rhs = ModelRhs {
                    worker: rhs_column(worker_rhs, column, self.operator.workers()),
                    firm: rhs_column(firm_rhs, column, self.operator.firms()),
                    control: rhs_column(control_rhs, column, self.operator.controls()),
                };
                let reduced = self
                    .operator
                    .reduce_rhs_with_interrupt(rhs, interrupt)
                    .map_err(|error| column_context(error, column))?;
                let begin = local * dimension;
                copy_into_with_interrupt(
                    &reduced,
                    &mut reduced_rhs[begin..begin + dimension],
                    interrupt,
                    "model_solver_copy_reduced_rhs",
                )?;
            }
            let solved = model_batched_pcg_with_interrupt(
                &self.operator,
                self.preconditioner(),
                &reduced_rhs,
                width,
                solve_options.pcg,
                interrupt,
            )
            .map_err(|error| {
                BackendError::new(
                    error.code,
                    "model_solver",
                    format!("batch beginning at zero-based column {first_column}: {error}"),
                )
            })?;
            for local in 0..width {
                interrupt.checkpoint("model_solver_certify_column")?;
                let column = first_column + local;
                let rhs = ModelRhs {
                    worker: rhs_column(worker_rhs, column, self.operator.workers()),
                    firm: rhs_column(firm_rhs, column, self.operator.firms()),
                    control: rhs_column(control_rhs, column, self.operator.controls()),
                };
                let mut reduced = copy_f64_with_interrupt(
                    solved.column(local),
                    "model solved column",
                    interrupt,
                    "model_solver_copy_solution",
                )?;
                self.operator
                    .project_parameters_with_interrupt(&mut reduced, interrupt)?;
                let receipt = solved.receipt[local].clone();
                solution.push(
                    self.finish_solution(rhs, reduced, receipt, solve_options, interrupt)
                        .map_err(|error| column_context(error, column))?,
                );
            }
            first_column += width;
        }
        Ok(ModelBatchSolve { columns, solution })
    }

    fn preconditioner(&self) -> &dyn ModelPreconditioner {
        match &self.backend {
            PreparedModelBackend::Diagonal(preconditioner) => preconditioner,
            PreparedModelBackend::Cmg(preconditioner) => preconditioner.as_ref(),
        }
    }

    fn finish_solution(
        &self,
        rhs: ModelRhs<'_>,
        reduced: Vec<f64>,
        pcg: ModelPcgReceipt,
        solve_options: ModelSolverOptions,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<ModelSolve> {
        if reduced.len() != self.operator.parameter_count() {
            return Err(BackendError::invariant(
                "model_solver",
                "reduced model solution has the wrong dimension",
            ));
        }
        let firm = copy_f64_with_interrupt(
            &reduced[..self.operator.firms()],
            "model full firm coefficients",
            interrupt,
            "model_solver_copy_firm",
        )?;
        let control = copy_f64_with_interrupt(
            &reduced[self.operator.firms()..],
            "model control coefficients",
            interrupt,
            "model_solver_copy_control",
        )?;
        let worker = self
            .operator
            .reconstruct_worker_with_interrupt(rhs.worker, &reduced, interrupt)?;
        let residual = self
            .operator
            .full_residual_with_interrupt(&worker, &firm, &control, rhs, interrupt)?;
        let tolerance = solve_options.full_residual_tolerance();
        if residual.relative_norm > tolerance {
            return Err(BackendError::new(
                ErrorCode::FullResidualFailed,
                "model_solver",
                format!(
                    "complete original W+F+Q residual {} exceeds required tolerance {tolerance}",
                    residual.relative_norm
                ),
            ));
        }
        Ok(ModelSolve {
            coefficients: ModelCoefficients {
                worker,
                firm,
                control,
            },
            receipt: ModelSolveReceipt {
                pcg,
                full_residual_tolerance: tolerance,
                full_residual: residual.relative_norm,
            },
            residual,
        })
    }
}

fn certify_control_rank(
    operator: &ModelOperator<'_>,
    options: ModelSolverOptions,
    retain_residualized_controls: bool,
    prepared_fe_solver: Option<&PreparedModelSolver<'_>>,
    interrupt: &mut dyn InterruptCheck,
) -> Result<PreparedControlSchur> {
    let controls = operator.controls();
    if controls == 0 {
        return Ok(PreparedControlSchur {
            receipt: ControlRankReceipt {
                controls: 0,
                projection_rhs: Vec::new(),
                rcond: 1.0,
                smallest_generalized_eigenvalue_lower: 1.0,
                largest_generalized_eigenvalue_upper: 1.0,
                projection_error_bound: 0.0,
                normalization_error_bound: 0.0,
                fe_information_eigenvalue_lower_bound: 0.0,
                maximum_projection_residual: 0.0,
                effective_rank_tolerance: options
                    .rank_tolerance
                    .max(RANK_NUMERICAL_TOLERANCE_FLOOR),
                projection_pcg_tolerance: RANK_PROJECTION_PCG_TOLERANCE,
                projection_residual_gate: RANK_PROJECTION_COMPLETE_RESIDUAL_GATE,
            },
            information: Vec::new(),
            residualized_controls: retain_residualized_controls.then(Vec::new),
        });
    }

    // The Q=0 recursive preparation terminates here. Generic JLA supplies its
    // route-frozen FE solver so strict projections share the selected route;
    // ordinary model preparation retains the legacy diagonal projection path.
    let data = operator.data();
    let fe_data = CanonicalModelData {
        workers: data.workers,
        firms: data.firms,
        row_worker: data.row_worker,
        row_firm: data.row_firm,
        weight: data.weight,
        controls: &[],
    };
    let rank_options = ModelSolverOptions {
        pcg: PcgOptions {
            tolerance: RANK_PROJECTION_PCG_TOLERANCE,
            maximum_iterations: RANK_PROJECTION_MAXIMUM_ITERATIONS,
            residual_replacement_interval: RANK_PROJECTION_REPLACEMENT_INTERVAL,
        },
        rank_tolerance: options.rank_tolerance,
    };
    let owned_fe_solver;
    let fe_solver = if let Some(prepared) = prepared_fe_solver {
        prepared
    } else {
        owned_fe_solver =
            PreparedModelSolver::prepare_with_interrupt(fe_data, rank_options, interrupt)?;
        &owned_fe_solver
    };
    let worker_length =
        checked_matrix_length(data.workers, controls, "control projection worker RHS")?;
    let firm_length = checked_matrix_length(data.firms, controls, "control projection firm RHS")?;
    let mut worker_rhs = zeroed_f64_with_interrupt(
        worker_length,
        "control projection worker RHS",
        interrupt,
        "model_rank_initialize_rhs",
    )?;
    let mut firm_rhs = zeroed_f64_with_interrupt(
        firm_length,
        "control projection firm RHS",
        interrupt,
        "model_rank_initialize_rhs",
    )?;
    let mut worker_correction = zeroed_f64_with_interrupt(
        worker_length,
        "control projection worker RHS corrections",
        interrupt,
        "model_rank_initialize_rhs",
    )?;
    let mut firm_correction = zeroed_f64_with_interrupt(
        firm_length,
        "control projection firm RHS corrections",
        interrupt,
        "model_rank_initialize_rhs",
    )?;
    let mut work = 0_usize;
    for &row in operator.row_order() {
        let worker = dense_index(data.row_worker[row]);
        let firm = dense_index(data.row_firm[row]);
        for control in 0..controls {
            checkpoint_chunk(interrupt, work, "model_rank_projection_rhs")?;
            let moment = data.weight[row] * data.controls[control][row];
            stable_add_index(
                &mut worker_rhs,
                &mut worker_correction,
                control * data.workers + worker,
                moment,
            );
            stable_add_index(
                &mut firm_rhs,
                &mut firm_correction,
                control * data.firms + firm,
                moment,
            );
            work = increment_work(work, "control projection RHS work overflow")?;
        }
    }
    finish_stable_vector(
        &mut worker_rhs,
        &worker_correction,
        interrupt,
        "model_rank_projection_rhs",
    )?;
    finish_stable_vector(
        &mut firm_rhs,
        &firm_correction,
        interrupt,
        "model_rank_projection_rhs",
    )?;
    drop(worker_correction);
    drop(firm_correction);
    let projected = fe_solver.solve_batch_with_options_and_interrupt(
        &worker_rhs,
        &firm_rhs,
        &[],
        controls,
        controls,
        rank_options,
        interrupt,
    )?;
    let maximum_projection_residual = projected
        .solution
        .iter()
        .map(|solution| solution.residual.relative_norm)
        .fold(0.0_f64, f64::max);
    if !maximum_projection_residual.is_finite()
        || maximum_projection_residual > RANK_PROJECTION_COMPLETE_RESIDUAL_GATE
    {
        return Err(BackendError::new(
            ErrorCode::SingularInformation,
            "model_rank",
            format!(
                "control rank is unresolved: complete FE projection residual {maximum_projection_residual} exceeds {}",
                RANK_PROJECTION_COMPLETE_RESIDUAL_GATE
            ),
        ));
    }
    let complete_residual_equations = data.workers.checked_add(data.firms).ok_or_else(|| {
        BackendError::new(
            ErrorCode::ResourceLimit,
            "model_rank",
            "control projection residual dimension overflow",
        )
    })?;
    let mut projection_rhs = Vec::new();
    reserve_exact(&mut projection_rhs, controls, "control projection receipts")?;
    for (logical_control, solution) in projected.solution.iter().enumerate() {
        projection_rhs.push(ControlProjectionReceipt {
            logical_control,
            pcg: solution.receipt.pcg.clone(),
            complete_residual: solution.receipt.full_residual,
            complete_residual_tolerance: solution.receipt.full_residual_tolerance,
            complete_residual_equations,
            solver_dimension: data.firms,
        });
    }
    let effective_rank_tolerance = options.rank_tolerance.max(RANK_NUMERICAL_TOLERANCE_FLOOR);
    let fe_information_eigenvalue_lower_bound =
        fe_information_spectral_lower_bound(operator, interrupt)?;
    let mut projection_energy_error = zeroed_f64_with_interrupt(
        controls,
        "control projection energy-error bounds",
        interrupt,
        "model_rank_initialize_information",
    )?;
    for (control, solution) in projected.solution.iter().enumerate() {
        checkpoint_chunk(interrupt, control, "model_rank_projection_error")?;
        let residual_norm = solution.residual.absolute_norm;
        let bound = residual_norm * residual_norm / fe_information_eigenvalue_lower_bound;
        if !bound.is_finite() || bound < 0.0 {
            return Err(BackendError::new(
                ErrorCode::SingularInformation,
                "model_rank",
                "control projection error has no finite conservative energy bound",
            ));
        }
        projection_energy_error[control] = bound;
    }

    let information_length =
        checked_matrix_length(controls, controls, "control Schur information")?;
    let mut information = zeroed_f64_with_interrupt(
        information_length,
        "control Schur information",
        interrupt,
        "model_rank_initialize_information",
    )?;
    let mut original_information = zeroed_f64_with_interrupt(
        information_length,
        "control original information",
        interrupt,
        "model_rank_initialize_information",
    )?;
    let mut original_correction = zeroed_f64_with_interrupt(
        information_length,
        "control original-information corrections",
        interrupt,
        "model_rank_initialize_information",
    )?;
    let mut residualized_controls = if retain_residualized_controls {
        Some(zeroed_f64_with_interrupt(
            checked_matrix_length(data.rows(), controls, "residualized controls")?,
            "residualized controls",
            interrupt,
            "model_rank_initialize_residualized_controls",
        )?)
    } else {
        None
    };
    if let Some(residualized) = residualized_controls.as_mut() {
        for (control, solution) in projected.solution.iter().enumerate() {
            for (position, &row) in operator.row_order().iter().enumerate() {
                checkpoint_chunk(
                    interrupt,
                    control * data.rows() + position,
                    "model_rank_residualized_controls",
                )?;
                let worker = dense_index(data.row_worker[row]);
                let firm = dense_index(data.row_firm[row]);
                residualized[control * data.rows() + row] = data.controls[control][row]
                    - solution.coefficients.worker[worker]
                    - solution.coefficients.firm[firm];
            }
        }
    }
    work = 0;
    for left in 0..controls {
        let left_solution = &projected.solution[left].coefficients;
        for right in left..controls {
            let right_solution = &projected.solution[right].coefficients;
            let mut value = 0.0;
            let mut correction = 0.0;
            let original_index = left * controls + right;
            for &row in operator.row_order() {
                checkpoint_chunk(interrupt, work, "model_rank_residualized_information")?;
                let worker = dense_index(data.row_worker[row]);
                let firm = dense_index(data.row_firm[row]);
                let left_residual = residualized_controls.as_ref().map_or_else(
                    || {
                        data.controls[left][row]
                            - left_solution.worker[worker]
                            - left_solution.firm[firm]
                    },
                    |residualized| residualized[left * data.rows() + row],
                );
                let right_residual = residualized_controls.as_ref().map_or_else(
                    || {
                        data.controls[right][row]
                            - right_solution.worker[worker]
                            - right_solution.firm[firm]
                    },
                    |residualized| residualized[right * data.rows() + row],
                );
                compensated_add(
                    &mut value,
                    &mut correction,
                    data.weight[row] * left_residual * right_residual,
                );
                compensated_add(
                    &mut original_information[original_index],
                    &mut original_correction[original_index],
                    data.weight[row] * data.controls[left][row] * data.controls[right][row],
                );
                work = increment_work(work, "control rank information work overflow")?;
            }
            information[left * controls + right] = value;
            information[right * controls + left] = value;
            original_information[right * controls + left] = original_information[original_index];
        }
    }
    let original_inverse = invert_scaled_spd(
        &original_information,
        controls,
        RANK_NUMERICAL_TOLERANCE_FLOOR,
        interrupt,
        "model_control_original_gram",
    )
    .map_err(|error| {
        if error.code == ErrorCode::SingularInformation {
            BackendError::new(
                ErrorCode::SingularInformation,
                "model_rank",
                "original canonical-control Gram is singular or numerically unresolved",
            )
        } else {
            error
        }
    })?;
    let whitener = cholesky_factor(
        &original_inverse.inverse,
        controls,
        interrupt,
        "model_control_original_whitener",
    )?;
    let normalized_original = congruence_transform(
        &original_information,
        &whitener,
        controls,
        interrupt,
        "model_rank_normalize_original",
    )?;
    let normalization_error_bound = identity_residual(&normalized_original, controls);
    if !normalization_error_bound.is_finite() || normalization_error_bound >= 0.25 {
        return Err(BackendError::new(
            ErrorCode::SingularInformation,
            "model_rank",
            "original-control whitening is too inaccurate to certify generalized rank",
        ));
    }
    let normalized_information = congruence_transform(
        &information,
        &whitener,
        controls,
        interrupt,
        "model_rank_normalize_schur",
    )?;
    let projection_error_bound =
        whitened_projection_error_bound(&projection_energy_error, &whitener, controls, interrupt)?;
    let extremes = symmetric_eigen_extremes(
        &normalized_information,
        controls,
        interrupt,
        "model_control_generalized_schur",
    )?;
    let smallest_whitened_lower = extremes.smallest_lower - projection_error_bound;
    let smallest_generalized_eigenvalue_lower =
        smallest_whitened_lower / (1.0 + normalization_error_bound);
    let largest_generalized_eigenvalue_upper =
        extremes.largest_upper / (1.0 - normalization_error_bound);
    let rcond = smallest_generalized_eigenvalue_lower / largest_generalized_eigenvalue_upper;
    if !smallest_generalized_eigenvalue_lower.is_finite()
        || !largest_generalized_eigenvalue_upper.is_finite()
        || largest_generalized_eigenvalue_upper <= 0.0
        || !rcond.is_finite()
        || rcond <= effective_rank_tolerance
    {
        return Err(BackendError::new(
            ErrorCode::SingularInformation,
            "model_rank",
            "FE-residualized generalized control information is singular or numerically unresolved",
        ));
    }
    Ok(PreparedControlSchur {
        receipt: ControlRankReceipt {
            controls,
            projection_rhs,
            rcond,
            smallest_generalized_eigenvalue_lower,
            largest_generalized_eigenvalue_upper,
            projection_error_bound,
            normalization_error_bound,
            fe_information_eigenvalue_lower_bound,
            maximum_projection_residual,
            effective_rank_tolerance,
            projection_pcg_tolerance: RANK_PROJECTION_PCG_TOLERANCE,
            projection_residual_gate: RANK_PROJECTION_COMPLETE_RESIDUAL_GATE,
        },
        information,
        residualized_controls,
    })
}

/// The sign-flipped W+F information is a connected weighted graph Laplacian.
/// For `v = W + F`, Popoviciu's variance bound followed by Cauchy--Schwarz on
/// a path between the extreme vertices gives
/// `lambda_2 >= 4 min(weight) / (v (v - 1))`.  This deliberately conservative
/// bound depends on neither vertex labels nor a chosen spanning tree.
fn fe_information_spectral_lower_bound(
    operator: &ModelOperator<'_>,
    interrupt: &mut dyn InterruptCheck,
) -> Result<f64> {
    let data = operator.data();
    let vertices = data
        .workers
        .checked_add(data.firms)
        .ok_or_else(|| rank_resource("FE information vertex count overflow"))?;
    let predecessor = vertices
        .checked_sub(1)
        .ok_or_else(|| rank_resource("FE information vertex count underflow"))?;
    let mut minimum_weight = f64::INFINITY;
    for (position, &row) in operator.row_order().iter().enumerate() {
        checkpoint_chunk(interrupt, position, "model_rank_fe_spectral_bound")?;
        minimum_weight = minimum_weight.min(data.weight[row]);
    }
    let denominator = (vertices as f64) * (predecessor as f64);
    let bound = (minimum_weight / denominator) * 4.0;
    if !bound.is_finite() || bound <= 0.0 {
        return Err(BackendError::new(
            ErrorCode::SingularInformation,
            "model_rank",
            "FE information has no positive finite relabel-invariant spectral lower bound",
        ));
    }
    Ok(bound)
}

/// Return `factor' * matrix * factor`, where `factor` is row-major.
fn congruence_transform(
    matrix: &[f64],
    factor: &[f64],
    dimension: usize,
    interrupt: &mut dyn InterruptCheck,
    phase: &'static str,
) -> Result<Vec<f64>> {
    let length = checked_matrix_length(dimension, dimension, "rank congruence transform")?;
    if matrix.len() != length || factor.len() != length {
        return Err(BackendError::invalid(
            phase,
            "rank congruence-transform dimensions disagree",
        ));
    }
    let mut intermediate =
        zeroed_f64_with_interrupt(length, "rank congruence intermediate", interrupt, phase)?;
    let mut output = zeroed_f64_with_interrupt(length, "rank congruence output", interrupt, phase)?;
    let mut work = 0_usize;
    for row in 0..dimension {
        for column in 0..dimension {
            let mut value = 0.0;
            let mut correction = 0.0;
            for inner in 0..dimension {
                checkpoint_chunk(interrupt, work, phase)?;
                compensated_add(
                    &mut value,
                    &mut correction,
                    matrix[row * dimension + inner] * factor[inner * dimension + column],
                );
                work = increment_work(work, "rank congruence work overflow")?;
            }
            intermediate[row * dimension + column] = value;
        }
    }
    for row in 0..dimension {
        for column in 0..dimension {
            let mut value = 0.0;
            let mut correction = 0.0;
            for inner in 0..dimension {
                checkpoint_chunk(interrupt, work, phase)?;
                compensated_add(
                    &mut value,
                    &mut correction,
                    factor[inner * dimension + row] * intermediate[inner * dimension + column],
                );
                work = increment_work(work, "rank congruence work overflow")?;
            }
            output[row * dimension + column] = value;
        }
    }
    Ok(output)
}

fn identity_residual(matrix: &[f64], dimension: usize) -> f64 {
    let mut norm_square = 0.0;
    for row in 0..dimension {
        for column in 0..dimension {
            let target = f64::from(row == column);
            let difference = matrix[row * dimension + column] - target;
            norm_square += difference * difference;
        }
    }
    norm_square.sqrt()
}

/// If projection column `j` has W-norm error at most `sqrt(error[j])`, then
/// the spectral norm of the error Gram after whitening is at most the sum of
/// squared triangle-inequality radii of the transformed columns.
fn whitened_projection_error_bound(
    projection_energy_error: &[f64],
    whitener: &[f64],
    dimension: usize,
    interrupt: &mut dyn InterruptCheck,
) -> Result<f64> {
    if projection_energy_error.len() != dimension
        || whitener.len() != dimension.saturating_mul(dimension)
    {
        return Err(BackendError::invalid(
            "model_rank",
            "projection-error whitening dimensions disagree",
        ));
    }
    let mut bound = 0.0;
    let mut correction = 0.0;
    let mut work = 0_usize;
    for column in 0..dimension {
        let mut radius = 0.0;
        let mut radius_correction = 0.0;
        for row in 0..dimension {
            checkpoint_chunk(interrupt, work, "model_rank_projection_error_bound")?;
            compensated_add(
                &mut radius,
                &mut radius_correction,
                whitener[row * dimension + column].abs() * projection_energy_error[row].sqrt(),
            );
            work = increment_work(work, "projection-error bound work overflow")?;
        }
        compensated_add(&mut bound, &mut correction, radius * radius);
    }
    if !bound.is_finite() || bound < 0.0 {
        return Err(BackendError::new(
            ErrorCode::SingularInformation,
            "model_rank",
            "whitened projection error has no finite conservative bound",
        ));
    }
    Ok(bound)
}

fn prepare_model_control_cross(
    operator: &ModelOperator<'_>,
    interrupt: &mut dyn InterruptCheck,
) -> Result<Vec<f64>> {
    let firms = operator.firms();
    let controls = operator.controls();
    let cross_length = checked_matrix_length(firms, controls, "model CMG control cross block")?;
    let mut cross = zeroed_f64_with_interrupt(
        cross_length,
        "model CMG control cross block",
        interrupt,
        "model_cmg_cross_initialize",
    )?;
    if controls == 0 {
        return Ok(cross);
    }
    let dimension = operator.parameter_count();
    let mut input = zeroed_f64_with_interrupt(
        dimension,
        "model CMG cross input",
        interrupt,
        "model_cmg_cross_initialize",
    )?;
    let mut action = zeroed_f64_with_interrupt(
        dimension,
        "model CMG cross action",
        interrupt,
        "model_cmg_cross_initialize",
    )?;
    let mut workspace = ModelWorkspace::new_with_interrupt(operator, interrupt)?;
    for control in 0..controls {
        interrupt.checkpoint("model_cmg_cross_column")?;
        fill_f64_with_interrupt(&mut input, 0.0, interrupt, "model_cmg_cross_zero")?;
        input[firms + control] = 1.0;
        operator.apply_with_workspace_and_interrupt(
            &input,
            &mut action,
            &mut workspace,
            interrupt,
        )?;
        copy_into_with_interrupt(
            &action[..firms],
            &mut cross[control * firms..(control + 1) * firms],
            interrupt,
            "model_cmg_cross_copy",
        )?;
    }
    Ok(cross)
}

/// Build the exact aggregated weighted worker--firm cells consumed by the
/// existing two-way CMG constructor. Other `CompressedProblem` fields remain
/// empty because CMG graph preparation reads only dimensions, cells, and the
/// worker/firm cell indices.
fn model_cmg_problem(
    operator: &ModelOperator<'_>,
    interrupt: &mut dyn InterruptCheck,
) -> Result<CompressedProblem> {
    let data = operator.data();
    let mut order = Vec::new();
    reserve_exact(&mut order, data.rows(), "model CMG row order")?;
    for row in 0..data.rows() {
        checkpoint_chunk(interrupt, row, "model_cmg_cell_order")?;
        order.push(row);
    }
    stable_sort_by_with_interrupt(
        &mut order,
        |&left, &right| {
            data.row_worker[left]
                .cmp(&data.row_worker[right])
                .then_with(|| data.row_firm[left].cmp(&data.row_firm[right]))
                .then_with(|| data.weight[left].total_cmp(&data.weight[right]))
        },
        interrupt,
        "model_cmg_cell_sort",
    )?;
    let mut cell_worker = Vec::new();
    let mut cell_firm = Vec::new();
    let mut cell_weight = Vec::new();
    reserve_exact(&mut cell_worker, data.rows(), "model CMG cell workers")?;
    reserve_exact(&mut cell_firm, data.rows(), "model CMG cell firms")?;
    reserve_exact(&mut cell_weight, data.rows(), "model CMG cell weights")?;
    let mut current = None::<(u32, u32)>;
    let mut weight_sum = 0.0;
    let mut weight_correction = 0.0;
    for (position, &row) in order.iter().enumerate() {
        checkpoint_chunk(interrupt, position, "model_cmg_cell_aggregate")?;
        let pair = (data.row_worker[row], data.row_firm[row]);
        if let Some((worker, firm)) = current {
            if (worker, firm) != pair {
                cell_worker.push(worker);
                cell_firm.push(firm);
                cell_weight.push(weight_sum);
                weight_sum = 0.0;
                weight_correction = 0.0;
            }
        }
        current = Some(pair);
        compensated_add(&mut weight_sum, &mut weight_correction, data.weight[row]);
    }
    if let Some((worker, firm)) = current {
        cell_worker.push(worker);
        cell_firm.push(firm);
        cell_weight.push(weight_sum);
    }
    let worker_index = model_cmg_group_index(
        data.workers,
        &cell_worker,
        &cell_firm,
        interrupt,
        "model_cmg_worker_index",
        "model_cmg_worker_items",
    )?;
    let firm_index = model_cmg_group_index(
        data.firms,
        &cell_firm,
        &cell_worker,
        interrupt,
        "model_cmg_firm_index",
        "model_cmg_firm_items",
    )?;
    let dimensions = Dimensions {
        workers: u64::try_from(data.workers)
            .map_err(|_| rank_resource("model CMG worker dimension is not representable"))?,
        firms: u64::try_from(data.firms)
            .map_err(|_| rank_resource("model CMG firm dimension is not representable"))?,
        cells: u64::try_from(cell_worker.len())
            .map_err(|_| rank_resource("model CMG cell dimension is not representable"))?,
        ..Dimensions::default()
    };
    Ok(CompressedProblem {
        dimensions,
        retained_rows: Vec::new(),
        row_worker: Vec::new(),
        row_firm: Vec::new(),
        row_deletion: Vec::new(),
        row_cell: Vec::new(),
        row_target: Vec::new(),
        outcome: Vec::new(),
        frequency: Vec::new(),
        target_weight: Vec::new(),
        controls: Vec::new(),
        probe_order: None,
        cell_worker,
        cell_firm,
        cell_weight,
        cell_outcome_sum: Vec::new(),
        cell_target_sum: Vec::new(),
        worker_index,
        firm_index,
        deletion_index: GroupIndex {
            ptr: Vec::new(),
            items: Vec::new(),
        },
        target_index: GroupIndex {
            ptr: Vec::new(),
            items: Vec::new(),
        },
        physical_total: 0,
        target_total: 0.0,
        topology_checksum: 0,
    })
}

fn model_cmg_group_index(
    groups: usize,
    primary: &[u32],
    secondary: &[u32],
    interrupt: &mut dyn InterruptCheck,
    phase: &'static str,
    item_phase: &'static str,
) -> Result<GroupIndex> {
    if primary.len() != secondary.len() {
        return Err(BackendError::invariant(
            "model_cmg_setup",
            "model CMG cell-index columns disagree",
        ));
    }
    let mut order = Vec::new();
    reserve_exact(&mut order, primary.len(), "model CMG grouped cell order")?;
    for cell in 0..primary.len() {
        checkpoint_chunk(interrupt, cell, phase)?;
        order.push(cell);
    }
    stable_sort_by_with_interrupt(
        &mut order,
        |&left, &right| {
            primary[left]
                .cmp(&primary[right])
                .then_with(|| secondary[left].cmp(&secondary[right]))
        },
        interrupt,
        phase,
    )?;
    let pointer_count = groups
        .checked_add(1)
        .ok_or_else(|| rank_resource("model CMG group pointer count overflow"))?;
    let mut ptr = Vec::new();
    reserve_exact(&mut ptr, pointer_count, "model CMG group pointers")?;
    let mut items = Vec::new();
    reserve_exact(&mut items, primary.len(), "model CMG group items")?;
    let mut position = 0_usize;
    for group in 0..groups {
        checkpoint_chunk(interrupt, group, phase)?;
        ptr.push(
            u64::try_from(items.len())
                .map_err(|_| rank_resource("model CMG group pointer is not representable"))?,
        );
        while position < order.len()
            && usize::try_from(primary[order[position]]).ok() == Some(group)
        {
            checkpoint_chunk(interrupt, position, item_phase)?;
            items.push(u32::try_from(order[position]).map_err(|_| {
                rank_resource("model CMG cell index exceeds the u32 implementation limit")
            })?);
            position += 1;
        }
    }
    ptr.push(
        u64::try_from(items.len())
            .map_err(|_| rank_resource("model CMG terminal pointer is not representable"))?,
    );
    if position != order.len() {
        return Err(BackendError::invariant(
            "model_cmg_setup",
            "model CMG group index left cells unassigned",
        ));
    }
    let index = GroupIndex { ptr, items };
    index.validate_with_interrupt(groups, primary.len(), interrupt)?;
    Ok(index)
}

pub fn solve_model_diagonal_pcg(
    data: CanonicalModelData<'_>,
    rhs: ModelRhs<'_>,
    options: ModelSolverOptions,
) -> Result<ModelSolve> {
    PreparedModelSolver::prepare(data, options)?.solve(rhs)
}

fn compensated_add(sum: &mut f64, correction: &mut f64, value: f64) {
    let adjusted = value - *correction;
    let next = *sum + adjusted;
    *correction = (next - *sum) - adjusted;
    *sum = next;
}

fn stable_add_index(sum: &mut [f64], correction: &mut [f64], index: usize, value: f64) {
    let next = sum[index] + value;
    if sum[index].abs() >= value.abs() {
        correction[index] += (sum[index] - next) + value;
    } else {
        correction[index] += (value - next) + sum[index];
    }
    sum[index] = next;
}

fn finish_stable_vector(
    sum: &mut [f64],
    correction: &[f64],
    interrupt: &mut dyn InterruptCheck,
    phase: &'static str,
) -> Result<()> {
    if sum.len() != correction.len() {
        return Err(BackendError::invariant(
            phase,
            "compensated vector dimensions disagree",
        ));
    }
    for (index, (value, &adjustment)) in sum.iter_mut().zip(correction).enumerate() {
        checkpoint_chunk(interrupt, index, phase)?;
        *value += adjustment;
    }
    Ok(())
}

fn dense_index(value: u32) -> usize {
    usize::try_from(value).expect("validated dense model index")
}

fn increment_work(value: usize, message: &str) -> Result<usize> {
    value
        .checked_add(1)
        .ok_or_else(|| BackendError::new(ErrorCode::ResourceLimit, "model_rank", message))
}

fn rank_resource(message: &str) -> BackendError {
    BackendError::new(ErrorCode::ResourceLimit, "model_rank", message)
}

fn is_model_cmg_setup_fallback_error(error: &BackendError) -> bool {
    matches!(
        error.code,
        ErrorCode::CmgSetupFailed | ErrorCode::ResourceLimit | ErrorCode::AllocationFailed
    )
}

fn model_fallback(error: BackendError) -> ModelSolverFallback {
    ModelSolverFallback {
        from: ModelSolverRoute::Cmg,
        to: ModelSolverRoute::Diagonal,
        code: error.code,
        message: error.to_string(),
    }
}

fn validate_batch_rhs(
    operator: &ModelOperator<'_>,
    worker_rhs: &[f64],
    firm_rhs: &[f64],
    control_rhs: &[f64],
    columns: usize,
    batch_width: usize,
    interrupt: &mut dyn InterruptCheck,
) -> Result<()> {
    if columns == 0 || batch_width == 0 {
        return Err(BackendError::invalid(
            "model_solver",
            "model column count and batch width must be positive",
        ));
    }
    if worker_rhs.len()
        != checked_matrix_length(operator.workers(), columns, "model batched worker RHS")?
        || firm_rhs.len()
            != checked_matrix_length(operator.firms(), columns, "model batched firm RHS")?
        || control_rhs.len()
            != checked_matrix_length(operator.controls(), columns, "model batched control RHS")?
    {
        return Err(BackendError::invalid(
            "model_solver",
            "batched original model RHS arrays have incompatible dimensions",
        ));
    }
    validate_finite_with_interrupt(worker_rhs, interrupt, "model_solver_validate_batch_rhs")?;
    validate_finite_with_interrupt(firm_rhs, interrupt, "model_solver_validate_batch_rhs")?;
    validate_finite_with_interrupt(control_rhs, interrupt, "model_solver_validate_batch_rhs")?;
    Ok(())
}

fn rhs_column(values: &[f64], column: usize, rows: usize) -> &[f64] {
    let begin = column * rows;
    &values[begin..begin + rows]
}

fn column_context(error: BackendError, column: usize) -> BackendError {
    BackendError::new(
        error.code,
        "model_solver",
        format!("zero-based RHS column {column}: {error}"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn projection_rhs_compensation_retains_deep_cancellation() {
        let mut sum = [0.0];
        let mut correction = [0.0];
        for value in [1.0e16, 1.0, -1.0e16] {
            stable_add_index(&mut sum, &mut correction, 0, value);
        }
        finish_stable_vector(
            &mut sum,
            &correction,
            &mut NeverInterrupt,
            "model_rank_compensation_test",
        )
        .expect("compensated projection RHS");
        assert_eq!(sum[0].to_bits(), 1.0_f64.to_bits());
    }

    #[test]
    fn only_generic_jla_preparation_retains_residualized_controls_once() {
        let row_worker = [0, 0, 1, 1];
        let row_firm = [0, 1, 0, 1];
        let weight = [1.0; 4];
        let controls = vec![vec![1.0, -1.0, -1.0, 1.0]];
        let data = CanonicalModelData {
            workers: 2,
            firms: 2,
            row_worker: &row_worker,
            row_firm: &row_firm,
            weight: &weight,
            controls: &controls,
        };

        let mut ordinary = PreparedModelSolver::prepare(data, ModelSolverOptions::default())
            .expect("ordinary preparation");
        let absent = ordinary
            .take_generic_jla_residualized_controls()
            .expect_err("ordinary preparation must not retain N by Q");
        assert_eq!(absent.code, ErrorCode::InternalInvariantFailed);

        let mut specialized = PreparedModelSolver::prepare_generic_jla_with_interrupt(
            data,
            ModelSolverOptions::default(),
            &mut NeverInterrupt,
        )
        .expect("generic-JLA preparation");
        assert_eq!(specialized.control_rank_receipt().projection_rhs.len(), 1);
        let residualized = specialized
            .take_generic_jla_residualized_controls()
            .expect("generic-JLA retained residualized controls");
        assert_eq!(residualized.len(), 4);
        assert!(residualized.iter().all(|value| value.is_finite()));
        let consumed = specialized
            .take_generic_jla_residualized_controls()
            .expect_err("generic-JLA residualized controls move exactly once");
        assert_eq!(consumed.code, ErrorCode::InternalInvariantFailed);
    }

    #[test]
    fn routed_generic_jla_full_and_fe_solvers_share_one_cmg_arc() {
        let mut row_worker = Vec::new();
        let mut row_firm = Vec::new();
        let mut control = Vec::new();
        for worker in 0..3_u32 {
            for firm in 0..3_u32 {
                row_worker.push(worker);
                row_firm.push(firm);
                control.push(f64::from(worker == firm));
            }
        }
        let weight = vec![1.0; row_worker.len()];
        let no_controls = Vec::new();
        let one_control = [control];
        for controls in [&no_controls[..], &one_control[..]] {
            let prepared = PreparedModelSolver::prepare_generic_jla_routed_with_interrupt(
                CanonicalModelData {
                    workers: 3,
                    firms: 3,
                    row_worker: &row_worker,
                    row_firm: &row_firm,
                    weight: &weight,
                    controls,
                },
                ModelRoutingOptions {
                    route: ModelSolverRoute::Cmg,
                    ..ModelRoutingOptions::default()
                },
                &mut NeverInterrupt,
            )
            .expect("routed generic-JLA CMG preparation");
            let full = prepared
                .full
                .cmg_fe_preconditioner()
                .expect("full CMG hierarchy");
            let fe = prepared
                .fe
                .cmg_fe_preconditioner()
                .expect("FE CMG hierarchy");
            assert!(Arc::ptr_eq(&full, &fe));
            assert!(prepared.fe_hierarchy_reused);
        }
    }
}
