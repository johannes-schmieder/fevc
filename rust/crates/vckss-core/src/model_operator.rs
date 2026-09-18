// SPDX-License-Identifier: GPL-3.0-only

//! Matrix-free worker--firm--control normal-equation operator.
//!
//! Firms are represented in the full, unweighted zero-sum quotient and
//! controls are supplied in their already-canonical, column-major basis.  This
//! module deliberately does not choose or construct that basis: callers that
//! begin with raw controls must use [`crate::control_basis`] before preparing
//! this operator.  The certified first implementation supports at most 32
//! controls.

use crate::control_basis::MAX_CANONICAL_CONTROLS;
use crate::error::{BackendError, ErrorCode, Result};
use crate::interrupt::{
    checkpoint_chunk, stable_sort_by_with_interrupt, InterruptCheck, NeverInterrupt,
    INTERRUPT_CHECK_CHUNK,
};
use crate::operator::SymmetricOperator;

const RHS_COMPATIBILITY_FACTOR: f64 = 128.0;

/// Borrowed, row-level model columns. Worker and firm indices must be dense,
/// zero based, and cover the declared dimensions. Controls are canonical and
/// stored one complete observation column at a time.
#[derive(Clone, Copy, Debug)]
pub struct CanonicalModelData<'a> {
    pub workers: usize,
    pub firms: usize,
    pub row_worker: &'a [u32],
    pub row_firm: &'a [u32],
    pub weight: &'a [f64],
    pub controls: &'a [Vec<f64>],
}

impl CanonicalModelData<'_> {
    #[must_use]
    pub fn rows(self) -> usize {
        self.weight.len()
    }

    #[must_use]
    pub fn control_count(self) -> usize {
        self.controls.len()
    }
}

/// Original, unreduced W+F+Q right-hand side.
#[derive(Clone, Copy, Debug)]
pub struct ModelRhs<'a> {
    pub worker: &'a [f64],
    pub firm: &'a [f64],
    pub control: &'a [f64],
}

/// Complete original-system residual. `relative_norm` is an absolute norm
/// when the complete original RHS is exactly zero.
#[derive(Clone, Debug)]
pub struct ModelResidual {
    pub worker: Vec<f64>,
    pub firm: Vec<f64>,
    pub control: Vec<f64>,
    pub absolute_norm: f64,
    pub relative_norm: f64,
    pub rhs_norm: f64,
}

/// Reusable scalar action workspace. Its only row-scaled allocation is W.
#[derive(Clone, Debug)]
pub struct ModelWorkspace {
    worker_mean: Vec<f64>,
}

impl ModelWorkspace {
    pub fn new(operator: &ModelOperator<'_>) -> Result<Self> {
        Self::new_with_interrupt(operator, &mut NeverInterrupt)
    }

    pub fn new_with_interrupt(
        operator: &ModelOperator<'_>,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<Self> {
        Ok(Self {
            worker_mean: zeroed_f64_with_interrupt(
                operator.workers(),
                "model scalar worker workspace",
                interrupt,
                "model_workspace_initialize",
            )?,
        })
    }
}

/// Matrix-free Schur operator after exact elimination of worker effects.
#[derive(Clone, Debug)]
pub struct ModelOperator<'a> {
    data: CanonicalModelData<'a>,
    /// Canonical row traversal makes observation relabeling deterministic when
    /// complete row values agree.
    row_order: Vec<usize>,
    worker_diagonal: Vec<f64>,
    firm_diagonal: Vec<f64>,
    reduced_diagonal: Vec<f64>,
    sufficient: ModelSufficientStatistics,
}

#[derive(Clone, Debug)]
struct ModelSufficientStatistics {
    pair_worker: Vec<u32>,
    pair_firm: Vec<u32>,
    pair_weight: Vec<f64>,
    worker_control: Vec<f64>,
    firm_control: Vec<f64>,
    control_cross: Vec<f64>,
}

impl ModelSufficientStatistics {
    fn build(
        data: CanonicalModelData<'_>,
        row_order: &[usize],
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<Self> {
        let mut pair_count = 0_usize;
        let mut previous = None;
        for (position, &row) in row_order.iter().enumerate() {
            checkpoint_chunk(interrupt, position, "model_sufficient_pair_count")?;
            let key = (data.row_worker[row], data.row_firm[row]);
            if previous != Some(key) {
                pair_count = pair_count
                    .checked_add(1)
                    .ok_or_else(|| resource_error("model sufficient pair count overflow"))?;
                previous = Some(key);
            }
        }
        let mut pair_worker = Vec::new();
        let mut pair_firm = Vec::new();
        let mut pair_weight = Vec::new();
        reserve_exact(
            &mut pair_worker,
            pair_count,
            "model sufficient pair workers",
        )?;
        reserve_exact(&mut pair_firm, pair_count, "model sufficient pair firms")?;
        reserve_exact(
            &mut pair_weight,
            pair_count,
            "model sufficient pair weights",
        )?;
        let mut begin = 0_usize;
        while begin < row_order.len() {
            interrupt.checkpoint("model_sufficient_pairs")?;
            let first = row_order[begin];
            let worker = data.row_worker[first];
            let firm = data.row_firm[first];
            let mut end = begin + 1;
            let mut weight = data.weight[first];
            while end < row_order.len()
                && data.row_worker[row_order[end]] == worker
                && data.row_firm[row_order[end]] == firm
            {
                checkpoint_chunk(interrupt, end - begin, "model_sufficient_pairs")?;
                weight += data.weight[row_order[end]];
                end += 1;
            }
            if !weight.is_finite() || weight <= 0.0 {
                return Err(invariant_nonfinite(
                    "model_sufficient_pairs",
                    "model sufficient pair weight is not positive and finite",
                ));
            }
            pair_worker.push(worker);
            pair_firm.push(firm);
            pair_weight.push(weight);
            begin = end;
        }

        let controls = data.control_count();
        let worker_control_length = checked_matrix_length(
            data.workers,
            controls,
            "model worker-control sufficient statistics",
        )?;
        let firm_control_length = checked_matrix_length(
            data.firms,
            controls,
            "model firm-control sufficient statistics",
        )?;
        let control_cross_length = checked_matrix_length(
            controls,
            controls,
            "model control-cross sufficient statistics",
        )?;
        let _ = checked_matrix_length(
            data.rows(),
            control_cross_length.max(1),
            "model sufficient-stat setup work",
        )?;
        let mut worker_control = zeroed_f64_with_interrupt(
            worker_control_length,
            "model worker-control sufficient statistics",
            interrupt,
            "model_sufficient_initialize",
        )?;
        let mut firm_control = zeroed_f64_with_interrupt(
            firm_control_length,
            "model firm-control sufficient statistics",
            interrupt,
            "model_sufficient_initialize",
        )?;
        let mut control_cross = zeroed_f64_with_interrupt(
            control_cross_length,
            "model control-cross sufficient statistics",
            interrupt,
            "model_sufficient_initialize",
        )?;
        let mut work = 0_usize;
        for &row in row_order {
            let worker = dense_index(data.row_worker[row]);
            let firm = dense_index(data.row_firm[row]);
            for left in 0..controls {
                checkpoint_chunk(interrupt, work, "model_sufficient_controls")?;
                let weighted = data.weight[row] * data.controls[left][row];
                worker_control[left * data.workers + worker] += weighted;
                firm_control[left * data.firms + firm] += weighted;
                for right in 0..controls {
                    work = work.saturating_add(1);
                    checkpoint_chunk(interrupt, work, "model_sufficient_controls")?;
                    control_cross[left * controls + right] += weighted * data.controls[right][row];
                }
                work = work.saturating_add(1);
            }
        }
        if worker_control
            .iter()
            .chain(&firm_control)
            .chain(&control_cross)
            .any(|value| !value.is_finite())
        {
            return Err(invariant_nonfinite(
                "model_sufficient_controls",
                "model sufficient control statistic is nonfinite",
            ));
        }
        Ok(Self {
            pair_worker,
            pair_firm,
            pair_weight,
            worker_control,
            firm_control,
            control_cross,
        })
    }
}

impl<'a> ModelOperator<'a> {
    pub fn new(data: CanonicalModelData<'a>) -> Result<Self> {
        Self::new_with_interrupt(data, &mut NeverInterrupt)
    }

    pub fn new_with_interrupt(
        data: CanonicalModelData<'a>,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<Self> {
        interrupt.checkpoint("model_operator_setup")?;
        validate_shape(data, interrupt)?;
        let row_order = canonical_row_order(data, interrupt)?;
        let (worker_diagonal, firm_diagonal) = information_diagonals(data, &row_order, interrupt)?;
        require_connected(data, &row_order, interrupt)?;
        let reduced_diagonal = reduced_diagonal(
            data,
            &row_order,
            &worker_diagonal,
            &firm_diagonal,
            interrupt,
        )?;
        let sufficient = ModelSufficientStatistics::build(data, &row_order, interrupt)?;
        Ok(Self {
            data,
            row_order,
            worker_diagonal,
            firm_diagonal,
            reduced_diagonal,
            sufficient,
        })
    }

    #[must_use]
    pub fn data(&self) -> CanonicalModelData<'a> {
        self.data
    }

    #[must_use]
    pub fn rows(&self) -> usize {
        self.data.rows()
    }

    #[must_use]
    pub fn workers(&self) -> usize {
        self.data.workers
    }

    #[must_use]
    pub fn firms(&self) -> usize {
        self.data.firms
    }

    #[must_use]
    pub fn controls(&self) -> usize {
        self.data.control_count()
    }

    #[must_use]
    pub fn parameter_count(&self) -> usize {
        self.firms() + self.controls()
    }

    #[must_use]
    pub fn worker_diagonal(&self) -> &[f64] {
        &self.worker_diagonal
    }

    #[must_use]
    pub fn firm_diagonal(&self) -> &[f64] {
        &self.firm_diagonal
    }

    /// Exact diagonal of the reduced, unregularized Schur operator in full
    /// zero-sum-firm plus canonical-control coordinates.
    #[must_use]
    pub fn reduced_diagonal(&self) -> &[f64] {
        &self.reduced_diagonal
    }

    #[must_use]
    pub(crate) fn row_order(&self) -> &[usize] {
        &self.row_order
    }

    #[must_use]
    pub(crate) fn pair_worker(&self) -> &[u32] {
        &self.sufficient.pair_worker
    }

    #[must_use]
    pub(crate) fn pair_firm(&self) -> &[u32] {
        &self.sufficient.pair_firm
    }

    #[must_use]
    pub(crate) fn pair_weight(&self) -> &[f64] {
        &self.sufficient.pair_weight
    }

    #[must_use]
    pub(crate) fn worker_control(&self) -> &[f64] {
        &self.sufficient.worker_control
    }

    #[must_use]
    pub(crate) fn firm_control(&self) -> &[f64] {
        &self.sufficient.firm_control
    }

    #[must_use]
    pub(crate) fn control_cross(&self) -> &[f64] {
        &self.sufficient.control_cross
    }

    pub fn project_parameters(&self, values: &mut [f64]) -> Result<()> {
        self.project_parameters_with_interrupt(values, &mut NeverInterrupt)
    }

    pub fn project_parameters_with_interrupt(
        &self,
        values: &mut [f64],
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<()> {
        if values.len() != self.parameter_count() {
            return Err(BackendError::invalid(
                "model_operator",
                "model quotient projection has incompatible dimensions",
            ));
        }
        validate_finite_with_interrupt(values, interrupt, "model_operator_project")?;
        center_firms_with_interrupt(
            &mut values[..self.firms()],
            interrupt,
            "model_operator_project",
        )
    }

    pub fn apply_with_workspace(
        &self,
        input: &[f64],
        output: &mut [f64],
        workspace: &mut ModelWorkspace,
    ) -> Result<()> {
        self.apply_with_workspace_and_interrupt(input, output, workspace, &mut NeverInterrupt)
    }

    pub fn apply_with_workspace_and_interrupt(
        &self,
        input: &[f64],
        output: &mut [f64],
        workspace: &mut ModelWorkspace,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<()> {
        validate_parameter_vectors(self, input, output, interrupt, "model_operator_apply")?;
        if workspace.worker_mean.len() != self.workers() {
            return Err(BackendError::invalid(
                "model_operator_apply",
                "model scalar workspace has incompatible dimensions",
            ));
        }
        let firm_mean = firm_mean_with_interrupt(
            &input[..self.firms()],
            interrupt,
            "model_operator_firm_mean",
        )?;
        fill_f64_with_interrupt(
            &mut workspace.worker_mean,
            0.0,
            interrupt,
            "model_operator_zero_workspace",
        )?;
        fill_f64_with_interrupt(output, 0.0, interrupt, "model_operator_zero_output")?;

        for pair in 0..self.sufficient.pair_weight.len() {
            checkpoint_chunk(interrupt, pair, "model_operator_worker_accumulate")?;
            let worker = dense_index(self.sufficient.pair_worker[pair]);
            let firm = dense_index(self.sufficient.pair_firm[pair]);
            workspace.worker_mean[worker] +=
                self.sufficient.pair_weight[pair] * (input[firm] - firm_mean);
        }
        for control in 0..self.controls() {
            interrupt.checkpoint("model_operator_worker_controls")?;
            let coefficient = input[self.firms() + control];
            let begin = control * self.workers();
            for worker in 0..self.workers() {
                checkpoint_chunk(interrupt, worker, "model_operator_worker_controls")?;
                workspace.worker_mean[worker] +=
                    self.sufficient.worker_control[begin + worker] * coefficient;
            }
        }
        for (worker, value) in workspace.worker_mean.iter_mut().enumerate() {
            checkpoint_chunk(interrupt, worker, "model_operator_worker_scale")?;
            *value /= self.worker_diagonal[worker];
        }

        for firm in 0..self.firms() {
            checkpoint_chunk(interrupt, firm, "model_operator_direct_firm")?;
            output[firm] = self.firm_diagonal[firm] * (input[firm] - firm_mean);
        }
        for control in 0..self.controls() {
            interrupt.checkpoint("model_operator_firm_controls")?;
            let coefficient = input[self.firms() + control];
            let begin = control * self.firms();
            for firm in 0..self.firms() {
                checkpoint_chunk(interrupt, firm, "model_operator_firm_controls")?;
                output[firm] += self.sufficient.firm_control[begin + firm] * coefficient;
            }
        }
        for pair in 0..self.sufficient.pair_weight.len() {
            checkpoint_chunk(interrupt, pair, "model_operator_absorb_firm")?;
            let worker = dense_index(self.sufficient.pair_worker[pair]);
            let firm = dense_index(self.sufficient.pair_firm[pair]);
            output[firm] -= self.sufficient.pair_weight[pair] * workspace.worker_mean[worker];
        }
        for control in 0..self.controls() {
            interrupt.checkpoint("model_operator_control_output")?;
            let mut value = 0.0;
            let firm_begin = control * self.firms();
            for firm in 0..self.firms() {
                checkpoint_chunk(interrupt, firm, "model_operator_control_output")?;
                value +=
                    self.sufficient.firm_control[firm_begin + firm] * (input[firm] - firm_mean);
            }
            let cross_begin = control * self.controls();
            for right in 0..self.controls() {
                checkpoint_chunk(interrupt, right, "model_operator_control_output")?;
                value += self.sufficient.control_cross[cross_begin + right]
                    * input[self.firms() + right];
            }
            let worker_begin = control * self.workers();
            for worker in 0..self.workers() {
                checkpoint_chunk(interrupt, worker, "model_operator_control_output")?;
                value -= self.sufficient.worker_control[worker_begin + worker]
                    * workspace.worker_mean[worker];
            }
            output[self.firms() + control] = value;
        }
        center_firms_with_interrupt(
            &mut output[..self.firms()],
            interrupt,
            "model_operator_output_center",
        )?;
        validate_internal_finite_with_interrupt(
            output,
            interrupt,
            "model_operator_output",
            "model Schur action produced a nonfinite value",
        )?;
        Ok(())
    }

    pub fn reduce_rhs(&self, rhs: ModelRhs<'_>) -> Result<Vec<f64>> {
        self.reduce_rhs_with_interrupt(rhs, &mut NeverInterrupt)
    }

    pub fn reduce_rhs_with_interrupt(
        &self,
        rhs: ModelRhs<'_>,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<Vec<f64>> {
        validate_rhs(self, rhs, interrupt)?;
        require_compatible_rhs(rhs, interrupt)?;
        let mut worker_scaled = copy_f64_with_interrupt(
            rhs.worker,
            "model scaled worker RHS",
            interrupt,
            "model_rhs_copy_worker",
        )?;
        for (worker, value) in worker_scaled.iter_mut().enumerate() {
            checkpoint_chunk(interrupt, worker, "model_rhs_worker_scale")?;
            *value /= self.worker_diagonal[worker];
        }
        let mut reduced = zeroed_f64_with_interrupt(
            self.parameter_count(),
            "model reduced RHS",
            interrupt,
            "model_rhs_initialize",
        )?;
        copy_into_with_interrupt(
            rhs.firm,
            &mut reduced[..self.firms()],
            interrupt,
            "model_rhs_copy_firm",
        )?;
        copy_into_with_interrupt(
            rhs.control,
            &mut reduced[self.firms()..],
            interrupt,
            "model_rhs_copy_control",
        )?;
        // B' D_worker^-1 rhs needs only the already admitted weighted pair
        // and worker-control sums, not another observation scan per RHS.
        // The final certification remains independent and row-based.
        for pair in 0..self.sufficient.pair_weight.len() {
            checkpoint_chunk(interrupt, pair, "model_rhs_reduce")?;
            let worker = dense_index(self.sufficient.pair_worker[pair]);
            let firm = dense_index(self.sufficient.pair_firm[pair]);
            reduced[firm] -= self.sufficient.pair_weight[pair] * worker_scaled[worker];
        }
        for control in 0..self.controls() {
            let begin = control * self.workers();
            for (worker, &scaled) in worker_scaled.iter().enumerate() {
                checkpoint_chunk(interrupt, worker, "model_rhs_reduce")?;
                reduced[self.firms() + control] -=
                    self.sufficient.worker_control[begin + worker] * scaled;
            }
        }
        center_firms_with_interrupt(&mut reduced[..self.firms()], interrupt, "model_rhs_center")?;
        validate_internal_finite_with_interrupt(
            &reduced,
            interrupt,
            "model_rhs_validate_output",
            "reduced model RHS is nonfinite",
        )?;
        Ok(reduced)
    }

    pub fn reconstruct_worker(&self, worker_rhs: &[f64], reduced: &[f64]) -> Result<Vec<f64>> {
        self.reconstruct_worker_with_interrupt(worker_rhs, reduced, &mut NeverInterrupt)
    }

    pub fn reconstruct_worker_with_interrupt(
        &self,
        worker_rhs: &[f64],
        reduced: &[f64],
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<Vec<f64>> {
        if worker_rhs.len() != self.workers() || reduced.len() != self.parameter_count() {
            return Err(BackendError::invalid(
                "model_reconstruct",
                "worker reconstruction has incompatible dimensions",
            ));
        }
        validate_finite_with_interrupt(worker_rhs, interrupt, "model_reconstruct_validate")?;
        validate_finite_with_interrupt(reduced, interrupt, "model_reconstruct_validate")?;
        let firm_mean = firm_mean_with_interrupt(
            &reduced[..self.firms()],
            interrupt,
            "model_reconstruct_firm_mean",
        )?;
        let mut worker = copy_f64_with_interrupt(
            worker_rhs,
            "model reconstructed workers",
            interrupt,
            "model_reconstruct_copy",
        )?;
        // Recover D_worker^-1 (rhs - B reduced) from the same prepared sums.
        // No workspace or retained state is added, and the full W+F+Q check
        // below the solver boundary still traverses original observations.
        for pair in 0..self.sufficient.pair_weight.len() {
            checkpoint_chunk(interrupt, pair, "model_reconstruct")?;
            let worker_index = dense_index(self.sufficient.pair_worker[pair]);
            let firm = dense_index(self.sufficient.pair_firm[pair]);
            worker[worker_index] -= self.sufficient.pair_weight[pair] * (reduced[firm] - firm_mean);
        }
        for control in 0..self.controls() {
            let begin = control * self.workers();
            let coefficient = reduced[self.firms() + control];
            for (index, value) in worker.iter_mut().enumerate() {
                checkpoint_chunk(interrupt, index, "model_reconstruct")?;
                *value -= self.sufficient.worker_control[begin + index] * coefficient;
            }
        }
        for (index, value) in worker.iter_mut().enumerate() {
            checkpoint_chunk(interrupt, index, "model_reconstruct_scale")?;
            *value /= self.worker_diagonal[index];
        }
        validate_internal_finite_with_interrupt(
            &worker,
            interrupt,
            "model_reconstruct_output",
            "worker reconstruction produced a nonfinite value",
        )?;
        Ok(worker)
    }

    /// Fill caller-owned row predictions without allocating an N-vector.
    pub fn predict_into(
        &self,
        worker: &[f64],
        firm: &[f64],
        control: &[f64],
        output: &mut [f64],
    ) -> Result<()> {
        self.predict_into_with_interrupt(worker, firm, control, output, &mut NeverInterrupt)
    }

    pub fn predict_into_with_interrupt(
        &self,
        worker: &[f64],
        firm: &[f64],
        control: &[f64],
        output: &mut [f64],
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<()> {
        if output.len() != self.rows() {
            return Err(BackendError::invalid(
                "model_prediction",
                "prediction arrays have incompatible dimensions",
            ));
        }
        self.validate_prediction_coefficients(worker, firm, control, interrupt)?;
        let chunk_rows = checked_row_chunk(self.rows(), self.controls(), 1)?;
        for (chunk, predictions) in output.chunks_mut(chunk_rows).enumerate() {
            interrupt.checkpoint("model_prediction")?;
            for (offset, prediction) in predictions.iter_mut().enumerate() {
                let row = chunk * chunk_rows + offset;
                *prediction = self.prediction_at(worker, firm, control, row)?;
            }
        }
        interrupt.checkpoint("model_prediction")?;
        Ok(())
    }

    pub(crate) fn validate_prediction_coefficients(
        &self,
        worker: &[f64],
        firm: &[f64],
        control: &[f64],
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<()> {
        if worker.len() != self.workers()
            || firm.len() != self.firms()
            || control.len() != self.controls()
        {
            return Err(BackendError::invalid(
                "model_prediction",
                "prediction arrays have incompatible dimensions",
            ));
        }
        validate_finite_with_interrupt(worker, interrupt, "model_prediction_validate")?;
        validate_finite_with_interrupt(firm, interrupt, "model_prediction_validate")?;
        validate_finite_with_interrupt(control, interrupt, "model_prediction_validate")
    }

    /// Caller validates coefficient dimensions/finiteness once before a
    /// disjoint-row sweep. Arithmetic and the per-row finite gate are shared
    /// with the full prediction method, including active canonical controls.
    #[inline]
    pub(crate) fn prediction_at(
        &self,
        worker: &[f64],
        firm: &[f64],
        control: &[f64],
        row: usize,
    ) -> Result<f64> {
        let mut value = worker[dense_index(self.data.row_worker[row])]
            + firm[dense_index(self.data.row_firm[row])];
        for (coefficient, column) in control.iter().zip(self.data.controls) {
            value += coefficient * column[row];
        }
        if !value.is_finite() {
            return Err(invariant_nonfinite(
                "model_prediction",
                "model prediction is nonfinite",
            ));
        }
        Ok(value)
    }

    pub fn full_residual(
        &self,
        worker: &[f64],
        firm: &[f64],
        control: &[f64],
        rhs: ModelRhs<'_>,
    ) -> Result<ModelResidual> {
        self.full_residual_with_interrupt(worker, firm, control, rhs, &mut NeverInterrupt)
    }

    /// Independently recompute every original worker, firm, and active-control
    /// equation. This path does not reuse the reduced action or predictions.
    pub fn full_residual_with_interrupt(
        &self,
        worker: &[f64],
        firm: &[f64],
        control: &[f64],
        rhs: ModelRhs<'_>,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<ModelResidual> {
        validate_rhs(self, rhs, interrupt)?;
        if worker.len() != self.workers()
            || firm.len() != self.firms()
            || control.len() != self.controls()
        {
            return Err(BackendError::invalid(
                "model_full_residual",
                "model coefficients have incompatible dimensions",
            ));
        }
        validate_finite_with_interrupt(worker, interrupt, "model_residual_validate")?;
        validate_finite_with_interrupt(firm, interrupt, "model_residual_validate")?;
        validate_finite_with_interrupt(control, interrupt, "model_residual_validate")?;
        let mut worker_residual = copy_f64_with_interrupt(
            rhs.worker,
            "model worker residual",
            interrupt,
            "model_residual_copy_worker",
        )?;
        let mut firm_residual = copy_f64_with_interrupt(
            rhs.firm,
            "model firm residual",
            interrupt,
            "model_residual_copy_firm",
        )?;
        let mut control_residual = copy_f64_with_interrupt(
            rhs.control,
            "model control residual",
            interrupt,
            "model_residual_copy_control",
        )?;
        let chunk_rows = checked_row_chunk(self.rows(), self.controls(), 2)?;
        for rows in self.row_order.chunks(chunk_rows) {
            interrupt.checkpoint("model_full_residual")?;
            for &row in rows {
                let worker_index = dense_index(self.data.row_worker[row]);
                let firm_index = dense_index(self.data.row_firm[row]);
                let mut prediction = worker[worker_index] + firm[firm_index];
                for (coefficient, column) in control.iter().zip(self.data.controls) {
                    prediction += coefficient * column[row];
                }
                let weighted = self.data.weight[row] * prediction;
                worker_residual[worker_index] -= weighted;
                firm_residual[firm_index] -= weighted;
                for (residual, column) in control_residual.iter_mut().zip(self.data.controls) {
                    *residual -= column[row] * weighted;
                }
            }
        }
        interrupt.checkpoint("model_full_residual")?;
        let absolute_norm = triple_norm_with_interrupt(
            &worker_residual,
            &firm_residual,
            &control_residual,
            interrupt,
            "model_residual_norm",
        )?;
        let rhs_norm = triple_norm_with_interrupt(
            rhs.worker,
            rhs.firm,
            rhs.control,
            interrupt,
            "model_rhs_norm",
        )?;
        let relative_norm = if rhs_norm == 0.0 {
            absolute_norm
        } else {
            absolute_norm / rhs_norm
        };
        if !absolute_norm.is_finite() || !relative_norm.is_finite() {
            return Err(BackendError::new(
                ErrorCode::FullResidualFailed,
                "model_full_residual",
                "complete W+F+Q residual is nonfinite",
            ));
        }
        Ok(ModelResidual {
            worker: worker_residual,
            firm: firm_residual,
            control: control_residual,
            absolute_norm,
            relative_norm,
            rhs_norm,
        })
    }
}

impl SymmetricOperator for ModelOperator<'_> {
    fn dimension(&self) -> usize {
        self.parameter_count()
    }

    fn apply(&self, input: &[f64], output: &mut [f64]) -> Result<()> {
        let mut workspace = ModelWorkspace::new(self)?;
        self.apply_with_workspace(input, output, &mut workspace)
    }

    fn apply_with_interrupt(
        &self,
        input: &[f64],
        output: &mut [f64],
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<()> {
        let mut workspace = ModelWorkspace::new_with_interrupt(self, interrupt)?;
        self.apply_with_workspace_and_interrupt(input, output, &mut workspace, interrupt)
    }

    fn project(&self, values: &mut [f64]) -> Result<()> {
        self.project_parameters(values)
    }
}

fn validate_shape(data: CanonicalModelData<'_>, interrupt: &mut dyn InterruptCheck) -> Result<()> {
    let rows = data.rows();
    if rows == 0 || data.workers == 0 || data.firms < 2 {
        return Err(BackendError::new(
            ErrorCode::GraphUnidentified,
            "model_operator_setup",
            "generic model requires rows, workers, and at least two firms",
        ));
    }
    if data.row_worker.len() != rows
        || data.row_firm.len() != rows
        || data.controls.iter().any(|column| column.len() != rows)
    {
        return Err(BackendError::invalid(
            "model_operator_setup",
            "generic model columns have incompatible lengths",
        ));
    }
    if data.control_count() > MAX_CANONICAL_CONTROLS {
        return Err(BackendError::new(
            ErrorCode::AmbiguousControlBasis,
            "model_operator_setup",
            "generic model supports at most 32 canonical controls",
        ));
    }
    if data.firms > (1_usize << f64::MANTISSA_DIGITS) {
        return Err(resource_error(
            "firm count exceeds exact integer representation for centering",
        ));
    }
    for row in 0..rows {
        checkpoint_chunk(interrupt, row, "model_operator_validate_rows")?;
        let worker = usize::try_from(data.row_worker[row]).map_err(|_| {
            BackendError::new(
                ErrorCode::InvalidIdentifier,
                "model_operator_setup",
                "worker index is not addressable",
            )
        })?;
        let firm = usize::try_from(data.row_firm[row]).map_err(|_| {
            BackendError::new(
                ErrorCode::InvalidIdentifier,
                "model_operator_setup",
                "firm index is not addressable",
            )
        })?;
        if worker >= data.workers || firm >= data.firms {
            return Err(BackendError::new(
                ErrorCode::InvalidIdentifier,
                "model_operator_setup",
                "worker or firm index exceeds the declared dense dimension",
            ));
        }
        if !data.weight[row].is_finite() || data.weight[row] <= 0.0 {
            return Err(BackendError::new(
                ErrorCode::InvalidWeight,
                "model_operator_setup",
                "model information weight must be positive and finite",
            ));
        }
        if data.controls.iter().any(|column| !column[row].is_finite()) {
            return Err(BackendError::invalid(
                "model_operator_setup",
                "canonical control value is nonfinite",
            ));
        }
    }
    Ok(())
}

fn canonical_row_order(
    data: CanonicalModelData<'_>,
    interrupt: &mut dyn InterruptCheck,
) -> Result<Vec<usize>> {
    let mut order = Vec::new();
    reserve_exact(&mut order, data.rows(), "model canonical row order")?;
    for row in 0..data.rows() {
        checkpoint_chunk(interrupt, row, "model_operator_row_order")?;
        order.push(row);
    }
    stable_sort_by_with_interrupt(
        &mut order,
        |&left, &right| {
            data.row_worker[left]
                .cmp(&data.row_worker[right])
                .then_with(|| data.row_firm[left].cmp(&data.row_firm[right]))
                .then_with(|| data.weight[left].total_cmp(&data.weight[right]))
                .then_with(|| {
                    data.controls
                        .iter()
                        .map(|column| column[left].total_cmp(&column[right]))
                        .find(|ordering| !ordering.is_eq())
                        .unwrap_or_else(|| left.cmp(&right))
                })
        },
        interrupt,
        "model_operator_row_order",
    )?;
    Ok(order)
}

fn information_diagonals(
    data: CanonicalModelData<'_>,
    row_order: &[usize],
    interrupt: &mut dyn InterruptCheck,
) -> Result<(Vec<f64>, Vec<f64>)> {
    let mut worker = zeroed_f64_with_interrupt(
        data.workers,
        "model worker diagonal",
        interrupt,
        "model_operator_initialize_diagonal",
    )?;
    let mut firm = zeroed_f64_with_interrupt(
        data.firms,
        "model firm diagonal",
        interrupt,
        "model_operator_initialize_diagonal",
    )?;
    for (position, &row) in row_order.iter().enumerate() {
        checkpoint_chunk(interrupt, position, "model_operator_diagonal")?;
        worker[dense_index(data.row_worker[row])] += data.weight[row];
        firm[dense_index(data.row_firm[row])] += data.weight[row];
    }
    for (index, &value) in worker.iter().chain(&firm).enumerate() {
        checkpoint_chunk(interrupt, index, "model_operator_diagonal")?;
        if !value.is_finite() || value <= 0.0 {
            return Err(BackendError::new(
                ErrorCode::GraphUnidentified,
                "model_operator_setup",
                "worker or firm information diagonal is not positive and finite",
            ));
        }
    }
    Ok((worker, firm))
}

fn reduced_diagonal(
    data: CanonicalModelData<'_>,
    row_order: &[usize],
    worker_diagonal: &[f64],
    firm_diagonal: &[f64],
    interrupt: &mut dyn InterruptCheck,
) -> Result<Vec<f64>> {
    let dimension = data
        .firms
        .checked_add(data.control_count())
        .ok_or_else(|| resource_error("model reduced diagonal dimension overflow"))?;
    let mut diagonal = zeroed_f64_with_interrupt(
        dimension,
        "model reduced diagonal",
        interrupt,
        "model_operator_initialize_reduced_diagonal",
    )?;
    copy_into_with_interrupt(
        firm_diagonal,
        &mut diagonal[..data.firms],
        interrupt,
        "model_operator_copy_firm_diagonal",
    )?;

    let mut begin = 0_usize;
    while begin < row_order.len() {
        checkpoint_chunk(interrupt, begin, "model_operator_firm_diagonal")?;
        let first = row_order[begin];
        let worker = dense_index(data.row_worker[first]);
        let firm = dense_index(data.row_firm[first]);
        let mut end = begin + 1;
        let mut pair_weight = data.weight[first];
        while end < row_order.len()
            && data.row_worker[row_order[end]] == data.row_worker[first]
            && data.row_firm[row_order[end]] == data.row_firm[first]
        {
            checkpoint_chunk(interrupt, end - begin, "model_operator_firm_diagonal")?;
            pair_weight += data.weight[row_order[end]];
            end += 1;
        }
        diagonal[firm] -= pair_weight * pair_weight / worker_diagonal[worker];
        begin = end;
    }

    let mut worker_sum = zeroed_f64_with_interrupt(
        data.workers,
        "model control diagonal workspace",
        interrupt,
        "model_operator_initialize_control_diagonal",
    )?;
    for (control, column) in data.controls.iter().enumerate() {
        interrupt.checkpoint("model_operator_control_diagonal")?;
        fill_f64_with_interrupt(
            &mut worker_sum,
            0.0,
            interrupt,
            "model_operator_zero_control_workspace",
        )?;
        let mut direct = 0.0;
        let mut direct_correction = 0.0;
        for (position, &row) in row_order.iter().enumerate() {
            checkpoint_chunk(interrupt, position, "model_operator_control_diagonal")?;
            let weighted = data.weight[row] * column[row];
            worker_sum[dense_index(data.row_worker[row])] += weighted;
            let square = weighted * column[row];
            let adjusted = square - direct_correction;
            let next = direct + adjusted;
            direct_correction = (next - direct) - adjusted;
            direct = next;
        }
        let mut absorbed = 0.0;
        let mut absorbed_correction = 0.0;
        for (worker, &sum) in worker_sum.iter().enumerate() {
            checkpoint_chunk(interrupt, worker, "model_operator_control_diagonal")?;
            let term = sum * sum / worker_diagonal[worker];
            let adjusted = term - absorbed_correction;
            let next = absorbed + adjusted;
            absorbed_correction = (next - absorbed) - adjusted;
            absorbed = next;
        }
        diagonal[data.firms + control] = direct - absorbed;
    }
    for (index, &value) in diagonal.iter().enumerate() {
        checkpoint_chunk(interrupt, index, "model_operator_reduced_diagonal")?;
        if !value.is_finite() || value <= 0.0 {
            return Err(BackendError::new(
                ErrorCode::SingularInformation,
                "model_operator_setup",
                "exact reduced model diagonal is not positive and finite",
            ));
        }
    }
    Ok(diagonal)
}

fn require_connected(
    data: CanonicalModelData<'_>,
    row_order: &[usize],
    interrupt: &mut dyn InterruptCheck,
) -> Result<()> {
    let vertices = data
        .workers
        .checked_add(data.firms)
        .ok_or_else(|| resource_error("model graph vertex count overflow"))?;
    let mut parent = Vec::new();
    reserve_exact(&mut parent, vertices, "model graph parent")?;
    for vertex in 0..vertices {
        checkpoint_chunk(interrupt, vertex, "model_operator_initialize_connectivity")?;
        parent.push(vertex);
    }
    let mut rank = zeroed_u8_with_interrupt(
        vertices,
        "model graph rank",
        interrupt,
        "model_operator_initialize_connectivity",
    )?;
    for (position, &row) in row_order.iter().enumerate() {
        checkpoint_chunk(interrupt, position, "model_operator_connectivity")?;
        let left = dense_index(data.row_worker[row]);
        let right = data.workers + dense_index(data.row_firm[row]);
        union(&mut parent, &mut rank, left, right);
    }
    let root = find(&mut parent, 0);
    for vertex in 1..vertices {
        checkpoint_chunk(interrupt, vertex, "model_operator_connectivity")?;
        if find(&mut parent, vertex) != root {
            return Err(BackendError::new(
                ErrorCode::GraphUnidentified,
                "model_operator_setup",
                "worker--firm graph is disconnected",
            ));
        }
    }
    Ok(())
}

fn find(parent: &mut [usize], mut value: usize) -> usize {
    let mut root = value;
    while parent[root] != root {
        root = parent[root];
    }
    while parent[value] != value {
        let next = parent[value];
        parent[value] = root;
        value = next;
    }
    root
}

fn union(parent: &mut [usize], rank: &mut [u8], left: usize, right: usize) {
    let left = find(parent, left);
    let right = find(parent, right);
    if left == right {
        return;
    }
    if rank[left] < rank[right] {
        parent[left] = right;
    } else {
        parent[right] = left;
        if rank[left] == rank[right] {
            rank[left] = rank[left].saturating_add(1);
        }
    }
}

fn validate_parameter_vectors(
    operator: &ModelOperator<'_>,
    input: &[f64],
    output: &[f64],
    interrupt: &mut dyn InterruptCheck,
    phase: &'static str,
) -> Result<()> {
    if input.len() != operator.parameter_count() || output.len() != operator.parameter_count() {
        return Err(BackendError::invalid(
            phase,
            "model Schur action has incompatible dimensions",
        ));
    }
    validate_finite_with_interrupt(input, interrupt, phase)
}

fn validate_rhs(
    operator: &ModelOperator<'_>,
    rhs: ModelRhs<'_>,
    interrupt: &mut dyn InterruptCheck,
) -> Result<()> {
    if rhs.worker.len() != operator.workers()
        || rhs.firm.len() != operator.firms()
        || rhs.control.len() != operator.controls()
    {
        return Err(BackendError::invalid(
            "model_rhs",
            "original model RHS has incompatible dimensions",
        ));
    }
    validate_finite_with_interrupt(rhs.worker, interrupt, "model_rhs_validate")?;
    validate_finite_with_interrupt(rhs.firm, interrupt, "model_rhs_validate")?;
    validate_finite_with_interrupt(rhs.control, interrupt, "model_rhs_validate")?;
    Ok(())
}

fn require_compatible_rhs(rhs: ModelRhs<'_>, interrupt: &mut dyn InterruptCheck) -> Result<()> {
    let worker_sum =
        compensated_sum_with_interrupt(rhs.worker, interrupt, "model_rhs_compatibility")?;
    let firm_sum = compensated_sum_with_interrupt(rhs.firm, interrupt, "model_rhs_compatibility")?;
    let mut scale = 0.0;
    for (index, value) in rhs.worker.iter().chain(rhs.firm).enumerate() {
        checkpoint_chunk(interrupt, index, "model_rhs_compatibility")?;
        scale += value.abs();
    }
    scale = scale.max(1.0);
    let tolerance = RHS_COMPATIBILITY_FACTOR * f64::EPSILON * scale;
    if !worker_sum.is_finite() || !firm_sum.is_finite() || (worker_sum - firm_sum).abs() > tolerance
    {
        return Err(BackendError::invalid(
            "model_rhs",
            "original W+F+Q RHS is incompatible with the worker--firm null direction",
        ));
    }
    Ok(())
}

pub(crate) fn center_firms_with_interrupt(
    values: &mut [f64],
    interrupt: &mut dyn InterruptCheck,
    phase: &'static str,
) -> Result<()> {
    let mean = firm_mean_with_interrupt(values, interrupt, phase)?;
    for (index, value) in values.iter_mut().enumerate() {
        checkpoint_chunk(interrupt, index, phase)?;
        *value -= mean;
    }
    Ok(())
}

pub(crate) fn firm_mean_with_interrupt(
    values: &[f64],
    interrupt: &mut dyn InterruptCheck,
    phase: &'static str,
) -> Result<f64> {
    if values.is_empty() {
        return Err(BackendError::invalid(
            "model_operator",
            "cannot center empty firm coordinates",
        ));
    }
    validate_finite_with_interrupt(values, interrupt, phase)?;
    let count = values.len() as f64;
    if !count.is_finite() || count as usize != values.len() {
        return Err(resource_error(
            "firm count is not represented exactly during centering",
        ));
    }
    Ok(compensated_sum_with_interrupt(values, interrupt, phase)? / count)
}

fn triple_norm_with_interrupt(
    first: &[f64],
    second: &[f64],
    third: &[f64],
    interrupt: &mut dyn InterruptCheck,
    phase: &'static str,
) -> Result<f64> {
    Ok((stable_dot_with_interrupt(first, first, interrupt, phase)?
        + stable_dot_with_interrupt(second, second, interrupt, phase)?
        + stable_dot_with_interrupt(third, third, interrupt, phase)?)
    .max(0.0)
    .sqrt())
}

fn dense_index(value: u32) -> usize {
    usize::try_from(value).expect("validated dense model index")
}

pub(crate) fn checked_matrix_length(rows: usize, columns: usize, label: &str) -> Result<usize> {
    rows.checked_mul(columns)
        .ok_or_else(|| resource_error(&format!("{label} length overflow for {rows} by {columns}")))
}

pub(crate) fn zeroed_f64_with_interrupt(
    length: usize,
    label: &str,
    interrupt: &mut dyn InterruptCheck,
    phase: &'static str,
) -> Result<Vec<f64>> {
    let _bytes = length
        .checked_mul(core::mem::size_of::<f64>())
        .ok_or_else(|| resource_error(&format!("{label} byte-size overflow")))?;
    let mut values = Vec::new();
    reserve_exact(&mut values, length, label)?;
    for index in 0..length {
        checkpoint_chunk(interrupt, index, phase)?;
        values.push(0.0);
    }
    Ok(values)
}

fn zeroed_u8_with_interrupt(
    length: usize,
    label: &str,
    interrupt: &mut dyn InterruptCheck,
    phase: &'static str,
) -> Result<Vec<u8>> {
    let mut values = Vec::new();
    reserve_exact(&mut values, length, label)?;
    for index in 0..length {
        checkpoint_chunk(interrupt, index, phase)?;
        values.push(0);
    }
    Ok(values)
}

pub(crate) fn copy_f64_with_interrupt(
    values: &[f64],
    label: &str,
    interrupt: &mut dyn InterruptCheck,
    phase: &'static str,
) -> Result<Vec<f64>> {
    let mut copy = Vec::new();
    reserve_exact(&mut copy, values.len(), label)?;
    for (index, &value) in values.iter().enumerate() {
        checkpoint_chunk(interrupt, index, phase)?;
        copy.push(value);
    }
    Ok(copy)
}

pub(crate) fn copy_into_with_interrupt(
    source: &[f64],
    destination: &mut [f64],
    interrupt: &mut dyn InterruptCheck,
    phase: &'static str,
) -> Result<()> {
    if source.len() != destination.len() {
        return Err(BackendError::invariant(
            phase,
            "interruptible copy dimensions disagree",
        ));
    }
    for (index, (output, &input)) in destination.iter_mut().zip(source).enumerate() {
        checkpoint_chunk(interrupt, index, phase)?;
        *output = input;
    }
    Ok(())
}

pub(crate) fn fill_f64_with_interrupt(
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

pub(crate) fn validate_finite_with_interrupt(
    values: &[f64],
    interrupt: &mut dyn InterruptCheck,
    phase: &'static str,
) -> Result<()> {
    for (index, value) in values.iter().enumerate() {
        checkpoint_chunk(interrupt, index, phase)?;
        if !value.is_finite() {
            return Err(BackendError::invalid(phase, "numeric vector is nonfinite"));
        }
    }
    Ok(())
}

fn validate_internal_finite_with_interrupt(
    values: &[f64],
    interrupt: &mut dyn InterruptCheck,
    phase: &'static str,
    message: &'static str,
) -> Result<()> {
    for (index, value) in values.iter().enumerate() {
        checkpoint_chunk(interrupt, index, phase)?;
        if !value.is_finite() {
            return Err(invariant_nonfinite(phase, message));
        }
    }
    Ok(())
}

pub(crate) fn compensated_sum_with_interrupt(
    values: &[f64],
    interrupt: &mut dyn InterruptCheck,
    phase: &'static str,
) -> Result<f64> {
    let mut sum = 0.0;
    let mut correction = 0.0;
    for (index, &value) in values.iter().enumerate() {
        checkpoint_chunk(interrupt, index, phase)?;
        let adjusted = value - correction;
        let next = sum + adjusted;
        correction = (next - sum) - adjusted;
        sum = next;
    }
    Ok(sum)
}

pub(crate) fn stable_dot_with_interrupt(
    left: &[f64],
    right: &[f64],
    interrupt: &mut dyn InterruptCheck,
    phase: &'static str,
) -> Result<f64> {
    if left.len() != right.len() {
        return Err(BackendError::invariant(
            phase,
            "stable dot-product dimensions disagree",
        ));
    }
    let mut sum = 0.0;
    let mut correction = 0.0;
    for (index, (&left_value, &right_value)) in left.iter().zip(right).enumerate() {
        checkpoint_chunk(interrupt, index, phase)?;
        let product = left_value * right_value;
        let adjusted = product - correction;
        let next = sum + adjusted;
        correction = (next - sum) - adjusted;
        sum = next;
    }
    Ok(sum)
}

pub(crate) fn reserve_exact<T>(values: &mut Vec<T>, length: usize, label: &str) -> Result<()> {
    let _bytes = length
        .checked_mul(core::mem::size_of::<T>())
        .ok_or_else(|| resource_error(&format!("{label} byte-size overflow")))?;
    values.try_reserve_exact(length).map_err(|_| {
        BackendError::new(
            ErrorCode::AllocationFailed,
            "model_allocation",
            format!("could not allocate {label} with {length} elements"),
        )
    })
}

/// Bound the same row-plus-control work previously checked in inner loops.
/// Canonical controls are capped at 32, so a complete row always fits. The
/// row and control arithmetic order is unchanged; no residual is reused.
fn checked_row_chunk(rows: usize, controls: usize, passes: usize) -> Result<usize> {
    let work_per_row = controls
        .checked_mul(passes)
        .and_then(|work| work.checked_add(1))
        .ok_or_else(|| resource_error("model row work overflow"))?;
    let _work = checked_matrix_length(rows, work_per_row, "model row work")?;
    let chunk = INTERRUPT_CHECK_CHUNK / work_per_row;
    if chunk == 0 {
        return Err(resource_error("model row exceeds interruption work bound"));
    }
    Ok(chunk)
}

fn invariant_nonfinite(phase: &'static str, message: &'static str) -> BackendError {
    BackendError::new(ErrorCode::InternalInvariantFailed, phase, message)
}

fn resource_error(message: &str) -> BackendError {
    BackendError::new(ErrorCode::ResourceLimit, "model_resource", message)
}

#[cfg(test)]
mod row_chunk_tests {
    use super::*;

    #[test]
    fn row_chunk_bounds_and_overflow_are_checked() {
        for controls in 0..=MAX_CANONICAL_CONTROLS {
            for passes in [1, 2] {
                let width = checked_row_chunk(8_001, controls, passes).unwrap();
                let work = 1 + controls * passes;
                assert!(width > 0);
                assert!(width * work <= INTERRUPT_CHECK_CHUNK);
                assert!((width + 1) * work > INTERRUPT_CHECK_CHUNK);
                assert_eq!(checked_row_chunk(0, controls, passes).unwrap(), width);
            }
        }
        for (rows, controls, passes) in [
            (usize::MAX, 1, 1),
            (1, usize::MAX, 2),
            (1, INTERRUPT_CHECK_CHUNK, 1),
        ] {
            assert_eq!(
                checked_row_chunk(rows, controls, passes).unwrap_err().code,
                ErrorCode::ResourceLimit
            );
        }
    }
}
