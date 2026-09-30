// SPDX-License-Identifier: GPL-3.0-only
use super::*;
use crate::problem::CanonicalInput;
use crate::stayer_hybrid::{prepare_exact_stayer_hybrid, StayerAugmentationInput};
use crate::types::InputColumns;

#[test]
#[allow(clippy::excessive_precision)]
fn actual_point_clipping_flag_survives_different_derivative_rounding() {
    let raw = [
        0.00030000000000000003,
        2.9997,
        3.0000000000000004e-8,
        2.9994000300000003,
        0.00029997000000000004,
    ];
    let mut moments = FiveMoments::default();
    moments.projection.add(raw[0]);
    moments.residual.add(raw[1]);
    moments.projection_fourth.add(raw[2]);
    moments.residual_fourth.add(raw[3]);
    moments.mixed.add(raw[4]);
    let finite = moments.finite(3., GenericJlaOptions::default(), 0).unwrap();
    assert!(finite.variance_clipped);
    assert!(
        !FiniteDerivative::new(raw.map(|x| x / 3.), 3)
            .unwrap()
            .nonsmooth
    );
    let mut state = State::new(&fixture(), 3, 3, &mut NeverInterrupt).unwrap();
    state.point_clipping(finite.variance_clipped);
    assert_eq!(state.status, Status::NonsmoothAdjustment);
}

fn fixture() -> CompressedProblem {
    let mut input = InputColumns {
        worker: vec![],
        firm: vec![],
        deletion: vec![],
        outcome: vec![],
        frequency: vec![],
        target_weight: vec![],
        controls: vec![vec![]],
    };
    for w in 0..4 {
        for f in 0..3 {
            for repeat in 0..3 {
                let row = input.worker.len();
                input.worker.push(w + 1);
                input.firm.push(f + 1);
                input.deletion.push(100 + w * 3 + f);
                input
                    .outcome
                    .push(w as f64 * 0.4 - f as f64 * 0.3 + (row as f64 * 0.7).cos());
                input.frequency.push(1 + repeat % 2);
                input.target_weight.push(1.0 + (row % 5) as f64);
                input.controls[0].push((row as f64 * 1.7).sin() * 0.2);
            }
        }
    }
    CanonicalInput::from_validated(input.validate().unwrap())
        .unwrap()
        .compress(&[true; 36])
        .unwrap()
}

#[test]
#[ignore = "explicit local oracle gate requires the repository Python venv and NumPy"]
fn actual_cached_whole_responses_match_independent_dense_complex_step() {
    let problem = fixture();
    for deletion in [DeletionMode::Observation, DeletionMode::Match] {
        for nuisance in [NuisanceMode::Joint, NuisanceMode::FixedOffset] {
            let options = GenericJlaOptions {
                deletion,
                nuisance,
                probes: 17,
                leverage_batch_width: 3,
                target_batch_width: 5,
                solver: ModelSolverOptions {
                    pcg: crate::krylov::PcgOptions {
                        tolerance: 1e-12,
                        maximum_iterations: 5000,
                        residual_replacement_interval: 17,
                    },
                    ..ModelSolverOptions::default()
                },
                ..GenericJlaOptions::default()
            };
            let execution = GenericJlaExecutionOptions {
                estimator: options,
                routing: ModelRoutingOptions {
                    route: ModelSolverRoute::Diagonal,
                    solver: options.solver,
                    ..ModelRoutingOptions::default()
                },
                leverage_batch: BatchRequest::Explicit(3),
                target_batch: BatchRequest::Explicit(5),
                wallseconds: None,
            };
            run_generic_jla_with_numerical_mc_interrupt(
                &problem,
                execution,
                None,
                &mut NeverInterrupt,
            )
            .unwrap();
            run_generic_jla_reference_interrupt(
                &problem,
                execution,
                None,
                ReferenceProbePlan {
                    leverage_seed: 771,
                    target_seed: 992,
                    leverage_probes: 17,
                    target_probes: 23,
                },
                true,
                &mut NeverInterrupt,
            )
            .unwrap();
            if deletion == DeletionMode::Match {
                let hybrid = prepare_exact_stayer_hybrid(
                    &problem,
                    StayerAugmentationInput {
                        worker: vec![1, 1, 1],
                        firm: vec![1, 1, 1],
                        outcome: vec![0.2, 0.9, -0.4],
                        frequency: vec![1, 2, 1],
                        target_weight: vec![2., 1., 3.],
                        controls: vec![vec![0.1, -0.2, 0.7]],
                    },
                )
                .unwrap();
                run_generic_jla_with_numerical_mc_interrupt(
                    &hybrid.problem,
                    execution,
                    Some(&hybrid.plan),
                    &mut NeverInterrupt,
                )
                .unwrap();
                let mut observation = execution;
                observation.estimator.deletion = DeletionMode::Observation;
                run_generic_jla_with_numerical_mc_interrupt(
                    &hybrid.problem,
                    observation,
                    None,
                    &mut NeverInterrupt,
                )
                .unwrap();
            }
        }
    }
}

#[test]
fn replay_batch_width_and_packed_word_boundaries_preserve_covariance() {
    // Physical copies cross 64-bit words; repeated stored rows split one
    // design-equivalent class across a word boundary.
    let mut input = InputColumns {
        worker: vec![],
        firm: vec![],
        deletion: vec![],
        outcome: vec![],
        frequency: vec![],
        target_weight: vec![],
        controls: vec![vec![]],
    };
    for w in 0..4 {
        for f in 0..3 {
            for copy in 0..3 {
                let row = w * 3 + f;
                input.worker.push(w + 1);
                input.firm.push(f + 1);
                input.deletion.push(100 + row);
                input
                    .outcome
                    .push(w as f64 * 0.4 - f as f64 * 0.3 + (row as f64 * 0.7).cos());
                input.frequency.push(if copy == 0 {
                    63
                } else if copy == 1 {
                    3
                } else {
                    2
                });
                input.target_weight.push(
                    (if copy == 0 {
                        63.0
                    } else if copy == 1 {
                        3.0
                    } else {
                        2.0
                    }) * (1.0 + row as f64 / 7.0),
                );
                input.controls[0]
                    .push((row as f64 * 1.7).sin() * 0.2 + if copy == 2 { 0.15 } else { 0.0 });
            }
        }
    }
    let problem = CanonicalInput::from_validated(input.validate().unwrap())
        .unwrap()
        .compress(&[true; 36])
        .unwrap();
    for deletion in [DeletionMode::Observation, DeletionMode::Match] {
        let mut reference: Option<NumericalMcResult> = None;
        for width in [1, 2, 3, 4, 7] {
            let options = GenericJlaOptions {
                deletion,
                probes: 17,
                leverage_batch_width: width,
                target_batch_width: 5,
                solver: ModelSolverOptions {
                    pcg: crate::krylov::PcgOptions {
                        tolerance: 1e-12,
                        maximum_iterations: 5000,
                        residual_replacement_interval: 17,
                    },
                    ..ModelSolverOptions::default()
                },
                ..GenericJlaOptions::default()
            };
            let execution = GenericJlaExecutionOptions {
                estimator: options,
                routing: ModelRoutingOptions {
                    route: ModelSolverRoute::Diagonal,
                    solver: options.solver,
                    ..ModelRoutingOptions::default()
                },
                leverage_batch: BatchRequest::Explicit(width),
                target_batch: BatchRequest::Explicit(5),
                wallseconds: None,
            };
            let (_, result) = run_generic_jla_with_numerical_mc_interrupt(
                &problem,
                execution,
                None,
                &mut NeverInterrupt,
            )
            .unwrap();
            assert_eq!(result.replay_attempted_rhs_count, 17);
            assert_eq!(result.replay_executed_rhs_count, 17);
            // One observation class crosses a word boundary; the other
            // supplies within-cell control variation for deletion identification.
            assert_eq!(
                result.replay_generator_word_evaluations,
                17 * 12
                    * if deletion == DeletionMode::Observation {
                        3
                    } else {
                        2
                    }
            );
            if let Some(base) = &reference {
                let scale = base
                    .covariance
                    .raw
                    .iter()
                    .flatten()
                    .chain(result.covariance.raw.iter().flatten())
                    .fold(0.0_f64, |s, v| s.max(v.abs()));
                assert_eq!(result.covariance.status, base.covariance.status);
                for (a, b) in base
                    .covariance
                    .raw
                    .iter()
                    .flatten()
                    .zip(result.covariance.raw.iter().flatten())
                {
                    assert!((a / scale - b / scale).abs() <= 1e-11);
                }
            } else {
                reference = Some(result);
            }
        }
    }
}
