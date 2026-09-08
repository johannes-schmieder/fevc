// SPDX-License-Identifier: GPL-3.0-only

//! Private live-solver adapter. No legacy caller selects this path.
use super::*;
use crate::residual_moment_inference::{
    Diagnostic, Options, ProjectionReceipt, DESIGN_ORDERING_CONTRACT, MATCH_ORDERING_CONTRACT,
    ORDERING_CONTRACT,
};
use crate::residual_moments::{memory_plan, prepare_with_interrupt};
use crate::structured_variance::{basis_row, normalized_midranks};

pub(crate) fn design_order(
    problem: &CompressedProblem,
    interrupt: &mut dyn InterruptCheck,
) -> Result<Vec<usize>> {
    let keys = problem.probe_order.as_ref().ok_or_else(|| {
        invalid(
        "internal residual-moment inference requires an explicit outcome-free unique probe order")
    })?;
    if keys.len() != problem.outcome.len() || keys.iter().any(|x| !x.is_finite()) {
        return Err(invalid(
            "residual-moment probe order is nonfinite or has the wrong length",
        ));
    }
    let mut order = index_vector(keys.len(), interrupt, "residual_moment_design_order")?;
    stable_sort_by_with_interrupt(
        &mut order,
        |&a, &b| canonical_zero(keys[a]).total_cmp(&canonical_zero(keys[b])),
        interrupt,
        "residual_moment_design_order",
    )?;
    for pair in order.windows(2) {
        if canonical_zero(keys[pair[0]]) == canonical_zero(keys[pair[1]]) {
            return Err(invalid(
                "residual-moment probe-order keys must be globally unique",
            ));
        }
    }
    Ok(order)
}

pub(super) fn canonical_design_order(
    problem: &CompressedProblem,
    interrupt: &mut dyn InterruptCheck,
) -> Result<Vec<usize>> {
    let mut order = index_vector(problem.outcome.len(), interrupt, "inference_design_order")?;
    stable_sort_by_with_interrupt(
        &mut order,
        |&a, &b| {
            let mut cmp = problem.row_worker[a]
                .cmp(&problem.row_worker[b])
                .then_with(|| problem.row_firm[a].cmp(&problem.row_firm[b]))
                .then_with(|| problem.frequency[a].cmp(&problem.frequency[b]))
                .then_with(|| {
                    canonical_zero(problem.target_weight[a])
                        .total_cmp(&canonical_zero(problem.target_weight[b]))
                });
            for control in &problem.controls {
                cmp = cmp.then_with(|| {
                    canonical_zero(control[a]).total_cmp(&canonical_zero(control[b]))
                });
            }
            if let Some(keys) = &problem.probe_order {
                cmp = cmp.then_with(|| canonical_zero(keys[a]).total_cmp(&canonical_zero(keys[b])));
            }
            // Exact structural copies receive the same contiguous pool of
            // distinct probe addresses, without ordering them by their outcomes.
            cmp
        },
        interrupt,
        "inference_design_order",
    )?;
    Ok(order)
}

pub(super) fn row_ranks(order: &[usize]) -> Vec<u64> {
    let mut ranks = vec![0; order.len()];
    for (position, &row) in order.iter().enumerate() {
        ranks[row] = position as u64 + 1;
    }
    ranks
}

pub(super) fn terms(prepared: &PreparedComponentInference) -> Result<usize> {
    match prepared.variance_source {
        ComponentVarianceSource::StructuredCommon => Ok(
            if prepared.inference_unit == ComponentInferenceUnit::Match {
                21
            } else {
                15
            },
        ),
        ComponentVarianceSource::StructuredLeverage => Ok(3),
        ComponentVarianceSource::Oracle => Err(invalid(
            "residual moments cannot have oracle variance source",
        )),
    }
}

fn callback_bytes(problem: &CompressedProblem, route: RouteMemory) -> Result<u64> {
    let p = checked_sum(&[
        problem.workers() as u64,
        problem.firms() as u64,
        problem.controls.len() as u64,
    ])?;
    // Scalar complete-system solve/RHS/result work plus two row permutations.
    // The live solver and its control sufficient statistics are already owned
    // and admitted by generic JLA. CMG's per-column workspace remains charged.
    checked_sum(&[
        checked_product(&[p, 64, 8], "residual projection coefficient workspace")?,
        checked_product(
            &[problem.outcome.len() as u64, 4, 8],
            "residual projection row workspace",
        )?,
        route.cmg_batch_workspace_per_column,
    ])
}

pub(super) fn peak_bytes(
    problem: &CompressedProblem,
    prepared: &PreparedComponentInference,
    route: RouteMemory,
) -> Result<u64> {
    let options = prepared
        .residual_moments
        .ok_or_else(|| invalid("missing residual moment options"))?;
    let terms = terms(prepared)?;
    let mut moment = options.moment_options();
    if prepared.design_only_order || prepared.unified_variance_fit {
        moment.rank_tolerance = prepared.structured_options.rank_tolerance;
        moment.positivity_multiplier = prepared.structured_options.positivity_multiplier;
        moment.observations_per_term = prepared.structured_options.observations_per_term;
    }
    moment.projection_workspace_bytes = usize::try_from(callback_bytes(problem, route)?)
        .map_err(|_| resource("residual projection workspace overflow"))?;
    moment.additional_memory_limit_bytes = usize::MAX;
    let units = match prepared.inference_unit {
        ComponentInferenceUnit::Observation => problem.outcome.len(),
        ComponentInferenceUnit::Match => problem.deletion_units(),
    };
    let plan = if prepared.unified_variance_fit {
        crate::residual_moments::memory_envelope(units, terms, moment)?
    } else {
        memory_plan(units, terms, moment)?
    };
    checked_sum(&[
        plan.additional_peak_bytes as u64,
        checked_product(
            &[units as u64, 2 * terms as u64 + 18, 8],
            "residual basis ranks ordering and fit inputs",
        )?,
        checked_product(
            &[
                options.probes as u64,
                core::mem::size_of::<ProjectionReceipt>() as u64,
            ],
            "residual projection receipts",
        )?,
    ])
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
pub(super) fn fit(
    problem: &CompressedProblem,
    prepared: &PreparedComponentInference,
    solver: &PreparedModelSolver<'_>,
    inference_rows: ComponentInferenceRows<'_>,
    residual: &[f64],
    leverage: &[f64],
    target_diagonal: &[Vec<f64>; PRIMITIVE_TARGETS],
    addresses: &ComponentInferenceAddresses,
    match_mass: Option<&[f64]>,
    interrupt: &mut dyn InterruptCheck,
) -> Result<Diagnostic> {
    let options: Options = prepared
        .residual_moments
        .ok_or_else(|| invalid("missing residual moment options"))?;
    let match_order;
    let order = match inference_rows {
        ComponentInferenceRows::Observation { row_order, .. } => row_order,
        ComponentInferenceRows::Match { .. } => {
            let mut order = index_vector(addresses.entity.len(), interrupt, "match_moment_order")?;
            stable_sort_by_with_interrupt(
                &mut order,
                |&a, &b| {
                    (addresses.entity[a], addresses.subdraw[a])
                        .cmp(&(addresses.entity[b], addresses.subdraw[b]))
                },
                interrupt,
                "match_moment_order",
            )?;
            match_order = order;
            &match_order
        }
    };
    let n = order.len();
    let terms = terms(prepared)?;
    let model = prepared
        .variance_source
        .structured_model()
        .ok_or_else(|| invalid("missing structured basis"))?;
    let mut ranks = vec![normalized_midranks(leverage, interrupt)?];
    for diagonal in target_diagonal {
        ranks.push(normalized_midranks(diagonal, interrupt)?);
    }
    if let Some(mass) = match_mass {
        ranks.push(normalized_midranks(mass, interrupt)?);
    }
    let mut basis = Vec::with_capacity(n * terms);
    let mut h = Vec::with_capacity(n);
    let mut e = Vec::with_capacity(n);
    let mut ids = Vec::with_capacity(n);
    for (position, &row) in order.iter().enumerate() {
        checkpoint_chunk(interrupt, position, "residual_moment_basis")?;
        basis.extend_from_slice(&basis_row(model, &ranks, row)[..terms]);
        h.push(leverage[row]);
        e.push(residual[row]);
        ids.push((addresses.entity[row], addresses.subdraw[row]));
    }
    drop(ranks);
    let (basis_columns, basis_reconstruction_error) = if prepared.unified_variance_fit {
        crate::residual_moments::basis::reduce(&mut basis, terms, interrupt)?
    } else {
        ((0..terms).collect(), 0.0)
    };
    let terms = basis_columns.len();
    let route = route_memory_forecast(problem, solver, problem.controls.len())?;
    let mut moment = options.moment_options();
    if prepared.design_only_order || prepared.unified_variance_fit {
        moment.rank_tolerance = prepared.structured_options.rank_tolerance;
        moment.positivity_multiplier = prepared.structured_options.positivity_multiplier;
        moment.observations_per_term = prepared.structured_options.observations_per_term;
    }
    moment.effective_projection_tolerance = solver.options().pcg.tolerance;
    moment.projection_workspace_bytes = usize::try_from(callback_bytes(problem, route)?)
        .map_err(|_| resource("residual projection workspace overflow"))?;
    // This increment was added to the complete generic-JLA admission pre-RNG.
    moment.additional_memory_limit_bytes = usize::MAX;
    let mut projections = Vec::with_capacity(options.probes);
    let mut input = vec![0.0; n];
    let mut prediction = vec![0.0; n];
    // Estimated h is explicitly identified in the result; its statistical
    // properties are not the exact-h moment identity of the stand-alone API.
    let candidate = prepare_with_interrupt(
        &basis,
        terms,
        &h,
        &ids,
        moment,
        |gaussian, columns, projected, certificates, interrupt| {
            for column in 0..columns {
                for (position, &row) in order.iter().enumerate() {
                    input[row] = gaussian[column * n + position];
                }
                let rhs = component_transpose_rhs(problem, inference_rows, &input, interrupt)?;
                let solved = solver.solve_with_interrupt(
                    ModelRhs {
                        worker: &rhs.0,
                        firm: &rhs.1,
                        control: &rhs.2,
                    },
                    interrupt,
                )?;
                component_predict(
                    problem,
                    inference_rows,
                    solver,
                    &solved.coefficients,
                    &mut prediction,
                    interrupt,
                )?;
                for (position, &row) in order.iter().enumerate() {
                    projected[column * n + position] = prediction[row];
                }
                certificates[column] = solved.receipt.full_residual;
                projections.push(ProjectionReceipt {
                    probe: projections.len(),
                    iterations: solved.receipt.pcg.iterations,
                    reduced_residual: solved.receipt.pcg.relative_residual,
                    complete_residual: solved.receipt.full_residual,
                    full_residual_tolerance: solved.receipt.full_residual_tolerance,
                });
            }
            Ok(())
        },
        interrupt,
    )?;
    let mut fit = candidate.fit_with_interrupt(&e, interrupt)?;
    let mut raw = vec![0.0; n];
    let mut positive = vec![0.0; n];
    for (position, &row) in order.iter().enumerate() {
        raw[row] = fit.raw_variance[position];
        positive[row] = fit.positive_variance[position];
    }
    fit.raw_variance = raw;
    fit.positive_variance = positive;
    Ok(Diagnostic {
        ordering_contract: if prepared.inference_unit == ComponentInferenceUnit::Match {
            MATCH_ORDERING_CONTRACT
        } else if prepared.design_only_order {
            DESIGN_ORDERING_CONTRACT
        } else {
            ORDERING_CONTRACT
        },
        estimated_leverage_input: true,
        preparation: candidate.diagnostic,
        gram: candidate.gram().to_vec(),
        fit,
        projections,
        basis_columns,
        basis_reconstruction_error,
    })
}
