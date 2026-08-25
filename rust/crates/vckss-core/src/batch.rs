// SPDX-License-Identifier: GPL-3.0-only

//! Batched scalar PCG for repeated two-way fixed-effect right-hand sides.
//!
//! Each logical right-hand side retains an independent scalar recurrence,
//! convergence decision, residual-replacement count, and receipt. The Schur
//! operator traverses coefficient cells once per active batch while preserving
//! the scalar summation order within every column.

use crate::error::{BackendError, ErrorCode, Result};
use crate::interrupt::{InterruptCheck, NeverInterrupt, INTERRUPT_CHECK_CHUNK};
use crate::krylov::{PcgOptions, PcgReceipt, Preconditioner};
use crate::operator::{stable_dot, stable_norm, SymmetricOperator, TwoWayOperator, TwoWaySolution};

#[derive(Clone, Debug)]
pub struct BatchedPcgSolve {
    pub dimension: usize,
    pub columns: usize,
    pub solution: Vec<f64>,
    pub receipt: Vec<PcgReceipt>,
}

impl BatchedPcgSolve {
    #[must_use]
    pub fn column(&self, column: usize) -> &[f64] {
        let begin = column * self.dimension;
        &self.solution[begin..begin + self.dimension]
    }
}

#[derive(Clone, Debug)]
pub struct TwoWayBatchedPcgSolve {
    pub columns: usize,
    pub solution: Vec<TwoWaySolution>,
    pub pcg: Vec<PcgReceipt>,
}

#[derive(Clone, Debug)]
pub struct TwoWayBatchWorkspace {
    // Entity-major layout keeps every logical column for one firm/worker
    // contiguous while the cell traversal preserves scalar accumulation order.
    full_firm: Vec<f64>,
    worker_sum: Vec<f64>,
    full_output: Vec<f64>,
}

impl TwoWayBatchWorkspace {
    pub fn new(operator: &TwoWayOperator<'_>, columns: usize) -> Result<Self> {
        let firms = operator.problem().firms();
        let workers = operator.problem().workers();
        Ok(Self {
            full_firm: zero_matrix(firms, columns, "full firm workspace")?,
            worker_sum: zero_matrix(workers, columns, "worker workspace")?,
            full_output: zero_matrix(firms, columns, "full output workspace")?,
        })
    }
}

pub fn apply_two_way_schur_batch(
    operator: &TwoWayOperator<'_>,
    input: &[f64],
    output: &mut [f64],
    columns: usize,
    workspace: &mut TwoWayBatchWorkspace,
) -> Result<()> {
    let mut interrupt = NeverInterrupt;
    apply_two_way_schur_batch_with_interrupt(
        operator,
        input,
        output,
        columns,
        workspace,
        &mut interrupt,
    )
}

pub fn apply_two_way_schur_batch_with_interrupt(
    operator: &TwoWayOperator<'_>,
    input: &[f64],
    output: &mut [f64],
    columns: usize,
    workspace: &mut TwoWayBatchWorkspace,
    interrupt: &mut dyn InterruptCheck,
) -> Result<()> {
    let dimension = operator.dimension();
    let expected = checked_matrix_length(dimension, columns, "Schur batch")?;
    if columns == 0 || input.len() != expected || output.len() != expected {
        return Err(BackendError::invalid(
            "batch_operator",
            "batched Schur action has incompatible dimensions",
        ));
    }
    for chunk in input.chunks(INTERRUPT_CHECK_CHUNK) {
        interrupt.checkpoint("batch_operator_input")?;
        for value in chunk {
            if !value.is_finite() {
                return Err(BackendError::invalid(
                    "batch_operator",
                    "batched Schur input is nonfinite",
                ));
            }
        }
    }

    let problem = operator.problem();
    let firms = problem.firms();
    let workers = problem.workers();
    let required_firm = checked_matrix_length(firms, columns, "full firm workspace")?;
    let required_worker = checked_matrix_length(workers, columns, "worker workspace")?;
    if workspace.full_firm.len() != required_firm
        || workspace.full_output.len() != required_firm
        || workspace.worker_sum.len() != required_worker
    {
        return Err(BackendError::invalid(
            "batch_operator",
            "batched Schur workspace has incompatible dimensions",
        ));
    }

    workspace.full_firm.fill(0.0);
    workspace.worker_sum.fill(0.0);
    workspace.full_output.fill(0.0);
    for column in 0..columns {
        interrupt.checkpoint("batch_operator_project")?;
        let reduced_begin = column * dimension;
        output[reduced_begin..reduced_begin + dimension]
            .copy_from_slice(&input[reduced_begin..reduced_begin + dimension]);
        operator.project(&mut output[reduced_begin..reduced_begin + dimension])?;
        for firm in 0..firms {
            workspace.full_firm[firm * columns + column] = output[reduced_begin + firm];
        }
    }

    let flattened_cells = checked_matrix_length(problem.cells(), columns, "cell-column work")?;
    let mut chunk_begin = 0_usize;
    while chunk_begin < flattened_cells {
        interrupt.checkpoint("batch_operator_cells_accumulate")?;
        let chunk_end = chunk_begin
            .saturating_add(INTERRUPT_CHECK_CHUNK)
            .min(flattened_cells);
        let mut cursor = chunk_begin;
        while cursor < chunk_end {
            let cell = cursor / columns;
            let first_column = cursor - cell * columns;
            let last_column = columns.min(first_column + chunk_end - cursor);
            let worker = usize::try_from(problem.cell_worker[cell]).expect("validated worker");
            let firm = usize::try_from(problem.cell_firm[cell]).expect("validated firm");
            let weight = problem.cell_weight[cell];
            for column in first_column..last_column {
                workspace.worker_sum[worker * columns + column] +=
                    weight * workspace.full_firm[firm * columns + column];
            }
            cursor += last_column - first_column;
        }
        chunk_begin = chunk_end;
    }
    let mut scale_work = 0_usize;
    let mut next_checkpoint = 0_usize;
    for column in 0..columns {
        for (worker, &diagonal) in operator.worker_diagonal().iter().enumerate() {
            if scale_work == next_checkpoint {
                interrupt.checkpoint("batch_operator_scale")?;
                next_checkpoint = next_checkpoint
                    .checked_add(INTERRUPT_CHECK_CHUNK)
                    .unwrap_or(usize::MAX);
            }
            workspace.worker_sum[worker * columns + column] /= diagonal;
            scale_work += 1;
        }
    }
    debug_assert_eq!(scale_work, required_worker);
    let mut diagonal_work = 0_usize;
    let mut next_checkpoint = 0_usize;
    for column in 0..columns {
        for (firm, &diagonal) in operator.firm_diagonal().iter().enumerate() {
            if diagonal_work == next_checkpoint {
                interrupt.checkpoint("batch_operator_diagonal")?;
                next_checkpoint = next_checkpoint
                    .checked_add(INTERRUPT_CHECK_CHUNK)
                    .unwrap_or(usize::MAX);
            }
            let offset = firm * columns + column;
            workspace.full_output[offset] = diagonal * workspace.full_firm[offset];
            diagonal_work += 1;
        }
    }
    debug_assert_eq!(diagonal_work, required_firm);
    chunk_begin = 0;
    while chunk_begin < flattened_cells {
        interrupt.checkpoint("batch_operator_cells_scatter")?;
        let chunk_end = chunk_begin
            .saturating_add(INTERRUPT_CHECK_CHUNK)
            .min(flattened_cells);
        let mut cursor = chunk_begin;
        while cursor < chunk_end {
            let cell = cursor / columns;
            let first_column = cursor - cell * columns;
            let last_column = columns.min(first_column + chunk_end - cursor);
            let worker = usize::try_from(problem.cell_worker[cell]).expect("validated worker");
            let firm = usize::try_from(problem.cell_firm[cell]).expect("validated firm");
            let weight = problem.cell_weight[cell];
            for column in first_column..last_column {
                workspace.full_output[firm * columns + column] -=
                    weight * workspace.worker_sum[worker * columns + column];
            }
            cursor += last_column - first_column;
        }
        chunk_begin = chunk_end;
    }
    for column in 0..columns {
        let reduced_begin = column * dimension;
        for firm in 0..firms {
            output[reduced_begin + firm] = workspace.full_output[firm * columns + column];
        }
        operator.project(&mut output[reduced_begin..reduced_begin + dimension])?;
    }
    if output.iter().any(|value| !value.is_finite()) {
        return Err(BackendError::new(
            ErrorCode::InternalInvariantFailed,
            "batch_operator",
            "batched Schur action produced a nonfinite value",
        ));
    }
    Ok(())
}

#[allow(clippy::too_many_lines)]
pub fn batched_pcg(
    operator: &TwoWayOperator<'_>,
    preconditioner: &impl Preconditioner,
    right_hand_side: &[f64],
    columns: usize,
    options: PcgOptions,
) -> Result<BatchedPcgSolve> {
    let mut interrupt = NeverInterrupt;
    batched_pcg_with_interrupt(
        operator,
        preconditioner,
        right_hand_side,
        columns,
        options,
        &mut interrupt,
    )
}

#[allow(clippy::too_many_lines)]
pub fn batched_pcg_with_interrupt(
    operator: &TwoWayOperator<'_>,
    preconditioner: &impl Preconditioner,
    right_hand_side: &[f64],
    columns: usize,
    options: PcgOptions,
    interrupt: &mut dyn InterruptCheck,
) -> Result<BatchedPcgSolve> {
    let options = options.validate()?;
    let dimension = operator.dimension();
    let expected = checked_matrix_length(dimension, columns, "PCG batch")?;
    if columns == 0 || right_hand_side.len() != expected || preconditioner.dimension() != dimension
    {
        return Err(BackendError::invalid(
            "batch_pcg",
            "operator, preconditioner, and batched right-hand side dimensions are incompatible",
        ));
    }
    if right_hand_side.iter().any(|value| !value.is_finite()) {
        return Err(BackendError::invalid(
            "batch_pcg",
            "batched right-hand side is nonfinite",
        ));
    }

    let mut projected_rhs = right_hand_side.to_vec();
    for column in 0..columns {
        operator.project(&mut projected_rhs[column_range(column, dimension)])?;
    }
    let mut operator_workspace = TwoWayBatchWorkspace::new(operator, columns)?;
    let mut solution = vec![0.0; expected];
    let mut residual = projected_rhs.clone();
    let mut preconditioned = vec![0.0; expected];
    let mut direction = vec![0.0; expected];
    let mut action = vec![0.0; expected];
    let mut verified_action = vec![0.0; expected];
    let mut active = vec![true; columns];
    let mut restarted = vec![false; columns];
    let mut rhs_norm = vec![0.0; columns];
    let mut residual_product = vec![0.0; columns];
    let mut relative_residual = vec![0.0; columns];
    let mut operator_applications = vec![0_u32; columns];
    let mut preconditioner_applications = vec![0_u32; columns];
    let mut residual_replacements = vec![0_u32; columns];
    let mut receipt = vec![None::<PcgReceipt>; columns];
    let mut candidates = vec![false; columns];

    for column in 0..columns {
        let range = column_range(column, dimension);
        rhs_norm[column] = stable_norm(&projected_rhs[range.clone()]);
        if rhs_norm[column] == 0.0 {
            active[column] = false;
            receipt[column] = Some(zero_receipt());
        }
    }
    if active.iter().all(|&value| !value) {
        return Ok(BatchedPcgSolve {
            dimension,
            columns,
            solution,
            receipt: collect_receipts(receipt)?,
        });
    }

    apply_preconditioner_columns(
        operator,
        preconditioner,
        &residual,
        &mut preconditioned,
        &active,
        &mut preconditioner_applications,
        interrupt,
    )?;
    direction.copy_from_slice(&preconditioned);
    for column in 0..columns {
        if !active[column] {
            continue;
        }
        let range = column_range(column, dimension);
        residual_product[column] = stable_dot(&residual[range.clone()], &preconditioned[range]);
        require_positive_finite(
            residual_product[column],
            ErrorCode::PcgPreconditionerBreakdown,
            column,
            "initial preconditioned residual has nonpositive curvature",
        )?;
    }

    for iteration in 1..=options.maximum_iterations {
        interrupt.checkpoint("batch_pcg_iteration")?;
        apply_two_way_schur_batch_with_interrupt(
            operator,
            &direction,
            &mut action,
            columns,
            &mut operator_workspace,
            interrupt,
        )?;
        increment_active(&mut operator_applications, &active)?;
        for column in 0..columns {
            if !active[column] {
                continue;
            }
            let range = column_range(column, dimension);
            let curvature = stable_dot(&direction[range.clone()], &action[range.clone()]);
            require_positive_finite(
                curvature,
                ErrorCode::PcgCurvatureBreakdown,
                column,
                "search direction has nonpositive operator curvature",
            )?;
            let alpha = residual_product[column] / curvature;
            if !alpha.is_finite() {
                return Err(column_error(
                    ErrorCode::PcgCurvatureBreakdown,
                    column,
                    "PCG step length is nonfinite",
                ));
            }
            let mut begin = range.start;
            while begin < range.end {
                if begin % INTERRUPT_CHECK_CHUNK == 0 {
                    interrupt.checkpoint("batch_pcg_recurrence")?;
                }
                let next_boundary = begin
                    .saturating_add(INTERRUPT_CHECK_CHUNK - begin % INTERRUPT_CHECK_CHUNK)
                    .min(range.end);
                for index in begin..next_boundary {
                    solution[index] += alpha * direction[index];
                    residual[index] -= alpha * action[index];
                }
                begin = next_boundary;
            }
            operator.project(&mut residual[column_range(column, dimension)])?;
        }
        if solution
            .iter()
            .chain(&residual)
            .any(|value| !value.is_finite())
        {
            return Err(BackendError::new(
                ErrorCode::PcgStagnation,
                "batch_pcg",
                "batched PCG recurrence produced a nonfinite value",
            ));
        }

        restarted.fill(false);
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
                &mut operator_workspace,
                interrupt,
            )?;
            for column in 0..columns {
                if !active[column] {
                    continue;
                }
                let range = column_range(column, dimension);
                let explicit = &verified_action[range.clone()];
                let recurrence = &residual[range.clone()];
                let explicit_norm = stable_norm(explicit);
                let drift_norm = stable_difference_norm(explicit, recurrence);
                if !explicit_norm.is_finite() || !drift_norm.is_finite() {
                    return Err(column_error(
                        ErrorCode::PcgStagnation,
                        column,
                        "explicit PCG residual or recurrence drift is nonfinite",
                    ));
                }
                let drift_gate =
                    (1.0e-14 * rhs_norm[column]).max(0.1 * options.tolerance * rhs_norm[column]);
                if explicit_norm <= options.tolerance * rhs_norm[column] || drift_norm > drift_gate
                {
                    residual[range].copy_from_slice(explicit);
                    restarted[column] = true;
                }
            }
        }

        candidates.fill(false);
        for column in 0..columns {
            if !active[column] {
                continue;
            }
            let range = column_range(column, dimension);
            relative_residual[column] = stable_norm(&residual[range]) / rhs_norm[column];
            if !relative_residual[column].is_finite() {
                return Err(column_error(
                    ErrorCode::PcgStagnation,
                    column,
                    "PCG relative residual is nonfinite",
                ));
            }
            candidates[column] = relative_residual[column] <= options.tolerance;
        }
        if candidates.iter().any(|&value| value) {
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
                    &mut operator_workspace,
                    interrupt,
                )?;
            }
            for column in 0..columns {
                if !candidates[column] {
                    continue;
                }
                let range = column_range(column, dimension);
                let verified = stable_norm(&verified_action[range.clone()]) / rhs_norm[column];
                if !verified.is_finite() {
                    return Err(column_error(
                        ErrorCode::PcgStagnation,
                        column,
                        "verified PCG residual is nonfinite",
                    ));
                }
                relative_residual[column] = verified;
                if verified <= options.tolerance {
                    active[column] = false;
                    direction[range].fill(0.0);
                    receipt[column] = Some(PcgReceipt {
                        iterations: iteration,
                        relative_residual: verified,
                        residual_replacements: residual_replacements[column],
                        operator_applications: operator_applications[column],
                        preconditioner_applications: preconditioner_applications[column],
                        zero_rhs: false,
                    });
                } else {
                    residual[range.clone()].copy_from_slice(&verified_action[range]);
                    restarted[column] = true;
                }
            }
        }
        if active.iter().all(|&value| !value) {
            for column in 0..columns {
                operator.project(&mut solution[column_range(column, dimension)])?;
            }
            return Ok(BatchedPcgSolve {
                dimension,
                columns,
                solution,
                receipt: collect_receipts(receipt)?,
            });
        }

        apply_preconditioner_columns(
            operator,
            preconditioner,
            &residual,
            &mut preconditioned,
            &active,
            &mut preconditioner_applications,
            interrupt,
        )?;
        for column in 0..columns {
            if !active[column] {
                continue;
            }
            let range = column_range(column, dimension);
            let next_product = stable_dot(&residual[range.clone()], &preconditioned[range.clone()]);
            require_positive_finite(
                next_product,
                ErrorCode::PcgPreconditionerBreakdown,
                column,
                "preconditioned residual has nonpositive curvature",
            )?;
            if restarted[column] {
                direction[range.clone()].copy_from_slice(&preconditioned[range]);
            } else {
                let beta = next_product / residual_product[column];
                if !beta.is_finite() || beta < 0.0 {
                    return Err(column_error(
                        ErrorCode::PcgPreconditionerBreakdown,
                        column,
                        "PCG direction coefficient is invalid",
                    ));
                }
                let mut begin = range.start;
                while begin < range.end {
                    if begin % INTERRUPT_CHECK_CHUNK == 0 {
                        interrupt.checkpoint("batch_pcg_direction")?;
                    }
                    let next_boundary = begin
                        .saturating_add(INTERRUPT_CHECK_CHUNK - begin % INTERRUPT_CHECK_CHUNK)
                        .min(range.end);
                    for index in begin..next_boundary {
                        direction[index] = preconditioned[index] + beta * direction[index];
                    }
                    begin = next_boundary;
                }
            }
            residual_product[column] = next_product;
        }
    }

    let first = active
        .iter()
        .position(|&value| value)
        .expect("at least one unconverged RHS");
    Err(column_error(
        ErrorCode::PcgMaxIterations,
        first,
        &format!(
            "PCG did not converge in {} iterations; final relative residual was {}",
            options.maximum_iterations, relative_residual[first]
        ),
    ))
}

pub fn solve_two_way_pcg_batch(
    operator: &TwoWayOperator<'_>,
    preconditioner: &impl Preconditioner,
    worker_rhs: &[f64],
    firm_rhs: &[f64],
    columns: usize,
    options: PcgOptions,
    full_residual_tolerance: f64,
) -> Result<TwoWayBatchedPcgSolve> {
    let mut interrupt = NeverInterrupt;
    solve_two_way_pcg_batch_with_interrupt(
        operator,
        preconditioner,
        worker_rhs,
        firm_rhs,
        columns,
        options,
        full_residual_tolerance,
        &mut interrupt,
    )
}

pub fn solve_two_way_pcg_batch_with_interrupt(
    operator: &TwoWayOperator<'_>,
    preconditioner: &impl Preconditioner,
    worker_rhs: &[f64],
    firm_rhs: &[f64],
    columns: usize,
    options: PcgOptions,
    full_residual_tolerance: f64,
    interrupt: &mut dyn InterruptCheck,
) -> Result<TwoWayBatchedPcgSolve> {
    if columns == 0
        || worker_rhs.len()
            != checked_matrix_length(operator.problem().workers(), columns, "worker RHS")?
        || firm_rhs.len() != checked_matrix_length(operator.problem().firms(), columns, "firm RHS")?
    {
        return Err(BackendError::invalid(
            "batch_pcg",
            "two-way batched right-hand side has incompatible dimensions",
        ));
    }
    if !full_residual_tolerance.is_finite() || full_residual_tolerance <= 0.0 {
        return Err(BackendError::invalid(
            "batch_pcg",
            "full residual tolerance must be positive and finite",
        ));
    }

    let dimension = operator.dimension();
    let mut reduced_rhs = zero_matrix(dimension, columns, "reduced RHS")?;
    for column in 0..columns {
        interrupt.checkpoint("batch_rhs")?;
        let worker_range = column_range(column, operator.problem().workers());
        let firm_range = column_range(column, operator.problem().firms());
        let reduced = operator.schur_rhs_with_interrupt(
            &worker_rhs[worker_range],
            &firm_rhs[firm_range],
            interrupt,
        )?;
        reduced_rhs[column_range(column, dimension)].copy_from_slice(&reduced);
    }
    let reduced = batched_pcg_with_interrupt(
        operator,
        preconditioner,
        &reduced_rhs,
        columns,
        options,
        interrupt,
    )?;
    let mut solution = Vec::with_capacity(columns);
    for column in 0..columns {
        interrupt.checkpoint("batch_full_residual")?;
        let worker_range = column_range(column, operator.problem().workers());
        let firm_range = column_range(column, operator.problem().firms());
        let firm = operator.expand_firm(reduced.column(column))?;
        let worker = operator.reconstruct_worker_with_interrupt(
            &worker_rhs[worker_range.clone()],
            &firm,
            interrupt,
        )?;
        let residual = operator.full_residual_with_interrupt(
            &worker,
            &firm,
            &worker_rhs[worker_range],
            &firm_rhs[firm_range],
            interrupt,
        )?;
        if residual.relative_norm > full_residual_tolerance {
            return Err(column_error(
                ErrorCode::FullResidualFailed,
                column,
                &format!(
                    "complete normal-equation residual {} exceeds tolerance {}",
                    residual.relative_norm, full_residual_tolerance
                ),
            ));
        }
        solution.push(TwoWaySolution {
            worker,
            firm,
            reduced_firm: reduced.column(column).to_vec(),
            residual,
        });
    }
    Ok(TwoWayBatchedPcgSolve {
        columns,
        solution,
        pcg: reduced.receipt,
    })
}

fn apply_preconditioner_columns(
    operator: &TwoWayOperator<'_>,
    preconditioner: &impl Preconditioner,
    input: &[f64],
    output: &mut [f64],
    active: &[bool],
    applications: &mut [u32],
    interrupt: &mut dyn InterruptCheck,
) -> Result<()> {
    output.fill(0.0);
    preconditioner.apply_columns_with_interrupt(
        operator,
        input,
        output,
        active.len(),
        active,
        interrupt,
    )?;
    for (column, &is_active) in active.iter().enumerate() {
        if is_active {
            applications[column] = applications[column]
                .checked_add(1)
                .ok_or_else(|| resource_error("preconditioner application counter overflow"))?;
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn recompute_selected_explicit(
    operator: &TwoWayOperator<'_>,
    right_hand_side: &[f64],
    solution: &[f64],
    explicit_residual: &mut [f64],
    columns: usize,
    dimension: usize,
    selected: &[bool],
    operator_applications: &mut [u32],
    residual_replacements: &mut [u32],
    workspace: &mut TwoWayBatchWorkspace,
    interrupt: &mut dyn InterruptCheck,
) -> Result<()> {
    interrupt.checkpoint("batch_residual_replacement")?;
    apply_two_way_schur_batch_with_interrupt(
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
        let mut begin = range.start;
        while begin < range.end {
            if begin % INTERRUPT_CHECK_CHUNK == 0 {
                interrupt.checkpoint("batch_residual_replacement")?;
            }
            let next_boundary = begin
                .saturating_add(INTERRUPT_CHECK_CHUNK - begin % INTERRUPT_CHECK_CHUNK)
                .min(range.end);
            for index in begin..next_boundary {
                explicit_residual[index] = right_hand_side[index] - explicit_residual[index];
            }
            begin = next_boundary;
        }
        operator.project(&mut explicit_residual[range])?;
        operator_applications[column] = operator_applications[column]
            .checked_add(1)
            .ok_or_else(|| resource_error("operator application counter overflow"))?;
        residual_replacements[column] = residual_replacements[column]
            .checked_add(1)
            .ok_or_else(|| resource_error("residual replacement counter overflow"))?;
    }
    Ok(())
}

fn stable_difference_norm(left: &[f64], right: &[f64]) -> f64 {
    let mut sum = 0.0;
    let mut correction = 0.0;
    for (&left_value, &right_value) in left.iter().zip(right) {
        let difference = left_value - right_value;
        let square = difference * difference;
        let adjusted = square - correction;
        let next = sum + adjusted;
        correction = (next - sum) - adjusted;
        sum = next;
    }
    sum.max(0.0).sqrt()
}

fn increment_active(counter: &mut [u32], active: &[bool]) -> Result<()> {
    for (value, &is_active) in counter.iter_mut().zip(active) {
        if is_active {
            *value = value
                .checked_add(1)
                .ok_or_else(|| resource_error("application counter overflow"))?;
        }
    }
    Ok(())
}

fn zero_receipt() -> PcgReceipt {
    PcgReceipt {
        iterations: 0,
        relative_residual: 0.0,
        residual_replacements: 0,
        operator_applications: 0,
        preconditioner_applications: 0,
        zero_rhs: true,
    }
}

fn collect_receipts(receipt: Vec<Option<PcgReceipt>>) -> Result<Vec<PcgReceipt>> {
    receipt
        .into_iter()
        .enumerate()
        .map(|(column, value)| {
            value.ok_or_else(|| {
                BackendError::invariant(
                    "batch_pcg",
                    format!("missing receipt for zero-based RHS column {column}"),
                )
            })
        })
        .collect()
}

fn require_positive_finite(
    value: f64,
    code: ErrorCode,
    column: usize,
    message: &str,
) -> Result<()> {
    if value.is_finite() && value > 0.0 {
        Ok(())
    } else {
        Err(column_error(code, column, message))
    }
}

fn column_error(code: ErrorCode, column: usize, message: &str) -> BackendError {
    BackendError::new(
        code,
        "batch_pcg",
        format!("zero-based RHS column {column}: {message}"),
    )
}

fn column_range(column: usize, dimension: usize) -> core::ops::Range<usize> {
    let begin = column * dimension;
    begin..begin + dimension
}

fn zero_matrix(rows: usize, columns: usize, label: &str) -> Result<Vec<f64>> {
    Ok(vec![0.0; checked_matrix_length(rows, columns, label)?])
}

fn checked_matrix_length(rows: usize, columns: usize, label: &str) -> Result<usize> {
    rows.checked_mul(columns).ok_or_else(|| {
        resource_error(&format!(
            "{label} matrix length overflow for {rows} by {columns}"
        ))
    })
}

fn resource_error(message: &str) -> BackendError {
    BackendError::new(ErrorCode::ResourceLimit, "batch_pcg", message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::krylov::{pcg, DiagonalPreconditioner};
    use crate::problem::CanonicalInput;
    use crate::types::InputColumns;

    struct BreakOnPhase {
        phase: &'static str,
        break_call: usize,
        calls: usize,
    }

    impl InterruptCheck for BreakOnPhase {
        fn checkpoint(&mut self, phase: &'static str) -> Result<()> {
            if phase == self.phase {
                self.calls += 1;
                if self.calls == self.break_call {
                    return Err(BackendError::new(
                        ErrorCode::UserBreak,
                        phase,
                        "injected flattened-work break",
                    ));
                }
            }
            Ok(())
        }
    }

    fn fixture() -> crate::problem::CompressedProblem {
        let rows = 12_usize;
        CanonicalInput::from_validated(
            InputColumns {
                worker: vec![1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6],
                firm: vec![1, 2, 2, 3, 3, 4, 4, 1, 1, 3, 2, 4],
                deletion: (1..=u64::try_from(rows).expect("rows")).collect(),
                outcome: vec![
                    1.0, -1.0, 2.0, -2.0, 3.0, -3.0, 4.0, -4.0, 2.0, -2.0, 1.0, -1.0,
                ],
                frequency: vec![1; rows],
                target_weight: vec![1.0; rows],
                controls: Vec::new(),
            }
            .validate()
            .expect("fixture"),
        )
        .expect("canonical")
        .compress(&vec![true; rows])
        .expect("compressed")
    }

    fn options() -> PcgOptions {
        PcgOptions {
            tolerance: 1.0e-12,
            maximum_iterations: 200,
            residual_replacement_interval: 7,
        }
    }

    fn weak_cycle_fixture(firms: usize) -> crate::problem::CompressedProblem {
        let workers = firms;
        let rows = workers * 4;
        let mut worker = Vec::with_capacity(rows);
        let mut firm = Vec::with_capacity(rows);
        for worker_index in 0..workers {
            for period in 0..4 {
                worker.push(u64::try_from(worker_index + 1).expect("worker"));
                let firm_index = if period < 2 {
                    worker_index
                } else {
                    (worker_index + 1) % firms
                };
                firm.push(u64::try_from(firm_index + 1).expect("firm"));
            }
        }
        CanonicalInput::from_validated(
            InputColumns {
                worker,
                firm,
                deletion: (1..=u64::try_from(rows).expect("rows")).collect(),
                outcome: (0..rows)
                    .map(|index| (f64::from(u32::try_from(index).expect("index")) / 17.0).sin())
                    .collect(),
                frequency: vec![1; rows],
                target_weight: vec![1.0; rows],
                controls: Vec::new(),
            }
            .validate()
            .expect("weak cycle fixture"),
        )
        .expect("weak cycle canonical")
        .compress(&vec![true; rows])
        .expect("weak cycle compressed")
    }

    #[test]
    fn batched_operator_matches_scalar_actions_bitwise() {
        let problem = fixture();
        let operator = TwoWayOperator::new(&problem).expect("operator");
        let dimension = operator.dimension();
        let columns = 3;
        let input = vec![
            0.1, -0.3, 0.2, 0.4, -0.4, 0.7, 0.8, -0.1, -1.0, 0.5, -0.2, 0.9,
        ];
        assert_eq!(input.len(), dimension * columns);
        let mut workspace = TwoWayBatchWorkspace::new(&operator, columns).expect("workspace");
        let mut output = vec![0.0; input.len()];
        apply_two_way_schur_batch(&operator, &input, &mut output, columns, &mut workspace)
            .expect("batch action");
        for column in 0..columns {
            let range = column_range(column, dimension);
            let mut scalar = vec![0.0; dimension];
            operator
                .apply(&input[range.clone()], &mut scalar)
                .expect("scalar action");
            assert_eq!(&output[range], scalar.as_slice());
        }
    }

    #[test]
    fn batched_schur_polls_flattened_cell_column_work() {
        let problem = fixture();
        let operator = TwoWayOperator::new(&problem).expect("operator");
        let columns = 342;
        assert!(problem.cells() * columns > crate::interrupt::INTERRUPT_CHECK_CHUNK);
        let dimension = operator.dimension();
        let input = vec![0.0; dimension * columns];
        let mut output = vec![0.0; input.len()];
        let mut workspace = TwoWayBatchWorkspace::new(&operator, columns).expect("workspace");
        let mut interrupt = BreakOnPhase {
            phase: "batch_operator_cells_accumulate",
            break_call: 2,
            calls: 0,
        };
        let error = apply_two_way_schur_batch_with_interrupt(
            &operator,
            &input,
            &mut output,
            columns,
            &mut workspace,
            &mut interrupt,
        )
        .expect_err("flattened cell-column kernel must poll after 4096 updates");
        assert_eq!(error.code, ErrorCode::UserBreak);
        assert_eq!(error.phase, "batch_operator_cells_accumulate");
        assert_eq!(interrupt.calls, 2);
    }

    #[test]
    fn batched_pcg_matches_independent_scalar_solves() {
        let problem = fixture();
        let operator = TwoWayOperator::new(&problem).expect("operator");
        let preconditioner =
            DiagonalPreconditioner::new(operator.reduced_diagonal()).expect("diagonal");
        let dimension = operator.dimension();
        let columns = 3;
        let rhs = vec![
            1.0, -0.5, 0.25, -0.75, -1.0, 0.5, -0.25, 0.75, 0.0, 0.0, 0.0, 0.0,
        ];
        assert_eq!(rhs.len(), dimension * columns);
        let batch =
            batched_pcg(&operator, &preconditioner, &rhs, columns, options()).expect("batch solve");
        for column in 0..columns {
            let range = column_range(column, dimension);
            let scalar =
                pcg(&operator, &preconditioner, &rhs[range], options()).expect("scalar solve");
            for (&left, &right) in batch.column(column).iter().zip(&scalar.solution) {
                assert!((left - right).abs() < 1.0e-13);
            }
            assert!(
                (batch.receipt[column].relative_residual - scalar.receipt.relative_residual).abs()
                    < 1.0e-13
            );
            assert_eq!(batch.receipt[column].zero_rhs, scalar.receipt.zero_rhs);
        }
        assert!(batch.receipt[2].zero_rhs);
    }

    #[test]
    fn two_way_batch_certifies_every_full_residual() {
        let problem = fixture();
        let operator = TwoWayOperator::new(&problem).expect("operator");
        let preconditioner =
            DiagonalPreconditioner::new(operator.reduced_diagonal()).expect("diagonal");
        let (worker, firm) = operator.outcome_rhs().expect("outcome RHS");
        let mut worker_batch = Vec::new();
        let mut firm_batch = Vec::new();
        worker_batch.extend_from_slice(&worker);
        worker_batch.extend(worker.iter().map(|value| -value));
        worker_batch.extend(vec![0.0; worker.len()]);
        firm_batch.extend_from_slice(&firm);
        firm_batch.extend(firm.iter().map(|value| -value));
        firm_batch.extend(vec![0.0; firm.len()]);
        let solve = solve_two_way_pcg_batch(
            &operator,
            &preconditioner,
            &worker_batch,
            &firm_batch,
            3,
            options(),
            1.0e-11,
        )
        .expect("two-way batch");
        assert_eq!(solve.solution.len(), 3);
        assert!(solve
            .solution
            .iter()
            .all(|item| item.residual.relative_norm <= 1.0e-11));
        assert!(solve.pcg[2].zero_rhs);
    }

    #[test]
    fn periodic_residual_checks_preserve_conjugacy_on_a_weak_cycle() {
        let problem = weak_cycle_fixture(250);
        let operator = TwoWayOperator::new(&problem).expect("operator");
        let preconditioner =
            DiagonalPreconditioner::new(operator.reduced_diagonal()).expect("diagonal");
        let dimension = operator.dimension();
        let first = (0..dimension)
            .map(|index| {
                let value = f64::from(u32::try_from(index + 1).expect("index"));
                (value / 19.0).sin() + (value / 31.0).cos()
            })
            .collect::<Vec<_>>();
        let mut rhs = first.clone();
        rhs.extend(first.iter().map(|value| -value));
        let options = PcgOptions {
            tolerance: 1.0e-10,
            maximum_iterations: 400,
            residual_replacement_interval: 50,
        };
        let batch = batched_pcg(&operator, &preconditioner, &rhs, 2, options)
            .expect("weak-cycle batch solve");
        let scalar =
            pcg(&operator, &preconditioner, &first, options).expect("weak-cycle scalar solve");
        assert!(scalar.receipt.iterations <= 300);
        assert!(scalar.receipt.residual_replacements >= 2);
        for receipt in &batch.receipt {
            assert!(receipt.iterations <= 300);
            assert!(receipt.residual_replacements >= 2);
            assert!(receipt.relative_residual <= options.tolerance);
        }
        for (&left, &right) in batch.column(0).iter().zip(&scalar.solution) {
            assert!((left - right).abs() < 1.0e-11);
        }
    }
}
