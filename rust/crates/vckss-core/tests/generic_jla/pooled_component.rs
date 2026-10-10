// SPDX-License-Identifier: GPL-3.0-only
use super::*;

fn fixture(expand: bool) -> vckss_core::stayer_hybrid::PreparedExactStayerHybrid {
    let mover = grouped_component_inference_fixture(8);
    let mut stayers = StayerAugmentationInput {
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
    if expand {
        let mut expanded = StayerAugmentationInput {
            firm: Vec::new(),
            worker: Vec::new(),
            outcome: Vec::new(),
            frequency: Vec::new(),
            target_weight: Vec::new(),
            controls: vec![Vec::new(), Vec::new()],
        };
        for row in 0..stayers.outcome.len() {
            for _ in 0..stayers.frequency[row] {
                expanded.firm.push(stayers.firm[row]);
                expanded.worker.push(stayers.worker[row]);
                expanded.outcome.push(stayers.outcome[row]);
                expanded.frequency.push(1);
                expanded
                    .target_weight
                    .push(stayers.target_weight[row] / stayers.frequency[row] as f64);
                for j in 0..2 {
                    expanded.controls[j].push(stayers.controls[j][row]);
                }
            }
        }
        stayers = expanded;
    }
    prepare_exact_stayer_hybrid(&mover, stayers).expect("hybrid preparation")
}

#[test]
fn pooled_component_retains_hybrid_point_and_counts_physical_stayers() {
    let hybrid = fixture(false);
    let execution = routed_options(
        GenericJlaOptions {
            centering: vckss_core::types::Centering::Mean,
            ..options(DeletionMode::Match, NuisanceMode::FixedOffset)
        },
        ModelSolverRoute::Diagonal,
    );
    let point = run_generic_jla_routed_with_attachments_and_hybrid_interrupt(
        &hybrid.problem,
        execution,
        None,
        None,
        Some(&hybrid.plan),
        &mut NeverInterrupt,
    )
    .expect("pooled point");
    let inference = vckss_core::residual_moment_inference::prepare_direct_with_interrupt(
        &hybrid.problem,
        ComponentInferenceUnit::Match,
        ComponentVarianceSource::StructuredLeverage,
        ComponentInferenceOptions {
            probes: 100,
            spectrum_probes: 32,
            ..ComponentInferenceOptions::default()
        },
        StructuredVarianceOptions::default(),
        512,
        &mut NeverInterrupt,
    )
    .expect("component preparation");
    let result = run_generic_jla_routed_with_attachments_and_hybrid_interrupt(
        &hybrid.problem,
        execution,
        None,
        Some(&inference),
        Some(&hybrid.plan),
        &mut NeverInterrupt,
    )
    .expect("mixed component inference");
    for (a, b) in components(point.corrected)
        .into_iter()
        .zip(components(result.corrected))
    {
        assert!((a - b).abs() < 1e-10 * a.abs().max(b.abs()).max(1.0));
    }
    let attached = result.component_inference.expect("attached inference");
    assert_eq!(
        attached.independent_units,
        hybrid.receipt.combined_deletion_units
    );
    assert!(attached.point_correction_identity_error < 1e-9);
    assert!(attached.nuisance_uncertainty_conditioned_away);
    assert_eq!(
        attached
            .residual_moments
            .as_ref()
            .unwrap()
            .fit
            .coefficients
            .len(),
        6
    );
    dense_mixed_oracle(&hybrid, &attached, components(result.corrected));
}

fn dense_mixed_oracle(
    hybrid: &vckss_core::stayer_hybrid::PreparedExactStayerHybrid,
    inference: &vckss_core::component_inference::ComponentInferenceResult,
    point: [f64; 4],
) {
    let problem = &hybrid.problem;
    let (raw_x, p) = super::component_centering::design(problem, false);
    let working =
        super::component_centering::working_outcome(problem, ComponentInferenceUnit::Match);
    let center = working
        .iter()
        .zip(&problem.frequency)
        .map(|(y, f)| y * *f as f64)
        .sum::<f64>()
        / problem.physical_total as f64;
    let rows = inference.independent_units as usize;
    let movers = hybrid.plan.mover_deletion_units;
    let variance = &inference
        .residual_moments
        .as_ref()
        .unwrap()
        .fit
        .positive_variance;
    let mut x = vec![0.0; rows * p];
    let mut y = vec![0.0; rows];
    let mut mass = vec![0.0_f64; movers];
    let mut next = movers;
    for row in 0..problem.outcome.len() {
        if hybrid.plan.stayer_rows[row] {
            for _ in 0..problem.frequency[row] {
                x[next * p..(next + 1) * p].copy_from_slice(&raw_x[row * p..(row + 1) * p]);
                y[next] = working[row] - center;
                next += 1;
            }
        } else {
            let group = problem.row_deletion[row] as usize;
            let f = problem.frequency[row] as f64;
            mass[group] += f;
            y[group] += f * (working[row] - center);
            x[group * p..(group + 1) * p].copy_from_slice(&raw_x[row * p..(row + 1) * p]);
        }
    }
    assert_eq!(next, rows);
    for group in 0..movers {
        y[group] /= mass[group].sqrt();
        for value in &mut x[group * p..(group + 1) * p] {
            *value *= mass[group].sqrt();
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
    let mut worker_mean = vec![0.0; p];
    let mut firm_mean = vec![0.0; p];
    for cell in 0..problem.cells() {
        let mass = problem.cell_target_sum[cell] / problem.target_total;
        worker_mean[problem.cell_worker[cell] as usize] += mass;
        let firm = problem.cell_firm[cell] as usize;
        if firm + 1 < problem.firms() {
            firm_mean[problem.workers() + firm] += mass;
        }
    }
    let mut targets: [Vec<f64>; 3] = core::array::from_fn(|_| vec![0.0; p * p]);
    for cell in 0..problem.cells() {
        let mass = problem.cell_target_sum[cell] / problem.target_total;
        let mut worker: Vec<f64> = worker_mean.iter().map(|v| -v).collect();
        let mut firm: Vec<f64> = firm_mean.iter().map(|v| -v).collect();
        worker[problem.cell_worker[cell] as usize] += 1.0;
        let firm_index = problem.cell_firm[cell] as usize;
        if firm_index + 1 < problem.firms() {
            firm[problem.workers() + firm_index] += 1.0;
        }
        for i in 0..p {
            for j in 0..p {
                targets[0][i * p + j] += mass * worker[i] * worker[j];
                targets[1][i * p + j] += mass * firm[i] * firm[j];
                targets[2][i * p + j] += 0.5 * mass * (worker[i] * firm[j] + firm[i] * worker[j]);
            }
        }
    }
    let mut dense_influences: [Vec<f64>; 3] = core::array::from_fn(|_| Vec::new());
    for target in 0..3 {
        let mut kernel = dense_multiply(
            &dense_multiply(&bread, rows, p, &targets[target], p),
            rows,
            p,
            &bread_t,
            rows,
        );
        for i in 0..rows {
            for j in 0..rows {
                kernel[i * rows + j] -= 0.5
                    * maker[i * rows + j]
                    * (inference.target_diagonal[target][i] * inference.maker_inverse[i]
                        + inference.target_diagonal[target][j] * inference.maker_inverse[j]);
            }
        }
        let influence = dense_multiply(&kernel, rows, rows, &y, 1);
        for (&expected, &actual) in influence.iter().zip(&inference.influence[target]) {
            assert_close(expected, actual, 2.0e-9);
        }
        assert_close(
            y.iter().zip(&influence).map(|(a, b)| a * b).sum(),
            point[target],
            2.0e-9,
        );
        assert_close(
            4.0 * influence
                .iter()
                .zip(variance)
                .map(|(a, v)| a * a * v)
                .sum::<f64>(),
            inference.influence_term[target * 3 + target],
            2.0e-9,
        );
        dense_influences[target] = influence;
    }
    for left in 0..3 {
        for right in 0..3 {
            let expected = 4.0
                * dense_influences[left]
                    .iter()
                    .zip(&dense_influences[right])
                    .zip(variance)
                    .map(|((a, b), v)| a * b * v)
                    .sum::<f64>();
            assert_close(expected, inference.influence_term[left * 3 + right], 2.0e-9);
        }
    }
    for column in 0..4 {
        assert_eq!(
            inference.covariance[12 + column],
            inference.covariance[column]
                + inference.covariance[4 + column]
                + 2.0 * inference.covariance[8 + column]
        );
    }
}

#[test]
fn pooled_component_literal_frequency_expansion() {
    let compressed = fixture(false);
    let expanded = fixture(true);
    let mut results = Vec::new();
    for hybrid in [&compressed, &expanded] {
        let prepared = vckss_core::residual_moment_inference::prepare_direct_with_interrupt(
            &hybrid.problem,
            ComponentInferenceUnit::Match,
            ComponentVarianceSource::StructuredLeverage,
            ComponentInferenceOptions {
                probes: 100,
                spectrum_probes: 32,
                ..ComponentInferenceOptions::default()
            },
            StructuredVarianceOptions::default(),
            512,
            &mut NeverInterrupt,
        )
        .unwrap();
        results.push(
            run_generic_jla_routed_with_attachments_and_hybrid_interrupt(
                &hybrid.problem,
                routed_options(
                    GenericJlaOptions {
                        centering: vckss_core::types::Centering::Mean,
                        ..options(DeletionMode::Match, NuisanceMode::FixedOffset)
                    },
                    ModelSolverRoute::Diagonal,
                ),
                None,
                Some(&prepared),
                Some(&hybrid.plan),
                &mut NeverInterrupt,
            )
            .unwrap(),
        );
    }
    for (hybrid, result) in [&compressed, &expanded].into_iter().zip(&results) {
        let attached = result.component_inference.as_ref().unwrap();
        assert_eq!(
            attached.independent_units,
            compressed.receipt.combined_deletion_units
        );
        dense_mixed_oracle(hybrid, attached, components(result.corrected));
    }
    // Existing finite point probes use synthetic stored-row deletion IDs.
    // Literal expansion can therefore change their numerical realization;
    // each realization must satisfy its independent physical-unit oracle.
}

#[test]
fn pooled_weighted_common_solver_invariance() {
    let mut input = InputColumns {
        worker: Vec::new(),
        firm: Vec::new(),
        deletion: Vec::new(),
        outcome: Vec::new(),
        frequency: Vec::new(),
        target_weight: Vec::new(),
        controls: vec![Vec::new()],
    };
    for i in 1..=256_u64 {
        let w = (i - 1) / 16 + 1;
        let f = ((i - 1) % 16) / 2 + 1;
        let freq = 1 + i % 3;
        input.worker.push(w);
        input.firm.push(f);
        input.deletion.push(w * 8 + f);
        input.frequency.push(freq);
        input
            .target_weight
            .push(freq as f64 * (1. + (i % 7) as f64 / 10.));
        input.controls[0].push((i as f64 / 7.).sin() + (i % 5) as f64 / 3.);
        input
            .outcome
            .push(w as f64 * 0.2 - f as f64 * 0.3 + (i as f64 * 1.9).sin());
    }
    let mover = CanonicalInput::from_validated(input.validate().unwrap())
        .unwrap()
        .compress(&vec![true; 256])
        .unwrap();
    let mut stayers = StayerAugmentationInput {
        worker: Vec::new(),
        firm: Vec::new(),
        outcome: Vec::new(),
        frequency: Vec::new(),
        target_weight: Vec::new(),
        controls: vec![Vec::new()],
    };
    for i in 257..=512_u64 {
        let w = 17 + (i - 257) / 8;
        let f = w % 8 + 1;
        let freq = 1 + i % 3;
        stayers.worker.push(w - 16);
        stayers.firm.push(f);
        stayers.frequency.push(freq);
        stayers
            .target_weight
            .push(freq as f64 * (1. + (i % 7) as f64 / 10.));
        stayers.controls[0].push((i as f64 / 7.).sin() + (i % 5) as f64 / 3.);
        stayers
            .outcome
            .push(w as f64 * 0.2 - f as f64 * 0.3 + (i as f64 * 1.9).sin());
    }
    let hybrid = prepare_exact_stayer_hybrid(&mover, stayers).unwrap();
    let inference = vckss_core::residual_moment_inference::prepare_direct_with_interrupt(
        &hybrid.problem,
        ComponentInferenceUnit::Match,
        ComponentVarianceSource::StructuredCommon,
        ComponentInferenceOptions {
            probes: 100,
            seed: 1791,
            ..ComponentInferenceOptions::default()
        },
        StructuredVarianceOptions::default(),
        2048,
        &mut NeverInterrupt,
    )
    .unwrap();
    let mut results = Vec::new();
    for route in [ModelSolverRoute::Diagonal, ModelSolverRoute::Cmg] {
        let execution = routed_options(
            GenericJlaOptions {
                probes: 200,
                centering: vckss_core::types::Centering::Mean,
                ..options(DeletionMode::Match, NuisanceMode::FixedOffset)
            },
            route,
        );
        results.push(
            run_generic_jla_routed_with_attachments_and_hybrid_interrupt(
                &hybrid.problem,
                execution,
                None,
                Some(&inference),
                Some(&hybrid.plan),
                &mut NeverInterrupt,
            )
            .unwrap()
            .component_inference
            .unwrap(),
        );
    }
    for (a, b) in results[0].covariance.iter().zip(&results[1].covariance) {
        assert!((a - b).abs() < 1e-8 * a.abs().max(b.abs()).max(1.));
    }
}
