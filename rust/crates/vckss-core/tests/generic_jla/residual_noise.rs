// SPDX-License-Identifier: MIT
use super::*;
use vckss_core::component_inference::prepare_residual_noise_with_interrupt;
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
    collapsed: [Vec<f64>; 2],
    residualized: [Vec<f64>; 2],
    within: [Vec<f64>; 2],
    precision: Vec<f64>,
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
    let weight = problem.frequency.iter().map(|&f| f as f64).collect::<Vec<_>>();
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
    let partial: [Vec<f64>; 2] = core::array::from_fn(|j| {
        let beta = dense_multiply(
            &fe_inverse,
            p,
            p,
            &dense_multiply(&xtw, p, stored, &problem.controls[j], 1),
            1,
        );
        let fitted = dense_multiply(&raw_x, stored, p, &beta, 1);
        (0..stored).map(|row| problem.controls[j][row] - fitted[row]).collect()
    });
    let mut information = vec![0.0; 4];
    for left in 0..2 {
        for right in 0..2 {
            information[left * 2 + right] = (0..stored)
                .map(|row| weight[row] * partial[left][row] * partial[right][row])
                .sum();
        }
    }
    let precision = dense_inverse(&information, 2);

    let rows = inference.independent_units as usize;
    let movers = hybrid.plan.mover_deletion_units;
    let mut x = vec![0.0; rows * p];
    let mut y = vec![0.0; rows];
    let mut mass = vec![0.0_f64; rows];
    let mut collapsed: [Vec<f64>; 2] = core::array::from_fn(|_| vec![0.0; rows]);
    let mut residualized: [Vec<f64>; 2] = core::array::from_fn(|_| vec![0.0; rows]);
    let mut within: [Vec<f64>; 2] = core::array::from_fn(|_| vec![0.0; rows]);
    let mut outcome_sum = vec![0.0; movers];
    let mut next = movers;
    for row in 0..stored {
        if hybrid.plan.stayer_rows[row] {
            for _ in 0..problem.frequency[row] {
                x[next * p..(next + 1) * p].copy_from_slice(&raw_x[row * p..(row + 1) * p]);
                y[next] = working[row] - center;
                mass[next] = 1.0;
                for j in 0..2 {
                    collapsed[j][next] = problem.controls[j][row];
                    residualized[j][next] = partial[j][row];
                }
                next += 1;
            }
        } else {
            let group = problem.row_deletion[row] as usize;
            mass[group] += weight[row];
            outcome_sum[group] += weight[row] * working[row];
            y[group] += weight[row] * (working[row] - center);
            x[group * p..(group + 1) * p].copy_from_slice(&raw_x[row * p..(row + 1) * p]);
            for j in 0..2 {
                collapsed[j][group] += weight[row] * problem.controls[j][row];
                residualized[j][group] += weight[row] * partial[j][row];
            }
        }
    }
    assert_eq!(next, rows);
    for row in 0..stored {
        if !hybrid.plan.stayer_rows[row] {
            let group = problem.row_deletion[row] as usize;
            for j in 0..2 {
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
        for j in 0..2 {
            collapsed[j][group] /= root;
            residualized[j][group] /= root;
        }
    }
    let xt = dense_transpose(&x, rows, p);
    let inverse = dense_inverse(&dense_multiply(&xt, p, rows, &x, p), p);
    let bread = dense_multiply(&x, rows, p, &inverse, p);
    let bread_t = dense_transpose(&bread, rows, p);
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
    }
}

fn run(
    hybrid: &vckss_core::stayer_hybrid::PreparedExactStayerHybrid,
    route: ModelSolverRoute,
    attach: bool,
) -> vckss_core::generic_jla::GenericJlaResult {
    let prepared = prepare_residual_noise_with_interrupt(
        &hybrid.problem,
        ComponentInferenceUnit::Match,
        ComponentInferenceOptions {
            probes: DRAWS,
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
                centering: Centering::Mean,
                ..options(DeletionMode::Match, NuisanceMode::FixedOffset)
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
    let attached = result.component_inference.expect("attached standard errors");
    let diagnostic = attached.residual_noise.as_ref().expect("diagnostic");
    assert_eq!(attached.variance_source, ComponentVarianceSource::LeaveOutResidual);
    assert_eq!(attached.independent_units, hybrid.receipt.combined_deletion_units);
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
                    let (k_left, k_right) =
                        (dense.kernel[left][a * rows + b], dense.kernel[right][a * rows + b]);
                    expected_square += maker * maker * variance[b];
                    cross += k_left * k_right * variance[b];
                    covariance[0] += k_left * maker * variance[b];
                    covariance[1] += k_right * maker * variance[b];
                    trace += k_left * variance[b] * dense.kernel[right][b * rows + a] * variance[a];
                }
                noise += attached.maker_inverse[a]
                    * (expected_square * cross + 2.0 * covariance[0] * covariance[1]);
            }
            let scale = attached.trace_term[0].abs().max(attached.trace_term[4].abs());
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
            -2.0 * (0..rows).map(|a| dense.collapsed[j][a] * influence[t][a]).sum::<f64>()
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
    let scale = attached.influence_term[0].abs().max(attached.influence_term[4].abs());
    for left in 0..3 {
        for right in 0..3 {
            let index = left * 3 + right;
            let expected = (0..rows).map(|a| score(left, a) * score(right, a)).sum::<f64>()
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
