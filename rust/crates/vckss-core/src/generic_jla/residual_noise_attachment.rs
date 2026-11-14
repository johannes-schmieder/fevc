// SPDX-License-Identifier: MIT

//! Leave-out residual standard errors on the live generic-JLA solver.
//!
//! The pass repeats the influence construction of the structured attachment
//! for Gaussian pseudo-outcomes: one solve gives each pseudo-fit and three more
//! give its kernel images. It needs no spectral pass and no residual Gram.
use super::*;
use crate::component_inference::residual_noise_result;
use crate::residual_noise::{
    center_units, finish_covariance, finish_repeat_covariance, finish_repeat_q1_target,
    fit_repeat_variance, fit_working_variance, ControlAdjustment, Diagnostic, NoiseAccumulator,
    RepeatControl, RepeatInputs, RepeatLeading, RepeatQ1Control, RepeatVariance,
};

/// Leading directions of the four reported targets and what the `q = 1`
/// decomposition along them needs from the realized outcome.
struct LeadingState {
    direction: Vec<leading_direction::LeadingDirection>,
    /// Target diagonal net of the direction, over the maker diagonal.
    ratio: [Vec<f64>; REPORTED_TARGETS],
    /// Realized influence of the remainder.
    influence: [Vec<f64>; REPORTED_TARGETS],
    point_estimate: [f64; REPORTED_TARGETS],
    leading_score: [f64; REPORTED_TARGETS],
    leading_leave_out: [f64; REPORTED_TARGETS],
    direct_remainder: [f64; REPORTED_TARGETS],
    identity_tolerance: [f64; REPORTED_TARGETS],
    /// Sum and sum of squares over draws of `sum_a v_a (K_r zeta)_a^2`, or
    /// with controls of the bilinear form of the remainder image.
    trace: [(f64, f64); REPORTED_TARGETS],
    direction_solves: u32,
    kernel_solves: u32,
}

fn reported_diagonal(target_diagonal: &[Vec<f64>; PRIMITIVE_TARGETS], target: usize) -> Vec<f64> {
    if target < PRIMITIVE_TARGETS {
        return target_diagonal[target].clone();
    }
    (0..target_diagonal[0].len())
        .map(|row| {
            target_diagonal[0][row] + target_diagonal[1][row] + 2.0 * target_diagonal[2][row]
        })
        .collect()
}

/// Standard normals of the listed units for one pseudo-noise draw, from the
/// control stream: the unit's own address, a domain of their own, and the
/// Box-Muller construction of the unit draws.
fn control_normals(
    rng: CounterRng,
    probe: u64,
    addresses: &ComponentInferenceAddresses,
    units: &[usize],
    interrupt: &mut dyn InterruptCheck,
) -> Result<Vec<f64>> {
    let domain = crate::rng::ProbeDomain::ComponentInferenceControl;
    let mut output = Vec::new();
    reserve_exact(&mut output, units.len(), "control-term standard normals")?;
    for (position, &unit) in units.iter().enumerate() {
        checkpoint_chunk(interrupt, position, "repeat_measurement_control_normals")?;
        let first_word = addresses.subdraw[unit]
            .checked_mul(2)
            .ok_or_else(|| resource("control-term word index overflow"))?;
        let first = rng.word(domain, probe, addresses.entity[unit], first_word);
        let second = rng.word(domain, probe, addresses.entity[unit], first_word + 1);
        let u1 = (((first >> 11) as f64) + 0.5) * (1.0 / 9_007_199_254_740_992.0);
        let u2 = (((second >> 11) as f64) + 0.5) * (1.0 / 9_007_199_254_740_992.0);
        output.push((-2.0 * u1.ln()).sqrt() * (core::f64::consts::TAU * u2).cos());
    }
    Ok(output)
}

/// Right-hand sides of the remainder kernels of one unit-space vector:
/// `A_t beta + X'(ratio_t z / 2)` for the four reported targets.
#[allow(clippy::too_many_arguments)]
fn remainder_rhs(
    problem: &CompressedProblem,
    inference_rows: ComponentInferenceRows<'_>,
    state: &LeadingState,
    outcome: &[f64],
    fit: &ModelCoefficients,
    worker: &mut [f64],
    firm: &mut [f64],
    interrupt: &mut dyn InterruptCheck,
) -> Result<()> {
    for target in 0..REPORTED_TARGETS {
        let (target_worker, target_firm, _) = reported_target_rhs(problem, fit, target, interrupt)?;
        let score = component_transpose_rhs_by(
            problem,
            inference_rows,
            |row| 0.5 * state.ratio[target][row] * outcome[row],
            interrupt,
        )?;
        for index in 0..problem.workers() {
            worker[target * problem.workers() + index] = target_worker[index] + score.0[index];
        }
        for index in 0..problem.firms() {
            firm[target * problem.firms() + index] = target_firm[index] + score.1[index];
        }
    }
    Ok(())
}

/// Unit-level inputs for the first-order control-estimation term.
///
/// `plan` lists the stored rows of every inferential unit. Mover units weight
/// a stored row by its frequency; a physical stayer copy has weight one.
/// Without a plan every stored row of unit frequency is a unit, and its
/// within-unit products vanish.
#[allow(clippy::too_many_arguments)]
pub(super) fn control_adjustment(
    problem: &CompressedProblem,
    plan: Option<&MatchPlan>,
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
    let units = plan.map_or(rows, |plan| plan.rows.len());
    if q == 0
        || geometry.controls != q
        || geometry.residualized.len() != checked_matrix_length(rows, q, "residualized controls")?
        || working_y.len() != rows
        || mover_units > units
        || (plan.is_none() && problem.frequency.iter().any(|&value| value != 1))
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
        let (unit_rows, mass) = match plan {
            Some(plan) => (plan.rows[unit].as_slice(), plan.physical_count[unit] as f64),
            None => (core::slice::from_ref(&unit), 1.0),
        };
        let mut outcome = StableAccumulator::default();
        for &row in unit_rows {
            outcome.add(weight(row) * working_y[row]);
        }
        let outcome_mean = outcome.finish() / mass;
        for control in 0..q {
            let mut raw = StableAccumulator::default();
            let mut partial = StableAccumulator::default();
            for &row in unit_rows {
                raw.add(weight(row) * controls[control][row]);
                partial.add(weight(row) * geometry.residualized[control * rows + row]);
            }
            let raw = raw.finish();
            let control_mean = raw / mass;
            let mut product = StableAccumulator::default();
            for &row in unit_rows {
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

/// Incremental heap of this attachment, tallied by allocation lifetime.
///
/// `U` is the number of inferential units, `N` the stored rows, `P` the
/// complete parameter count and `b` the draws solved together. Three groups
/// can coexist with the command peak:
///
/// * state the command keeps for the attachment: fit coefficients, unit
///   addresses, unit geometry, retained target diagonals, the collapsed unit
///   data of a grouped request, the control aggregates, and the retained
///   buffers of an offset pass;
/// * state `run` owns: unit order, target ratios, working variances, one
///   prediction, the influence vectors, one pseudo-residual, `b` pseudo-
///   outcomes, the kernel images of the current and the previous draw, and
///   the copies moved into the result;
/// * solve buffers: `b` pseudo-fits, `3b` kernel right-hand sides and `3b`
///   kernel solutions, two transient right-hand sides, and the solver's own
///   batch workspace for `b` columns.
///
/// The pass has no spectral phase and no residual Gram, so neither is charged.
pub(super) fn peak_bytes(
    problem: &CompressedProblem,
    prepared: &ComponentExecution<'_>,
    full_parameters: usize,
    route_memory: RouteMemory,
    offset_pass: bool,
    point_probes: u32,
) -> Result<u64> {
    let count = |value: usize, what: &'static str| {
        u64::try_from(value).map_err(|_| resource(format!("{what} is not representable")))
    };
    let units = count(
        prepared.unit_count(problem),
        "leave-out residual unit count",
    )?;
    let stored = count(problem.outcome.len(), "leave-out residual stored rows")?;
    let rows = units.max(stored);
    let workers = count(problem.workers(), "leave-out residual worker count")?;
    let firms = count(problem.firms(), "leave-out residual firm count")?;
    let controls = count(problem.controls.len(), "leave-out residual control count")?;
    let parameters = checked_sum(&[workers, firms, controls])?;
    let reduced = count(full_parameters, "leave-out residual reduced dimension")?;
    let draws = crate::component_inference::residual_noise_draws(prepared.options.probes)?;
    let batch = count(
        prepared.options.batch_width.clamp(1, draws),
        "leave-out residual batch width",
    )?;
    let grouped = prepared.inference_unit == ComponentInferenceUnit::Match;

    // Fit coefficients, two address words and two geometry values per unit,
    // and the three retained target diagonals on stored rows.
    let mut retained = checked_sum(&[
        checked_product(&[parameters, 8], "leave-out residual fit coefficients")?,
        checked_product(&[rows, 4, 8], "leave-out residual addresses and geometry")?,
        checked_product(&[stored, 3, 8], "leave-out residual target diagonals")?,
    ])?;
    if grouped {
        // Outcome, residual, adjusted residual, mass and three diagonals.
        retained = checked_sum(&[
            retained,
            checked_product(&[units, 7, 8], "collapsed unit data")?,
        ])?;
        if controls > 0 {
            // Collapsed, residualized and within-unit control aggregates,
            // their information matrix and its inverse with workspace.
            retained = checked_sum(&[
                retained,
                checked_product(&[units, controls, 3, 8], "unit control aggregates")?,
                checked_product(&[controls, controls, 4, 8], "control information")?,
            ])?;
        }
    } else if prepared.repeat_measurement && controls > 0 {
        // The same aggregates with one unit per observation.
        retained = checked_sum(&[
            retained,
            checked_product(&[units, controls, 3, 8], "unit control aggregates")?,
            checked_product(&[controls, controls, 4, 8], "control information")?,
        ])?;
    }
    if prepared.repeat_measurement && controls > 0 {
        // The control term of repeat-measurement standard errors: the
        // leave-out residuals, three starred influences with their
        // within-unit parts, one projected image at a time, `W'W`, and the
        // coefficient vectors of one draw.
        retained = checked_sum(&[
            retained,
            checked_product(&[units, 8, 8], "repeat-measurement control term")?,
            checked_product(
                &[controls, controls, 8],
                "repeat-measurement control moments",
            )?,
            checked_product(&[controls, 8, 8], "repeat-measurement control coefficients")?,
        ])?;
    }
    if prepared.repeat_measurement {
        // Unit cells and mobility, the two effects, unit mass, the set
        // ordering, and for the fitted units their index, leverage, mass, two
        // rank vectors and fitted scale; one firm count per worker.
        retained = checked_sum(&[
            retained,
            checked_product(&[units, 13, 8], "repeat-measurement unit state")?,
            checked_product(&[workers, 4], "repeat-measurement worker firm counts")?,
        ])?;
    }
    if prepared.mixed_units.is_some() {
        retained = checked_sum(&[
            retained,
            checked_product(
                &[problem.physical_total, 2, 8],
                "mixed physical maker capture",
            )?,
            checked_product(&[rows, 128], "mixed inference unit plan")?,
            checked_product(&[stored, 8], "mixed plan mover rows")?,
        ])?;
    }
    let offset_receipts = if offset_pass {
        // Offset outcome, worker-firm adjusted residuals and three target
        // diagonals on stored rows; unit leverage and maker inverse; the
        // worker-firm coefficients; and two solves per point-estimate probe.
        retained = checked_sum(&[
            retained,
            checked_product(&[stored, 5, 8], "offset-pass stored rows")?,
            checked_product(&[rows, 2, 8], "offset-pass unit geometry")?,
            checked_product(&[workers + firms, 8], "offset-pass coefficients")?,
        ])?;
        let solves = u64::from(point_probes)
            .checked_mul(2)
            .ok_or_else(|| resource("offset-pass solve count overflow"))?;
        retained = checked_sum(&[
            retained,
            checked_product(
                &[
                    solves,
                    count(
                        core::mem::size_of::<GenericJlaRhsReceipt>(),
                        "offset-pass receipt size",
                    )?,
                ],
                "offset-pass solve receipts",
            )?,
        ])?;
        solves
    } else {
        0
    };

    // `run`: order, three ratios, variances, prediction, three influence
    // vectors, pseudo-residual, two kernel triples and five result copies,
    // plus one pseudo-outcome per draw of the batch.
    let owned = checked_product(
        &[
            units,
            batch
                .checked_add(21)
                .ok_or_else(|| resource("leave-out residual unit state overflow"))?,
            8,
        ],
        "leave-out residual unit state",
    )?;
    // Pseudo-fits, kernel right-hand sides and kernel solutions, with two
    // transient right-hand sides while a kernel column is assembled.
    let exchanged = checked_product(
        &[
            parameters,
            batch
                .checked_mul(7)
                .and_then(|value| value.checked_add(2))
                .ok_or_else(|| resource("leave-out residual solve exchange overflow"))?,
            8,
        ],
        "leave-out residual solve exchange",
    )?;
    let solver = checked_sum(&[
        checked_product(&[reduced, batch, 8, 11], "leave-out residual PCG batch")?,
        checked_product(
            &[workers, batch, 8, 3],
            "leave-out residual worker workspace",
        )?,
        checked_product(&[batch, 8, 8], "leave-out residual scalar workspace")?,
        checked_product(&[batch, 96], "leave-out residual solver receipts")?,
        checked_product(
            &[route_memory.cmg_batch_workspace_per_column, batch],
            "leave-out residual CMG batch workspace",
        )?,
        route_memory.full_cmg.map_or(Ok(0), |setup| {
            // One pseudo-fit and three kernel columns per draw of the batch.
            direct::solve_bytes(
                workers,
                firms,
                controls,
                checked_product(&[batch, 4], "leave-out residual direct columns")?,
                setup,
            )
        })?,
    ])?;
    // Leading directions: four directions with their ratios and remainder
    // influences, the leave-out products and one image on the units; the
    // Lanczos basis with its work vectors and the four directions'
    // coefficients; four remainder right-hand sides and solutions per draw of
    // the batch; and one receipt per solve, the search counted at its cap.
    let (leading_bytes, leading_receipts) = if prepared.repeat_measurement
        && prepared.options.reference_distribution == ComponentReferenceDistribution::Q1
    {
        (
            checked_sum(&[
                checked_product(&[units, 14, 8], "leading-direction unit state")?,
                checked_product(&[parameters, 18, 8], "leading-direction coefficient state")?,
                checked_product(&[parameters, batch, 12, 8], "remainder kernel exchange")?,
            ])?,
            u64::from(prepared.options.spectrum_iterations)
                .checked_mul(REPORTED_TARGETS as u64)
                .and_then(|value| value.checked_add(REPORTED_TARGETS as u64))
                .and_then(|value| value.checked_add(u64::from(prepared.options.probes)))
                .ok_or_else(|| resource("leading-direction receipt count overflow"))?,
        )
    } else {
        (0, 0)
    };
    let receipts = checked_product(
        &[
            u64::from(prepared.options.probes)
                .checked_add(PRIMITIVE_TARGETS as u64)
                .and_then(|value| value.checked_add(offset_receipts))
                .and_then(|value| value.checked_add(leading_receipts))
                .ok_or_else(|| resource("leave-out residual receipt count overflow"))?,
            count(
                core::mem::size_of::<ComponentInferenceSolveReceipt>(),
                "leave-out residual receipt size",
            )?,
        ],
        "leave-out residual solve receipts",
    )?;
    checked_sum(&[
        prepared.persistent_bytes,
        retained,
        owned,
        exchanged,
        solver,
        receipts,
        leading_bytes,
        // The result crosses Rust and C once more on export.
        if prepared.individual_intervals {
            2 * (846 * 8 + 288)
        } else {
            0
        },
        2_048,
    ])
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
    // Either source ends in one positive variance per unit for the
    // pseudo-noise; they differ in how it is formed and assembled.
    let (fit, repeat) = if prepared.repeat_measurement {
        if !controls.is_empty() {
            // Units of one cell are repeat measurements only when the
            // controls are not part of the unit geometry; their estimation
            // enters through the first-order control term instead.
            return Err(BackendError::new(
                ErrorCode::UnsupportedFeature,
                "residual_noise_attachment",
                "repeat-measurement standard errors hold the offset fixed",
            ));
        }
        let mut set = Vec::new();
        let mut mover = Vec::new();
        let mut worker_effect = Vec::new();
        let mut firm_effect = Vec::new();
        reserve_exact(&mut set, rows, "repeat-measurement unit cells")?;
        reserve_exact(&mut mover, rows, "repeat-measurement unit mobility")?;
        reserve_exact(
            &mut worker_effect,
            rows,
            "repeat-measurement worker effects",
        )?;
        reserve_exact(&mut firm_effect, rows, "repeat-measurement firm effects")?;
        // Physical observations of one worker-firm cell have identical
        // regressors and form a set. A collapsed mover match is scaled by its
        // own mass, so it is always a unit of its own, whatever cell it is in.
        let mut unit_mass = Vec::new();
        let unit_cell = |unit: usize| -> usize {
            match inference_rows {
                ComponentInferenceRows::Match { plan, .. } => plan.cell[unit] as usize,
                ComponentInferenceRows::Observation { .. } => problem.row_cell[unit] as usize,
            }
        };
        match inference_rows {
            ComponentInferenceRows::Match { .. } => {
                for unit in 0..rows {
                    checkpoint_chunk(interrupt, unit, "repeat_measurement_units")?;
                    let stayer = unit >= mover_units;
                    set.push(if stayer {
                        rows as u64 + unit_cell(unit) as u64
                    } else {
                        unit as u64
                    });
                    mover.push(!stayer);
                }
            }
            ComponentInferenceRows::Observation { .. } => {
                let mut firms = Vec::new();
                reserve_exact(
                    &mut firms,
                    problem.workers(),
                    "repeat-measurement worker firm counts",
                )?;
                firms.resize(problem.workers(), 0_u32);
                for cell in 0..problem.cells() {
                    checkpoint_chunk(interrupt, cell, "repeat_measurement_units")?;
                    let worker = problem.cell_worker[cell] as usize;
                    firms[worker] = firms[worker].saturating_add(1);
                }
                reserve_exact(&mut unit_mass, rows, "repeat-measurement unit mass")?;
                for unit in 0..rows {
                    checkpoint_chunk(interrupt, unit, "repeat_measurement_units")?;
                    set.push(u64::from(problem.row_cell[unit]));
                    mover.push(firms[problem.row_worker[unit] as usize] > 1);
                    unit_mass.push(1.0);
                }
            }
        }
        for unit in 0..rows {
            checkpoint_chunk(interrupt, unit, "repeat_measurement_units")?;
            let cell = unit_cell(unit);
            worker_effect.push(coefficients.worker[problem.cell_worker[cell] as usize]);
            firm_effect.push(coefficients.firm[problem.cell_firm[cell] as usize]);
        }
        let mass = match (inference_rows, match_mass) {
            (ComponentInferenceRows::Match { .. }, Some(mass)) => mass,
            (ComponentInferenceRows::Observation { .. }, None) => unit_mass.as_slice(),
            _ => {
                return Err(BackendError::invariant(
                    "residual_noise_attachment",
                    "repeat-measurement standard errors lost their unit mass",
                ));
            }
        };
        let repeat = fit_repeat_variance(
            RepeatInputs {
                proxy: &proxy,
                outcome: working_y,
                residual,
                maker_inverse,
                leverage,
                mass,
                set: &set,
                mover: &mover,
                worker_effect: &worker_effect,
                firm_effect: &firm_effect,
                target_diagonal: [
                    &target_diagonal[0],
                    &target_diagonal[1],
                    &target_diagonal[2],
                ],
                positivity_multiplier: prepared.structured_options.positivity_multiplier,
            },
            interrupt,
        )?;
        (None, Some(repeat))
    } else {
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
        (Some(fit), None)
    };
    let leading_requested = repeat.is_some()
        && prepared.options.reference_distribution == ComponentReferenceDistribution::Q1;
    if !leading_requested {
        proxy = Vec::new();
    }
    let variance: &[f64] = match (&fit, &repeat) {
        (Some(fit), _) => &fit.variance,
        (None, Some(repeat)) => &repeat.variance,
        (None, None) => unreachable!("one variance source is always formed"),
    };
    // Repeat-measurement standard errors with controls: the first-order term
    // of their estimated coefficients, from the unit aggregates.
    let repeat_control = match (&repeat, control) {
        (Some(_), Some(control)) => Some(RepeatControl::new(
            control,
            residual,
            maker_inverse,
            interrupt,
        )?),
        _ => None,
    };

    let mut solve_receipts = Vec::new();
    let draws = crate::component_inference::residual_noise_draws(prepared.options.probes)?;
    reserve_exact(
        &mut solve_receipts,
        (prepared.options.probes as usize)
            .checked_add(PRIMITIVE_TARGETS)
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

    let mut leading = if leading_requested {
        Some(prepare_leading(
            problem,
            prepared,
            solver,
            coefficients,
            inference_rows,
            working_y,
            centered,
            residual,
            maker_inverse,
            target_diagonal,
            &influence,
            &proxy,
            match_mass,
            &mut solve_receipts,
            interrupt,
        )?)
    } else {
        None
    };
    // The control term's leave-out score variance needs the products again.
    if repeat_control.is_none() {
        proxy = Vec::new();
    }

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
            // With controls the draw takes the covariance of the noise of the
            // control-adjusted outcome, before centering and any kernel. The
            // unit draws above are those of a request without controls.
            if let Some(control) = repeat_control.as_ref() {
                let xi = control_normals(
                    rng,
                    (first + local) as u64,
                    addresses,
                    control.within_units(),
                    interrupt,
                )?;
                control.transform_draw(outcome, &xi)?;
            }
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
                &mut kernel_worker
                    [base * problem.workers()..(base + PRIMITIVE_TARGETS) * problem.workers()],
                &mut kernel_firm
                    [base * problem.firms()..(base + PRIMITIVE_TARGETS) * problem.firms()],
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
        // The remainder kernels of the four reported targets, on the same
        // pseudo-noise.
        let remainder_images = match leading.as_ref() {
            Some(state) => {
                let remainder_columns = width * REPORTED_TARGETS;
                let mut remainder_worker = vec![0.0; problem.workers() * remainder_columns];
                let mut remainder_firm = vec![0.0; problem.firms() * remainder_columns];
                for local in 0..width {
                    let base = local * REPORTED_TARGETS;
                    remainder_rhs(
                        problem,
                        inference_rows,
                        state,
                        &pseudo_outcome[local * rows..(local + 1) * rows],
                        &fitted.solution[local].coefficients,
                        &mut remainder_worker[base * problem.workers()
                            ..(base + REPORTED_TARGETS) * problem.workers()],
                        &mut remainder_firm
                            [base * problem.firms()..(base + REPORTED_TARGETS) * problem.firms()],
                        interrupt,
                    )?;
                }
                Some(solver.solve_batch_with_interrupt(
                    &remainder_worker,
                    &remainder_firm,
                    &[],
                    remainder_columns,
                    width,
                    interrupt,
                )?)
            }
            None => None,
        };
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
            if let (Some(state), Some(images)) = (leading.as_mut(), remainder_images.as_ref()) {
                for target in 0..REPORTED_TARGETS {
                    let image = &images.solution[local * REPORTED_TARGETS + target];
                    component_predict(
                        problem,
                        inference_rows,
                        solver,
                        &image.coefficients,
                        &mut prediction,
                        interrupt,
                    )?;
                    let direction = &state.direction[target];
                    let score = component_prediction_inner(&direction.mode, outcome)?;
                    let mut remainder = finish_q1_influence(
                        &prediction,
                        &state.ratio[target],
                        outcome,
                        &pseudo_residual,
                        &direction.mode,
                        direction.eigenvalue,
                        score,
                    )?;
                    if centered {
                        center_units(&mut remainder, match_mass, interrupt)?;
                    }
                    let value = match repeat_control.as_ref() {
                        Some(control) => control.image_square(variance, &remainder)?,
                        None => {
                            let mut value = StableAccumulator::default();
                            for row in 0..rows {
                                value.add(variance[row] * remainder[row] * remainder[row]);
                            }
                            value.finish()
                        }
                    };
                    state.trace[target].0 += value;
                    state.trace[target].1 += value * value;
                    state.kernel_solves += 1;
                    solve_receipts.push(component_solve_receipt(
                        ComponentInferenceSolvePhase::Influence,
                        (PRIMITIVE_TARGETS + target) as u32,
                        image,
                    ));
                }
            }
            match repeat_control.as_ref() {
                Some(control) => {
                    let mut projected = core::array::from_fn(|_| Vec::new());
                    let mut coefficient = core::array::from_fn(|_| Vec::new());
                    for target in 0..PRIMITIVE_TARGETS {
                        let image = control.image(&kernel[target])?;
                        projected[target] = image.projected;
                        coefficient[target] = image.coefficient;
                    }
                    drop(kernel);
                    accumulator.push_control(
                        variance,
                        maker_inverse,
                        &pseudo_residual,
                        projected,
                        coefficient,
                        control.omega(),
                        interrupt,
                    )?;
                }
                None => {
                    accumulator.push(
                        variance,
                        maker_inverse,
                        &pseudo_residual,
                        kernel,
                        interrupt,
                    )?;
                }
            }
        }
    }
    crate::progress::advance(crate::progress::INFERENCE, draws, draws);
    let moments = accumulator.finish()?;
    let (covariance, diagnostic) = match (&fit, &repeat) {
        (Some(fit), _) => {
            let covariance = finish_covariance(
                &influence,
                residual,
                maker_inverse,
                variance,
                control,
                &moments,
                interrupt,
            )?;
            let diagnostic = Diagnostic::new(
                fit,
                control.map_or(0, |value| value.controls),
                &moments,
                &covariance,
            )?;
            (covariance, diagnostic)
        }
        (None, Some(repeat)) => {
            let (covariance, control_terms) = finish_repeat_covariance(
                &influence,
                variance,
                &repeat.adjustment,
                &moments,
                repeat_control.as_ref(),
                interrupt,
            )?;
            let mut diagnostic = Diagnostic::repeat(
                repeat,
                repeat_control.as_ref().map_or(0, RepeatControl::controls),
                &moments,
                &covariance,
            )?;
            if let Some(shape) = diagnostic.repeat.as_mut() {
                shape.control_terms = control_terms;
            }
            if let (Some(state), Some(shape)) = (leading.take(), diagnostic.repeat.as_mut()) {
                shape.leading = Some(finish_leading(
                    prepared,
                    state,
                    repeat,
                    variance,
                    maker_inverse,
                    target_diagonal,
                    &moments,
                    repeat_control
                        .as_ref()
                        .map(|control| (control, proxy.as_slice())),
                    interrupt,
                )?);
            }
            (covariance, diagnostic)
        }
        (None, None) => unreachable!("one variance source is always formed"),
    };
    let mut result = residual_noise_result(rows, &covariance, &moments, diagnostic)?;
    // Transports reconcile probes with pseudo-noise solves; the draw count
    // stays in the diagnostic and the spectrum row.
    result.probes = prepared.options.probes;

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
    // The control term draws one more standard normal, two words, per draw
    // for every unit with a within-unit product; which units these are is
    // known only from the data, so they join the planned unit draws here.
    let (control_atoms, control_words) = match repeat_control.as_ref() {
        Some(control) => {
            let atoms = u64::try_from(control.within_units().len())
                .ok()
                .and_then(|units| units.checked_mul(draws as u64))
                .ok_or_else(|| resource("control-term Counter atom count overflow"))?;
            let words = atoms
                .checked_mul(2)
                .ok_or_else(|| resource("control-term Counter word count overflow"))?;
            (atoms, words)
        }
        None => (0, 0),
    };
    result.counter_atoms = counter_plan
        .logical_atoms
        .checked_add(control_atoms)
        .ok_or_else(|| resource("component-inference Counter atom count overflow"))?;
    result.counter_words = counter_plan
        .unique_words
        .checked_add(control_words)
        .ok_or_else(|| resource("component-inference Counter word count overflow"))?;
    result.inference_unit = prepared.inference_unit;
    // The control term accounts for the estimated offset to first order.
    // Without controls there is no offset, and repeat-measurement standard
    // errors report it as held fixed on any unit, as before.
    result.nuisance_uncertainty_conditioned_away = if prepared.repeat_measurement {
        repeat_control.is_none()
    } else {
        prepared.inference_unit == ComponentInferenceUnit::Match && control.is_none()
    };
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

/// Leading directions and the realized pieces of the `q = 1` decomposition.
///
/// The directions are found one after the other, each started from the
/// previous one: under a bottleneck the targets share their direction, and
/// the later searches then need a couple of solves. For every target the
/// remainder kernel is the target's kernel with its diagonal ratio net of the
/// direction, minus the rank-one part; its influence on the outcome costs one
/// solve.
#[allow(clippy::too_many_arguments)]
fn prepare_leading(
    problem: &CompressedProblem,
    prepared: &ComponentExecution<'_>,
    solver: &PreparedModelSolver<'_>,
    coefficients: &ModelCoefficients,
    inference_rows: ComponentInferenceRows<'_>,
    working_y: &[f64],
    centered: bool,
    residual: &[f64],
    maker_inverse: &[f64],
    target_diagonal: &[Vec<f64>; PRIMITIVE_TARGETS],
    influence: &[Vec<f64>; PRIMITIVE_TARGETS],
    proxy: &[f64],
    match_mass: Option<&[f64]>,
    solve_receipts: &mut Vec<ComponentInferenceSolveReceipt>,
    interrupt: &mut dyn InterruptCheck,
) -> Result<LeadingState> {
    let rows = working_y.len();
    let before = solve_receipts.len();
    let mut direction: Vec<leading_direction::LeadingDirection> = Vec::new();
    for target in 0..REPORTED_TARGETS {
        let found = leading_direction::leading_direction(
            problem,
            solver,
            inference_rows,
            target,
            direction.last().map(|previous| &previous.coefficients),
            prepared.options.spectrum_iterations,
            prepared.options.spectrum_tolerance,
            solve_receipts,
            interrupt,
        )?;
        direction.push(found);
    }
    let direction_solves = (solve_receipts.len() - before) as u32;

    let mut ratio: [Vec<f64>; REPORTED_TARGETS] = core::array::from_fn(|_| Vec::new());
    let mut leading_score = [0.0; REPORTED_TARGETS];
    let mut leading_leave_out = [0.0; REPORTED_TARGETS];
    let mut point_estimate = [0.0; REPORTED_TARGETS];
    for target in 0..REPORTED_TARGETS {
        let found = &direction[target];
        ratio[target] = q1_remainder_ratio(
            &reported_diagonal(target_diagonal, target),
            maker_inverse,
            &found.mode,
            found.eigenvalue,
        )?;
        leading_score[target] = component_prediction_inner(&found.mode, working_y)?;
        let mut leave_out = StableAccumulator::default();
        let mut point = StableAccumulator::default();
        for row in 0..rows {
            leave_out.add(found.mode[row] * found.mode[row] * proxy[row]);
            let image = if target < PRIMITIVE_TARGETS {
                influence[target][row]
            } else {
                influence[0][row] + influence[1][row] + 2.0 * influence[2][row]
            };
            point.add(working_y[row] * image);
        }
        leading_leave_out[target] = leave_out.finish();
        point_estimate[target] = point.finish();
    }

    let mut state = LeadingState {
        direction,
        ratio,
        influence: core::array::from_fn(|_| Vec::new()),
        point_estimate,
        leading_score,
        leading_leave_out,
        direct_remainder: [0.0; REPORTED_TARGETS],
        identity_tolerance: [0.0; REPORTED_TARGETS],
        trace: [(0.0, 0.0); REPORTED_TARGETS],
        direction_solves,
        kernel_solves: 0,
    };
    let mut worker_rhs = vec![0.0; problem.workers() * REPORTED_TARGETS];
    let mut firm_rhs = vec![0.0; problem.firms() * REPORTED_TARGETS];
    remainder_rhs(
        problem,
        inference_rows,
        &state,
        working_y,
        coefficients,
        &mut worker_rhs,
        &mut firm_rhs,
        interrupt,
    )?;
    let solved = solver.solve_batch_with_interrupt(
        &worker_rhs,
        &firm_rhs,
        &[],
        REPORTED_TARGETS,
        REPORTED_TARGETS,
        interrupt,
    )?;
    let mut prediction = vec![0.0; rows];
    for target in 0..REPORTED_TARGETS {
        let solution = &solved.solution[target];
        component_predict(
            problem,
            inference_rows,
            solver,
            &solution.coefficients,
            &mut prediction,
            interrupt,
        )?;
        let found = &state.direction[target];
        let mut remainder = finish_q1_influence(
            &prediction,
            &state.ratio[target],
            working_y,
            residual,
            &found.mode,
            found.eigenvalue,
            state.leading_score[target],
        )?;
        if centered {
            center_units(&mut remainder, match_mass, interrupt)?;
        }
        state.direct_remainder[target] = component_prediction_inner(working_y, &remainder)?;
        // The identity between the recentred decomposition and this direct
        // value holds to the accuracy of the two solves behind it.
        state.identity_tolerance[target] = (64.0 * solution.receipt.full_residual).max(1.0e-7);
        state.influence[target] = remainder;
        solve_receipts.push(component_solve_receipt(
            ComponentInferenceSolvePhase::Influence,
            (PRIMITIVE_TARGETS + target) as u32,
            solution,
        ));
    }
    Ok(state)
}

/// Assemble the decomposition and the interval of every reported target.
#[allow(clippy::too_many_arguments)]
fn finish_leading(
    prepared: &ComponentExecution<'_>,
    state: LeadingState,
    repeat: &RepeatVariance,
    variance: &[f64],
    maker_inverse: &[f64],
    target_diagonal: &[Vec<f64>; PRIMITIVE_TARGETS],
    moments: &crate::residual_noise::NoiseMoments,
    control: Option<(&RepeatControl<'_>, &[f64])>,
    interrupt: &mut dyn InterruptCheck,
) -> Result<RepeatLeading> {
    let draws = f64::from(moments.draws);
    let mut spectrum = [ComponentSpectrumDiagnostics::default(); REPORTED_TARGETS];
    let mut q1 = [ComponentQ1TargetResult::default(); REPORTED_TARGETS];
    let mut leading_variance_fitted = [0.0; REPORTED_TARGETS];
    let mut remainder_pair_term = [0.0; REPORTED_TARGETS];
    for target in 0..REPORTED_TARGETS {
        let found = &state.direction[target];
        let mean = state.trace[target].0 / draws;
        let spread = (state.trace[target].1 / draws - mean * mean).max(0.0);
        let mcse = if moments.draws > 1 {
            (spread / (draws - 1.0)).sqrt()
        } else {
            0.0
        };
        let pair = repeat.remainder_pair_term(
            &reported_diagonal(target_diagonal, target),
            maker_inverse,
            &found.mode,
            found.eigenvalue,
            target == 0 || target == REPORTED_TARGETS - 1,
            interrupt,
        )?;
        remainder_pair_term[target] = pair;
        let decomposition = finish_repeat_q1_target(
            state.point_estimate[target],
            state.leading_score[target],
            state.leading_leave_out[target],
            state.direct_remainder[target],
            state.identity_tolerance[target],
            found.eigenvalue,
            &found.mode,
            &state.influence[target],
            variance,
            mean,
            mcse,
            pair,
            prepared.options.psd_tolerance,
            control.map(|(control, proxy)| RepeatQ1Control { control, proxy }),
        )
        .map_err(|mut error| {
            error.message = format!("q1 target {target}: {}", error.message);
            error
        })?;
        q1[target] = finish_q1_interval(
            decomposition,
            prepared.options.seed,
            target,
            found.eigenvalue,
            prepared.options.confidence_level,
            prepared.options.critical_simulations,
        )?;
        let mut fitted = StableAccumulator::default();
        let mut largest = 0.0_f64;
        for row in 0..variance.len() {
            fitted.add(found.mode[row] * found.mode[row] * variance[row]);
            largest = largest.max(found.mode[row] * found.mode[row]);
        }
        let fitted = fitted.finish();
        // With controls the score's fitted variance and the trace both belong
        // to the covariance of the control-adjusted noise.
        let fitted = match control {
            Some((control, proxy)) => {
                control.score_fitted_variance(&found.mode, variance, proxy)?
            }
            None => fitted,
        };
        leading_variance_fitted[target] = fitted;
        // tr(K V K V) of the target, from the primitive traces.
        let trace = if target < PRIMITIVE_TARGETS {
            moments.trace[target * PRIMITIVE_TARGETS + target]
        } else {
            let weight = [1.0, 1.0, 2.0];
            let mut total = 0.0;
            for left in 0..PRIMITIVE_TARGETS {
                for right in 0..PRIMITIVE_TARGETS {
                    total += weight[left]
                        * weight[right]
                        * moments.trace[left.min(right) * PRIMITIVE_TARGETS + left.max(right)];
                }
            }
            total
        };
        let square = (found.eigenvalue * fitted) * (found.eigenvalue * fitted);
        spectrum[target] = ComponentSpectrumDiagnostics {
            certified: found.converged,
            leading_eigenvalue: found.eigenvalue,
            second_eigenvalue: found.second_eigenvalue,
            trace_square_raw: trace,
            trace_square: trace,
            trace_square_mcse: 0.0,
            trace_reconciliation: 0.0,
            leading_share: if trace > 0.0 {
                square / trace
            } else {
                f64::NAN
            },
            leading_share_mcse_trace_only: 0.0,
            remainder_leading_share: 0.0,
            maximum_mode_weight_squared: largest,
            leading_residual: found.residual,
            second_residual: f64::NAN,
            probes: moments.draws,
            iterations: found.steps,
        };
    }
    Ok(RepeatLeading {
        spectrum,
        q1,
        leading_variance_fitted,
        remainder_pair_term,
        direction_solves: state.direction_solves,
        influence_solves: REPORTED_TARGETS as u32,
        kernel_solves: state.kernel_solves,
        critical_simulations: prepared.options.critical_simulations,
    })
}
