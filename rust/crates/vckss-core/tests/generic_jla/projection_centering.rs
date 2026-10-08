// SPDX-License-Identifier: GPL-3.0-only
use super::*;
use vckss_core::projection::ProjectionResult;
use vckss_core::types::Centering;

fn fixture() -> CompressedProblem {
    let mut columns = InputColumns {
        worker: vec![],
        firm: vec![],
        deletion: vec![],
        outcome: vec![],
        frequency: vec![],
        target_weight: vec![],
        controls: vec![vec![]],
    };
    for worker in 0..8 {
        for firm in 0..6 {
            for repeat in 0..(2 + (worker + firm) % 3) {
                let row = columns.outcome.len();
                let control = 1.5 + ((row * 7) % 13) as f64 / 5.0 + repeat as f64 / 3.0;
                columns.worker.push(10 + worker);
                columns.firm.push(100 + firm);
                columns.deletion.push(1000 + worker * 6 + firm);
                columns.frequency.push(1 + (row % 3) as u64);
                columns.target_weight.push(0.5 + (row % 7) as f64 / 5.0);
                columns.controls[0].push(control);
                columns.outcome.push(
                    2.3 + 0.4 * control + 0.07 * worker as f64 - 0.09 * firm as f64
                        + 2.0 * ((row * 11 + 5) as f64).sin(),
                );
            }
        }
    }
    let rows = columns.outcome.len();
    CanonicalInput::from_validated(columns.validate().unwrap())
        .unwrap()
        .compress(&vec![true; rows])
        .unwrap()
}

// Independent weighted normal equations: the tested centering helper and
// production canonical control basis are deliberately not used by this oracle.
fn working_mean(problem: &CompressedProblem, nuisance: NuisanceMode) -> f64 {
    let n = problem.outcome.len();
    let fe = problem.workers() + problem.firms() - 1;
    let p = fe + problem.controls.len();
    let mut x = vec![0.0; n * p];
    for row in 0..n {
        x[row * p + problem.row_worker[row] as usize] = 1.0;
        let firm = problem.row_firm[row] as usize;
        if firm + 1 < problem.firms() {
            x[row * p + problem.workers() + firm] = 1.0;
        }
        for (j, z) in problem.controls.iter().enumerate() {
            x[row * p + fe + j] = z[row];
        }
    }
    let mut xtw = dense_transpose(&x, n, p);
    for j in 0..p {
        for row in 0..n {
            xtw[j * n + row] *= problem.frequency[row] as f64;
        }
    }
    let h_inv = dense_inverse(&dense_multiply(&xtw, p, n, &x, p), p);
    let beta = dense_multiply(
        &h_inv,
        p,
        p,
        &dense_multiply(&xtw, p, n, &problem.outcome, 1),
        1,
    );
    let weighted_sum: f64 = (0..n)
        .map(|row| {
            let offset = if nuisance == NuisanceMode::FixedOffset {
                problem
                    .controls
                    .iter()
                    .enumerate()
                    .map(|(j, z)| z[row] * beta[fe + j])
                    .sum()
            } else {
                0.0
            };
            problem.frequency[row] as f64 * (problem.outcome[row] - offset)
        })
        .sum();
    weighted_sum / problem.physical_total as f64
}

fn shifted(problem: &CompressedProblem, location: f64) -> CompressedProblem {
    let mut result = problem.clone();
    result.cell_outcome_sum.fill(0.0);
    for row in 0..result.outcome.len() {
        result.outcome[row] += location;
        result.cell_outcome_sum[result.row_cell[row] as usize] +=
            result.frequency[row] as f64 * result.outcome[row];
    }
    result
}

fn run(
    problem: &CompressedProblem,
    deletion: DeletionMode,
    nuisance: NuisanceMode,
    weight: ProjectionWeight,
    centering: Centering,
) -> Result<ProjectionResult> {
    let project = (0..problem.outcome.len())
        .map(|row| {
            let firm = problem.row_firm[row] as f64;
            let worker = problem.row_worker[row] as f64;
            (firm / 2.0).sin() + worker / 30.0 + row as f64 / 700.0
        })
        .collect::<Vec<_>>();
    let prepared =
        prepare_projection(problem, &[project], ProjectionEffect::Firm, weight, 1.0e-10)?;
    let mut estimator = options(deletion, nuisance);
    estimator.probes = 200;
    estimator.centering = centering;
    estimator.projection_columns = prepared.columns;
    estimator.prepared_persistent_bytes = prepared.persistent_bytes;
    estimator.projection_result_bytes = 4096;
    estimator.projection_export_bytes = 4096;
    Ok(run_generic_jla_routed_with_projection_and_hybrid_interrupt(
        problem,
        routed_options(estimator, ModelSolverRoute::Diagonal),
        Some(&prepared),
        None,
        &mut NeverInterrupt,
    )?
    .projection
    .expect("requested projection"))
}

fn close(left: &[f64], right: &[f64]) {
    for (&a, &b) in left.iter().zip(right) {
        assert_close(a, b, 2.0e-9);
    }
}

#[test]
fn mean_projection_matches_independently_centered_input_and_is_location_invariant() {
    let problem = fixture();
    for deletion in [DeletionMode::Observation, DeletionMode::Match] {
        for nuisance in [NuisanceMode::Joint, NuisanceMode::FixedOffset] {
            let center = working_mean(&problem, nuisance);
            let unweighted = problem.outcome.iter().sum::<f64>() / problem.outcome.len() as f64;
            assert!((center - unweighted).abs() > 1.0e-4);
            let centered = shifted(&problem, -center);
            assert!(working_mean(&centered, nuisance).abs() < 1.0e-11);
            for weight in [ProjectionWeight::Frequency, ProjectionWeight::Target] {
                let mean = run(&problem, deletion, nuisance, weight, Centering::Mean)
                    .expect("Mean covariance is identified and positive semidefinite");
                let oracle = run(&centered, deletion, nuisance, weight, Centering::None)
                    .expect("independently centered None oracle");
                let location = run(
                    &shifted(&problem, 19.0),
                    deletion,
                    nuisance,
                    weight,
                    Centering::Mean,
                )
                .expect("location-shifted Mean covariance");
                close(&mean.covariance, &oracle.covariance);
                close(&mean.covariance, &location.covariance);
                close(&mean.coefficients, &oracle.coefficients);
                close(&mean.coefficients, &location.coefficients);
                close(&mean.naive_covariance, &oracle.naive_covariance);
                close(&mean.naive_covariance, &location.naive_covariance);
                let none = run(&problem, deletion, nuisance, weight, Centering::None)
                    .expect("unchanged None route");
                close(&mean.coefficients, &none.coefficients);
                close(&mean.naive_covariance, &none.naive_covariance);
                assert!(mean
                    .covariance
                    .iter()
                    .zip(&none.covariance)
                    .any(|(a, b)| (a - b).abs() > 1.0e-6));
                assert!(mean.maximum_complete_residual <= mean.full_residual_tolerance);
            }
        }
    }
}

#[test]
fn corrected_projection_and_centered_component_inference_reject_before_rng() {
    let problem = fixture();
    let error = run(
        &problem,
        DeletionMode::Observation,
        NuisanceMode::Joint,
        ProjectionWeight::Frequency,
        Centering::Corrected,
    )
    .expect_err("Corrected projection is not implemented");
    assert_eq!(error.code, ErrorCode::UnsupportedFeature);
    assert_eq!(error.phase, "centering");

    let problem = component_inference_fixture(false);
    let prepared = prepare_oracle_component_inference(
        &problem,
        &vec![0.002; problem.outcome.len()],
        ComponentInferenceOptions::default(),
    )
    .unwrap();
    for centering in [Centering::Mean, Centering::Corrected] {
        let mut estimator = options(DeletionMode::Observation, NuisanceMode::Joint);
        estimator.centering = centering;
        let error = run_generic_jla_routed_with_attachments_and_hybrid_interrupt(
            &problem,
            routed_options(estimator, ModelSolverRoute::Diagonal),
            None,
            Some(&prepared),
            None,
            &mut NeverInterrupt,
        )
        .expect_err("component inference remains None only");
        assert_eq!(error.code, ErrorCode::UnsupportedFeature);
        assert_eq!(error.phase, "centering");
    }
}
