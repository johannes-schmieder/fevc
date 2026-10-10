// SPDX-License-Identifier: GPL-3.0-only
use super::*;
use vckss_core::component_inference::{ComponentInferenceResult, PreparedComponentInference};
use vckss_core::types::Centering;

fn shifted(problem: &CompressedProblem, shift: f64) -> CompressedProblem {
    let mut result = problem.clone();
    result.cell_outcome_sum.fill(0.0);
    for row in 0..result.outcome.len() {
        result.outcome[row] += shift;
        result.cell_outcome_sum[result.row_cell[row] as usize] +=
            result.frequency[row] as f64 * result.outcome[row];
    }
    result
}

pub(super) fn design(problem: &CompressedProblem, controls: bool) -> (Vec<f64>, usize) {
    let fe = problem.workers() + problem.firms() - 1;
    let p = fe + if controls { problem.controls.len() } else { 0 };
    let mut x = vec![0.0; problem.outcome.len() * p];
    for row in 0..problem.outcome.len() {
        x[row * p + problem.row_worker[row] as usize] = 1.0;
        let firm = problem.row_firm[row] as usize;
        if firm + 1 < problem.firms() {
            x[row * p + problem.workers() + firm] = 1.0;
        }
        if controls {
            for (j, z) in problem.controls.iter().enumerate() {
                x[row * p + fe + j] = z[row];
            }
        }
    }
    (x, p)
}

// Fit the full raw control basis independently of production centering and
// control canonicalization; fixed-offset c is the mean of y minus Z gamma.
pub(super) fn working_outcome(
    problem: &CompressedProblem,
    unit: ComponentInferenceUnit,
) -> Vec<f64> {
    if unit == ComponentInferenceUnit::Observation {
        return problem.outcome.clone();
    }
    let n = problem.outcome.len();
    let (x, p) = design(problem, true);
    let mut xtw = dense_transpose(&x, n, p);
    for j in 0..p {
        for row in 0..n {
            xtw[j * n + row] *= problem.frequency[row] as f64;
        }
    }
    let inverse = dense_inverse(&dense_multiply(&xtw, p, n, &x, p), p);
    let beta = dense_multiply(
        &inverse,
        p,
        p,
        &dense_multiply(&xtw, p, n, &problem.outcome, 1),
        1,
    );
    let fe = problem.workers() + problem.firms() - 1;
    (0..n)
        .map(|row| {
            problem.outcome[row]
                - problem
                    .controls
                    .iter()
                    .enumerate()
                    .map(|(j, z)| z[row] * beta[fe + j])
                    .sum::<f64>()
        })
        .collect()
}

fn mean(problem: &CompressedProblem, unit: ComponentInferenceUnit) -> f64 {
    working_outcome(problem, unit)
        .iter()
        .zip(&problem.frequency)
        .map(|(y, f)| y * *f as f64)
        .sum::<f64>()
        / problem.physical_total as f64
}

fn request(unit: ComponentInferenceUnit, centering: Centering) -> GenericJlaExecutionOptions {
    let mut estimator = if unit == ComponentInferenceUnit::Observation {
        options(DeletionMode::Observation, NuisanceMode::Joint)
    } else {
        options(DeletionMode::Match, NuisanceMode::FixedOffset)
    };
    estimator.probes = 200;
    estimator.centering = centering;
    routed_options(estimator, ModelSolverRoute::Diagonal)
}

fn inference_options(reference: ComponentReferenceDistribution) -> ComponentInferenceOptions {
    ComponentInferenceOptions {
        seed: 0x3f91_c0de_2026_1008,
        probes: 257,
        batch_width: 7,
        spectrum_probes: 64,
        spectrum_iterations: 128,
        spectrum_tolerance: 1.0e-3,
        reference_distribution: reference,
        critical_simulations: 2_000,
        ..ComponentInferenceOptions::default()
    }
}

fn run(
    problem: &CompressedProblem,
    unit: ComponentInferenceUnit,
    centering: Centering,
    prepared: &PreparedComponentInference,
) -> GenericJlaResult {
    run_generic_jla_routed_with_attachments_and_hybrid_interrupt(
        problem,
        request(unit, centering),
        None,
        Some(prepared),
        None,
        &mut NeverInterrupt,
    )
    .expect("fixed-observed-mean component inference")
}

fn unchanged_probe_geometry(a: &ComponentInferenceResult, b: &ComponentInferenceResult) {
    assert_eq!(a.trace_term, b.trace_term);
    assert_eq!(a.trace_mcse, b.trace_mcse);
    for (a, b) in a.spectrum.iter().zip(&b.spectrum) {
        assert_eq!(a.certified, b.certified);
        assert_eq!(
            a.leading_eigenvalue.to_bits(),
            b.leading_eigenvalue.to_bits()
        );
        assert_eq!(a.second_eigenvalue.to_bits(), b.second_eigenvalue.to_bits());
        assert_eq!(a.leading_residual.to_bits(), b.leading_residual.to_bits());
        assert_eq!(a.second_residual.to_bits(), b.second_residual.to_bits());
        assert_eq!(a.trace_square_raw.to_bits(), b.trace_square_raw.to_bits());
        assert_eq!(a.leading_share.to_bits(), b.leading_share.to_bits());
        assert_eq!(
            a.remainder_leading_share.to_bits(),
            b.remainder_leading_share.to_bits()
        );
    }
}

// The only production geometry inputs are the retained finite-JLA diagonals
// and maker inverses. Dense normal equations independently construct B and M;
// thus C = B - (D M + M D)/2 tests the actual finite-probe point kernel.
fn check_dense_influence(
    problem: &CompressedProblem,
    unit: ComponentInferenceUnit,
    inference: &ComponentInferenceResult,
    point: [f64; 4],
    variance: f64,
) {
    let (raw_x, p) = design(problem, unit == ComponentInferenceUnit::Observation);
    let rows = inference.independent_units as usize;
    let working = working_outcome(problem, unit);
    let center = mean(problem, unit);
    let mut x = vec![0.0; rows * p];
    let mut y = vec![0.0; rows];
    if unit == ComponentInferenceUnit::Observation {
        x.copy_from_slice(&raw_x);
        for row in 0..rows {
            y[row] = working[row] - center;
        }
    } else {
        assert_eq!(problem.deletion_units(), problem.cells());
        let mut mass = vec![0.0; rows];
        for row in 0..problem.outcome.len() {
            let group = problem.row_deletion[row] as usize;
            let f = problem.frequency[row] as f64;
            mass[group] += f;
            y[group] += f * (working[row] - center);
            x[group * p..(group + 1) * p].copy_from_slice(&raw_x[row * p..(row + 1) * p]);
        }
        for group in 0..rows {
            y[group] /= mass[group].sqrt();
            for value in &mut x[group * p..(group + 1) * p] {
                *value *= mass[group].sqrt();
            }
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
            4.0 * variance * influence.iter().map(|a| a * a).sum::<f64>(),
            inference.influence_term[target * 3 + target],
            2.0e-9,
        );
        dense_influences[target] = influence;
    }
    for left in 0..3 {
        for right in 0..3 {
            let expected = 4.0
                * variance
                * dense_influences[left]
                    .iter()
                    .zip(&dense_influences[right])
                    .map(|(a, b)| a * b)
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
fn mean_component_oracle_q0_q1_matches_dense_frozen_c_and_shifted_none() {
    for unit in [
        ComponentInferenceUnit::Observation,
        ComponentInferenceUnit::Match,
    ] {
        let problem = if unit == ComponentInferenceUnit::Observation {
            component_inference_fixture(true)
        } else {
            grouped_component_inference_fixture(7)
        };
        let units = if unit == ComponentInferenceUnit::Observation {
            problem.outcome.len()
        } else {
            problem.deletion_units()
        };
        for reference in [
            ComponentReferenceDistribution::Q0,
            ComponentReferenceDistribution::Q1,
        ] {
            let variance = 0.002;
            let prepare = if unit == ComponentInferenceUnit::Observation {
                prepare_oracle_component_inference
            } else {
                prepare_grouped_oracle_component_inference
            };
            let prepared = prepare(
                &problem,
                &vec![variance; units],
                inference_options(reference),
            )
            .unwrap();
            let centered = shifted(&problem, -mean(&problem, unit));
            let mean_result = run(&problem, unit, Centering::Mean, &prepared);
            let none_result = run(&centered, unit, Centering::None, &prepared);
            let translated = run(&shifted(&problem, 19.0), unit, Centering::Mean, &prepared);
            queued_component::science(&mean_result, &none_result);
            queued_component::science(&mean_result, &translated);
            let point_only =
                run_generic_jla_routed(&problem, request(unit, Centering::Mean)).unwrap();
            assert_eq!(point_only.corrected, mean_result.corrected);
            assert_eq!(point_only.numerical_mcse, mean_result.numerical_mcse);
            check_dense_influence(
                &problem,
                unit,
                mean_result.component_inference.as_ref().unwrap(),
                components(mean_result.corrected),
                variance,
            );
            // Freezing c leaves every Gaussian quadratic-noise draw and the
            // design spectrum unchanged; sample-recentering probes fails this.
            let uncentered = run(&problem, unit, Centering::None, &prepared);
            let a = mean_result.component_inference.as_ref().unwrap();
            let b = uncentered.component_inference.as_ref().unwrap();
            unchanged_probe_geometry(a, b);
            if let (Some(a), Some(b)) = (a.q1, b.q1) {
                for (a, b) in a.iter().zip(&b) {
                    assert_eq!(a.remainder_trace_variance, b.remainder_trace_variance);
                    assert!(a.remainder_identity_error < 1.0e-9);
                }
            }
        }
    }
}

#[test]
fn mean_component_residual_moments_q0_q1_preserve_both_variance_models() {
    for unit in [
        ComponentInferenceUnit::Observation,
        ComponentInferenceUnit::Match,
    ] {
        let problem = if unit == ComponentInferenceUnit::Observation {
            structured_component_inference_fixture()
        } else {
            grouped_component_inference_fixture(14)
        };
        let centered = shifted(&problem, -mean(&problem, unit));
        for source in [
            ComponentVarianceSource::StructuredCommon,
            ComponentVarianceSource::StructuredLeverage,
        ] {
            for reference in [
                ComponentReferenceDistribution::Q0,
                ComponentReferenceDistribution::Q1,
            ] {
                let prepared =
                    vckss_core::residual_moment_inference::prepare_direct_with_interrupt(
                        &problem,
                        unit,
                        source,
                        inference_options(reference),
                        StructuredVarianceOptions::default(),
                        513,
                        &mut NeverInterrupt,
                    )
                    .unwrap();
                let actual = run(&problem, unit, Centering::Mean, &prepared);
                let expected = run(&centered, unit, Centering::None, &prepared);
                queued_component::science(&actual, &expected);
                let uncentered = run(&problem, unit, Centering::None, &prepared);
                let a = actual.component_inference.as_ref().unwrap();
                let b = uncentered.component_inference.as_ref().unwrap();
                let fit = a.residual_moments.as_ref().unwrap();
                assert_eq!(fit.projections.len(), 513);
                assert_eq!(
                    fit.fit.raw_variance,
                    b.residual_moments.as_ref().unwrap().fit.raw_variance
                );
                assert_eq!(fit.gram, b.residual_moments.as_ref().unwrap().gram);
                unchanged_probe_geometry(a, b);
            }
        }
    }
}
