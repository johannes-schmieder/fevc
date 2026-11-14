// SPDX-License-Identifier: MIT
use super::*;
use vckss_core::component_inference::{
    prepare_repeat_measurement_with_interrupt, prepare_residual_noise_with_interrupt,
};
use vckss_core::residual_noise::fit_working_variance;
use vckss_core::types::Centering;

const DRAWS: u32 = 96;

/// Independent unit-level reconstruction of the pooled fixed-offset design:
/// one row per mover match scaled by sqrt(mass), one per physical stayer copy.
struct DenseUnits {
    rows: usize,
    movers: usize,
    mass: Vec<f64>,
    y: Vec<f64>,
    maker: Vec<f64>,
    kernel: [Vec<f64>; 3],
    /// Control-major unit aggregates of the `k` controls.
    collapsed: Vec<Vec<f64>>,
    residualized: Vec<Vec<f64>>,
    within: Vec<Vec<f64>>,
    precision: Vec<f64>,
    /// Worker of every unit and the dense least-squares effects of its
    /// worker and firm (last firm grounded at zero).
    worker: Vec<u32>,
    worker_effect: Vec<f64>,
    firm_effect: Vec<f64>,
}

fn dense_units(
    hybrid: &vckss_core::stayer_hybrid::PreparedExactStayerHybrid,
    inference: &vckss_core::component_inference::ComponentInferenceResult,
) -> DenseUnits {
    let problem = &hybrid.problem;
    let stored = problem.outcome.len();
    let (raw_x, p) = super::component_centering::design(problem, false);
    let working =
        super::component_centering::working_outcome(problem, ComponentInferenceUnit::Match);
    let weight = problem
        .frequency
        .iter()
        .map(|&f| f as f64)
        .collect::<Vec<_>>();
    let center = working.iter().zip(&weight).map(|(y, f)| y * f).sum::<f64>()
        / problem.physical_total as f64;
    // Controls residualized on the two-way effects over physical observations.
    let mut xtw = dense_transpose(&raw_x, stored, p);
    for column in 0..p {
        for row in 0..stored {
            xtw[column * stored + row] *= weight[row];
        }
    }
    let fe_inverse = dense_inverse(&dense_multiply(&xtw, p, stored, &raw_x, p), p);
    let k = problem.controls.len();
    let partial: Vec<Vec<f64>> = (0..k)
        .map(|j| {
            let beta = dense_multiply(
                &fe_inverse,
                p,
                p,
                &dense_multiply(&xtw, p, stored, &problem.controls[j], 1),
                1,
            );
            let fitted = dense_multiply(&raw_x, stored, p, &beta, 1);
            (0..stored)
                .map(|row| problem.controls[j][row] - fitted[row])
                .collect()
        })
        .collect();
    let mut information = vec![0.0; k * k];
    for left in 0..k {
        for right in 0..k {
            information[left * k + right] = (0..stored)
                .map(|row| weight[row] * partial[left][row] * partial[right][row])
                .sum();
        }
    }
    let precision = dense_inverse(&information, k);

    let rows = inference.independent_units as usize;
    let movers = hybrid.plan.mover_deletion_units;
    let mut x = vec![0.0; rows * p];
    let mut y = vec![0.0; rows];
    let mut mass = vec![0.0_f64; rows];
    let mut collapsed = vec![vec![0.0; rows]; k];
    let mut residualized = vec![vec![0.0; rows]; k];
    let mut within = vec![vec![0.0; rows]; k];
    let mut outcome_sum = vec![0.0; movers];
    let mut worker = vec![0_u32; rows];
    let mut firm = vec![0_u32; rows];
    let mut next = movers;
    for row in 0..stored {
        if hybrid.plan.stayer_rows[row] {
            for _ in 0..problem.frequency[row] {
                worker[next] = problem.row_worker[row];
                firm[next] = problem.row_firm[row];
                x[next * p..(next + 1) * p].copy_from_slice(&raw_x[row * p..(row + 1) * p]);
                y[next] = working[row] - center;
                mass[next] = 1.0;
                for j in 0..k {
                    collapsed[j][next] = problem.controls[j][row];
                    residualized[j][next] = partial[j][row];
                }
                next += 1;
            }
        } else {
            let group = problem.row_deletion[row] as usize;
            worker[group] = problem.row_worker[row];
            firm[group] = problem.row_firm[row];
            mass[group] += weight[row];
            outcome_sum[group] += weight[row] * working[row];
            y[group] += weight[row] * (working[row] - center);
            x[group * p..(group + 1) * p].copy_from_slice(&raw_x[row * p..(row + 1) * p]);
            for j in 0..k {
                collapsed[j][group] += weight[row] * problem.controls[j][row];
                residualized[j][group] += weight[row] * partial[j][row];
            }
        }
    }
    assert_eq!(next, rows);
    for row in 0..stored {
        if !hybrid.plan.stayer_rows[row] {
            let group = problem.row_deletion[row] as usize;
            for j in 0..k {
                within[j][group] += weight[row]
                    * (problem.controls[j][row] - collapsed[j][group] / mass[group])
                    * (working[row] - outcome_sum[group] / mass[group]);
            }
        }
    }
    for group in 0..movers {
        let root = mass[group].sqrt();
        y[group] /= root;
        for value in &mut x[group * p..(group + 1) * p] {
            *value *= root;
        }
        for j in 0..k {
            collapsed[j][group] /= root;
            residualized[j][group] /= root;
        }
    }
    let xt = dense_transpose(&x, rows, p);
    let inverse = dense_inverse(&dense_multiply(&xt, p, rows, &x, p), p);
    let bread = dense_multiply(&x, rows, p, &inverse, p);
    let bread_t = dense_transpose(&bread, rows, p);
    let beta = dense_multiply(&bread_t, p, rows, &y, 1);
    let worker_effect = worker.iter().map(|&w| beta[w as usize]).collect();
    let firm_effect = firm
        .iter()
        .map(|&f| {
            if (f as usize) + 1 < problem.firms() {
                beta[problem.workers() + f as usize]
            } else {
                0.0
            }
        })
        .collect();
    let projection = dense_multiply(&bread, rows, p, &xt, rows);
    let maker: Vec<f64> = projection
        .iter()
        .enumerate()
        .map(|(index, value)| f64::from(index / rows == index % rows) - value)
        .collect();
    // Target matrices on the pooled target population.
    let mut worker_mean = vec![0.0; p];
    let mut firm_mean = vec![0.0; p];
    for cell in 0..problem.cells() {
        let share = problem.cell_target_sum[cell] / problem.target_total;
        worker_mean[problem.cell_worker[cell] as usize] += share;
        let firm = problem.cell_firm[cell] as usize;
        if firm + 1 < problem.firms() {
            firm_mean[problem.workers() + firm] += share;
        }
    }
    let mut targets: [Vec<f64>; 3] = core::array::from_fn(|_| vec![0.0; p * p]);
    for cell in 0..problem.cells() {
        let share = problem.cell_target_sum[cell] / problem.target_total;
        let mut worker: Vec<f64> = worker_mean.iter().map(|v| -v).collect();
        let mut firm: Vec<f64> = firm_mean.iter().map(|v| -v).collect();
        worker[problem.cell_worker[cell] as usize] += 1.0;
        let firm_index = problem.cell_firm[cell] as usize;
        if firm_index + 1 < problem.firms() {
            firm[problem.workers() + firm_index] += 1.0;
        }
        for i in 0..p {
            for j in 0..p {
                targets[0][i * p + j] += share * worker[i] * worker[j];
                targets[1][i * p + j] += share * firm[i] * firm[j];
                targets[2][i * p + j] += 0.5 * share * (worker[i] * firm[j] + firm[i] * worker[j]);
            }
        }
    }
    // Mean-centred leave-out kernel H C_t H with the engine's own diagonals.
    let root_total = mass.iter().sum::<f64>();
    let center_matrix: Vec<f64> = (0..rows * rows)
        .map(|index| {
            let (a, b) = (index / rows, index % rows);
            f64::from(a == b) - mass[a].sqrt() * mass[b].sqrt() / root_total
        })
        .collect();
    let kernel = core::array::from_fn(|target| {
        let mut raw = dense_multiply(
            &dense_multiply(&bread, rows, p, &targets[target], p),
            rows,
            p,
            &bread_t,
            rows,
        );
        for i in 0..rows {
            for j in 0..rows {
                raw[i * rows + j] -= 0.5
                    * maker[i * rows + j]
                    * (inference.target_diagonal[target][i] * inference.maker_inverse[i]
                        + inference.target_diagonal[target][j] * inference.maker_inverse[j]);
            }
        }
        dense_multiply(
            &dense_multiply(&center_matrix, rows, rows, &raw, rows),
            rows,
            rows,
            &center_matrix,
            rows,
        )
    });
    DenseUnits {
        rows,
        movers,
        mass,
        y,
        maker,
        kernel,
        collapsed,
        residualized,
        within,
        precision,
        worker,
        worker_effect,
        firm_effect,
    }
}

fn run(
    hybrid: &vckss_core::stayer_hybrid::PreparedExactStayerHybrid,
    route: ModelSolverRoute,
    attach: bool,
) -> vckss_core::generic_jla::GenericJlaResult {
    run_centered(hybrid, route, Centering::Mean, attach)
}

fn run_centered(
    hybrid: &vckss_core::stayer_hybrid::PreparedExactStayerHybrid,
    route: ModelSolverRoute,
    centering: Centering,
    attach: bool,
) -> vckss_core::generic_jla::GenericJlaResult {
    run_nuisance(hybrid, route, centering, NuisanceMode::FixedOffset, attach)
}

fn run_nuisance(
    hybrid: &vckss_core::stayer_hybrid::PreparedExactStayerHybrid,
    route: ModelSolverRoute,
    centering: Centering,
    nuisance: NuisanceMode,
    attach: bool,
) -> vckss_core::generic_jla::GenericJlaResult {
    let prepared = prepare_residual_noise_with_interrupt(
        &hybrid.problem,
        ComponentInferenceUnit::Match,
        ComponentInferenceOptions {
            probes: 4 * DRAWS,
            ..ComponentInferenceOptions::default()
        },
        StructuredVarianceOptions::default(),
        &mut NeverInterrupt,
    )
    .expect("standard-error preparation");
    run_generic_jla_routed_with_attachments_and_hybrid_interrupt(
        &hybrid.problem,
        routed_options(
            GenericJlaOptions {
                centering,
                ..options(DeletionMode::Match, nuisance)
            },
            route,
        ),
        None,
        attach.then_some(&prepared),
        Some(&hybrid.plan),
        &mut NeverInterrupt,
    )
    .expect("pooled run")
}

#[test]
fn pooled_fixed_offset_standard_errors_match_dense_expectations() {
    let hybrid = super::pooled_component::fixture(false);
    let point = run(&hybrid, ModelSolverRoute::Diagonal, false);
    let result = run(&hybrid, ModelSolverRoute::Diagonal, true);
    // The attachment must leave the point estimate untouched.
    for (a, b) in components(point.corrected)
        .into_iter()
        .zip(components(result.corrected))
    {
        assert!((a - b).abs() <= 1e-12 * a.abs().max(1.0), "{a} versus {b}");
    }
    let attached = result
        .component_inference
        .expect("attached standard errors");
    let diagnostic = attached.residual_noise.as_ref().expect("diagnostic");
    assert_eq!(attached.variance_source, ComponentVarianceSource::Oracle);
    assert_eq!(
        attached.independent_units,
        hybrid.receipt.combined_deletion_units
    );
    assert_eq!(attached.solve_receipts.len(), 3 + 4 * DRAWS as usize);
    assert_eq!(diagnostic.draws, DRAWS);
    assert_eq!(diagnostic.controls, 2);
    assert!(!attached.nuisance_uncertainty_conditioned_away);
    assert!(attached.q1.is_none() && attached.residual_moments.is_none());

    let dense = dense_units(&hybrid, &attached);
    let rows = dense.rows;
    let residual = dense_multiply(&dense.maker, rows, rows, &dense.y, 1);
    let influence: [Vec<f64>; 3] =
        core::array::from_fn(|t| dense_multiply(&dense.kernel[t], rows, rows, &dense.y, 1));
    for target in 0..3 {
        for (&expected, &actual) in influence[target].iter().zip(&attached.influence[target]) {
            assert_close(expected, actual, 2.0e-9);
        }
    }
    let proxy: Vec<f64> = (0..rows)
        .map(|a| dense.y[a] * attached.maker_inverse[a] * residual[a])
        .collect();
    let fit = fit_working_variance(
        &proxy,
        &residual,
        &attached.maker_inverse,
        &attached.leverage,
        Some(&dense.mass),
        dense.movers,
        StructuredVarianceOptions::default().positivity_multiplier,
        &mut NeverInterrupt,
    )
    .expect("dense working variance");
    assert_eq!(fit.terms, diagnostic.working_terms);
    let variance = &fit.variance;

    // Pure-noise expectations in closed form against the pseudo-noise draws.
    for left in 0..3 {
        for right in 0..3 {
            let index = left * 3 + right;
            let mut trace = 0.0;
            let mut noise = 0.0;
            for a in 0..rows {
                let mut expected_square = 0.0;
                let mut cross = 0.0;
                let mut covariance = [0.0; 2];
                for b in 0..rows {
                    let maker = dense.maker[a * rows + b];
                    let (k_left, k_right) = (
                        dense.kernel[left][a * rows + b],
                        dense.kernel[right][a * rows + b],
                    );
                    expected_square += maker * maker * variance[b];
                    cross += k_left * k_right * variance[b];
                    covariance[0] += k_left * maker * variance[b];
                    covariance[1] += k_right * maker * variance[b];
                    trace += k_left * variance[b] * dense.kernel[right][b * rows + a] * variance[a];
                }
                noise += attached.maker_inverse[a]
                    * (expected_square * cross + 2.0 * covariance[0] * covariance[1]);
            }
            let scale = attached.trace_term[0]
                .abs()
                .max(attached.trace_term[4].abs());
            assert!(
                (attached.trace_term[index] - 2.0 * trace).abs()
                    <= 6.0 * attached.trace_mcse[index] + 1e-9 * scale,
                "trace {index}: {} versus {}",
                attached.trace_term[index],
                2.0 * trace
            );
            assert!(
                (diagnostic.residual_noise[index] - noise).abs()
                    <= 6.0 * diagnostic.residual_noise_mcse[index]
                        + 1e-9 * diagnostic.residual_noise[0].abs(),
                "noise {index}: {} versus {noise}",
                diagnostic.residual_noise[index]
            );
        }
    }

    // Signal: squared unit scores with the control-estimation term, minus the
    // engine's own pseudo-noise correction.
    let load: [[f64; 2]; 3] = core::array::from_fn(|t| {
        let slope: [f64; 2] = core::array::from_fn(|j| {
            -2.0 * (0..rows)
                .map(|a| dense.collapsed[j][a] * influence[t][a])
                .sum::<f64>()
        });
        core::array::from_fn(|j| (0..2).map(|k| dense.precision[j * 2 + k] * slope[k]).sum())
    });
    let score = |t: usize, a: usize| {
        let scaled = residual[a] * attached.maker_inverse[a].sqrt();
        let mut slope = 2.0 * influence[t][a];
        let mut shift = 0.0;
        for j in 0..2 {
            slope += dense.residualized[j][a] * load[t][j];
            shift += dense.within[j][a] * load[t][j];
        }
        slope * scaled + shift
    };
    let scale = attached.influence_term[0]
        .abs()
        .max(attached.influence_term[4].abs());
    for left in 0..3 {
        for right in 0..3 {
            let index = left * 3 + right;
            let expected = (0..rows)
                .map(|a| score(left, a) * score(right, a))
                .sum::<f64>()
                - 4.0 * diagnostic.residual_noise[index];
            assert!(
                (attached.influence_term[index] - expected).abs() <= 1e-7 * scale,
                "signal {index}: {} versus {expected}",
                attached.influence_term[index]
            );
        }
    }
    // The control term must matter here, or the check above proves nothing.
    let without = (0..rows)
        .map(|a| {
            let value = 2.0 * influence[1][a] * residual[a] * attached.maker_inverse[a].sqrt();
            value * value
        })
        .sum::<f64>()
        - 4.0 * diagnostic.residual_noise[4];
    assert!((attached.influence_term[4] - without).abs() > 1e-4 * scale);

    // Reported covariance: truncated signal plus trace, mapped to four targets.
    for target in 0..3 {
        let index = target * 3 + target;
        assert!(attached.primitive_covariance[index] > 0.0);
        assert!(attached.primitive_covariance[index] >= attached.trace_term[index] * (1.0 - 1e-12));
    }
    if attached.psd_cleanup == 0.0 {
        for index in 0..9 {
            assert_close(
                attached.influence_term[index] + attached.trace_term[index],
                attached.primitive_covariance[index],
                1e-12,
            );
        }
    }
    for column in 0..4 {
        assert_close(
            attached.covariance[column]
                + attached.covariance[4 + column]
                + 2.0 * attached.covariance[8 + column],
            attached.covariance[12 + column],
            1e-12,
        );
    }
    for target in 0..4 {
        assert_eq!(
            attached.q0_status[target],
            vckss_core::component_inference::ComponentQ0Status::Computed
        );
        assert!(attached.covariance[target * 5] > 0.0);
        assert!(diagnostic.model_covariance[target * 5] > 0.0);
        assert!(attached.influence_concentration[target] > 0.0);
        assert!(diagnostic.effective_rank[target] >= 1.0);
        assert!(attached.spectrum[target].leading_share <= 1.0);
    }
}

#[test]
fn standard_errors_are_invariant_to_the_solver_route() {
    let hybrid = super::pooled_component::fixture(false);
    let diagonal = run(&hybrid, ModelSolverRoute::Diagonal, true)
        .component_inference
        .expect("diagonal");
    let cmg = run(&hybrid, ModelSolverRoute::Cmg, true)
        .component_inference
        .expect("cmg");
    for index in 0..16 {
        assert_close(diagonal.covariance[index], cmg.covariance[index], 1e-6);
    }
}

#[test]
fn standard_errors_follow_the_automatic_solver_route() {
    let hybrid = super::pooled_component::fixture(false);
    let diagonal = run(&hybrid, ModelSolverRoute::Diagonal, true);
    let automatic = run(&hybrid, ModelSolverRoute::Auto, true);
    assert_eq!(
        automatic.receipt.execution.requested_route,
        ModelSolverRoute::Auto
    );
    let (diagonal, automatic) = (
        diagonal.component_inference.expect("diagonal"),
        automatic.component_inference.expect("automatic"),
    );
    for index in 0..16 {
        assert_close(
            diagonal.covariance[index],
            automatic.covariance[index],
            1e-6,
        );
    }
}

#[test]
fn probe_counts_must_be_whole_draws() {
    let hybrid = super::pooled_component::fixture(false);
    for probes in [4, 7, 30] {
        assert!(
            prepare_residual_noise_with_interrupt(
                &hybrid.problem,
                ComponentInferenceUnit::Match,
                ComponentInferenceOptions {
                    probes,
                    ..ComponentInferenceOptions::default()
                },
                StructuredVarianceOptions::default(),
                &mut NeverInterrupt,
            )
            .is_err(),
            "{probes} probes"
        );
    }
}

// The corrected point estimate adds a small increment to the mean-centred
// one. The reported standard error is that of the mean-centred estimate.
#[test]
fn corrected_centering_reports_the_mean_centered_standard_error() {
    let hybrid = super::pooled_component::fixture(false);
    let mean = run_centered(&hybrid, ModelSolverRoute::Diagonal, Centering::Mean, true);
    let corrected = run_centered(
        &hybrid,
        ModelSolverRoute::Diagonal,
        Centering::Corrected,
        true,
    );
    let point_only = run_centered(
        &hybrid,
        ModelSolverRoute::Diagonal,
        Centering::Corrected,
        false,
    );
    assert_eq!(point_only.corrected, corrected.corrected);
    assert_ne!(mean.corrected, corrected.corrected);
    let (mean, corrected) = (
        mean.component_inference.expect("mean"),
        corrected.component_inference.expect("corrected"),
    );
    for index in 0..16 {
        assert_close(mean.covariance[index], corrected.covariance[index], 1e-12);
    }
}

fn mover_only(
    problem: &CompressedProblem,
    nuisance: NuisanceMode,
) -> vckss_core::component_inference::ComponentInferenceResult {
    let prepared = prepare_residual_noise_with_interrupt(
        problem,
        ComponentInferenceUnit::Match,
        ComponentInferenceOptions {
            probes: 4 * DRAWS,
            ..ComponentInferenceOptions::default()
        },
        StructuredVarianceOptions::default(),
        &mut NeverInterrupt,
    )
    .expect("standard-error preparation");
    run_generic_jla_routed_with_attachments_and_hybrid_interrupt(
        problem,
        routed_options(
            GenericJlaOptions {
                centering: Centering::Mean,
                ..options(DeletionMode::Match, nuisance)
            },
            ModelSolverRoute::Diagonal,
        ),
        None,
        Some(&prepared),
        None,
        &mut NeverInterrupt,
    )
    .expect("mover-only run")
    .component_inference
    .expect("attached standard errors")
}

// Without controls there is no offset, so the default joint mode must give
// the fixed-offset standard errors with no further solves.
#[test]
fn no_control_match_units_accept_either_nuisance_mode() {
    let problem = fixture(false);
    assert!(problem.controls.is_empty());
    let fixed = mover_only(&problem, NuisanceMode::FixedOffset);
    let joint = mover_only(&problem, NuisanceMode::Joint);
    assert!(fixed.nuisance_uncertainty_conditioned_away);
    for index in 0..16 {
        assert_close(fixed.covariance[index], joint.covariance[index], 1e-10);
    }
}

// Observation units keep the controls in the operator, so the joint kernel
// carries their estimation and no separate control term is added.
#[test]
fn observation_units_with_joint_controls_use_the_joint_kernel() {
    let problem = component_inference_fixture(true);
    let n = problem.outcome.len();
    let mut estimator = options(DeletionMode::Observation, NuisanceMode::Joint);
    estimator.probes = 200;
    estimator.centering = Centering::Mean;
    let request = routed_options(estimator, ModelSolverRoute::Diagonal);
    let oracle = prepare_oracle_component_inference(
        &problem,
        &vec![0.002; n],
        ComponentInferenceOptions {
            probes: 64,
            spectrum_probes: 64,
            spectrum_iterations: 128,
            spectrum_tolerance: 1.0e-3,
            ..ComponentInferenceOptions::default()
        },
    )
    .expect("oracle preparation");
    let reference = run_generic_jla_routed_with_attachments_and_hybrid_interrupt(
        &problem,
        request,
        None,
        Some(&oracle),
        None,
        &mut NeverInterrupt,
    )
    .expect("oracle run")
    .component_inference
    .expect("oracle inference");
    let prepared = prepare_residual_noise_with_interrupt(
        &problem,
        ComponentInferenceUnit::Observation,
        ComponentInferenceOptions {
            probes: 4 * DRAWS,
            ..ComponentInferenceOptions::default()
        },
        StructuredVarianceOptions::default(),
        &mut NeverInterrupt,
    )
    .expect("standard-error preparation");
    let result = run_generic_jla_routed_with_attachments_and_hybrid_interrupt(
        &problem,
        request,
        None,
        Some(&prepared),
        None,
        &mut NeverInterrupt,
    )
    .expect("observation run");
    let attached = result.component_inference.expect("standard errors");
    let diagnostic = attached.residual_noise.as_ref().expect("diagnostic");
    assert_eq!(attached.inference_unit, ComponentInferenceUnit::Observation);
    assert_eq!(attached.independent_units as usize, n);
    assert_eq!(diagnostic.controls, 0);
    assert!(!attached.nuisance_uncertainty_conditioned_away);
    // Dense joint kernel H C_t H on physical rows, with the engine's own
    // leverage and target diagonals, applied to the outcome.
    let (x, p) = super::component_centering::design(&problem, true);
    let xt = dense_transpose(&x, n, p);
    let inverse = dense_inverse(&dense_multiply(&xt, p, n, &x, p), p);
    let bread = dense_multiply(&x, n, p, &inverse, p);
    let bread_t = dense_transpose(&bread, n, p);
    let projection = dense_multiply(&bread, n, p, &xt, n);
    let mut worker_mean = vec![0.0; p];
    let mut firm_mean = vec![0.0; p];
    for cell in 0..problem.cells() {
        let share = problem.cell_target_sum[cell] / problem.target_total;
        worker_mean[problem.cell_worker[cell] as usize] += share;
        let firm = problem.cell_firm[cell] as usize;
        if firm + 1 < problem.firms() {
            firm_mean[problem.workers() + firm] += share;
        }
    }
    let mut targets: [Vec<f64>; 3] = core::array::from_fn(|_| vec![0.0; p * p]);
    for cell in 0..problem.cells() {
        let share = problem.cell_target_sum[cell] / problem.target_total;
        let mut worker: Vec<f64> = worker_mean.iter().map(|v| -v).collect();
        let mut firm: Vec<f64> = firm_mean.iter().map(|v| -v).collect();
        worker[problem.cell_worker[cell] as usize] += 1.0;
        let firm_index = problem.cell_firm[cell] as usize;
        if firm_index + 1 < problem.firms() {
            firm[problem.workers() + firm_index] += 1.0;
        }
        for i in 0..p {
            for j in 0..p {
                targets[0][i * p + j] += share * worker[i] * worker[j];
                targets[1][i * p + j] += share * firm[i] * firm[j];
                targets[2][i * p + j] += 0.5 * share * (worker[i] * firm[j] + firm[i] * worker[j]);
            }
        }
    }
    let mean = problem.outcome.iter().sum::<f64>() / n as f64;
    let centered: Vec<f64> = problem.outcome.iter().map(|y| y - mean).collect();
    for target in 0..3 {
        let mut raw = dense_multiply(
            &dense_multiply(&bread, n, p, &targets[target], p),
            n,
            p,
            &bread_t,
            n,
        );
        for i in 0..n {
            for j in 0..n {
                let maker = f64::from(i == j) - projection[i * n + j];
                raw[i * n + j] -= 0.5
                    * maker
                    * (attached.target_diagonal[target][i] * attached.maker_inverse[i]
                        + attached.target_diagonal[target][j] * attached.maker_inverse[j]);
            }
        }
        let image = dense_multiply(&raw, n, n, &centered, 1);
        let image_mean = image.iter().sum::<f64>() / n as f64;
        for (expected, &actual) in image.iter().zip(&attached.influence[target]) {
            assert_close(expected - image_mean, actual, 2.0e-9);
        }
        // The validated sources hold the observed mean fixed and so leave
        // the image uncentred; the two differ by exactly that constant.
        for (expected, &actual) in image.iter().zip(&reference.influence[target]) {
            assert_close(*expected, actual, 2.0e-9);
        }
    }
    // Squared joint residuals carry the signal term.
    let beta = dense_multiply(
        &inverse,
        p,
        p,
        &dense_multiply(&xt, p, n, &problem.outcome, 1),
        1,
    );
    let fitted = dense_multiply(&x, n, p, &beta, 1);
    let scale = attached.influence_term[0]
        .abs()
        .max(attached.influence_term[4].abs());
    for left in 0..3 {
        for right in 0..3 {
            let index = left * 3 + right;
            let expected = 4.0
                * (0..n)
                    .map(|a| {
                        let residual = problem.outcome[a] - fitted[a];
                        residual
                            * residual
                            * attached.maker_inverse[a]
                            * attached.influence[left][a]
                            * attached.influence[right][a]
                    })
                    .sum::<f64>()
                - 4.0 * diagnostic.residual_noise[index];
            assert!(
                (attached.influence_term[index] - expected).abs() <= 1e-7 * scale,
                "signal {index}: {} versus {expected}",
                attached.influence_term[index]
            );
        }
    }
    for target in 0..4 {
        assert!(attached.covariance[target * 5] > 0.0);
    }
}

// With controls kept in the operator, the point estimate is the joint one and
// the standard error is the fixed-offset one. The engine forms the
// fixed-offset unit geometry beside the joint run: shared leverage moments,
// leave-out adjustments without controls, and one more target pass.
#[test]
fn joint_nuisance_with_controls_reports_the_fixed_offset_standard_errors() {
    let hybrid = super::pooled_component::fixture(false);
    for centering in [Centering::Mean, Centering::None] {
        let route = ModelSolverRoute::Diagonal;
        let point_only = run_nuisance(&hybrid, route, centering, NuisanceMode::Joint, false);
        let joint = run_nuisance(&hybrid, route, centering, NuisanceMode::Joint, true);
        let fixed = run_nuisance(&hybrid, route, centering, NuisanceMode::FixedOffset, true);
        // The attachment leaves the joint point estimate untouched, and the
        // two nuisance modes are different estimators here.
        assert_eq!(point_only.corrected, joint.corrected);
        assert_ne!(joint.corrected, fixed.corrected);
        let probes = options(DeletionMode::Match, NuisanceMode::Joint).probes as usize;
        let (joint, fixed) = (
            joint.component_inference.expect("joint"),
            fixed.component_inference.expect("fixed offset"),
        );
        let diagnostic = joint.residual_noise.as_ref().expect("diagnostic");
        assert_eq!(diagnostic.offset_pass_solves, 2 * probes);
        assert_eq!(diagnostic.controls, 2);
        assert_eq!(
            joint.solve_receipts.len(),
            3 + 4 * DRAWS as usize + 2 * probes
        );
        assert_eq!(fixed.solve_receipts.len(), 3 + 4 * DRAWS as usize);
        assert_eq!(
            fixed
                .residual_noise
                .as_ref()
                .expect("diagnostic")
                .offset_pass_solves,
            0
        );
        assert!(!joint.nuisance_uncertainty_conditioned_away);
        assert_eq!(joint.independent_units, fixed.independent_units);
        for index in 0..16 {
            assert_close(joint.covariance[index], fixed.covariance[index], 1e-7);
        }
        for target in 0..3 {
            for (a, b) in joint.influence[target].iter().zip(&fixed.influence[target]) {
                assert_close(*a, *b, 1e-7);
            }
        }
    }
}

#[test]
fn joint_nuisance_offset_pass_covers_mover_only_units() {
    let problem = fixture(true);
    assert!(!problem.controls.is_empty());
    let joint = mover_only(&problem, NuisanceMode::Joint);
    let fixed = mover_only(&problem, NuisanceMode::FixedOffset);
    assert!(
        joint
            .residual_noise
            .as_ref()
            .expect("diagnostic")
            .offset_pass_solves
            > 0
    );
    assert!(!joint.nuisance_uncertainty_conditioned_away);
    for index in 0..16 {
        assert_close(joint.covariance[index], fixed.covariance[index], 1e-7);
    }
}

// The attachment's own lifetime forecast sets the admission boundary: the
// forecast peak is accepted exactly and one byte less is rejected. The offset
// pass and the control aggregates must enlarge the attachment's share.
#[test]
fn leaveout_residual_attachment_has_exact_memory_boundary() {
    let hybrid = super::pooled_component::fixture(false);
    let prepared = prepare_residual_noise_with_interrupt(
        &hybrid.problem,
        ComponentInferenceUnit::Match,
        ComponentInferenceOptions {
            probes: 4 * DRAWS,
            batch_width: 7,
            ..ComponentInferenceOptions::default()
        },
        StructuredVarianceOptions::default(),
        &mut NeverInterrupt,
    )
    .expect("standard-error preparation");
    let mut attachment = [0_u64; 2];
    for (slot, nuisance) in [NuisanceMode::FixedOffset, NuisanceMode::Joint]
        .into_iter()
        .enumerate()
    {
        let mut estimator = options(DeletionMode::Match, nuisance);
        estimator.memory_limit_bytes = u64::MAX;
        let run = |estimator| {
            run_generic_jla_routed_with_attachments_and_hybrid_interrupt(
                &hybrid.problem,
                routed_options(estimator, ModelSolverRoute::Diagonal),
                None,
                Some(&prepared),
                Some(&hybrid.plan),
                &mut NeverInterrupt,
            )
        };
        let baseline = run(estimator).expect("unlimited run");
        let memory = baseline.receipt.execution.memory;
        let peak = baseline.receipt.peak_forecast_bytes;
        assert_eq!(memory.peak_bytes, peak);
        assert!(memory.component_inference_peak_bytes > 0);
        assert_eq!(
            baseline
                .component_inference
                .as_ref()
                .expect("attachment")
                .peak_forecast_bytes,
            memory.component_inference_peak_bytes
        );
        attachment[slot] = memory.component_inference_peak_bytes;
        estimator.memory_limit_bytes = peak;
        run(estimator).expect("the exact forecast is accepted");
        estimator.memory_limit_bytes = peak - 1;
        let error = run(estimator).expect_err("one byte below the forecast is rejected");
        assert_eq!(error.code, ErrorCode::ResourceLimit);
        assert_eq!(error.phase, "generic_jla_memory");
    }
    // Offset outcome, adjusted residuals, three diagonals, unit geometry,
    // worker-firm coefficients and two receipts per point probe.
    assert!(attachment[1] > attachment[0], "{attachment:?}");
}

/// Covariance of the noise of the control-adjusted outcome on the units,
/// `S = Pi' V Pi + Zc Lam W'W Lam Zc'` with `Pi = I - Zt Lam Zc'`, from
/// control-major aggregates; `V = diag(variance)` without controls.
fn dense_noise_covariance(
    variance: &[f64],
    collapsed: &[Vec<f64>],
    residualized: &[Vec<f64>],
    within: &[Vec<f64>],
    precision: &[f64],
) -> Vec<f64> {
    let n = variance.len();
    let k = collapsed.len();
    let column_major =
        |columns: &[Vec<f64>]| -> Vec<f64> { (0..n * k).map(|i| columns[i % k][i / k]).collect() };
    let (zc, zt, w) = (
        column_major(collapsed),
        column_major(residualized),
        column_major(within),
    );
    // Pi = I - Zt Lam Zc'.
    let lam_zc = dense_multiply(precision, k, k, &dense_transpose(&zc, n, k), n);
    let mut pi = dense_multiply(&zt, n, k, &lam_zc, n);
    for value in &mut pi {
        *value = -*value;
    }
    for a in 0..n {
        pi[a * n + a] += 1.0;
    }
    let mut v_pi = pi.clone();
    for a in 0..n {
        for b in 0..n {
            v_pi[a * n + b] *= variance[a];
        }
    }
    let mut output = dense_multiply(&dense_transpose(&pi, n, n), n, n, &v_pi, n);
    if k > 0 {
        let omega = dense_multiply(&dense_transpose(&w, n, k), k, n, &w, k);
        let middle = dense_multiply(
            &dense_multiply(precision, k, k, &omega, k),
            k,
            k,
            precision,
            k,
        );
        let low = dense_multiply(
            &dense_multiply(&zc, n, k, &middle, k),
            n,
            k,
            &dense_transpose(&zc, n, k),
            n,
        );
        for (value, extra) in output.iter_mut().zip(&low) {
            *value += extra;
        }
    }
    output
}

/// Midranks on (-1, 1), written out independently of the production helper.
fn dense_ranks(values: &[f64]) -> Vec<f64> {
    let n = values.len();
    (0..n)
        .map(|i| {
            let below = values.iter().filter(|&&v| v < values[i]).count() as f64;
            let equal = values.iter().filter(|&&v| v == values[i]).count() as f64;
            2.0 * ((below + 0.5 * (equal + 1.0) - 0.5) / n as f64) - 1.0
        })
        .collect()
}

/// Least squares on the leading `terms` columns through the dense helpers.
fn dense_fit(columns: &[Vec<f64>], terms: usize, outcome: &[f64]) -> Vec<f64> {
    let n = outcome.len();
    let mut x = vec![0.0; n * terms];
    for row in 0..n {
        for term in 0..terms {
            x[row * terms + term] = columns[term][row];
        }
    }
    let xt = dense_transpose(&x, n, terms);
    let inverse = dense_inverse(&dense_multiply(&xt, terms, n, &x, terms), terms);
    let coefficient = dense_multiply(
        &inverse,
        terms,
        terms,
        &dense_multiply(&xt, terms, n, outcome, 1),
        1,
    );
    dense_multiply(&x, n, terms, &coefficient, 1)
}

fn run_repeat(
    hybrid: &vckss_core::stayer_hybrid::PreparedExactStayerHybrid,
    route: ModelSolverRoute,
    nuisance: NuisanceMode,
) -> vckss_core::generic_jla::GenericJlaResult {
    let prepared = prepare_repeat_measurement_with_interrupt(
        &hybrid.problem,
        ComponentInferenceUnit::Match,
        ComponentInferenceOptions {
            probes: 4 * DRAWS,
            ..ComponentInferenceOptions::default()
        },
        StructuredVarianceOptions::default(),
        &mut NeverInterrupt,
    )
    .expect("repeat-measurement preparation");
    assert!(prepared.leave_out_residual && prepared.repeat_measurement);
    run_generic_jla_routed_with_attachments_and_hybrid_interrupt(
        &hybrid.problem,
        routed_options(
            GenericJlaOptions {
                centering: Centering::Mean,
                ..options(DeletionMode::Match, nuisance)
            },
            route,
        ),
        None,
        Some(&prepared),
        Some(&hybrid.plan),
        &mut NeverInterrupt,
    )
    .expect("pooled repeat-measurement run")
}

/// Thirty movers with two cells of one to three observations each and ten
/// stayers with two to four observations, on six firms.
fn observation_repeat_fixture() -> CompressedProblem {
    let (worker, firm, outcome, _) = observation_repeat_columns();
    observation_repeat_problem(worker, firm, outcome, Vec::new())
}

/// Worker, firm, outcome and two controls that vary within cells, in input
/// order.
fn observation_repeat_columns() -> (Vec<u64>, Vec<u64>, Vec<f64>, Vec<Vec<f64>>) {
    let mut worker = Vec::new();
    let mut firm = Vec::new();
    let mut outcome = Vec::new();
    let mut push = |w: u64, f: u64, copies: u64| {
        for copy in 0..copies {
            let row = worker.len() as u64;
            worker.push(10 + w);
            firm.push(100 + f);
            outcome.push(
                0.31 * (w % 7) as f64 - 0.22 * f as f64
                    + ((row * 11 + copy * 5) % 17) as f64 / 13.0
                    + ((row * row) % 7) as f64 / 29.0,
            );
        }
    };
    for mover in 0..30_u64 {
        let first = mover % 6;
        let second = (first + 1 + mover / 6) % 6;
        push(mover, first, 1 + mover % 3);
        push(mover, second, 1 + (mover / 3) % 3);
    }
    for stayer in 0..10_u64 {
        push(30 + stayer, stayer % 6, 2 + stayer % 3);
    }
    let rows = worker.len() as u64;
    let controls = vec![
        (0..rows)
            .map(|row| ((row * 13) % 19) as f64 / 7.0 + (row % 4) as f64 / 3.0)
            .collect(),
        (0..rows)
            .map(|row| ((row * 5 + 3) % 23) as f64 / 11.0 - ((row * row) % 5) as f64 / 4.0)
            .collect(),
    ];
    (worker, firm, outcome, controls)
}

fn observation_repeat_problem(
    worker: Vec<u64>,
    firm: Vec<u64>,
    outcome: Vec<f64>,
    controls: Vec<Vec<f64>>,
) -> CompressedProblem {
    let rows = worker.len();
    CanonicalInput::from_validated(
        InputColumns {
            worker,
            firm,
            deletion: (0..rows as u64).map(|row| 1_000 + row).collect(),
            outcome,
            frequency: vec![1; rows],
            target_weight: vec![1.0; rows],
            controls,
        }
        .validate()
        .expect("observation fixture validates"),
    )
    .expect("observation fixture canonicalizes")
    .compress(&vec![true; rows])
    .expect("observation fixture compresses")
}

fn run_observation_repeat(
    problem: &CompressedProblem,
) -> vckss_core::generic_jla::GenericJlaResult {
    run_observation_repeat_with(problem, false)
}

fn run_observation_repeat_with(
    problem: &CompressedProblem,
    leading: bool,
) -> vckss_core::generic_jla::GenericJlaResult {
    let mut estimator = options(DeletionMode::Observation, NuisanceMode::Joint);
    estimator.probes = 200;
    estimator.centering = Centering::Mean;
    let prepared = prepare_repeat_measurement_with_interrupt(
        problem,
        ComponentInferenceUnit::Observation,
        ComponentInferenceOptions {
            probes: 4 * DRAWS,
            reference_distribution: if leading {
                vckss_core::component_inference::ComponentReferenceDistribution::Q1
            } else {
                vckss_core::component_inference::ComponentReferenceDistribution::Q0
            },
            spectrum_iterations: 60,
            spectrum_tolerance: 1.0e-9,
            ..ComponentInferenceOptions::default()
        },
        StructuredVarianceOptions::default(),
        &mut NeverInterrupt,
    )
    .expect("repeat-measurement preparation");
    assert!(prepared.leave_out_residual && prepared.repeat_measurement);
    run_generic_jla_routed_with_attachments_and_hybrid_interrupt(
        problem,
        routed_options(estimator, ModelSolverRoute::Diagonal),
        None,
        Some(&prepared),
        None,
        &mut NeverInterrupt,
    )
    .expect("observation run")
}

/// Dense expectations for repeat-measurement standard errors on physical
/// rows: the worker-firm kernels `H C_t H` with the engine's own leverage and
/// target diagonals, applied to `outcome`; the sample variance of every cell
/// of two or more rows; the common variance of the single rows of movers; the
/// pair term in all entries; and the assembled covariance.
fn assert_observation_repeat(
    problem: &CompressedProblem,
    outcome: &[f64],
    attached: &vckss_core::component_inference::ComponentInferenceResult,
    tolerance: f64,
) {
    assert_observation_repeat_with(problem, outcome, &[], attached, tolerance);
}

/// The same with the first-order term of the estimated coefficients of
/// `controls`, `outcome` being the outcome net of their fitted index. Every
/// observation is a unit, so the within-unit products vanish and the term
/// acts through the starred influences `Pi g = g - Zt Lam Z' g`.
fn assert_observation_repeat_with(
    problem: &CompressedProblem,
    outcome: &[f64],
    controls: &[Vec<f64>],
    attached: &vckss_core::component_inference::ComponentInferenceResult,
    tolerance: f64,
) {
    let n = outcome.len();
    let k = controls.len();
    let diagnostic = attached.residual_noise.as_ref().expect("diagnostic");
    let shape = diagnostic
        .repeat
        .as_ref()
        .expect("repeat-measurement shape");
    assert_eq!(attached.inference_unit, ComponentInferenceUnit::Observation);
    assert_eq!(attached.independent_units as usize, n);
    assert_eq!(attached.psd_cleanup, 0.0);
    assert_eq!(diagnostic.controls, k);
    assert_eq!(attached.nuisance_uncertainty_conditioned_away, k == 0);
    assert_eq!(shape.control_terms.is_some(), k > 0);
    // No within-unit product, so no extra normal: the unit draws alone.
    if shape.leading.is_none() {
        assert_eq!(
            attached.counter_atoms,
            n as u64 * u64::from(diagnostic.draws)
        );
        assert_eq!(attached.counter_words, 2 * attached.counter_atoms);
    }

    let (x, p) = super::component_centering::design(problem, false);
    let xt = dense_transpose(&x, n, p);
    let inverse = dense_inverse(&dense_multiply(&xt, p, n, &x, p), p);
    // Controls residualized on the two-way effects, and their precision.
    let projection_fe = dense_multiply(&dense_multiply(&x, n, p, &inverse, p), n, p, &xt, n);
    let residualized: Vec<Vec<f64>> = controls
        .iter()
        .map(|z| {
            let fitted = dense_multiply(&projection_fe, n, n, z, 1);
            (0..n).map(|a| z[a] - fitted[a]).collect()
        })
        .collect();
    let mut information = vec![0.0; k * k];
    for j in 0..k {
        for l in 0..k {
            information[j * k + l] = (0..n)
                .map(|a| residualized[j][a] * residualized[l][a])
                .sum();
        }
    }
    let precision = if k > 0 {
        dense_inverse(&information, k)
    } else {
        Vec::new()
    };
    // Pi h = h - Zt Lam Z' h.
    let starred = |h: &[f64]| -> Vec<f64> {
        let slope: Vec<f64> = (0..k)
            .map(|j| (0..n).map(|a| controls[j][a] * h[a]).sum())
            .collect();
        let u: Vec<f64> = (0..k)
            .map(|j| (0..k).map(|l| precision[j * k + l] * slope[l]).sum())
            .collect();
        (0..n)
            .map(|a| h[a] - (0..k).map(|j| residualized[j][a] * u[j]).sum::<f64>())
            .collect()
    };
    let bread = dense_multiply(&x, n, p, &inverse, p);
    let bread_t = dense_transpose(&bread, n, p);
    let projection = dense_multiply(&bread, n, p, &xt, n);
    let mut worker_mean = vec![0.0; p];
    let mut firm_mean = vec![0.0; p];
    for cell in 0..problem.cells() {
        let share = problem.cell_target_sum[cell] / problem.target_total;
        worker_mean[problem.cell_worker[cell] as usize] += share;
        let firm = problem.cell_firm[cell] as usize;
        if firm + 1 < problem.firms() {
            firm_mean[problem.workers() + firm] += share;
        }
    }
    let mut targets: [Vec<f64>; 3] = core::array::from_fn(|_| vec![0.0; p * p]);
    for cell in 0..problem.cells() {
        let share = problem.cell_target_sum[cell] / problem.target_total;
        let mut worker: Vec<f64> = worker_mean.iter().map(|v| -v).collect();
        let mut firm: Vec<f64> = firm_mean.iter().map(|v| -v).collect();
        worker[problem.cell_worker[cell] as usize] += 1.0;
        let firm_index = problem.cell_firm[cell] as usize;
        if firm_index + 1 < problem.firms() {
            firm[problem.workers() + firm_index] += 1.0;
        }
        for i in 0..p {
            for j in 0..p {
                targets[0][i * p + j] += share * worker[i] * worker[j];
                targets[1][i * p + j] += share * firm[i] * firm[j];
                targets[2][i * p + j] += 0.5 * share * (worker[i] * firm[j] + firm[i] * worker[j]);
            }
        }
    }
    let mean = outcome.iter().sum::<f64>() / n as f64;
    let centered: Vec<f64> = outcome.iter().map(|y| y - mean).collect();
    let center = |matrix: &mut Vec<f64>| {
        // H M H with H = I - 11'/n.
        let row_mean: Vec<f64> = (0..n)
            .map(|i| matrix[i * n..(i + 1) * n].iter().sum::<f64>() / n as f64)
            .collect();
        let column_mean: Vec<f64> = (0..n)
            .map(|j| (0..n).map(|i| matrix[i * n + j]).sum::<f64>() / n as f64)
            .collect();
        let grand = row_mean.iter().sum::<f64>() / n as f64;
        for i in 0..n {
            for j in 0..n {
                matrix[i * n + j] += grand - row_mean[i] - column_mean[j];
            }
        }
    };
    let kernel: [Vec<f64>; 3] = core::array::from_fn(|target| {
        let mut raw = dense_multiply(
            &dense_multiply(&bread, n, p, &targets[target], p),
            n,
            p,
            &bread_t,
            n,
        );
        for i in 0..n {
            for j in 0..n {
                let maker = f64::from(i == j) - projection[i * n + j];
                raw[i * n + j] -= 0.5
                    * maker
                    * (attached.target_diagonal[target][i] * attached.maker_inverse[i]
                        + attached.target_diagonal[target][j] * attached.maker_inverse[j]);
            }
        }
        center(&mut raw);
        raw
    });
    let influence: [Vec<f64>; 3] =
        core::array::from_fn(|t| dense_multiply(&kernel[t], n, n, outcome, 1));
    let star: Vec<Vec<f64>> = influence.iter().map(|g| starred(g)).collect();
    for target in 0..3 {
        let largest = influence[target]
            .iter()
            .fold(0.0_f64, |a, b| a.max(b.abs()));
        for (expected, &actual) in influence[target].iter().zip(&attached.influence[target]) {
            assert!(
                (expected - actual).abs() <= tolerance * largest,
                "influence {target}: {actual} versus {expected}"
            );
        }
    }

    // Variances. A cell with two or more rows is a set; the single rows of
    // movers share the mean leave-out product of all rows of movers, because
    // there are fewer than a hundred of them.
    let beta = dense_multiply(&inverse, p, p, &dense_multiply(&xt, p, n, outcome, 1), 1);
    let fitted = dense_multiply(&x, n, p, &beta, 1);
    let proxy: Vec<f64> = (0..n)
        .map(|a| centered[a] * attached.maker_inverse[a] * (outcome[a] - fitted[a]))
        .collect();
    let mut firms_of = vec![0; problem.workers()];
    for cell in 0..problem.cells() {
        firms_of[problem.cell_worker[cell] as usize] += 1;
    }
    let mover = |row: usize| firms_of[problem.row_worker[row] as usize] > 1;
    let mover_rows: Vec<usize> = (0..n).filter(|&row| mover(row)).collect();
    let common = mover_rows.iter().map(|&row| proxy[row]).sum::<f64>() / mover_rows.len() as f64;
    assert!(common > 0.0);
    let mut variance = vec![0.0; n];
    let mut adjustment = [0.0; 9];
    let (mut sets, mut set_units, mut singles, mut mover_sets) = (0_u64, 0_u64, 0_u64, 0);
    for cell in 0..problem.cells() {
        let units: Vec<usize> = (0..n)
            .filter(|&row| problem.row_cell[row] as usize == cell)
            .collect();
        if units.len() == 1 {
            assert!(mover(units[0]));
            variance[units[0]] = common;
            singles += 1;
            continue;
        }
        let count = units.len() as f64;
        let cell_mean = units.iter().map(|&u| outcome[u]).sum::<f64>() / count;
        let sample = units
            .iter()
            .map(|&u| (outcome[u] - cell_mean).powi(2))
            .sum::<f64>()
            / (count - 1.0);
        let weight: [f64; 3] = core::array::from_fn(|target| {
            if mover(units[0]) {
                units
                    .iter()
                    .map(|&u| attached.target_diagonal[target][u] * attached.maker_inverse[u])
                    .sum::<f64>()
                    / count
            } else if target == 0 {
                units
                    .iter()
                    .map(|&u| attached.target_diagonal[0][u])
                    .sum::<f64>()
                    / (count - 1.0)
            } else {
                0.0
            }
        });
        let factor = 4.0 * sample * sample * (count - 2.0) * (count - 1.0) / (count + 1.0);
        for left in 0..3 {
            for right in 0..3 {
                adjustment[left * 3 + right] += factor * weight[left] * weight[right];
            }
        }
        for &u in &units {
            variance[u] = sample;
        }
        sets += 1;
        set_units += units.len() as u64;
        mover_sets += usize::from(mover(units[0]));
    }
    assert_eq!((shape.stayers, shape.stayer_units), (sets, set_units));
    assert_eq!(singles + set_units, n as u64);
    assert!(singles >= 10 && mover_sets >= 20 && sets as usize > mover_sets);
    assert_eq!(diagnostic.working_terms, [1, 1]);
    assert!(adjustment[0] > 0.0 && adjustment[4] > 0.0);
    assert!(
        (adjustment[0] - shape.stayer_pair_term).abs() <= 10.0 * tolerance * adjustment[0],
        "pair term {} versus {}",
        shape.stayer_pair_term,
        adjustment[0]
    );

    let scale_value = attached.trace_term[0]
        .abs()
        .max(attached.trace_term[4].abs())
        .max(attached.primitive_covariance[0].abs());
    // tr(C_s S C_t S) with the covariance S of the control-adjusted noise;
    // every observation is a unit, so W = 0 and S = Pi' V Pi.
    let noise_covariance = dense_noise_covariance(
        &variance,
        controls,
        &residualized,
        &vec![vec![0.0; n]; k],
        &precision,
    );
    let product: Vec<Vec<f64>> = (0..3)
        .map(|t| dense_multiply(&kernel[t], n, n, &noise_covariance, n))
        .collect();
    for left in 0..3 {
        for right in 0..3 {
            let index = left * 3 + right;
            let mut trace = 0.0;
            for u in 0..n {
                for v in 0..n {
                    trace += product[left][u * n + v] * product[right][v * n + u];
                }
            }
            assert!(
                (attached.trace_term[index] - 2.0 * trace).abs()
                    <= 6.0 * attached.trace_mcse[index] + 100.0 * tolerance * scale_value,
                "trace {index}: {} versus {}",
                attached.trace_term[index],
                2.0 * trace
            );
            // With the engine's own draws for the noise means, the rest is
            // exact; the pair term of the sets of movers reaches every entry.
            let expected = match shape.control_terms.as_ref() {
                None => {
                    let leading = (0..n)
                        .map(|u| 4.0 * variance[u] * influence[left][u] * influence[right][u])
                        .sum::<f64>();
                    leading - attached.trace_term[index] + adjustment[index]
                }
                Some(terms) => {
                    let unit = (0..n)
                        .map(|u| 4.0 * variance[u] * star[left][u] * star[right][u])
                        .sum::<f64>();
                    assert!(
                        (terms.linear_unit[index] - unit).abs() <= 100.0 * tolerance * scale_value,
                        "unit {index}: {} versus {unit}",
                        terms.linear_unit[index]
                    );
                    // No unit holds two observations: no within-unit part.
                    assert_eq!(terms.linear_cross[index], 0.0);
                    assert_eq!(terms.linear_within[index], 0.0);
                    unit - attached.trace_term[index] + adjustment[index]
                }
            };
            assert!(
                (attached.primitive_covariance[index] - expected).abs()
                    <= 100.0 * tolerance * scale_value,
                "covariance {index}: {} versus {expected}",
                attached.primitive_covariance[index]
            );
            assert!(adjustment[index].abs() > 1e-6 * scale_value || index == 0);
        }
    }

    // Leading directions and the q = 1 decomposition, when requested.
    let Some(leading) = shape.leading.as_ref() else {
        return;
    };
    assert_eq!(leading.influence_solves, 4);
    assert_eq!(
        leading.kernel_solves as usize,
        4 * diagnostic.draws as usize
    );
    let combine = |values: &[Vec<f64>; 3], target: usize| -> Vec<f64> {
        if target < 3 {
            values[target].clone()
        } else {
            (0..values[0].len())
                .map(|i| values[0][i] + values[1][i] + 2.0 * values[2][i])
                .collect()
        }
    };
    let mut steps = 0;
    let mut score_change = 0.0_f64;
    for target in 0..4 {
        let spectrum = &leading.spectrum[target];
        let q1 = &leading.q1[target];
        let lambda = spectrum.leading_eigenvalue;
        steps += spectrum.iterations;
        assert!(spectrum.certified, "target {target} did not converge");
        // The direction is an eigenvector of B_t = X S^-1 A_t S^-1 X' with the
        // eigenvalue of largest magnitude. The engine does not return the
        // weights, so they are recovered from the eigen-equation by a dense
        // power iteration on B_t^2, which shares nothing with the Lanczos run.
        let matrix = combine(&targets, target);
        let b = dense_multiply(&dense_multiply(&bread, n, p, &matrix, p), n, p, &bread_t, n);
        let mut mode: Vec<f64> = (0..n).map(|i| ((i * 37 + 11) % 23) as f64 - 11.0).collect();
        for _ in 0..600 {
            let image = dense_multiply(&b, n, n, &dense_multiply(&b, n, n, &mode, 1), 1);
            let norm = image.iter().map(|v| v * v).sum::<f64>().sqrt();
            mode = image.iter().map(|v| v / norm).collect();
        }
        let image = dense_multiply(&b, n, n, &mode, 1);
        let dense_lambda: f64 = mode.iter().zip(&image).map(|(a, b)| a * b).sum();
        // All nonzero eigenvalues of the sum's target are equal, so any of
        // its directions is a leading one; compare the eigenvalue only.
        assert!(
            (lambda - dense_lambda).abs() <= 1.0e-6 * dense_lambda.abs().max(tolerance),
            "target {target}: eigenvalue {lambda} versus {dense_lambda}"
        );
        assert!(spectrum.second_eigenvalue.abs() <= lambda.abs() * (1.0 + 1.0e-9));
        if target == 3 {
            continue;
        }
        let largest = mode.iter().fold(0.0_f64, |a, b| a.max(b * b));
        assert!((spectrum.maximum_mode_weight_squared - largest).abs() <= 1.0e-5);
        // The leading score fixes the sign of the dense direction.
        let score: f64 = mode.iter().zip(&centered).map(|(a, b)| a * b).sum();
        if score * q1.leading_score < 0.0 {
            for value in &mut mode {
                *value = -*value;
            }
        }
        let score = score.abs() * q1.leading_score.signum();
        let loose = 2.0e-4;
        assert!((q1.leading_score - score).abs() <= loose * score.abs().max(1.0e-3));
        let leave_out: f64 = (0..n).map(|a| mode[a] * mode[a] * proxy[a]).sum();
        let plain_fitted: f64 = (0..n).map(|a| mode[a] * mode[a] * variance[a]).sum();
        // The score's variances use the starred direction Pi d; the
        // identity's leave-out correction keeps the weights d^2.
        let mode_star = starred(&mode);
        let fitted: f64 = (0..n)
            .map(|a| mode_star[a] * mode_star[a] * variance[a])
            .sum();
        let leave_out_score: f64 = (0..n).map(|a| mode_star[a] * mode_star[a] * proxy[a]).sum();
        let leading_variance = fitted.max(leave_out_score);
        assert!((q1.leading_variance_correction - leave_out).abs() <= loose * fitted);
        assert!((leading.leading_variance_fitted[target] - fitted).abs() <= loose * fitted);
        assert!((q1.leading_variance - leading_variance).abs() <= loose * fitted);
        score_change = score_change.max((fitted - plain_fitted).abs() / fitted);
        // Remainder kernel K_t - lambda H K1 H with K1 = w w' - (D1 M + M D1)/2.
        let diagonal = combine(&attached.target_diagonal, target);
        let mut k1 = vec![0.0; n * n];
        for i in 0..n {
            for j in 0..n {
                let maker = f64::from(i == j) - projection[i * n + j];
                k1[i * n + j] = mode[i] * mode[j]
                    - 0.5
                        * maker
                        * (mode[i] * mode[i] * attached.maker_inverse[i]
                            + mode[j] * mode[j] * attached.maker_inverse[j]);
            }
        }
        center(&mut k1);
        let full = combine(&kernel, target);
        let remainder: Vec<f64> = (0..n * n).map(|i| full[i] - dense_lambda * k1[i]).collect();
        let influence_r = dense_multiply(&remainder, n, n, outcome, 1);
        let point: f64 = (0..n)
            .map(|a| centered[a] * dense_multiply(&full, n, n, outcome, 1)[a])
            .sum();
        let direct: f64 = (0..n).map(|a| centered[a] * influence_r[a]).sum();
        let recentred = point - dense_lambda * (score * score - leave_out);
        let scale = point.abs().max(1.0e-6);
        assert!((direct - recentred).abs() <= 1.0e-8 * scale.max(1.0));
        assert!((q1.point_estimate - point).abs() <= loose * scale);
        assert!((q1.remainder_estimate - recentred).abs() <= loose * scale);
        assert!(q1.remainder_identity_error <= 1.0e-7);
        let influence_star = starred(&influence_r);
        let linear: f64 = (0..n)
            .map(|a| 4.0 * variance[a] * influence_star[a] * influence_star[a])
            .sum();
        assert!((q1.remainder_influence_variance - linear).abs() <= loose * linear);
        let cross: f64 = (0..n)
            .map(|a| 2.0 * mode_star[a] * variance[a] * influence_star[a])
            .sum();
        assert!(
            (q1.leading_remainder_covariance - cross).abs() <= loose * (fitted * linear).sqrt()
        );
        // tr(C_r S C_r S), with the same S as the high-rank trace.
        let remainder_product = dense_multiply(&remainder, n, n, &noise_covariance, n);
        let mut trace = 0.0;
        for u in 0..n {
            for v in 0..n {
                trace += remainder_product[u * n + v] * remainder_product[v * n + u];
            }
        }
        assert!(
            (q1.remainder_trace_variance - 2.0 * trace).abs()
                <= 6.0 * q1.remainder_trace_mcse + loose * linear,
            "target {target}: remainder trace {} versus {}",
            q1.remainder_trace_variance,
            2.0 * trace
        );
        // Pair term of the remainder, set by set.
        let mut pair = 0.0;
        for cell in 0..problem.cells() {
            let units: Vec<usize> = (0..n)
                .filter(|&row| problem.row_cell[row] as usize == cell)
                .collect();
            if units.len() < 2 {
                continue;
            }
            let count = units.len() as f64;
            let cell_mean = units.iter().map(|&u| outcome[u]).sum::<f64>() / count;
            let sample = units
                .iter()
                .map(|&u| (outcome[u] - cell_mean).powi(2))
                .sum::<f64>()
                / (count - 1.0);
            let weight = if mover(units[0]) {
                units
                    .iter()
                    .map(|&u| {
                        (diagonal[u] - dense_lambda * mode[u] * mode[u]) * attached.maker_inverse[u]
                    })
                    .sum::<f64>()
                    / count
            } else {
                units
                    .iter()
                    .map(|&u| {
                        (if target == 0 { diagonal[u] } else { 0.0 })
                            - dense_lambda * mode[u] * mode[u]
                    })
                    .sum::<f64>()
                    / (count - 1.0)
            };
            pair += 4.0 * weight * weight * sample * sample * (count - 2.0) * (count - 1.0)
                / (count + 1.0);
        }
        assert!(
            (leading.remainder_pair_term[target] - pair).abs() <= loose * pair.abs().max(1.0e-12)
        );
        let remainder_variance = linear - q1.remainder_trace_variance + pair;
        assert!((q1.remainder_variance - remainder_variance).abs() <= loose * linear);
        if q1.status == vckss_core::component_inference::ComponentQ1Status::Computed {
            let correlation = cross / (leading_variance * remainder_variance).sqrt();
            let curvature = 2.0 * dense_lambda.abs() * leading_variance
                / (remainder_variance * (1.0 - correlation * correlation)).sqrt();
            assert!((q1.curvature - curvature).abs() <= 10.0 * loose * curvature);
            // The interval contains the estimate, and its critical value is
            // the normal one or larger.
            assert!(q1.critical_value >= 1.95 && q1.critical_value <= 2.5);
            assert!(q1.confidence_lower < point && point < q1.confidence_upper);
            let share = (dense_lambda * fitted).powi(2) / (0.5 * attached.trace_term[target * 4]);
            assert!((spectrum.leading_share - share).abs() <= 10.0 * loose * share);
        }
    }
    assert_eq!(leading.direction_solves, steps);
    // The control term moves the score's fitted variance of some target.
    println!("largest relative change of the fitted score variance: {score_change:e}");
    assert_eq!(score_change > 1.0e-6, k > 0, "{score_change}");
    assert_eq!(
        attached.solve_receipts.len(),
        3 + 4 * diagnostic.draws as usize
            + diagnostic.offset_pass_solves
            + (leading.direction_solves + leading.influence_solves + leading.kernel_solves)
                as usize
    );
}

#[test]
fn repeat_measurement_on_observation_units_matches_dense_expectations() {
    let problem = observation_repeat_fixture();
    let attached = run_observation_repeat(&problem)
        .component_inference
        .expect("standard errors");
    assert_eq!(attached.solve_receipts.len(), 3 + 4 * DRAWS as usize);
    assert!(attached.nuisance_uncertainty_conditioned_away);
    assert_observation_repeat(&problem, &problem.outcome, &attached, 2.0e-9);
}

// With jointly estimated controls the rows of one cell differ in their
// regressors. The repeat-measurement variances are those of the outcome net
// of the fitted control index, formed beside the joint point estimate by the
// offset pass, and the estimation of the control coefficients enters to first
// order. The reference uses the dense control coefficients and the run the
// solver's, so the two agree to the solver's accuracy, far inside what a
// wrong geometry would give. One and two controls, with and without the
// leading directions.
#[test]
fn repeat_measurement_on_observation_units_include_joint_controls() {
    for (controls, leading) in [(2, false), (2, true), (1, true)] {
        let (worker, firm, outcome, mut columns) = observation_repeat_columns();
        columns.truncate(controls);
        let problem = observation_repeat_problem(worker, firm, outcome, columns);
        let n = problem.outcome.len();
        let k = problem.controls.len();
        assert_eq!(k, controls);
        let (x, p) = super::component_centering::design(&problem, true);
        let xt = dense_transpose(&x, n, p);
        let beta = dense_multiply(
            &dense_inverse(&dense_multiply(&xt, p, n, &x, p), p),
            p,
            p,
            &dense_multiply(&xt, p, n, &problem.outcome, 1),
            1,
        );
        let net: Vec<f64> = (0..n)
            .map(|row| {
                problem.outcome[row]
                    - (0..k)
                        .map(|j| problem.controls[j][row] * beta[p - k + j])
                        .sum::<f64>()
            })
            .collect();
        let result = run_observation_repeat_with(&problem, leading);
        let attached = result.component_inference.expect("standard errors");
        let diagnostic = attached.residual_noise.as_ref().expect("diagnostic");
        let probes = 200;
        assert_eq!(diagnostic.offset_pass_solves, 2 * probes);
        let leading_solves = diagnostic
            .repeat
            .as_ref()
            .and_then(|shape| shape.leading.as_ref())
            .map_or(0, |value| {
                (value.direction_solves + value.influence_solves + value.kernel_solves) as usize
            });
        // The control term adds no solve.
        assert_eq!(
            attached.solve_receipts.len(),
            3 + 4 * DRAWS as usize + 2 * probes + leading_solves
        );
        assert_observation_repeat_with(&problem, &net, &problem.controls, &attached, 1.0e-5);
        // The control term matters: without it the same checks fail.
        let raw = std::panic::catch_unwind(|| {
            assert_observation_repeat_with(&problem, &net, &[], &attached, 1.0e-5);
        });
        assert!(raw.is_err());
    }
}

/// The pooled fixture with its first `controls` controls: thirty-two stayers
/// of four observations and sixty-four mover matches of two stored rows with
/// frequencies one to three, whose controls vary inside the match.
fn pooled_repeat_fixture(controls: usize) -> vckss_core::stayer_hybrid::PreparedExactStayerHybrid {
    let mut mover = super::grouped_component_inference_fixture(8);
    mover.controls.truncate(controls);
    let mut stayers = vckss_core::stayer_hybrid::StayerAugmentationInput {
        firm: Vec::new(),
        worker: Vec::new(),
        outcome: Vec::new(),
        frequency: Vec::new(),
        target_weight: Vec::new(),
        controls: vec![Vec::new(), Vec::new()],
    };
    for worker in 0..32_u64 {
        for time in 0..4_u64 {
            stayers.worker.push(worker + 1);
            stayers.firm.push(worker % 8 + 1);
            stayers.frequency.push(1 + time % 2);
            stayers.target_weight.push(1.0);
            stayers
                .outcome
                .push(0.17 * worker as f64 + (time as f64 - 1.5) * 0.4);
            stayers.controls[0].push(time as f64 * 0.23 + (worker % 3) as f64);
            stayers.controls[1].push(((worker * 7 + time * 3) % 11) as f64 / 7.0);
        }
    }
    stayers.controls.truncate(controls);
    vckss_core::stayer_hybrid::prepare_exact_stayer_hybrid(&mover, stayers)
        .expect("hybrid preparation")
}

/// Repeat-measurement standard errors under match deletion against a dense
/// reconstruction: the variances, the pair term, the control aggregates with
/// the linear block of the control term (unit, cross and within parts), and
/// the assembled covariance with the engine's own pseudo-noise means.
#[test]
fn repeat_measurement_standard_errors_match_dense_expectations() {
    let hybrid = super::pooled_component::fixture(false);
    let point = run(&hybrid, ModelSolverRoute::Diagonal, false);
    let result = run_repeat(
        &hybrid,
        ModelSolverRoute::Diagonal,
        NuisanceMode::FixedOffset,
    );
    for (a, b) in components(point.corrected)
        .into_iter()
        .zip(components(result.corrected))
    {
        assert!((a - b).abs() <= 1e-12 * a.abs().max(1.0), "{a} versus {b}");
    }
    let attached = result
        .component_inference
        .expect("attached standard errors");
    assert_match_repeat(&hybrid, &attached, 0);
    // One control, and the joint nuisance mode with its offset pass.
    let one = pooled_repeat_fixture(1);
    let attached = run_repeat(&one, ModelSolverRoute::Diagonal, NuisanceMode::FixedOffset)
        .component_inference
        .expect("one control");
    assert_match_repeat(&one, &attached, 0);
    let probes = options(DeletionMode::Match, NuisanceMode::Joint).probes as usize;
    let attached = run_repeat(&hybrid, ModelSolverRoute::Diagonal, NuisanceMode::Joint)
        .component_inference
        .expect("joint nuisance");
    assert_match_repeat(&hybrid, &attached, 2 * probes);
}

fn assert_match_repeat(
    hybrid: &vckss_core::stayer_hybrid::PreparedExactStayerHybrid,
    attached: &vckss_core::component_inference::ComponentInferenceResult,
    offset_solves: usize,
) {
    let k = hybrid.problem.controls.len();
    let diagnostic = attached.residual_noise.as_ref().expect("diagnostic");
    let shape = diagnostic
        .repeat
        .as_ref()
        .expect("repeat-measurement shape");
    assert_eq!(diagnostic.offset_pass_solves, offset_solves);
    assert_eq!(
        attached.solve_receipts.len(),
        3 + 4 * DRAWS as usize + offset_solves
    );
    assert_eq!((diagnostic.draws, diagnostic.controls), (DRAWS, k));
    // The control coefficients' estimation enters, and the result says so.
    assert_eq!(attached.nuisance_uncertainty_conditioned_away, k == 0);
    assert_eq!(attached.psd_cleanup, 0.0);

    let dense = dense_units(hybrid, attached);
    let (rows, movers) = (dense.rows, dense.movers);
    let residual = dense_multiply(&dense.maker, rows, rows, &dense.y, 1);
    let influence: [Vec<f64>; 3] =
        core::array::from_fn(|t| dense_multiply(&dense.kernel[t], rows, rows, &dense.y, 1));
    let proxy: Vec<f64> = (0..rows)
        .map(|a| dense.y[a] * attached.maker_inverse[a] * residual[a])
        .collect();

    // Movers: the two steps written out with the dense helpers.
    let permitted = |limit: usize| (movers / 100).clamp(1, limit);
    let lever = dense_ranks(&attached.leverage[..movers]);
    let mass = dense_ranks(&dense.mass[..movers]);
    let scale_columns = vec![
        vec![1.0; movers],
        lever.clone(),
        mass.clone(),
        lever.iter().map(|v| v * v).collect(),
        mass.iter().map(|v| v * v).collect(),
    ];
    let mean = proxy[..movers].iter().sum::<f64>() / movers as f64;
    let scale: Vec<f64> = dense_fit(&scale_columns, permitted(5), &proxy[..movers])
        .into_iter()
        .map(|value| value.max(0.05 * mean))
        .collect();
    let standard = |values: &[f64]| -> Vec<f64> {
        let total = dense.mass.iter().sum::<f64>();
        let mean = (0..rows).map(|a| dense.mass[a] * values[a]).sum::<f64>() / total;
        let deviation = ((0..rows)
            .map(|a| dense.mass[a] * (values[a] - mean).powi(2))
            .sum::<f64>()
            / total)
            .sqrt();
        values
            .iter()
            .map(|v| ((v - mean) / deviation).clamp(-3.0, 3.0))
            .collect()
    };
    let a = standard(&dense.worker_effect);
    let f = standard(&dense.firm_effect);
    let profile_columns: Vec<Vec<f64>> = (0..6)
        .map(|term| {
            (0..movers)
                .map(|u| scale[u] * [1.0, a[u], a[u] * a[u], f[u], f[u] * f[u], a[u] * f[u]][term])
                .collect()
        })
        .collect();
    let mut scales: Vec<f64> = (0..rows)
        .map(|u| residual[u] * residual[u] * attached.maker_inverse[u])
        .collect();
    scales.sort_by(f64::total_cmp);
    let floor = StructuredVarianceOptions::default().positivity_multiplier * scales[rows / 2];
    let mut variance: Vec<f64> = dense_fit(&profile_columns, permitted(6), &proxy[..movers])
        .into_iter()
        .map(|value| value.max(floor))
        .collect();
    variance.resize(rows, 0.0);
    assert_eq!(diagnostic.working_terms, [permitted(5), permitted(6)]);

    // Stayers: each worker's own sample variance, and the pair term.
    let mut pair_term = 0.0;
    let mut stayers = 0;
    let mut seen = std::collections::BTreeSet::new();
    for first in movers..rows {
        if !seen.insert(dense.worker[first]) {
            continue;
        }
        let units: Vec<usize> = (movers..rows)
            .filter(|&u| dense.worker[u] == dense.worker[first])
            .collect();
        let count = units.len() as f64;
        let mean = units.iter().map(|&u| dense.y[u]).sum::<f64>() / count;
        let sample = units
            .iter()
            .map(|&u| (dense.y[u] - mean).powi(2))
            .sum::<f64>()
            / (count - 1.0);
        let pair = units
            .iter()
            .map(|&u| attached.target_diagonal[0][u])
            .sum::<f64>()
            / count
            * count
            / (count - 1.0);
        pair_term +=
            4.0 * pair * pair * sample * sample * (count - 2.0) * (count - 1.0) / (count + 1.0);
        for &u in &units {
            variance[u] = sample;
        }
        stayers += 1;
    }
    assert_eq!(shape.stayers, stayers);
    assert_eq!(shape.stayer_units, (rows - movers) as u64);
    assert_eq!(stayers, 32);
    // The pair term is small beside the movers' signal in this fixture, so it
    // is compared on its own scale; the unit tests cover its place in the sum.
    assert!(pair_term > 0.0);
    assert!(
        (pair_term - shape.stayer_pair_term).abs() <= 1.0e-9 * pair_term,
        "pair term {} versus {pair_term}",
        shape.stayer_pair_term
    );

    // The control term: loads c_t = -2 Lam Zc' g_t, starred influences
    // g_t - Zt Lam Zc' g_t and within parts W c_t from the dense aggregates,
    // and the leave-out residual e / M.
    let lo: Vec<f64> = (0..rows)
        .map(|a| residual[a] * attached.maker_inverse[a])
        .collect();
    let coefficient = |h: &[f64]| -> Vec<f64> {
        let slope: Vec<f64> = (0..k)
            .map(|j| (0..rows).map(|a| dense.collapsed[j][a] * h[a]).sum())
            .collect();
        (0..k)
            .map(|j| (0..k).map(|l| dense.precision[j * k + l] * slope[l]).sum())
            .collect()
    };
    let star: Vec<Vec<f64>> = influence
        .iter()
        .map(|g| {
            let u = coefficient(g);
            (0..rows)
                .map(|a| g[a] - (0..k).map(|j| dense.residualized[j][a] * u[j]).sum::<f64>())
                .collect()
        })
        .collect();
    let within: Vec<Vec<f64>> = influence
        .iter()
        .map(|g| {
            let u = coefficient(g);
            (0..rows)
                .map(|a| {
                    (0..k)
                        .map(|j| -2.0 * dense.within[j][a] * u[j])
                        .sum::<f64>()
                })
                .collect()
        })
        .collect();
    if k > 0 {
        // Collapsed mover matches carry within-unit products; stayers' single
        // observations do not.
        for j in 0..k {
            assert!(dense.within[j][..movers]
                .iter()
                .any(|value| value.abs() > 1.0e-3));
            assert!(dense.within[j][movers..].iter().all(|&value| value == 0.0));
        }
    }
    // One Gaussian atom of two words per unit and draw, and one more for
    // every unit with a within-unit product.
    let within_units = (0..rows)
        .filter(|&a| (0..k).any(|j| dense.within[j][a] != 0.0))
        .count() as u64;
    assert_eq!(within_units > 0, k > 0);
    let draws = u64::from(DRAWS);
    assert_eq!(attached.counter_atoms, (rows as u64 + within_units) * draws);
    assert_eq!(attached.counter_words, 2 * attached.counter_atoms);

    // The trace term against its closed form tr(C_s S C_t S), with S the
    // covariance of the control-adjusted noise, Pi' V Pi + Zc Lam W'W Lam Zc',
    // and the assembled covariance.
    let noise_covariance = dense_noise_covariance(
        &variance,
        &dense.collapsed,
        &dense.residualized,
        &dense.within,
        &dense.precision,
    );
    let product: Vec<Vec<f64>> = (0..3)
        .map(|t| dense_multiply(&dense.kernel[t], rows, rows, &noise_covariance, rows))
        .collect();
    let scale_value = attached.trace_term[0]
        .abs()
        .max(attached.trace_term[4].abs())
        .max(attached.primitive_covariance[0].abs());
    for left in 0..3 {
        for right in 0..3 {
            let index = left * 3 + right;
            let mut trace = 0.0;
            for u in 0..rows {
                for v in 0..rows {
                    trace += product[left][u * rows + v] * product[right][v * rows + u];
                }
            }
            assert!(
                (attached.trace_term[index] - 2.0 * trace).abs()
                    <= 6.0 * attached.trace_mcse[index] + 1e-9 * scale_value,
                "trace {index}: {} versus {}",
                attached.trace_term[index],
                2.0 * trace
            );
            let adjustment = if index == 0 { pair_term } else { 0.0 };
            // With the engine's own draws for the noise means, the rest is
            // exact.
            let expected = match shape.control_terms.as_ref() {
                None => {
                    assert_eq!(k, 0);
                    let leading = (0..rows)
                        .map(|u| 4.0 * variance[u] * influence[left][u] * influence[right][u])
                        .sum::<f64>();
                    leading - attached.trace_term[index] + adjustment
                }
                Some(terms) => {
                    let unit = (0..rows)
                        .map(|a| 4.0 * variance[a] * star[left][a] * star[right][a])
                        .sum::<f64>();
                    let cross = (0..rows)
                        .map(|a| {
                            2.0 * lo[a]
                                * (star[left][a] * within[right][a]
                                    + star[right][a] * within[left][a])
                        })
                        .sum::<f64>();
                    let inside = (0..rows)
                        .map(|a| within[left][a] * within[right][a])
                        .sum::<f64>();
                    for (name, engine, dense_value) in [
                        ("unit", terms.linear_unit[index], unit),
                        ("cross", terms.linear_cross[index], cross),
                        ("within", terms.linear_within[index], inside),
                    ] {
                        assert!(
                            (engine - dense_value).abs() <= 1e-7 * scale_value,
                            "{name} {index}: {engine} versus {dense_value}"
                        );
                    }
                    // Both parts that only collapsed matches carry are
                    // exercised: they are not negligible beside the change
                    // of the unit part.
                    let plain = (0..rows)
                        .map(|a| 4.0 * variance[a] * influence[left][a] * influence[right][a])
                        .sum::<f64>();
                    if left == right {
                        assert!(inside > 1e-3 * (unit - plain).abs(), "within {index}");
                        assert!(cross.abs() > 1e-6 * scale_value, "cross {index}");
                    }
                    unit + cross + inside + adjustment - attached.trace_term[index]
                }
            };
            assert!(
                (attached.primitive_covariance[index] - expected).abs() <= 1e-7 * scale_value,
                "covariance {index}: {} versus {expected}",
                attached.primitive_covariance[index]
            );
            assert_close(
                attached.influence_term[index] + attached.trace_term[index],
                attached.primitive_covariance[index],
                1.0e-12,
            );
        }
    }
}

#[test]
fn repeat_measurement_standard_errors_do_not_depend_on_the_solver_route() {
    let hybrid = super::pooled_component::fixture(false);
    let diagonal = run_repeat(
        &hybrid,
        ModelSolverRoute::Diagonal,
        NuisanceMode::FixedOffset,
    )
    .component_inference
    .expect("diagonal route");
    let cmg = run_repeat(&hybrid, ModelSolverRoute::Cmg, NuisanceMode::FixedOffset)
        .component_inference
        .expect("multigrid route");
    for index in 0..9 {
        assert_close(
            diagonal.primitive_covariance[index],
            cmg.primitive_covariance[index],
            1.0e-6,
        );
    }
}

// The leading direction of every target and the q = 1 decomposition along it
// are formed beside the high-rank covariance, which they do not change.
#[test]
fn repeat_measurement_leading_directions_match_dense_expectations() {
    let problem = observation_repeat_fixture();
    let plain = run_observation_repeat(&problem)
        .component_inference
        .expect("standard errors");
    let attached = run_observation_repeat_with(&problem, true)
        .component_inference
        .expect("standard errors with leading directions");
    for index in 0..9 {
        assert_eq!(
            plain.primitive_covariance[index].to_bits(),
            attached.primitive_covariance[index].to_bits()
        );
    }
    assert!(plain
        .residual_noise
        .as_ref()
        .and_then(|value| value.repeat.as_ref())
        .is_some_and(|shape| shape.leading.is_none()));
    assert!(attached.q1.is_none());
    assert_observation_repeat(&problem, &problem.outcome, &attached, 2.0e-9);
}

// The control term's aggregates and work vectors enlarge the attachment's
// own forecast on observation units too, and the forecast stays the exact
// admission boundary.
#[test]
fn repeat_measurement_control_term_has_exact_memory_boundary() {
    let (worker, firm, outcome, controls) = observation_repeat_columns();
    let mut attachment = [0_u64; 2];
    for (slot, columns) in [Vec::new(), controls].into_iter().enumerate() {
        let k = columns.len() as u64;
        let problem =
            observation_repeat_problem(worker.clone(), firm.clone(), outcome.clone(), columns);
        let prepared = prepare_repeat_measurement_with_interrupt(
            &problem,
            ComponentInferenceUnit::Observation,
            ComponentInferenceOptions {
                probes: 4 * DRAWS,
                ..ComponentInferenceOptions::default()
            },
            StructuredVarianceOptions::default(),
            &mut NeverInterrupt,
        )
        .expect("repeat-measurement preparation");
        let mut estimator = options(DeletionMode::Observation, NuisanceMode::Joint);
        estimator.probes = 200;
        estimator.centering = Centering::Mean;
        let run = |estimator| {
            run_generic_jla_routed_with_attachments_and_hybrid_interrupt(
                &problem,
                routed_options(estimator, ModelSolverRoute::Diagonal),
                None,
                Some(&prepared),
                None,
                &mut NeverInterrupt,
            )
        };
        estimator.memory_limit_bytes = u64::MAX;
        let baseline = run(estimator).expect("unlimited run");
        let memory = baseline.receipt.execution.memory;
        let peak = baseline.receipt.peak_forecast_bytes;
        assert_eq!(memory.peak_bytes, peak);
        attachment[slot] = memory.component_inference_peak_bytes;
        estimator.memory_limit_bytes = peak;
        run(estimator).expect("the exact forecast is accepted");
        estimator.memory_limit_bytes = peak - 1;
        let error = run(estimator).expect_err("one byte below the forecast is rejected");
        assert_eq!(error.code, ErrorCode::ResourceLimit);
        if k > 0 {
            // At least the three aggregates and the eight work vectors.
            let units = problem.outcome.len() as u64;
            assert!(attachment[1] >= attachment[0] + units * (3 * k + 8) * 8);
        }
    }
}

/// Bit patterns captured before the control term was added (source
/// 29ace86c, aarch64-apple-darwin, Rust 1.85.1): the observation fixture
/// without controls with its leading directions (primitive covariance, the
/// four-target covariance, and ten q = 1 values per target), then the
/// mover-only match fixture without controls (primitive covariance).
const NO_CONTROL_BITS: [u64; 74] = [
    0x3f40375e827dde43,
    0xbf101afbaba65de2,
    0x3f1458406be9b45d,
    0xbf101afbaba65de2,
    0x3f39b869580de0c7,
    0xbf283e0bcc3c83a0,
    0x3f1458406be9b45d,
    0xbf283e0bcc3c83a0,
    0x3f26e09979bdacd2,
    0x3f40375e827dde43,
    0xbf101afbaba65de2,
    0x3f1458406be9b45d,
    0x3f434a0f28037f9e,
    0xbf101afbaba65de2,
    0x3f39b869580de0c7,
    0xbf283e0bcc3c83a0,
    0xbf04630af8c1d290,
    0x3f1458406be9b45d,
    0xbf283e0bcc3c83a0,
    0x3f26e09979bdacd2,
    0x3f2faf475d33b032,
    0x3f434a0f28037f9e,
    0xbf04630af8c1d290,
    0x3f2faf475d33b032,
    0x3f50edc113889d47,
    0x3fc8c7336a8d93e8,
    0x3fc8c7336a8d93e8,
    0xbf65b9776f9a84b1,
    0x3f43b5585f3a2012,
    0x3f45c699590d4849,
    0x3f12c778392ab76c,
    0x3fd2a05c87cd8671,
    0x3fdcecd74d036629,
    0x3fe1f517e8630ddf,
    0x3fb2ab571a965555,
    0x3fc5ba89b412c8df,
    0x3fc5ba89b412c8df,
    0x3f4cd106649b907a,
    0x3f302bc4d1500745,
    0x3f309b7efc1dc734,
    0x3edc9f4d78d0180d,
    0x3fd8f364400cc2dd,
    0x3fb9e125d7f29821,
    0x3fc93d41618ec9c9,
    0x3fb6600ed88f24c6,
    0x3fc9780d747d2ca8,
    0x3fc9780d747d2ca8,
    0xbf51ec8d486bbdaf,
    0x3f126162d775e7d9,
    0x3f1314bbb2a69e2c,
    0x3ec66e588b74b98f,
    0x3fe3ecf24c1e80c9,
    0xbfa1f2b66699d84a,
    0x3fa27785f5a1c888,
    0x3fb562935be2d9c0,
    0x3fc9780d747d2ca8,
    0x3fc9780d747d2ca8,
    0xbf71c20146ae67fc,
    0x3f52a579cafc2cab,
    0x3f53b136cafca469,
    0x3f135b2fb7833676,
    0x3fb59981d49e91a4,
    0x3fe32aec7739bffa,
    0x3fe7513b011d903f,
    0x3fb562935be2d9c0,
    0x3f6d182babe9158f,
    0x3ef03bb6755938fa,
    0x3f220fed195d31ea,
    0x3ef03bb6755938fa,
    0x3f593be917126586,
    0x3f22e5f5bf0db1e4,
    0x3f220fed195d31ea,
    0x3f22e5f5bf0db1e4,
    0x3efc3a5710aff8d5,
];

// Without controls the control term takes no part: the covariance and the
// leading export are bit for bit what they were before it existed. The bits
// were captured on one platform; elsewhere the transcendental functions of
// the pseudo-noise may round differently, so the comparison runs there only.
#[test]
fn without_controls_the_repeat_measurement_results_are_unchanged() {
    let problem = observation_repeat_fixture();
    let attached = run_observation_repeat_with(&problem, true)
        .component_inference
        .expect("standard errors");
    let shape = attached
        .residual_noise
        .as_ref()
        .and_then(|value| value.repeat.as_ref())
        .expect("repeat-measurement shape");
    assert!(shape.control_terms.is_none());
    let leading = shape.leading.as_ref().expect("leading directions");
    let mut bits: Vec<u64> = attached
        .primitive_covariance
        .iter()
        .chain(&attached.covariance)
        .map(|value| value.to_bits())
        .collect();
    for target in 0..4 {
        let q = &leading.q1[target];
        for value in [
            q.leading_variance,
            q.leading_variance_correction,
            q.leading_remainder_covariance,
            q.remainder_variance,
            q.remainder_influence_variance,
            q.remainder_trace_variance,
            q.curvature,
            q.confidence_lower,
            q.confidence_upper,
            leading.leading_variance_fitted[target],
        ] {
            bits.push(value.to_bits());
        }
    }
    let mover = fixture(false);
    let prepared = prepare_repeat_measurement_with_interrupt(
        &mover,
        ComponentInferenceUnit::Match,
        ComponentInferenceOptions {
            probes: 4 * DRAWS,
            ..ComponentInferenceOptions::default()
        },
        StructuredVarianceOptions::default(),
        &mut NeverInterrupt,
    )
    .expect("repeat-measurement preparation");
    let matched = run_generic_jla_routed_with_attachments_and_hybrid_interrupt(
        &mover,
        routed_options(
            GenericJlaOptions {
                centering: Centering::Mean,
                ..options(DeletionMode::Match, NuisanceMode::FixedOffset)
            },
            ModelSolverRoute::Diagonal,
        ),
        None,
        Some(&prepared),
        None,
        &mut NeverInterrupt,
    )
    .expect("mover-only run")
    .component_inference
    .expect("standard errors");
    assert!(matched.nuisance_uncertainty_conditioned_away);
    bits.extend(
        matched
            .primitive_covariance
            .iter()
            .map(|value| value.to_bits()),
    );
    assert_eq!(bits.len(), NO_CONTROL_BITS.len());
    if cfg!(all(target_arch = "aarch64", target_os = "macos")) {
        assert_eq!(bits, NO_CONTROL_BITS);
    }
}
