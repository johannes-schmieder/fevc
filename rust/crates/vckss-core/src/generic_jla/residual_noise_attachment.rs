// SPDX-License-Identifier: MIT

//! Leave-out residual standard errors on the live generic-JLA solver.
//!
//! The pass repeats the influence construction of the structured attachment
//! for Gaussian pseudo-outcomes: one solve gives each pseudo-fit and three more
//! give its kernel images. It needs no spectral pass and no residual Gram.
use super::*;
use crate::component_inference::residual_noise_result;
use crate::residual_noise::{
    center_units, finish_covariance, fit_working_variance, ControlAdjustment, Diagnostic,
    NoiseAccumulator,
};

/// Unit-level inputs for the first-order control-estimation term.
///
/// `plan` lists the stored rows of every inferential unit. Mover units weight
/// a stored row by its frequency; a physical stayer copy has weight one.
#[allow(clippy::too_many_arguments)]
pub(super) fn control_adjustment(
    problem: &CompressedProblem,
    plan: &MatchPlan,
    mover_units: usize,
    controls: &[Vec<f64>],
    geometry: &ControlGeometry,
    weights: &[f64],
    row_order: &[usize],
    working_y: &[f64],
    options: GenericJlaOptions,
    interrupt: &mut dyn InterruptCheck,
) -> Result<ControlAdjustment> {
    let q = controls.len();
    let rows = problem.outcome.len();
    let units = plan.rows.len();
    if q == 0
        || geometry.controls != q
        || geometry.residualized.len() != checked_matrix_length(rows, q, "residualized controls")?
        || working_y.len() != rows
        || mover_units > units
    {
        return Err(BackendError::invariant(
            "residual_noise_control",
            "control-estimation inputs have inconsistent dimensions",
        ));
    }
    let length = checked_matrix_length(units, q, "unit control aggregates")?;
    let mut collapsed = zeroed_f64_with_interrupt(
        length,
        "collapsed controls",
        interrupt,
        "residual_noise_control_allocate",
    )?;
    let mut residualized = zeroed_f64_with_interrupt(
        length,
        "collapsed residualized controls",
        interrupt,
        "residual_noise_control_allocate",
    )?;
    let mut within = zeroed_f64_with_interrupt(
        length,
        "within-unit control products",
        interrupt,
        "residual_noise_control_allocate",
    )?;
    for unit in 0..units {
        checkpoint_chunk(interrupt, unit, "residual_noise_control_units")?;
        let weight = |row: usize| {
            if unit < mover_units {
                problem.frequency[row] as f64
            } else {
                1.0
            }
        };
        let mass = plan.physical_count[unit] as f64;
        let mut outcome = StableAccumulator::default();
        for &row in &plan.rows[unit] {
            outcome.add(weight(row) * working_y[row]);
        }
        let outcome_mean = outcome.finish() / mass;
        for control in 0..q {
            let mut raw = StableAccumulator::default();
            let mut partial = StableAccumulator::default();
            for &row in &plan.rows[unit] {
                raw.add(weight(row) * controls[control][row]);
                partial.add(weight(row) * geometry.residualized[control * rows + row]);
            }
            let raw = raw.finish();
            let control_mean = raw / mass;
            let mut product = StableAccumulator::default();
            for &row in &plan.rows[unit] {
                product.add(
                    weight(row)
                        * (controls[control][row] - control_mean)
                        * (working_y[row] - outcome_mean),
                );
            }
            let index = control * units + unit;
            collapsed[index] = raw / mass.sqrt();
            residualized[index] = partial.finish() / mass.sqrt();
            within[index] = product.finish();
            if !collapsed[index].is_finite()
                || !residualized[index].is_finite()
                || !within[index].is_finite()
            {
                return Err(nonfinite("a unit control aggregate is nonfinite"));
            }
        }
    }
    let information = weighted_crossproduct_flat(
        &geometry.residualized,
        q,
        rows,
        weights,
        row_order,
        interrupt,
        "residual_noise_control_information",
    )?;
    let precision = invert_scaled_spd(
        &information,
        q,
        options.rank_tolerance,
        interrupt,
        "residual_noise_control_information",
    )?
    .inverse;
    Ok(ControlAdjustment {
        controls: q,
        collapsed,
        residualized,
        within,
        precision,
    })
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
pub(super) fn run(
    problem: &CompressedProblem,
    prepared: &ComponentExecution<'_>,
    solver: &PreparedModelSolver<'_>,
    coefficients: &ModelCoefficients,
    inference_rows: ComponentInferenceRows<'_>,
    working_y: &[f64],
    centered: bool,
    residual: &[f64],
    deleted_adjusted: &[f64],
    leverage: &[f64],
    maker_inverse: &[f64],
    target_diagonal: &[Vec<f64>; PRIMITIVE_TARGETS],
    addresses: &ComponentInferenceAddresses,
    match_mass: Option<&[f64]>,
    control: Option<&ControlAdjustment>,
    point_correction: VarianceComponents,
    peak_forecast_bytes: u64,
    counter_plan: ComponentInferenceCounterPlan,
    interrupt: &mut dyn InterruptCheck,
) -> Result<ComponentInferenceResult> {
    let rows = inference_rows.len(problem);
    let controls = inference_rows.control_columns();
    let unit_order = match inference_rows {
        ComponentInferenceRows::Observation { row_order, .. } => row_order.to_vec(),
        ComponentInferenceRows::Match { .. } => (0..rows).collect(),
    };
    let mover_units = prepared.mixed_units.map_or(rows, |(movers, _)| movers);
    if mover_units > rows {
        return Err(BackendError::invariant(
            "residual_noise_attachment",
            "mixed unit counts exceed the inference rows",
        ));
    }
    if prepared.inference_unit == ComponentInferenceUnit::Match {
        // The squared-residual weight and the leave-out product must describe
        // the same scalar maker for every collapsed unit.
        for row in 0..rows {
            let reconstructed = maker_inverse[row] * residual[row];
            let scale = reconstructed
                .abs()
                .max(deleted_adjusted[row].abs())
                .max(1.0);
            if !reconstructed.is_finite()
                || (reconstructed - deleted_adjusted[row]).abs() > 1.0e-10 * scale
            {
                return Err(BackendError::new(
                    ErrorCode::TargetIdentityFailed,
                    "residual_noise_maker_identity",
                    "the scalar maker does not reproduce the adjusted match residual",
                ));
            }
        }
    }
    let ratios = target_ratios(target_diagonal, maker_inverse)?;
    let expected_correction = [
        point_correction.worker,
        point_correction.firm,
        point_correction.covariance,
    ];
    let mut identity_error = 0.0_f64;
    let mut proxy = zeroed_f64_with_interrupt(
        rows,
        "leave-out variance products",
        interrupt,
        "residual_noise_proxy",
    )?;
    for (position, &row) in unit_order.iter().enumerate() {
        checkpoint_chunk(interrupt, position, "residual_noise_proxy")?;
        proxy[row] = working_y[row] * deleted_adjusted[row];
    }
    for target in 0..PRIMITIVE_TARGETS {
        let mut value = StableAccumulator::default();
        for (position, &row) in unit_order.iter().enumerate() {
            checkpoint_chunk(interrupt, position, "residual_noise_point_identity")?;
            value.add(proxy[row] * target_diagonal[target][row]);
        }
        let value = value.finish();
        let scale = value.abs().max(expected_correction[target].abs()).max(1.0);
        let error = (value - expected_correction[target]).abs();
        identity_error = identity_error.max(error);
        if !error.is_finite() || error > 1.0e-10 * scale {
            return Err(BackendError::new(
                ErrorCode::TargetIdentityFailed,
                "residual_noise_point_identity",
                "retained target diagonals do not reproduce the unchanged JLA point correction",
            ));
        }
    }
    let fit = fit_working_variance(
        &proxy,
        residual,
        maker_inverse,
        leverage,
        match_mass,
        mover_units,
        prepared.structured_options.positivity_multiplier,
        interrupt,
    )?;
    drop(proxy);
    let variance = &fit.variance;

    let mut solve_receipts = Vec::new();
    let draws = prepared.options.probes as usize;
    reserve_exact(
        &mut solve_receipts,
        draws
            .checked_mul(1 + PRIMITIVE_TARGETS)
            .and_then(|value| value.checked_add(PRIMITIVE_TARGETS))
            .ok_or_else(|| resource("residual-noise receipt count overflow"))?,
        "residual-noise solve receipts",
    )?;

    // Kernel image of one unit-space vector, given the fit of that vector.
    let kernel_rhs = |outcome: &[f64],
                      fit: &ModelCoefficients,
                      worker: &mut [f64],
                      firm: &mut [f64],
                      control_block: &mut [f64],
                      interrupt: &mut dyn InterruptCheck|
     -> Result<()> {
        for target in 0..PRIMITIVE_TARGETS {
            let (target_worker, target_firm, target_control) =
                primitive_target_rhs(problem, fit, target, interrupt)?;
            let score = component_transpose_rhs_by(
                problem,
                inference_rows,
                |row| 0.5 * ratios[target][row] * outcome[row],
                interrupt,
            )?;
            for index in 0..problem.workers() {
                worker[target * problem.workers() + index] = target_worker[index] + score.0[index];
            }
            for index in 0..problem.firms() {
                firm[target * problem.firms() + index] = target_firm[index] + score.1[index];
            }
            for index in 0..controls.len() {
                control_block[target * controls.len() + index] =
                    target_control[index] + score.2[index];
            }
        }
        Ok(())
    };

    let columns = PRIMITIVE_TARGETS;
    let mut worker_rhs = vec![0.0; problem.workers() * columns];
    let mut firm_rhs = vec![0.0; problem.firms() * columns];
    let mut control_rhs = vec![0.0; controls.len() * columns];
    kernel_rhs(
        working_y,
        coefficients,
        &mut worker_rhs,
        &mut firm_rhs,
        &mut control_rhs,
        interrupt,
    )?;
    let solved = solver.solve_batch_with_interrupt(
        &worker_rhs,
        &firm_rhs,
        &control_rhs,
        columns,
        columns,
        interrupt,
    )?;
    let mut prediction = vec![0.0; rows];
    let mut influence = core::array::from_fn(|_| Vec::new());
    for target in 0..PRIMITIVE_TARGETS {
        let solution = &solved.solution[target];
        component_predict(
            problem,
            inference_rows,
            solver,
            &solution.coefficients,
            &mut prediction,
            interrupt,
        )?;
        influence[target] = finish_influence(&prediction, &ratios[target], working_y, residual)?;
        if centered {
            center_units(&mut influence[target], match_mass, interrupt)?;
        }
        solve_receipts.push(component_solve_receipt(
            ComponentInferenceSolvePhase::Influence,
            target as u32,
            solution,
        ));
    }
    drop(solved);

    let rng = CounterRng::new(prepared.options.seed);
    let mut accumulator = NoiseAccumulator::default();
    let batch_width = prepared.options.batch_width.clamp(1, draws);
    let mut pseudo_residual = vec![0.0; rows];
    for first in (0..draws).step_by(batch_width) {
        crate::progress::advance(crate::progress::INFERENCE, first, draws);
        interrupt.checkpoint("residual_noise_batch")?;
        let width = batch_width.min(draws - first);
        let mut pseudo_outcome = vec![0.0; rows * width];
        let mut pseudo_worker = vec![0.0; problem.workers() * width];
        let mut pseudo_firm = vec![0.0; problem.firms() * width];
        let mut pseudo_control = vec![0.0; controls.len() * width];
        for local in 0..width {
            let outcome = &mut pseudo_outcome[local * rows..(local + 1) * rows];
            fill_gaussian_pseudo_outcome(
                rng,
                (first + local) as u64,
                &addresses.entity,
                &addresses.subdraw,
                variance,
                outcome,
                interrupt,
            )?;
            // The reported estimate centers the outcome at its retained mean,
            // so pure noise must pass through the same centering.
            if centered {
                center_units(outcome, match_mass, interrupt)?;
            }
            let rhs = component_transpose_rhs(problem, inference_rows, outcome, interrupt)?;
            pseudo_worker[local * problem.workers()..(local + 1) * problem.workers()]
                .copy_from_slice(&rhs.0);
            pseudo_firm[local * problem.firms()..(local + 1) * problem.firms()]
                .copy_from_slice(&rhs.1);
            if !controls.is_empty() {
                pseudo_control[local * controls.len()..(local + 1) * controls.len()]
                    .copy_from_slice(&rhs.2);
            }
        }
        let fitted = solver.solve_batch_with_interrupt(
            &pseudo_worker,
            &pseudo_firm,
            &pseudo_control,
            width,
            width,
            interrupt,
        )?;
        drop((pseudo_worker, pseudo_firm, pseudo_control));
        let kernel_columns = width * PRIMITIVE_TARGETS;
        let mut kernel_worker = vec![0.0; problem.workers() * kernel_columns];
        let mut kernel_firm = vec![0.0; problem.firms() * kernel_columns];
        let mut kernel_control = vec![0.0; controls.len() * kernel_columns];
        for local in 0..width {
            let outcome = &pseudo_outcome[local * rows..(local + 1) * rows];
            let base = local * PRIMITIVE_TARGETS;
            kernel_rhs(
                outcome,
                &fitted.solution[local].coefficients,
                &mut kernel_worker[base * problem.workers()
                    ..(base + PRIMITIVE_TARGETS) * problem.workers()],
                &mut kernel_firm[base * problem.firms()..(base + PRIMITIVE_TARGETS) * problem.firms()],
                &mut kernel_control
                    [base * controls.len()..(base + PRIMITIVE_TARGETS) * controls.len()],
                interrupt,
            )?;
        }
        let images = solver.solve_batch_with_interrupt(
            &kernel_worker,
            &kernel_firm,
            &kernel_control,
            kernel_columns,
            width,
            interrupt,
        )?;
        drop((kernel_worker, kernel_firm, kernel_control));
        for local in 0..width {
            let outcome = &pseudo_outcome[local * rows..(local + 1) * rows];
            let solution = &fitted.solution[local];
            component_predict(
                problem,
                inference_rows,
                solver,
                &solution.coefficients,
                &mut prediction,
                interrupt,
            )?;
            for row in 0..rows {
                pseudo_residual[row] = outcome[row] - prediction[row];
            }
            solve_receipts.push(component_solve_receipt(
                ComponentInferenceSolvePhase::CovarianceProbe,
                (first + local) as u32,
                solution,
            ));
            let mut kernel = core::array::from_fn(|_| Vec::new());
            for target in 0..PRIMITIVE_TARGETS {
                let image = &images.solution[local * PRIMITIVE_TARGETS + target];
                component_predict(
                    problem,
                    inference_rows,
                    solver,
                    &image.coefficients,
                    &mut prediction,
                    interrupt,
                )?;
                kernel[target] =
                    finish_influence(&prediction, &ratios[target], outcome, &pseudo_residual)?;
                if centered {
                    center_units(&mut kernel[target], match_mass, interrupt)?;
                }
                solve_receipts.push(component_solve_receipt(
                    ComponentInferenceSolvePhase::Influence,
                    target as u32,
                    image,
                ));
            }
            accumulator.push(variance, maker_inverse, &pseudo_residual, kernel, interrupt)?;
        }
    }
    crate::progress::advance(crate::progress::INFERENCE, draws, draws);
    let moments = accumulator.finish()?;
    let covariance = finish_covariance(
        &influence,
        residual,
        maker_inverse,
        variance,
        control,
        &moments,
        interrupt,
    )?;
    let diagnostic = Diagnostic::new(&fit, control.map_or(0, |value| value.controls), &moments, &covariance)?;
    let mut result = residual_noise_result(rows, &covariance, &moments, diagnostic)?;

    let mut maximum_iterations = 0_u32;
    let mut maximum_reduced_residual = 0.0_f64;
    let mut maximum_complete_residual = 0.0_f64;
    for receipt in &solve_receipts {
        maximum_iterations = maximum_iterations.max(receipt.iterations);
        maximum_reduced_residual = maximum_reduced_residual.max(receipt.reduced_residual);
        maximum_complete_residual = maximum_complete_residual.max(receipt.complete_residual);
    }
    result.solve_receipts = solve_receipts;
    result.maximum_iterations = maximum_iterations;
    result.maximum_reduced_residual = maximum_reduced_residual;
    result.maximum_complete_residual = maximum_complete_residual;
    result.full_residual_tolerance = solver.options().full_residual_tolerance();
    result.peak_forecast_bytes = peak_forecast_bytes;
    result.leverage = leverage.to_vec();
    result.maker_inverse = maker_inverse.to_vec();
    result.target_diagonal = target_diagonal.clone();
    result.influence = influence;
    result.point_correction_identity_error = identity_error;
    result.counter_atoms = counter_plan.logical_atoms;
    result.counter_words = counter_plan.unique_words;
    result.inference_unit = prepared.inference_unit;
    // The control term accounts for the estimated offset to first order.
    result.nuisance_uncertainty_conditioned_away =
        prepared.inference_unit == ComponentInferenceUnit::Match && control.is_none();
    result.mixed_units = prepared
        .mixed_units
        .map(|(movers, stayers)| (movers as u64, stayers as u64));
    if let Some(match_mass) = match_mass {
        let match_mass = &match_mass[..mover_units];
        let total = match_mass.iter().sum::<f64>();
        let square = match_mass.iter().map(|value| value * value).sum::<f64>();
        if !total.is_finite() || total <= 0.0 || !square.is_finite() || square <= 0.0 {
            return Err(nonfinite("collapsed-match mass diagnostics are invalid"));
        }
        result.effective_match_count = total * total / square;
        result.largest_match_mass_share =
            match_mass.iter().copied().fold(0.0_f64, f64::max) / total;
        result.largest_match_leverage = leverage[..mover_units]
            .iter()
            .copied()
            .fold(0.0_f64, f64::max);
        result.smallest_maker_denominator = maker_inverse[..mover_units]
            .iter()
            .map(|value| value.recip())
            .fold(f64::INFINITY, f64::min);
    }
    result.variance_source = prepared.variance_source;
    Ok(result)
}
