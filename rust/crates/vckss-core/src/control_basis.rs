// SPDX-License-Identifier: GPL-3.0-only

//! Deterministic, coordinate-free representatives of a low-dimensional
//! control span.
//!
//! The algorithm mirrors the Mata oracle: weighted whitening is followed by
//! deterministic row-pivoted anchor selection and an RREF-style span map.
//! It deliberately fails closed at ambiguous score boundaries and is
//! certified only for at most 32 controls.

use crate::dense::{cholesky_factor, inverse_forward_error, invert_scaled_spd};
use crate::error::{BackendError, ErrorCode, Result};
use crate::interrupt::{
    checkpoint_chunk, stable_sort_by_with_interrupt, InterruptCheck, NeverInterrupt,
};
use crate::model_operator::{checked_matrix_length, reserve_exact, zeroed_f64_with_interrupt};

pub const MAX_CANONICAL_CONTROLS: usize = 32;
const CONTROL_FORWARD_LIMIT: f64 = 1.0e-8;

#[derive(Clone, Copy, Debug, Default)]
struct StableAccumulator {
    sum: f64,
    correction: f64,
}

impl StableAccumulator {
    fn add(&mut self, value: f64) {
        let next = self.sum + value;
        if self.sum.abs() >= value.abs() {
            self.correction += (self.sum - next) + value;
        } else {
            self.correction += (value - next) + self.sum;
        }
        self.sum = next;
    }

    fn finish(self) -> f64 {
        self.sum + self.correction
    }
}

#[derive(Clone, Debug)]
pub struct ControlBasisReceipt {
    pub controls: usize,
    pub relres: f64,
    pub forward_error: f64,
    pub whitening_residual: f64,
    pub anchor_residual: f64,
    pub span_residual: f64,
}

#[derive(Clone, Debug)]
pub struct CanonicalControlBasis {
    /// Column-major control storage, matching [`crate::problem::CompressedProblem`].
    pub columns: Vec<Vec<f64>>,
    pub receipt: ControlBasisReceipt,
}

pub fn canonicalize_controls(
    controls: &[Vec<f64>],
    frequency: &[u64],
    rank_tolerance: f64,
) -> Result<CanonicalControlBasis> {
    canonicalize_controls_with_interrupt(controls, frequency, rank_tolerance, &mut NeverInterrupt)
}

pub fn canonicalize_controls_with_interrupt(
    controls: &[Vec<f64>],
    frequency: &[u64],
    rank_tolerance: f64,
    interrupt: &mut dyn InterruptCheck,
) -> Result<CanonicalControlBasis> {
    let order = index_vector(frequency.len(), interrupt, "control_basis_default_order")?;
    canonicalize_controls_in_order_with_interrupt(
        controls,
        frequency,
        &order,
        rank_tolerance,
        interrupt,
    )
}

/// Refine already-sorted semantic tie classes without using caller row
/// positions or raw control coordinates as the registered tie breaker.
///
/// Each tied row receives a signature of its row in the weighted projection
/// kernel `X (X' W X)^{-1} X'`.  The signature consists of the diagonal and
/// compensated power sums of the kernel entries in the tie class, hence is
/// invariant to every nonsingular change of control coordinates.  A fixed
/// length signature avoids an unbounded `N x N` allocation.  If two distinct
/// control rows cannot be separated beyond the certified inverse/rounding
/// uncertainty, the contract fails closed rather than consulting input order.
pub(crate) fn refine_control_semantic_order_with_interrupt<F>(
    controls: &[Vec<f64>],
    frequency: &[u64],
    coarse_order: &[usize],
    rank_tolerance: f64,
    mut coarse_equal: F,
    interrupt: &mut dyn InterruptCheck,
) -> Result<Vec<usize>>
where
    F: FnMut(usize, usize) -> bool,
{
    let rows = frequency.len();
    let q = controls.len();
    if coarse_order.len() != rows {
        return Err(BackendError::invalid(
            "control_basis_tie_order",
            "coarse semantic order has the wrong length",
        ));
    }
    let mut seen = zeroed_bool_with_interrupt(
        rows,
        "control tie order seen flags",
        interrupt,
        "control_basis_tie_allocate",
    )?;
    for (position, &row) in coarse_order.iter().enumerate() {
        checkpoint_chunk(interrupt, position, "control_basis_tie_validate")?;
        if row >= rows || seen[row] {
            return Err(BackendError::invalid(
                "control_basis_tie_order",
                "coarse semantic order is not a permutation",
            ));
        }
        seen[row] = true;
    }
    drop(seen);
    for (control, column) in controls.iter().enumerate() {
        checkpoint_chunk(interrupt, control, "control_basis_tie_validate")?;
        if column.len() != rows {
            return Err(BackendError::invalid(
                "control_basis_tie_order",
                "control column has the wrong length",
            ));
        }
    }
    let mut order = copy_usize_with_interrupt(
        coarse_order,
        "control semantic tie order",
        interrupt,
        "control_basis_tie_order",
    )?;
    if q == 0 || rows < 2 {
        return Ok(order);
    }

    // This provisional order is used only to freeze summation order. It never
    // becomes a semantic tie breaker: the final order below is kernel based.
    let mut cursor = 0;
    let mut has_semantic_ties = false;
    while cursor < rows {
        checkpoint_chunk(interrupt, cursor, "control_basis_tie_classes")?;
        let begin = cursor;
        cursor += 1;
        while cursor < rows && coarse_equal(order[begin], order[cursor]) {
            checkpoint_chunk(interrupt, cursor - begin, "control_basis_tie_classes")?;
            cursor += 1;
        }
        if cursor - begin > 1 {
            has_semantic_ties = true;
            stable_sort_by_with_interrupt(
                &mut order[begin..cursor],
                |&left, &right| {
                    controls
                        .iter()
                        .map(|column| column[left].total_cmp(&column[right]))
                        .find(|comparison| !comparison.is_eq())
                        .unwrap_or(core::cmp::Ordering::Equal)
                },
                interrupt,
                "control_basis_tie_provisional",
            )?;
        }
    }
    if !has_semantic_ties {
        return Ok(order);
    }

    let original = columns_to_row_major_in_order(
        controls,
        &order,
        rows,
        q,
        interrupt,
        "control_basis_tie_copy",
    )?;
    let mut ordered_frequency = Vec::new();
    reserve_exact(
        &mut ordered_frequency,
        rows,
        "control tie ordered frequencies",
    )?;
    for (position, &row) in order.iter().enumerate() {
        checkpoint_chunk(interrupt, position, "control_basis_tie_frequency")?;
        ordered_frequency.push(frequency[row]);
    }
    let gram = weighted_crossproduct(&original, rows, q, &ordered_frequency, interrupt)?;
    let inverse = invert_scaled_spd(
        &gram,
        q,
        rank_tolerance,
        interrupt,
        "control_basis_tie_gram",
    )
    .map_err(|error| {
        preserve_break_or(
            error,
            ErrorCode::SingularInformation,
            "control-span kernel is singular while refining semantic ties",
        )
    })?;
    let forward = inverse_forward_error(inverse.relres, inverse.rcond, q)
        .ok_or_else(|| ambiguous("control-span tie kernel has no certified forward bound"))?;
    let uncertainty = (256.0 * (forward + rounding_gamma(8.0 * q as f64)?)).max(1.0e-12);
    if !uncertainty.is_finite() || uncertainty >= 0.25 {
        return Err(ambiguous(
            "control-span tie kernel has excessive numerical uncertainty",
        ));
    }

    const SIGNATURE_VALUES: usize = 8;
    let signature_len = checked_matrix_length(rows, SIGNATURE_VALUES, "control tie signatures")?;
    let mut signatures = zeroed_f64_with_interrupt(
        signature_len,
        "control tie signatures",
        interrupt,
        "control_basis_tie_allocate",
    )?;
    let mut row_position = zeroed_usize_with_interrupt(
        rows,
        "control tie row positions",
        interrupt,
        "control_basis_tie_allocate",
    )?;
    for (position, &row) in order.iter().enumerate() {
        checkpoint_chunk(interrupt, position, "control_basis_tie_positions")?;
        row_position[row] = position;
    }
    cursor = 0;
    while cursor < rows {
        checkpoint_chunk(interrupt, cursor, "control_basis_tie_signature")?;
        let begin = cursor;
        cursor += 1;
        while cursor < rows && coarse_equal(order[begin], order[cursor]) {
            checkpoint_chunk(interrupt, cursor - begin, "control_basis_tie_signature")?;
            cursor += 1;
        }
        if cursor - begin < 2 {
            continue;
        }
        for left in begin..cursor {
            let mut powers = [StableAccumulator::default(); 6];
            let mut diagonal = 0.0;
            let mut maximum = f64::NEG_INFINITY;
            for right in begin..cursor {
                checkpoint_chunk(
                    interrupt,
                    (left - begin) * (cursor - begin) + right - begin,
                    "control_basis_tie_kernel",
                )?;
                let mut kernel = StableAccumulator::default();
                for a in 0..q {
                    for b in 0..q {
                        kernel.add(
                            original[left * q + a]
                                * inverse.inverse[a * q + b]
                                * original[right * q + b],
                        );
                    }
                }
                let kernel = kernel.finish();
                if !kernel.is_finite() {
                    return Err(ambiguous(
                        "control-span tie kernel produced a nonfinite invariant",
                    ));
                }
                if left == right {
                    diagonal = kernel;
                }
                maximum = maximum.max(kernel);
                let mut power = kernel;
                for accumulator in &mut powers {
                    accumulator.add(power);
                    power *= kernel;
                }
            }
            let base = left * SIGNATURE_VALUES;
            signatures[base] = diagonal;
            signatures[base + 1] = maximum;
            for (power, accumulator) in powers.into_iter().enumerate() {
                signatures[base + 2 + power] = accumulator.finish();
            }
            if signatures[base..base + SIGNATURE_VALUES]
                .iter()
                .any(|value| !value.is_finite())
            {
                return Err(ambiguous(
                    "control-span tie signature produced a nonfinite invariant",
                ));
            }
        }
        stable_sort_by_with_interrupt(
            &mut order[begin..cursor],
            |&left_row, &right_row| {
                let left_position = row_position[left_row];
                let right_position = row_position[right_row];
                (0..SIGNATURE_VALUES)
                    .map(|value| {
                        signatures[left_position * SIGNATURE_VALUES + value]
                            .total_cmp(&signatures[right_position * SIGNATURE_VALUES + value])
                    })
                    .find(|comparison| !comparison.is_eq())
                    .unwrap_or(core::cmp::Ordering::Equal)
            },
            interrupt,
            "control_basis_tie_kernel_sort",
        )?;
        for position in (begin + 1)..cursor {
            checkpoint_chunk(interrupt, position - begin, "control_basis_tie_certify")?;
            let left_row = order[position - 1];
            let right_row = order[position];
            let identical_contribution = controls
                .iter()
                .all(|column| column[left_row] == column[right_row]);
            let left_position = row_position[left_row];
            let right_position = row_position[right_row];
            certify_lexicographic_signature_pair(
                &signatures
                    [left_position * SIGNATURE_VALUES..(left_position + 1) * SIGNATURE_VALUES],
                &signatures
                    [right_position * SIGNATURE_VALUES..(right_position + 1) * SIGNATURE_VALUES],
                uncertainty,
                identical_contribution,
            )?;
        }
    }
    Ok(order)
}

fn certify_lexicographic_signature_pair(
    left: &[f64],
    right: &[f64],
    uncertainty: f64,
    identical_contribution: bool,
) -> Result<()> {
    if left.len() != right.len() {
        return Err(BackendError::invariant(
            "control_basis_tie_order",
            "control tie signature dimensions disagree",
        ));
    }
    if identical_contribution {
        return Ok(());
    }
    let left_value = left.first().copied().ok_or_else(|| {
        BackendError::invariant("control_basis_tie_order", "control tie signature is empty")
    })?;
    let right_value = right[0];
    let separation = (left_value - right_value).abs();
    let combined_bound = 2.0 * uncertainty * (1.0 + left_value.abs().max(right_value.abs()));
    if !separation.is_finite() || !combined_bound.is_finite() || separation <= combined_bound {
        return Err(ambiguous(
            "distinct tied control rows are not separated by the first kernel-signature coordinate",
        ));
    }
    // Computed equality is not a proof of mathematical equality. Coordinate
    // zero alone certifies every pair of distinct contribution rows.
    Ok(())
}

/// Canonicalize after placing rows in a validated, control-coordinate-free
/// semantic order. The returned columns are mapped back to caller row order.
pub fn canonicalize_controls_in_order_with_interrupt(
    controls: &[Vec<f64>],
    frequency: &[u64],
    semantic_order: &[usize],
    rank_tolerance: f64,
    interrupt: &mut dyn InterruptCheck,
) -> Result<CanonicalControlBasis> {
    let rows = frequency.len();
    if semantic_order.len() != rows {
        return Err(BackendError::invalid(
            "control_basis",
            "canonical semantic order has the wrong length",
        ));
    }
    let mut seen = zeroed_bool_with_interrupt(
        rows,
        "control semantic seen flags",
        interrupt,
        "control_basis_semantic_allocate",
    )?;
    for (position, &row) in semantic_order.iter().enumerate() {
        checkpoint_chunk(interrupt, position, "control_basis_semantic_order")?;
        if row >= rows || seen[row] {
            return Err(BackendError::invalid(
                "control_basis",
                "canonical semantic order is not a permutation",
            ));
        }
        seen[row] = true;
    }
    drop(seen);
    let mut ordered_frequency = Vec::new();
    reserve_exact(
        &mut ordered_frequency,
        rows,
        "control semantic ordered frequencies",
    )?;
    for (position, &row) in semantic_order.iter().enumerate() {
        checkpoint_chunk(interrupt, position, "control_basis_semantic_frequency")?;
        ordered_frequency.push(frequency[row]);
    }
    let mut ordered_controls = zeroed_matrix_columns(
        controls.len(),
        rows,
        "control semantic ordered controls",
        interrupt,
        "control_basis_semantic_allocate",
    )?;
    for (control, column) in controls.iter().enumerate() {
        if column.len() != rows {
            return Err(BackendError::invalid(
                "control_basis",
                "canonical-control column has the wrong length",
            ));
        }
        for (position, &row) in semantic_order.iter().enumerate() {
            checkpoint_chunk(
                interrupt,
                control * rows + position,
                "control_basis_semantic_copy",
            )?;
            ordered_controls[control][position] = column[row];
        }
    }
    let mut result = canonicalize_controls_ordered_input_with_interrupt(
        &ordered_controls,
        &ordered_frequency,
        rank_tolerance,
        interrupt,
    )?;
    drop(ordered_controls);
    drop(ordered_frequency);
    let mut restored = zeroed_matrix_columns(
        result.columns.len(),
        rows,
        "restored canonical controls",
        interrupt,
        "control_basis_semantic_allocate",
    )?;
    for (control, column) in result.columns.iter().enumerate() {
        for (position, &row) in semantic_order.iter().enumerate() {
            checkpoint_chunk(
                interrupt,
                control * rows + position,
                "control_basis_semantic_restore",
            )?;
            restored[control][row] = column[position];
        }
    }
    result.columns = restored;
    Ok(result)
}

fn canonicalize_controls_ordered_input_with_interrupt(
    controls: &[Vec<f64>],
    frequency: &[u64],
    rank_tolerance: f64,
    interrupt: &mut dyn InterruptCheck,
) -> Result<CanonicalControlBasis> {
    let rows = frequency.len();
    let control_count = controls.len();
    if rows == 0 || !rank_tolerance.is_finite() || rank_tolerance <= 0.0 {
        return Err(BackendError::invalid(
            "control_basis",
            "canonical-control inputs are invalid",
        ));
    }
    for (control, column) in controls.iter().enumerate() {
        checkpoint_chunk(interrupt, control, "control_basis_validate")?;
        if column.len() != rows {
            return Err(BackendError::invalid(
                "control_basis",
                "canonical-control inputs are invalid",
            ));
        }
        for (row, value) in column.iter().enumerate() {
            checkpoint_chunk(
                interrupt,
                control
                    .checked_mul(rows)
                    .and_then(|offset| offset.checked_add(row))
                    .ok_or_else(|| resource("control validation work overflow"))?,
                "control_basis_validate",
            )?;
            if !value.is_finite() {
                return Err(BackendError::invalid(
                    "control_basis",
                    "canonical-control inputs are invalid",
                ));
            }
        }
    }
    for (row, &value) in frequency.iter().enumerate() {
        checkpoint_chunk(interrupt, row, "control_basis_validate")?;
        if value == 0 {
            return Err(BackendError::invalid(
                "control_basis",
                "canonical-control inputs are invalid",
            ));
        }
    }
    if control_count == 0 {
        return Ok(CanonicalControlBasis {
            columns: Vec::new(),
            receipt: ControlBasisReceipt {
                controls: 0,
                relres: 0.0,
                forward_error: 0.0,
                whitening_residual: 0.0,
                anchor_residual: 0.0,
                span_residual: 0.0,
            },
        });
    }
    if control_count > MAX_CANONICAL_CONTROLS {
        return Err(BackendError::new(
            ErrorCode::AmbiguousControlBasis,
            "control_basis",
            "control count exceeds the certified canonical-basis range (Q <= 32)",
        ));
    }

    let original = columns_to_row_major(controls, rows, control_count, interrupt)?;
    let gram_product = certified_weighted_product(
        &original,
        &original,
        rows,
        control_count,
        frequency,
        interrupt,
    )?;
    let gram = &gram_product.values;
    let gram_inverse = invert_scaled_spd(
        gram,
        control_count,
        rank_tolerance,
        interrupt,
        "control_basis_gram",
    )
    .map_err(|error| {
        if error.code == ErrorCode::SingularInformation {
            BackendError::new(
                ErrorCode::SingularInformation,
                "control_basis",
                "requested controls are singular before FE absorption",
            )
        } else {
            error
        }
    })?;
    let summation_error = scaled_gram_error(&gram_product, control_count, gram_inverse.rcond)?;
    drop(gram_product);
    let whitener = cholesky_factor(
        &gram_inverse.inverse,
        control_count,
        interrupt,
        "control_basis_whitener",
    )?;
    let orthonormal = multiply(
        &original,
        rows,
        control_count,
        &whitener,
        control_count,
        interrupt,
    )?;
    drop(original);
    drop(whitener);
    let checked = certified_weighted_product(
        &orthonormal,
        &orthonormal,
        rows,
        control_count,
        frequency,
        interrupt,
    )?;
    let whitening_error = bound_up(
        identity_residual(&checked.values, control_count, interrupt)?
            + checked.norm_error(control_count),
    );
    drop(checked);
    let margin = (1.0e-10_f64).max(1000.0 * rank_tolerance);
    if !whitening_error.is_finite() || whitening_error > margin {
        return Err(BackendError::new(
            ErrorCode::SingularInformation,
            "control_basis",
            "control-span whitening failed its residual gate",
        ));
    }

    let q = control_count as f64;
    let cholesky_error = q * rounding_gamma(2.0 * q + 1.0)? / gram_inverse.rcond;
    let basis_product_error = rounding_gamma(2.0 * q)? * (1.0 + gram_inverse.rcond.recip());
    let inverse_error =
        inverse_forward_error(gram_inverse.relres, gram_inverse.rcond, control_count).ok_or_else(
            || ambiguous("control-span whitening error has no certified forward bound"),
        )?;
    let mut numerical_error = bound_up(
        whitening_error + summation_error + cholesky_error + basis_product_error + inverse_error,
    );
    if !numerical_error.is_finite() || numerical_error >= 0.25 {
        return Err(ambiguous(
            "control-span whitening error has no certified forward bound",
        ));
    }
    numerical_error = bound_up(numerical_error / (1.0 - numerical_error));
    let mut uncertainty = score_uncertainty(numerical_error, q)?;
    if uncertainty >= margin / 4.0 {
        return Err(ambiguous(
            "control-span whitening error is too large to certify a canonical anchor",
        ));
    }

    let mut selected = Vec::new();
    reserve_exact(&mut selected, control_count, "control anchor row indices")?;
    let mut residualized = copy_f64_local(
        &orthonormal,
        "control residualized matrix",
        interrupt,
        "control_basis_residualized_copy",
    )?;
    let mut last_anchor_inverse = None;
    for pivot in 0..control_count {
        checkpoint_chunk(interrupt, pivot, "control_basis_anchor")?;
        let score = row_norm_squares(&residualized, rows, control_count, interrupt)?;
        let chosen = certified_anchor(&score, margin, uncertainty, interrupt)?;
        selected.push(chosen);
        let anchor = select_rows(&orthonormal, control_count, &selected, interrupt)?;
        let anchor_gram = crossproduct(&anchor, pivot + 1, control_count, interrupt)?;
        let anchor_inverse = invert_scaled_spd(
            &anchor_gram,
            pivot + 1,
            rank_tolerance,
            interrupt,
            "control_basis_anchor_inverse",
        )
        .map_err(|error| {
            preserve_break_or(
                error,
                ErrorCode::SingularInformation,
                "control-span anchor is numerically singular",
            )
        })?;
        let anchor_forward_error =
            inverse_forward_error(anchor_inverse.relres, anchor_inverse.rcond, pivot + 1)
                .ok_or_else(|| {
                    ambiguous("control-span anchor update has no certified forward bound")
                })?;
        let product_error = rounding_gamma(6.0 * q)?
            * (1.0 + (pivot + 1) as f64 / anchor_inverse.rcond.max(rank_tolerance));
        let projector = anchor_projector(
            &anchor,
            pivot + 1,
            control_count,
            &anchor_inverse.inverse,
            interrupt,
        )?;
        let projector_square = multiply(
            &projector,
            control_count,
            control_count,
            &projector,
            control_count,
            interrupt,
        )?;
        let projection_error = difference_norm(
            &projector_square,
            &projector,
            interrupt,
            "control_basis_projector_difference",
        )? + symmetry_residual(&projector, control_count, interrupt)?;
        let projected = multiply(
            &orthonormal,
            rows,
            control_count,
            &projector,
            control_count,
            interrupt,
        )?;
        residualized = subtract_matrix(
            &orthonormal,
            &projected,
            interrupt,
            "control_basis_residualized_subtract",
        )?;
        let selected_residual = select_rows(&residualized, control_count, &selected, interrupt)?;
        let selected_zero_error = stable_norm_with_interrupt(
            &selected_residual,
            interrupt,
            "control_basis_selected_residual",
        )?;
        numerical_error = bound_up(
            numerical_error
                + anchor_forward_error
                + product_error
                + projection_error
                + selected_zero_error,
        );
        if !numerical_error.is_finite() || numerical_error >= 0.25 {
            return Err(ambiguous(
                "control-span anchor update has no certified forward bound",
            ));
        }
        uncertainty = score_uncertainty(numerical_error, q)?;
        if uncertainty >= margin / 4.0 {
            return Err(ambiguous(
                "control-span anchor error is too large to certify a canonical basis",
            ));
        }
        last_anchor_inverse = Some(anchor_inverse);
    }

    drop(residualized);
    let anchor = select_rows(&orthonormal, control_count, &selected, interrupt)?;
    let anchor_inverse = last_anchor_inverse.expect("positive control count has an anchor inverse");
    let anchor_transpose_inverse = transpose_times(
        &anchor,
        control_count,
        control_count,
        &anchor_inverse.inverse,
        control_count,
        interrupt,
    )?;
    let canonical = multiply(
        &orthonormal,
        rows,
        control_count,
        &anchor_transpose_inverse,
        control_count,
        interrupt,
    )?;
    let selected_canonical = select_rows(&canonical, control_count, &selected, interrupt)?;
    let anchor_error = identity_residual(&selected_canonical, control_count, interrupt)?;
    let checked_span = certified_weighted_product(
        &orthonormal,
        &canonical,
        rows,
        control_count,
        frequency,
        interrupt,
    )?;
    let projected_span = multiply(
        &orthonormal,
        rows,
        control_count,
        &checked_span.values,
        control_count,
        interrupt,
    )?;
    let span_error = bound_up(
        (difference_norm(
            &canonical,
            &projected_span,
            interrupt,
            "control_basis_span_difference",
        )? + checked_span.norm_error(control_count) * (q * (1.0 + whitening_error)).sqrt())
            / stable_norm_with_interrupt(&canonical, interrupt, "control_basis_span_norm")?
                .max(1.0),
    );
    let anchor_forward_error =
        inverse_forward_error(anchor_inverse.relres, anchor_inverse.rcond, control_count)
            .ok_or_else(|| ambiguous("complete control-span anchor has no forward-error bound"))?;
    let product_error =
        rounding_gamma(8.0 * q)? * (1.0 + q / anchor_inverse.rcond.max(rank_tolerance));
    let canonical_error = anchor_error + span_error + anchor_forward_error + product_error;
    numerical_error = bound_up(numerical_error + canonical_error);
    let mut canonical_finite = true;
    for (index, value) in canonical.iter().enumerate() {
        checkpoint_chunk(interrupt, index, "control_basis_output_validate")?;
        canonical_finite &= value.is_finite();
    }
    if !canonical_finite
        || !anchor_error.is_finite()
        || !span_error.is_finite()
        || !numerical_error.is_finite()
        || numerical_error >= 0.25
        || anchor_error > margin
    {
        return Err(BackendError::new(
            ErrorCode::SingularInformation,
            "control_basis",
            "canonical control basis failed its anchor residual gate",
        ));
    }
    Ok(CanonicalControlBasis {
        columns: row_major_to_columns(&canonical, rows, control_count, interrupt)?,
        receipt: ControlBasisReceipt {
            controls: control_count,
            relres: gram_inverse
                .relres
                .max(whitening_error)
                .max(anchor_inverse.relres)
                .max(anchor_error),
            forward_error: numerical_error,
            whitening_residual: whitening_error,
            anchor_residual: anchor_error,
            span_residual: span_error,
        },
    })
}

pub(crate) fn propagated_error(forward_error: f64, reciprocal_margin: f64) -> Option<f64> {
    if !forward_error.is_finite()
        || !reciprocal_margin.is_finite()
        || forward_error < 0.0
        || reciprocal_margin <= forward_error
    {
        None
    } else {
        Some(forward_error / (reciprocal_margin - forward_error))
    }
}

pub(crate) fn enforce_downstream_bound(
    forward_error: f64,
    reciprocal_margin: f64,
    message: &'static str,
) -> Result<()> {
    let propagated =
        propagated_error(forward_error, reciprocal_margin).ok_or_else(|| ambiguous(message))?;
    if propagated > CONTROL_FORWARD_LIMIT {
        return Err(ambiguous(message));
    }
    Ok(())
}

// Bounds use full epsilon (twice unit roundoff). This outward inflation also
// covers <=16 positive bound operations and the denominators bounded away
// from zero below. MIN_SUBNORMAL covers rounding of subnormal bounds.
fn bound_up(value: f64) -> f64 {
    value * (1.0 + 64.0 * f64::EPSILON) + f64::from_bits(1)
}

struct CertifiedProduct {
    values: Vec<f64>,
    errors: Vec<f64>,
}

impl CertifiedProduct {
    fn norm_error(&self, columns: usize) -> f64 {
        bound_up(columns as f64 * self.errors.iter().copied().fold(0.0_f64, f64::max))
    }
}

// Neumaier's error-free residual and accumulated correction retain the low
// term in [1e16, 1, -1e16]. The n-dependent error is second order, not gamma_n.
// See docs/CONTROL_BASIS_CERTIFICATION.md for product and underflow terms.
fn certified_weighted_product(
    left: &[f64],
    right: &[f64],
    rows: usize,
    columns: usize,
    frequency: &[u64],
    interrupt: &mut dyn InterruptCheck,
) -> Result<CertifiedProduct> {
    let mut values = zeroed_f64_with_interrupt(
        columns * columns,
        "certified control product",
        interrupt,
        "control_basis_crossproduct_allocate",
    )?;
    let mut errors = values.clone();
    let gamma = rounding_gamma(4.0 * rows as f64)?;
    let rho = bound_up(f64::EPSILON + gamma * gamma);
    let products = rounding_gamma(2.0)?;
    if rho >= 0.25 {
        return Err(ambiguous(
            "compensated control sum has no finite certificate",
        ));
    }
    let underflow = bound_up(rows as f64 * f64::from_bits(1) / (1.0 - products));
    let coefficient = bound_up(rho + products / (1.0 - products));
    for a in 0..columns {
        for b in 0..columns {
            let mut value = StableAccumulator::default();
            let mut absolute = StableAccumulator::default();
            for row in 0..rows {
                checkpoint_chunk(
                    interrupt,
                    (a * columns + b) * rows + row,
                    "control_basis_crossproduct",
                )?;
                let weight = frequency[row] as f64;
                // u64::MAX rounds up to 2^64 and a saturating cast would hide it.
                if weight >= 18446744073709551616.0 || weight as u64 != frequency[row] {
                    return Err(resource(
                        "control weight is not exactly representable in binary64",
                    ));
                }
                let term = (weight * left[row * columns + a]) * right[row * columns + b];
                if !term.is_finite() {
                    return Err(ambiguous("nonfinite weighted control product"));
                }
                value.add(term);
                absolute.add(term.abs());
            }
            let index = a * columns + b;
            values[index] = value.finish();
            let magnitude = bound_up(absolute.finish() / (1.0 - rho));
            errors[index] = bound_up(coefficient * magnitude + underflow);
            if !values[index].is_finite() || !errors[index].is_finite() {
                return Err(ambiguous("nonfinite compensated control accumulation"));
            }
        }
    }
    Ok(CertifiedProduct { values, errors })
}

fn scaled_gram_error(product: &CertifiedProduct, columns: usize, rcond: f64) -> Result<f64> {
    let mut norm = 0.0_f64;
    for a in 0..columns {
        let mut row_sum = 0.0;
        for b in 0..columns {
            // Division in two steps avoids overflowing/underflowing a product
            // of diagonal scales. Symmetrization is covered by max(Eab,Eba).
            let error = product.errors[a * columns + b].max(product.errors[b * columns + a]);
            row_sum = bound_up(
                row_sum
                    + bound_up(
                        bound_up(error / product.values[a * columns + a].sqrt())
                            / product.values[b * columns + b].sqrt(),
                    ),
            );
        }
        norm = norm.max(row_sum);
    }
    // The diagonally scaled matrix has lambda_max >= 1 up to scaling
    // roundoff; 1/2 is a conservative lower bound throughout Q <= 32.
    let error = bound_up(2.0 * norm / rcond);
    if !error.is_finite() || error >= 0.25 {
        return Err(ambiguous(
            "control Gram accumulation has no certified forward bound",
        ));
    }
    Ok(error)
}

fn weighted_crossproduct(
    matrix: &[f64],
    rows: usize,
    columns: usize,
    frequency: &[u64],
    interrupt: &mut dyn InterruptCheck,
) -> Result<Vec<f64>> {
    weighted_transpose_product(matrix, matrix, rows, columns, frequency, interrupt)
}

fn weighted_transpose_product(
    left: &[f64],
    right: &[f64],
    rows: usize,
    columns: usize,
    frequency: &[u64],
    interrupt: &mut dyn InterruptCheck,
) -> Result<Vec<f64>> {
    let length = checked_matrix_length(columns, columns, "control weighted product")?;
    let mut output = zeroed_f64_with_interrupt(
        length,
        "control weighted product",
        interrupt,
        "control_basis_crossproduct_allocate",
    )?;
    for a in 0..columns {
        for b in 0..columns {
            let mut value = StableAccumulator::default();
            for row in 0..rows {
                checkpoint_chunk(
                    interrupt,
                    (a * columns + b) * rows + row,
                    "control_basis_crossproduct",
                )?;
                let weight = frequency[row] as f64;
                if weight as u64 != frequency[row] {
                    return Err(BackendError::new(
                        ErrorCode::ResourceLimit,
                        "control_basis",
                        "control weight is not exactly representable in binary64",
                    ));
                }
                value.add(weight * left[row * columns + a] * right[row * columns + b]);
            }
            output[a * columns + b] = value.finish();
        }
    }
    Ok(output)
}

fn crossproduct(
    matrix: &[f64],
    rows: usize,
    columns: usize,
    interrupt: &mut dyn InterruptCheck,
) -> Result<Vec<f64>> {
    let length = checked_matrix_length(rows, rows, "control anchor crossproduct")?;
    let mut output = zeroed_f64_with_interrupt(
        length,
        "control anchor crossproduct",
        interrupt,
        "control_basis_anchor_crossproduct_allocate",
    )?;
    for left in 0..rows {
        for right in 0..rows {
            let mut value = StableAccumulator::default();
            for column in 0..columns {
                checkpoint_chunk(
                    interrupt,
                    (left * rows + right) * columns + column,
                    "control_basis_anchor_crossproduct",
                )?;
                value.add(matrix[left * columns + column] * matrix[right * columns + column]);
            }
            output[left * rows + right] = value.finish();
        }
    }
    Ok(output)
}

fn anchor_projector(
    anchor: &[f64],
    rows: usize,
    columns: usize,
    inverse: &[f64],
    interrupt: &mut dyn InterruptCheck,
) -> Result<Vec<f64>> {
    let transpose_inverse = transpose_times(anchor, rows, columns, inverse, rows, interrupt)?;
    multiply(
        &transpose_inverse,
        columns,
        rows,
        anchor,
        columns,
        interrupt,
    )
}

fn transpose_times(
    left: &[f64],
    left_rows: usize,
    left_columns: usize,
    right: &[f64],
    right_columns: usize,
    interrupt: &mut dyn InterruptCheck,
) -> Result<Vec<f64>> {
    if left.len() != left_rows.saturating_mul(left_columns)
        || right.len() != left_rows.saturating_mul(right_columns)
    {
        return Err(BackendError::invalid(
            "control_basis",
            "transposed matrix multiplication dimensions disagree",
        ));
    }
    let length = checked_matrix_length(left_columns, right_columns, "control transpose product")?;
    let mut output = zeroed_f64_with_interrupt(
        length,
        "control transpose product",
        interrupt,
        "control_basis_transpose_product_allocate",
    )?;
    for row in 0..left_columns {
        for column in 0..right_columns {
            let mut value = StableAccumulator::default();
            for inner in 0..left_rows {
                checkpoint_chunk(
                    interrupt,
                    (row * right_columns + column) * left_rows + inner,
                    "control_basis_transpose_product",
                )?;
                value.add(left[inner * left_columns + row] * right[inner * right_columns + column]);
            }
            output[row * right_columns + column] = value.finish();
        }
    }
    Ok(output)
}

fn multiply(
    left: &[f64],
    left_rows: usize,
    inner: usize,
    right: &[f64],
    right_columns: usize,
    interrupt: &mut dyn InterruptCheck,
) -> Result<Vec<f64>> {
    if left.len() != left_rows.saturating_mul(inner)
        || right.len() != inner.saturating_mul(right_columns)
    {
        return Err(BackendError::invalid(
            "control_basis",
            "matrix multiplication dimensions disagree",
        ));
    }
    let length = checked_matrix_length(left_rows, right_columns, "control matrix product")?;
    let mut output = zeroed_f64_with_interrupt(
        length,
        "control matrix product",
        interrupt,
        "control_basis_product_allocate",
    )?;
    for row in 0..left_rows {
        for column in 0..right_columns {
            let mut value = StableAccumulator::default();
            for index in 0..inner {
                checkpoint_chunk(
                    interrupt,
                    (row * right_columns + column) * inner + index,
                    "control_basis_product",
                )?;
                value.add(left[row * inner + index] * right[index * right_columns + column]);
            }
            output[row * right_columns + column] = value.finish();
        }
    }
    Ok(output)
}

fn identity_residual(
    matrix: &[f64],
    dimension: usize,
    interrupt: &mut dyn InterruptCheck,
) -> Result<f64> {
    let mut square = StableAccumulator::default();
    for row in 0..dimension {
        for column in 0..dimension {
            checkpoint_chunk(
                interrupt,
                row * dimension + column,
                "control_basis_identity_residual",
            )?;
            let value = matrix[row * dimension + column] - if row == column { 1.0 } else { 0.0 };
            square.add(value * value);
        }
    }
    Ok(square.finish().max(0.0).sqrt())
}

fn symmetry_residual(
    matrix: &[f64],
    dimension: usize,
    interrupt: &mut dyn InterruptCheck,
) -> Result<f64> {
    let mut square = StableAccumulator::default();
    for row in 0..dimension {
        for column in 0..dimension {
            checkpoint_chunk(
                interrupt,
                row * dimension + column,
                "control_basis_symmetry_residual",
            )?;
            let value = matrix[row * dimension + column] - matrix[column * dimension + row];
            square.add(value * value);
        }
    }
    Ok(square.finish().max(0.0).sqrt())
}

fn difference_norm(
    left: &[f64],
    right: &[f64],
    interrupt: &mut dyn InterruptCheck,
    phase: &'static str,
) -> Result<f64> {
    if left.len() != right.len() {
        return Err(BackendError::invariant(
            phase,
            "control difference dimensions disagree",
        ));
    }
    let mut square = StableAccumulator::default();
    for (index, (&a, &b)) in left.iter().zip(right).enumerate() {
        checkpoint_chunk(interrupt, index, phase)?;
        let value = a - b;
        square.add(value * value);
    }
    Ok(square.finish().max(0.0).sqrt())
}

fn stable_norm_with_interrupt(
    values: &[f64],
    interrupt: &mut dyn InterruptCheck,
    phase: &'static str,
) -> Result<f64> {
    let mut square = StableAccumulator::default();
    for (index, &value) in values.iter().enumerate() {
        checkpoint_chunk(interrupt, index, phase)?;
        square.add(value * value);
    }
    Ok(square.finish().max(0.0).sqrt())
}

fn subtract_matrix(
    left: &[f64],
    right: &[f64],
    interrupt: &mut dyn InterruptCheck,
    phase: &'static str,
) -> Result<Vec<f64>> {
    if left.len() != right.len() {
        return Err(BackendError::invariant(
            phase,
            "control subtraction dimensions disagree",
        ));
    }
    let mut output = Vec::new();
    reserve_exact(&mut output, left.len(), "control matrix difference")?;
    for (index, (&a, &b)) in left.iter().zip(right).enumerate() {
        checkpoint_chunk(interrupt, index, phase)?;
        output.push(a - b);
    }
    Ok(output)
}

fn row_norm_squares(
    matrix: &[f64],
    rows: usize,
    columns: usize,
    interrupt: &mut dyn InterruptCheck,
) -> Result<Vec<f64>> {
    let mut output = zeroed_f64_with_interrupt(
        rows,
        "control row scores",
        interrupt,
        "control_basis_scores_allocate",
    )?;
    for row in 0..rows {
        let mut value = StableAccumulator::default();
        for column in 0..columns {
            checkpoint_chunk(interrupt, row * columns + column, "control_basis_scores")?;
            let entry = matrix[row * columns + column];
            value.add(entry * entry);
        }
        output[row] = value.finish();
    }
    Ok(output)
}

fn select_rows(
    matrix: &[f64],
    columns: usize,
    selected: &[usize],
    interrupt: &mut dyn InterruptCheck,
) -> Result<Vec<f64>> {
    let length = checked_matrix_length(selected.len(), columns, "selected control rows")?;
    let mut output = Vec::new();
    reserve_exact(&mut output, length, "selected control rows")?;
    for (index, &row) in selected.iter().enumerate() {
        for column in 0..columns {
            checkpoint_chunk(
                interrupt,
                index * columns + column,
                "control_basis_select_rows",
            )?;
            output.push(matrix[row * columns + column]);
        }
    }
    Ok(output)
}

fn columns_to_row_major(
    controls: &[Vec<f64>],
    rows: usize,
    columns: usize,
    interrupt: &mut dyn InterruptCheck,
) -> Result<Vec<f64>> {
    let length = checked_matrix_length(rows, columns, "control row-major copy")?;
    let mut output = zeroed_f64_with_interrupt(
        length,
        "control row-major copy",
        interrupt,
        "control_basis_copy_allocate",
    )?;
    for row in 0..rows {
        for column in 0..columns {
            checkpoint_chunk(interrupt, row * columns + column, "control_basis_copy")?;
            output[row * columns + column] = controls[column][row];
        }
    }
    Ok(output)
}

fn columns_to_row_major_in_order(
    controls: &[Vec<f64>],
    order: &[usize],
    rows: usize,
    columns: usize,
    interrupt: &mut dyn InterruptCheck,
    phase: &'static str,
) -> Result<Vec<f64>> {
    let length = checked_matrix_length(rows, columns, "ordered control row-major copy")?;
    let mut output = zeroed_f64_with_interrupt(
        length,
        "ordered control row-major copy",
        interrupt,
        "control_basis_tie_allocate",
    )?;
    for (position, &row) in order.iter().enumerate() {
        for column in 0..columns {
            checkpoint_chunk(interrupt, position * columns + column, phase)?;
            output[position * columns + column] = controls[column][row];
        }
    }
    Ok(output)
}

fn row_major_to_columns(
    matrix: &[f64],
    rows: usize,
    columns: usize,
    interrupt: &mut dyn InterruptCheck,
) -> Result<Vec<Vec<f64>>> {
    let mut output = zeroed_matrix_columns(
        columns,
        rows,
        "canonical control output",
        interrupt,
        "control_basis_output_allocate",
    )?;
    for row in 0..rows {
        for column in 0..columns {
            checkpoint_chunk(interrupt, row * columns + column, "control_basis_output")?;
            output[column][row] = matrix[row * columns + column];
        }
    }
    Ok(output)
}

fn index_vector(
    length: usize,
    interrupt: &mut dyn InterruptCheck,
    phase: &'static str,
) -> Result<Vec<usize>> {
    let mut output = Vec::new();
    reserve_exact(&mut output, length, "control row order")?;
    for index in 0..length {
        checkpoint_chunk(interrupt, index, phase)?;
        output.push(index);
    }
    Ok(output)
}

fn copy_usize_with_interrupt(
    values: &[usize],
    label: &str,
    interrupt: &mut dyn InterruptCheck,
    phase: &'static str,
) -> Result<Vec<usize>> {
    let mut output = Vec::new();
    reserve_exact(&mut output, values.len(), label)?;
    for (index, &value) in values.iter().enumerate() {
        checkpoint_chunk(interrupt, index, phase)?;
        output.push(value);
    }
    Ok(output)
}

fn copy_f64_local(
    values: &[f64],
    label: &str,
    interrupt: &mut dyn InterruptCheck,
    phase: &'static str,
) -> Result<Vec<f64>> {
    let mut output = Vec::new();
    reserve_exact(&mut output, values.len(), label)?;
    for (index, &value) in values.iter().enumerate() {
        checkpoint_chunk(interrupt, index, phase)?;
        output.push(value);
    }
    Ok(output)
}

fn zeroed_bool_with_interrupt(
    length: usize,
    label: &str,
    interrupt: &mut dyn InterruptCheck,
    phase: &'static str,
) -> Result<Vec<bool>> {
    let mut output = Vec::new();
    reserve_exact(&mut output, length, label)?;
    for index in 0..length {
        checkpoint_chunk(interrupt, index, phase)?;
        output.push(false);
    }
    Ok(output)
}

fn zeroed_usize_with_interrupt(
    length: usize,
    label: &str,
    interrupt: &mut dyn InterruptCheck,
    phase: &'static str,
) -> Result<Vec<usize>> {
    let mut output = Vec::new();
    reserve_exact(&mut output, length, label)?;
    for index in 0..length {
        checkpoint_chunk(interrupt, index, phase)?;
        output.push(0);
    }
    Ok(output)
}

fn zeroed_matrix_columns(
    columns: usize,
    rows: usize,
    label: &str,
    interrupt: &mut dyn InterruptCheck,
    phase: &'static str,
) -> Result<Vec<Vec<f64>>> {
    checked_matrix_length(rows, columns, label)?;
    let mut output = Vec::new();
    reserve_exact(&mut output, columns, label)?;
    for column in 0..columns {
        checkpoint_chunk(interrupt, column, phase)?;
        output.push(zeroed_f64_with_interrupt(rows, label, interrupt, phase)?);
    }
    Ok(output)
}

fn certified_anchor(
    score: &[f64],
    margin: f64,
    uncertainty: f64,
    interrupt: &mut dyn InterruptCheck,
) -> Result<usize> {
    let mut maximum = f64::NEG_INFINITY;
    for (row, &candidate) in score.iter().enumerate() {
        checkpoint_chunk(interrupt, row, "control_basis_anchor_maximum")?;
        maximum = maximum.max(candidate);
    }
    if !maximum.is_finite() || maximum <= margin {
        return Err(BackendError::new(
            ErrorCode::SingularInformation,
            "control_basis",
            "control-span anchor selection lost rank",
        ));
    }
    let cutoff = maximum - margin * maximum.max(1.0);
    let mut chosen = None;
    for (row, &candidate) in score.iter().enumerate() {
        checkpoint_chunk(interrupt, row, "control_basis_anchor_ambiguity")?;
        if (candidate - cutoff).abs() <= uncertainty {
            return Err(ambiguous(
                "control-span anchor score is numerically ambiguous at the canonical tie boundary",
            ));
        }
        // Only this row and its predecessors determine the first eligible
        // anchor. The maximum above still includes every row.
        if candidate > cutoff {
            chosen = Some(row);
            break;
        }
    }
    chosen.ok_or_else(|| {
        BackendError::new(
            ErrorCode::SingularInformation,
            "control_basis",
            "control-span anchor selection failed",
        )
    })
}

fn score_uncertainty(numerical_error: f64, controls: f64) -> Result<f64> {
    let score_error =
        2.0 * numerical_error + rounding_gamma(2.0 * controls)? * (1.0 + numerical_error).powi(2);
    Ok((1.0e-12_f64).max(bound_up(4.0 * score_error)))
}

fn rounding_gamma(operations: f64) -> Result<f64> {
    let product = operations * f64::EPSILON;
    if !product.is_finite() || product >= 0.5 {
        return Err(ambiguous("canonical-control rounding bound is not finite"));
    }
    Ok(bound_up(product / (1.0 - product)))
}

fn ambiguous(message: &'static str) -> BackendError {
    BackendError::new(ErrorCode::AmbiguousControlBasis, "control_basis", message)
}

fn resource(message: &'static str) -> BackendError {
    BackendError::new(ErrorCode::ResourceLimit, "control_basis", message)
}

fn preserve_break_or(error: BackendError, code: ErrorCode, message: &'static str) -> BackendError {
    if error.code == ErrorCode::UserBreak {
        error
    } else {
        BackendError::new(code, "control_basis", message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selection_certifies_only_the_decisive_prefix_but_uses_the_global_maximum() {
        let choose =
            |scores: &[f64], error| certified_anchor(scores, 0.1, error, &mut NeverInterrupt);
        assert_eq!(choose(&[1.0, 0.9], 1e-12).unwrap(), 0); // late exact boundary
        assert_eq!(choose(&[0.8, 1.0, 0.9], 1e-12).unwrap(), 1);
        assert_eq!(
            choose(&[0.9, 1.0], 1e-12).unwrap_err().code,
            ErrorCode::AmbiguousControlBasis
        );
        assert_eq!(
            choose(&[0.9000000000001, 1.0], 1e-12).unwrap_err().code,
            ErrorCode::AmbiguousControlBasis
        );
        assert_eq!(choose(&[0.5, 1.0, 2.0], 1e-12).unwrap(), 2); // late maximum changes cutoff
        assert_eq!(
            choose(&[0.9001, 1.0], 0.001).unwrap_err().code,
            ErrorCode::AmbiguousControlBasis
        );
    }

    #[test]
    fn compensated_products_enclose_independent_decimal_oracles() {
        let fixtures = include_str!("../../../../fevc/tests/fixtures/control_crossproducts.txt");
        for line in fixtures.lines().filter(|line| !line.starts_with('#')) {
            let parts: Vec<_> = line.split(';').collect();
            let parse = |text: &str| {
                text.split(',')
                    .map(|x| x.parse::<f64>().unwrap())
                    .collect::<Vec<_>>()
            };
            let left = parse(parts[1]);
            let right = parse(parts[2]);
            let frequency: Vec<_> = parts[3]
                .split(',')
                .map(|x| x.parse::<u64>().unwrap())
                .collect();
            let product = certified_weighted_product(
                &left,
                &right,
                left.len(),
                1,
                &frequency,
                &mut NeverInterrupt,
            );
            if parts[4] == "FAIL" {
                assert!(product.is_err(), "{}", parts[0]);
                continue;
            }
            let product = product.unwrap();
            let lo: f64 = parts[4].parse().unwrap();
            let hi: f64 = parts[5].parse().unwrap();
            assert!(
                product.values[0] - product.errors[0] <= lo
                    && product.values[0] + product.errors[0] >= hi,
                "{}: value={} error={} [{},{}]",
                parts[0],
                product.values[0],
                product.errors[0],
                lo,
                hi
            );
        }
        let product = certified_weighted_product(
            &[1., 1., 1.],
            &[1e16, 1., -1e16],
            3,
            1,
            &[1, 1, 1],
            &mut NeverInterrupt,
        )
        .unwrap();
        assert_eq!(product.values[0], 1.0);
        assert!(
            certified_weighted_product(&[1.], &[1.], 1, 1, &[u64::MAX], &mut NeverInterrupt)
                .is_err()
        );
    }

    #[test]
    fn scores_anchors_and_span_enclose_independent_high_precision_oracle() {
        let fixture = include_str!("../../../../fevc/tests/fixtures/control_scores.txt");
        let anchors: Vec<usize> = fixture
            .lines()
            .next()
            .unwrap()
            .trim_start_matches("#anchors=")
            .split(',')
            .map(|x| x.parse::<usize>().unwrap() - 1)
            .collect();
        let oracle: Vec<Vec<f64>> = fixture
            .lines()
            .filter(|x| !x.starts_with('#'))
            .map(|x| x.split(',').map(|x| x.parse().unwrap()).collect())
            .collect();
        let controls: Vec<Vec<f64>> = (0..2)
            .map(|j| oracle.iter().map(|x| x[j]).collect())
            .collect();
        let frequency: Vec<u64> = oracle.iter().map(|x| x[2] as u64).collect();
        let basis = canonicalize_controls(&controls, &frequency, 1e-10).unwrap();
        let original: Vec<f64> = oracle.iter().flat_map(|x| [x[0], x[1]]).collect();
        let mut interrupt = NeverInterrupt;
        let gram = certified_weighted_product(
            &original,
            &original,
            oracle.len(),
            2,
            &frequency,
            &mut interrupt,
        )
        .unwrap();
        let inverse = invert_scaled_spd(&gram.values, 2, 1e-10, &mut interrupt, "oracle").unwrap();
        let whitener = cholesky_factor(&inverse.inverse, 2, &mut interrupt, "oracle").unwrap();
        let u = multiply(&original, oracle.len(), 2, &whitener, 2, &mut interrupt).unwrap();
        let mut residualized = u.clone();
        let uncertainty = score_uncertainty(basis.receipt.forward_error, 2.).unwrap();
        for (pivot, &chosen) in anchors.iter().enumerate() {
            let scores = row_norm_squares(&residualized, oracle.len(), 2, &mut interrupt).unwrap();
            assert_eq!(
                certified_anchor(&scores, 1e-7, uncertainty, &mut interrupt).unwrap(),
                chosen
            );
            for (i, &score) in scores.iter().enumerate() {
                assert!(
                    score - uncertainty <= oracle[i][3 + 2 * pivot]
                        && score + uncertainty >= oracle[i][4 + 2 * pivot]
                );
            }
            let anchor = select_rows(&u, 2, &[chosen], &mut interrupt).unwrap();
            let gram = crossproduct(&anchor, 1, 2, &mut interrupt).unwrap();
            let inverse = invert_scaled_spd(&gram, 1, 1e-10, &mut interrupt, "oracle").unwrap();
            let projector =
                anchor_projector(&anchor, 1, 2, &inverse.inverse, &mut interrupt).unwrap();
            let projected = multiply(&u, oracle.len(), 2, &projector, 2, &mut interrupt).unwrap();
            residualized = u.iter().zip(projected).map(|(x, y)| x - y).collect();
        }
        for (i, row) in oracle.iter().enumerate() {
            for j in 0..2 {
                let error = basis.receipt.forward_error * (1.0 + basis.columns[j][i].abs());
                assert!(
                    basis.columns[j][i] - error <= row[7 + 2 * j]
                        && basis.columns[j][i] + error >= row[8 + 2 * j]
                );
            }
        }
    }

    #[test]
    fn adversarial_control_gram_preserves_inverse_failure_detail() {
        let x = [
            0.7037651560862372,
            -0.2552775470643382,
            -0.112896606015219,
            -0.26212880006292877,
            -0.08287286955710015,
            -0.0693863854324555,
            0.08296261314678174,
            -0.5826882952311562,
        ];
        let y = [
            -0.01213100064390886,
            0.6559466556462534,
            0.28729193595054625,
            -0.15584457130166096,
            -0.1733117487431483,
            -0.0772968683597004,
            -0.5616086968926074,
            -0.33368629034981906,
        ];
        let mut controls = vec![vec![0.; 14]; 2];
        for i in 0..8 {
            controls[0][i] = 8000. * x[i];
            controls[1][i] = x[i] + 0.000125 * y[i];
        }
        let error = canonicalize_controls(
            &controls,
            &[1, 1, 1, 1, 1, 1, 1, 1, 8, 8, 11, 14, 2, 3],
            1e-10,
        )
        .unwrap_err();
        assert_eq!(error.code, ErrorCode::InverseResidualFailed);
        assert_eq!(error.phase, "control_basis_gram");
        assert!(error.message.contains("scaled="));
        assert!(error.message.contains("original="));
        assert!(error.message.contains("gate="));
    }

    #[test]
    fn control_counts_through_32_match_exact_integer_arithmetic() {
        for q in 1..=32 {
            let n = 67;
            let left: Vec<_> = (0..n * q)
                .map(|i| ((i * 104729 % 67108859) as i64 - 33554429) as f64)
                .collect();
            let right: Vec<_> = (0..n * q)
                .map(|i| ((i * 130363 % 67108859) as i64 - 33554429) as f64)
                .collect();
            let weights: Vec<_> = (0..n).map(|i| 1 + (i % 17) as u64).collect();
            let product =
                certified_weighted_product(&left, &right, n, q, &weights, &mut NeverInterrupt)
                    .unwrap();
            for a in 0..q {
                for b in 0..q {
                    let exact: i128 = (0..n)
                        .map(|i| {
                            weights[i] as i128 * left[i * q + a] as i128 * right[i * q + b] as i128
                        })
                        .sum();
                    let rounded = exact as f64;
                    assert!(
                        (product.values[a * q + b] - rounded).abs() + rounded.abs() * f64::EPSILON
                            <= product.errors[a * q + b]
                    );
                }
            }
        }
    }

    struct BreakOnAnchor;

    impl InterruptCheck for BreakOnAnchor {
        fn checkpoint(&mut self, phase: &'static str) -> Result<()> {
            if phase == "control_basis_anchor" {
                Err(BackendError::new(
                    ErrorCode::UserBreak,
                    phase,
                    "injected anchor break",
                ))
            } else {
                Ok(())
            }
        }
    }

    struct BreakAfterPhase {
        target: &'static str,
        seen: usize,
    }

    impl InterruptCheck for BreakAfterPhase {
        fn checkpoint(&mut self, phase: &'static str) -> Result<()> {
            if phase == self.target {
                self.seen += 1;
                if self.seen == 2 {
                    return Err(BackendError::new(
                        ErrorCode::UserBreak,
                        phase,
                        "delayed control-basis break",
                    ));
                }
            }
            Ok(())
        }
    }

    #[test]
    fn canonical_basis_is_invariant_to_nonsingular_control_coordinates() {
        let first = vec![
            vec![1.0, 2.0, -1.0, 0.5, 3.0, -2.0],
            vec![-1.0, 0.5, 2.0, 3.0, -0.5, 1.0],
        ];
        let transformed = vec![
            first[0]
                .iter()
                .zip(&first[1])
                .map(|(&a, &b)| 1.0e4 * (2.0 * a - b))
                .collect(),
            first[0]
                .iter()
                .zip(&first[1])
                .map(|(&a, &b)| 1.0e-3 * (a + 3.0 * b))
                .collect(),
        ];
        let weights = vec![1, 2, 1, 3, 1, 2];
        let left = canonicalize_controls(&first, &weights, 1.0e-10).expect("first basis");
        let right =
            canonicalize_controls(&transformed, &weights, 1.0e-10).expect("transformed basis");
        for (left_column, right_column) in left.columns.iter().zip(&right.columns) {
            for (&a, &b) in left_column.iter().zip(right_column) {
                assert!((a - b).abs() < 1.0e-9, "{a} versus {b}");
            }
        }
    }

    #[test]
    fn singular_and_over_limit_controls_have_distinct_statuses() {
        let singular = vec![vec![1.0, 2.0, 3.0], vec![2.0, 4.0, 6.0]];
        let singular_error =
            canonicalize_controls(&singular, &[1, 1, 1], 1.0e-10).expect_err("singular controls");
        assert_eq!(singular_error.code, ErrorCode::SingularInformation);

        let over_limit = vec![vec![1.0, 2.0]; MAX_CANONICAL_CONTROLS + 1];
        let limit_error =
            canonicalize_controls(&over_limit, &[1, 1], 1.0e-10).expect_err("Q limit");
        assert_eq!(limit_error.code, ErrorCode::AmbiguousControlBasis);
    }

    #[test]
    fn canonical_anchor_preserves_injected_user_break() {
        let controls = vec![
            vec![1.0, 2.0, -1.0, 0.5, 3.0, -2.0],
            vec![-1.0, 0.5, 2.0, 3.0, -0.5, 1.0],
        ];
        let error = canonicalize_controls_with_interrupt(
            &controls,
            &[1, 2, 1, 3, 1, 2],
            1.0e-10,
            &mut BreakOnAnchor,
        )
        .expect_err("anchor must be interruptible");
        assert_eq!(error.code, ErrorCode::UserBreak);
    }

    #[test]
    fn ordered_control_product_retains_deep_cancellation_term() {
        let left = [1.0, 1.0, 1.0];
        let right = [1.0e16, 1.0, -1.0e16];
        let product =
            weighted_transpose_product(&left, &right, 3, 1, &[1, 1, 1], &mut NeverInterrupt)
                .expect("compensated ordered product");
        assert_eq!(product[0].to_bits(), 1.0_f64.to_bits());
    }

    #[test]
    fn singleton_semantic_classes_skip_the_unneeded_tie_kernel() {
        let singular_controls = vec![vec![0.0, 0.0, 0.0]];
        let order = [2, 0, 1];
        let refined = refine_control_semantic_order_with_interrupt(
            &singular_controls,
            &[1, 1, 1],
            &order,
            1.0e-10,
            |_left, _right| false,
            &mut NeverInterrupt,
        )
        .expect("singleton classes need no control-span tie certificate");
        assert_eq!(refined, order);
    }

    #[test]
    fn invariant_tie_collision_fails_closed_under_coordinate_change() {
        let controls = vec![vec![-1.0, 1.0, 2.0]];
        let transformed = vec![vec![-7.0, 7.0, 14.0]];
        let order = [0, 1, 2];
        let left = refine_control_semantic_order_with_interrupt(
            &controls,
            &[1, 1, 1],
            &order,
            1.0e-10,
            |left, right| left < 2 && right < 2,
            &mut NeverInterrupt,
        )
        .expect_err("symmetric distinct tied rows are ambiguous");
        let right = refine_control_semantic_order_with_interrupt(
            &transformed,
            &[1, 1, 1],
            &order,
            1.0e-10,
            |left, right| left < 2 && right < 2,
            &mut NeverInterrupt,
        )
        .expect_err("coordinate change preserves ambiguous status");
        assert_eq!(left.code, ErrorCode::AmbiguousControlBasis);
        assert_eq!(right.code, ErrorCode::AmbiguousControlBasis);
    }

    #[test]
    fn first_uncertain_signature_coordinate_cannot_be_rescued_by_later_coordinates() {
        const WIDTH: usize = 8;
        fn status(mut signatures: [[f64; WIDTH]; 3]) -> ErrorCode {
            signatures.sort_by(|left, right| {
                left.iter()
                    .zip(right)
                    .map(|(&a, &b)| a.total_cmp(&b))
                    .find(|ordering| !ordering.is_eq())
                    .unwrap_or(core::cmp::Ordering::Equal)
            });
            for pair in signatures.windows(2) {
                if let Err(error) =
                    certify_lexicographic_signature_pair(&pair[0], &pair[1], 1.0e-12, false)
                {
                    return error.code;
                }
            }
            panic!("the deliberately uncertain first coordinate must reject");
        }

        let original = [
            [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            [1.0, 0.5, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            [3.0, 200.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        ];
        let reversed = [original[2], original[1], original[0]];
        // A nonsingular coordinate transformation leaves the exact kernel
        // invariant but can perturb its computed coordinates within the
        // certified forward bound. Later coordinates remain deliberately far.
        let transformed_rounding = [
            [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            [1.0 + 5.0e-13, 0.5, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            [4.0, 250.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        ];
        let transformed_reversed = [
            transformed_rounding[2],
            transformed_rounding[1],
            transformed_rounding[0],
        ];
        assert_eq!(status(original), ErrorCode::AmbiguousControlBasis);
        assert_eq!(status(reversed), ErrorCode::AmbiguousControlBasis);
        assert_eq!(
            status(transformed_rounding),
            ErrorCode::AmbiguousControlBasis
        );
        assert_eq!(
            status(transformed_reversed),
            ErrorCode::AmbiguousControlBasis
        );
    }

    #[test]
    fn ordered_control_product_is_interruptible_beyond_the_first_chunk() {
        let rows = 4_100;
        let left = vec![1.0; rows];
        let right = vec![2.0; rows];
        let weights = vec![1; rows];
        let mut interrupt = BreakAfterPhase {
            target: "control_basis_crossproduct",
            seen: 0,
        };
        let error = weighted_transpose_product(&left, &right, rows, 1, &weights, &mut interrupt)
            .expect_err("second in-phase checkpoint must be reachable");
        assert_eq!(interrupt.seen, 2);
        assert_eq!(error.code, ErrorCode::UserBreak);
        assert_eq!(error.phase, "control_basis_crossproduct");
    }
}
