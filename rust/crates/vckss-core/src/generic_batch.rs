// SPDX-License-Identifier: GPL-3.0-only

//! Deterministic independent-scalar batched PCG for the generic model.
//!
//! Every logical column has its own recurrence, convergence decision,
//! residual replacements, status, and receipt. The row traversal is shared,
//! but each column retains the scalar arithmetic order. No N-by-B prediction
//! or N-by-parameter workspace is formed.

use crate::error::{BackendError, ErrorCode, Result};
use crate::interrupt::{checkpoint_chunk, InterruptCheck, NeverInterrupt};
use crate::krylov::PcgOptions;
use crate::model_operator::{
    checked_matrix_length, copy_f64_with_interrupt, copy_into_with_interrupt,
    firm_mean_with_interrupt, reserve_exact, stable_dot_with_interrupt,
    validate_finite_with_interrupt, zeroed_f64_with_interrupt, ModelOperator,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModelPcgStatus {
    ZeroRhs,
    Converged,
}

#[derive(Clone, Debug)]
pub struct ModelPcgReceipt {
    pub status: ModelPcgStatus,
    pub iterations: u32,
    pub relative_residual: f64,
    pub residual_replacements: u32,
    pub operator_applications: u32,
    pub preconditioner_applications: u32,
}

#[derive(Clone, Debug)]
pub struct ModelBatchedPcgSolve {
    pub dimension: usize,
    pub columns: usize,
    pub solution: Vec<f64>,
    pub receipt: Vec<ModelPcgReceipt>,
}

impl ModelBatchedPcgSolve {
    #[must_use]
    pub fn column(&self, column: usize) -> &[f64] {
        let begin = column * self.dimension;
        &self.solution[begin..begin + self.dimension]
    }
}

/// Checked physical sizes of the matrix-free action workspace. `parameters`
/// prices both the caller-owned action/output and the reusable entity-major
/// parameter action retained beside the worker values and column means.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ModelBatchWorkspaceLayout {
    pub worker_values: usize,
    pub parameter_values: usize,
    pub row_major_parameter_values: usize,
    pub mean_values: usize,
}

impl ModelBatchWorkspaceLayout {
    pub fn checked(workers: usize, parameters: usize, columns: usize) -> Result<Self> {
        if columns == 0 {
            return Err(BackendError::invalid(
                "model_batch_workspace",
                "model batch width must be positive",
            ));
        }
        let worker_values = checked_matrix_length(workers, columns, "model worker workspace")?;
        let parameter_values =
            checked_matrix_length(parameters, columns, "model parameter workspace")?;
        let row_major_parameter_values = parameter_values;
        let mean_values = columns;
        for (length, label) in [
            (worker_values, "model worker workspace"),
            (parameter_values, "model parameter workspace"),
            (
                row_major_parameter_values,
                "model row-major parameter workspace",
            ),
            (mean_values, "model mean workspace"),
        ] {
            length
                .checked_mul(core::mem::size_of::<f64>())
                .ok_or_else(|| resource_error(&format!("{label} byte-size overflow")))?;
        }
        Ok(Self {
            worker_values,
            parameter_values,
            row_major_parameter_values,
            mean_values,
        })
    }

    /// Total bytes represented by the layout: the internal worker/mean
    /// workspace plus one caller-owned parameter-sized action buffer.
    pub fn bytes(self) -> Result<u64> {
        let values = self
            .worker_values
            .checked_add(self.parameter_values)
            .and_then(|value| value.checked_add(self.row_major_parameter_values))
            .and_then(|value| value.checked_add(self.mean_values))
            .ok_or_else(|| resource_error("model batch workspace value-count overflow"))?;
        let values = u64::try_from(values).map_err(|_| {
            resource_error("model batch workspace value count is not representable")
        })?;
        let element_bytes = u64::try_from(core::mem::size_of::<f64>())
            .map_err(|_| resource_error("f64 byte size is not representable"))?;
        values
            .checked_mul(element_bytes)
            .ok_or_else(|| resource_error("model batch workspace byte-size overflow"))
    }
}

#[derive(Clone, Debug)]
pub struct ModelBatchWorkspace {
    layout: ModelBatchWorkspaceLayout,
    worker_mean: Vec<f64>,
    parameter_output: Vec<f64>,
    firm_mean: Vec<f64>,
}

impl ModelBatchWorkspace {
    pub fn new(operator: &ModelOperator<'_>, columns: usize) -> Result<Self> {
        Self::new_with_interrupt(operator, columns, &mut NeverInterrupt)
    }

    pub fn new_with_interrupt(
        operator: &ModelOperator<'_>,
        columns: usize,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<Self> {
        let layout = ModelBatchWorkspaceLayout::checked(
            operator.workers(),
            operator.parameter_count(),
            columns,
        )?;
        Ok(Self {
            worker_mean: zeroed_f64_with_interrupt(
                layout.worker_values,
                "model batched worker workspace",
                interrupt,
                "model_batch_workspace_initialize",
            )?,
            parameter_output: zeroed_f64_with_interrupt(
                layout.row_major_parameter_values,
                "model batched row-major parameter workspace",
                interrupt,
                "model_batch_workspace_initialize",
            )?,
            firm_mean: zeroed_f64_with_interrupt(
                layout.mean_values,
                "model batched mean workspace",
                interrupt,
                "model_batch_workspace_initialize",
            )?,
            layout,
        })
    }

    #[must_use]
    pub const fn layout(&self) -> ModelBatchWorkspaceLayout {
        self.layout
    }
}

/// Exact, unmodified diagonal preconditioner. No ridge, clipping, or floor is
/// applied to the reduced diagonal.
#[derive(Clone, Debug)]
pub struct ModelDiagonalPreconditioner {
    inverse: Vec<f64>,
}

/// Preconditioner contract for the full zero-sum-firm plus canonical-control
/// reduced model. Implementations must be symmetric on the firm quotient.
pub trait ModelPreconditioner {
    fn dimension(&self) -> usize;

    fn apply(&self, residual: &[f64], output: &mut [f64]) -> Result<()> {
        self.apply_with_interrupt(residual, output, &mut NeverInterrupt)
    }

    fn apply_with_interrupt(
        &self,
        residual: &[f64],
        output: &mut [f64],
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<()>;
}

impl ModelDiagonalPreconditioner {
    pub fn new(diagonal: &[f64]) -> Result<Self> {
        Self::new_with_interrupt(diagonal, &mut NeverInterrupt)
    }

    pub fn new_with_interrupt(
        diagonal: &[f64],
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<Self> {
        if diagonal.is_empty() {
            return Err(BackendError::new(
                ErrorCode::PcgPreconditionerBreakdown,
                "model_diagonal_preconditioner",
                "exact model diagonal must be positive and finite",
            ));
        }
        let mut inverse = zeroed_f64_with_interrupt(
            diagonal.len(),
            "model inverse diagonal",
            interrupt,
            "model_preconditioner_initialize",
        )?;
        for (index, (output, &value)) in inverse.iter_mut().zip(diagonal).enumerate() {
            checkpoint_chunk(interrupt, index, "model_preconditioner_initialize")?;
            if !value.is_finite() || value <= 0.0 {
                return Err(BackendError::new(
                    ErrorCode::PcgPreconditionerBreakdown,
                    "model_diagonal_preconditioner",
                    "exact model diagonal must be positive and finite",
                ));
            }
            *output = value.recip();
            if !output.is_finite() {
                return Err(BackendError::new(
                    ErrorCode::PcgPreconditionerBreakdown,
                    "model_diagonal_preconditioner",
                    "inverse exact model diagonal is nonfinite",
                ));
            }
        }
        Ok(Self { inverse })
    }

    #[must_use]
    pub fn dimension(&self) -> usize {
        self.inverse.len()
    }
}

impl ModelPreconditioner for ModelDiagonalPreconditioner {
    fn dimension(&self) -> usize {
        self.dimension()
    }

    fn apply_with_interrupt(
        &self,
        residual: &[f64],
        output: &mut [f64],
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<()> {
        if residual.len() != self.dimension() || output.len() != self.dimension() {
            return Err(BackendError::invalid(
                "model_diagonal_preconditioner",
                "diagonal model preconditioner has incompatible dimensions",
            ));
        }
        for (index, ((value, &residual_value), &inverse)) in output
            .iter_mut()
            .zip(residual)
            .zip(&self.inverse)
            .enumerate()
        {
            checkpoint_chunk(interrupt, index, "model_batch_preconditioner")?;
            *value = residual_value * inverse;
        }
        validate_internal_finite(
            output,
            interrupt,
            "model_batch_preconditioner_validate",
            "model diagonal preconditioner produced a nonfinite value",
        )
    }
}

pub fn apply_model_batch(
    operator: &ModelOperator<'_>,
    input: &[f64],
    output: &mut [f64],
    columns: usize,
    workspace: &mut ModelBatchWorkspace,
) -> Result<()> {
    apply_model_batch_with_interrupt(
        operator,
        input,
        output,
        columns,
        workspace,
        &mut NeverInterrupt,
    )
}

pub fn apply_model_batch_with_interrupt(
    operator: &ModelOperator<'_>,
    input: &[f64],
    output: &mut [f64],
    columns: usize,
    workspace: &mut ModelBatchWorkspace,
    interrupt: &mut dyn InterruptCheck,
) -> Result<()> {
    let dimension = operator.parameter_count();
    let expected = checked_matrix_length(dimension, columns, "model batch action")?;
    let required = ModelBatchWorkspaceLayout::checked(operator.workers(), dimension, columns)?;
    if input.len() != expected
        || output.len() != expected
        || workspace.layout != required
        || workspace.worker_mean.len() != required.worker_values
        || workspace.parameter_output.len() != required.row_major_parameter_values
        || workspace.firm_mean.len() != required.mean_values
    {
        return Err(BackendError::invalid(
            "model_batch_operator",
            "model batch action or workspace has incompatible dimensions",
        ));
    }
    for (index, value) in input.iter().enumerate() {
        checkpoint_chunk(interrupt, index, "model_batch_validate")?;
        if !value.is_finite() {
            return Err(BackendError::invalid(
                "model_batch_operator",
                "model batch input is nonfinite",
            ));
        }
    }
    fill_f64_with_interrupt(
        &mut workspace.worker_mean,
        0.0,
        interrupt,
        "model_batch_zero_workspace",
    )?;
    fill_f64_with_interrupt(output, 0.0, interrupt, "model_batch_zero_output")?;
    let firms = operator.firms();
    let workers = operator.workers();
    let controls = operator.controls();
    for column in 0..columns {
        let begin = column * dimension;
        workspace.firm_mean[column] = firm_mean_with_interrupt(
            &input[begin..begin + firms],
            interrupt,
            "model_batch_firm_mean",
        )?;
    }

    let pair_work = checked_matrix_length(
        operator.pair_weight().len(),
        columns,
        "model pair-column work",
    )?;
    for column in 0..columns {
        let parameter_begin = column * dimension;
        let worker_begin = column * workers;
        let pair_begin = column * operator.pair_weight().len();
        for pair in 0..operator.pair_weight().len() {
            checkpoint_chunk(interrupt, pair_begin + pair, "model_batch_worker_pairs")?;
            let worker = dense_index(operator.pair_worker()[pair]);
            let firm = dense_index(operator.pair_firm()[pair]);
            workspace.worker_mean[worker_begin + worker] += operator.pair_weight()[pair]
                * (input[parameter_begin + firm] - workspace.firm_mean[column]);
        }
        for control in 0..controls {
            interrupt.checkpoint("model_batch_worker_controls")?;
            let coefficient = input[parameter_begin + firms + control];
            let control_begin = control * workers;
            for worker in 0..workers {
                checkpoint_chunk(interrupt, worker, "model_batch_worker_controls")?;
                workspace.worker_mean[worker_begin + worker] +=
                    operator.worker_control()[control_begin + worker] * coefficient;
            }
        }
    }
    debug_assert_eq!(pair_work, operator.pair_weight().len() * columns);
    for column in 0..columns {
        for worker in 0..workers {
            let index = column * workers + worker;
            checkpoint_chunk(interrupt, index, "model_batch_worker_scale")?;
            workspace.worker_mean[index] /= operator.worker_diagonal()[worker];
        }
    }

    for column in 0..columns {
        let parameter_begin = column * dimension;
        let worker_begin = column * workers;
        for firm in 0..firms {
            checkpoint_chunk(interrupt, firm, "model_batch_direct_firm")?;
            output[parameter_begin + firm] = operator.firm_diagonal()[firm]
                * (input[parameter_begin + firm] - workspace.firm_mean[column]);
        }
        for control in 0..controls {
            interrupt.checkpoint("model_batch_firm_controls")?;
            let coefficient = input[parameter_begin + firms + control];
            let control_begin = control * firms;
            for firm in 0..firms {
                checkpoint_chunk(interrupt, firm, "model_batch_firm_controls")?;
                output[parameter_begin + firm] +=
                    operator.firm_control()[control_begin + firm] * coefficient;
            }
        }
        for pair in 0..operator.pair_weight().len() {
            checkpoint_chunk(interrupt, pair, "model_batch_absorb_firm")?;
            let worker = dense_index(operator.pair_worker()[pair]);
            let firm = dense_index(operator.pair_firm()[pair]);
            output[parameter_begin + firm] -=
                operator.pair_weight()[pair] * workspace.worker_mean[worker_begin + worker];
        }
        for control in 0..controls {
            interrupt.checkpoint("model_batch_control_output")?;
            let mut value = 0.0;
            let firm_control_begin = control * firms;
            for firm in 0..firms {
                checkpoint_chunk(interrupt, firm, "model_batch_control_output")?;
                value += operator.firm_control()[firm_control_begin + firm]
                    * (input[parameter_begin + firm] - workspace.firm_mean[column]);
            }
            let cross_begin = control * controls;
            for right in 0..controls {
                checkpoint_chunk(interrupt, right, "model_batch_control_output")?;
                value += operator.control_cross()[cross_begin + right]
                    * input[parameter_begin + firms + right];
            }
            let worker_control_begin = control * workers;
            for worker in 0..workers {
                checkpoint_chunk(interrupt, worker, "model_batch_control_output")?;
                value -= operator.worker_control()[worker_control_begin + worker]
                    * workspace.worker_mean[worker_begin + worker];
            }
            output[parameter_begin + firms + control] = value;
        }
    }
    for column in 0..columns {
        interrupt.checkpoint("model_batch_project")?;
        operator.project_parameters_with_interrupt(
            &mut output[column_range(column, dimension)],
            interrupt,
        )?;
    }
    validate_internal_finite(
        output,
        interrupt,
        "model_batch_validate_output",
        "model batch Schur action produced a nonfinite value",
    )?;
    Ok(())
}

pub fn model_batched_pcg(
    operator: &ModelOperator<'_>,
    preconditioner: &(impl ModelPreconditioner + ?Sized),
    right_hand_side: &[f64],
    columns: usize,
    options: PcgOptions,
) -> Result<ModelBatchedPcgSolve> {
    model_batched_pcg_with_interrupt(
        operator,
        preconditioner,
        right_hand_side,
        columns,
        options,
        &mut NeverInterrupt,
    )
}

#[allow(clippy::too_many_lines)]
pub fn model_batched_pcg_with_interrupt(
    operator: &ModelOperator<'_>,
    preconditioner: &(impl ModelPreconditioner + ?Sized),
    right_hand_side: &[f64],
    columns: usize,
    options: PcgOptions,
    interrupt: &mut dyn InterruptCheck,
) -> Result<ModelBatchedPcgSolve> {
    let options = options.validate()?;
    let dimension = operator.parameter_count();
    let expected = checked_matrix_length(dimension, columns, "model PCG batch")?;
    if columns == 0 || right_hand_side.len() != expected || preconditioner.dimension() != dimension
    {
        return Err(BackendError::invalid(
            "model_batch_pcg",
            "operator, diagonal, and model RHS dimensions are incompatible",
        ));
    }
    validate_finite_with_interrupt(right_hand_side, interrupt, "model_batch_pcg_validate_rhs")?;

    let mut projected_rhs = copy_f64_with_interrupt(
        right_hand_side,
        "model projected RHS",
        interrupt,
        "model_batch_pcg_copy_rhs",
    )?;
    for column in 0..columns {
        operator.project_parameters_with_interrupt(
            &mut projected_rhs[column_range(column, dimension)],
            interrupt,
        )?;
    }
    let mut workspace = ModelBatchWorkspace::new_with_interrupt(operator, columns, interrupt)?;
    let mut solution = zeroed_f64_with_interrupt(
        expected,
        "model PCG solution",
        interrupt,
        "model_batch_pcg_initialize",
    )?;
    let mut residual = copy_f64_with_interrupt(
        &projected_rhs,
        "model PCG residual",
        interrupt,
        "model_batch_pcg_initialize",
    )?;
    let mut preconditioned = zeroed_f64_with_interrupt(
        expected,
        "model PCG preconditioned residual",
        interrupt,
        "model_batch_pcg_initialize",
    )?;
    let mut direction = zeroed_f64_with_interrupt(
        expected,
        "model PCG direction",
        interrupt,
        "model_batch_pcg_initialize",
    )?;
    let mut action = zeroed_f64_with_interrupt(
        expected,
        "model PCG action",
        interrupt,
        "model_batch_pcg_initialize",
    )?;
    let mut verified_action = zeroed_f64_with_interrupt(
        expected,
        "model PCG verified action",
        interrupt,
        "model_batch_pcg_initialize",
    )?;
    let mut active = repeated_with_interrupt(
        columns,
        true,
        "model PCG active flags",
        interrupt,
        "model_batch_pcg_initialize",
    )?;
    let mut restarted = repeated_with_interrupt(
        columns,
        false,
        "model PCG restart flags",
        interrupt,
        "model_batch_pcg_initialize",
    )?;
    let mut candidates = repeated_with_interrupt(
        columns,
        false,
        "model PCG candidate flags",
        interrupt,
        "model_batch_pcg_initialize",
    )?;
    let mut rhs_norm = zeroed_f64_with_interrupt(
        columns,
        "model PCG RHS norms",
        interrupt,
        "model_batch_pcg_initialize",
    )?;
    let mut residual_product = zeroed_f64_with_interrupt(
        columns,
        "model PCG residual products",
        interrupt,
        "model_batch_pcg_initialize",
    )?;
    let mut relative_residual = zeroed_f64_with_interrupt(
        columns,
        "model PCG relative residuals",
        interrupt,
        "model_batch_pcg_initialize",
    )?;
    let mut operator_applications = repeated_with_interrupt(
        columns,
        0_u32,
        "model PCG operator counters",
        interrupt,
        "model_batch_pcg_initialize",
    )?;
    let mut preconditioner_applications = repeated_with_interrupt(
        columns,
        0_u32,
        "model PCG preconditioner counters",
        interrupt,
        "model_batch_pcg_initialize",
    )?;
    let mut residual_replacements = repeated_with_interrupt(
        columns,
        0_u32,
        "model PCG replacement counters",
        interrupt,
        "model_batch_pcg_initialize",
    )?;
    let mut receipts = repeated_with_interrupt(
        columns,
        None::<ModelPcgReceipt>,
        "model PCG receipts",
        interrupt,
        "model_batch_pcg_initialize",
    )?;

    for column in 0..columns {
        let range = column_range(column, dimension);
        rhs_norm[column] = stable_norm_with_interrupt(
            &projected_rhs[range],
            interrupt,
            "model_batch_pcg_rhs_norm",
        )?;
        if rhs_norm[column] == 0.0 {
            active[column] = false;
            receipts[column] = Some(zero_receipt());
        }
    }
    if active.iter().all(|value| !*value) {
        return Ok(ModelBatchedPcgSolve {
            dimension,
            columns,
            solution,
            receipt: collect_receipts(receipts, interrupt)?,
        });
    }

    apply_preconditioner(
        operator,
        preconditioner,
        &residual,
        &mut preconditioned,
        dimension,
        &active,
        &mut preconditioner_applications,
        interrupt,
    )?;
    copy_into_with_interrupt(
        &preconditioned,
        &mut direction,
        interrupt,
        "model_batch_pcg_initial_direction",
    )?;
    for column in 0..columns {
        if !active[column] {
            continue;
        }
        let range = column_range(column, dimension);
        residual_product[column] = stable_dot_with_interrupt(
            &residual[range.clone()],
            &preconditioned[range],
            interrupt,
            "model_batch_pcg_initial_dot",
        )?;
        require_positive(
            residual_product[column],
            ErrorCode::PcgPreconditionerBreakdown,
            column,
            "initial preconditioned residual has nonpositive curvature",
        )?;
    }

    for iteration in 1..=options.maximum_iterations {
        interrupt.checkpoint("model_batch_pcg_iteration")?;
        apply_model_batch_with_interrupt(
            operator,
            &direction,
            &mut action,
            columns,
            &mut workspace,
            interrupt,
        )?;
        increment_active(&mut operator_applications, &active)?;
        for column in 0..columns {
            if !active[column] {
                continue;
            }
            let range = column_range(column, dimension);
            let curvature = stable_dot_with_interrupt(
                &direction[range.clone()],
                &action[range.clone()],
                interrupt,
                "model_batch_pcg_curvature",
            )?;
            require_positive(
                curvature,
                ErrorCode::PcgCurvatureBreakdown,
                column,
                "search direction has nonpositive model curvature",
            )?;
            let alpha = residual_product[column] / curvature;
            if !alpha.is_finite() {
                return Err(column_error(
                    ErrorCode::PcgCurvatureBreakdown,
                    column,
                    "model PCG step length is nonfinite",
                ));
            }
            for index in range {
                checkpoint_chunk(interrupt, index, "model_batch_pcg_recurrence")?;
                solution[index] += alpha * direction[index];
                residual[index] -= alpha * action[index];
            }
            operator.project_parameters_with_interrupt(
                &mut residual[column_range(column, dimension)],
                interrupt,
            )?;
        }
        validate_pcg_finite(&solution, interrupt, "model_batch_pcg_validate_recurrence")?;
        validate_pcg_finite(&residual, interrupt, "model_batch_pcg_validate_recurrence")?;

        fill_with_interrupt(
            &mut restarted,
            false,
            interrupt,
            "model_batch_pcg_restart_flags",
        )?;
        let recomputed = iteration % options.residual_replacement_interval == 0;
        if recomputed {
            recompute_selected_explicit(
                operator,
                &projected_rhs,
                &solution,
                &mut verified_action,
                columns,
                dimension,
                &active,
                &mut operator_applications,
                &mut residual_replacements,
                &mut workspace,
                interrupt,
            )?;
            for column in 0..columns {
                if !active[column] {
                    continue;
                }
                let range = column_range(column, dimension);
                let explicit_norm = stable_norm_with_interrupt(
                    &verified_action[range.clone()],
                    interrupt,
                    "model_batch_pcg_explicit_norm",
                )?;
                let drift_norm = stable_difference_norm_with_interrupt(
                    &verified_action[range.clone()],
                    &residual[range.clone()],
                    interrupt,
                )?;
                if !explicit_norm.is_finite() || !drift_norm.is_finite() {
                    return Err(column_error(
                        ErrorCode::PcgStagnation,
                        column,
                        "explicit model PCG residual or recurrence drift is nonfinite",
                    ));
                }
                let drift_gate =
                    (1.0e-14 * rhs_norm[column]).max(0.1 * options.tolerance * rhs_norm[column]);
                if explicit_norm <= options.tolerance * rhs_norm[column] || drift_norm > drift_gate
                {
                    copy_into_with_interrupt(
                        &verified_action[range.clone()],
                        &mut residual[range],
                        interrupt,
                        "model_batch_pcg_conditional_restart",
                    )?;
                    restarted[column] = true;
                }
            }
        }

        fill_with_interrupt(
            &mut candidates,
            false,
            interrupt,
            "model_batch_pcg_candidate_flags",
        )?;
        for column in 0..columns {
            if !active[column] {
                continue;
            }
            let range = column_range(column, dimension);
            relative_residual[column] = stable_norm_with_interrupt(
                &residual[range],
                interrupt,
                "model_batch_pcg_residual_norm",
            )? / rhs_norm[column];
            if !relative_residual[column].is_finite() {
                return Err(column_error(
                    ErrorCode::PcgStagnation,
                    column,
                    "model PCG relative residual is nonfinite",
                ));
            }
            candidates[column] = relative_residual[column] <= options.tolerance;
        }
        if candidates.iter().any(|value| *value) {
            if !recomputed {
                recompute_selected_explicit(
                    operator,
                    &projected_rhs,
                    &solution,
                    &mut verified_action,
                    columns,
                    dimension,
                    &candidates,
                    &mut operator_applications,
                    &mut residual_replacements,
                    &mut workspace,
                    interrupt,
                )?;
            }
            for column in 0..columns {
                if !candidates[column] {
                    continue;
                }
                let range = column_range(column, dimension);
                let verified = stable_norm_with_interrupt(
                    &verified_action[range.clone()],
                    interrupt,
                    "model_batch_pcg_verified_norm",
                )? / rhs_norm[column];
                if !verified.is_finite() {
                    return Err(column_error(
                        ErrorCode::PcgStagnation,
                        column,
                        "verified model PCG residual is nonfinite",
                    ));
                }
                relative_residual[column] = verified;
                if verified <= options.tolerance {
                    active[column] = false;
                    fill_f64_with_interrupt(
                        &mut direction[range],
                        0.0,
                        interrupt,
                        "model_batch_pcg_zero_direction",
                    )?;
                    receipts[column] = Some(ModelPcgReceipt {
                        status: ModelPcgStatus::Converged,
                        iterations: iteration,
                        relative_residual: verified,
                        residual_replacements: residual_replacements[column],
                        operator_applications: operator_applications[column],
                        preconditioner_applications: preconditioner_applications[column],
                    });
                } else {
                    copy_into_with_interrupt(
                        &verified_action[range.clone()],
                        &mut residual[range],
                        interrupt,
                        "model_batch_pcg_failed_verification_restart",
                    )?;
                    restarted[column] = true;
                }
            }
        }
        if active.iter().all(|value| !*value) {
            for column in 0..columns {
                operator.project_parameters_with_interrupt(
                    &mut solution[column_range(column, dimension)],
                    interrupt,
                )?;
            }
            return Ok(ModelBatchedPcgSolve {
                dimension,
                columns,
                solution,
                receipt: collect_receipts(receipts, interrupt)?,
            });
        }

        apply_preconditioner(
            operator,
            preconditioner,
            &residual,
            &mut preconditioned,
            dimension,
            &active,
            &mut preconditioner_applications,
            interrupt,
        )?;
        for column in 0..columns {
            if !active[column] {
                continue;
            }
            let range = column_range(column, dimension);
            let next_product = stable_dot_with_interrupt(
                &residual[range.clone()],
                &preconditioned[range.clone()],
                interrupt,
                "model_batch_pcg_preconditioned_dot",
            )?;
            require_positive(
                next_product,
                ErrorCode::PcgPreconditionerBreakdown,
                column,
                "preconditioned model residual has nonpositive curvature",
            )?;
            if restarted[column] {
                copy_into_with_interrupt(
                    &preconditioned[range.clone()],
                    &mut direction[range],
                    interrupt,
                    "model_batch_pcg_restart_direction",
                )?;
            } else {
                let beta = next_product / residual_product[column];
                if !beta.is_finite() || beta < 0.0 {
                    return Err(column_error(
                        ErrorCode::PcgPreconditionerBreakdown,
                        column,
                        "model PCG direction coefficient is invalid",
                    ));
                }
                for index in range {
                    checkpoint_chunk(interrupt, index, "model_batch_pcg_direction")?;
                    direction[index] = preconditioned[index] + beta * direction[index];
                }
            }
            residual_product[column] = next_product;
        }
    }

    let first = active
        .iter()
        .position(|value| *value)
        .expect("at least one unconverged model RHS");
    Err(column_error(
        ErrorCode::PcgMaxIterations,
        first,
        &format!(
            "model PCG did not converge in {} iterations; final reduced residual was {}",
            options.maximum_iterations, relative_residual[first]
        ),
    ))
}

#[allow(clippy::too_many_arguments)]
fn apply_preconditioner(
    operator: &ModelOperator<'_>,
    preconditioner: &(impl ModelPreconditioner + ?Sized),
    input: &[f64],
    output: &mut [f64],
    dimension: usize,
    active: &[bool],
    applications: &mut [u32],
    interrupt: &mut dyn InterruptCheck,
) -> Result<()> {
    fill_f64_with_interrupt(output, 0.0, interrupt, "model_batch_preconditioner_zero")?;
    for (column, &is_active) in active.iter().enumerate() {
        if !is_active {
            continue;
        }
        let range = column_range(column, dimension);
        preconditioner.apply_with_interrupt(
            &input[range.clone()],
            &mut output[range.clone()],
            interrupt,
        )?;
        operator.project_parameters_with_interrupt(&mut output[range], interrupt)?;
        applications[column] = applications[column]
            .checked_add(1)
            .ok_or_else(|| resource_error("model preconditioner counter overflow"))?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn recompute_selected_explicit(
    operator: &ModelOperator<'_>,
    right_hand_side: &[f64],
    solution: &[f64],
    explicit_residual: &mut [f64],
    columns: usize,
    dimension: usize,
    selected: &[bool],
    operator_applications: &mut [u32],
    residual_replacements: &mut [u32],
    workspace: &mut ModelBatchWorkspace,
    interrupt: &mut dyn InterruptCheck,
) -> Result<()> {
    interrupt.checkpoint("model_batch_residual_replacement")?;
    apply_model_batch_with_interrupt(
        operator,
        solution,
        explicit_residual,
        columns,
        workspace,
        interrupt,
    )?;
    for (column, &is_selected) in selected.iter().enumerate() {
        if !is_selected {
            continue;
        }
        let range = column_range(column, dimension);
        for index in range.clone() {
            checkpoint_chunk(interrupt, index, "model_batch_residual_replacement")?;
            explicit_residual[index] = right_hand_side[index] - explicit_residual[index];
        }
        operator.project_parameters_with_interrupt(&mut explicit_residual[range], interrupt)?;
        operator_applications[column] = operator_applications[column]
            .checked_add(1)
            .ok_or_else(|| resource_error("model operator counter overflow"))?;
        residual_replacements[column] = residual_replacements[column]
            .checked_add(1)
            .ok_or_else(|| resource_error("model replacement counter overflow"))?;
    }
    Ok(())
}

fn stable_difference_norm_with_interrupt(
    left: &[f64],
    right: &[f64],
    interrupt: &mut dyn InterruptCheck,
) -> Result<f64> {
    let mut sum = 0.0;
    let mut correction = 0.0;
    for (index, (&left_value, &right_value)) in left.iter().zip(right).enumerate() {
        checkpoint_chunk(interrupt, index, "model_batch_pcg_drift_norm")?;
        let difference = left_value - right_value;
        let square = difference * difference;
        let adjusted = square - correction;
        let next = sum + adjusted;
        correction = (next - sum) - adjusted;
        sum = next;
    }
    Ok(sum.max(0.0).sqrt())
}

fn zero_receipt() -> ModelPcgReceipt {
    ModelPcgReceipt {
        status: ModelPcgStatus::ZeroRhs,
        iterations: 0,
        relative_residual: 0.0,
        residual_replacements: 0,
        operator_applications: 0,
        preconditioner_applications: 0,
    }
}

fn collect_receipts(
    values: Vec<Option<ModelPcgReceipt>>,
    interrupt: &mut dyn InterruptCheck,
) -> Result<Vec<ModelPcgReceipt>> {
    let mut receipts = Vec::new();
    reserve_exact(&mut receipts, values.len(), "model collected PCG receipts")?;
    for (column, value) in values.into_iter().enumerate() {
        checkpoint_chunk(interrupt, column, "model_batch_collect_receipts")?;
        receipts.push(value.ok_or_else(|| {
            BackendError::invariant(
                "model_batch_pcg",
                format!("missing receipt for zero-based model RHS column {column}"),
            )
        })?);
    }
    Ok(receipts)
}

fn increment_active(counters: &mut [u32], active: &[bool]) -> Result<()> {
    for (counter, &is_active) in counters.iter_mut().zip(active) {
        if is_active {
            *counter = counter
                .checked_add(1)
                .ok_or_else(|| resource_error("model PCG application counter overflow"))?;
        }
    }
    Ok(())
}

fn require_positive(value: f64, code: ErrorCode, column: usize, message: &str) -> Result<()> {
    if value.is_finite() && value > 0.0 {
        Ok(())
    } else {
        Err(column_error(code, column, message))
    }
}

fn column_error(code: ErrorCode, column: usize, message: &str) -> BackendError {
    BackendError::new(
        code,
        "model_batch_pcg",
        format!("zero-based RHS column {column}: {message}"),
    )
}

fn column_range(column: usize, dimension: usize) -> core::ops::Range<usize> {
    let begin = column * dimension;
    begin..begin + dimension
}

fn dense_index(value: u32) -> usize {
    usize::try_from(value).expect("validated dense model index")
}

fn repeated_with_interrupt<T: Clone>(
    length: usize,
    value: T,
    label: &str,
    interrupt: &mut dyn InterruptCheck,
    phase: &'static str,
) -> Result<Vec<T>> {
    let mut values = Vec::new();
    reserve_exact(&mut values, length, label)?;
    for index in 0..length {
        checkpoint_chunk(interrupt, index, phase)?;
        values.push(value.clone());
    }
    Ok(values)
}

fn fill_f64_with_interrupt(
    values: &mut [f64],
    value: f64,
    interrupt: &mut dyn InterruptCheck,
    phase: &'static str,
) -> Result<()> {
    for (index, output) in values.iter_mut().enumerate() {
        checkpoint_chunk(interrupt, index, phase)?;
        *output = value;
    }
    Ok(())
}

fn fill_with_interrupt<T: Clone>(
    values: &mut [T],
    value: T,
    interrupt: &mut dyn InterruptCheck,
    phase: &'static str,
) -> Result<()> {
    for (index, output) in values.iter_mut().enumerate() {
        checkpoint_chunk(interrupt, index, phase)?;
        output.clone_from(&value);
    }
    Ok(())
}

fn stable_norm_with_interrupt(
    values: &[f64],
    interrupt: &mut dyn InterruptCheck,
    phase: &'static str,
) -> Result<f64> {
    Ok(stable_dot_with_interrupt(values, values, interrupt, phase)?
        .max(0.0)
        .sqrt())
}

fn validate_internal_finite(
    values: &[f64],
    interrupt: &mut dyn InterruptCheck,
    phase: &'static str,
    message: &'static str,
) -> Result<()> {
    for (index, value) in values.iter().enumerate() {
        checkpoint_chunk(interrupt, index, phase)?;
        if !value.is_finite() {
            return Err(invariant_nonfinite(message));
        }
    }
    Ok(())
}

fn validate_pcg_finite(
    values: &[f64],
    interrupt: &mut dyn InterruptCheck,
    phase: &'static str,
) -> Result<()> {
    for (index, value) in values.iter().enumerate() {
        checkpoint_chunk(interrupt, index, phase)?;
        if !value.is_finite() {
            return Err(BackendError::new(
                ErrorCode::PcgStagnation,
                "model_batch_pcg",
                "model PCG recurrence produced a nonfinite value",
            ));
        }
    }
    Ok(())
}

fn invariant_nonfinite(message: &'static str) -> BackendError {
    BackendError::new(
        ErrorCode::InternalInvariantFailed,
        "model_batch_operator",
        message,
    )
}

fn resource_error(message: &str) -> BackendError {
    BackendError::new(ErrorCode::ResourceLimit, "model_batch_resource", message)
}
