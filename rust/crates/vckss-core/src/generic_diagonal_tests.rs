// SPDX-License-Identifier: GPL-3.0-only
use super::direct_tests::{controlled_fixture, equivalent, fixture};
use super::*;
use crate::memory::{MemoryBudget, MemoryCheck};

fn request(
    deletion: DeletionMode,
    nuisance: NuisanceMode,
    probes: u32,
) -> GenericJlaExecutionOptions {
    GenericJlaExecutionOptions {
        estimator: GenericJlaOptions {
            deletion,
            nuisance,
            probes,
            memory_budget: MemoryBudget::Unspecified,
            memory_limit_bytes: 0,
            ..Default::default()
        },
        routing: ModelRoutingOptions {
            route: ModelSolverRoute::Diagonal,
            ..Default::default()
        },
        leverage_batch: BatchRequest::Auto,
        target_batch: BatchRequest::Auto,
        wallseconds: None,
    }
}

fn run(
    problem: &CompressedProblem,
    options: GenericJlaExecutionOptions,
    threads: usize,
) -> Result<GenericJlaResult> {
    run_generic_jla_with_diagonal_queue_interrupt(
        problem,
        options,
        None,
        None,
        threads,
        &mut NeverInterrupt,
    )
}

#[test]
fn diagonal_estimator_both_deletions_weights_controls_and_thread_caps() {
    for controlled in [false, true] {
        let problem = if controlled {
            controlled_fixture()
        } else {
            fixture(true, true)
        };
        for deletion in [DeletionMode::Observation, DeletionMode::Match] {
            for nuisance in [NuisanceMode::Joint, NuisanceMode::FixedOffset] {
                let options = request(deletion, nuisance, 33);
                let reference = run_generic_jla_routed(&problem, options).unwrap();
                assert!(reference.receipt.execution.diagonal_queue.is_none());
                for threads in [1, 2, 3, 4, 7, 14, 28, 64] {
                    let candidate = run(&problem, options, threads).unwrap();
                    equivalent(&reference, &candidate);
                    candidate.corrected.verify_accounting(1e-11).unwrap();
                    let execution = &candidate.receipt.execution;
                    let queue = execution.diagonal_queue.unwrap();
                    assert_eq!(execution.selected_route, ModelSolverRoute::Diagonal);
                    assert!(execution.full_cmg.is_none());
                    assert_eq!(
                        execution.batch.leverage_active_width,
                        33.min(32.max(8 * threads))
                    );
                    assert_eq!(
                        execution.batch.target_active_width,
                        33.min(32.max(4 * threads))
                    );
                    assert_eq!(queue.plan.permitted_threads, threads);
                    assert_eq!(queue.plan.workers, threads.min(queue.plan.maximum_rhs));
                    assert_eq!(execution.threads.used, queue.plan.workers);
                    assert_eq!(queue.work.completed_rhs, 99 + problem.controls.len());
                    assert!(queue.work.maximum_active_workers <= queue.plan.workers);
                    assert_eq!(queue.plan.command_peak_bytes, execution.memory.peak_bytes);
                    assert_eq!(
                        execution.batch.plan.selected_command_peak_bytes,
                        execution.memory.peak_bytes
                    );
                    assert!(execution.plan_frozen_before_rng);
                    assert_eq!(execution.counter_atoms_before_plan_freeze, 0);
                    assert!(candidate.receipt.maximum_complete_residual <= 1e-9);
                    if controlled {
                        assert_eq!(
                            candidate.receipt.control_rank.projection_pcg_tolerance,
                            1e-13
                        );
                        assert_eq!(
                            candidate.receipt.control_rank.projection_residual_gate,
                            1e-11
                        );
                        assert!(
                            candidate.receipt.control_rank.maximum_projection_residual <= 1e-11
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn diagonal_estimator_explicit_batches_budgets_and_pre_rng_breaks() {
    let problem = controlled_fixture();
    let mut options = request(DeletionMode::Observation, NuisanceMode::Joint, 33);
    options.leverage_batch = BatchRequest::Explicit(3);
    options.target_batch = BatchRequest::Explicit(5);
    let free = run(&problem, options, 7).unwrap();
    let boundary = free.receipt.execution.memory.peak_bytes;
    for (bytes, passes) in [(boundary, true), (boundary - 1, false)] {
        options.estimator.memory_budget = MemoryBudget::Explicit {
            bytes,
            check: MemoryCheck::Error,
        };
        let result = run(&problem, options, 7);
        assert_eq!(result.is_ok(), passes, "{result:?}");
        if let Ok(result) = result {
            equivalent(&free, &result);
        } else {
            assert_eq!(result.unwrap_err().code, ErrorCode::ResourceLimit);
        }
    }
    for check in [MemoryCheck::Warn, MemoryCheck::Off] {
        options.estimator.memory_budget = MemoryBudget::Explicit { bytes: 1, check };
        let result = run(&problem, options, 7).unwrap();
        equivalent(&free, &result);
        assert_eq!(result.receipt.execution.batch.leverage_active_width, 3);
        assert_eq!(result.receipt.execution.batch.target_active_width, 5);
    }
    options.leverage_batch = BatchRequest::Auto;
    options.target_batch = BatchRequest::Auto;
    let reduced = run(&problem, options, 7).unwrap();
    assert_eq!(reduced.receipt.execution.batch.leverage_active_width, 1);
    assert_eq!(reduced.receipt.execution.batch.target_active_width, 1);
    let small_bound = reduced.receipt.execution.memory.peak_bytes;
    options.estimator.memory_budget = MemoryBudget::Explicit {
        bytes: small_bound,
        check: MemoryCheck::Error,
    };
    assert!(run(&problem, options, 7).is_ok());
    options.estimator.memory_budget = MemoryBudget::Unspecified;
    let automatic = run(&problem, options, 7).unwrap();
    assert_eq!(automatic.receipt.execution.batch.leverage_active_width, 33);
    assert_eq!(automatic.receipt.execution.batch.target_active_width, 32);
    equivalent(&automatic, &reduced);
    struct BreakAt(&'static str);
    impl InterruptCheck for BreakAt {
        fn checkpoint(&mut self, phase: &'static str) -> Result<()> {
            if phase == self.0 {
                Err(BackendError::new(
                    ErrorCode::UserBreak,
                    phase,
                    "injected queue break",
                ))
            } else {
                Ok(())
            }
        }
    }
    for phase in [
        "generic_diagonal_queue_admitted",
        "model_diagonal_queue_admitted",
        "model_diagonal_queue_wait",
        "generic_jla_target_batch",
    ] {
        let error = run_generic_jla_with_diagonal_queue_interrupt(
            &problem,
            options,
            None,
            None,
            7,
            &mut BreakAt(phase),
        )
        .unwrap_err();
        assert_eq!(error.code, ErrorCode::UserBreak, "{phase}");
        equivalent(&automatic, &run(&problem, options, 7).unwrap());
    }
    options.routing.route = ModelSolverRoute::Cmg;
    assert_eq!(
        run(&problem, options, 7).unwrap_err().code,
        ErrorCode::InvalidInput
    );
}

#[test]
fn diagonal_estimator_pooled_stayers_and_projection_keep_their_estimands() {
    use crate::projection::{prepare_projection, ProjectionEffect, ProjectionWeight};
    use crate::stayer_hybrid::{prepare_exact_stayer_hybrid, StayerAugmentationInput};
    let mover = controlled_fixture();
    let mut stayers = StayerAugmentationInput {
        worker: vec![],
        firm: vec![],
        outcome: vec![],
        frequency: vec![],
        target_weight: vec![],
        controls: vec![vec![], vec![]],
    };
    for worker in 0..40 {
        for time in 0..(2 + worker % 7) {
            stayers.worker.push(worker + 1);
            stayers.firm.push(worker % 16 + 1);
            stayers
                .outcome
                .push(worker as f64 * 0.03 + time as f64 * 0.19);
            stayers.frequency.push(1 + time % 3);
            stayers.target_weight.push(0.7 + time as f64 * 0.11);
            stayers.controls[0].push((worker as f64 + time as f64 * 0.21).sin());
            stayers.controls[1].push((worker as f64 * 0.38 + time as f64).cos());
        }
    }
    let combined = prepare_exact_stayer_hybrid(&mover, stayers).unwrap();
    let problem = &combined.problem;
    let columns = vec![
        problem.outcome.clone(),
        (0..problem.outcome.len())
            .map(|r| (r as f64 * 0.17).sin())
            .collect(),
    ];
    let projection = prepare_projection(
        problem,
        &columns,
        ProjectionEffect::Worker,
        ProjectionWeight::Target,
        1e-10,
    )
    .unwrap();
    for deletion in [DeletionMode::Match, DeletionMode::Observation] {
        for nuisance in [NuisanceMode::Joint, NuisanceMode::FixedOffset] {
            let hybrid = (deletion == DeletionMode::Match).then_some(&combined.plan);
            let mut options = request(deletion, nuisance, 33);
            options.estimator.projection_columns = projection.columns;
            options.estimator.projection_result_bytes = 4096;
            options.estimator.projection_export_bytes = 4096;
            options.estimator.prepared_persistent_bytes = projection.persistent_bytes;
            let reference = run_generic_jla_routed_with_projection_and_hybrid_interrupt(
                problem,
                options,
                Some(&projection),
                hybrid,
                &mut NeverInterrupt,
            )
            .unwrap();
            for threads in [1, 7] {
                let candidate = run_generic_jla_with_diagonal_queue_interrupt(
                    problem,
                    options,
                    Some(&projection),
                    hybrid,
                    threads,
                    &mut NeverInterrupt,
                )
                .unwrap();
                equivalent(&reference, &candidate);
                assert_eq!(
                    candidate
                        .receipt
                        .execution
                        .diagonal_queue
                        .unwrap()
                        .work
                        .completed_rhs,
                    99 + problem.controls.len() + projection.columns
                );
                let left = reference.projection.as_ref().unwrap();
                let right = candidate.projection.as_ref().unwrap();
                for (&a, &b) in left
                    .covariance
                    .iter()
                    .chain(&left.coefficients)
                    .zip(right.covariance.iter().chain(&right.coefficients))
                {
                    assert!((a - b).abs() <= 1e-8 * a.abs().max(b.abs()).max(1.0));
                }
                assert!(right.maximum_complete_residual <= right.full_residual_tolerance);
            }
            // Same mixed-degree, weighted pooled projection, now exercising
            // the separately admitted internal CMG attachment executor.
            let mut cmg_options = options;
            cmg_options.routing.route = ModelSolverRoute::Cmg;
            cmg_options.leverage_batch = BatchRequest::Auto;
            cmg_options.target_batch = BatchRequest::Auto;
            for threads in [1, 7] {
                let candidate = run_generic_jla_with_direct_attachments_interrupt(
                    problem,
                    cmg_options,
                    Some(&projection),
                    None,
                    hybrid,
                    FullCmgPlanOptions::production(threads, 1e-10, None),
                    &mut NeverInterrupt,
                )
                .unwrap();
                equivalent(&reference, &candidate);
                let execution = &candidate.receipt.execution;
                let work = execution.direct_attachments.unwrap();
                assert_eq!(work.projection_rhs, projection.columns);
                assert_eq!(work.component_rhs + work.gram_rhs, 0);
                assert_eq!(
                    execution.memory.peak_bytes,
                    execution.batch.plan.selected_command_peak_bytes
                );
                let left = reference.projection.as_ref().unwrap();
                let right = candidate.projection.as_ref().unwrap();
                for (&a, &b) in left
                    .covariance
                    .iter()
                    .chain(&left.coefficients)
                    .chain(&left.naive_covariance)
                    .zip(
                        right
                            .covariance
                            .iter()
                            .chain(&right.coefficients)
                            .chain(&right.naive_covariance),
                    )
                {
                    assert!((a - b).abs() <= 1e-8 * a.abs().max(b.abs()).max(1.0));
                }
                assert!(right.maximum_complete_residual <= right.full_residual_tolerance);
            }
        }
    }
}

#[test]
fn diagonal_estimator_strict_rank_columns_can_exceed_queue_capacity() {
    // Orthogonal within-cell DCT controls keep this a capacity test. A prior
    // sinusoidal fixture failed the unchanged control-basis gate even serially.
    let mut input = crate::types::InputColumns {
        worker: vec![],
        firm: vec![],
        deletion: vec![],
        outcome: vec![],
        frequency: vec![],
        target_weight: vec![],
        controls: vec![vec![]; 32],
    };
    for w in 0..4 {
        for f in 0..4 {
            for repetition in 0..34 {
                let row = input.worker.len();
                input.worker.push(w + 1);
                input.firm.push(f + 1);
                input.deletion.push(1 + w * 4 + f);
                input.outcome.push(
                    0.4 * w as f64 - 0.3 * f as f64
                        + repetition as f64 / 17.0
                        + (row % 9) as f64 / 23.0,
                );
                input.frequency.push(1);
                input.target_weight.push(0.75 + (row + 1) as f64 / 1000.0);
                for q in 0..32 {
                    input.controls[q].push(
                        (std::f64::consts::PI * (repetition as f64 + 0.5) * (q + 1) as f64 / 34.0)
                            .cos(),
                    );
                }
            }
        }
    }
    let n = input.worker.len();
    let problem = crate::problem::CanonicalInput::from_validated(input.validate().unwrap())
        .unwrap()
        .compress(&vec![true; n])
        .unwrap();
    let options = request(DeletionMode::Observation, NuisanceMode::Joint, 2);
    let reference = run_generic_jla_routed(&problem, options).unwrap();
    let result = run(&problem, options, 7).unwrap();
    equivalent(&reference, &result);
    let queue = result.receipt.execution.diagonal_queue.unwrap();
    assert_eq!(queue.plan.maximum_rhs, 4);
    assert_eq!(queue.work.completed_rhs, 38);
    assert_eq!(result.receipt.control_rank.projection_rhs.len(), 32);
    assert!(result.receipt.control_rank.maximum_projection_residual <= 1e-11);
}
